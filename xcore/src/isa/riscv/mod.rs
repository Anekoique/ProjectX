crate::import_modules!(riscv32 => rv32, riscv64 => rv64);

pub mod decoder;
pub mod reg;

#[cfg(isa32)]
pub use self::rv32::IMG;
#[cfg(isa64)]
pub use self::rv64::IMG;