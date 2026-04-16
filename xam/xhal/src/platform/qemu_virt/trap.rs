use core::arch::global_asm;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct GeneralRegs {
    pub zero: usize,
    pub ra: usize,
    pub sp: usize,
    pub gp: usize,
    pub tp: usize,
    pub t0: usize,
    pub t1: usize,
    pub t2: usize,
    pub s0: usize,
    pub s1: usize,
    pub a0: usize,
    pub a1: usize,
    pub a2: usize,
    pub a3: usize,
    pub a4: usize,
    pub a5: usize,
    pub a6: usize,
    pub a7: usize,
    pub s2: usize,
    pub s3: usize,
    pub s4: usize,
    pub s5: usize,
    pub s6: usize,
    pub s7: usize,
    pub s8: usize,
    pub s9: usize,
    pub s10: usize,
    pub s11: usize,
    pub t3: usize,
    pub t4: usize,
    pub t5: usize,
    pub t6: usize,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct TrapFrame {
    pub regs: GeneralRegs,
    pub mepc: usize,
    pub mcause: usize,
}

type Handler = extern "C" fn(*mut TrapFrame);
static mut HANDLER: Option<Handler> = None;

unsafe extern "C" {
    fn __am_trap_entry();
}

#[unsafe(no_mangle)]
pub extern "C" fn init_trap(handler: Handler) {
    unsafe {
        HANDLER = Some(handler);
        core::arch::asm!("csrw mtvec, {}", in(reg) __am_trap_entry as *const () as usize);
    }
}

#[unsafe(no_mangle)]
extern "C" fn __trap_dispatch(tf: *mut TrapFrame) {
    unsafe {
        if let Some(h) = HANDLER {
            h(tf);
        }
    }
}

global_asm!(include_str!("trap.S"));
