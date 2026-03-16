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

fn max(a: i32, b: i32) -> i32 {
    if a > b { a } else { b }
}

const TEST_DATA: [i32; 8] = [
    0,
    1,
    2,
    0x7fff_ffff,
    0x8000_0000u32 as i32,
    0x8000_0001u32 as i32,
    0xffff_fffeu32 as i32,
    0xffff_ffffu32 as i32,
];

const ANS: [i32; 64] = [
    0, 1, 2, 0x7fffffff, 0, 0, 0, 0,
    1, 1, 2, 0x7fffffff, 1, 1, 1, 1,
    2, 2, 2, 0x7fffffff, 2, 2, 2, 2,
    0x7fffffff, 0x7fffffff, 0x7fffffff, 0x7fffffff, 0x7fffffff, 0x7fffffff, 0x7fffffff, 0x7fffffff,
    0, 1, 2, 0x7fffffff, 0x80000000u32 as i32, 0x80000001u32 as i32, 0xfffffffeu32 as i32, 0xffffffffu32 as i32,
    0, 1, 2, 0x7fffffff, 0x80000001u32 as i32, 0x80000001u32 as i32, 0xfffffffeu32 as i32, 0xffffffffu32 as i32,
    0, 1, 2, 0x7fffffff, 0xfffffffeu32 as i32, 0xfffffffeu32 as i32, 0xfffffffeu32 as i32, 0xffffffffu32 as i32,
    0, 1, 2, 0x7fffffff, 0xffffffffu32 as i32, 0xffffffffu32 as i32, 0xffffffffu32 as i32, 0xffffffffu32 as i32,
];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut idx = 0;
    for &a in &TEST_DATA {
        for &b in &TEST_DATA {
            check(max(a, b) == ANS[idx]);
            idx += 1;
        }
    }
    0
}
