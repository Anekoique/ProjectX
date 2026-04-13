//! Shared utilities: bit extraction, sign extension, and instruction table
//! macro.

pub(crate) mod bit;
mod riscv;

pub use bit::{bit_u32, sext_u32, sext_word};
