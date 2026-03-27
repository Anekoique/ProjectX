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

pub struct Aclint {
    epoch: Instant,
    mtime: u64,
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
            msip: 0,
            mtimecmp: u64::MAX,
            ssip,
            irq,
        }
    }

    fn set_msip(&mut self, v: u32) {
        self.msip = v;
        if v != 0 {
            self.irq.set(MSIP);
        } else {
            self.irq.clear(MSIP);
        }
    }

    fn check_timer(&mut self) {
        if self.mtime >= self.mtimecmp {
            self.irq.set(MTIP);
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
            Some(Reg::MtimeLo) => self.mtime as u32 as Word,
            Some(Reg::MtimeHi) => (self.mtime >> 32) as u32 as Word,
            Some(Reg::Setssip) => 0,
            None => 0,
        })
    }

    fn write(&mut self, offset: usize, _size: usize, val: Word) -> XResult {
        match Reg::decode(offset) {
            Some(Reg::Msip) => self.set_msip(val as u32 & 1),
            Some(Reg::MtimecmpLo) => {
                self.mtimecmp = (self.mtimecmp & !0xFFFF_FFFF) | val as u32 as u64;
                self.check_timer();
            }
            Some(Reg::MtimecmpHi) => {
                self.mtimecmp = (self.mtimecmp & 0xFFFF_FFFF) | ((val as u32 as u64) << 32);
                self.check_timer();
            }
            Some(Reg::Setssip) if val as u32 & 1 != 0 => {
                self.ssip.store(true, Relaxed);
            }
            _ => {}
        }
        Ok(())
    }

    fn tick(&mut self) {
        // 10 MHz tick rate: nanos / 100. Divide before truncation to avoid u128→u64
        // wrap.
        self.mtime = (self.epoch.elapsed().as_nanos() / 100) as u64;
        self.check_timer();
    }

    fn reset(&mut self) {
        self.mtime = 0;
        self.msip = 0;
        self.mtimecmp = u64::MAX;
        self.ssip.store(false, Relaxed);
        self.epoch = Instant::now();
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
    fn mtime_advances_after_tick() {
        let (mut dev, ..) = setup();
        dev.tick();
        let t1 = dev.read(0xBFF8, 4).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
        dev.tick();
        let t2 = dev.read(0xBFF8, 4).unwrap();
        assert!(t2 > t1);
    }

    #[test]
    fn mtime_frozen_without_tick() {
        let (mut dev, ..) = setup();
        dev.tick();
        let t1 = dev.read(0xBFF8, 4).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(1));
        let t2 = dev.read(0xBFF8, 4).unwrap();
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
        dev.tick();
        let before = dev.read(0xBFF8, 4).unwrap();
        dev.write(0xBFF8, 4, 0xDEAD).unwrap();
        assert_eq!(dev.read(0xBFF8, 4).unwrap(), before);
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
        // mtimecmp is MAX after reset, so MTIP should not be set
        assert_eq!(irq.load() & MTIP, 0);
        assert_eq!(dev.read(0x0000, 4).unwrap() as u32, 0);
    }
}
