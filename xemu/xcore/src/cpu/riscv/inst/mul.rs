use super::RVCore;
use crate::{
    config::{SWord, Word},
    error::XResult,
    isa::RVReg,
};
#[cfg(isa32)]
use crate::error::XError;

impl RVCore {
    pub(super) fn mul(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let value = self.gpr[rs1].wrapping_mul(self.gpr[rs2]);
        self.set_gpr(rd, value)
    }

    pub(super) fn mulw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        #[cfg(isa32)]
        {
            let _ = (rd, rs1, rs2);
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        {
            let lhs = self.gpr[rs1] as i32;
            let rhs = self.gpr[rs2] as i32;
            let value = lhs.wrapping_mul(rhs);
            self.set_gpr(rd, value as i64 as Word)
        }
    }

    pub(super) fn mulh(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let lhs = self.gpr[rs1] as SWord as i128;
        let rhs = self.gpr[rs2] as SWord as i128;
        let value = ((lhs * rhs) >> Word::BITS) as SWord as Word;
        self.set_gpr(rd, value)
    }

    pub(super) fn mulhsu(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let lhs = self.gpr[rs1] as SWord as i128;
        let rhs = self.gpr[rs2] as u128;
        let value = ((lhs * rhs as i128) >> Word::BITS) as SWord as Word;
        self.set_gpr(rd, value)
    }

    pub(super) fn mulhu(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let lhs = self.gpr[rs1] as u128;
        let rhs = self.gpr[rs2] as u128;
        let value = ((lhs * rhs) >> Word::BITS) as Word;
        self.set_gpr(rd, value)
    }

    pub(super) fn div(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let dividend = self.gpr[rs1] as SWord;
        let divisor = self.gpr[rs2] as SWord;
        let value = if divisor == 0 {
            Word::MAX
        } else if dividend == SWord::MIN && divisor == -1 {
            dividend as Word
        } else {
            (dividend / divisor) as Word
        };
        self.set_gpr(rd, value)
    }

    pub(super) fn divw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        #[cfg(isa32)]
        {
            let _ = (rd, rs1, rs2);
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        {
            let dividend = self.gpr[rs1] as i32;
            let divisor = self.gpr[rs2] as i32;
            let value = if divisor == 0 {
                -1
            } else if dividend == i32::MIN && divisor == -1 {
                dividend
            } else {
                dividend / divisor
            };
            self.set_gpr(rd, value as i64 as Word)
        }
    }

    pub(super) fn divu(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let dividend = self.gpr[rs1];
        let divisor = self.gpr[rs2];
        let value = if divisor == 0 {
            Word::MAX
        } else {
            dividend / divisor
        };
        self.set_gpr(rd, value)
    }

    pub(super) fn divuw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        #[cfg(isa32)]
        {
            let _ = (rd, rs1, rs2);
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        {
            let dividend = self.gpr[rs1] as u32;
            let divisor = self.gpr[rs2] as u32;
            let value = if divisor == 0 {
                u32::MAX
            } else {
                dividend / divisor
            };
            self.set_gpr(rd, (value as i32) as i64 as Word)
        }
    }

    pub(super) fn rem(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let dividend = self.gpr[rs1] as SWord;
        let divisor = self.gpr[rs2] as SWord;
        let value = if divisor == 0 {
            dividend as Word
        } else if dividend == SWord::MIN && divisor == -1 {
            0
        } else {
            (dividend % divisor) as Word
        };
        self.set_gpr(rd, value)
    }

    pub(super) fn remw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        #[cfg(isa32)]
        {
            let _ = (rd, rs1, rs2);
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        {
            let dividend = self.gpr[rs1] as i32;
            let divisor = self.gpr[rs2] as i32;
            let value = if divisor == 0 {
                dividend
            } else if dividend == i32::MIN && divisor == -1 {
                0
            } else {
                dividend % divisor
            };
            self.set_gpr(rd, value as i64 as Word)
        }
    }

    pub(super) fn remu(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        let dividend = self.gpr[rs1];
        let divisor = self.gpr[rs2];
        let value = if divisor == 0 {
            dividend
        } else {
            dividend % divisor
        };
        self.set_gpr(rd, value)
    }

    pub(super) fn remuw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        #[cfg(isa32)]
        {
            let _ = (rd, rs1, rs2);
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        {
            let dividend = self.gpr[rs1] as u32;
            let divisor = self.gpr[rs2] as u32;
            let value = if divisor == 0 {
                dividend
            } else {
                dividend % divisor
            };
            self.set_gpr(rd, (value as i32) as i64 as Word)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
