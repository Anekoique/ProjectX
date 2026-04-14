//! MSWI: Machine Software Interrupt device (ACLINT spec §3).

use crate::{
    arch::riscv::cpu::trap::interrupt::MSIP,
    config::Word,
    device::{Device, IrqState, mmio_regs},
    error::XResult,
};

mmio_regs! {
    enum Reg {
        Msip[4, 0x4000],
    }
}

/// MSWI: per-hart MSIP register driving each hart's machine-software IRQ
/// line.
pub(super) struct Mswi {
    msip: Vec<u32>,
    irqs: Vec<IrqState>,
}

impl Mswi {
    pub(super) fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self {
        debug_assert_eq!(irqs.len(), num_harts);
        Self {
            msip: vec![0; num_harts],
            irqs,
        }
    }

    fn set_msip(&mut self, hart: usize, v: u32) {
        self.msip[hart] = v;
        debug!("mswi: hart={} msip={}", hart, v);
        if v != 0 {
            self.irqs[hart].set(MSIP);
        } else {
            self.irqs[hart].clear(MSIP);
        }
    }
}

#[allow(clippy::unnecessary_cast)]
impl Device for Mswi {
    fn read(&mut self, offset: usize, _size: usize) -> XResult<Word> {
        Ok(match Reg::decode(offset) {
            Some(Reg::Msip { index, sub: 0 }) if index < self.msip.len() => {
                self.msip[index] as Word
            }
            _ => 0,
        })
    }

    fn write(&mut self, offset: usize, _size: usize, val: Word) -> XResult {
        if let Some(Reg::Msip { index, sub: 0 }) = Reg::decode(offset)
            && index < self.msip.len()
        {
            self.set_msip(index, val as u32 & 1);
        }
        Ok(())
    }

    fn reset(&mut self) {
        for hart in 0..self.msip.len() {
            self.set_msip(hart, 0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(num_harts: usize) -> (Mswi, Vec<IrqState>) {
        let irqs = super::super::test_utils::make_irqs(num_harts);
        (Mswi::new(num_harts, irqs.clone()), irqs)
    }

    #[test]
    fn msip_set_and_clear() {
        let (mut dev, irqs) = setup(1);
        dev.write(0x0000, 4, 1).unwrap();
        assert_ne!(irqs[0].load() & MSIP, 0);
        assert_eq!(dev.read(0x0000, 4).unwrap() as u32, 1);
        dev.write(0x0000, 4, 0).unwrap();
        assert_eq!(irqs[0].load() & MSIP, 0);
    }

    #[test]
    fn unmapped_offset_returns_zero() {
        let (mut dev, _) = setup(1);
        assert_eq!(dev.read(0x0100, 4).unwrap(), 0);
    }

    #[test]
    fn reset_clears_state() {
        let (mut dev, irqs) = setup(1);
        dev.write(0x0000, 4, 1).unwrap();
        assert_ne!(irqs[0].load() & MSIP, 0);
        dev.reset();
        assert_eq!(irqs[0].load() & MSIP, 0);
        assert_eq!(dev.read(0x0000, 4).unwrap() as u32, 0);
    }

    #[test]
    fn mswi_independent_of_mtimer() {
        let (mut dev, irqs) = setup(1);
        // Exercise MSIP standalone — no MTIMER, no bus, no SSIP.
        dev.write(0x0000, 4, 1).unwrap();
        assert_ne!(irqs[0].load() & MSIP, 0);
    }

    #[test]
    fn mswi_two_harts_msip1_raises_only_irq1() {
        let (mut dev, irqs) = setup(2);
        // Write MSIP for hart 1 (offset = 1 * stride = 4).
        dev.write(0x0004, 4, 1).unwrap();
        assert_eq!(irqs[0].load() & MSIP, 0, "hart 0 unaffected");
        assert_ne!(irqs[1].load() & MSIP, 0, "hart 1 raised");
        assert_eq!(dev.read(0x0004, 4).unwrap() as u32, 1);
        assert_eq!(dev.read(0x0000, 4).unwrap() as u32, 0);
    }
}
