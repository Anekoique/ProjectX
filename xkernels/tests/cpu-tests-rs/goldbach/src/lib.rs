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

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut n = 4;
    while n <= 30 {
        let mut found = false;
        let mut i = 2;
        while i <= n / 2 {
            if is_prime(i) && is_prime(n - i) {
                found = true;
                break;
            }
            i += 1;
        }
        check(found);
        n += 2;
    }
    0
}
