//! xvisor — Type-1 RISC-V hypervisor (HS-mode payload above OpenSBI fw_jump).

#![no_std]
#![no_main]
#![deny(missing_docs)]
#![warn(clippy::missing_docs_in_private_items)]

use core::{fmt::Write, sync::atomic::Ordering};

mod hal;
mod mm;
mod sbi;
mod vcpu;
mod vm;

use hal::{
    arch::DTB_ADDR,
    platform::{
        halt::{HaltCode, terminate},
        uart::writer,
    },
};

/// HS-mode entry point. Called by `_start` once `tp` and `DTB_ADDR` are
/// populated; the authoritative reads are via `hal::arch::percpu()` and
/// `DTB_ADDR`.
#[unsafe(no_mangle)]
pub extern "C" fn rust_main(_hartid: usize, _dtb_ptr: usize) -> ! {
    let hartid = hal::arch::percpu().hartid;
    let dtb_ptr = DTB_ADDR.load(Ordering::Acquire);

    // Trap-canary demo: fire `ebreak` so the freshly installed trap entry
    // saves the frame, the dispatcher logs the trap, and `sret` lands on
    // the banner below. Off by default; gated by the `trap-canary` feature.
    #[cfg(feature = "trap-canary")]
    // SAFETY: `ebreak` is a documented HS-mode breakpoint instruction; the
    // resulting synchronous trap is routed to `hal::arch::trap::trap_entry`,
    // installed by `_start`, which advances `sepc` past the instruction.
    unsafe {
        core::arch::asm!("ebreak", options(nomem, nostack, preserves_flags));
    }

    let _ = writeln!(
        writer(),
        "xvisor: hello from HS-mode (hartid={}, dtb=0x{:x})",
        hartid,
        dtb_ptr
    );
    terminate(HaltCode::Success);
}

/// Panic handler: print the message over UART, then exit via the SiFive-test
/// finisher with a non-zero status.
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    let _ = writeln!(writer(), "xvisor: panic: {}", info.message());
    terminate(HaltCode::Failure);
}
