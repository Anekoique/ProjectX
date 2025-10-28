#[inline(always)]
pub fn terminate(code: i32) -> ! {
    unsafe {
        core::arch::asm!(
            "mv a0, {code}",
            "ebreak",
            "1:",
            "j 1b",
            code = in(reg) code,
            options(noreturn),
        );
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn halt(code: i32) -> ! {
    terminate(code)
}
