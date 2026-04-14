//! MTIMER: Machine Timer device (ACLINT spec §4).

use std::time::Instant;

use crate::{
    arch::riscv::cpu::trap::interrupt::MTIP,
    config::Word,
    device::{Device, IrqState, mmio_regs},
    error::XResult,
};

/// Read wall-clock every SYNC_INTERVAL ticks to amortize syscall cost.
/// Between syncs, mtime holds the last-read value (frozen).
/// 512 balances accuracy (~50µs granularity at 10M IPS) vs. overhead.
const SYNC_INTERVAL: u64 = 512;

mmio_regs! {
    enum Reg {
        MtimeLo = 0x7FF8,
        MtimeHi = 0x7FFC;
        Mtimecmp[8, 0x4000],
    }
}

/// MTIMER: wall-clock-backed `mtime` + per-hart `mtimecmp` driving MTIP.
pub(super) struct Mtimer {
    epoch: Instant,
    mtime: u64,
    ticks: u64,
    mtimecmp: Vec<u64>,
    irqs: Vec<IrqState>,
}

impl Mtimer {
    pub(super) fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self {
        debug_assert_eq!(irqs.len(), num_harts);
        Self {
            epoch: Instant::now(),
            mtime: 0,
            ticks: 0,
            mtimecmp: vec![u64::MAX; num_harts],
            irqs,
        }
    }

    /// Snap mtime to real wall-clock (10 MHz = nanos / 100).
    #[inline]
    fn sync_wallclock(&mut self) {
        self.mtime = (self.epoch.elapsed().as_nanos() / 100) as u64;
    }

    fn check_timer(&mut self, hart: usize) {
        let was_set = self.irqs[hart].load() & MTIP != 0;
        if self.mtime >= self.mtimecmp[hart] {
            self.irqs[hart].set(MTIP);
            if !was_set {
                debug!(
                    "mtimer: hart={} timer interrupt fired (mtime={:#x} >= mtimecmp={:#x})",
                    hart, self.mtime, self.mtimecmp[hart]
                );
            }
        } else {
            self.irqs[hart].clear(MTIP);
        }
    }

    fn check_all(&mut self) {
        for hart in 0..self.mtimecmp.len() {
            self.check_timer(hart);
        }
    }
}

#[allow(clippy::unnecessary_cast)]
impl Device for Mtimer {
    fn read(&mut self, offset: usize, _size: usize) -> XResult<Word> {
        Ok(match Reg::decode(offset) {
            Some(Reg::MtimeLo) => {
                self.sync_wallclock();
                self.mtime as u32 as Word
            }
            Some(Reg::MtimeHi) => {
                self.sync_wallclock();
                (self.mtime >> 32) as u32 as Word
            }
            Some(Reg::Mtimecmp { index, sub }) if index < self.mtimecmp.len() => match sub {
                0 => self.mtimecmp[index] as u32 as Word,
                4 => (self.mtimecmp[index] >> 32) as u32 as Word,
                _ => 0,
            },
            _ => 0,
        })
    }

    fn write(&mut self, offset: usize, size: usize, val: Word) -> XResult {
        if let Some(Reg::Mtimecmp { index, sub }) = Reg::decode(offset)
            && index < self.mtimecmp.len()
        {
            let cmp = &mut self.mtimecmp[index];
            match sub {
                0 if size >= 8 => *cmp = val as u64,
                0 => *cmp = (*cmp & !0xFFFF_FFFF) | val as u32 as u64,
                4 => *cmp = (*cmp & 0xFFFF_FFFF) | ((val as u32 as u64) << 32),
                _ => return Ok(()),
            }
            debug!("mtimer: hart={} mtimecmp={:#x}", index, *cmp);
            self.check_timer(index);
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
        self.check_all();
    }

    fn mtime(&self) -> Option<u64> {
        Some(self.mtime)
    }

    fn reset(&mut self) {
        self.epoch = Instant::now();
        self.mtime = 0;
        self.ticks = 0;
        for cmp in &mut self.mtimecmp {
            *cmp = u64::MAX;
        }
        for irq in &self.irqs {
            irq.clear(MTIP);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(num_harts: usize) -> (Mtimer, Vec<IrqState>) {
        let irqs = super::super::test_utils::make_irqs(num_harts);
        (Mtimer::new(num_harts, irqs.clone()), irqs)
    }

    #[test]
    fn mtime_advances_after_sync() {
        let (mut dev, _) = setup(1);
        // Force a sync
        dev.ticks = SYNC_INTERVAL - 1;
        std::thread::sleep(std::time::Duration::from_millis(2));
        dev.tick(); // triggers sync_wallclock
        let t = dev.mtime().unwrap();
        assert!(t > 0, "mtime should reflect wall-clock: {t}");
    }

    #[test]
    fn mtime_frozen_between_syncs() {
        let (mut dev, _) = setup(1);
        dev.ticks = SYNC_INTERVAL - 1;
        dev.tick(); // sync
        let t1 = dev.mtime().unwrap();
        dev.tick(); // no sync — mtime stays
        let t2 = dev.mtime().unwrap();
        assert_eq!(t1, t2);
    }

    #[test]
    fn mtimecmp_sets_mtip() {
        let (mut dev, irqs) = setup(1);
        dev.write(0x0000, 4, 0).unwrap();
        dev.write(0x0004, 4, 0).unwrap();
        dev.tick();
        assert_ne!(irqs[0].load() & MTIP, 0);
    }

    #[test]
    fn mtimecmp_max_clears_mtip() {
        let (mut dev, irqs) = setup(1);
        dev.write(0x0000, 4, u32::MAX as Word).unwrap();
        dev.write(0x0004, 4, u32::MAX as Word).unwrap();
        dev.tick();
        assert_eq!(irqs[0].load() & MTIP, 0);
    }

    #[test]
    fn mtime_write_ignored() {
        let (mut dev, _) = setup(1);
        dev.sync_wallclock();
        let before = dev.mtime;
        dev.write(0x7FF8, 4, 0xDEAD).unwrap();
        // mtime field should be unchanged by write (mtime is read-only)
        assert_eq!(dev.mtime, before);
    }

    #[test]
    fn unmapped_offset_returns_zero() {
        let (mut dev, _) = setup(1);
        assert_eq!(dev.read(0x0100, 4).unwrap(), 0);
    }

    #[test]
    fn mtimer_independent_of_sswi() {
        // Exercise mtimecmp/mtime standalone — no SSWI coupling.
        let (mut dev, irqs) = setup(1);
        dev.write(0x0000, 4, 0).unwrap();
        dev.write(0x0004, 4, 0).unwrap();
        dev.tick();
        assert_ne!(irqs[0].load() & MTIP, 0);
    }

    #[test]
    fn mtimer_two_harts_mtimecmp0_fires_only_irq0() {
        let (mut dev, irqs) = setup(2);
        // hart 0: mtimecmp = 0 → MTIP fires. hart 1: mtimecmp stays MAX.
        dev.write(0x0000, 4, 0).unwrap();
        dev.write(0x0004, 4, 0).unwrap();
        dev.tick();
        assert_ne!(irqs[0].load() & MTIP, 0, "hart 0 timer fires");
        assert_eq!(irqs[1].load() & MTIP, 0, "hart 1 unaffected");
    }
}
