#[macro_use]
extern crate log;
#[macro_use]
extern crate xcore;

mod cli;
mod cmd;
mod logger;

pub fn main() {
    crate::init_xdb();
    xcore::init_xcore()
        .map_err(|e| error!("XCore Error: {e}"))
        .ok();
    crate::xdb_mainloop()
        .map_err(|e| error!("XDB Error: {e}"))
        .ok();
    xcore::XCPU
        .lock()
        .map(|cpu| !cpu.is_exit_normal())
        .unwrap_or(false)
        .then(|| std::process::exit(1));
}

pub fn init_xdb() {
    crate::logger::init();
    crate::logger::set_max_level(option_env!("X_LOG").unwrap_or(""));
    trace!("Hello, xdb!");
}

pub fn xdb_mainloop() -> Result<(), String> {
    loop {
        let line = cli::readline()?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        match cli::respond(line) {
            Ok(_continue) => {
                if !_continue {
                    return Ok(());
                }
            }
            Err(err) => print!("{err}"),
        }
    }
}
