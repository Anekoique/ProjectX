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

fn fact(n: i32) -> i32 {
    if n <= 1 { 1 } else { n.wrapping_mul(fact(n - 1)) }
}

const ANS: [i32; 13] = [1, 1, 2, 6, 24, 120, 720, 5040, 40320, 362880, 3628800, 39916800, 479001600];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    for i in 0..13 {
        check(fact(i) == ANS[i as usize]);
    }
    0
}
