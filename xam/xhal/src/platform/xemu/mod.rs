mod boot;
pub mod misc;

unsafe extern "C" {
    fn main() -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn _trm_init() -> ! {
    let ret = unsafe { main() };
    self::misc::terminate(ret)
}
