//! Arch-agnostic debug inspection trait: breakpoints, register/memory reads,
//! and instruction disassembly.

use crate::error::XResult;

/// Breakpoint with stable user-visible ID.
#[derive(Debug, Clone)]
pub struct Breakpoint {
    pub id: u32,
    pub addr: usize,
}

/// Unified debug facade — breakpoint management + read-only inspection.
/// Implemented per-arch, called through CPU pass-through methods.
pub trait DebugOps: super::CoreOps {
    /// Insert a breakpoint at `addr`, returning its stable ID.
    fn add_breakpoint(&mut self, addr: usize) -> u32;
    /// Remove breakpoint by ID. Returns `true` if found.
    fn remove_breakpoint(&mut self, id: u32) -> bool;
    /// List all active breakpoints.
    fn list_breakpoints(&self) -> &[Breakpoint];
    /// Skip breakpoint check for the next step (used after hitting one).
    fn set_skip_bp(&mut self);

    // ── State snapshot ──

    /// Capture architectural state as a lightweight, cloneable context.
    /// Used by difftest (state comparison) and debugger (`info reg`).
    fn context(&self) -> super::CoreContext;

    // ── Read-only inspection ──

    /// Read named register (descriptor-aware for shadow CSRs).
    fn read_register(&self, name: &str) -> Option<u64>;

    /// Read physical memory (RAM only, side-effect-free).
    fn read_memory(&self, paddr: usize, size: usize) -> XResult<u64>;

    /// Fetch raw instruction at physical address.
    fn fetch_inst(&self, paddr: usize) -> XResult<u32>;

    /// Decode raw instruction to mnemonic string.
    fn disasm_raw(&self, raw: u32) -> String;

    /// Return the byte width of a raw instruction (e.g. 2 for compressed, 4 for
    /// standard).
    fn inst_size(&self, raw: u32) -> usize;
}
