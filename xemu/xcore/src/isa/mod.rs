//! ISA-specific instruction decoder and register definitions.
//!
//! Compile-time `cfg(riscv)` / `cfg(loongarch)` selects the active ISA.

cfg_if::cfg_if! {
    if #[cfg(riscv)] {
        mod riscv;
        pub use self::riscv::*;
    } else if #[cfg(loongarch)] {
        mod loongarch;
        pub use self::loongarch::*;
    }
}
