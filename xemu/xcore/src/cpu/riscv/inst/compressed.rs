// cfg(isa32) blocks use `return` before cfg(isa64) alternatives
#![allow(clippy::needless_return)]

use memory_addr::{MemoryAddr, VirtAddr};

use super::RVCore;
use crate::{
    config::{SWord, Word},
    cpu::riscv::trap::{Exception, TrapCause},
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
        let imm = (bits(inst, 12, 10) << 3) | (bits(inst, 6, 6) << 2) | (bits(inst, 5, 5) << 6);
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
        let imm = (bits(inst, 12, 10) << 3) | (bits(inst, 6, 6) << 2) | (bits(inst, 5, 5) << 6);
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
        if rd == RVReg::zero {
            return Err(XError::InvalidInst);
        }
        let imm = (bits(inst, 12, 12) << 5) | (bits(inst, 6, 4) << 2) | (bits(inst, 3, 2) << 6);
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
            if rd == RVReg::zero {
                return Err(XError::InvalidInst);
            }
            let imm = (bits(inst, 12, 12) << 5) | (bits(inst, 6, 5) << 3) | (bits(inst, 4, 2) << 6);
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
        self.raise_trap(
            TrapCause::Exception(Exception::Breakpoint),
            self.pc.as_usize() as Word,
        );
        Ok(())
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

#[cfg(test)]
mod tests {
    use memory_addr::VirtAddr;

    use super::*;
    use crate::{
        config::{CONFIG_MBASE, Word},
        cpu::MemOps,
        memory::with_mem,
    };

    const TEST_BASE: usize = CONFIG_MBASE + 0x2000;

    fn setup_core() -> RVCore {
        let mut core = RVCore::new();
        core.pc = VirtAddr::from(TEST_BASE);
        core.npc = core.pc.wrapping_add(2);
        core
    }

    #[test]
    fn c_li_loads_and_sign_extends() {
        let mut core = setup_core();
        // c.li x10, 5
        core.c_li(0b010_0_01010_00101_01).unwrap();
        assert_eq!(core.gpr[RVReg::a0], 5);

        // c.li x10, -1 (sign-extended)
        core.c_li(0b010_1_01010_11111_01).unwrap();
        assert_eq!(core.gpr[RVReg::a0] as SWord, -1);
    }

    #[test]
    fn c_addi_positive_and_negative() {
        let mut core = setup_core();
        core.gpr[RVReg::a0] = 10;

        // c.addi x10, 3
        core.c_addi(0b000_0_01010_00011_01).unwrap();
        assert_eq!(core.gpr[RVReg::a0], 13);

        // c.addi x10, -3
        core.c_addi(0b000_1_01010_11101_01).unwrap();
        assert_eq!(core.gpr[RVReg::a0], 10);
    }

    #[test]
    fn c_mv_moves_register() {
        let mut core = setup_core();
        core.gpr[RVReg::t0] = 42;
        // c.mv x10, x5: funct4=1000, rd=01010, rs2=00101, op=10
        let inst: u32 = 0b100_0_01010_00101_10;
        core.c_mv(inst).unwrap();
        assert_eq!(core.gpr[RVReg::a0], 42);
    }

    #[test]
    fn c_mv_rejects_rs2_zero() {
        let mut core = setup_core();
        // c.mv x10, x0: rs2=00000 → InvalidInst
        let inst: u32 = 0b100_0_01010_00000_10;
        assert!(matches!(core.c_mv(inst), Err(XError::InvalidInst)));
    }

    #[test]
    fn c_add_adds_registers() {
        let mut core = setup_core();
        core.gpr[RVReg::a0] = 10;
        core.gpr[RVReg::t0] = 32;
        // c.add x10, x5: funct4=1001, rd=01010, rs2=00101, op=10
        let inst: u32 = 0b100_1_01010_00101_10;
        core.c_add(inst).unwrap();
        assert_eq!(core.gpr[RVReg::a0], 42);
    }

    #[test]
    fn c_sub_subtracts_prime_registers() {
        let mut core = setup_core();
        // c.sub operates on prime regs (x8-x15)
        core.gpr[RVReg::s0] = 10; // x8
        core.gpr[RVReg::s1] = 3; // x9
        // c.sub x8, x9: 100_0_11_000_00_001_01
        let inst: u32 = 0b100_0_11_000_00_001_01;
        core.c_sub(inst).unwrap();
        assert_eq!(core.gpr[RVReg::s0], 7);
    }

    #[test]
    fn c_and_or_xor_work() {
        let mut core = setup_core();
        core.gpr[RVReg::s0] = 0xFF;
        core.gpr[RVReg::s1] = 0x0F;

        // c.and x8, x9: 100_0_11_000_11_001_01
        let c_and: u32 = 0b100_0_11_000_11_001_01;
        core.c_and(c_and).unwrap();
        assert_eq!(core.gpr[RVReg::s0], 0x0F);

        core.gpr[RVReg::s0] = 0xF0;
        // c.or x8, x9: 100_0_11_000_10_001_01
        let c_or: u32 = 0b100_0_11_000_10_001_01;
        core.c_or(c_or).unwrap();
        assert_eq!(core.gpr[RVReg::s0], 0xFF);

        core.gpr[RVReg::s0] = 0xFF;
        // c.xor x8, x9: 100_0_11_000_01_001_01
        let c_xor: u32 = 0b100_0_11_000_01_001_01;
        core.c_xor(c_xor).unwrap();
        assert_eq!(core.gpr[RVReg::s0], 0xF0);
    }

    #[test]
    fn c_j_updates_npc() {
        let mut core = setup_core();
        // c.j with offset +0 (all imm bits zero): 101_00000000000_01
        let inst: u32 = 0b101_0_00000_00000_01;
        core.c_j(inst).unwrap();
        assert_eq!(core.npc, core.pc); // jump to self
    }

    #[test]
    fn c_beqz_and_bnez_branch_correctly() {
        let mut core = setup_core();
        let fallthrough = core.npc;
        let beqz: u32 = 0b110_0_00_000_000_00_01; // c.beqz x8, +0
        let bnez: u32 = 0b111_0_00_000_000_00_01; // c.bnez x8, +0

        // beqz: taken when rs1==0
        core.gpr[RVReg::s0] = 0;
        core.c_beqz(beqz).unwrap();
        assert_eq!(core.npc, core.pc);

        // beqz: not taken when rs1!=0
        core.gpr[RVReg::s0] = 1;
        core.npc = fallthrough;
        core.c_beqz(beqz).unwrap();
        assert_eq!(core.npc, fallthrough);

        // bnez: taken when rs1!=0
        core.gpr[RVReg::s0] = 5;
        core.c_bnez(bnez).unwrap();
        assert_eq!(core.npc, core.pc);

        // bnez: not taken when rs1==0
        core.gpr[RVReg::s0] = 0;
        core.npc = fallthrough;
        core.c_bnez(bnez).unwrap();
        assert_eq!(core.npc, fallthrough);
    }

    #[test]
    fn c_jr_jumps_to_register() {
        let mut core = setup_core();
        core.gpr[RVReg::ra] = (TEST_BASE + 0x101) as Word;
        // c.jr x1: 100_0_00001_00000_10
        let inst: u32 = 0b100_0_00001_00000_10;
        core.c_jr(inst).unwrap();
        assert_eq!(core.npc, VirtAddr::from(TEST_BASE + 0x100));
    }

    #[test]
    fn c_jr_rejects_zero_register() {
        let mut core = setup_core();
        // c.jr x0: 100_0_00000_00000_10
        let inst: u32 = 0b100_0_00000_00000_10;
        assert!(matches!(core.c_jr(inst), Err(XError::InvalidInst)));
    }

    #[test]
    fn c_jalr_saves_link_and_jumps() {
        let mut core = setup_core();
        core.gpr[RVReg::t0] = (TEST_BASE + 0x201) as Word;
        // c.jalr x5: 100_1_00101_00000_10
        let inst: u32 = 0b100_1_00101_00000_10;
        core.c_jalr(inst).unwrap();
        assert_eq!(
            core.gpr[RVReg::ra],
            core.pc.wrapping_add(2).as_usize() as Word
        );
        assert_eq!(core.npc, VirtAddr::from(TEST_BASE + 0x200));
    }

    #[test]
    fn c_ebreak_sets_breakpoint_trap() {
        let mut core = setup_core();
        assert!(core.c_ebreak(0).is_ok());
        let trap = core.pending_trap.unwrap();
        assert_eq!(trap.cause, TrapCause::Exception(Exception::Breakpoint));
    }

    #[test]
    fn c_lwsp_loads_from_sp_offset() {
        let mut core = setup_core();
        let sp_val = (TEST_BASE + 0x400) as Word;
        core.gpr[RVReg::sp] = sp_val;

        // Write a known value at sp+0
        let addr = core.virt_to_phys(VirtAddr::from_usize(sp_val as usize));
        with_mem!(write(addr, 4, 0x12345678)).unwrap();

        // c.lwsp x10, 0: 010_0_01010_00000_10 (offset=0)
        let inst: u32 = 0b010_0_01010_000_00_10;
        core.c_lwsp(inst).unwrap();
        assert_eq!(core.gpr[RVReg::a0] as u32, 0x12345678);
    }

    #[test]
    fn c_swsp_stores_to_sp_offset() {
        let mut core = setup_core();
        let sp_val = (TEST_BASE + 0x500) as Word;
        core.gpr[RVReg::sp] = sp_val;
        core.gpr[RVReg::t0] = 0xABCD1234;

        // c.swsp x5, 0: 110_000000_00101_10 (offset=0)
        let inst: u32 = 0b110_0_0000_0_00101_10;
        core.c_swsp(inst).unwrap();

        let addr = core.virt_to_phys(VirtAddr::from_usize(sp_val as usize));
        let stored = with_mem!(read(addr, 4)).unwrap();
        assert_eq!(stored as u32, 0xABCD1234);
    }

    #[test]
    fn c_addi4spn_adds_scaled_imm_to_sp() {
        let mut core = setup_core();
        core.gpr[RVReg::sp] = 0x1000;
        // c.addi4spn rd', imm
        // imm = (bits[10:7] << 6) | (bits[12:11] << 4) | (bits[6] << 2) | (bits[5] <<
        // 3) Set bits[6]=1 → imm = (1 << 2) = 4
        // inst: funct3[15:13]=000, imm_bits[12:5]=00000010, rd'[4:2]=000, op[1:0]=00
        let inst: u32 = 0b000_00000_010_000_00;
        core.c_addi4spn(inst).unwrap();
        assert_eq!(core.gpr[RVReg::s0], 0x1004);
    }

    #[test]
    fn c_addi4spn_rejects_zero_immediate() {
        let mut core = setup_core();
        // All imm bits zero → InvalidInst
        let inst: u32 = 0b000_00000_00_000_00;
        assert!(matches!(core.c_addi4spn(inst), Err(XError::InvalidInst)));
    }

    #[test]
    fn c_slli_shifts_left() {
        let mut core = setup_core();
        core.gpr[RVReg::a0] = 1;
        // c.slli x10, 4: 000_0_01010_00100_10
        let inst: u32 = 0b000_0_01010_00100_10;
        core.c_slli(inst).unwrap();
        assert_eq!(core.gpr[RVReg::a0], 16);
    }

    #[test]
    fn c_srli_shifts_right_logical() {
        let mut core = setup_core();
        core.gpr[RVReg::s0] = 0x80;
        // c.srli x8, 4: 100_0_00_000_00100_01
        let inst: u32 = 0b100_0_00_000_00100_01;
        core.c_srli(inst).unwrap();
        assert_eq!(core.gpr[RVReg::s0], 0x8);
    }

    #[test]
    fn c_srai_shifts_right_arithmetic() {
        let mut core = setup_core();
        core.gpr[RVReg::s0] = (-16 as SWord) as Word;
        // c.srai x8, 2: 100_0_01_000_00010_01
        let inst: u32 = 0b100_0_01_000_00010_01;
        core.c_srai(inst).unwrap();
        assert_eq!(core.gpr[RVReg::s0] as SWord, -4);
    }

    #[test]
    fn c_andi_masks_register() {
        let mut core = setup_core();
        core.gpr[RVReg::s0] = 0xFF;
        // c.andi x8, 0xF: 100_0_10_000_01111_01
        let inst: u32 = 0b100_0_10_000_01111_01;
        core.c_andi(inst).unwrap();
        assert_eq!(core.gpr[RVReg::s0], 0x0F);
    }

    #[test]
    fn c_lwsp_rejects_rd_zero() {
        let mut core = setup_core();
        // c.lwsp x0, 0: 010_0_00000_00000_10
        let inst: u32 = 0b010_0_00000_000_00_10;
        assert!(matches!(core.c_lwsp(inst), Err(XError::InvalidInst)));
    }

    #[test]
    #[cfg(isa64)]
    fn c_ldsp_rejects_rd_zero() {
        let mut core = setup_core();
        // c.ldsp x0, 0: 011_0_00000_00000_10
        let inst: u32 = 0b011_0_00000_000_00_10;
        assert!(matches!(core.c_ldsp(inst), Err(XError::InvalidInst)));
    }

    #[test]
    #[cfg(isa32)]
    fn rv64_only_compressed_instructions_are_rejected_on_rv32() {
        let mut core = setup_core();

        for op in [
            RVCore::c_ld,
            RVCore::c_sd,
            RVCore::c_addiw,
            RVCore::c_subw,
            RVCore::c_addw,
            RVCore::c_ldsp,
            RVCore::c_sdsp,
        ] {
            assert!(matches!(op(&mut core, 0), Err(XError::InvalidInst)));
        }
    }
}
