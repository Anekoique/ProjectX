mod boot;
pub mod console;
pub mod misc;
pub mod timer;
pub mod trap;

unsafe extern "C" {
    fn main(args: *const u8) -> i32;
    static mainargs: u8;
}

// Weak default: programs that don't define mainargs get an empty string
core::arch::global_asm!(".weak mainargs; mainargs: .byte 0");

#[unsafe(no_mangle)]
pub extern "C" fn _trm_init() -> ! {
    let args = unsafe { &mainargs as *const u8 };
    let ret = unsafe { main(args) };
    self::misc::terminate(ret)
}
