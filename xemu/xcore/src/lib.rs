#[macro_use]
extern crate log;
#[macro_use]
extern crate xlogger;

mod config;
mod cpu;
mod error;
mod isa;
mod memory;
mod utils;

pub use cpu::{State, XCPU, with_xcpu};
pub use error::{XError, XResult};
pub use memory::MEMORY;

pub fn init_xcore() -> XResult {
    info!("Hello xcore!");
    with_xcpu!(reset())
}
