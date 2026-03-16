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

fn partition(a: &mut [i32], p: usize, q: usize) -> usize {
    let pivot = a[p];
    let mut i = p;
    let mut j = q;
    while i < j {
        while i < j && a[j] > pivot {
            j -= 1;
        }
        a[i] = a[j];
        while i < j && a[i] <= pivot {
            i += 1;
        }
        a[j] = a[i];
    }
    a[i] = pivot;
    i
}

fn quick_sort(a: &mut [i32], p: usize, q: usize) {
    if p >= q {
        return;
    }
    let m = partition(a, p, q);
    if m > 0 {
        quick_sort(a, p, m - 1);
    }
    quick_sort(a, m + 1, q);
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut a = [2, 12, 14, 6, 13, 15, 16, 10, 0, 18, 11, 19, 9, 1, 7, 5, 4, 3, 8, 17];

    quick_sort(&mut a, 0, N - 1);
    for i in 0..N {
        check(a[i] == i as i32);
    }

    quick_sort(&mut a, 0, N - 1);
    for i in 0..N {
        check(a[i] == i as i32);
    }
    0
}
