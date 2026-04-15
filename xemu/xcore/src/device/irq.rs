//! Direct device → PLIC interrupt signaling.
//!
//! A device holds an [`IrqLine`] for its source and calls `raise` / `lower`
//! the moment an event occurs. The signal plane is a lock-free atomic
//! bitmap inside the PLIC; the CPU thread drains it at the next bus tick via
//! `Plic::tick`. See `docs/archived/fix/directIrq/02_PLAN.md` §Async Posture.
//!
//! Memory ordering (directIrq 01_PLAN I-D14):
//! - Producers publish with `Release`; the drain acquires with `Acquire`. This
//!   establishes happens-before from the device's pre-raise state to the CPU's
//!   post-drain observation.
//! - The drain swaps `pending_raises` first (Acquire) as an epoch gate — when
//!   no producer has stored since the last drain, the drain returns in one
//!   atomic operation with zero per-source work (event-driven property).

use std::sync::{
    Arc,
    atomic::{
        AtomicBool, AtomicU32,
        Ordering::{Acquire, Release},
    },
};

/// Shared atomic signal plane: one bit per source for level, plus a
/// single-bit epoch flag that gates the drain. The width pins
/// `NUM_SRC <= 32` (directIrq I-D12 / C-12).
pub struct PlicSignals {
    level: AtomicU32,
    pending_raises: AtomicBool,
}

impl PlicSignals {
    pub fn new() -> Self {
        Self {
            level: AtomicU32::new(0),
            pending_raises: AtomicBool::new(false),
        }
    }

    /// Last store on any raise path — `Release` pairs with the drain's
    /// `Acquire` swap.
    #[inline]
    fn notify(&self) {
        self.pending_raises.store(true, Release);
    }

    /// Drain the signal plane. Returns `None` if no producer has stored
    /// since the last drain — the fast path is one `Acquire` swap and no
    /// per-source work (I-D14). Returns `Some(level)` otherwise.
    pub fn drain(&self) -> Option<u32> {
        self.pending_raises
            .swap(false, Acquire)
            .then(|| self.level.load(Acquire))
    }

    /// Clear all plane state. Sets `pending_raises = true` so the next drain
    /// is forced to run and de-assert IRQ lines (directIrq F-6).
    pub fn reset(&self) {
        self.level.store(0, Release);
        self.pending_raises.store(true, Release);
    }
}

impl Default for PlicSignals {
    fn default() -> Self {
        Self::new()
    }
}

/// Arch-neutral handle a device holds to signal its PLIC source directly.
/// Clones alias the same source (directIrq I-D7 coalesce).
#[derive(Clone)]
pub struct IrqLine {
    signals: Arc<PlicSignals>,
    bit: u32,
}

impl IrqLine {
    /// Construct for source `src`. `src` must be in `1..=31` (source 0 is
    /// reserved per PLIC spec; 31 is the highest bit in the u32 bitmap /
    /// I-D12).
    pub fn new(signals: Arc<PlicSignals>, src: u32) -> Self {
        assert!(
            (1..=31).contains(&src),
            "IrqLine src must be in 1..=31, got {src}"
        );
        Self {
            signals,
            bit: 1u32 << src,
        }
    }

    /// Drive the line high. Idempotent (I-D2).
    pub fn raise(&self) {
        self.signals.level.fetch_or(self.bit, Release);
        self.signals.notify();
    }

    /// Drive the line low. Idempotent (I-D3).
    pub fn lower(&self) {
        self.signals.level.fetch_and(!self.bit, Release);
        self.signals.notify();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn raise_lower_roundtrip() {
        let plane = Arc::new(PlicSignals::new());
        let line = IrqLine::new(plane.clone(), 10);
        line.raise();
        assert_eq!(plane.drain().unwrap(), 1 << 10);
        line.lower();
        assert_eq!(plane.drain().unwrap(), 0);
    }

    #[test]
    fn drain_with_no_raise_is_none() {
        let plane = PlicSignals::new();
        assert!(plane.drain().is_none());
        assert!(plane.drain().is_none());
    }

    #[test]
    fn reset_forces_next_drain_and_clears_bits() {
        let plane = Arc::new(PlicSignals::new());
        IrqLine::new(plane.clone(), 5).raise();
        plane.reset();
        assert_eq!(plane.drain().unwrap(), 0);
    }

    #[test]
    fn clones_alias_same_source() {
        let plane = Arc::new(PlicSignals::new());
        let a = IrqLine::new(plane.clone(), 7);
        let b = a.clone();
        a.raise();
        b.lower();
        assert_eq!(plane.drain().unwrap(), 0);
    }

    #[test]
    #[should_panic(expected = "IrqLine src must be in 1..=31")]
    fn src_zero_rejected() {
        IrqLine::new(Arc::new(PlicSignals::new()), 0);
    }

    #[test]
    #[should_panic(expected = "IrqLine src must be in 1..=31")]
    fn src_out_of_range_rejected() {
        IrqLine::new(Arc::new(PlicSignals::new()), 32);
    }
}
