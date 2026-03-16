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

fn is_perfect(n: i32) -> bool {
    let mut sum = 0;
    let mut i = 1;
    while i < n {
        if n % i == 0 {
            sum += i;
        }
        i += 1;
    }
    sum == n
}

const ANS: [i32; 2] = [6, 28];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut count = 0;
    for n in 1..=30 {
        if is_perfect(n) {
            check(n == ANS[count]);
            count += 1;
        }
    }
    check(count == 2);
    0
}
