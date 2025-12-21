use memory_addr::{MemoryAddr, VirtAddr};

use super::RVCore;
use crate::{
    config::{SWord, Word},
    error::{XError, XResult},
    isa::RVReg,
    utils::{bit_u32, sext_word},
};

#[inline(always)]
fn bits(inst: u32, hi: u8, lo: u8) -> u32 {
    bit_u32(inst, hi, lo)
}

#[inline(always)]
fn reg(inst: u32, hi: u8, lo: u8) -> XResult<RVReg> {
    RVReg::from_u32(bits(inst, hi, lo))
}

#[inline(always)]
fn reg_prime(inst: u32, hi: u8, lo: u8) -> XResult<RVReg> {
    RVReg::from_u32(bits(inst, hi, lo) + 8)
}

#[inline(always)]
fn sext_imm(value: u32, bits: u32) -> SWord {
    sext_word(value as Word, bits) as SWord
}

impl RVCore {
    pub(super) fn c_addi4spn(&mut self, inst: u32) -> XResult {
        let rd = reg_prime(inst, 4, 2)?;
        let imm = (bits(inst, 10, 7) << 6)
            | (bits(inst, 12, 11) << 4)
            | (bits(inst, 6, 6) << 2)
            | (bits(inst, 5, 5) << 3);
        if imm == 0 {
            return Err(XError::InvalidInst);
        }
        self.imm_op(rd, RVReg::sp, imm as SWord, |lhs, imm| {
            lhs.wrapping_add(imm as Word)
        })
    }

    pub(super) fn c_lw(&mut self, inst: u32) -> XResult {
        let rd = reg_prime(inst, 4, 2)?;
        let rs1 = reg_prime(inst, 9, 7)?;
        let imm =
            (bits(inst, 12, 10) << 3) | (bits(inst, 6, 6) << 2) | (bits(inst, 5, 5) << 6);
        self.load_with(rd, rs1, imm as SWord, 4, |value| sext_word(value, 32))
    }

    pub(super) fn c_ld(&mut self, inst: u32) -> XResult {
        #[cfg(isa32)]
        {
            let _ = inst;
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        {
            let rd = reg_prime(inst, 4, 2)?;
            let rs1 = reg_prime(inst, 9, 7)?;
            let imm = (bits(inst, 12, 10) << 3) | (bits(inst, 6, 5) << 6);
            self.load_with(rd, rs1, imm as SWord, 8, |value| value)
        }
    }

    pub(super) fn c_sw(&mut self, inst: u32) -> XResult {
        let rs2 = reg_prime(inst, 4, 2)?;
        let rs1 = reg_prime(inst, 9, 7)?;
        let imm =
            (bits(inst, 12, 10) << 3) | (bits(inst, 6, 6) << 2) | (bits(inst, 5, 5) << 6);
        self.store(rs1, rs2, imm as SWord, 4)
    }

    pub(super) fn c_sd(&mut self, inst: u32) -> XResult {
        #[cfg(isa32)]
        {
            let _ = inst;
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        {
            let rs2 = reg_prime(inst, 4, 2)?;
            let rs1 = reg_prime(inst, 9, 7)?;
            let imm = (bits(inst, 12, 10) << 3) | (bits(inst, 6, 5) << 6);
            self.store(rs1, rs2, imm as SWord, 8)
        }
    }

    pub(super) fn c_nop(&mut self, _inst: u32) -> XResult {
        Ok(())
    }

    pub(super) fn c_addi(&mut self, inst: u32) -> XResult {
        let rd = reg(inst, 11, 7)?;
        let imm = sext_imm((bits(inst, 12, 12) << 5) | bits(inst, 6, 2), 6);
        self.imm_op(rd, rd, imm, |lhs, imm| lhs.wrapping_add(imm as Word))
    }

    pub(super) fn c_addiw(&mut self, inst: u32) -> XResult {
        #[cfg(isa32)]
        {
            let _ = inst;
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        {
            let rd = reg(inst, 11, 7)?;
            let imm = sext_imm((bits(inst, 12, 12) << 5) | bits(inst, 6, 2), 6);
            let value = (self.gpr[rd] as i64).wrapping_add(imm as i64) as i32;
            self.set_gpr(rd, value as i64 as Word)
        }
    }

    pub(super) fn c_li(&mut self, inst: u32) -> XResult {
        let rd = reg(inst, 11, 7)?;
        let imm = sext_imm((bits(inst, 12, 12) << 5) | bits(inst, 6, 2), 6);
        self.set_gpr(rd, imm as Word)
    }

    pub(super) fn c_addi16sp(&mut self, inst: u32) -> XResult {
        let imm = (bits(inst, 12, 12) << 9)
            | (bits(inst, 6, 6) << 4)
            | (bits(inst, 5, 5) << 6)
            | (bits(inst, 4, 4) << 8)
            | (bits(inst, 3, 3) << 7)
            | (bits(inst, 2, 2) << 5);
        if imm == 0 {
            return Err(XError::InvalidInst);
        }
        let imm = sext_imm(imm, 10);
        self.imm_op(RVReg::sp, RVReg::sp, imm, |lhs, imm| {
            lhs.wrapping_add(imm as Word)
        })
    }

    pub(super) fn c_lui(&mut self, inst: u32) -> XResult {
        let rd = reg(inst, 11, 7)?;
        let imm = sext_imm((bits(inst, 12, 12) << 5) | bits(inst, 6, 2), 6);
        self.set_gpr(rd, (imm << 12) as Word)
    }

    pub(super) fn c_srli(&mut self, inst: u32) -> XResult {
        #[cfg(isa32)]
        if bits(inst, 12, 12) != 0 {
            return Err(XError::InvalidInst);
        }
        let rd = reg_prime(inst, 9, 7)?;
        let shamt = (bits(inst, 12, 12) << 5) | bits(inst, 6, 2);
        self.imm_op(rd, rd, shamt as SWord, |lhs, imm| {
            lhs >> Self::shamt_from_imm(imm)
        })
    }

    pub(super) fn c_srai(&mut self, inst: u32) -> XResult {
        #[cfg(isa32)]
        if bits(inst, 12, 12) != 0 {
            return Err(XError::InvalidInst);
        }
        let rd = reg_prime(inst, 9, 7)?;
        let shamt = (bits(inst, 12, 12) << 5) | bits(inst, 6, 2);
        self.imm_op(rd, rd, shamt as SWord, |lhs, imm| {
            ((lhs as SWord) >> Self::shamt_from_imm(imm)) as Word
        })
    }

    pub(super) fn c_andi(&mut self, inst: u32) -> XResult {
        let rd = reg_prime(inst, 9, 7)?;
        let imm = sext_imm((bits(inst, 12, 12) << 5) | bits(inst, 6, 2), 6);
        self.imm_op(rd, rd, imm, |lhs, imm| lhs & (imm as Word))
    }

    pub(super) fn c_sub(&mut self, inst: u32) -> XResult {
        let rd = reg_prime(inst, 9, 7)?;
        let rs2 = reg_prime(inst, 4, 2)?;
        self.binary_op(rd, rd, rs2, |lhs, rhs| lhs.wrapping_sub(rhs))
    }

    pub(super) fn c_xor(&mut self, inst: u32) -> XResult {
        let rd = reg_prime(inst, 9, 7)?;
        let rs2 = reg_prime(inst, 4, 2)?;
        self.binary_op(rd, rd, rs2, |lhs, rhs| lhs ^ rhs)
    }

    pub(super) fn c_or(&mut self, inst: u32) -> XResult {
        let rd = reg_prime(inst, 9, 7)?;
        let rs2 = reg_prime(inst, 4, 2)?;
        self.binary_op(rd, rd, rs2, |lhs, rhs| lhs | rhs)
    }

    pub(super) fn c_and(&mut self, inst: u32) -> XResult {
        let rd = reg_prime(inst, 9, 7)?;
        let rs2 = reg_prime(inst, 4, 2)?;
        self.binary_op(rd, rd, rs2, |lhs, rhs| lhs & rhs)
    }

    pub(super) fn c_subw(&mut self, inst: u32) -> XResult {
        #[cfg(isa32)]
        {
            let _ = inst;
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        {
            let rd = reg_prime(inst, 9, 7)?;
            let rs2 = reg_prime(inst, 4, 2)?;
            let value = (self.gpr[rd] as i64).wrapping_sub(self.gpr[rs2] as i64) as i32;
            self.set_gpr(rd, value as i64 as Word)
        }
    }

    pub(super) fn c_addw(&mut self, inst: u32) -> XResult {
        #[cfg(isa32)]
        {
            let _ = inst;
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        {
            let rd = reg_prime(inst, 9, 7)?;
            let rs2 = reg_prime(inst, 4, 2)?;
            let value = (self.gpr[rd] as i64).wrapping_add(self.gpr[rs2] as i64) as i32;
            self.set_gpr(rd, value as i64 as Word)
        }
    }

    pub(super) fn c_j(&mut self, inst: u32) -> XResult {
        let imm = (bits(inst, 12, 12) << 11)
            | (bits(inst, 11, 11) << 4)
            | (bits(inst, 10, 9) << 8)
            | (bits(inst, 8, 8) << 10)
            | (bits(inst, 7, 7) << 6)
            | (bits(inst, 6, 6) << 7)
            | (bits(inst, 5, 3) << 1)
            | (bits(inst, 2, 2) << 5);
        let imm = sext_imm(imm, 12);
        self.npc = self.pc.wrapping_add(imm as _);
        Ok(())
    }

    pub(super) fn c_beqz(&mut self, inst: u32) -> XResult {
        let rs1 = reg_prime(inst, 9, 7)?;
        let imm = (bits(inst, 12, 12) << 8)
            | (bits(inst, 6, 5) << 6)
            | (bits(inst, 2, 2) << 5)
            | (bits(inst, 11, 10) << 3)
            | (bits(inst, 4, 3) << 1);
        let imm = sext_imm(imm, 9);
        self.branch(rs1, RVReg::zero, imm, |lhs, rhs| lhs == rhs)
    }

    pub(super) fn c_bnez(&mut self, inst: u32) -> XResult {
        let rs1 = reg_prime(inst, 9, 7)?;
        let imm = (bits(inst, 12, 12) << 8)
            | (bits(inst, 6, 5) << 6)
            | (bits(inst, 2, 2) << 5)
            | (bits(inst, 11, 10) << 3)
            | (bits(inst, 4, 3) << 1);
        let imm = sext_imm(imm, 9);
        self.branch(rs1, RVReg::zero, imm, |lhs, rhs| lhs != rhs)
    }

    pub(super) fn c_slli(&mut self, inst: u32) -> XResult {
        #[cfg(isa32)]
        if bits(inst, 12, 12) != 0 {
            return Err(XError::InvalidInst);
        }
        let rd = reg(inst, 11, 7)?;
        let shamt = (bits(inst, 12, 12) << 5) | bits(inst, 6, 2);
        self.imm_op(rd, rd, shamt as SWord, |lhs, imm| {
            lhs << Self::shamt_from_imm(imm)
        })
    }

    pub(super) fn c_lwsp(&mut self, inst: u32) -> XResult {
        let rd = reg(inst, 11, 7)?;
        let imm = (bits(inst, 12, 12) << 5)
            | (bits(inst, 6, 4) << 2)
            | (bits(inst, 3, 2) << 6);
        self.load_with(rd, RVReg::sp, imm as SWord, 4, |value| sext_word(value, 32))
    }

    pub(super) fn c_ldsp(&mut self, inst: u32) -> XResult {
        #[cfg(isa32)]
        {
            let _ = inst;
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        {
            let rd = reg(inst, 11, 7)?;
            let imm = (bits(inst, 12, 12) << 5)
                | (bits(inst, 6, 5) << 3)
                | (bits(inst, 4, 2) << 6);
            self.load_with(rd, RVReg::sp, imm as SWord, 8, |value| value)
        }
    }

    pub(super) fn c_jr(&mut self, inst: u32) -> XResult {
        let rs1 = reg(inst, 11, 7)?;
        if rs1 == RVReg::zero {
            return Err(XError::InvalidInst);
        }
        let target = self.gpr[rs1] & !1;
        self.npc = VirtAddr::from_usize(target as usize);
        Ok(())
    }

    pub(super) fn c_mv(&mut self, inst: u32) -> XResult {
        let rd = reg(inst, 11, 7)?;
        let rs2 = reg(inst, 6, 2)?;
        if rs2 == RVReg::zero {
            return Err(XError::InvalidInst);
        }
        self.set_gpr(rd, self.gpr[rs2])
    }

    pub(super) fn c_ebreak(&mut self, _inst: u32) -> XResult {
        Err(XError::ToTerminate)
    }

    pub(super) fn c_jalr(&mut self, inst: u32) -> XResult {
        let rs1 = reg(inst, 11, 7)?;
        if rs1 == RVReg::zero {
            return Err(XError::InvalidInst);
        }
        let link = self.pc.wrapping_add(2);
        let target = (self.gpr[rs1]) & !1;
        self.set_gpr(RVReg::ra, link.as_usize() as Word)?;
        self.npc = VirtAddr::from_usize(target as usize);
        Ok(())
    }

    pub(super) fn c_add(&mut self, inst: u32) -> XResult {
        let rd = reg(inst, 11, 7)?;
        let rs2 = reg(inst, 6, 2)?;
        if rs2 == RVReg::zero {
            return Err(XError::InvalidInst);
        }
        self.binary_op(rd, rd, rs2, |lhs, rhs| lhs.wrapping_add(rhs))
    }

    pub(super) fn c_swsp(&mut self, inst: u32) -> XResult {
        let rs2 = reg(inst, 6, 2)?;
        let imm = (bits(inst, 12, 9) << 2) | (bits(inst, 8, 7) << 6);
        self.store(RVReg::sp, rs2, imm as SWord, 4)
    }

    pub(super) fn c_sdsp(&mut self, inst: u32) -> XResult {
        #[cfg(isa32)]
        {
            let _ = inst;
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        {
            let rs2 = reg(inst, 6, 2)?;
            let imm = (bits(inst, 12, 10) << 3) | (bits(inst, 9, 7) << 6);
            self.store(RVReg::sp, rs2, imm as SWord, 8)
        }
    }
}
