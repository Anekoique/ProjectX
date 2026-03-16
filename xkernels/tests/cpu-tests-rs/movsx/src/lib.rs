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

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let a: [i8; 5] = [0, 1, -1, 127, -128];
    let expected: [i32; 5] = [0, 1, -1, 127, -128];

    for i in 0..a.len() {
        let val = a[i] as i32;
        check(val == expected[i]);
    }

    check(0x80u8 as i8 as i32 == -128);
    check(0xffu8 as i8 as i32 == -1);
    0
}
