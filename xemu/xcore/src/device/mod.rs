//! Emulated hardware devices: memory bus, RAM, UART, interrupt controllers.
//!
//! All devices implement the [`Device`] trait for uniform MMIO dispatch.
//! Shared interrupt state is communicated via [`IrqState`] (lock-free atomic).

pub mod bus;
pub mod irq;
pub mod ram;
pub mod test_finisher;
pub mod uart;
pub mod virtio;
pub mod virtio_blk;

use std::sync::{
    Arc,
    atomic::{
        AtomicBool, AtomicU64,
        Ordering::{Acquire, Release},
    },
};

pub use self::irq::IrqLine;
use crate::{config::Word, error::XResult};

/// MMIO device interface. All offsets are relative to the device's base
/// address. IRQ-raising devices own an [`IrqLine`] handle and signal the
/// PLIC directly (see `device::irq`); the bus does not poll device IRQ state.
pub trait Device: Send {
    /// Read `size` bytes at `offset`. Returns the value as a [`Word`].
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    /// Write `size` bytes of `value` at `offset`.
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    /// Called every bus tick to advance device state (e.g. timer, FIFO drain,
    /// PLIC signal-plane drain).
    fn tick(&mut self) {}
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
///
/// `bits` carries the hardware-level `mip` pending bitmap (MEIP/SEIP/MSIP/
/// MTIP). `ssip_edge` is the edge-triggered SSIP signal raised by ACLINT
/// SSWI. `epoch` is a publish flag bumped by every producer-side change so
/// the CPU can fast-path its step loop when nothing new has arrived —
/// `take_epoch` returns `false` on a hot loop with no pending hardware
/// event, turning `sync_interrupts` into a single `Acquire` swap.
pub struct IrqState {
    bits: Arc<AtomicU64>,
    ssip_edge: Arc<AtomicBool>,
    epoch: Arc<AtomicBool>,
}

impl Clone for IrqState {
    fn clone(&self) -> Self {
        Self {
            bits: self.bits.clone(),
            ssip_edge: self.ssip_edge.clone(),
            epoch: self.epoch.clone(),
        }
    }
}

impl IrqState {
    /// Create a new state with all bits cleared.
    pub fn new() -> Self {
        Self {
            bits: Arc::new(AtomicU64::new(0)),
            ssip_edge: Arc::new(AtomicBool::new(false)),
            epoch: Arc::new(AtomicBool::new(false)),
        }
    }
    /// Atomically assert interrupt bit(s). Publishes via `epoch` (Release).
    pub fn set(&self, bit: u64) {
        self.bits.fetch_or(bit, Release);
        self.epoch.store(true, Release);
    }
    /// Atomically deassert interrupt bit(s). Publishes via `epoch`.
    pub fn clear(&self, bit: u64) {
        self.bits.fetch_and(!bit, Release);
        self.epoch.store(true, Release);
    }
    /// Read current interrupt pending bitmap (Acquire — pairs with `set`).
    pub fn load(&self) -> u64 {
        self.bits.load(Acquire)
    }
    /// Raise the SSIP edge signal. Called by ACLINT SSWI's `setssip`.
    pub fn raise_ssip_edge(&self) {
        self.ssip_edge.store(true, Release);
        self.epoch.store(true, Release);
    }
    /// Consume and return the SSIP edge signal.
    pub fn take_ssip_edge(&self) -> bool {
        self.ssip_edge.swap(false, Acquire)
    }
    /// Swap the publish epoch to `false`. Returns `true` iff a producer
    /// raised the flag since the last call — a fast-path gate for the CPU
    /// step loop. `Acquire` pairs with every producer's `Release` store.
    pub fn take_epoch(&self) -> bool {
        self.epoch.swap(false, Acquire)
    }
    /// Clear all pending interrupts. Forces the next `take_epoch` to return
    /// `true` so the CPU observes the cleared state.
    pub fn reset(&self) {
        self.bits.store(0, Release);
        self.ssip_edge.store(false, Release);
        self.epoch.store(true, Release);
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
