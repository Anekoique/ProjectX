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

fn getbit(buf: &[u8], offset: usize) -> bool {
    let byte = offset >> 3;
    let bit = offset & 7;
    (buf[byte] & (1 << bit)) != 0
}

fn setbit(buf: &mut [u8], offset: usize, val: bool) {
    let byte = offset >> 3;
    let bit = offset & 7;
    let mask = 1u8 << bit;
    if val {
        buf[byte] |= mask;
    } else {
        buf[byte] &= !mask;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut buf = [0xaau8, 0x00u8];

    check(!getbit(&buf, 0));
    check(getbit(&buf, 1));
    check(!getbit(&buf, 2));
    check(getbit(&buf, 3));
    check(!getbit(&buf, 4));
    check(getbit(&buf, 5));
    check(!getbit(&buf, 6));
    check(getbit(&buf, 7));

    setbit(&mut buf, 8, true);
    setbit(&mut buf, 9, false);
    setbit(&mut buf, 10, true);
    setbit(&mut buf, 11, false);
    setbit(&mut buf, 12, true);
    setbit(&mut buf, 13, false);
    setbit(&mut buf, 14, true);
    setbit(&mut buf, 15, false);
    check(buf[1] == 0x55);

    0
}
