//! Platform-specific types and operations.

cfg_if::cfg_if! {
    if #[cfg(all(target_arch = "riscv64", platform = "xemu"))] {
        mod xemu;
        pub use self::xemu::*;
    }
}