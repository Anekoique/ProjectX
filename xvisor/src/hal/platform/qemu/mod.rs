//! QEMU `virt` machine platform: MMIO base addresses, UART driver, host halt
//! via the SiFive-test finisher.

pub mod halt;
pub mod uart;

/// ns16550 UART0 base.
pub const UART0_BASE: usize = 0x1000_0000;

/// SiFive-test finisher base — write the magic value to terminate QEMU.
pub const SIFIVE_TEST_BASE: usize = 0x10_0000;
