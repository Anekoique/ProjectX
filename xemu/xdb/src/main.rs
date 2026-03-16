#[macro_use]
extern crate log;
#[macro_use]
extern crate xcore;

mod cli;
mod cmd;

pub fn main() {
    crate::init_xdb();
    xcore::init_xcore()
        .map_err(|e| error!("XCore Error: {e}"))
        .unwrap();
    crate::xdb_mainloop()
        .map_err(|e| error!("XDB Error: {e}"))
        .unwrap();
    xcore::XCPU
        .lock()
        .map(|cpu| !cpu.is_exit_normal())
        .unwrap_or(false)
        .then(|| std::process::exit(1));
}

pub fn init_xdb() {
    xlogger::init();
    xlogger::set_max_level(option_env!("X_LOG").unwrap_or(""));
    trace!("Hello, xdb!");
}

pub fn xdb_mainloop() -> Result<(), String> {
    let file = option_env!("X_FILE")
        .filter(|s| !s.is_empty())
        .map(String::from);
    match option_env!("X_MODE") {
        Some("y") => with_xcpu!(load(file)?.run(u32::MAX)).or_else(|e| {
            terminate!(e);
            Ok(())
        }),
        _ => loop {
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
        },
    }
}
