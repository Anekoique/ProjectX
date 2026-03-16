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

// Check if 2^p - 1 is a Mersenne prime
fn mersenne(p: i32) -> bool {
    let m = (1i32 << p) - 1;
    is_prime(m)
}

const ANS: [i32; 7] = [2, 3, 5, 7, 13, 17, 19];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut count = 0;
    for p in 2..=20 {
        if is_prime(p) && mersenne(p) {
            check(p == ANS[count]);
            count += 1;
        }
    }
    check(count == 7);
    0
}
