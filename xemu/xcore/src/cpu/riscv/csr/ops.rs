//! CSR read/write operations with privilege checks and write side-effects.

use super::{AccessRule, CsrAddr, CsrDesc, MStatus, PrivilegeMode, counteren_bit, find_desc};
use crate::{config::Word, cpu::riscv::RVCore, error::XResult};

impl RVCore {
    pub(in crate::cpu::riscv) fn csr_read(&self, addr: u16) -> XResult<Word> {
        let desc = find_desc(addr)
            .filter(|_| !Self::is_illegal_csr(addr))
            .ok_or(self.illegal_inst())?;

        self.check_csr_access(addr, &desc)?;
        Ok(self.csr.read_with_desc(desc))
    }

    pub(in crate::cpu::riscv) fn csr_write(&mut self, addr: u16, val: Word) -> XResult {
        let desc = find_desc(addr)
            .filter(|_| !Self::is_illegal_csr(addr))
            .filter(|_| !Self::is_read_only(addr))
            .ok_or(self.illegal_inst())?;

        self.check_csr_access(addr, &desc)?;
        self.csr.write_with_desc(desc, val);
        self.csr_write_side_effects(addr, desc);
        Ok(())
    }

    fn csr_write_side_effects(&mut self, addr: u16, desc: CsrDesc) {
        match addr {
            0x180 /* satp */ => {
                // WARL: unsupported MODE → entire write has no effect (§4.1.11).
                #[cfg(isa64)]
                {
                    let satp = self.csr.get(CsrAddr::satp);
                    let mode = satp >> 60;
                    if mode != 0 && mode != 8 {
                        self.csr.set(CsrAddr::satp, satp & ((1u64 << 60) - 1));
                    }
                }
                let satp = self.csr.get(CsrAddr::satp);
                debug!("satp write: {:#x}", satp);
                self.mmu.update_satp(satp);
            }
            0x300 /* mstatus */ | 0x100 /* sstatus */ => {
                // Recompute SD from FS dirtiness (§3.1.6)
                let ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
                let sd = if ms.contains(MStatus::FS) { MStatus::SD } else { MStatus::empty() };
                self.csr
                    .set(CsrAddr::mstatus, ((ms - MStatus::SD) | sd).bits());
                self.mmu
                    .update_mstatus(ms.contains(MStatus::SUM), ms.contains(MStatus::MXR));
            }
            0x3A0..=0x3A3 /* pmpcfg */ => {
                debug!("pmpcfg write: csr={:#x} val={:#x}", addr, self.csr.get_by_addr(addr));
                if let Some(base) = Self::pmpcfg_base(addr) {
                    let val = self.csr.get_by_addr(addr);
                    let mut wb: Word = 0;
                    for i in 0..std::mem::size_of::<Word>() {
                        self.pmp.update_cfg(base + i, (val >> (i * 8)) as u8);
                        wb |= (self.pmp.get_cfg(base + i) as Word) << (i * 8);
                    }
                    self.csr.write_with_desc(desc, wb);
                }
            }
            0x3B0..=0x3BF /* pmpaddr */ => {
                let idx = (addr - 0x3B0) as usize;
                debug!("pmpaddr write: idx={} val={:#x}", idx, self.csr.get_by_addr(addr));
                self.pmp.update_addr(idx, self.csr.get_by_addr(addr) as usize);
                self.csr.write_with_desc(desc, self.pmp.get_addr(idx) as Word);
            }
            _ => {}
        }
        // FP CSR writes transition FS to Dirty (privileged spec §3.1.6.5)
        if matches!(desc.access, AccessRule::RequireFP) {
            self.dirty_fp();
        }
    }

    /// RV64 pmpcfg base entry index, or None for illegal odd-indexed CSRs.
    fn pmpcfg_base(addr: u16) -> Option<usize> {
        #[cfg(isa64)]
        {
            match addr {
                0x3A0 => Some(0),
                0x3A2 => Some(8),
                _ => None,
            }
        }
        #[cfg(isa32)]
        {
            Some((addr - 0x3A0) as usize * 4)
        }
    }

    #[cfg(isa64)]
    fn is_illegal_csr(addr: u16) -> bool {
        addr == CsrAddr::pmpcfg1 as u16 || addr == CsrAddr::pmpcfg3 as u16
    }

    #[cfg(isa32)]
    fn is_illegal_csr(_addr: u16) -> bool {
        false
    }

    fn is_read_only(addr: u16) -> bool {
        (addr >> 10) & 0x3 == 0x3
    }

    fn check_csr_access(&self, addr: u16, desc: &CsrDesc) -> XResult {
        let required = PrivilegeMode::from_bits(((addr >> 8) & 0x3) as Word);
        (self.privilege >= required).ok_or(self.illegal_inst())?;
        match desc.access {
            AccessRule::Standard => true,
            AccessRule::BlockedByMstatus(flag) => {
                let ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
                !(self.privilege == PrivilegeMode::Supervisor && ms.contains(flag))
            }
            AccessRule::CounterGated => {
                let bit = counteren_bit(addr);
                let m_ok = (self.csr.get(CsrAddr::mcounteren) >> bit) & 1 == 1;
                let s_ok = (self.csr.get(CsrAddr::scounteren) >> bit) & 1 == 1;

                match self.privilege {
                    PrivilegeMode::Machine => true,
                    PrivilegeMode::Supervisor => m_ok,
                    PrivilegeMode::User => m_ok && s_ok,
                }
            }
            AccessRule::RequireFP => (self.csr.get(CsrAddr::mstatus) >> 13) & 0x3 != 0,
        }
        .ok_or(self.illegal_inst())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::riscv::trap::test_helpers::assert_illegal_inst;

    #[test]
    fn csr_read_unknown_traps() {
        assert_illegal_inst(RVCore::new().csr_read(0xFFF));
    }

    #[test]
    fn csr_write_unknown_traps() {
        assert_illegal_inst(RVCore::new().csr_write(0xFFF, 0));
    }

    #[test]
    fn csr_write_read_only_traps() {
        assert_illegal_inst(RVCore::new().csr_write(CsrAddr::mvendorid as u16, 42));
    }

    #[test]
    fn csr_read_privilege_violation() {
        let mut core = RVCore::new();
        core.privilege = PrivilegeMode::User;
        assert_illegal_inst(core.csr_read(CsrAddr::mstatus as u16));
    }

    #[test]
    fn counter_gated_m_mode_allowed() {
        assert!(RVCore::new().csr_read(CsrAddr::cycle as u16).is_ok());
    }

    #[test]
    fn counter_gated_s_mode() {
        let mut core = RVCore::new();
        core.privilege = PrivilegeMode::Supervisor;
        core.csr.set(CsrAddr::mcounteren, 0);
        assert_illegal_inst(core.csr_read(CsrAddr::cycle as u16));
        core.csr.set(CsrAddr::mcounteren, 0x7);
        assert!(core.csr_read(CsrAddr::cycle as u16).is_ok());
    }

    #[test]
    fn counter_gated_u_mode_needs_both() {
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
