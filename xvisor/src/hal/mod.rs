//! Hardware abstraction layer. `arch` is selected by `target_arch`; `platform`
//! is selected by a Cargo feature flag (default: `platform-qemu`).
//!
//! Backends currently shipped:
//!   * `arch::riscv` — RISC-V (RV64GCH); the live backend.
//!   * `arch::loongarch` — stub; reserved namespace.
//!   * `platform::qemu` — QEMU `-machine virt -cpu rv64,h=true`; the live
//!     backend.
//!   * `platform::xemu` — stub; reserved namespace.

#[cfg_attr(target_arch = "riscv64", path = "arch/riscv/mod.rs")]
#[cfg_attr(target_arch = "loongarch64", path = "arch/loongarch/mod.rs")]
pub mod arch;

#[cfg_attr(feature = "platform-qemu", path = "platform/qemu/mod.rs")]
#[cfg_attr(feature = "platform-xemu", path = "platform/xemu/mod.rs")]
pub mod platform;
