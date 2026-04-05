//! xdb — interactive debugger and non-interactive runner for xemu.
//!
//! In `debug` mode: provides a GDB-style REPL with breakpoints, watchpoints,
//! expression evaluation, and optional difftest against QEMU/Spike.
//! In release mode: boots and runs to completion (firmware or bare-metal).

#[macro_use]
extern crate log;
#[macro_use]
extern crate xcore;
#[macro_use]
extern crate anyhow;

mod cli;
mod cmd;
#[cfg(feature = "difftest")]
pub mod difftest;
mod expr;
mod session;
mod watchpoint;

/// Entry point: initialize, boot, and run the emulator.
pub fn main() -> anyhow::Result<()> {
    init_xdb();

    let config = machine_config()?;
    xcore::init_xcore(config).map_err(|e| anyhow!("XCore Error: {e}"))?;

    run(boot_config()).map_err(|e| anyhow!("XDB Error: {e}"))?;

    if !xcore::with_xcpu(|cpu| cpu.is_exit_normal()) {
        std::process::exit(1);
    }
    Ok(())
}

fn init_xdb() {
    xlogger::init();
    xlogger::set_max_level(option_env!("X_LOG").unwrap_or(""));
    info!("Hello, xdb!");
}

fn machine_config() -> anyhow::Result<xcore::MachineConfig> {
    let env = |n: &str| std::env::var(n).ok().filter(|s| !s.is_empty());
    match env("X_DISK") {
        Some(path) => {
            let disk = std::fs::read(&path)
                .map_err(|e| anyhow!("Failed to read disk image {path}: {e}"))?;
            info!("Loaded disk image: {} ({} bytes)", path, disk.len());
            Ok(xcore::MachineConfig::with_disk(disk))
        }
        None => Ok(xcore::MachineConfig::default()),
    }
}

fn boot_config() -> xcore::BootConfig {
    let env = |n| std::env::var(n).ok().filter(|s| !s.is_empty());

    if let Some((fw, fdt)) = env("X_FW").zip(env("X_FDT")) {
        xcore::BootConfig::Firmware {
            fw,
            fdt,
            kernel: env("X_KERNEL"),
            initrd: env("X_INITRD"),
        }
    } else {
        xcore::BootConfig::Direct {
            file: env("X_FILE"),
        }
    }
}

fn run(config: xcore::BootConfig) -> Result<(), String> {
    xcore::with_xcpu(|cpu| {
        let uart = if cfg!(feature = "debug") {
            xcore::Uart::with_pty()
                .inspect_err(|e| warn!("PTY UART unavailable ({e})"))
                .ok()
        } else if matches!(config, xcore::BootConfig::Firmware { .. }) {
            Some(xcore::Uart::with_stdio())
        } else {
            None
        };

        if let Some(u) = uart {
            cpu.replace_device("uart0", Box::new(u));
        }
        cpu.boot(config)
    })
    .map_err(|e| format!("Boot error: {e}"))?;

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
    let mut sess = session::Session::new();
    loop {
        let line = cli::readline()?;
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match cli::respond(line, &mut sess) {
            Ok(true) => {}
            Ok(false) => return Ok(()),
            Err(err) => print!("{err}"),
        }
    }
}
