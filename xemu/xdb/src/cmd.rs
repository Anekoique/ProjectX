use xcore::{XResult, with_xcpu};

pub fn cmd_continue() -> XResult {
    cmd_step(u64::MAX)
}

pub fn cmd_step(count: u64) -> XResult {
    with_xcpu!(run(count))
}

pub fn cmd_load(file: String) -> XResult {
    with_xcpu!(load(Some(file)).map(|_| ()))
}

pub fn cmd_reset() -> XResult {
    with_xcpu!(reset())
}
