use super::{AccessRule, CsrAddr, CsrDesc, MStatus, PrivilegeMode, counteren_bit, find_desc};
use crate::{
    config::Word,
    cpu::riscv::{
        RVCore,
        trap::{Exception, TrapCause},
    },
};

impl RVCore {
    /// CSR read with existence check, privilege check, and dynamic access
    /// rules.
    pub fn csr_read(&mut self, addr: u16) -> Option<Word> {
        let Some(desc) = find_desc(addr) else {
            self.raise_trap(TrapCause::Exception(Exception::IllegalInstruction), 0);
            return None;
        };
        if !self.check_csr_access(addr, &desc) {
            return None;
        }
        Some(self.csr.read_with_desc(desc))
    }

    /// CSR write with existence check, read-only check, privilege check, and
    /// side effects.
    pub fn csr_write(&mut self, addr: u16, val: Word) -> bool {
        let Some(desc) = find_desc(addr) else {
            self.raise_trap(TrapCause::Exception(Exception::IllegalInstruction), 0);
            return false;
        };
        if Self::is_read_only(addr) {
            self.raise_trap(TrapCause::Exception(Exception::IllegalInstruction), 0);
            return false;
        }
        if !self.check_csr_access(addr, &desc) {
            return false;
        }
        self.csr.write_with_desc(desc, val);
        // TODO: flush TLB on satp write
        true
    }

    fn is_read_only(addr: u16) -> bool {
        (addr >> 10) & 0x3 == 0x3
    }

    fn check_csr_access(&mut self, addr: u16, desc: &CsrDesc) -> bool {
        let required = PrivilegeMode::from_bits(((addr >> 8) & 0x3) as Word);
        if self.privilege < required {
            self.raise_trap(TrapCause::Exception(Exception::IllegalInstruction), 0);
            return false;
        }
        match desc.access {
            AccessRule::Standard => true,
            AccessRule::BlockedByMstatus(flag) => {
                let ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
                if ms.contains(flag) && self.privilege == PrivilegeMode::Supervisor {
                    self.raise_trap(TrapCause::Exception(Exception::IllegalInstruction), 0);
                    false
                } else {
                    true
                }
            }
            AccessRule::CounterGated => {
                let bit = counteren_bit(addr);
                match self.privilege {
                    PrivilegeMode::Machine => true,
                    PrivilegeMode::Supervisor => {
                        if (self.csr.get(CsrAddr::mcounteren) >> bit) & 1 == 0 {
                            self.raise_trap(TrapCause::Exception(Exception::IllegalInstruction), 0);
                            false
                        } else {
                            true
                        }
                    }
                    PrivilegeMode::User => {
                        let m_ok = (self.csr.get(CsrAddr::mcounteren) >> bit) & 1 == 1;
                        let s_ok = (self.csr.get(CsrAddr::scounteren) >> bit) & 1 == 1;
                        if !(m_ok && s_ok) {
                            self.raise_trap(TrapCause::Exception(Exception::IllegalInstruction), 0);
                            false
                        } else {
                            true
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn csr_read_unknown_traps() {
        let mut core = RVCore::new();
        assert!(core.csr_read(0xFFF).is_none());
        assert_eq!(
            core.pending_trap.unwrap().cause,
            TrapCause::Exception(Exception::IllegalInstruction),
        );
    }

    #[test]
    fn csr_write_unknown_traps() {
        let mut core = RVCore::new();
        assert!(!core.csr_write(0xFFF, 0));
        assert_eq!(
            core.pending_trap.unwrap().cause,
            TrapCause::Exception(Exception::IllegalInstruction),
        );
    }

    #[test]
    fn csr_write_read_only_by_encoding_traps() {
        let mut core = RVCore::new();
        assert!(!core.csr_write(CsrAddr::mvendorid as u16, 42));
        assert_eq!(
            core.pending_trap.unwrap().cause,
            TrapCause::Exception(Exception::IllegalInstruction),
        );
    }

    #[test]
    fn csr_read_privilege_violation() {
        let mut core = RVCore::new();
        core.privilege = PrivilegeMode::User;
        assert!(core.csr_read(CsrAddr::mstatus as u16).is_none());
        assert!(core.pending_trap.is_some());
    }

    #[test]
    fn counter_gated_m_mode_allowed() {
        let mut core = RVCore::new();
        assert!(core.csr_read(CsrAddr::cycle as u16).is_some());
    }

    #[test]
    fn counter_gated_s_mode_blocked_by_mcounteren() {
        let mut core = RVCore::new();
        core.privilege = PrivilegeMode::Supervisor;
        core.csr.set(CsrAddr::mcounteren, 0);
        assert!(core.csr_read(CsrAddr::cycle as u16).is_none());
        assert!(core.pending_trap.is_some());
    }

    #[test]
    fn counter_gated_s_mode_allowed_by_mcounteren() {
        let mut core = RVCore::new();
        core.privilege = PrivilegeMode::Supervisor;
        core.csr.set(CsrAddr::mcounteren, 0x7);
        assert!(core.csr_read(CsrAddr::cycle as u16).is_some());
    }

    #[test]
    fn counter_gated_u_mode_needs_both_counteren() {
        let mut core = RVCore::new();
        core.privilege = PrivilegeMode::User;

        core.csr.set(CsrAddr::mcounteren, 0x7);
        core.csr.set(CsrAddr::scounteren, 0);
        assert!(core.csr_read(CsrAddr::cycle as u16).is_none());

        core.pending_trap = None;
        core.csr.set(CsrAddr::scounteren, 0x7);
        assert!(core.csr_read(CsrAddr::cycle as u16).is_some());
    }

    #[test]
    fn tvm_blocks_satp_in_s_mode() {
        let mut core = RVCore::new();
        core.privilege = PrivilegeMode::Supervisor;
        core.csr.set(CsrAddr::mstatus, MStatus::TVM.bits());
        assert!(core.csr_read(CsrAddr::satp as u16).is_none());
        assert!(core.pending_trap.is_some());
    }

    #[test]
    fn tvm_does_not_block_satp_in_m_mode() {
        let mut core = RVCore::new();
        core.csr.set(CsrAddr::mstatus, MStatus::TVM.bits());
        assert!(core.csr_read(CsrAddr::satp as u16).is_some());
    }
}
