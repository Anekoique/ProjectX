#[macro_use]
extern crate log;

mod config;
mod error;
mod isa;
mod memory;

pub fn init_xcore() {
    trace!("hello xcore");
}
