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

const MEM: [u16; 8] = [0x0, 0x0258, 0x4abc, 0x7fff, 0x8000, 0x8100, 0xabcd, 0xffff];

const LH_ANS: [i32; 8] = [
    0x00000000, 0x00000258, 0x00004abc, 0x00007fff,
    0xffff8000u32 as i32, 0xffff8100u32 as i32, 0xffffabcdu32 as i32, 0xffffffffu32 as i32,
];

const LHU_ANS: [u32; 8] = [
    0x00000000, 0x00000258, 0x00004abc, 0x00007fff,
    0x00008000, 0x00008100, 0x0000abcd, 0x0000ffff,
];

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    for i in 0..MEM.len() {
        check(MEM[i] as i16 as i32 == LH_ANS[i]);
    }
    for i in 0..MEM.len() {
        check(MEM[i] as u32 == LHU_ANS[i]);
    }

    let mut mem = MEM;
    let sh_ans: [u16; 8] = [0xfffd, 0xfff7, 0xffdf, 0xff7f, 0xfdff, 0xf7ff, 0xdfff, 0x7fff];
    for i in 0..mem.len() {
        mem[i] = !(1u16 << (2 * i + 1));
        check(mem[i] == sh_ans[i]);
    }
    0
}
