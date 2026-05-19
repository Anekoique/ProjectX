//! Host shutdown via the SiFive-test finisher (`0x100000`).
//!
//! xvisor is a Type-1 hypervisor and owns the machine — host shutdown goes
//! through the QEMU finisher directly rather than SBI SRST.

use super::SIFIVE_TEST_BASE;

/// Halt exit code carried into the SiFive-test finisher.
#[repr(i32)]
pub enum HaltCode {
    /// Clean shutdown: QEMU exits with status 0.
    Success = 0,
    /// Failure shutdown: QEMU exits with non-zero status.
    Failure = 1,
}

/// SiFive-test finisher magic for a clean shutdown.
const FINISHER_PASS: u32 = 0x5555;
/// SiFive-test finisher magic for a failure shutdown (OR'd with code << 16).
const FINISHER_FAIL: u32 = 0x3333;

/// Write the SiFive-test finisher magic for `code`, then `wfi`-loop forever as
/// a fallback in case the finisher write returns (it should not).
pub fn terminate(code: HaltCode) -> ! {
    let magic: u32 = match code {
        HaltCode::Success => FINISHER_PASS,
        HaltCode::Failure => FINISHER_FAIL | ((HaltCode::Failure as u32) << 16),
    };
    // SAFETY: SIFIVE_TEST_BASE is the QEMU virt finisher MMIO; writing the
    // magic value terminates the VM and never returns.
    unsafe {
        (SIFIVE_TEST_BASE as *mut u32).write_volatile(magic);
    }
    loop {
        // SAFETY: wfi is unprivileged on HS-mode and side-effect-free.
        unsafe {
            core::arch::asm!("wfi", options(nomem, nostack));
        }
    }
}
