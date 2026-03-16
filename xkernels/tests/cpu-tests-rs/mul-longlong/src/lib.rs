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

fn mul(a: i64, b: i64) -> i64 {
    a.wrapping_mul(b)
}

const TEST_DATA: [i32; 4] = [
    0xaeb1c2aau32 as i32,
    0x4500ff2b,
    0x877190afu32 as i32,
    0x11f42438,
];

const ANS: [i64; 10] = [
    0x19d29ab9db1a18e4,
    0xea15986d3ac3088eu64 as i64,
    0x2649e980fc0db236,
    0xfa4c43da0a4a7d30u64 as i64,
    0x1299898e2c56b139,
    0xdf8123d50a319e65u64 as i64,
    0x04d6dfa84c15dd68,
    0x38c5d79b9e4357a1,
    0xf78b91cb1efc4248u64 as i64,
    0x014255a47fdfcc40,
];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let mut idx = 0;
    for i in 0..TEST_DATA.len() {
        for j in i..TEST_DATA.len() {
            check(mul(TEST_DATA[i] as i64, TEST_DATA[j] as i64) == ANS[idx]);
            idx += 1;
        }
    }
    0
}
