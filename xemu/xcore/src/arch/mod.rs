//! Architecture-specific backends.
//!
//! Compile-time `cfg(riscv)` / `cfg(loongarch)` selects the active backend.
//! Upper-layer seam modules (`cpu/mod.rs`, `isa/mod.rs`,
//! `device/intc/mod.rs`) name the concrete types from here via cfg-gated
//! `pub type` / `pub use` aliases.

#[cfg(loongarch)]
pub mod loongarch;
#[cfg(riscv)]
pub mod riscv;
