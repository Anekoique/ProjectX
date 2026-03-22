use super::RVCore;
use crate::{
    config::SWord,
    cpu::riscv::csr::{CsrAddr, Exception, MStatus, PrivilegeMode, TrapCause},
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
        self.raise_trap(TrapCause::Exception(exc), 0);
        Ok(())
    }

    pub(super) fn ebreak(&mut self, _rd: RVReg, _rs1: RVReg, _imm: SWord) -> XResult {
        self.raise_trap(
            TrapCause::Exception(Exception::Breakpoint),
            self.pc.as_usize() as crate::config::Word,
        );
        Ok(())
    }

    pub(super) fn mret(&mut self, _rd: RVReg, _rs1: RVReg, _rs2: RVReg) -> XResult {
        if self.privilege < PrivilegeMode::Machine {
            self.raise_trap(TrapCause::Exception(Exception::IllegalInstruction), 0);
            return Ok(());
        }
        self.do_mret();
        Ok(())
    }

    pub(super) fn sret(&mut self, _rd: RVReg, _rs1: RVReg, _rs2: RVReg) -> XResult {
        if self.privilege < PrivilegeMode::Supervisor {
            self.raise_trap(TrapCause::Exception(Exception::IllegalInstruction), 0);
            return Ok(());
        }
        let ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
        if ms.contains(MStatus::TSR) && self.privilege == PrivilegeMode::Supervisor {
            self.raise_trap(TrapCause::Exception(Exception::IllegalInstruction), 0);
            return Ok(());
        }
        self.do_sret();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use memory_addr::VirtAddr;

    use super::*;

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
            core.ecall(RVReg::zero, RVReg::zero, 0).unwrap();
            assert_eq!(
                core.pending_trap.unwrap().cause,
                TrapCause::Exception(expected)
            );
        }
    }

    #[test]
    fn ebreak_sets_breakpoint_with_pc_as_tval() {
        let mut core = setup_core();
        core.pc = VirtAddr::from(0x1234_usize);
        core.ebreak(RVReg::zero, RVReg::zero, 0).unwrap();
        let trap = core.pending_trap.unwrap();
        assert_eq!(trap.cause, TrapCause::Exception(Exception::Breakpoint));
        assert_eq!(trap.tval, 0x1234);
    }

    #[test]
    fn mret_from_lower_privilege_traps() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Supervisor;
        core.mret(RVReg::zero, RVReg::zero, RVReg::zero).unwrap();
        assert_eq!(
            core.pending_trap.unwrap().cause,
            TrapCause::Exception(Exception::IllegalInstruction),
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
        core.sret(RVReg::zero, RVReg::zero, RVReg::zero).unwrap();
        assert_eq!(
            core.pending_trap.unwrap().cause,
            TrapCause::Exception(Exception::IllegalInstruction),
        );
    }

    #[test]
    fn sret_with_tsr_traps_in_s_mode() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Supervisor;
        core.csr.set(CsrAddr::mstatus, MStatus::TSR.bits());
        core.sret(RVReg::zero, RVReg::zero, RVReg::zero).unwrap();
        assert_eq!(
            core.pending_trap.unwrap().cause,
            TrapCause::Exception(Exception::IllegalInstruction),
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
