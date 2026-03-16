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

fn sub(a: i32, b: i32) -> i32 {
    a.wrapping_sub(b)
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

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    for &lhs in &TEST_DATA {
        for &rhs in &TEST_DATA {
            check(sub(lhs, rhs) == lhs.wrapping_sub(rhs));
        }
    }
    0
}
