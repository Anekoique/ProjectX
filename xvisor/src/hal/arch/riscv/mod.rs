//! RISC-V backend for the HAL: boot entry, per-hart state, CSR helpers, trap
//! layout.

pub mod boot;
pub mod cpu;
pub mod csr;
pub mod trap;

pub use cpu::{DTB_ADDR, MAX_HARTS, PER_CPU, PerCpu, STACK_SIZE_PER_HART, percpu};
