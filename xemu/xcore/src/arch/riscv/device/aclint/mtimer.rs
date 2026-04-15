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
    /// Cached `min(mtimecmp[*])`: earliest instant any hart's timer can
    /// fire. While `mtime < next_fire_mtime`, no `check_all` is needed.
    /// RISC-V Privileged §3.1.10: MTIP pends iff `mtime >= mtimecmp[h]`,
    /// so the minimum is a sound lower bound.
    next_fire_mtime: u64,
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
            next_fire_mtime: u64::MAX,
            irqs,
        }
    }

    /// Snap mtime to real wall-clock (10 MHz = nanos / 100).
    #[inline]
    fn sync_wallclock(&mut self) {
        self.mtime = (self.epoch.elapsed().as_nanos() / 100) as u64;
    }

    /// Refresh `next_fire_mtime = min(mtimecmp[*])`. Called on any
    /// `mtimecmp` mutation; the per-tick fast path reads the cached
    /// value only.
    #[inline]
    fn recompute_next_fire(&mut self) {
        self.next_fire_mtime = self.mtimecmp.iter().copied().min().unwrap_or(u64::MAX);
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
            self.recompute_next_fire();
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
        // Deadline fast path (P3): while `mtime` is below the earliest
        // `mtimecmp[*]`, no hart can fire and `check_all` is a no-op.
        // MTIP stays coherent because every `mtimecmp` write re-runs
        // `check_timer` and refreshes `next_fire_mtime`.
        if self.mtime < self.next_fire_mtime {
            return;
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
        self.next_fire_mtime = u64::MAX;
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

    /// P3 V-UT-8a — deadline gate: `next_fire_mtime = u64::MAX` on
    /// construction; all `mtimecmp` writes refresh it to the minimum.
    #[test]
    fn next_fire_tracks_mtimecmp_min() {
        let (mut dev, _) = setup(2);
        assert_eq!(dev.next_fire_mtime, u64::MAX, "initial");

        // Write mtimecmp[0] = 0xAAAA_BBBB_CCCC_DDDD in two halves.
        // Mtimecmp[n, sub=0 low, sub=4 high] at offsets 8*n and 8*n+4.
        dev.write(0x0000, 4, 0xCCCC_DDDD).unwrap();
        dev.write(0x0004, 4, 0xAAAA_BBBB).unwrap();
        assert_eq!(dev.next_fire_mtime, 0xAAAA_BBBB_CCCC_DDDD);

        // Write a smaller mtimecmp[1] — min shrinks.
        dev.write(0x0008, 4, 0x1000).unwrap();
        dev.write(0x000C, 4, 0).unwrap();
        assert_eq!(dev.next_fire_mtime, 0x1000);
    }

    /// P3 V-UT-8b — `tick()` fast-returns without running `check_all`
    /// whenever `mtime < next_fire_mtime`. Observable: MTIP stays clear
    /// while mtimecmp is at default `u64::MAX` regardless of how many
    /// ticks we issue.
    #[test]
    fn tick_fast_path_skips_check_all_while_below_deadline() {
        let (mut dev, irqs) = setup(1);
        // mtimecmp stays u64::MAX → next_fire_mtime = u64::MAX → fast
        // path always taken.
        for _ in 0..1024 {
            dev.tick();
        }
        assert_eq!(irqs[0].load() & MTIP, 0, "MTIP never set on fast path");
        assert_eq!(dev.next_fire_mtime, u64::MAX);
    }

    /// P3 V-UT-8c — reset clears the cached deadline back to u64::MAX.
    #[test]
    fn reset_restores_next_fire_to_max() {
        let (mut dev, _) = setup(1);
        dev.write(0x0000, 4, 0).unwrap();
        dev.write(0x0004, 4, 0).unwrap();
        assert_eq!(dev.next_fire_mtime, 0);
        dev.reset();
        assert_eq!(dev.next_fire_mtime, u64::MAX);
    }
}
