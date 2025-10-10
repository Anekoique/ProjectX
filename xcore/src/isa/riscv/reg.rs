use std::ops::{Index, IndexMut};

use bitflags::bitflags;

use crate::{XError, XResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub fn from_u8(value: u8) -> XResult<Self> {
        value.try_into().map_err(|_| XError::InvalidReg)
    }

    pub fn from_u32(value: u32) -> XResult<Self> {
        value.try_into().map_err(|_| XError::InvalidReg)
    }
}

impl From<RVReg> for u8 {
    fn from(reg: RVReg) -> Self {
        reg as u8
    }
}

impl TryFrom<u8> for RVReg {
    type Error = &'static str;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value < 32 {
            Ok(unsafe { std::mem::transmute::<u8, RVReg>(value) })
        } else {
            Err("Invalid register number")
        }
    }
}

impl TryFrom<u32> for RVReg {
    type Error = &'static str;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        if value < 32 {
            Ok(unsafe { std::mem::transmute::<u8, RVReg>(value as u8) })
        } else {
            Err("Invalid register number")
        }
    }
}

impl PartialEq<u8> for RVReg {
    fn eq(&self, other: &u8) -> bool {
        (*self as u8) == *other
    }
}

impl PartialEq<RVReg> for u8 {
    fn eq(&self, other: &RVReg) -> bool {
        *self == (*other as u8)
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
        #[cfg(isa64)]
        const SD    = 1 << 63;
        #[cfg(isa32)]
        const SD    = 1 << 31;
    }
}
