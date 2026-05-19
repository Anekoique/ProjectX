//! Trap-frame layout and entry symbol. The frame is fixed by `#[repr(C)]`
//! field order so future trap-entry assembly can index by `offset_of!`.
//!
//! `sscratch` is reserved for the trap-entry SP swap (the standard
//! HS-mode pattern: trap entry `csrrw sp, sscratch, sp` to land on the
//! hypervisor's stack, then save the user GPRs into the frame). It is
//! left zero until the trap entry path is implemented.

/// Trap context. `regs[0]` is the x0 slot and is preserved zero so
/// `frame.regs[rd]` indexing works for any encoded `rd` value.
#[repr(C)]
pub struct TrapFrame {
    /// General-purpose registers x0..x31. `regs[0]` is x0 (always zero).
    pub regs: [usize; 32],
    /// Trap PC.
    pub sepc: usize,
    /// Trap cause.
    pub scause: usize,
    /// Trap value (faulting address / illegal instruction encoding / …).
    pub stval: usize,
    /// Saved status register at trap entry.
    pub sstatus: usize,
}

// `trap_entry` (the HS-mode trap entry assembly symbol) lands when trap
// handling arrives; `boot.rs` currently points `stvec` at a `wfi` parking
// pad instead. The `TrapFrame` field order above is the binding contract
// future trap-entry assembly will index by `offset_of!`.

const _: () = assert!(core::mem::size_of::<TrapFrame>() == 36 * core::mem::size_of::<usize>());
