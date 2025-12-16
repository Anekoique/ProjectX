#[macro_use]
extern crate log;

mod config;
mod cpu;
mod error;
mod isa;
mod memory;
mod utils;

pub use cpu::{State, XCPU};
pub use error::{XError, XResult};
pub use memory::MEMORY;

pub fn init_xcore() -> XResult {
    trace!("hello xcore");
    with_xcpu!(reset())
}
