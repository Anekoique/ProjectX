use xconfig::TASK_STACK_SIZE;

#[unsafe(link_section = ".bss.stack")]
static mut STACK: [u8; TASK_STACK_SIZE] = [0; TASK_STACK_SIZE];

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub unsafe extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        "
            la      sp, {stack}
            li      t0, {stack_size}
            add     sp, sp, t0

            call    {_trm_init}
        ",
        stack_size = const TASK_STACK_SIZE,
        stack = sym STACK,
        _trm_init = sym super::_trm_init,
    );
}
