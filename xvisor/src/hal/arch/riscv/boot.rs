//! HS-mode boot entry — invoked by OpenSBI fw_jump.
//!
//! On entry: `a0` = hartid, `a1` = dtb-ptr, privilege = HS-mode. `_start`
//! establishes the per-hart stack, zeroes BSS, captures the DTB pointer into
//! [`DTB_ADDR`], installs `tp` to point at the per-hart [`PerCpu`] slot,
//! installs a `wfi` parking pad in `stvec`, and tail-calls [`rust_main`].

use core::{ptr::addr_of_mut, sync::atomic::Ordering};

use super::{DTB_ADDR, MAX_HARTS, PER_CPU, PerCpu, STACK_SIZE_PER_HART, csr};

/// Total boot-time stack reservation in `.bss.stack`.
const STACK_BYTES: usize = STACK_SIZE_PER_HART * MAX_HARTS;

/// Boot-hart stack storage referenced symbolically by `_start`.
#[unsafe(link_section = ".bss.stack")]
static mut STACK: [u8; STACK_BYTES] = [0; STACK_BYTES];

/// HS-mode entry point. Layout requirements:
///   * `.text.boot` section so the linker script places it at `BASE_ADDRESS`.
///   * `#[unsafe(naked)]` so the compiler emits no prologue / epilogue and the
///     ABI of `a0` / `a1` from OpenSBI flows through unmodified.
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub unsafe extern "C" fn _start() -> ! {
    core::arch::naked_asm!(
        // sp = top of the boot-hart slice of STACK
        "la     sp, {stack}",
        "li     t0, {stack_bytes}",
        "add    sp, sp, t0",

        // preserve the OpenSBI handoff across Rust calls
        "mv     s0, a0",
        "mv     s1, a1",

        // zero BSS before any Rust static is read or written
        "call   {clear_bss}",

        // stash the DTB pointer and set up tp = &PER_CPU[hartid]
        "mv     a0, s0",
        "mv     a1, s1",
        "call   {arch_setup}",

        // install the wfi parking pad in stvec
        "call   {install_trap_trampoline}",

        // restore the handoff and call rust_main(hartid, dtb)
        "mv     a0, s0",
        "mv     a1, s1",
        "call   {rust_main}",

        // defensive park if rust_main ever returns
        "1:",
        "wfi",
        "j      1b",
        stack                   = sym STACK,
        stack_bytes             = const STACK_BYTES,
        clear_bss               = sym clear_bss,
        arch_setup              = sym arch_setup,
        install_trap_trampoline = sym install_trap_trampoline,
        rust_main               = sym crate::rust_main,
    );
}

/// Zero the BSS segment `[_bss_start, _bss_end)`.
///
/// # Safety
///
/// The caller must guarantee that the linker symbols `_bss_start` /
/// `_bss_end` bracket a writable, hypervisor-exclusively-owned region;
/// alignment is the byte alignment of `*mut u8`, which is 1.
unsafe extern "C" fn clear_bss() {
    // SAFETY: `_bss_start` / `_bss_end` are linker-defined section markers
    // bracketing the BSS region xvisor owns exclusively at boot time.
    unsafe {
        let start = addr_of_mut!(_bss_start);
        let end = addr_of_mut!(_bss_end);
        let len = end.offset_from(start) as usize;
        core::slice::from_raw_parts_mut(start, len).fill(0);
    }
}

/// Stash the DTB pointer into [`DTB_ADDR`] and install `tp = &PER_CPU[hartid]`
/// with the slot populated for this hart.
///
/// # Safety
///
/// Must run after `clear_bss` so writes land in actually-zeroed memory and
/// not in whatever OpenSBI left in BSS.
unsafe extern "C" fn arch_setup(hartid: usize, dtb_ptr: usize) {
    DTB_ADDR.store(dtb_ptr, Ordering::Release);

    // Single-hart configuration — only hart 0 has a slot today.
    debug_assert!(hartid < MAX_HARTS);
    // SAFETY: single-hart configuration — only this hart writes its own
    // `PER_CPU` slot; no concurrent writers exist.
    let slot: *mut PerCpu = unsafe { addr_of_mut!(PER_CPU[0]) };
    unsafe {
        // SAFETY: `slot` points to a valid, properly aligned `PerCpu` inside
        // the `'static` `PER_CPU` array.
        (*slot).hartid = hartid;
        (*slot).stack_top = addr_of_mut!(STACK) as *mut u8;

        // SAFETY: `tp` write is the standard convention; the address points
        // inside the `'static` `PER_CPU` array.
        core::arch::asm!(
            "mv tp, {}",
            in(reg) slot as usize,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Install a single-instruction `wfi` parking pad in `stvec` so unintended
/// traps loop visibly instead of triple-bouncing through whatever value
/// OpenSBI left in the CSR.
unsafe extern "C" fn install_trap_trampoline() {
    // SAFETY: `trap_trampoline` is a real code symbol with 4-byte alignment;
    // writing `stvec` to point at it is the documented HS-mode setup step.
    unsafe {
        csr::write_stvec(trap_trampoline as *const () as usize);
    }
}

/// Single-instruction parking pad. `stvec` points here pre-trap-handler;
/// any unintended trap loops on `wfi` and is operator-visible. Placed in
/// the regular `.text` section so it never precedes `_start` at the binary
/// entry point.
#[unsafe(naked)]
unsafe extern "C" fn trap_trampoline() -> ! {
    core::arch::naked_asm!("1:", "wfi", "j 1b");
}

unsafe extern "C" {
    /// Linker-defined start of the BSS region.
    static mut _bss_start: u8;
    /// Linker-defined one-past-end of the BSS region.
    static mut _bss_end: u8;
}
