//! Lightweight, cloneable snapshot of architectural state for difftest and
//! debugger register inspection.

/// Arch-specific core context snapshot.
/// Plain data, Clone — safe to pass across crate boundaries.
/// Used by both difftest (state comparison) and debugger (register inspection).
#[derive(Clone)]
pub struct RVCoreContext {
    pub pc: u64,
    pub gprs: Vec<(&'static str, u64)>,
    pub privilege: u64,
    /// Difftest CSR whitelist: (addr, name, comparison_mask, raw_value).
    pub csrs: Vec<(u16, &'static str, u64, u64)>,
    pub word_size: usize,
    pub isa: &'static str,
}
