//! ns16550 UART driver — direct MMIO, THRE-poll, no interrupts.
//!
//! Mirrors `xam/xhal/src/platform/xemu/console.rs` line-for-line, ported from
//! the M-mode caller to HS-mode and wrapped in a `core::fmt::Write` adapter so
//! `write!` / `writeln!` work in `rust_main` and the panic handler.

use core::fmt;

use super::UART0_BASE;

/// ns16550 transmit holding register (TX FIFO entry).
const UART_THR: *mut u8 = UART0_BASE as *mut u8;
/// ns16550 line status register (THRE bit is bit 5).
const UART_LSR: *const u8 = (UART0_BASE + 5) as *const u8;
/// LSR.THRE — transmit holding register empty.
const LSR_THRE: u8 = 0x20;

/// Write a single byte to UART0, polling LSR.THRE first.
pub fn putch(b: u8) {
    // SAFETY: UART0 MMIO is hypervisor-owned; no other code path writes to
    // these addresses.
    unsafe {
        while UART_LSR.read_volatile() & LSR_THRE == 0 {}
        UART_THR.write_volatile(b);
    }
}

/// `core::fmt::Write` adapter for the UART, enabling `write!` macros.
pub struct UartWriter;

impl fmt::Write for UartWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        for b in s.bytes() {
            putch(b);
        }
        Ok(())
    }
}

/// Construct a `UartWriter` for `write!` / `writeln!` callers.
pub fn writer() -> UartWriter {
    UartWriter
}
