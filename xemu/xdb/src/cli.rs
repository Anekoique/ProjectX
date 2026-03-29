use std::{io::Write, sync::OnceLock};

use clap::{Parser, Subcommand};
use regex::Regex;

use crate::{cmd::*, watchpoint::WatchManager};

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
    /// Exit xdb
    #[command(aliases = ["quit", "q", "e"])]
    Exit,
}

/// Expand GDB-style `x/Nf addr` → `x -f F -n N addr` before clap parsing.
fn preprocess_line(line: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^x/(\d+)?([ixbd])\s*(.*)").unwrap());

    if let Some(caps) = re.captures(line.trim()) {
        let n = caps.get(1).map_or("1", |m| m.as_str());
        let f = &caps[2];
        let rest = caps.get(3).map_or("", |m| m.as_str()).trim();
        format!("x -f {f} -n {n} {rest}")
    } else {
        line.to_string()
    }
}

pub fn respond(line: &str, watch_mgr: &mut WatchManager) -> Result<bool, String> {
    let line = preprocess_line(line);
    let args = shlex::split(&line).ok_or("error: Invalid quoting")?;
    let cli = Cli::try_parse_from(args).map_err(|e| e.to_string())?;
    match cli.command {
        Commands::Step { count } => cmd_step(count, watch_mgr),
        Commands::Continue => cmd_continue(watch_mgr),
        Commands::Examine {
            format,
            count,
            addr,
        } => cmd_examine(format, count, addr),
        Commands::Break { addr } => cmd_break(&addr),
        Commands::BreakDelete { id } => cmd_break_delete(id),
        Commands::BreakList => cmd_break_list(),
        Commands::Watch { expr } => cmd_watch(&expr.join(" "), watch_mgr),
        Commands::WatchDelete { id } => {
            watch_mgr.remove(id);
            Ok(())
        }
        Commands::WatchList => {
            cmd_watch_list(watch_mgr);
            Ok(())
        }
        Commands::Print { expr } => cmd_print(&expr.join(" ")),
        Commands::Info { what, name } => cmd_info(&what, name.as_deref()),
        Commands::Load { file } => cmd_load(file),
        Commands::Reset => cmd_reset(),
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

pub fn readline() -> Result<String, String> {
    print!("xdb> ");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut buffer = String::new();
    std::io::stdin()
        .read_line(&mut buffer)
        .map_err(|e| e.to_string())?;
    Ok(buffer)
}
