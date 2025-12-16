use memory_addr::{MemoryAddr, VirtAddr};

use super::RVCore;
use crate::{
    config::{SWord, Word, word_to_shamt},
    error::XResult,
    isa::RVReg,
    utils::sext_word,
    with_mem,
};

impl RVCore {
    #[inline(always)]
    fn eff_addr(&self, base: RVReg, offset: SWord) -> VirtAddr {
        let addr = self.gpr[base].wrapping_add(offset as Word);
        VirtAddr::from_usize(addr as usize)
    }

    #[inline(always)]
    fn shamt_from_word(value: Word) -> u32 {
        word_to_shamt(value)
    }

    #[inline(always)]
    fn shamt_from_imm(imm: SWord) -> u32 {
        word_to_shamt(imm as Word)
    }

    #[inline(always)]
    fn binary_op<F>(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, op: F) -> XResult
    where
        F: FnOnce(Word, Word) -> Word,
    {
        let value = op(self.gpr[rs1], self.gpr[rs2]);
        self.set_gpr(rd, value)
    }

    #[inline(always)]
    fn imm_op<F>(&mut self, rd: RVReg, rs1: RVReg, imm: SWord, op: F) -> XResult
    where
        F: FnOnce(Word, SWord) -> Word,
    {
        let value = op(self.gpr[rs1], imm);
        self.set_gpr(rd, value)
    }

    #[inline(always)]
    fn load_with<F>(&mut self, rd: RVReg, rs1: RVReg, imm: SWord, size: usize, extend: F) -> XResult
    where
        F: FnOnce(Word) -> Word,
    {
        let addr = self.eff_addr(rs1, imm);
        let value = with_mem!(read(self.virt_to_phys(addr), size))?;
        self.set_gpr(rd, extend(value))
    }

    #[inline(always)]
    fn store(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord, size: usize) -> XResult {
        let addr = self.eff_addr(rs1, imm);
        let mask: Word = match size {
            1 => 0xFF,
            2 => 0xFFFF,
            4 => 0xFFFF_FFFF,
            8 => Word::MAX,
            _ => unreachable!("unsupported store size"),
        };
        let value = self.gpr[rs2] & mask;
        with_mem!(write(self.virt_to_phys(addr), size, value))?;
        Ok(())
    }

    #[inline(always)]
    fn branch<F>(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord, cond: F) -> XResult
    where
        F: FnOnce(Word, Word) -> bool,
    {
        if cond(self.gpr[rs1], self.gpr[rs2]) {
            self.npc = self.pc.wrapping_add(imm as _);
        }
        Ok(())
    }
}

impl RVCore {
    pub(super) fn add(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |lhs, rhs| lhs.wrapping_add(rhs))
    }

    pub(super) fn sub(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |lhs, rhs| lhs.wrapping_sub(rhs))
    }

    pub(super) fn sll(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |lhs, rhs| lhs << Self::shamt_from_word(rhs))
    }

    pub(super) fn slt(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |lhs, rhs| {
            ((lhs as SWord) < (rhs as SWord)) as Word
        })
    }

    pub(super) fn sltu(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |lhs, rhs| (lhs < rhs) as Word)
    }

    pub(super) fn xor(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |lhs, rhs| lhs ^ rhs)
    }

    pub(super) fn srl(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |lhs, rhs| lhs >> Self::shamt_from_word(rhs))
    }

    pub(super) fn sra(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |lhs, rhs| {
            ((lhs as SWord) >> Self::shamt_from_word(rhs)) as Word
        })
    }

    pub(super) fn or(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |lhs, rhs| lhs | rhs)
    }

    pub(super) fn and(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |lhs, rhs| lhs & rhs)
    }

    pub(super) fn addi(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |lhs, imm| lhs.wrapping_add(imm as Word))
    }

    pub(super) fn slli(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |lhs, imm| lhs << Self::shamt_from_imm(imm))
    }

    pub(super) fn slti(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |lhs, imm| ((lhs as SWord) < imm) as Word)
    }

    pub(super) fn sltiu(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |lhs, imm| (lhs < imm as Word) as Word)
    }

    pub(super) fn xori(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |lhs, imm| lhs ^ imm as Word)
    }

    pub(super) fn srli(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |lhs, imm| lhs >> Self::shamt_from_imm(imm))
    }

    pub(super) fn srla(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |lhs, imm| {
            ((lhs as SWord) >> Self::shamt_from_imm(imm)) as Word
        })
    }

    pub(super) fn ori(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |lhs, imm| lhs | imm as Word)
    }

    pub(super) fn andi(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |lhs, imm| lhs & (imm as Word))
    }

    pub(super) fn lb(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.load_with(rd, rs1, imm, 1, |value| sext_word(value, 8))
    }

    pub(super) fn lh(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.load_with(rd, rs1, imm, 2, |value| sext_word(value, 16))
    }

    pub(super) fn lw(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.load_with(rd, rs1, imm, 4, |value| sext_word(value, 32))
    }

    pub(super) fn lbu(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.load_with(rd, rs1, imm, 1, |value| value & 0xFF)
    }

    pub(super) fn lhu(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.load_with(rd, rs1, imm, 2, |value| value & 0xFFFF)
    }

    pub(super) fn sb(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.store(rs1, rs2, imm, 1)
    }

    pub(super) fn sh(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.store(rs1, rs2, imm, 2)
    }

    pub(super) fn sw(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.store(rs1, rs2, imm, 4)
    }

    pub(super) fn beq(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.branch(rs1, rs2, imm, |lhs, rhs| lhs == rhs)
    }

    pub(super) fn bne(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.branch(rs1, rs2, imm, |lhs, rhs| lhs != rhs)
    }

    pub(super) fn blt(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.branch(rs1, rs2, imm, |lhs, rhs| (lhs as SWord) < (rhs as SWord))
    }

    pub(super) fn bge(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.branch(rs1, rs2, imm, |lhs, rhs| (lhs as SWord) >= (rhs as SWord))
    }

    pub(super) fn bltu(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.branch(rs1, rs2, imm, |lhs, rhs| lhs < rhs)
    }

    pub(super) fn bgeu(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.branch(rs1, rs2, imm, |lhs, rhs| lhs >= rhs)
    }

    pub(super) fn jal(&mut self, rd: RVReg, imm: SWord) -> XResult {
        let link = self.pc.wrapping_add(4);
        self.set_gpr(rd, link.as_usize() as Word)?;
        self.npc = self.pc.wrapping_add(imm as _);
        Ok(())
    }

    pub(super) fn jalr(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        let link = self.pc.wrapping_add(4);
        self.set_gpr(rd, link.as_usize() as Word)?;
        let target = (self.gpr[rs1].wrapping_add(imm as Word)) & !1;
        self.npc = VirtAddr::from_usize(target as usize);
        Ok(())
    }

    pub(super) fn lui(&mut self, rd: RVReg, imm: SWord) -> XResult {
        self.set_gpr(rd, imm as Word)
    }

    pub(super) fn auipc(&mut self, rd: RVReg, imm: SWord) -> XResult {
        let base = self.pc.as_usize() as Word;
        self.set_gpr(rd, base.wrapping_add(imm as Word))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CONFIG_MBASE;

    const TEST_BASE: usize = CONFIG_MBASE + 0x1000;

    fn phys(core: &RVCore, offset: usize) -> memory_addr::PhysAddr {
        core.virt_to_phys(VirtAddr::from(TEST_BASE + offset))
    }

    fn write_bytes(core: &RVCore, offset: usize, bytes: &[u8]) {
        with_mem!(load(phys(core, offset), bytes)).expect("write_bytes failed");
    }

    fn read_word(core: &RVCore, offset: usize, size: usize) -> Word {
        with_mem!(read(phys(core, offset), size)).expect("read_word failed")
    }

    #[test]
    fn add_and_zero_register_are_correct() {
        let mut core = RVCore::new();
        core.gpr[RVReg::t0] = 5;
        core.gpr[RVReg::t1] = 7;

        core.add(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], 12);

        core.add(RVReg::zero, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::zero], 0);
    }

    #[test]
    fn shifts_mask_shamt() {
        let mut core = RVCore::new();
        core.gpr[RVReg::t0] = 1;
        core.gpr[RVReg::t1] = crate::config::SHAMT_MASK + 0x40;

        core.sll(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        let expected = (1 as Word) << ((core.gpr[RVReg::t1] & crate::config::SHAMT_MASK) as u32);
        assert_eq!(core.gpr[RVReg::t2], expected);

        core.srl(RVReg::t3, RVReg::t2, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t3], 1);
    }

    #[test]
    fn load_instructions_handle_sign_extension() {
        let mut core = RVCore::new();
        core.gpr[RVReg::t0] = TEST_BASE as Word;

        write_bytes(&core, 0, &[0x80]);
        core.lb(RVReg::t1, RVReg::t0, 0).unwrap();
        assert_eq!(core.gpr[RVReg::t1], sext_word(0x80, 8));

        core.lbu(RVReg::t2, RVReg::t0, 0).unwrap();
        assert_eq!(core.gpr[RVReg::t2], 0x80);

        write_bytes(&core, 4, &[0x00, 0x80]);
        core.lh(RVReg::t3, RVReg::t0, 4).unwrap();
        assert_eq!(core.gpr[RVReg::t3], sext_word(0x8000, 16));

        core.lhu(RVReg::t4, RVReg::t0, 4).unwrap();
        assert_eq!(core.gpr[RVReg::t4], 0x8000);

        write_bytes(&core, 8, &[0x78, 0x56, 0x34, 0x12]);
        core.lw(RVReg::t5, RVReg::t0, 8).unwrap();
        assert_eq!(core.gpr[RVReg::t5], sext_word(0x12345678, 32));
    }

    #[test]
    fn store_instructions_truncate_values() {
        let mut core = RVCore::new();
        core.gpr[RVReg::t0] = TEST_BASE as Word;
        core.gpr[RVReg::t1] = 0xDEADBEEF;

        core.sb(RVReg::t0, RVReg::t1, 0x100).unwrap();
        assert_eq!(read_word(&core, 0x100, 1) & 0xFF, 0xEF);

        core.sh(RVReg::t0, RVReg::t1, 0x102).unwrap();
        assert_eq!(read_word(&core, 0x102, 2) & 0xFFFF, 0xBEEF);

        core.sw(RVReg::t0, RVReg::t1, 0x104).unwrap();
        assert_eq!(read_word(&core, 0x104, 4), 0xDEADBEEF & 0xFFFF_FFFF);
    }

    #[test]
    fn branches_update_npc_when_taken() {
        let mut core = RVCore::new();
        core.pc = VirtAddr::from(TEST_BASE);
        core.npc = core.pc.wrapping_add(4);

        core.gpr[RVReg::t0] = 1;
        core.gpr[RVReg::t1] = 1;
        let offset: SWord = 16;
        core.beq(RVReg::t0, RVReg::t1, offset).unwrap();
        assert_eq!(core.npc, core.pc.wrapping_add(offset as usize));

        core.npc = core.pc.wrapping_add(4);
        core.gpr[RVReg::t1] = 2;
        core.beq(RVReg::t0, RVReg::t1, offset).unwrap();
        assert_eq!(core.npc, core.pc.wrapping_add(4));
    }

    #[test]
    fn jal_and_jalr_produce_correct_targets() {
        let mut core = RVCore::new();
        core.pc = VirtAddr::from(TEST_BASE);
        core.npc = core.pc;

        let imm: SWord = 20;
        core.jal(RVReg::ra, imm).unwrap();
        assert_eq!(
            core.gpr[RVReg::ra],
            core.pc.wrapping_add(4).as_usize() as Word
        );
        assert_eq!(core.npc, core.pc.wrapping_add(imm as usize));

        core.gpr[RVReg::t0] = (TEST_BASE + 0x123) as Word;
        core.jalr(RVReg::t1, RVReg::t0, 3).unwrap();
        assert_eq!(
            core.gpr[RVReg::t1],
            core.pc.wrapping_add(4).as_usize() as Word
        );
        assert_eq!(
            core.npc,
            VirtAddr::from(((TEST_BASE + 0x123 + 3) & !1) as usize)
        );
    }
}
