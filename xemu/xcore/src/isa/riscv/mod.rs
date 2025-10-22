mod decoder;
mod inst;
mod reg;

pub use decoder::{DECODER, DecodedInst};
pub use inst::{InstFormat, InstKind};
pub use reg::RVReg;

pub const IMG: [u32; 5] = [
    0x00000297, // auipc t0,0
    0x00028823, // sb  zero,16(t0)
    0x0102c503, // lbu a0,16(t0)
    0x00100073, // ebreak (used as nemu_trap)
    0xdeadbeef, // some data
];
