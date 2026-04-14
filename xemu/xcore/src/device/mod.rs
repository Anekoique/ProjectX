//! Emulated hardware devices: memory bus, RAM, UART, interrupt controllers.
//!
//! All devices implement the [`Device`] trait for uniform MMIO dispatch.
//! Shared interrupt state is communicated via [`IrqState`] (lock-free atomic).

pub mod bus;
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

/// MMIO register-map helper. Two variant forms — fixed offsets and strided
/// array regions — may appear alone or together:
///
/// - `Name = offset`       — fixed single-offset register; matches `offset ==
///   N`.
/// - `Name[stride, end]`   — strided array region, matches `offset < end` and
///   yields `{ index = offset / stride, sub = offset % stride }`. Callers must
///   bound-check `index` against runtime element count and handle sub-word
///   offsets (`sub != 0`).
///
/// When mixing both kinds, separate the two sections with `;` (fixed first,
/// then arrays). Fixed variants are matched before array regions; array ranges
/// must be disjoint.
macro_rules! mmio_regs {
    // Array-only form.
    (
        $vis:vis enum $Reg:ident {
            $( $aname:ident [ $astride:expr , $aend:expr ] ),+ $(,)?
        }
    ) => {
        $crate::device::mmio_regs!(@emit $vis $Reg { } { $( $aname [$astride, $aend] ),+ });
    };

    // Fixed-only form (original syntax).
    (
        $vis:vis enum $Reg:ident {
            $( $fname:ident = $foff:expr ),+ $(,)?
        }
    ) => {
        $crate::device::mmio_regs!(@emit $vis $Reg { $( $fname = $foff ),+ } { });
    };

    // Fixed + array regions (two sections separated by `;`).
    (
        $vis:vis enum $Reg:ident {
            $( $fname:ident = $foff:expr ),+ $(,)?
            ;
            $( $aname:ident [ $astride:expr , $aend:expr ] ),+ $(,)?
        }
    ) => {
        $crate::device::mmio_regs!(
            @emit $vis $Reg
            { $( $fname = $foff ),+ }
            { $( $aname [$astride, $aend] ),+ }
        );
    };

    // Internal emitter — not part of the public macro surface.
    (@emit $vis:vis $Reg:ident
        { $( $fname:ident = $foff:expr ),* }
        { $( $aname:ident [ $astride:expr , $aend:expr ] ),* }
    ) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        $vis enum $Reg {
            $( $fname, )*
            $( $aname { index: usize, sub: usize }, )*
        }

        impl $Reg {
            #[allow(dead_code)]
            fn decode(offset: usize) -> Option<Self> {
                $( if offset == $foff { return Some(Self::$fname); } )*
                $( if offset < $aend {
                    return Some(Self::$aname {
                        index: offset / $astride,
                        sub:   offset % $astride,
                    });
                } )*
                None
            }
        }
    };
}
pub(crate) use mmio_regs;
