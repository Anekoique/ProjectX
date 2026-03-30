#[macro_use]
extern crate log;
#[macro_use]
extern crate xcore;

mod cli;
mod cmd;
#[cfg(feature = "difftest")]
pub mod difftest;
mod expr;
mod watchpoint;

pub fn main() {
    init_xdb();
    if let Err(e) = xcore::init_xcore() {
        error!("XCore Error: {e}");
        std::process::exit(1);
    }
    match xcore::Uart::with_pty() {
        Ok(uart) => xcore::with_xcpu(|cpu| cpu.replace_device("uart0", Box::new(uart))),
        Err(e) => warn!("PTY UART unavailable ({e}), TX-only via stdout"),
    }
    if let Err(e) = engine_start() {
        error!("XDB Error: {e}");
        std::process::exit(1);
    }
    if !xcore::with_xcpu(|cpu| cpu.is_exit_normal()) {
        std::process::exit(1);
    }
}

fn init_xdb() {
    xlogger::init();
    xlogger::set_max_level(option_env!("X_LOG").unwrap_or(""));
    info!("Hello, xdb!");
}

fn engine_start() -> Result<(), String> {
    let file = option_env!("X_FILE")
        .filter(|s| !s.is_empty())
        .map(String::from);
    xcore::with_xcpu(|cpu| cpu.load(file).map(|_| ())).map_err(|e| format!("Load error: {e}"))?;

    if cfg!(feature = "debug") {
        xdb_mainloop()
    } else {
        with_xcpu!(run(u64::MAX)).or_else(|e| {
            terminate!(e);
            Ok(())
        })
    }
}

fn xdb_mainloop() -> Result<(), String> {
    let mut watch_mgr = watchpoint::WatchManager::new();
    #[cfg(feature = "difftest")]
    let mut loaded_binary_path: Option<String> =
        std::env::var("X_FILE").ok().filter(|s| !s.is_empty());
    #[cfg(feature = "difftest")]
    let mut diff_harness: Option<difftest::DiffHarness> = None;

    loop {
        let line = cli::readline()?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match cli::respond(
            line,
            &mut watch_mgr,
            #[cfg(feature = "difftest")]
            &mut loaded_binary_path,
            #[cfg(feature = "difftest")]
            &mut diff_harness,
        ) {
            Ok(true) => {}
            Ok(false) => return Ok(()),
            Err(err) => print!("{err}"),
        }
    }
}
