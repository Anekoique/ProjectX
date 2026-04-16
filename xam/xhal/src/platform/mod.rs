//! Platform-specific types and operations.

cfg_if::cfg_if! {
    if #[cfg(all(target_arch = "riscv64", platform = "xemu"))] {
        mod xemu;
        pub use self::xemu::*;
    } else if #[cfg(all(target_arch = "riscv64", platform = "riscv64-qemu-virt"))] {
        mod qemu_virt;
        pub use self::qemu_virt::*;
    } else  {
        mod dummy;
        pub use self::dummy::*;
    }
}
