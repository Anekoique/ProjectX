//! Emulated hardware devices: memory bus, RAM, UART, interrupt controllers.
//!
//! All devices implement the [`Device`] trait for uniform MMIO dispatch.
//! Shared interrupt state is communicated via [`IrqState`] (lock-free atomic).

pub mod bus;
pub mod intc;
pub mod ram;
pub mod test_finisher;
pub mod uart;
pub mod virtio;
pub mod virtio_blk;

use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering::Relaxed},
};

use crate::{config::Word, error::XResult};

/// MMIO device interface. All offsets are relative to the device's base
/// address.
pub trait Device: Send {
    /// Read `size` bytes at `offset`. Returns the value as a [`Word`].
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    /// Write `size` bytes of `value` at `offset`.
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    /// Called every bus tick to advance device state (e.g. timer, FIFO drain).
    fn tick(&mut self) {}
    /// True if the device is asserting its interrupt line.
    fn irq_line(&self) -> bool {
        false
    }
    /// Notify the device of current IRQ line bitmap (PLIC only).
    fn notify(&mut self, _irq_lines: u32) {}
    /// Soft reset: clear device-level state (VirtIO transport reset).
    fn reset(&mut self) {}
    /// Hard reset: full restore to power-on state (emulator-level reset).
    /// Default delegates to `reset()`. VirtioBlk overrides to restore disk.
    fn hard_reset(&mut self) {
        self.reset();
    }
    /// Return true if the device needs DMA processing after a write.
    fn take_notify(&mut self) -> bool {
        false
    }
    /// Process pending DMA operations with guest-memory access.
    fn process_dma(&mut self, _dma: &mut bus::DmaCtx) {}
    /// Return the current machine timer value (ACLINT only).
    fn mtime(&self) -> Option<u64> {
        None
    }
}

/// RISC-V `mip` bit positions — shared vocabulary between CPU and devices.
#[allow(dead_code)]
pub const SSIP: u64 = 1 << 1; // Supervisor software interrupt pending
/// Machine software interrupt pending.
pub const MSIP: u64 = 1 << 3; // Machine software interrupt pending
/// Supervisor timer interrupt pending.
#[allow(dead_code)]
pub const STIP: u64 = 1 << 5; // Supervisor timer interrupt pending
/// Machine timer interrupt pending.
pub const MTIP: u64 = 1 << 7; // Machine timer interrupt pending
/// Supervisor external interrupt pending.
pub const SEIP: u64 = 1 << 9; // Supervisor external interrupt pending
/// Machine external interrupt pending.
pub const MEIP: u64 = 1 << 11; // Machine external interrupt pending

/// Hardware-wired mip bits managed via IrqState (excludes SSIP/STIP —
/// software-controlled). STIP is managed by Sstc stimecmp comparison.
pub const HW_IP_MASK: Word = (MSIP | MTIP | SEIP | MEIP) as Word;

/// Shared interrupt state between CPU and devices.
#[derive(Clone)]
pub struct IrqState(Arc<AtomicU64>);

impl IrqState {
    /// Create a new state with all bits cleared.
    pub fn new() -> Self {
        Self(Arc::new(AtomicU64::new(0)))
    }
    /// Atomically assert interrupt bit(s).
    pub fn set(&self, bit: u64) {
        self.0.fetch_or(bit, Relaxed);
    }
    /// Atomically deassert interrupt bit(s).
    pub fn clear(&self, bit: u64) {
        self.0.fetch_and(!bit, Relaxed);
    }
    /// Read current interrupt pending bitmap.
    pub fn load(&self) -> u64 {
        self.0.load(Relaxed)
    }
    /// Clear all pending interrupts.
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
