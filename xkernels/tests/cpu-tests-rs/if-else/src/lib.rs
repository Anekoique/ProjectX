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

fn cost(x: i32) -> i32 {
    if x > 500 {
        x.wrapping_mul(8) / 10
    } else if x > 300 {
        x.wrapping_mul(85) / 100
    } else if x > 200 {
        x.wrapping_mul(9) / 10
    } else if x > 100 {
        x.wrapping_mul(95) / 100
    } else {
        x
    }
}

const INPUT: [i32; 14] = [0, 10, 50, 99, 100, 101, 150, 200, 201, 300, 301, 500, 501, 1000];
const ANS: [i32; 14] = [0, 10, 50, 99, 100, 95, 142, 190, 180, 270, 255, 425, 400, 800];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    for i in 0..INPUT.len() {
        check(cost(INPUT[i]) == ANS[i]);
    }
    0
}
