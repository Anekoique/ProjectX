//! SSWI: Supervisor Software Interrupt device (ACLINT spec §5).

use crate::{
    config::Word,
    device::{Device, IrqState, mmio_regs},
    error::XResult,
};

mmio_regs! {
    enum Reg {
        Setssip[4, 0x4000],
    }
}

/// SSWI: edge-triggered per-hart SETSSIP raising the SSIP edge signal on
/// the addressed hart's [`IrqState`].
pub(super) struct Sswi {
    irqs: Vec<IrqState>,
}

impl Sswi {
    pub(super) fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self {
        debug_assert_eq!(irqs.len(), num_harts);
        Self { irqs }
    }
}

#[allow(clippy::unnecessary_cast)]
impl Device for Sswi {
    fn read(&mut self, _offset: usize, _size: usize) -> XResult<Word> {
        // SETSSIP reads always 0 per spec; unmapped offsets also 0.
        Ok(0)
    }

    fn write(&mut self, offset: usize, _size: usize, val: Word) -> XResult {
        if let Some(Reg::Setssip { index, sub: 0 }) = Reg::decode(offset)
            && index < self.irqs.len()
            && val as u32 & 1 != 0
        {
            debug!("sswi: hart={} setssip", index);
            self.irqs[index].raise_ssip_edge();
        }
        Ok(())
    }

    fn reset(&mut self) {
        for irq in &self.irqs {
            let _ = irq.take_ssip_edge(); // consume any held edge
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(num_harts: usize) -> (Sswi, Vec<IrqState>) {
        let irqs = super::super::test_utils::make_irqs(num_harts);
        (Sswi::new(num_harts, irqs.clone()), irqs)
    }

    #[test]
    fn setssip_is_edge_triggered() {
        let (mut dev, irqs) = setup(1);
        dev.write(0x0000, 4, 1).unwrap();
        assert!(irqs[0].take_ssip_edge(), "ssip edge should be set");
        assert!(!irqs[0].take_ssip_edge(), "ssip edge should be consumed");
    }

    #[test]
    fn setssip_read_returns_zero() {
        let (mut dev, _) = setup(1);
        assert_eq!(dev.read(0x0000, 4).unwrap(), 0);
    }

    #[test]
    fn setssip_write_zero_no_effect() {
        let (mut dev, irqs) = setup(1);
        dev.write(0x0000, 4, 0).unwrap();
        assert!(!irqs[0].take_ssip_edge());
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
        dev.reset();
        assert!(!irqs[0].take_ssip_edge());
    }

    #[test]
    fn sswi_two_harts_setssip1_raises_only_ssip1() {
        let (mut dev, irqs) = setup(2);
        // SETSSIP for hart 1 (offset = 1 * stride = 4).
        dev.write(0x0004, 4, 1).unwrap();
        assert!(!irqs[0].take_ssip_edge(), "hart 0 unaffected");
        assert!(irqs[1].take_ssip_edge(), "hart 1 raised");
    }
}
