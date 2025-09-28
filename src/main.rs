#[macro_use]
extern crate log;

pub fn main() {
    xdb::init_xdb();
    xcore::init_xcore()
        .map_err(|e| error!("XCore Error: {e}"))
        .ok();
    xdb::xdb_mainloop()
        .map_err(|e| error!("XDB Error: {e}"))
        .ok();
    xcore::XCPU
        .lock()
        .map(|cpu| cpu.is_exit_status_bad())
        .unwrap_or(false)
        .then(|| std::process::exit(1));
}
