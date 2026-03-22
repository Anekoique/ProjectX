use memory_addr::VirtAddr;

use crate::{config::Word, error::XResult};

pub trait CoreOps {
    fn pc(&self) -> VirtAddr;
    fn reset(&mut self) -> XResult;
    /// Run one architectural step: interrupt check → fetch → decode → execute →
    /// retire. Returns `Err` only for true emulator-level errors.
    /// Halt conditions (e.g. breakpoint) are signalled via `halted()`.
    fn step(&mut self) -> XResult;
    /// Whether the core has entered a halt condition (e.g. breakpoint).
    /// The CPU layer uses this to stop the run loop.
    fn halted(&self) -> bool;
    fn halt_ret(&self) -> Word;
}
