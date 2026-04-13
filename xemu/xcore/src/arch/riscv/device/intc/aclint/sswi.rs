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
        Setssip = 0x0000,
    }
}

/// SSWI: edge-triggered SETSSIP raising the shared SSIP pending flag.
pub(super) struct Sswi {
    ssip: Arc<AtomicBool>,
}

impl Sswi {
    pub(super) fn new(ssip: Arc<AtomicBool>) -> Self {
        Self { ssip }
    }
}

#[allow(clippy::unnecessary_cast)]
impl Device for Sswi {
    fn read(&mut self, _offset: usize, _size: usize) -> XResult<Word> {
        // SETSSIP reads always 0 per spec; unmapped offsets also 0.
        Ok(0)
    }

    fn write(&mut self, offset: usize, _size: usize, val: Word) -> XResult {
        if let Some(Reg::Setssip) = Reg::decode(offset)
            && val as u32 & 1 != 0
        {
            debug!("sswi: setssip");
            self.ssip.store(true, Relaxed);
        }
        Ok(())
    }

    fn reset(&mut self) {
        self.ssip.store(false, Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup() -> (Sswi, Arc<AtomicBool>) {
        let ssip = Arc::new(AtomicBool::new(false));
        (Sswi::new(ssip.clone()), ssip)
    }

    #[test]
    fn setssip_is_edge_triggered() {
        let (mut dev, ssip) = setup();
        dev.write(0x0000, 4, 1).unwrap();
        assert!(ssip.swap(false, Relaxed), "ssip should be set");
        assert!(!ssip.load(Relaxed), "ssip should be consumed");
    }

    #[test]
    fn setssip_read_returns_zero() {
        let (mut dev, _) = setup();
        assert_eq!(dev.read(0x0000, 4).unwrap(), 0);
    }

    #[test]
    fn setssip_write_zero_no_effect() {
        let (mut dev, ssip) = setup();
        dev.write(0x0000, 4, 0).unwrap();
        assert!(!ssip.load(Relaxed));
    }

    #[test]
    fn unmapped_offset_returns_zero() {
        let (mut dev, _) = setup();
        assert_eq!(dev.read(0x0100, 4).unwrap(), 0);
    }

    #[test]
    fn reset_clears_state() {
        let (mut dev, ssip) = setup();
        dev.write(0x0000, 4, 1).unwrap();
        assert!(ssip.load(Relaxed));
        dev.reset();
        assert!(!ssip.load(Relaxed));
    }

    #[test]
    fn sswi_independent_of_mswi() {
        // Drive edge with a fresh Arc — no IrqState required.
        let (mut dev, ssip) = setup();
        dev.write(0x0000, 4, 1).unwrap();
        assert!(ssip.load(Relaxed));
    }
}
