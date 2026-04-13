//! MTIMER: Machine Timer device (ACLINT spec §4).

use std::time::Instant;

use crate::{
    arch::riscv::cpu::trap::interrupt::MTIP,
    config::Word,
    device::{Device, IrqState, mmio_regs},
    error::XResult,
};

mmio_regs! {
    enum Reg {
        MtimecmpLo = 0x0000,
        MtimecmpHi = 0x0004,
        MtimeLo    = 0x7FF8,
        MtimeHi    = 0x7FFC,
    }
}

/// Read wall-clock every SYNC_INTERVAL ticks to amortize syscall cost.
/// Between syncs, mtime holds the last-read value (frozen).
/// 512 balances accuracy (~50µs granularity at 10M IPS) vs. overhead.
const SYNC_INTERVAL: u64 = 512;

/// MTIMER: wall-clock-backed `mtime` + `mtimecmp` driving MTIP.
pub(super) struct Mtimer {
    epoch: Instant,
    mtime: u64,
    ticks: u64,
    mtimecmp: u64,
    irq: IrqState,
}

impl Mtimer {
    pub(super) fn new(irq: IrqState) -> Self {
        Self {
            epoch: Instant::now(),
            mtime: 0,
            ticks: 0,
            mtimecmp: u64::MAX,
            irq,
        }
    }

    /// Snap mtime to real wall-clock (10 MHz = nanos / 100).
    #[inline]
    fn sync_wallclock(&mut self) {
        self.mtime = (self.epoch.elapsed().as_nanos() / 100) as u64;
    }

    fn check_timer(&mut self) {
        let was_set = self.irq.load() & MTIP != 0;
        if self.mtime >= self.mtimecmp {
            self.irq.set(MTIP);
            if !was_set {
                debug!(
                    "mtimer: timer interrupt fired (mtime={:#x} >= mtimecmp={:#x})",
                    self.mtime, self.mtimecmp
                );
            }
        } else {
            self.irq.clear(MTIP);
        }
    }
}

#[allow(clippy::unnecessary_cast)]
impl Device for Mtimer {
    fn read(&mut self, offset: usize, _size: usize) -> XResult<Word> {
        Ok(match Reg::decode(offset) {
            Some(Reg::MtimecmpLo) => self.mtimecmp as u32 as Word,
            Some(Reg::MtimecmpHi) => (self.mtimecmp >> 32) as u32 as Word,
            Some(Reg::MtimeLo) => {
                self.sync_wallclock();
                self.mtime as u32 as Word
            }
            Some(Reg::MtimeHi) => {
                self.sync_wallclock();
                (self.mtime >> 32) as u32 as Word
            }
            None => 0,
        })
    }

    fn write(&mut self, offset: usize, size: usize, val: Word) -> XResult {
        match Reg::decode(offset) {
            Some(Reg::MtimecmpLo) => {
                if size >= 8 {
                    self.mtimecmp = val as u64;
                } else {
                    self.mtimecmp = (self.mtimecmp & !0xFFFF_FFFF) | val as u32 as u64;
                }
                debug!("mtimer: mtimecmp={:#x}", self.mtimecmp);
                self.check_timer();
            }
            Some(Reg::MtimecmpHi) => {
                self.mtimecmp = (self.mtimecmp & 0xFFFF_FFFF) | ((val as u32 as u64) << 32);
                debug!("mtimer: mtimecmp={:#x}", self.mtimecmp);
                self.check_timer();
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
        self.mtimecmp = u64::MAX;
        self.irq.clear(MTIP);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Mtimer, IrqState) {
        let irq = IrqState::new();
        (Mtimer::new(irq.clone()), irq)
    }

    #[test]
    fn mtime_advances_after_sync() {
        let (mut dev, _) = setup();
        // Force a sync
        dev.ticks = SYNC_INTERVAL - 1;
        std::thread::sleep(std::time::Duration::from_millis(2));
        dev.tick(); // triggers sync_wallclock
        let t = dev.mtime().unwrap();
        assert!(t > 0, "mtime should reflect wall-clock: {t}");
    }

    #[test]
    fn mtime_frozen_between_syncs() {
        let (mut dev, _) = setup();
        dev.ticks = SYNC_INTERVAL - 1;
        dev.tick(); // sync
        let t1 = dev.mtime().unwrap();
        dev.tick(); // no sync — mtime stays
        let t2 = dev.mtime().unwrap();
        assert_eq!(t1, t2);
    }

    #[test]
    fn mtimecmp_sets_mtip() {
        let (mut dev, irq) = setup();
        dev.write(0x0000, 4, 0).unwrap();
        dev.write(0x0004, 4, 0).unwrap();
        dev.tick();
        assert_ne!(irq.load() & MTIP, 0);
    }

    #[test]
    fn mtimecmp_max_clears_mtip() {
        let (mut dev, irq) = setup();
        dev.write(0x0000, 4, u32::MAX as Word).unwrap();
        dev.write(0x0004, 4, u32::MAX as Word).unwrap();
        dev.tick();
        assert_eq!(irq.load() & MTIP, 0);
    }

    #[test]
    fn mtime_write_ignored() {
        let (mut dev, _) = setup();
        dev.sync_wallclock();
        let before = dev.mtime;
        dev.write(0x7FF8, 4, 0xDEAD).unwrap();
        // mtime field should be unchanged by write (mtime is read-only)
        assert_eq!(dev.mtime, before);
    }

    #[test]
    fn unmapped_offset_returns_zero() {
        let (mut dev, _) = setup();
        assert_eq!(dev.read(0x0100, 4).unwrap(), 0);
    }

    #[test]
    fn mtimer_independent_of_sswi() {
        // Exercise mtimecmp/mtime standalone — no SSWI, no Bus::take_ssip.
        let (mut dev, irq) = setup();
        dev.write(0x0000, 4, 0).unwrap();
        dev.write(0x0004, 4, 0).unwrap();
        dev.tick();
        assert_ne!(irq.load() & MTIP, 0);
    }
}
