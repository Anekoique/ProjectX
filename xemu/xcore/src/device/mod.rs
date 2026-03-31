pub mod bus;
pub mod intc;
pub mod ram;
#[cfg(test)]
pub mod test_finisher;
pub mod uart;

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering::Relaxed},
};

use crate::{config::Word, error::XResult};

/// MMIO device interface. All offsets are relative to the device's base
/// address.
pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
    fn irq_line(&self) -> bool {
        false
    }
    fn notify(&mut self, _irq_lines: u32) {}
    fn reset(&mut self) {}
}

// Interrupt bit positions in mip
pub const SSIP: u64 = 1 << 1;
pub const MSIP: u64 = 1 << 3;
pub const STIP: u64 = 1 << 5;
pub const MTIP: u64 = 1 << 7;
pub const SEIP: u64 = 1 << 9;
pub const MEIP: u64 = 1 << 11;

/// Hardware-wired mip bits managed via IrqState (excludes SSIP/STIP —
/// software-controlled). STIP is managed by stimecmp comparison, not IrqState.
pub const HW_IP_MASK: Word = (MSIP | MTIP | SEIP | MEIP) as Word;

/// Shared interrupt state between CPU and devices.
#[derive(Clone)]
pub struct IrqState(Arc<AtomicU64>);

impl IrqState {
    pub fn new() -> Self {
        Self(Arc::new(AtomicU64::new(0)))
    }
    pub fn set(&self, bit: u64) {
        self.0.fetch_or(bit, Relaxed);
    }
    pub fn clear(&self, bit: u64) {
        self.0.fetch_and(!bit, Relaxed);
    }
    pub fn load(&self) -> u64 {
        self.0.load(Relaxed)
    }
    pub fn reset(&self) {
        self.0.store(0, Relaxed);
    }
}

/// Fixed-offset MMIO register helper for simple devices.
macro_rules! mmio_regs {
    ( $vis:vis enum $Reg:ident { $( $name:ident = $offset:expr ),* $(,)? } ) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        $vis enum $Reg { $( $name ),* }

        impl $Reg {
            fn decode(offset: usize) -> Option<Self> {
                match offset { $( $offset => Some(Self::$name), )* _ => None }
            }
        }
    };
}
pub(crate) use mmio_regs;
