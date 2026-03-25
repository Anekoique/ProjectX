use super::{AccessRule, CsrAddr, CsrDesc, MStatus, PrivilegeMode, counteren_bit, find_desc};
use crate::{config::Word, cpu::riscv::RVCore, error::XResult};

impl RVCore {
    /// CSR read with existence check, privilege check, and dynamic access
    /// rules.
    pub(in crate::cpu::riscv) fn csr_read(&self, addr: u16) -> XResult<Word> {
        let Some(desc) = find_desc(addr) else {
            return self.illegal_inst();
        };
        self.check_csr_access(addr, &desc)?;
        Ok(self.csr.read_with_desc(desc))
    }

    /// CSR write with existence check, read-only check, privilege check, and
    /// side effects.
    pub(in crate::cpu::riscv) fn csr_write(&mut self, addr: u16, val: Word) -> XResult {
        let Some(desc) = find_desc(addr) else {
            return self.illegal_inst();
        };
        if Self::is_read_only(addr) {
            return self.illegal_inst();
        }
        self.check_csr_access(addr, &desc)?;
        self.csr.write_with_desc(desc, val);
        self.csr_write_side_effects(addr);
        Ok(())
    }

    fn csr_write_side_effects(&mut self, addr: u16) {
        match addr {
            0x180 /* satp */ => {
                self.mmu.update_satp(self.csr.get(CsrAddr::satp));
            }
            0x300 /* mstatus */ | 0x100 /* sstatus */ => {
                let ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
                self.mmu.update_mstatus(ms.contains(MStatus::SUM), ms.contains(MStatus::MXR));
            }
            0x3A0..=0x3A3 /* pmpcfg0..3 */ => {
                let val = self.csr.get_by_addr(addr);
                let base = (addr - 0x3A0) as usize * std::mem::size_of::<Word>();
                for i in 0..std::mem::size_of::<Word>() {
                    self.pmp.update_cfg(base + i, (val >> (i * 8)) as u8);
                }
            }
            0x3B0..=0x3BF /* pmpaddr0..15 */ => {
                let idx = (addr - 0x3B0) as usize;
                self.pmp.update_addr(idx, self.csr.get_by_addr(addr) as usize);
            }
            _ => {}
        }
    }

    fn is_read_only(addr: u16) -> bool {
        (addr >> 10) & 0x3 == 0x3
    }

    fn check_csr_access(&self, addr: u16, desc: &CsrDesc) -> XResult {
        let required = PrivilegeMode::from_bits(((addr >> 8) & 0x3) as Word);
        if self.privilege < required {
            return self.illegal_inst();
        }
        match desc.access {
            AccessRule::Standard => Ok(()),
            AccessRule::BlockedByMstatus(flag) => {
                let ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
                if ms.contains(flag) && self.privilege == PrivilegeMode::Supervisor {
                    return self.illegal_inst();
                }
                Ok(())
            }
            AccessRule::CounterGated => {
                let bit = counteren_bit(addr);
                match self.privilege {
                    PrivilegeMode::Machine => Ok(()),
                    PrivilegeMode::Supervisor => {
                        if (self.csr.get(CsrAddr::mcounteren) >> bit) & 1 == 0 {
                            return self.illegal_inst();
                        }
                        Ok(())
                    }
                    PrivilegeMode::User => {
                        let m_ok = (self.csr.get(CsrAddr::mcounteren) >> bit) & 1 == 1;
                        let s_ok = (self.csr.get(CsrAddr::scounteren) >> bit) & 1 == 1;
                        if !(m_ok && s_ok) {
                            return self.illegal_inst();
                        }
                        Ok(())
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::riscv::trap::test_helpers::assert_illegal_inst;

    #[test]
    fn csr_read_unknown_traps() {
        let core = RVCore::new();
        assert_illegal_inst(core.csr_read(0xFFF));
    }

    #[test]
    fn csr_write_unknown_traps() {
        let mut core = RVCore::new();
        assert_illegal_inst(core.csr_write(0xFFF, 0));
    }

    #[test]
    fn csr_write_read_only_by_encoding_traps() {
        let mut core = RVCore::new();
        assert_illegal_inst(core.csr_write(CsrAddr::mvendorid as u16, 42));
    }

    #[test]
    fn csr_read_privilege_violation() {
        let mut core = RVCore::new();
        core.privilege = PrivilegeMode::User;
        assert_illegal_inst(core.csr_read(CsrAddr::mstatus as u16));
    }

    #[test]
    fn counter_gated_m_mode_allowed() {
        let core = RVCore::new();
        assert!(core.csr_read(CsrAddr::cycle as u16).is_ok());
    }

    #[test]
    fn counter_gated_s_mode_blocked_by_mcounteren() {
        let mut core = RVCore::new();
        core.privilege = PrivilegeMode::Supervisor;
        core.csr.set(CsrAddr::mcounteren, 0);
        assert_illegal_inst(core.csr_read(CsrAddr::cycle as u16));
    }

    #[test]
    fn counter_gated_s_mode_allowed_by_mcounteren() {
        let mut core = RVCore::new();
        core.privilege = PrivilegeMode::Supervisor;
        core.csr.set(CsrAddr::mcounteren, 0x7);
        assert!(core.csr_read(CsrAddr::cycle as u16).is_ok());
    }

    #[test]
    fn counter_gated_u_mode_needs_both_counteren() {
        let mut core = RVCore::new();
        core.privilege = PrivilegeMode::User;

        core.csr.set(CsrAddr::mcounteren, 0x7);
        core.csr.set(CsrAddr::scounteren, 0);
        assert_illegal_inst(core.csr_read(CsrAddr::cycle as u16));
        core.csr.set(CsrAddr::scounteren, 0x7);
        assert!(core.csr_read(CsrAddr::cycle as u16).is_ok());
    }

    #[test]
    fn tvm_blocks_satp_in_s_mode() {
        let mut core = RVCore::new();
        core.privilege = PrivilegeMode::Supervisor;
        core.csr.set(CsrAddr::mstatus, MStatus::TVM.bits());
        assert_illegal_inst(core.csr_read(CsrAddr::satp as u16));
    }

    #[test]
    fn tvm_does_not_block_satp_in_m_mode() {
        let mut core = RVCore::new();
        core.csr.set(CsrAddr::mstatus, MStatus::TVM.bits());
        assert!(core.csr_read(CsrAddr::satp as u16).is_ok());
    }
}
