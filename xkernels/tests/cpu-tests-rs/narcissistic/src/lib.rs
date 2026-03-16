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

fn is_narcissistic(n: i32) -> bool {
    let d0 = n % 10;
    let d1 = (n / 10) % 10;
    let d2 = n / 100;
    d0 * d0 * d0 + d1 * d1 * d1 + d2 * d2 * d2 == n
}

const ANS: [i32; 4] = [153, 370, 371, 407];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut count = 0;
    for n in 100..500 {
        if is_narcissistic(n) {
            check(n == ANS[count]);
            count += 1;
        }
    }
    check(count == 4);
    0
}
