//! RISC-V arch-specific devices.
//!
//! Houses interrupt controllers whose ISA-level semantics (mip bit vocabulary,
//! ACLINT/PLIC register maps) are RISC-V specific. Neutral devices (RAM,
//! UART, VirtIO, TestFinisher) remain in the top-level `device/` module.

pub mod aclint;
pub mod plic;
