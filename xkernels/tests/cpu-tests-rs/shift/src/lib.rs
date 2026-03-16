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

const TEST: [u32; 8] = [
    0x12345678, 0x98765432, 0x0, 0xeffa1000, 0x7fffffff, 0x80000000, 0x33, 0xffffffff,
];

const SRL_ANS: [u32; 8] = [
    0x2468ac, 0x130eca8, 0x0, 0x1dff420, 0xffffff, 0x1000000, 0x0, 0x1ffffff,
];

const SRLV_ANS: [u32; 8] = [
    0x1234567, 0x4c3b2a1, 0x0, 0x1dff420, 0x7fffff, 0x400000, 0x0, 0x1fffff,
];

const SRAV_ANS: [u32; 8] = [
    0x1234567, 0xfcc3b2a1, 0x0, 0xffdff420, 0x7fffff, 0xffc00000, 0x0, 0xffffffff,
];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    for i in 0..TEST.len() {
        check((TEST[i] >> 7) == SRL_ANS[i]);
    }
    for i in 0..TEST.len() {
        check(((TEST[i] as i32) >> (i as u32 + 4)) as u32 == SRAV_ANS[i]);
    }
    for i in 0..TEST.len() {
        check((TEST[i] >> (i as u32 + 4)) == SRLV_ANS[i]);
    }
    0
}
