#![no_std]

extern crate xhal;

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

const EXPECTED: [i32; TEST_DATA.len() * TEST_DATA.len()] = [
     0,         1,         2,  2147483647, -2147483648, -2147483647,         -2,         -1,
     1,         2,         3, -2147483648, -2147483647, -2147483646,         -1,          0,
     2,         3,         4, -2147483647, -2147483646, -2147483645,          0,          1,
 2147483647, -2147483648, -2147483647,         -2,         -1,          0,  2147483645,  2147483646,
-2147483648, -2147483647, -2147483646,         -1,          0,          1,  2147483646,  2147483647,
-2147483647, -2147483646, -2147483645,          0,          1,          2,  2147483647, -2147483648,
        -2,         -1,          0,  2147483645,  2147483646,  2147483647,         -4,         -3,
        -1,          0,          1,  2147483646,  2147483647, -2147483648,         -3,         -2,
];

unsafe extern "C" {
    fn halt(code: i32) -> !;
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut idx = 0;
    for &lhs in &TEST_DATA {
        for &rhs in &TEST_DATA {
            check(add(lhs, rhs) == EXPECTED[idx]);
            idx += 1;
        }
    }
    0
}

fn add(a: i32, b: i32) -> i32 {
    a.wrapping_add(b)
}

fn check(cond: bool) {
    if !cond {
        unsafe { halt(1) }
    }
}
