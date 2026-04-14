//! SSWI: Supervisor Software Interrupt device (ACLINT spec §5).

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering::Relaxed},
};

use crate::{
    config::Word,
    device::{Device, mmio_regs},
    error::XResult,
};

mmio_regs! {
    enum Reg {
        Setssip[4, 0x4000],
    }
}

/// SSWI: edge-triggered per-hart SETSSIP raising the shared SSIP pending
/// flag for the addressed hart.
pub(super) struct Sswi {
    ssip: Vec<Arc<AtomicBool>>,
}

impl Sswi {
    pub(super) fn new(num_harts: usize, ssips: Vec<Arc<AtomicBool>>) -> Self {
        debug_assert_eq!(ssips.len(), num_harts);
        Self { ssip: ssips }
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
            && index < self.ssip.len()
            && val as u32 & 1 != 0
        {
            debug!("sswi: hart={} setssip", index);
            self.ssip[index].store(true, Relaxed);
        }
        Ok(())
    }

    fn reset(&mut self) {
        for flag in &self.ssip {
            flag.store(false, Relaxed);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup(num_harts: usize) -> (Sswi, Vec<Arc<AtomicBool>>) {
        let ssips = super::super::test_utils::make_ssips(num_harts);
        (Sswi::new(num_harts, ssips.clone()), ssips)
    }

    #[test]
    fn setssip_is_edge_triggered() {
        let (mut dev, ssips) = setup(1);
        dev.write(0x0000, 4, 1).unwrap();
        assert!(ssips[0].swap(false, Relaxed), "ssip should be set");
        assert!(!ssips[0].load(Relaxed), "ssip should be consumed");
    }

    #[test]
    fn setssip_read_returns_zero() {
        let (mut dev, _) = setup(1);
        assert_eq!(dev.read(0x0000, 4).unwrap(), 0);
    }

    #[test]
    fn setssip_write_zero_no_effect() {
        let (mut dev, ssips) = setup(1);
        dev.write(0x0000, 4, 0).unwrap();
        assert!(!ssips[0].load(Relaxed));
    }

    #[test]
    fn unmapped_offset_returns_zero() {
        let (mut dev, _) = setup(1);
        assert_eq!(dev.read(0x0100, 4).unwrap(), 0);
    }

    #[test]
    fn reset_clears_state() {
        let (mut dev, ssips) = setup(1);
        dev.write(0x0000, 4, 1).unwrap();
        assert!(ssips[0].load(Relaxed));
        dev.reset();
        assert!(!ssips[0].load(Relaxed));
    }

    #[test]
    fn sswi_independent_of_mswi() {
        // Drive edge with a fresh Arc — no IrqState required.
        let (mut dev, ssips) = setup(1);
        dev.write(0x0000, 4, 1).unwrap();
        assert!(ssips[0].load(Relaxed));
    }

    #[test]
    fn sswi_two_harts_setssip1_raises_only_ssip1() {
        let (mut dev, ssips) = setup(2);
        // SETSSIP for hart 1 (offset = 1 * stride = 4).
        dev.write(0x0004, 4, 1).unwrap();
        assert!(!ssips[0].load(Relaxed), "hart 0 unaffected");
        assert!(ssips[1].load(Relaxed), "hart 1 raised");
    }
}
