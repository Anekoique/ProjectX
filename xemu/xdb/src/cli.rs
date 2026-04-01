//! CLI front-end: clap command parsing, GDB-style `x/Nf` preprocessing, and
//! REPL input handling.

use std::{io::Write, sync::OnceLock};

use clap::{Parser, Subcommand};
use regex::Regex;

use crate::{cmd::*, session::Session};

#[derive(Debug, Parser)]
#[command(multicall = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Step N instructions
    #[command(alias = "s")]
    Step {
        #[arg(default_value_t = 1)]
        count: u64,
    },
    /// Continue execution
    #[command(alias = "c")]
    Continue,
    /// Examine memory or disassemble (use x/Ni or x/Nx syntax)
    #[command(alias = "x")]
    Examine {
        /// Format: i(nstr), x(hex), b(yte)
        #[arg(short = 'f', default_value = "i")]
        format: char,
        /// Count
        #[arg(short = 'n', default_value_t = 1)]
        count: usize,
        /// Physical address (hex, default = current pc)
        addr: Option<String>,
    },
    /// Set breakpoint at address
    #[command(alias = "b")]
    Break {
        /// Physical address (hex)
        addr: String,
    },
    /// Delete breakpoint by ID
    #[command(name = "bd")]
    BreakDelete { id: u32 },
    /// List breakpoints
    #[command(name = "bl")]
    BreakList,
    /// Watch expression for value change
    #[command(alias = "w")]
    Watch {
        /// Expression to watch
        expr: Vec<String>,
    },
    /// Delete watchpoint by ID
    #[command(name = "wd")]
    WatchDelete { id: u32 },
    /// List watchpoints
    #[command(name = "wl")]
    WatchList,
    /// Evaluate and print expression
    #[command(alias = "p")]
    Print {
        /// Expression to evaluate
        expr: Vec<String>,
    },
    /// Show register info
    Info {
        /// What to show: "reg" or "r"
        what: String,
        /// Optional register name
        name: Option<String>,
    },
    /// Load a binary file into memory
    #[command(alias = "l")]
    Load {
        /// Path to the binary file
        file: String,
    },
    /// Reset CPU
    #[command(alias = "r")]
    Reset,
    /// Difftest control
    #[cfg(feature = "difftest")]
    #[command(name = "dt")]
    Difftest {
        #[command(subcommand)]
        subcmd: DtSubcommand,
    },
    /// Exit xdb
    #[command(aliases = ["quit", "q", "e"])]
    Exit,
}

#[cfg(feature = "difftest")]
#[derive(Debug, Subcommand)]
/// Difftest subcommands.
pub enum DtSubcommand {
    /// Attach difftest backend (requires loaded binary)
    Attach {
        #[arg(default_value = "qemu")]
        backend: String,
    },
    /// Detach difftest
    Detach,
    /// Show difftest status
    Status,
}

fn preprocess_line(line: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^x/(?:(\d+))?([ixb])?\s*(.*)").unwrap());

    let line = line.trim();
    if let Some(caps) = re.captures(line) {
        let n = caps.get(1).map_or("1", |m| m.as_str());
        let f = caps.get(2).map_or("i", |m| m.as_str());
        let rest = caps.get(3).map_or("", |m| m.as_str());
        format!("x -f {f} -n {n} {rest}")
    } else {
        line.to_string()
    }
}

/// Parse and execute one debugger command line.
pub fn respond(line: &str, sess: &mut Session) -> Result<bool, String> {
    let line = preprocess_line(line);
    let args = shlex::split(&line).ok_or("error: Invalid quoting")?;
    let cli = Cli::try_parse_from(args).map_err(|e| e.to_string())?;
    match cli.command {
        Commands::Step { count } => cmd_step(count, sess),
        Commands::Continue => cmd_continue(sess),
        Commands::Examine {
            format,
            count,
            addr,
        } => cmd_examine(format, count, addr),
        Commands::Break { addr } => cmd_break(&addr),
        Commands::BreakDelete { id } => cmd_break_delete(id),
        Commands::BreakList => cmd_break_list(),
        Commands::Watch { expr } => cmd_watch(&expr.join(" "), &mut sess.watch),
        Commands::WatchDelete { id } => {
            let msg = if sess.watch.remove(id) {
                "Deleted"
            } else {
                "No"
            };
            println!("{msg} watchpoint #{id}");
            Ok(())
        }
        Commands::WatchList => {
            cmd_watch_list(&sess.watch);
            Ok(())
        }
        Commands::Print { expr } => cmd_print(&expr.join(" ")),
        Commands::Info { what, name } => cmd_info(&what, name.as_deref()),
        Commands::Load { ref file } => {
            #[cfg(feature = "difftest")]
            {
                sess.loaded_path = Some(file.clone());
            }
            cmd_load(file.clone())
        }
        Commands::Reset => cmd_reset(),
        #[cfg(feature = "difftest")]
        Commands::Difftest { subcmd } => match subcmd {
            DtSubcommand::Attach { backend } => cmd_dt_attach(&backend, sess),
            DtSubcommand::Detach => cmd_dt_detach(sess),
            DtSubcommand::Status => {
                cmd_dt_status(sess);
                Ok(())
            }
        },
        Commands::Exit => {
            println!("Exiting ...");
            return Ok(false);
        }
    }
    .map(|_| true)
    .or_else(|e| {
        if let xcore::XError::DebugBreak(pc) = e {
            xcore::with_xcpu(|cpu| cpu.set_skip_bp());
            println!("Breakpoint at {pc:#x}");
            return Ok(true);
        }
        terminate!(e);
        Ok(true)
    })
}

/// Print prompt and read one line from stdin.
pub fn readline() -> Result<String, String> {
    print!("xdb> ");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut buffer = String::new();
    std::io::stdin()
        .read_line(&mut buffer)
        .map_err(|e| e.to_string())?;
    Ok(buffer)
}
