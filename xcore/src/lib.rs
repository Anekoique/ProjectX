#[macro_use]
extern crate log;

mod config;
mod error;
mod memory;

pub fn hello() {
    trace!("hello xcore");
}
