extern crate log;

use core::{
    fmt::{self, Write},
    str::FromStr,
};
use std::{sync::OnceLock, time::Instant};

use log::{Level, LevelFilter, Log, Metadata, Record};

#[macro_export]
macro_rules! xprintln {
    ($color_code:expr, $($arg:tt)*) => {
        format_args!("\x1B[{}m{}\x1B[m\n", $color_code as u8, format_args!($($arg)*))
    };
}

#[macro_export]
macro_rules! xprint {
    ($color_code:expr, $($arg:tt)*) => {
        format_args!("\x1B[{}m{}\x1B[m", $color_code as u8, format_args!($($arg)*))
    };
}

#[repr(u8)]
#[allow(dead_code)]
pub enum ColorCode {
    Black         = 30,
    Red           = 31,
    Green         = 32,
    Yellow        = 33,
    Blue          = 34,
    Magenta       = 35,
    Cyan          = 36,
    White         = 37,
    BrightBlack   = 90,
    BrightRed     = 91,
    BrightGreen   = 92,
    BrightYellow  = 93,
    BrightBlue    = 94,
    BrightMagenta = 95,
    BrightCyan    = 96,
    BrightWhite   = 97,
}

static START_TIME: OnceLock<Instant> = OnceLock::new();

struct Logger;

impl Write for Logger {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        std::print!("{s}");
        Ok(())
    }
}

impl Log for Logger {
    #[inline]
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }

        let level = record.level();
        let line = record.line().unwrap_or(0);
        let path = record.target();
        let args = record.args();
        let args_color = match level {
            Level::Error => ColorCode::Red,
            Level::Warn => ColorCode::Yellow,
            Level::Info => ColorCode::Green,
            Level::Debug => ColorCode::Cyan,
            Level::Trace => ColorCode::BrightBlack,
        };

        let boot_time = START_TIME.get().unwrap().elapsed().as_secs_f64();
        let format = xprint!(
            ColorCode::White,
            "[{boot_time:9.6} {path}:{line}] {args}\n",
            boot_time = boot_time,
            path = path,
            line = line,
            args = xprint!(args_color, "{}", args),
        );
        print!("{format}");
    }

    fn flush(&self) {}
}

/// Initializes the logger.
pub fn init() {
    START_TIME.set(Instant::now()).ok();
    log::set_logger(&Logger).unwrap();
    log::set_max_level(LevelFilter::Warn);
}

/// Set the maximum log level.
pub fn set_max_level(level: &str) {
    let lf = LevelFilter::from_str(level)
        .ok()
        .unwrap_or(LevelFilter::Off);
    log::set_max_level(lf);
}
