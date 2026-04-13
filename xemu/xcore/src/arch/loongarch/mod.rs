//! LoongArch architecture backend (stub).
//!
//! Flat topic layout mirroring `arch/riscv/`. Currently only `cpu` and `isa`
//! stubs exist; additional topics (`csr`, `mm`, `trap`, `inst`, `device`) will
//! be added as the LoongArch backend materialises.

pub mod cpu;
pub mod isa;
