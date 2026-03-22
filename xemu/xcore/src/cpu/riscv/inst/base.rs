// cfg(isa32) blocks use `return` before cfg(isa64) alternatives
#![allow(clippy::needless_return)]

use memory_addr::{MemoryAddr, VirtAddr};

use super::RVCore;
#[cfg(isa32)]
use crate::error::XError;
use crate::{
    config::{SWord, Word, word_to_shamt},
    cpu::MemOps,
    error::XResult,
    isa::RVReg,
    memory::with_mem,
    utils::sext_word,
};

/// RV64-only word-width operation. On RV32, returns `InvalidInst`.
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
    #[inline(always)]
    pub(super) fn eff_addr(&self, base: RVReg, offset: SWord) -> VirtAddr {
        let addr = self.gpr[base].wrapping_add(offset as Word);
        VirtAddr::from_usize(addr as usize)
    }

    #[inline(always)]
    pub(super) fn shamt_from_word(value: Word) -> u32 {
        word_to_shamt(value)
    }

    #[inline(always)]
    pub(super) fn shamt_from_imm(imm: SWord) -> u32 {
        word_to_shamt(imm as Word)
    }

    #[inline(always)]
    pub(super) fn binary_op<F>(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, op: F) -> XResult
    where
        F: FnOnce(Word, Word) -> Word,
    {
        self.set_gpr(rd, op(self.gpr[rs1], self.gpr[rs2]))
    }

    #[inline(always)]
    pub(super) fn imm_op<F>(&mut self, rd: RVReg, rs1: RVReg, imm: SWord, op: F) -> XResult
    where
        F: FnOnce(Word, SWord) -> Word,
    {
        self.set_gpr(rd, op(self.gpr[rs1], imm))
    }

    #[inline(always)]
    pub(super) fn load_with<F>(
        &mut self,
        rd: RVReg,
        rs1: RVReg,
        imm: SWord,
        size: usize,
        extend: F,
    ) -> XResult
    where
        F: FnOnce(Word) -> Word,
    {
        let addr = self.eff_addr(rs1, imm);
        let value = with_mem!(read(self.virt_to_phys(addr), size))?;
        self.set_gpr(rd, extend(value))
    }

    #[inline(always)]
    pub(super) fn store(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord, size: usize) -> XResult {
        let addr = self.eff_addr(rs1, imm);
        let mask = if size >= std::mem::size_of::<Word>() {
            Word::MAX
        } else {
            (1 as Word).wrapping_shl(size as u32 * 8) - 1
        };
        with_mem!(write(self.virt_to_phys(addr), size, self.gpr[rs2] & mask))?;
        Ok(())
    }

    #[inline(always)]
    pub(super) fn branch<F>(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord, cond: F) -> XResult
    where
        F: FnOnce(Word, Word) -> bool,
    {
        if cond(self.gpr[rs1], self.gpr[rs2]) {
            self.npc = self.pc.wrapping_add(imm as _);
        }
        Ok(())
    }
}

// --- R-type ---

impl RVCore {
    pub(super) fn add(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |a, b| a.wrapping_add(b))
    }

    pub(super) fn addw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        rv64_op!(self, rd, |rs1, rs2| (self.gpr[rs1] as i32)
            .wrapping_add(self.gpr[rs2] as i32))
    }

    pub(super) fn sub(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |a, b| a.wrapping_sub(b))
    }

    pub(super) fn subw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        rv64_op!(self, rd, |rs1, rs2| (self.gpr[rs1] as i32)
            .wrapping_sub(self.gpr[rs2] as i32))
    }

    pub(super) fn sll(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |a, b| a << Self::shamt_from_word(b))
    }

    pub(super) fn sllw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        rv64_op!(
            self,
            rd,
            |rs1, rs2| ((self.gpr[rs1] as u32) << ((self.gpr[rs2] & 0x1F) as u32)) as i32
        )
    }

    pub(super) fn slt(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |a, b| ((a as SWord) < (b as SWord)) as Word)
    }

    pub(super) fn sltu(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |a, b| (a < b) as Word)
    }

    pub(super) fn xor(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |a, b| a ^ b)
    }

    pub(super) fn srl(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |a, b| a >> Self::shamt_from_word(b))
    }

    pub(super) fn srlw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        rv64_op!(
            self,
            rd,
            |rs1, rs2| ((self.gpr[rs1] as u32) >> ((self.gpr[rs2] & 0x1F) as u32)) as i32
        )
    }

    pub(super) fn sra(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |a, b| {
            ((a as SWord) >> Self::shamt_from_word(b)) as Word
        })
    }

    pub(super) fn sraw(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        rv64_op!(self, rd, |rs1, rs2| (self.gpr[rs1] as i32)
            >> ((self.gpr[rs2] & 0x1F) as u32))
    }

    pub(super) fn or(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |a, b| a | b)
    }

    pub(super) fn and(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.binary_op(rd, rs1, rs2, |a, b| a & b)
    }
}

// --- I-type ---

impl RVCore {
    pub(super) fn addi(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |a, imm| a.wrapping_add(imm as Word))
    }

    pub(super) fn addiw(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        rv64_op!(self, rd, |rs1, imm| (self.gpr[rs1] as i32)
            .wrapping_add(imm as i32))
    }

    pub(super) fn slli(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |a, imm| a << Self::shamt_from_imm(imm))
    }

    pub(super) fn slliw(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        rv64_op!(
            self,
            rd,
            |rs1, imm| ((self.gpr[rs1] as u32) << ((imm as u32) & 0x1F)) as i32
        )
    }

    pub(super) fn slti(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |a, imm| ((a as SWord) < imm) as Word)
    }

    pub(super) fn sltiu(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |a, imm| (a < imm as Word) as Word)
    }

    pub(super) fn xori(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |a, imm| a ^ imm as Word)
    }

    pub(super) fn srli(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |a, imm| a >> Self::shamt_from_imm(imm))
    }

    pub(super) fn srliw(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        rv64_op!(
            self,
            rd,
            |rs1, imm| ((self.gpr[rs1] as u32) >> ((imm as u32) & 0x1F)) as i32
        )
    }

    pub(super) fn srai(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |a, imm| {
            ((a as SWord) >> Self::shamt_from_imm(imm)) as Word
        })
    }

    pub(super) fn sraiw(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        rv64_op!(self, rd, |rs1, imm| (self.gpr[rs1] as i32)
            >> ((imm as u32) & 0x1F))
    }

    pub(super) fn ori(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |a, imm| a | imm as Word)
    }

    pub(super) fn andi(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.imm_op(rd, rs1, imm, |a, imm| a & (imm as Word))
    }
}

// --- Load/Store ---

impl RVCore {
    pub(super) fn lb(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.load_with(rd, rs1, imm, 1, |v| sext_word(v, 8))
    }

    pub(super) fn lh(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.load_with(rd, rs1, imm, 2, |v| sext_word(v, 16))
    }

    pub(super) fn lw(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.load_with(rd, rs1, imm, 4, |v| sext_word(v, 32))
    }

    pub(super) fn ld(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.load_with(rd, rs1, imm, 8, |v| v)
    }

    pub(super) fn lbu(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.load_with(rd, rs1, imm, 1, |v| v & 0xFF)
    }

    pub(super) fn lhu(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.load_with(rd, rs1, imm, 2, |v| v & 0xFFFF)
    }

    pub(super) fn lwu(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        #[cfg(isa32)]
        {
            let _ = (rd, rs1, imm);
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        self.load_with(rd, rs1, imm, 4, |v| v & 0xFFFF_FFFF)
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

    pub(super) fn sd(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        #[cfg(isa32)]
        {
            let _ = (rs1, rs2, imm);
            return Err(XError::InvalidInst);
        }
        #[cfg(isa64)]
        self.store(rs1, rs2, imm, 8)
    }
}

// --- Branch/Jump ---

impl RVCore {
    pub(super) fn beq(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.branch(rs1, rs2, imm, |a, b| a == b)
    }

    pub(super) fn bne(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.branch(rs1, rs2, imm, |a, b| a != b)
    }

    pub(super) fn blt(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.branch(rs1, rs2, imm, |a, b| (a as SWord) < (b as SWord))
    }

    pub(super) fn bge(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.branch(rs1, rs2, imm, |a, b| (a as SWord) >= (b as SWord))
    }

    pub(super) fn bltu(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.branch(rs1, rs2, imm, |a, b| a < b)
    }

    pub(super) fn bgeu(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.branch(rs1, rs2, imm, |a, b| a >= b)
    }

    pub(super) fn jal(&mut self, rd: RVReg, imm: SWord) -> XResult {
        let link = self.pc.wrapping_add(4);
        self.set_gpr(rd, link.as_usize() as Word)?;
        self.npc = self.pc.wrapping_add(imm as _);
        Ok(())
    }

    pub(super) fn jalr(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        let link = self.pc.wrapping_add(4);
        let target = (self.gpr[rs1].wrapping_add(imm as Word)) & !1;
        self.set_gpr(rd, link.as_usize() as Word)?;
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
    fn sub_produces_correct_result() {
        let mut core = RVCore::new();
        core.gpr[RVReg::t0] = 10;
        core.gpr[RVReg::t1] = 3;
        core.sub(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], 7);

        // Wrapping underflow
        core.gpr[RVReg::t0] = 0;
        core.gpr[RVReg::t1] = 1;
        core.sub(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], Word::MAX);
    }

    #[test]
    fn logical_ops_work() {
        let mut core = RVCore::new();
        core.gpr[RVReg::t0] = 0xFF00;
        core.gpr[RVReg::t1] = 0x0FF0;

        core.and(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], 0x0F00);

        core.or(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], 0xFFF0);

        core.xor(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], 0xF0F0);
    }

    #[test]
    fn slt_and_sltu_compare_correctly() {
        let mut core = RVCore::new();
        // Signed: -1 < 1
        core.gpr[RVReg::t0] = (-1 as SWord) as Word;
        core.gpr[RVReg::t1] = 1;
        core.slt(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], 1);

        // Unsigned: MAX > 1
        core.sltu(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], 0); // MAX > 1 unsigned

        // Equal values
        core.gpr[RVReg::t1] = core.gpr[RVReg::t0];
        core.slt(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2], 0);
    }

    #[test]
    fn immediate_ops_work() {
        let mut core = RVCore::new();
        core.gpr[RVReg::t0] = 100;

        core.addi(RVReg::t1, RVReg::t0, -50).unwrap();
        assert_eq!(core.gpr[RVReg::t1], 50);

        core.xori(RVReg::t1, RVReg::t0, -1).unwrap();
        assert_eq!(core.gpr[RVReg::t1], !100);

        core.ori(RVReg::t1, RVReg::t0, 0xF).unwrap();
        assert_eq!(core.gpr[RVReg::t1], 100 | 0xF);

        core.andi(RVReg::t1, RVReg::t0, 0xF0 as SWord).unwrap();
        assert_eq!(core.gpr[RVReg::t1], 100 & 0xF0);

        core.slti(RVReg::t1, RVReg::t0, 200).unwrap();
        assert_eq!(core.gpr[RVReg::t1], 1);
        core.slti(RVReg::t1, RVReg::t0, 50).unwrap();
        assert_eq!(core.gpr[RVReg::t1], 0);

        core.sltiu(RVReg::t1, RVReg::t0, -1).unwrap();
        assert_eq!(core.gpr[RVReg::t1], 1); // 100 < MAX unsigned
    }

    #[test]
    fn shift_immediate_ops() {
        let mut core = RVCore::new();
        core.gpr[RVReg::t0] = 0x10;

        core.slli(RVReg::t1, RVReg::t0, 4).unwrap();
        assert_eq!(core.gpr[RVReg::t1], 0x100);

        core.srli(RVReg::t1, RVReg::t0, 2).unwrap();
        assert_eq!(core.gpr[RVReg::t1], 0x4);

        // Arithmetic right shift preserves sign
        core.gpr[RVReg::t0] = (-16 as SWord) as Word;
        core.srai(RVReg::t1, RVReg::t0, 2).unwrap();
        assert_eq!(core.gpr[RVReg::t1] as SWord, -4);
    }

    #[test]
    fn lui_and_auipc_produce_correct_values() {
        let mut core = RVCore::new();
        core.pc = VirtAddr::from(TEST_BASE);

        core.lui(RVReg::t0, 0x12345000 as SWord).unwrap();
        assert_eq!(core.gpr[RVReg::t0], 0x12345000 as Word);

        core.auipc(RVReg::t1, 0x1000 as SWord).unwrap();
        assert_eq!(
            core.gpr[RVReg::t1],
            (TEST_BASE as Word).wrapping_add(0x1000)
        );
    }

    #[test]
    fn all_branch_variants() {
        let mut core = RVCore::new();
        core.pc = VirtAddr::from(TEST_BASE);
        let offset: SWord = 100;

        // bne: taken when not equal
        core.gpr[RVReg::t0] = 1;
        core.gpr[RVReg::t1] = 2;
        core.npc = core.pc.wrapping_add(4);
        core.bne(RVReg::t0, RVReg::t1, offset).unwrap();
        assert_eq!(core.npc, core.pc.wrapping_add(offset as usize));

        // bne: not taken when equal
        core.gpr[RVReg::t1] = 1;
        core.npc = core.pc.wrapping_add(4);
        core.bne(RVReg::t0, RVReg::t1, offset).unwrap();
        assert_eq!(core.npc, core.pc.wrapping_add(4));

        // blt: signed less than
        core.gpr[RVReg::t0] = (-5 as SWord) as Word;
        core.gpr[RVReg::t1] = 3;
        core.npc = core.pc.wrapping_add(4);
        core.blt(RVReg::t0, RVReg::t1, offset).unwrap();
        assert_eq!(core.npc, core.pc.wrapping_add(offset as usize));

        // bge: signed >=
        core.npc = core.pc.wrapping_add(4);
        core.bge(RVReg::t1, RVReg::t0, offset).unwrap();
        assert_eq!(core.npc, core.pc.wrapping_add(offset as usize));

        // bltu: unsigned less than
        core.gpr[RVReg::t0] = 5;
        core.gpr[RVReg::t1] = Word::MAX;
        core.npc = core.pc.wrapping_add(4);
        core.bltu(RVReg::t0, RVReg::t1, offset).unwrap();
        assert_eq!(core.npc, core.pc.wrapping_add(offset as usize));

        // bgeu: unsigned >=
        core.npc = core.pc.wrapping_add(4);
        core.bgeu(RVReg::t1, RVReg::t0, offset).unwrap();
        assert_eq!(core.npc, core.pc.wrapping_add(offset as usize));
    }

    #[test]
    #[cfg(isa64)]
    fn addw_sign_extends_result() {
        let mut core = RVCore::new();
        // 0x7FFFFFFF + 1 = 0x80000000 which sign-extends to 0xFFFFFFFF_80000000
        core.gpr[RVReg::t0] = 0x7FFFFFFF;
        core.gpr[RVReg::t1] = 1;
        core.addw(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        assert_eq!(core.gpr[RVReg::t2] as i64, -2147483648_i64); // 0x80000000 sign-extended
    }

    #[test]
    #[cfg(isa64)]
    fn sllw_sign_extends_result() {
        let mut core = RVCore::new();
        core.gpr[RVReg::t0] = 1;
        core.gpr[RVReg::t1] = 31;
        core.sllw(RVReg::t2, RVReg::t0, RVReg::t1).unwrap();
        // 1 << 31 = 0x80000000, sign-extended to 0xFFFFFFFF_80000000
        assert_eq!(core.gpr[RVReg::t2] as i64, -2147483648_i64);
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
        core.jalr(RVReg::t1, RVReg::t0, 2).unwrap();
        assert_eq!(
            core.gpr[RVReg::t1],
            core.pc.wrapping_add(4).as_usize() as Word
        );
        assert_eq!(
            core.npc,
            VirtAddr::from(((TEST_BASE + 0x123 + 2) & !1) as usize)
        );
    }

    #[test]
    #[cfg(isa32)]
    fn rv64_only_base_instructions_are_rejected_on_rv32() {
        let mut core = RVCore::new();
        let r = (RVReg::t0, RVReg::t1, RVReg::t2);

        for result in [
            core.addw(r.0, r.1, r.2),
            core.subw(r.0, r.1, r.2),
            core.sllw(r.0, r.1, r.2),
            core.srlw(r.0, r.1, r.2),
            core.sraw(r.0, r.1, r.2),
            core.addiw(r.0, r.1, 1),
            core.slliw(r.0, r.1, 1),
            core.srliw(r.0, r.1, 1),
            core.sraiw(r.0, r.1, 1),
        ] {
            assert!(matches!(result, Err(crate::error::XError::InvalidInst)));
        }
    }
}
