#[macro_use]
extern crate log;

mod config;
mod cpu;
mod error;
mod isa;
mod memory;

pub fn init_xcore() {
    trace!("hello xcore");
    isa::init_decoder();
}
