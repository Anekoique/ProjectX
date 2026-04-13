//! ISA-specific instruction decoder and register definitions.
//!
//! The per-arch decoder, instruction tags, and register enums live under
//! `isa/<arch>/`; this top-level module re-exports them cfg-gated on the
//! active arch. The neutral pest grammar lives in `isa/instpat/`.

#[cfg(riscv)]
mod riscv;
#[cfg(riscv)]
pub use riscv::{DECODER, DecodedInst, IMG, InstFormat, InstKind, RVReg};

#[cfg(loongarch)]
pub use crate::arch::loongarch::isa::IMG;
