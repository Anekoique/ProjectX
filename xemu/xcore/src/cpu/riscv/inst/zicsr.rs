//! Zicsr extension: CSR read/write/set/clear instructions.

use super::RVCore;
use crate::{
    config::{SWord, Word},
    error::XResult,
    isa::RVReg,
};

#[inline(always)]
fn csr_addr(imm: SWord) -> u16 {
    (imm as u16) & 0xFFF
}

#[inline(always)]
fn csr_uimm(rs1: RVReg) -> Word {
    u8::from(rs1) as Word
}

impl RVCore {
    /// CSR read-write core: if `skip_read`, CSR is not read (no side effects).
    /// `merge` produces the new CSR value from (old, src).
    fn csr_op(
        &mut self,
        rd: RVReg,
        addr: u16,
        src: Word,
        skip_read: bool,
        skip_write: bool,
        merge: fn(Word, Word) -> Word,
    ) -> XResult {
        if skip_read {
            self.csr_write(addr, src)?;
        } else {
            let old = self.csr_read(addr)?;
            if !skip_write {
                self.csr_write(addr, merge(old, src))?;
            }
            self.set_gpr(rd, old)?;
        }
        Ok(())
    }

    pub(super) fn csrrw(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.csr_op(
            rd,
            csr_addr(imm),
            self.gpr[rs1],
            rd == RVReg::zero,
            false,
            |_, src| src,
        )
    }

    pub(super) fn csrrs(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.csr_op(
            rd,
            csr_addr(imm),
            self.gpr[rs1],
            false,
            rs1 == RVReg::zero,
            |old, src| old | src,
        )
    }

    pub(super) fn csrrc(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.csr_op(
            rd,
            csr_addr(imm),
            self.gpr[rs1],
            false,
            rs1 == RVReg::zero,
            |old, src| old & !src,
        )
    }

    pub(super) fn csrrwi(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.csr_op(
            rd,
            csr_addr(imm),
            csr_uimm(rs1),
            rd == RVReg::zero,
            false,
            |_, src| src,
        )
    }

    pub(super) fn csrrsi(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        let uimm = csr_uimm(rs1);
        self.csr_op(rd, csr_addr(imm), uimm, false, uimm == 0, |old, src| {
            old | src
        })
    }

    pub(super) fn csrrci(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        let uimm = csr_uimm(rs1);
        self.csr_op(rd, csr_addr(imm), uimm, false, uimm == 0, |old, src| {
            old & !src
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::riscv::{csr::CsrAddr, trap::test_helpers::assert_illegal_inst};

    fn setup_core() -> RVCore {
        RVCore::new()
    }

    fn mscratch_addr() -> SWord {
        CsrAddr::mscratch as SWord
    }

    #[test]
    fn csrrw_reads_and_writes() {
        let mut core = setup_core();
        core.csr.set(CsrAddr::mscratch, 0xAA);
        core.gpr[RVReg::t0] = 0xBB;

        core.csrrw(RVReg::t1, RVReg::t0, mscratch_addr()).unwrap();

        assert_eq!(core.gpr[RVReg::t1], 0xAA); // old value
        assert_eq!(core.csr.get(CsrAddr::mscratch), 0xBB); // new value
    }

    #[test]
    fn csrrw_rd_zero_skips_read() {
        let mut core = setup_core();
        core.gpr[RVReg::t0] = 0xCC;

        core.csrrw(RVReg::zero, RVReg::t0, mscratch_addr()).unwrap();

        assert_eq!(core.csr.get(CsrAddr::mscratch), 0xCC);
    }

    #[test]
    fn csrrs_sets_bits() {
        let mut core = setup_core();
        core.csr.set(CsrAddr::mscratch, 0x0F);
        core.gpr[RVReg::t0] = 0xF0;

        core.csrrs(RVReg::t1, RVReg::t0, mscratch_addr()).unwrap();

        assert_eq!(core.gpr[RVReg::t1], 0x0F); // old
        assert_eq!(core.csr.get(CsrAddr::mscratch), 0xFF); // OR'd
    }

    #[test]
    fn csrrs_rs1_zero_no_write() {
        let mut core = setup_core();
        core.csr.set(CsrAddr::mscratch, 0x42);

        core.csrrs(RVReg::t1, RVReg::zero, mscratch_addr()).unwrap();

        assert_eq!(core.gpr[RVReg::t1], 0x42);
        assert_eq!(core.csr.get(CsrAddr::mscratch), 0x42); // unchanged
    }

    #[test]
    fn csrrc_clears_bits() {
        let mut core = setup_core();
        core.csr.set(CsrAddr::mscratch, 0xFF);
        core.gpr[RVReg::t0] = 0x0F;

        core.csrrc(RVReg::t1, RVReg::t0, mscratch_addr()).unwrap();

        assert_eq!(core.gpr[RVReg::t1], 0xFF);
        assert_eq!(core.csr.get(CsrAddr::mscratch), 0xF0);
    }

    #[test]
    fn csrrwi_uses_immediate() {
        let mut core = setup_core();
        core.csr.set(CsrAddr::mscratch, 0xAA);

        // rs1 encodes the uimm[4:0] — use t5 (reg 30 = 0x1E)
        core.csrrwi(RVReg::t1, RVReg::t5, mscratch_addr()).unwrap();

        assert_eq!(core.gpr[RVReg::t1], 0xAA);
        assert_eq!(core.csr.get(CsrAddr::mscratch), 30); // t5 = reg 30
    }

    #[test]
    fn csrrsi_zero_imm_no_write() {
        let mut core = setup_core();
        core.csr.set(CsrAddr::mscratch, 0x42);

        // zero register = uimm 0
        core.csrrsi(RVReg::t1, RVReg::zero, mscratch_addr())
            .unwrap();

        assert_eq!(core.gpr[RVReg::t1], 0x42);
        assert_eq!(core.csr.get(CsrAddr::mscratch), 0x42);
    }

    #[test]
    fn unknown_csr_raises_trap() {
        let mut core = setup_core();
        let bad_addr = 0xFFF as SWord;

        assert_illegal_inst(core.csrrs(RVReg::t0, RVReg::zero, bad_addr));
    }

    #[test]
    fn read_only_csr_write_raises_trap() {
        let mut core = setup_core();
        // mvendorid (0xF11) has addr bits [11:10] == 0b11 → truly read-only by encoding
        let mvendorid_addr = CsrAddr::mvendorid as SWord;
        core.gpr[RVReg::t0] = 0xFF;

        assert_illegal_inst(core.csrrw(RVReg::t1, RVReg::t0, mvendorid_addr));
    }

    #[test]
    fn warl_readonly_csr_write_is_silent() {
        let mut core = setup_core();
        // misa (0x301) has wmask=0 but is NOT read-only by address encoding.
        // Write succeeds but has no effect (WARL).
        let misa_addr = CsrAddr::misa as SWord;
        let before = core.csr.get(CsrAddr::misa);
        core.gpr[RVReg::t0] = 0xFF;

        core.csrrw(RVReg::t1, RVReg::t0, misa_addr).unwrap();

        assert!(core.pending_trap.is_none());
        assert_eq!(core.csr.get(CsrAddr::misa), before); // unchanged
    }

    #[test]
    fn privilege_violation_raises_trap() {
        let mut core = setup_core();
        core.privilege = crate::cpu::riscv::csr::PrivilegeMode::User;
        // mscratch is M-mode (0x340) — U-mode can't access
        assert_illegal_inst(core.csrrs(RVReg::t0, RVReg::zero, mscratch_addr()));
    }
}
