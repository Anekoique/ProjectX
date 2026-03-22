use super::RVCore;
use crate::{config::SWord, error::XResult, isa::RVReg};

impl RVCore {
    /// CSRRW: Atomic read/write. rd = CSR; CSR = rs1.
    /// If rd == x0, CSR is not read (no read side effects).
    pub(super) fn csrrw(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        let addr = (imm as u16) & 0xFFF;
        let src = self.gpr[rs1];

        if rd != RVReg::zero {
            let Some(old) = self.csr_read(addr) else {
                return Ok(());
            };
            if !self.csr_write(addr, src) {
                return Ok(());
            }
            self.set_gpr(rd, old)?;
        } else if !self.csr_write(addr, src) {
            return Ok(());
        }
        Ok(())
    }

    /// CSRRS: Atomic read and set bits. rd = CSR; CSR |= rs1.
    /// If rs1 == x0, CSR is not written (no write side effects).
    pub(super) fn csrrs(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        let addr = (imm as u16) & 0xFFF;

        let Some(old) = self.csr_read(addr) else {
            return Ok(());
        };
        if rs1 != RVReg::zero && !self.csr_write(addr, old | self.gpr[rs1]) {
            return Ok(());
        }
        self.set_gpr(rd, old)?;
        Ok(())
    }

    /// CSRRC: Atomic read and clear bits. rd = CSR; CSR &= ~rs1.
    /// If rs1 == x0, CSR is not written.
    pub(super) fn csrrc(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        let addr = (imm as u16) & 0xFFF;

        let Some(old) = self.csr_read(addr) else {
            return Ok(());
        };
        if rs1 != RVReg::zero && !self.csr_write(addr, old & !self.gpr[rs1]) {
            return Ok(());
        }
        self.set_gpr(rd, old)?;
        Ok(())
    }

    /// CSRRWI: Like CSRRW but with 5-bit zero-extended immediate.
    pub(super) fn csrrwi(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        let addr = (imm as u16) & 0xFFF;
        let uimm = u8::from(rs1) as crate::config::Word;

        if rd != RVReg::zero {
            let Some(old) = self.csr_read(addr) else {
                return Ok(());
            };
            if !self.csr_write(addr, uimm) {
                return Ok(());
            }
            self.set_gpr(rd, old)?;
        } else if !self.csr_write(addr, uimm) {
            return Ok(());
        }
        Ok(())
    }

    /// CSRRSI: Like CSRRS but with 5-bit zero-extended immediate.
    /// If uimm == 0, CSR is not written.
    pub(super) fn csrrsi(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        let addr = (imm as u16) & 0xFFF;
        let uimm = u8::from(rs1) as crate::config::Word;

        let Some(old) = self.csr_read(addr) else {
            return Ok(());
        };
        if uimm != 0 && !self.csr_write(addr, old | uimm) {
            return Ok(());
        }
        self.set_gpr(rd, old)?;
        Ok(())
    }

    /// CSRRCI: Like CSRRC but with 5-bit zero-extended immediate.
    /// If uimm == 0, CSR is not written.
    pub(super) fn csrrci(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        let addr = (imm as u16) & 0xFFF;
        let uimm = u8::from(rs1) as crate::config::Word;

        let Some(old) = self.csr_read(addr) else {
            return Ok(());
        };
        if uimm != 0 && !self.csr_write(addr, old & !uimm) {
            return Ok(());
        }
        self.set_gpr(rd, old)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::riscv::csr::{CsrAddr, Exception, TrapCause};

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

        core.csrrs(RVReg::t0, RVReg::zero, bad_addr).unwrap();

        let trap = core.pending_trap.unwrap();
        assert_eq!(
            trap.cause,
            TrapCause::Exception(Exception::IllegalInstruction)
        );
    }

    #[test]
    fn read_only_csr_write_raises_trap() {
        let mut core = setup_core();
        // mvendorid (0xF11) has addr bits [11:10] == 0b11 → truly read-only by encoding
        let mvendorid_addr = CsrAddr::mvendorid as SWord;
        core.gpr[RVReg::t0] = 0xFF;

        core.csrrw(RVReg::t1, RVReg::t0, mvendorid_addr).unwrap();

        let trap = core.pending_trap.unwrap();
        assert_eq!(
            trap.cause,
            TrapCause::Exception(Exception::IllegalInstruction)
        );
    }

    #[test]
    fn warl_readonly_csr_write_is_silent() {
        let mut core = setup_core();
        // misa (0x301) has wmask=0 but is NOT read-only by address encoding.
        // Write succeeds but has no effect (WARL).
        let misa_addr = CsrAddr::misa as SWord;
        core.gpr[RVReg::t0] = 0xFF;

        core.csrrw(RVReg::t1, RVReg::t0, misa_addr).unwrap();

        assert!(core.pending_trap.is_none());
        assert_eq!(core.csr.get(CsrAddr::misa), 0); // unchanged
    }

    #[test]
    fn privilege_violation_raises_trap() {
        let mut core = setup_core();
        core.privilege = crate::cpu::riscv::csr::PrivilegeMode::User;
        // mscratch is M-mode (0x340) — U-mode can't access
        core.csrrs(RVReg::t0, RVReg::zero, mscratch_addr()).unwrap();

        let trap = core.pending_trap.unwrap();
        assert_eq!(
            trap.cause,
            TrapCause::Exception(Exception::IllegalInstruction)
        );
    }
}
