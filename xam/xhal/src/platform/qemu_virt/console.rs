const UART_THR: *mut u8 = 0x1000_0000 as *mut u8;
const UART_LSR: *const u8 = 0x1000_0005 as *const u8;

/// Strong override of xlib's weak `_putch`. Enables `printf()` for all C
/// programs.
#[unsafe(no_mangle)]
pub extern "C" fn _putch(c: i8) {
    unsafe {
        while UART_LSR.read_volatile() & 0x20 == 0 {}
        UART_THR.write_volatile(c as u8);
    }
}
