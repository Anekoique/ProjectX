use std::ops::{Index, IndexMut};

use bitflags::bitflags;
use num_enum::{IntoPrimitive, TryFromPrimitive};

use crate::{XError, XResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
#[allow(non_camel_case_types)]
pub enum RVReg {
    zero = 0,
    ra   = 1,
    sp   = 2,
    gp   = 3,
    tp   = 4,
    t0   = 5,
    t1   = 6,
    t2   = 7,
    s0   = 8,
    s1   = 9,
    a0   = 10,
    a1   = 11,
    a2   = 12,
    a3   = 13,
    a4   = 14,
    a5   = 15,
    a6   = 16,
    a7   = 17,
    s2   = 18,
    s3   = 19,
    s4   = 20,
    s5   = 21,
    s6   = 22,
    s7   = 23,
    s8   = 24,
    s9   = 25,
    s10  = 26,
    s11  = 27,
    t3   = 28,
    t4   = 29,
    t5   = 30,
    t6   = 31,
}

impl RVReg {
    #[inline]
    pub fn from_u8(value: u8) -> XResult<Self> {
        Self::try_from(value).map_err(|_| XError::InvalidReg)
    }

    #[inline]
    pub fn from_u32(value: u32) -> XResult<Self> {
        u8::try_from(value)
            .map_err(|_| XError::InvalidReg)
            .and_then(Self::from_u8)
    }
}

impl PartialEq<u8> for RVReg {
    fn eq(&self, other: &u8) -> bool {
        u8::from(*self) == *other
    }
}

impl Index<RVReg> for [crate::config::Word] {
    type Output = crate::config::Word;
    fn index(&self, reg: RVReg) -> &Self::Output {
        &self[reg as usize]
    }
}

impl IndexMut<RVReg> for [crate::config::Word] {
    fn index_mut(&mut self, reg: RVReg) -> &mut crate::config::Word {
        &mut self[reg as usize]
    }
}

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MStatus: crate::config::Word {
        const SIE   = 1 << 1;
        const MIE   = 1 << 3;
        const SPIE  = 1 << 5;
        const MPIE  = 1 << 7;
        const SPP   = 1 << 8;
        const MPP   = 0b11 << 11;
        const FS    = 0b11 << 13;
        const XS    = 0b11 << 15;
        const MPRV  = 1 << 17;
        const SUM   = 1 << 18;
        const MXR   = 1 << 19;
        const TVM   = 1 << 20;
        const TW    = 1 << 21;
        const TSR   = 1 << 22;
        const SD    = if cfg!(isa64) { 1 << 63 } else { 1 << 31 };
    }
}
