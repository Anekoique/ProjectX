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

static mut REC: i32 = 0;
static mut LVL: i32 = 0;

fn f0(n: i32, l: i32) -> i32 {
    unsafe {
        if l > LVL { LVL = l; }
        REC += 1;
    }
    if n <= 0 { 1 } else { f3(n / 3, l + 1) }
}

fn f1(n: i32, l: i32) -> i32 {
    unsafe {
        if l > LVL { LVL = l; }
        REC += 1;
    }
    if n <= 0 { 1 } else { f0(n - 1, l + 1) }
}

fn f2(n: i32, l: i32) -> i32 {
    unsafe {
        if l > LVL { LVL = l; }
        REC += 1;
    }
    if n <= 0 { 1 } else { f1(n, l + 1) + 9 }
}

fn f3(n: i32, l: i32) -> i32 {
    unsafe {
        if l > LVL { LVL = l; }
        REC += 1;
    }
    if n <= 0 {
        1
    } else {
        f2(n / 2, l + 1).wrapping_mul(3).wrapping_add(f2(n / 2, l + 1).wrapping_mul(2))
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn main() -> i32 {
    let x = f0(14371, 0);
    check(x == 38270);
    unsafe {
        check(REC == 218);
        check(LVL == 20);
    }
    0
}
