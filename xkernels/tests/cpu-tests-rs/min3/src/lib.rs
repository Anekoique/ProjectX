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

fn min3(a: i32, b: i32, c: i32) -> i32 {
    let mut m = a;
    if b < m { m = b; }
    if c < m { m = c; }
    m
}

const TEST_DATA: [i32; 4] = [0, 0x7fff_ffff, 0x8000_0000u32 as i32, 0xffff_ffffu32 as i32];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    for &a in &TEST_DATA {
        for &b in &TEST_DATA {
            for &c in &TEST_DATA {
                let expected = {
                    let mut m = a;
                    if b < m { m = b; }
                    if c < m { m = c; }
                    m
                };
                check(min3(a, b, c) == expected);
            }
        }
    }
    0
}
