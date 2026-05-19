//! S-mode (and HS-mode) CSR helpers used by `boot.rs` and the future trap
//! entry. H-extension CSR wrappers (`hgatp`, `hstatus`, …) land here in a
//! later feature.

/// Write `stvec` (S-mode trap vector base address).
///
/// # Safety
///
/// `addr` must be a 4-byte-aligned address of executable HS-mode code that
/// implements a valid trap handler (or, for the boot-time installation,
/// the `wfi` parking pad).
#[inline]
pub unsafe fn write_stvec(addr: usize) {
    // SAFETY: writing `stvec` is privileged but supported in HS-mode; the
    // caller has guaranteed `addr` is a valid handler entry.
    unsafe {
        core::arch::asm!("csrw stvec, {}", in(reg) addr, options(nomem, nostack, preserves_flags));
    }
}
