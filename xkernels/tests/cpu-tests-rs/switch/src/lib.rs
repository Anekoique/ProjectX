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

fn switch_val(x: i32) -> i32 {
    match x {
        0 => 0,
        1 => 1,
        2 | 3 => 2,
        4 | 5 | 6 | 7 => 3,
        8 ..=14 => 4,
        _ => 5,
    }
}

const ANS: [i32; 16] = [0, 1, 2, 2, 3, 3, 3, 3, 4, 4, 4, 4, 4, 4, 4, 5];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    for i in 0..ANS.len() {
        check(switch_val(i as i32) == ANS[i]);
    }
    0
}
