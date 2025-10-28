#[cfg(not(test))]
use core::panic::PanicInfo;

/// Minimal panic handler for bare-metal targets.
#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &PanicInfo) -> ! {
    // Note: when running on non-terminating platforms this will `unimplemented!()`.
    // TODO: hook up some form of logging/backtrace before termination.
    crate::platform::misc::terminate(-1)
}
