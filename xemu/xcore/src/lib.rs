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
    CoreContext, RESET_VECTOR, State, XCPU,
    debug::{Breakpoint, DebugOps},
    with_xcpu,
};
pub use device::uart::Uart;
pub use error::{XError, XResult};
pub use isa::RVReg;

pub fn init_xcore() -> XResult {
    info!("Hello xcore!");
    with_xcpu!(reset())
}
