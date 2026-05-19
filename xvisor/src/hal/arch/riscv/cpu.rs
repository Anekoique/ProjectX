//! Per-hart state slot reached via `tp`, plus the `DTB_ADDR` capture.

use core::sync::atomic::AtomicUsize;

/// Maximum number of harts supported in this build. Single-hart for now;
/// secondary harts spin in OpenSBI HSM until raised explicitly.
pub const MAX_HARTS: usize = 1;

/// Per-hart stack size. Sized to absorb trap reentry plus deep format-print
/// chains without overflow.
pub const STACK_SIZE_PER_HART: usize = 64 * 1024;

/// DTB pointer captured by `_start` from OpenSBI's `a1` register, before any
/// Rust code that could clobber it runs.
pub static DTB_ADDR: AtomicUsize = AtomicUsize::new(0);

/// Per-hart slot reached through `tp`. Field order is part of the binding
/// contract — `boot.rs` populates these by offset.
#[repr(C, align(64))]
pub struct PerCpu {
    /// SBI-reported hartid for this hart.
    pub hartid: usize,
    /// Top-of-stack pointer for this hart.
    pub stack_top: *mut u8,
    /// Reserved padding so `PerCpu` rounds to a power-of-two size for cheap
    /// future multi-hart indexing.
    _reserved: [usize; 6],
}

// SAFETY: `PerCpu` is owned by exactly one hart at a time via `tp`, with no
// cross-hart sharing in the current single-hart configuration.
unsafe impl Sync for PerCpu {}

impl PerCpu {
    /// Construct a zero-initialised slot. Real values land via `boot.rs`.
    pub const fn new() -> Self {
        Self {
            hartid: 0,
            stack_top: core::ptr::null_mut(),
            _reserved: [0; 6],
        }
    }
}

impl Default for PerCpu {
    fn default() -> Self {
        Self::new()
    }
}

/// Static `PerCpu` array — one slot per supported hart.
pub static mut PER_CPU: [PerCpu; MAX_HARTS] = [const { PerCpu::new() }; MAX_HARTS];

/// Return a reference to the current hart's `PerCpu` via `tp`.
///
/// # Safety
///
/// `tp` must have been set by `_start` to point inside `PER_CPU` before any
/// caller invokes this. Once set, `tp` is never reassigned at runtime.
#[inline]
pub fn percpu() -> &'static PerCpu {
    let tp: usize;
    // SAFETY: reading `tp` is side-effect-free; the value is established by
    // `_start` before any Rust code runs.
    unsafe {
        core::arch::asm!("mv {}, tp", out(reg) tp, options(nomem, nostack, preserves_flags));
    }
    // SAFETY: `tp` points to a `PerCpu` inside `PER_CPU`, alive for `'static`.
    unsafe { &*(tp as *const PerCpu) }
}

const _: () = assert!(core::mem::size_of::<PerCpu>().is_power_of_two());
const _: () = assert!(STACK_SIZE_PER_HART == 64 * 1024);
