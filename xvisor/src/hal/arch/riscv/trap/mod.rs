//! HS-mode trap entry, frame layout, and Rust dispatcher.
//!
//! The trap-entry assembly lives in [`trap.S`](./trap.S) and is included
//! via `global_asm!`; it allocates a 288-byte [`TrapFrame`] on the current
//! kernel stack, saves `x1..x31` and the CSR shadows (`sepc`, `scause`,
//! `stval`, `sstatus`), dispatches into [`trap_handler`], restores
//! `sepc` + `sstatus` (and `x1..x31`), and executes `sret`. The `x0` slot
//! is written zero so `frame.regs[rd]` is safe for any encoded `rd` once
//! future guest-exit emulation needs to index by destination register.
//!
//! `sscratch` is reserved for the future trap-entry SP swap (VS-mode →
//! HS-mode trap entry, where the guest's `sp` must not be reused) and is
//! left zero this iteration — `trap_entry` reuses the caller's `sp`
//! because HS-mode self-traps run on the hypervisor's own stack.

use core::{fmt::Write, mem::offset_of};

use crate::hal::platform::{
    halt::{HaltCode, terminate},
    uart::writer,
};

pub mod cause;
use cause::{Cause, EXCEPTION_BREAKPOINT, classify};

core::arch::global_asm!(include_str!("trap.S"));

unsafe extern "C" {
    /// HS-mode trap vector defined in `trap.S`. Installed in `stvec` by
    /// `boot.rs` (Direct mode; the lower two bits of the `stvec` write are
    /// zero). Not callable from Rust — the address is taken once during
    /// boot and written into the CSR; the hardware invokes the symbol on
    /// trap entry with the trap-time GPR state the assembly preserves.
    pub(crate) fn trap_entry();
}

/// Trap context. `regs[0]` is the `x0` slot and is written zero by
/// `trap_entry` so `frame.regs[rd]` indexing is safe for any encoded `rd`.
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

const _: () = assert!(core::mem::size_of::<TrapFrame>() == 36 * core::mem::size_of::<usize>());
const _: () = assert!(offset_of!(TrapFrame, regs) == 0);
const _: () = assert!(offset_of!(TrapFrame, sepc) == 32 * 8);
const _: () = assert!(offset_of!(TrapFrame, scause) == 33 * 8);
const _: () = assert!(offset_of!(TrapFrame, stval) == 34 * 8);
const _: () = assert!(offset_of!(TrapFrame, sstatus) == 35 * 8);

/// Rust trap dispatcher invoked by `trap_entry` with `a0 = &mut TrapFrame`.
///
/// Logs the trap, recovers from `ebreak` by advancing `sepc` past the
/// instruction, and halts the machine on any other cause (this iteration
/// services synchronous breakpoints only).
#[unsafe(no_mangle)]
pub extern "C" fn trap_handler(frame: &mut TrapFrame) {
    let _ = writeln!(
        writer(),
        "xvisor: trap cause=0x{:x} sepc=0x{:x} stval=0x{:x}",
        frame.scause,
        frame.sepc,
        frame.stval,
    );

    match classify(frame.scause) {
        Cause::Exception(EXCEPTION_BREAKPOINT) => {
            frame.sepc = frame.sepc.wrapping_add(instruction_width(frame.sepc));
        }
        _ => terminate(HaltCode::Failure),
    }
}

/// Width (in bytes) of the RISC-V instruction starting at `pc`. The C
/// extension means a synchronous trap on `ebreak` might be advanced by 2
/// (`c.ebreak`) or 4 (`ebreak`); decoding the leading bits at `pc` is the
/// only safe way to recover the trap PC for either encoding.
fn instruction_width(pc: usize) -> usize {
    // SAFETY: `pc` is the trap-faulting PC reported by hardware; the
    // instruction byte at that address is by construction live, executable,
    // and 2-byte aligned. We read it as `u16` only for the encoding bits.
    let first_halfword = unsafe { (pc as *const u16).read() };
    // RISC-V instruction-length convention: bits [1:0] == 0b11 → 32-bit
    // (or larger; xvisor's binary contains only 16/32-bit encodings).
    if first_halfword & 0b11 == 0b11 { 4 } else { 2 }
}
