//! Startup banner for `xcore::init_xcore`.
//!
//! Writes directly to stdout (not through `xlogger`) so the banner survives
//! any `X_LOG` filter. ANSI color wraps the art only when stdout is a TTY.
//! Setting `X_NO_LOGO` to a non-empty value suppresses the banner.

use std::io::{IsTerminal, Write, stdout};

const LOGO: &str = r"   _  __ ________  _____  __
  | |/ // ____/  |/  / / / /
  |   // __/ / /|_/ / / / /
 /   |/ /___/ /  / / /_/ /
/_/|_/_____/_/  /_/\____/";

const ANSI_COLOR: &str = "\x1b[36m";
const ANSI_RESET: &str = "\x1b[0m";
const ENV_NO_LOGO: &str = "X_NO_LOGO";

/// Print the xemu startup banner to stdout.
///
/// - Writes nothing when `X_NO_LOGO` is set to a non-empty value.
/// - Wraps the banner with ANSI color when `std::io::stdout().is_terminal()`
///   is true; otherwise emits the banner unchanged (no escape bytes).
/// - Swallows broken-pipe / closed-stdout errors; never panics.
pub(crate) fn print_logo() {
    let no_logo = std::env::var(ENV_NO_LOGO).ok();
    let is_tty = stdout().is_terminal();
    let s = render(no_logo.as_deref(), is_tty);
    if s.is_empty() {
        return;
    }
    writeln!(stdout(), "{s}").ok();
}

fn render(no_logo_env: Option<&str>, is_tty: bool) -> String {
    if no_logo_env.is_some_and(|s| !s.is_empty()) {
        return String::new();
    }
    if is_tty {
        format!("{ANSI_COLOR}{LOGO}{ANSI_RESET}")
    } else {
        LOGO.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logo_is_pure_ascii() {
        assert!(LOGO.bytes().all(|b| b.is_ascii()));
    }

    #[test]
    fn logo_lines_within_80_cols() {
        assert!(LOGO.lines().all(|l| l.len() <= 80));
    }

    #[test]
    fn logo_non_empty() {
        assert!(!LOGO.trim().is_empty());
    }

    #[test]
    fn render_suppressed_when_env_set() {
        assert_eq!(render(Some("1"), true), "");
        assert_eq!(render(Some("anything"), false), "");
    }

    #[test]
    fn render_not_suppressed_when_env_empty_or_none() {
        assert!(!render(Some(""), true).is_empty());
        assert!(!render(None, true).is_empty());
    }

    #[test]
    fn render_plain_when_not_tty() {
        let out = render(None, false);
        assert!(!out.contains('\x1b'));
        assert!(out.contains(LOGO.trim_start_matches('\n')));
    }

    #[test]
    fn render_colored_when_tty() {
        let out = render(None, true);
        assert!(out.starts_with(ANSI_COLOR));
        assert!(out.ends_with(ANSI_RESET));
    }
}
