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

const N: usize = 10;

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut a = [0i32; N];
    for i in 0..N {
        a[i] = i as i32;
    }
    for i in 0..N {
        for j in 1..N + 1 {
            a[i] = a[i].wrapping_mul(j as i32);
        }
    }
    for i in 0..N {
        for j in 1..N + 1 {
            a[i] /= j as i32;
        }
    }
    for i in 0..N {
        check(a[i] == i as i32);
    }
    0
}
