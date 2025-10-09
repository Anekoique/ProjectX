pub const CONFIG_MBASE: usize = 0x80000000;
pub const CONFIG_MSIZE: usize = 0x8000000;

#[cfg(isa64)]
pub type Word = u64;
#[cfg(isa32)]
pub type Word = u32;

#[cfg(isa64)]
pub type SWord = i64;
#[cfg(isa32)]
pub type SWord = i32;

#[cfg(isa64)]
pub const XLEN: usize = 64;
#[cfg(isa32)]
pub const XLEN: usize = 32;

pub const SHAMT_MASK: Word = (XLEN as Word) - 1;

#[cfg(isa64)]
#[inline(always)]
pub fn word_to_u32(value: Word) -> u32 {
    value as u32
}

#[cfg(isa32)]
#[inline(always)]
pub fn word_to_u32(value: Word) -> u32 {
    value
}

#[cfg(isa64)]
#[inline(always)]
pub fn word_to_shamt(value: Word) -> u32 {
    (value & SHAMT_MASK) as u32
}

#[cfg(isa32)]
#[inline(always)]
pub fn word_to_shamt(value: Word) -> u32 {
    value & SHAMT_MASK
}
