use std::io::Write;

use clap::{Parser, Subcommand};

use crate::cmd::*;

#[derive(Debug, Parser)]
#[command(multicall = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run a single instruction
    #[command(alias = "s")]
    Step {
        /// Number of instructions to step
        #[arg(default_value_t = 1)]
        count: u32,
    },
    /// Continue execution
    #[command(alias = "c")]
    Continue,
    /// Load a elf file into memory
    #[command(alias = "l")]
    Load {
        /// Path to the binary file
        file: String,
    },
    /// Reset
    #[command(alias = "r")]
    Reset,
    /// Exit the xdb
    #[command(aliases = ["quit", "q", "e"])]
    Exit,
}

pub fn respond(line: &str) -> Result<bool, String> {
    let args = shlex::split(line).ok_or("error: Invalid quoting")?;
    let cli = Cli::try_parse_from(args).map_err(|e| e.to_string())?;
    match cli.command {
        Commands::Step { count } => cmd_step(count),
        Commands::Continue => cmd_continue(),
        Commands::Load { file } => cmd_load(file),
        Commands::Reset => cmd_reset(),
        Commands::Exit => {
            println!("Exiting ...");
            return Ok(false);
        }
    }
    .map(|_| true)
    .or_else(|e| {
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
