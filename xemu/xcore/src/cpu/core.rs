use std::sync::{Arc, Mutex};

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

pub trait CoreOps {
    fn pc(&self) -> VirtAddr;
    fn bus(&self) -> &Arc<Mutex<Bus>>;
    fn reset(&mut self) -> XResult;
    /// Configure arch-specific state for the given boot mode.
    fn setup_boot(&mut self, mode: BootMode);
    fn step(&mut self) -> XResult;
    fn halted(&self) -> bool;
    fn halt_ret(&self) -> Word;
}
