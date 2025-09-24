#[macro_use]
extern crate xdb;

pub fn main() {
    xdb::init_sdb();
    xcore::hello();
}
