use xconfig::TASK_STACK_SIZE;

#[unsafe(link_section = ".bss.stack")]
static mut STACK: [u8; TASK_STACK_SIZE] = [0; TASK_STACK_SIZE];

// mstatus.FS bits [14:13].  Setting them to 0b01 (Initial) enables the F/D
// floating-point unit.  RISC-V resets FS to 0b00 (Off); executing any FP
// instruction while FS=Off raises an illegal-instruction exception.
//
// The de-facto convention on bare-metal QEMU/virt is for M-mode firmware to
// enable FS before jumping to the payload — OpenSBI sets FS=0b11 (Dirty) as
// its first act, and that's why Linux "just works" under `-bios opensbi`.
// Since our HAL boots directly (`-bios none`), we are the firmware, so we
// do the same here.  xemu permits FP regardless of FS, so it doesn't need
// this — but doing it in the HAL unifies guest behavior across platforms.
const MSTATUS_FS_INITIAL: usize = 1 << 13;

#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub unsafe extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        "
            li      t0, {fs_initial}
            csrs    mstatus, t0

            la      sp, {stack}
            li      t0, {stack_size}
            add     sp, sp, t0

            call    {_trm_init}
        ",
        fs_initial = const MSTATUS_FS_INITIAL,
        stack_size = const TASK_STACK_SIZE,
        stack = sym STACK,
        _trm_init = sym super::_trm_init,
    );
}
