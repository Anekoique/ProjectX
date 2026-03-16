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

fn select_sort(a: &mut [i32]) {
    let n = a.len();
    for i in 0..n {
        let mut min = i;
        for j in i + 1..n {
            if a[j] < a[min] {
                min = j;
            }
        }
        let t = a[i];
        a[i] = a[min];
        a[min] = t;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut a = [2, 12, 14, 6, 13, 15, 16, 10, 0, 18, 11, 19, 9, 1, 7, 5, 4, 3, 8, 17];

    select_sort(&mut a);
    for i in 0..N {
        check(a[i] == i as i32);
    }

    select_sort(&mut a);
    for i in 0..N {
        check(a[i] == i as i32);
    }
    0
}
