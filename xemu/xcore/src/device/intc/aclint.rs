use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
    time::Instant,
};

use crate::{
    config::Word,
    device::{Device, IrqState, MSIP, MTIP, mmio_regs},
    error::XResult,
};

mmio_regs! {
    enum Reg {
        Msip       = 0x0000,
        MtimecmpLo = 0x4000,
        MtimecmpHi = 0x4004,
        MtimeLo    = 0xBFF8,
        MtimeHi    = 0xBFFC,
        Setssip    = 0xC000,
    }
}

/// Read wall-clock every SYNC_INTERVAL ticks to amortize syscall cost.
/// Between syncs, mtime holds the last-read value (frozen).
/// 512 balances accuracy (~50µs granularity at 10M IPS) vs. overhead.
const SYNC_INTERVAL: u64 = 512;

pub struct Aclint {
    epoch: Instant,
    mtime: u64,
    ticks: u64,
    msip: u32,
    mtimecmp: u64,
    ssip: Arc<AtomicBool>,
    irq: IrqState,
}

impl Aclint {
    pub fn new(irq: IrqState, ssip: Arc<AtomicBool>) -> Self {
        Self {
            epoch: Instant::now(),
            mtime: 0,
            ticks: 0,
            msip: 0,
            mtimecmp: u64::MAX,
            ssip,
            irq,
        }
    }

    /// Snap mtime to real wall-clock (10 MHz = nanos / 100).
    #[inline]
    fn sync_wallclock(&mut self) {
        self.mtime = (self.epoch.elapsed().as_nanos() / 100) as u64;
    }

    fn set_msip(&mut self, v: u32) {
        self.msip = v;
        debug!("aclint: msip={}", v);
        if v != 0 {
            self.irq.set(MSIP);
        } else {
            self.irq.clear(MSIP);
        }
    }

    fn check_timer(&mut self) {
        let was_set = self.irq.load() & MTIP != 0;
        if self.mtime >= self.mtimecmp {
            self.irq.set(MTIP);
            if !was_set {
                debug!(
                    "aclint: timer interrupt fired (mtime={:#x} >= mtimecmp={:#x})",
                    self.mtime, self.mtimecmp
                );
            }
        } else {
            self.irq.clear(MTIP);
        }
    }
}

#[allow(clippy::unnecessary_cast)]
impl Device for Aclint {
    fn read(&mut self, offset: usize, _size: usize) -> XResult<Word> {
        Ok(match Reg::decode(offset) {
            Some(Reg::Msip) => self.msip as Word,
            Some(Reg::MtimecmpLo) => self.mtimecmp as u32 as Word,
            Some(Reg::MtimecmpHi) => (self.mtimecmp >> 32) as u32 as Word,
            Some(Reg::MtimeLo | Reg::MtimeHi) => {
                // Guest explicitly reading mtime — return fresh wall-clock.
                self.sync_wallclock();
                if offset == 0xBFF8 {
                    self.mtime as u32 as Word
                } else {
                    (self.mtime >> 32) as u32 as Word
                }
            }
            Some(Reg::Setssip) => 0,
            None => 0,
        })
    }

    fn write(&mut self, offset: usize, size: usize, val: Word) -> XResult {
        match Reg::decode(offset) {
            Some(Reg::Msip) => self.set_msip(val as u32 & 1),
            Some(Reg::MtimecmpLo) => {
                if size >= 8 {
                    self.mtimecmp = val as u64;
                } else {
                    self.mtimecmp = (self.mtimecmp & !0xFFFF_FFFF) | val as u32 as u64;
                }
                debug!("aclint: mtimecmp={:#x}", self.mtimecmp);
                self.check_timer();
            }
            Some(Reg::MtimecmpHi) => {
                self.mtimecmp = (self.mtimecmp & 0xFFFF_FFFF) | ((val as u32 as u64) << 32);
                debug!("aclint: mtimecmp={:#x}", self.mtimecmp);
                self.check_timer();
            }
            Some(Reg::Setssip) if val as u32 & 1 != 0 => {
                debug!("aclint: setssip");
                self.ssip.store(true, Relaxed);
            }
            _ => {}
        }
        Ok(())
    }

    fn tick(&mut self) {
        // Lazily start the epoch on first tick so that time spent in the
        // debugger prompt doesn't count toward mtime.
        if self.ticks == 0 {
            self.epoch = Instant::now();
        }
        self.ticks += 1;
        if self.ticks.is_multiple_of(SYNC_INTERVAL) {
            self.sync_wallclock();
        }
        self.check_timer();
    }

    fn mtime(&self) -> Option<u64> {
        Some(self.mtime)
    }

    fn reset(&mut self) {
        self.epoch = Instant::now();
        self.mtime = 0;
        self.ticks = 0;
        self.msip = 0;
        self.mtimecmp = u64::MAX;
        self.ssip.store(false, Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Aclint, IrqState, Arc<AtomicBool>) {
        let irq = IrqState::new();
        let ssip = Arc::new(AtomicBool::new(false));
        (Aclint::new(irq.clone(), ssip.clone()), irq, ssip)
    }

    #[test]
    fn mtime_advances_after_sync() {
        let (mut dev, ..) = setup();
        // Force a sync
        dev.ticks = SYNC_INTERVAL - 1;
        std::thread::sleep(std::time::Duration::from_millis(2));
        dev.tick(); // triggers sync_wallclock
        let t = dev.mtime().unwrap();
        assert!(t > 0, "mtime should reflect wall-clock: {t}");
    }

    #[test]
    fn mtime_frozen_between_syncs() {
        let (mut dev, ..) = setup();
        dev.ticks = SYNC_INTERVAL - 1;
        dev.tick(); // sync
        let t1 = dev.mtime().unwrap();
        dev.tick(); // no sync — mtime stays
        let t2 = dev.mtime().unwrap();
        assert_eq!(t1, t2);
    }

    #[test]
    fn mtimecmp_sets_mtip() {
        let (mut dev, irq, _) = setup();
        dev.write(0x4000, 4, 0).unwrap();
        dev.write(0x4004, 4, 0).unwrap();
        dev.tick();
        assert_ne!(irq.load() & MTIP, 0);
    }

    #[test]
    fn mtimecmp_max_clears_mtip() {
        let (mut dev, irq, _) = setup();
        dev.write(0x4000, 4, u32::MAX as Word).unwrap();
        dev.write(0x4004, 4, u32::MAX as Word).unwrap();
        dev.tick();
        assert_eq!(irq.load() & MTIP, 0);
    }

    #[test]
    fn msip_set_and_clear() {
        let (mut dev, irq, _) = setup();
        dev.write(0x0000, 4, 1).unwrap();
        assert_ne!(irq.load() & MSIP, 0);
        assert_eq!(dev.read(0x0000, 4).unwrap() as u32, 1);
        dev.write(0x0000, 4, 0).unwrap();
        assert_eq!(irq.load() & MSIP, 0);
    }

    #[test]
    fn setssip_is_edge_triggered() {
        let (mut dev, _, ssip) = setup();
        dev.write(0xC000, 4, 1).unwrap();
        assert!(ssip.swap(false, Relaxed), "ssip should be set");
        assert!(!ssip.load(Relaxed), "ssip should be consumed");
    }

    #[test]
    fn setssip_read_returns_zero() {
        let (mut dev, ..) = setup();
        assert_eq!(dev.read(0xC000, 4).unwrap(), 0);
    }

    #[test]
    fn setssip_write_zero_no_effect() {
        let (mut dev, _, ssip) = setup();
        dev.write(0xC000, 4, 0).unwrap();
        assert!(!ssip.load(Relaxed));
    }

    #[test]
    fn unmapped_offset_returns_zero() {
        let (mut dev, ..) = setup();
        assert_eq!(dev.read(0x0100, 4).unwrap(), 0);
    }

    #[test]
    fn mtime_write_ignored() {
        let (mut dev, ..) = setup();
        dev.sync_wallclock();
        let before = dev.mtime;
        dev.write(0xBFF8, 4, 0xDEAD).unwrap();
        // mtime field should be unchanged by write (mtime is read-only)
        assert_eq!(dev.mtime, before);
    }

    #[test]
    fn reset_clears_state() {
        let (mut dev, irq, _) = setup();
        dev.write(0x0000, 4, 1).unwrap();
        dev.write(0x4000, 4, 0).unwrap();
        dev.write(0x4004, 4, 0).unwrap();
        dev.tick();
        assert_ne!(irq.load(), 0);
        dev.reset();
        irq.reset();
        dev.tick();
        assert_eq!(irq.load() & MTIP, 0);
        assert_eq!(dev.read(0x0000, 4).unwrap() as u32, 0);
    }
}
