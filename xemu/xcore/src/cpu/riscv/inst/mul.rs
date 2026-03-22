// cfg(isa32) blocks use `return` before cfg(isa64) alternatives
#![allow(clippy::needless_return)]

use super::RVCore;
#[cfg(isa32)]
use crate::error::XError;
use crate::{
    config::{SWord, Word},
    error::XResult,
    isa::RVReg,
};

macro_rules! rv64_op {
    ($self:ident, $rd:ident, |$($param:ident),+| $body:expr) => {{
        #[cfg(isa32)]
        {
            let _ = ($rd, $($param),+);
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        {
            let value = { $body };
            $self.set_gpr($rd, value as i64 as Word)
        }
    }};
}

impl RVCore {
    pub(super) fn mul(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.set_gpr(rd, self.gpr[rs1].wrapping_mul(self.gpr[rs2]))
    }

    pub(super) fn mulw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        rv64_op!(self, rd, |rs1, rs2| (self.gpr[rs1] as i32)
            .wrapping_mul(self.gpr[rs2] as i32))
    }

    pub(super) fn mulh(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let result = (self.gpr[rs1] as SWord as i128) * (self.gpr[rs2] as SWord as i128);
        self.set_gpr(rd, (result >> Word::BITS) as SWord as Word)
    }

    pub(super) fn mulhsu(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let result = (self.gpr[rs1] as SWord as i128) * (self.gpr[rs2] as u128) as i128;
        self.set_gpr(rd, (result >> Word::BITS) as SWord as Word)
    }

    pub(super) fn mulhu(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let result = (self.gpr[rs1] as u128) * (self.gpr[rs2] as u128);
        self.set_gpr(rd, (result >> Word::BITS) as Word)
    }

    pub(super) fn div(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let (a, b) = (self.gpr[rs1] as SWord, self.gpr[rs2] as SWord);
        let value = match b {
            0 => Word::MAX,
            -1 if a == SWord::MIN => a as Word,
            _ => (a / b) as Word,
        };
        self.set_gpr(rd, value)
    }

    pub(super) fn divw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        rv64_op!(self, rd, |rs1, rs2| {
            let (a, b) = (self.gpr[rs1] as i32, self.gpr[rs2] as i32);
            match b {
                0 => -1,
                -1 if a == i32::MIN => a,
                _ => a / b,
            }
        })
    }

    pub(super) fn divu(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.set_gpr(
            rd,
            self.gpr[rs1]
                .checked_div(self.gpr[rs2])
                .unwrap_or(Word::MAX),
        )
    }

    pub(super) fn divuw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        rv64_op!(self, rd, |rs1, rs2| {
            (self.gpr[rs1] as u32)
                .checked_div(self.gpr[rs2] as u32)
                .unwrap_or(u32::MAX) as i32
        })
    }

    pub(super) fn rem(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let (a, b) = (self.gpr[rs1] as SWord, self.gpr[rs2] as SWord);
        let value = match b {
            0 => a as Word,
            -1 if a == SWord::MIN => 0,
            _ => (a % b) as Word,
        };
        self.set_gpr(rd, value)
    }

    pub(super) fn remw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        rv64_op!(self, rd, |rs1, rs2| {
            let (a, b) = (self.gpr[rs1] as i32, self.gpr[rs2] as i32);
            match b {
                0 => a,
                -1 if a == i32::MIN => 0,
                _ => a % b,
            }
        })
    }

    pub(super) fn remu(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let (a, b) = (self.gpr[rs1], self.gpr[rs2]);
        self.set_gpr(rd, if b == 0 { a } else { a % b })
    }

    pub(super) fn remuw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        rv64_op!(self, rd, |rs1, rs2| {
            let (a, b) = (self.gpr[rs1] as u32, self.gpr[rs2] as u32);
            (if b == 0 { a } else { a % b }) as i32
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(isa32)]
    use crate::error::XError;

    #[test]
    fn mul_variants_produce_expected_results() {
        let mut core = RVCore::new();
        core.gpr[RVReg::t0] = 6;
        core.gpr[RVReg::t1] = 7;
        core.mul(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], 42);

        let lhs: SWord = -12345;
        let rhs: SWord = 6789;
        core.gpr[RVReg::t0] = lhs as Word;
        core.gpr[RVReg::t1] = rhs as Word;
        core.mulh(RVReg::t3, RVReg::t0, RVReg::t1).unwrap();
        let expected_h = (((lhs as i128) * (rhs as i128)) >> Word::BITS) as SWord as Word;
        assert_eq!(core.gpr[RVReg::t3], expected_h);

        let lhs_su: SWord = -7;
        let rhs_su: Word = 9;
        core.gpr[RVReg::t0] = lhs_su as Word;
        core.gpr[RVReg::t1] = rhs_su;
        core.mulhsu(RVReg::t3, RVReg::t0, RVReg::t1).unwrap();
        let expected_hsu =
            (((lhs_su as i128) * (rhs_su as u128) as i128) >> Word::BITS) as SWord as Word;
        assert_eq!(core.gpr[RVReg::t3], expected_hsu);

        let lhs_u: Word = Word::MAX;
        let rhs_u: Word = 0x12345;
        core.gpr[RVReg::t0] = lhs_u;
        core.gpr[RVReg::t1] = rhs_u;
        core.mulhu(RVReg::t4, RVReg::t0, RVReg::t1).unwrap();
        let expected_hu = (((lhs_u as u128) * (rhs_u as u128)) >> Word::BITS) as Word;
        assert_eq!(core.gpr[RVReg::t4], expected_hu);
    }

    #[test]
    fn div_and_rem_cover_edge_cases() {
        let mut core = RVCore::new();
        core.gpr[RVReg::t0] = 20;
        core.gpr[RVReg::t1] = 6;
        core.div(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2] as SWord, 3);
        core.rem(RVReg::t3, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t3] as SWord, 2);

        core.gpr[RVReg::t1] = 0;
        core.div(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], Word::MAX);
        core.rem(RVReg::t3, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t3], core.gpr[RVReg::t0]);

        core.gpr[RVReg::t0] = SWord::MIN as Word;
        core.gpr[RVReg::t1] = (-1 as SWord) as Word;
        core.div(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], SWord::MIN as Word);
        core.rem(RVReg::t3, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t3], 0);
    }

    #[test]
    fn divu_and_remu_handle_zero_and_regular_paths() {
        let mut core = RVCore::new();
        core.gpr[RVReg::t0] = 25;
        core.gpr[RVReg::t1] = 4;
        core.divu(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], 6);
        core.remu(RVReg::t3, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t3], 1);

        core.gpr[RVReg::t1] = 0;
        core.divu(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], Word::MAX);
        core.remu(RVReg::t3, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t3], core.gpr[RVReg::t0]);
    }

    #[test]
    #[cfg(isa32)]
    fn rv64_only_muldiv_variants_are_rejected_on_rv32() {
        let mut core = RVCore::new();

        for op in [
            RVCore::mulw,
            RVCore::divw,
            RVCore::divuw,
            RVCore::remw,
            RVCore::remuw,
        ] {
            assert!(matches!(
                op(&mut core, RVReg::t0, RVReg::t1, RVReg::t2),
                Err(XError::InvalidInst)
            ));
        }
    }

    #[test]
    #[cfg(isa64)]
    fn rv64_word_muldiv_variants_sign_extend_results() {
        let mut core = RVCore::new();

        core.gpr[RVReg::t0] = 0xFFFF_FFFF;
        core.gpr[RVReg::t1] = 2;
        core.mulw(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2] as SWord, -2);

        core.gpr[RVReg::t0] = 0xFFFF_FFFF_8000_0000;
        core.gpr[RVReg::t1] = 2;
        core.divw(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2] as SWord, -1073741824);

        core.gpr[RVReg::t0] = 0xFFFF_FFFF;
        core.gpr[RVReg::t1] = 2;
        core.divuw(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], 0x7FFF_FFFF);

        core.gpr[RVReg::t0] = 0xFFFF_FFFF_8000_0003;
        core.gpr[RVReg::t1] = 2;
        core.remw(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2] as SWord, -1);

        core.gpr[RVReg::t0] = 0x1_0000_0003;
        core.gpr[RVReg::t1] = 2;
        core.remuw(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], 1);
    }
}
