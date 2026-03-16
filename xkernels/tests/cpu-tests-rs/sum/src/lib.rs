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

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut i = 1i32;
    let mut sum = 0i32;
    while i <= 100 {
        sum = sum.wrapping_add(i);
        i += 1;
    }
    check(sum == 5050);
    0
}
