crate::import_modules!(riscv, loongarch);

mod decoder;

// FIXME: use macro to reduce code duplication
#[cfg(loongarch)]
pub use self::loongarch::IMG;
#[cfg(riscv)]
pub use self::riscv::IMG;

crate::define_decoder!(
    riscv32 => "./instpat/rv32.toml",
    riscv64 => "./instpat/rv64.toml",
    loongarch32 => "./instpat/la32.toml",
    loongarch64 => "./instpat/la64.toml"
);

pub fn init_decoder() {
    trace!("hello xcore");
}
