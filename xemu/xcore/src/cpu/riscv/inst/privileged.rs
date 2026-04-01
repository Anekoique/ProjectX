//! Privileged instructions: `ecall`, `ebreak`, `mret`, `sret`, `wfi`,
//! `sfence.vma`, and memory fences.

use super::RVCore;
use crate::{
    config::{SWord, Word},
    cpu::riscv::csr::{CsrAddr, Exception, MStatus, PrivilegeMode},
    error::XResult,
    isa::RVReg,
};

impl RVCore {
    pub(super) fn ecall(&mut self, _rd: RVReg, _rs1: RVReg, _imm: SWord) -> XResult {
        let exc = match self.privilege {
            PrivilegeMode::User => Exception::EcallFromU,
            PrivilegeMode::Supervisor => Exception::EcallFromS,
            PrivilegeMode::Machine => Exception::EcallFromM,
        };
        Err(self.trap_exception(exc, 0))
    }

    pub(super) fn ebreak(&mut self, _rd: RVReg, _rs1: RVReg, _imm: SWord) -> XResult {
        Err(self.trap_exception(Exception::Breakpoint, self.pc.as_usize() as Word))
    }

    pub(super) fn mret(&mut self, _rd: RVReg, _rs1: RVReg, _rs2: RVReg) -> XResult {
        if self.privilege < PrivilegeMode::Machine {
            return Err(self.illegal_inst());
        }
        self.do_mret();
        Ok(())
    }

    pub(super) fn sret(&mut self, _rd: RVReg, _rs1: RVReg, _rs2: RVReg) -> XResult {
        if self.privilege < PrivilegeMode::Supervisor {
            return Err(self.illegal_inst());
        }
        let ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
        if ms.contains(MStatus::TSR) && self.privilege == PrivilegeMode::Supervisor {
            return Err(self.illegal_inst());
        }
        self.do_sret();
        Ok(())
    }

    /// Memory ordering fence — NOP on single-hart emulator.
    pub(super) fn fence(&mut self, _rd: RVReg, _rs1: RVReg, _imm: SWord) -> XResult {
        Ok(())
    }

    /// Instruction fence — NOP on single-hart emulator (no icache).
    pub(super) fn fence_i(&mut self, _rd: RVReg, _rs1: RVReg, _imm: SWord) -> XResult {
        Ok(())
    }

    /// Wait for interrupt — NOP (interrupt check happens in step loop).
    /// Traps in U-mode unconditionally, and in S-mode when mstatus.TW=1.
    pub(super) fn wfi(&mut self, _rd: RVReg, _rs1: RVReg, _imm: SWord) -> XResult {
        match self.privilege {
            PrivilegeMode::User => Err(self.illegal_inst()),
            PrivilegeMode::Supervisor => {
                let ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
                if ms.contains(MStatus::TW) {
                    return Err(self.illegal_inst());
                }
                Ok(())
            }
            PrivilegeMode::Machine => Ok(()),
        }
    }

    pub(super) fn sfence_vma(&mut self, _rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        if self.privilege == PrivilegeMode::User {
            return Err(self.illegal_inst());
        }
        if self.privilege == PrivilegeMode::Supervisor {
            let ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
            if ms.contains(MStatus::TVM) {
                return Err(self.illegal_inst());
            }
        }
        let vpn = if rs1 != RVReg::zero {
            Some((self.gpr[rs1] as usize) >> 12)
        } else {
            None
        };
        let asid = if rs2 != RVReg::zero {
            Some(self.gpr[rs2] as u16)
        } else {
            None
        };
        self.mmu.tlb.flush(vpn, asid);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use memory_addr::VirtAddr;

    use super::*;
    use crate::cpu::riscv::trap::{TrapCause, test_helpers::assert_trap};

    fn setup_core() -> RVCore {
        let mut core = RVCore::new();
        core.csr.set(CsrAddr::mtvec, 0x8000_0000);
        core.csr.set(CsrAddr::stvec, 0x4000_0000);
        core
    }

    #[test]
    fn ecall_from_each_privilege() {
        for (mode, expected) in [
            (PrivilegeMode::User, Exception::EcallFromU),
            (PrivilegeMode::Supervisor, Exception::EcallFromS),
            (PrivilegeMode::Machine, Exception::EcallFromM),
        ] {
            let mut core = setup_core();
            core.privilege = mode;
            assert_trap(
                core.ecall(RVReg::zero, RVReg::zero, 0),
                TrapCause::Exception(expected),
                0,
            );
        }
    }

    #[test]
    fn ebreak_sets_breakpoint_with_pc_as_tval() {
        let mut core = setup_core();
        core.pc = VirtAddr::from(0x1234_usize);
        assert_trap(
            core.ebreak(RVReg::zero, RVReg::zero, 0),
            TrapCause::Exception(Exception::Breakpoint),
            0x1234,
        );
    }

    #[test]
    fn mret_from_lower_privilege_traps() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Supervisor;
        assert_trap(
            core.mret(RVReg::zero, RVReg::zero, RVReg::zero),
            TrapCause::Exception(Exception::IllegalInstruction),
            0,
        );
    }

    #[test]
    fn mret_from_m_mode_succeeds() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Machine;
        let ms = MStatus::empty().with_mpp(PrivilegeMode::User);
        core.csr.set(CsrAddr::mstatus, ms.bits());
        core.csr.set(CsrAddr::mepc, 0x2000);

        core.mret(RVReg::zero, RVReg::zero, RVReg::zero).unwrap();

        assert!(core.pending_trap.is_none());
        assert_eq!(core.privilege, PrivilegeMode::User);
    }

    #[test]
    fn sret_from_u_mode_traps() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::User;
        assert_trap(
            core.sret(RVReg::zero, RVReg::zero, RVReg::zero),
            TrapCause::Exception(Exception::IllegalInstruction),
            0,
        );
    }

    #[test]
    fn sret_with_tsr_traps_in_s_mode() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Supervisor;
        core.csr.set(CsrAddr::mstatus, MStatus::TSR.bits());
        assert_trap(
            core.sret(RVReg::zero, RVReg::zero, RVReg::zero),
            TrapCause::Exception(Exception::IllegalInstruction),
            0,
        );
    }

    #[test]
    fn sret_without_tsr_succeeds() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Supervisor;
        let ms = MStatus::empty().with_spp(PrivilegeMode::User);
        core.csr.set(CsrAddr::mstatus, ms.bits());
        core.csr.set(CsrAddr::sepc, 0x3000);

        core.sret(RVReg::zero, RVReg::zero, RVReg::zero).unwrap();

        assert!(core.pending_trap.is_none());
        assert_eq!(core.privilege, PrivilegeMode::User);
    }

    #[test]
    fn sret_with_tsr_from_m_mode_succeeds() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Machine;
        let ms = MStatus::TSR | MStatus::empty().with_spp(PrivilegeMode::User);
        core.csr.set(CsrAddr::mstatus, ms.bits());
        core.csr.set(CsrAddr::sepc, 0x3000);

        core.sret(RVReg::zero, RVReg::zero, RVReg::zero).unwrap();

        // TSR only blocks S-mode, not M-mode
        assert!(core.pending_trap.is_none());
    }
}
