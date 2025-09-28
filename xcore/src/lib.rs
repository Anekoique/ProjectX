#[macro_use]
extern crate log;

mod config;
mod cpu;
mod error;
mod isa;
mod memory;
mod utils;

pub use cpu::XCPU;

use crate::error::XResult;

pub fn init_xcore() -> XResult {
    trace!("hello xcore");
    isa::init_decoder();
    XCPU.lock()
        .map_err(|e| {
            panic!("Failed to lock CPU mutex: {}", e);
        })?
        .reset();
    Ok(())
}
