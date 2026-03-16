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

fn add(a: i64, b: i64) -> i64 {
    a.wrapping_add(b)
}

const TEST_DATA: [i64; 8] = [
    0,
    1,
    2,
    0x7fff_ffff_ffff_ffff,
    0x8000_0000_0000_0000u64 as i64,
    0x8000_0000_0000_0001u64 as i64,
    0xffff_ffff_ffff_fffeu64 as i64,
    0xffff_ffff_ffff_ffffu64 as i64,
];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    for &lhs in &TEST_DATA {
        for &rhs in &TEST_DATA {
            check(add(lhs, rhs) == lhs.wrapping_add(rhs));
        }
    }
    0
}
