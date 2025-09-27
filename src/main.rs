#[macro_use]
extern crate log;

pub fn main() {
    xdb::init_xdb();
    xcore::init_xcore();
    xdb::xdb_mainloop()
        .map_err(|e| error!("XDB Error: {e}"))
        .ok();
}
