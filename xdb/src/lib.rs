#[macro_use]
extern crate log;

mod cli;
mod cmd;
mod logger;

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
            Ok(quit) => {
                if quit {
                    return Ok(());
                }
            }
            Err(err) => print!("{err}"),
        }
    }
}
