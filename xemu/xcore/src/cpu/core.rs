//! Arch-agnostic CPU core trait and boot mode definitions.

use memory_addr::VirtAddr;

use crate::{config::Word, error::XResult};

/// Stable, arch-agnostic hart identifier.
///
/// Wraps a 0-based hart index; mirrors the value of the RISC-V `mhartid`
/// CSR for hart `i` as `HartId(i as u32)`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HartId(pub u32);

impl HartId {
    /// Raw hart index as a machine word (drives `mhartid`, `a0`, etc.).
    #[inline]
    pub fn as_word(self) -> Word {
        self.0 as Word
    }
}

/// Convert a 0-based hart index to a [`HartId`]. Truncates to `u32`.
impl From<usize> for HartId {
    #[inline]
    fn from(v: usize) -> Self {
        HartId(v as u32)
    }
}

/// Boot mode passed to `setup_boot` — arch cores use this to configure
/// registers and halt semantics.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BootMode {
    /// Bare-metal: halt on debug-break, PC at entry.
    Direct,
    /// Firmware: trap debug-break, set arch-specific boot registers.
    Firmware { fdt_addr: usize },
}

/// Arch-specific CPU core interface.
///
/// Implemented by each ISA backend (e.g. `RVCore` for RISC-V).
pub trait CoreOps {
    /// Stable hart identifier (mirrors `mhartid` on RISC-V).
    fn id(&self) -> HartId;
    /// Current program counter.
    fn pc(&self) -> VirtAddr;
    /// Reset all architectural state to power-on defaults.
    fn reset(&mut self) -> XResult;
    /// Configure arch-specific state for the given boot mode.
    fn setup_boot(&mut self, mode: BootMode);
    /// Execute one instruction: fetch → decode → execute → retire.
    fn step(&mut self) -> XResult;
    /// True if the CPU has halted (e.g. `ebreak` in direct mode).
    fn halted(&self) -> bool;
    /// Return value on halt (typically `a0` for exit code).
    fn halt_ret(&self) -> Word;
}
