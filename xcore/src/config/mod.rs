pub const CONFIG_MBASE: usize = 0x80000000;
pub const CONFIG_MSIZE: usize = 0x8000000;

#[cfg(isa64)]
pub type Word = u64;
#[cfg(isa32)]
pub type Word = u32;

// #[cfg(isa64)]
// pub type SWord = i64;
// #[cfg(isa32)]
// pub type SWord = i32;

pub const WSIZE: usize = std::mem::size_of::<Word>();
pub const XLEN: usize = WSIZE * 8;
