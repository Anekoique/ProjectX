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
    let a: [i32; 10] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9];
    let mut x: i32;

    for i in 0..10 {
        x = a[i];
        check(x == i as i32);
    }

    x = 0;
    for &val in &a {
        check(val == x);
        x += 1;
    }
    0
}
