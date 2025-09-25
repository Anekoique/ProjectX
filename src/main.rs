#[macro_use]
extern crate log;

pub fn main() {
    xdb::init_sdb();
    trace!("Hello, xdb!");
    xcore::hello();
}
