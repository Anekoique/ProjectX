/// SiFive test finisher on QEMU virt: write to trigger shutdown.
/// See: QEMU hw/misc/sifive_test.c
///   - FINISHER_PASS = 0x5555  (exit code 0)
///   - FINISHER_FAIL = 0x3333  (exit code = (val >> 16) & 0xFFFF, forced >=1)
const FINISHER: *mut u32 = 0x10_0000 as *mut u32;
const FINISHER_PASS: u32 = 0x5555;
const FINISHER_FAIL: u32 = 0x3333;

/// UART THR for bare-metal early print. Duplicated here (not imported from
/// `console`) to keep the termination path self-contained and safe to call
/// even if higher-level console state is corrupt.
const UART_THR: *mut u8 = 0x1000_0000 as *mut u8;
const UART_LSR: *const u8 = 0x1000_0005 as *const u8;

#[inline(always)]
fn emit_byte(c: u8) {
    unsafe {
        while UART_LSR.read_volatile() & 0x20 == 0 {}
        UART_THR.write_volatile(c);
    }
}

#[inline(always)]
fn emit_str(s: &str) {
    for b in s.as_bytes() {
        emit_byte(*b);
    }
    emit_byte(b'\n');
}

#[inline(always)]
pub fn terminate(code: i32) -> ! {
    // Emit a guest-side signal string that test harnesses can grep for.
    // Matches xemu's "HIT GOOD TRAP" / "HIT BAD TRAP" convention so the same
    // am-tests Makefile works under either platform.
    if code == 0 {
        emit_str("HIT GOOD TRAP");
    } else {
        emit_str("HIT BAD TRAP");
    }

    let val = if code == 0 {
        FINISHER_PASS
    } else {
        // Pack the non-zero exit code into bits [31:16] so QEMU reports it.
        ((code as u32) << 16) | FINISHER_FAIL
    };
    unsafe {
        FINISHER.write_volatile(val);
    }
    // Should not reach here — QEMU exits on finisher write.
    loop {
        core::hint::spin_loop();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn halt(code: i32) -> ! {
    terminate(code)
}
