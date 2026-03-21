use std::ops::{Index, IndexMut};

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Word;

    #[test]
    fn from_u8_boundary_and_roundtrip() {
        assert_eq!(RVReg::from_u8(0).unwrap(), RVReg::zero);
        assert_eq!(RVReg::from_u8(31).unwrap(), RVReg::t6);
        assert!(matches!(RVReg::from_u8(32), Err(XError::InvalidReg)));

        for i in 0..32u8 {
            assert_eq!(u8::from(RVReg::from_u8(i).unwrap()), i);
        }
    }

    #[test]
    fn from_u32_rejects_overflow() {
        assert_eq!(RVReg::from_u32(5).unwrap(), RVReg::t0);
        assert!(matches!(RVReg::from_u32(32), Err(XError::InvalidReg)));
        assert!(matches!(RVReg::from_u32(0x100), Err(XError::InvalidReg)));
    }

    #[test]
    fn index_and_partial_eq_u8() {
        let mut gpr = [0 as Word; 32];
        gpr[RVReg::t0] = 42;
        assert_eq!(gpr[RVReg::t0], 42);
        assert!(RVReg::t0 == 5u8);
        assert!(!(RVReg::zero == 1u8));
    }
}
