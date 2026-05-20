//! `scause` classifier. Split out from `mod.rs` for readability — the
//! naked trap entry stays focused on save/restore, the dispatcher pattern-
//! matches on this enum, and future H-extension cause additions land here.

/// Classified `scause`. The top bit selects interrupt-vs-exception; the
/// remaining bits carry the cause code.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Cause {
    /// Asynchronous interrupt (top bit of `scause` set).
    Interrupt(usize),
    /// Synchronous exception (top bit of `scause` clear).
    Exception(usize),
}

/// Synchronous-exception cause code emitted by `ebreak` in HS-mode.
pub const EXCEPTION_BREAKPOINT: usize = 3;

/// Mask for the interrupt-vs-exception selector bit of `scause`
/// (top bit on the running XLEN).
const INTERRUPT_BIT: usize = 1 << (usize::BITS - 1);

/// Decode `scause` into [`Cause::Interrupt`] / [`Cause::Exception`].
pub const fn classify(scause: usize) -> Cause {
    if scause & INTERRUPT_BIT == 0 {
        Cause::Exception(scause)
    } else {
        Cause::Interrupt(scause & !INTERRUPT_BIT)
    }
}
