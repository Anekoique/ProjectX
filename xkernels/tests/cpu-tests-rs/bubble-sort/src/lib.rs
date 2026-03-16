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

const N: usize = 20;

fn bubble_sort(a: &mut [i32]) {
    let n = a.len();
    for j in 0..n {
        for i in 0..n - 1 - j {
            if a[i] > a[i + 1] {
                let t = a[i];
                a[i] = a[i + 1];
                a[i + 1] = t;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut a = [2, 12, 14, 6, 13, 15, 16, 10, 0, 18, 11, 19, 9, 1, 7, 5, 4, 3, 8, 17];

    bubble_sort(&mut a);
    for i in 0..N {
        check(a[i] == i as i32);
    }

    bubble_sort(&mut a);
    for i in 0..N {
        check(a[i] == i as i32);
    }
    0
}
