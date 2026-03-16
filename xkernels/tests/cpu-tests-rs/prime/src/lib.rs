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

fn is_prime(n: i32) -> bool {
    if n < 2 {
        return false;
    }
    let mut i = 2;
    while i * i <= n {
        if n % i == 0 {
            return false;
        }
        i += 1;
    }
    true
}

const ANS: [i32; 10] = [101, 103, 107, 109, 113, 127, 131, 137, 139, 149];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut count = 0;
    for n in 101..=150 {
        if is_prime(n) {
            check(n == ANS[count]);
            count += 1;
        }
    }
    check(count == 10);
    0
}
