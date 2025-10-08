crate::import_modules!(riscv, loongarch);

// FIXME: use macro to reduce code duplication
#[cfg(loongarch)]
pub use self::loongarch::IMG;
#[cfg(riscv)]
pub use self::riscv::{
    IMG,
    decoder::{DECODER, DecodedInst},
    reg::RVReg,
};
