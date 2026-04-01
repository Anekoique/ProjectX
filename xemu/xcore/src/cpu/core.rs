//! Arch-agnostic CPU core trait and boot mode definitions.

use memory_addr::VirtAddr;

use crate::{config::Word, device::bus::Bus, error::XResult};

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
    /// Current program counter.
    fn pc(&self) -> VirtAddr;
    /// Shared reference to the memory bus.
    fn bus(&self) -> &Bus;
    /// Mutable reference to the memory bus.
    fn bus_mut(&mut self) -> &mut Bus;
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
