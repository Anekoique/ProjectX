//! RISC-V architecture backend.
//!
//! Sub-modules: `cpu` (RVCore pipeline — CSR file, MMU, PMP, trap logic,
//! instruction handlers) and `device` (RISC-V-specific interrupt controllers:
//! ACLINT, PLIC). The ISA decoder lives at `crate::isa::riscv`.

pub mod cpu;
pub mod device;
