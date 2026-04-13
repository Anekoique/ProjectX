//! MSWI: Machine Software Interrupt device (ACLINT spec §3).

use crate::{
    arch::riscv::cpu::trap::interrupt::MSIP,
    config::Word,
    device::{Device, IrqState, mmio_regs},
    error::XResult,
};

mmio_regs! {
    enum Reg {
        Msip = 0x0000,
    }
}

/// MSWI: a single MSIP register driving the machine-software IRQ line.
pub(super) struct Mswi {
    msip: u32,
    irq: IrqState,
}

impl Mswi {
    pub(super) fn new(irq: IrqState) -> Self {
        Self { msip: 0, irq }
    }

    fn set_msip(&mut self, v: u32) {
        self.msip = v;
        debug!("mswi: msip={}", v);
        if v != 0 {
            self.irq.set(MSIP);
        } else {
            self.irq.clear(MSIP);
        }
    }
}

#[allow(clippy::unnecessary_cast)]
impl Device for Mswi {
    fn read(&mut self, offset: usize, _size: usize) -> XResult<Word> {
        Ok(match Reg::decode(offset) {
            Some(Reg::Msip) => self.msip as Word,
            None => 0,
        })
    }

    fn write(&mut self, offset: usize, _size: usize, val: Word) -> XResult {
        if let Some(Reg::Msip) = Reg::decode(offset) {
            self.set_msip(val as u32 & 1);
        }
        Ok(())
    }

    fn reset(&mut self) {
        self.set_msip(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Mswi, IrqState) {
        let irq = IrqState::new();
        (Mswi::new(irq.clone()), irq)
    }

    #[test]
    fn msip_set_and_clear() {
        let (mut dev, irq) = setup();
        dev.write(0x0000, 4, 1).unwrap();
        assert_ne!(irq.load() & MSIP, 0);
        assert_eq!(dev.read(0x0000, 4).unwrap() as u32, 1);
        dev.write(0x0000, 4, 0).unwrap();
        assert_eq!(irq.load() & MSIP, 0);
    }

    #[test]
    fn unmapped_offset_returns_zero() {
        let (mut dev, _) = setup();
        assert_eq!(dev.read(0x0100, 4).unwrap(), 0);
    }

    #[test]
    fn reset_clears_state() {
        let (mut dev, irq) = setup();
        dev.write(0x0000, 4, 1).unwrap();
        assert_ne!(irq.load() & MSIP, 0);
        dev.reset();
        assert_eq!(irq.load() & MSIP, 0);
        assert_eq!(dev.read(0x0000, 4).unwrap() as u32, 0);
    }

    #[test]
    fn mswi_independent_of_mtimer() {
        let (mut dev, irq) = setup();
        // Exercise MSIP standalone — no MTIMER, no bus, no SSIP.
        dev.write(0x0000, 4, 1).unwrap();
        assert_ne!(irq.load() & MSIP, 0);
    }
}
