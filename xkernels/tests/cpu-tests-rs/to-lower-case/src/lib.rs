#![no_std]

extern crate xhal;

unsafe extern "C" {
    fn halt(code: i32) -> !;
}

fn check(cond: bool) {
    if !cond {
        unsafe { halt(1) }
    }
}

fn to_lower(c: u8) -> u8 {
    if c >= b'A' && c <= b'Z' {
        c + (b'a' - b'A')
    } else {
        c
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    for c in 0u8..128 {
        let expected = if c >= b'A' && c <= b'Z' {
            c + 32
        } else {
            c
        };
        check(to_lower(c) == expected);
    }
    0
}
