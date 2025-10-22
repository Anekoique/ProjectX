//! Architecture-specific types and operations.

cfg_if::cfg_if! {
    if #[cfg(any(target_arch = "riscv32", target_arch = "riscv64"))] {
        mod riscv;
        pub use self::riscv::*;
    } else if #[cfg(any(target_arch = "loongarch32", target_arch = "loongarch64"))] {
        mod loongarch;
        pub use self::loongarch::*;
    }
}
