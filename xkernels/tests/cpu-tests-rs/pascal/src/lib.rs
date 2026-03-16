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

const N: usize = 31;

const ANS: [i32; N] = [
    1, 30, 435, 4060, 27405, 142506, 593775, 2035800, 5852925, 14307150, 30045015, 54627300,
    86493225, 119759850, 145422675, 155117520, 145422675, 119759850, 86493225, 54627300, 30045015,
    14307150, 5852925, 2035800, 593775, 142506, 27405, 4060, 435, 30, 1,
];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut a = [0i32; N];
    a[0] = 1;
    a[1] = 1;

    for i in 2..N {
        let mut t0 = 1i32;
        for j in 1..i {
            let t1 = a[j];
            a[j] = t0 + t1;
            t0 = t1;
        }
        a[i] = 1;
    }

    for j in 0..N {
        check(a[j] == ANS[j]);
    }

    check(N == 31);
    0
}
