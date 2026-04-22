//! Shared utilities: bit extraction, sign extension, and instruction table
//! macro.

pub(crate) mod bit;
mod logo;
mod riscv;

pub use bit::{bit_u32, sext_u32, sext_word};
pub(crate) use logo::print_logo;
