#[macro_use]
extern crate log;
#[macro_use]
extern crate xcore;

mod cli;
mod cmd;
mod expr;
mod watchpoint;

use watchpoint::WatchManager;

pub fn main() {
    crate::init_xdb();
    if let Err(e) = xcore::init_xcore() {
        error!("XCore Error: {e}");
        std::process::exit(1);
    }
    if let Err(e) = crate::xdb_mainloop() {
        error!("XDB Error: {e}");
        std::process::exit(1);
    }
    if !xcore::with_xcpu(|cpu| cpu.is_exit_normal()) {
        std::process::exit(1);
    }
}

pub fn init_xdb() {
    xlogger::init();
    xlogger::set_max_level(option_env!("X_LOG").unwrap_or(""));
    info!("Hello, xdb!");
}

pub fn xdb_mainloop() -> Result<(), String> {
    let file = option_env!("X_FILE")
        .filter(|s| !s.is_empty())
        .map(String::from);
    // Load file if provided (both batch and interactive modes)
    xcore::with_xcpu(|cpu| cpu.load(file).map(|_| ())).map_err(|e| format!("Load error: {e}"))?;

    let mut watch_mgr = WatchManager::new();

    match option_env!("X_BATCH") {
        Some("y") => with_xcpu!(run(u64::MAX)).or_else(|e| {
            terminate!(e);
            Ok(())
        }),
        _ => loop {
            let line = cli::readline()?;
            let line = line.trim();
            if line.is_empty() {
                continue;
            }

            match cli::respond(line, &mut watch_mgr) {
                Ok(true) => {}
                Ok(false) => return Ok(()),
                Err(err) => print!("{err}"),
            }
        },
    }
}
