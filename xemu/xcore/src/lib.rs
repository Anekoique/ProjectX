//! # xcore — RISC-V emulator core library
//!
//! This crate implements a cycle-accurate RISC-V emulator supporting RV32/RV64
//! with the IMAFDCZicsr extension set. It provides:
//!
//! - **CPU**: instruction fetch–decode–execute pipeline with privilege modes
//!   (M/S/U)
//! - **ISA**: two-level instruction decoder with compressed (RV-C) support
//! - **Memory**: MMU with Sv32/Sv39 page translation, TLB, and PMP
//! - **Devices**: UART (NS16550A), ACLINT timer, PLIC interrupt controller
//! - **Boot**: OpenSBI firmware boot chain for Linux kernel startup
//!
//! ## Architecture
//!
//! The crate is ISA-generic at compile time via `cfg(riscv)` /
//! `cfg(loongarch)`. Word width (`u32` or `u64`) is selected by `cfg(isa32)` /
//! `cfg(isa64)`. Upper layers ([`BootConfig`], [`DebugOps`]) use trait-based
//! abstractions and never expose ISA-specific types.

#![feature(bool_to_result)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate xlogger;

mod config;
mod cpu;
pub(crate) mod device;
mod error;
mod isa;
mod utils;

pub use cpu::{
    BootConfig, CoreContext, RESET_VECTOR, State, XCPU,
    debug::{Breakpoint, DebugOps},
    with_xcpu,
};
pub use device::uart::Uart;
pub use error::{XError, XResult};

/// Initialize the emulator core and reset the CPU to its initial state.
pub fn init_xcore() -> XResult {
    info!("Hello xcore!");
    with_xcpu!(reset())
}
