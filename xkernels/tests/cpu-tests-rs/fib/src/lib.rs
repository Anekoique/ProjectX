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

const ANS: [i32; 40] = [
    1, 1, 2, 3, 5, 8, 13, 21, 34, 55, 89, 144, 233, 377, 610, 987, 1597, 2584, 4181, 6765,
    10946, 17711, 28657, 46368, 75025, 121393, 196418, 317811, 514229, 832040, 1346269, 2178309,
    3524578, 5702887, 9227465, 14930352, 24157817, 39088169, 63245986, 102334155,
];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut fib = [0i32; 40];
    fib[0] = 1;
    fib[1] = 1;
    for i in 2..40 {
        fib[i] = fib[i - 1] + fib[i - 2];
        check(fib[i] == ANS[i]);
    }
    0
}
