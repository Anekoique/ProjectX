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
    // ── Breakpoint management ──

    fn add_breakpoint(&mut self, addr: usize) -> u32;
    fn remove_breakpoint(&mut self, id: u32) -> bool;
    fn list_breakpoints(&self) -> &[Breakpoint];
    fn set_skip_bp(&mut self);

    // ── Read-only inspection ──

    /// Read named register: "pc", "a0", "sp", "mstatus", "privilege", etc.
    fn read_register(&self, name: &str) -> Option<u64>;

    /// All registers as (name, value) pairs.
    fn dump_registers(&self) -> Vec<(&'static str, u64)>;

    /// Read physical memory (RAM only, side-effect-free).
    fn read_memory(&self, paddr: usize, size: usize) -> XResult<u64>;

    /// Fetch raw instruction at physical address.
    fn fetch_inst(&self, paddr: usize) -> XResult<u32>;

    /// Decode raw instruction to mnemonic string.
    fn disasm_raw(&self, raw: u32) -> String;
}
