use memory_addr::VirtAddr;

use crate::{
    config::Word,
    cpu::riscv::{
        RVCore,
        csr::{CsrAddr, MStatus, PrivilegeMode, TrapCause},
        trap::{Exception, Interrupt, PendingTrap},
    },
};

impl RVCore {
    /// Samples mip & mie, respects global enables and delegation.
    /// Returns true if an interrupt trap was raised.
    pub fn check_pending_interrupts(&mut self) -> bool {
        let mip = self.csr.get(CsrAddr::mip);
        let mie = self.csr.get(CsrAddr::mie);
        let pending = mip & mie;
        if pending == 0 {
            return false;
        }

        let ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
        let mideleg = self.csr.get(CsrAddr::mideleg);

        for &irq in Interrupt::PRIORITY_ORDER {
            if pending & irq.bit() == 0 {
                continue;
            }
            let delegated = (mideleg >> irq as Word) & 1 == 1;

            let enabled = if irq.is_machine() && !delegated {
                // M-level interrupt, not delegated: taken in M-mode
                self.privilege != PrivilegeMode::Machine || ms.contains(MStatus::MIE)
            } else {
                // S-level or delegated M-level: taken in S-mode
                match self.privilege {
                    PrivilegeMode::User => true,
                    PrivilegeMode::Supervisor => ms.contains(MStatus::SIE),
                    PrivilegeMode::Machine => false, // S-mode interrupt cannot preempt M-mode
                }
            };

            if enabled {
                debug!("interrupt selected: {:?}, delegated={}", irq, delegated);
                self.raise_trap(TrapCause::Interrupt(irq), 0);
                return true;
            }
        }
        false
    }

    /// Commit a taken trap — writes CSRs and sets npc to handler.
    /// The caller advances pc = npc afterward.
    pub fn commit_trap(&mut self, trap: PendingTrap) {
        // In direct (bare-metal) mode, ebreak halts the emulator.
        // In firmware mode (ebreak_as_trap), dispatch to the trap handler.
        if trap.cause == TrapCause::Exception(Exception::Breakpoint) && !self.ebreak_as_trap {
            self.halted = true;
            return;
        }
        let delegated = self.is_delegated(&trap.cause);
        info!(
            "trap: {:?} -> {}-mode (tval={:#x})",
            trap.cause,
            if delegated { "S" } else { "M" },
            trap.tval
        );
        if delegated {
            self.trap_to_s_mode(&trap);
        } else {
            self.trap_to_m_mode(&trap);
        }
    }

    fn trap_to_s_mode(&mut self, trap: &PendingTrap) {
        let mut ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
        self.csr
            .set(CsrAddr::sepc, (self.pc.as_usize() as Word) & !1);
        self.csr.set(CsrAddr::scause, trap.cause.to_mcause());
        self.csr.set(CsrAddr::stval, trap.tval);
        ms = ms.with_spp(self.privilege);
        ms.set(MStatus::SPIE, ms.contains(MStatus::SIE));
        ms.remove(MStatus::SIE);
        self.csr.set(CsrAddr::mstatus, ms.bits());
        self.privilege = PrivilegeMode::Supervisor;
        self.npc = Self::trap_vector(self.csr.get(CsrAddr::stvec), &trap.cause);
        debug!("trap_to_s_mode: handler={:#x}", self.npc.as_usize());
    }

    fn trap_to_m_mode(&mut self, trap: &PendingTrap) {
        let mut ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
        self.csr
            .set(CsrAddr::mepc, (self.pc.as_usize() as Word) & !1);
        self.csr.set(CsrAddr::mcause, trap.cause.to_mcause());
        self.csr.set(CsrAddr::mtval, trap.tval);
        ms = ms.with_mpp(self.privilege);
        ms.set(MStatus::MPIE, ms.contains(MStatus::MIE));
        ms.remove(MStatus::MIE);
        self.csr.set(CsrAddr::mstatus, ms.bits());
        self.privilege = PrivilegeMode::Machine;
        self.npc = Self::trap_vector(self.csr.get(CsrAddr::mtvec), &trap.cause);
        debug!("trap_to_m_mode: handler={:#x}", self.npc.as_usize());
    }

    fn trap_vector(tvec: Word, cause: &TrapCause) -> VirtAddr {
        let base = (tvec & !0x3) as usize;
        let mode = tvec & 0x3;
        if mode == 1 && cause.is_interrupt() {
            VirtAddr::from(base + 4 * cause.code() as usize)
        } else {
            VirtAddr::from(base)
        }
    }

    pub fn do_mret(&mut self) {
        let mut ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
        let mpp = ms.mpp();
        ms.set(MStatus::MIE, ms.contains(MStatus::MPIE));
        ms.insert(MStatus::MPIE);
        ms = ms.with_mpp(PrivilegeMode::User);
        if mpp != PrivilegeMode::Machine {
            ms.remove(MStatus::MPRV);
        }
        self.csr.set(CsrAddr::mstatus, ms.bits());
        self.privilege = mpp;
        self.npc = VirtAddr::from(self.csr.get(CsrAddr::mepc) as usize);
        debug!(
            "mret: pc={:#x} -> {:?}",
            self.npc.as_usize(),
            self.privilege
        );
    }

    pub fn do_sret(&mut self) {
        let mut ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
        let spp = ms.spp();
        ms.set(MStatus::SIE, ms.contains(MStatus::SPIE));
        ms.insert(MStatus::SPIE);
        ms = ms.with_spp(PrivilegeMode::User);
        ms.remove(MStatus::MPRV); // sret always returns to S/U (< M)
        self.csr.set(CsrAddr::mstatus, ms.bits());
        self.privilege = spp;
        self.npc = VirtAddr::from(self.csr.get(CsrAddr::sepc) as usize);
        debug!(
            "sret: pc={:#x} -> {:?}",
            self.npc.as_usize(),
            self.privilege
        );
    }

    fn is_delegated(&self, cause: &TrapCause) -> bool {
        // Traps never transition from a more-privileged mode to a less-privileged mode.
        if self.privilege == PrivilegeMode::Machine {
            return false;
        }
        let bit = cause.code();
        match cause {
            TrapCause::Exception(_) => (self.csr.get(CsrAddr::medeleg) >> bit) & 1 == 1,
            TrapCause::Interrupt(_) => (self.csr.get(CsrAddr::mideleg) >> bit) & 1 == 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cpu::riscv::{
        csr::{CsrAddr, MStatus},
        trap::{Exception, Interrupt, TrapCause},
    };

    fn setup_core() -> RVCore {
        let mut core = RVCore::new();
        core.pc = VirtAddr::from(0x1000_usize);
        core.npc = core.pc;
        core.csr.set(CsrAddr::mtvec, 0x8000_0000);
        core.csr.set(CsrAddr::stvec, 0x4000_0000);
        core
    }

    #[test]
    fn trap_entry_writes_npc_not_pc() {
        let mut core = setup_core();
        let original_pc = core.pc;
        core.raise_trap(TrapCause::Exception(Exception::IllegalInstruction), 0);
        let trap = core.pending_trap.take().unwrap();
        core.commit_trap(trap);

        assert_eq!(core.pc, original_pc);
        assert_eq!(core.npc, VirtAddr::from(0x8000_0000_usize));
    }

    #[test]
    fn trap_entry_saves_mepc_mcause_mtval() {
        let mut core = setup_core();
        core.raise_trap(TrapCause::Exception(Exception::EcallFromU), 0x42);
        let trap = core.pending_trap.take().unwrap();
        core.commit_trap(trap);

        assert_eq!(core.csr.get(CsrAddr::mepc), 0x1000);
        assert_eq!(core.csr.get(CsrAddr::mcause), 8);
        assert_eq!(core.csr.get(CsrAddr::mtval), 0x42);
    }

    #[test]
    fn trap_entry_saves_mpp_and_clears_mie() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Supervisor;
        core.csr.set(CsrAddr::mstatus, MStatus::MIE.bits());

        core.raise_trap(TrapCause::Exception(Exception::EcallFromS), 0);
        let trap = core.pending_trap.take().unwrap();
        core.commit_trap(trap);

        let ms = MStatus::from_bits_truncate(core.csr.get(CsrAddr::mstatus));
        assert_eq!(ms.mpp(), PrivilegeMode::Supervisor);
        assert!(ms.contains(MStatus::MPIE));
        assert!(!ms.contains(MStatus::MIE));
        assert_eq!(core.privilege, PrivilegeMode::Machine);
    }

    #[test]
    fn delegation_routes_to_s_mode() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::User;
        core.csr.set(CsrAddr::medeleg, 1 << 8);

        core.raise_trap(TrapCause::Exception(Exception::EcallFromU), 0);
        let trap = core.pending_trap.take().unwrap();
        core.commit_trap(trap);

        assert_eq!(core.privilege, PrivilegeMode::Supervisor);
        assert_eq!(core.npc, VirtAddr::from(0x4000_0000_usize));
        assert_eq!(core.csr.get(CsrAddr::sepc), 0x1000);
        assert_eq!(core.csr.get(CsrAddr::scause), 8);
    }

    #[test]
    fn mret_restores_privilege_and_enables() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Machine;
        let ms = MStatus::empty().with_mpp(PrivilegeMode::Supervisor) | MStatus::MPIE;
        core.csr.set(CsrAddr::mstatus, ms.bits());
        core.csr.set(CsrAddr::mepc, 0x2000);

        core.do_mret();

        assert_eq!(core.privilege, PrivilegeMode::Supervisor);
        assert_eq!(core.npc, VirtAddr::from(0x2000_usize));
        let ms = MStatus::from_bits_truncate(core.csr.get(CsrAddr::mstatus));
        assert!(ms.contains(MStatus::MIE));
        assert!(ms.contains(MStatus::MPIE));
        assert_eq!(ms.mpp(), PrivilegeMode::User);
    }

    #[test]
    fn mret_clears_mprv_when_returning_below_m() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Machine;
        let ms = MStatus::MPRV | MStatus::empty().with_mpp(PrivilegeMode::Supervisor);
        core.csr.set(CsrAddr::mstatus, ms.bits());
        core.csr.set(CsrAddr::mepc, 0x2000);

        core.do_mret();

        let ms = MStatus::from_bits_truncate(core.csr.get(CsrAddr::mstatus));
        assert!(!ms.contains(MStatus::MPRV));
    }

    #[test]
    fn mret_keeps_mprv_when_returning_to_m() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Machine;
        let ms = MStatus::MPRV | MStatus::empty().with_mpp(PrivilegeMode::Machine);
        core.csr.set(CsrAddr::mstatus, ms.bits());
        core.csr.set(CsrAddr::mepc, 0x2000);

        core.do_mret();

        let ms = MStatus::from_bits_truncate(core.csr.get(CsrAddr::mstatus));
        assert!(ms.contains(MStatus::MPRV));
    }

    #[test]
    fn sret_restores_privilege_and_clears_mprv() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Supervisor;
        let ms = MStatus::MPRV | MStatus::SPIE | MStatus::empty().with_spp(PrivilegeMode::User);
        core.csr.set(CsrAddr::mstatus, ms.bits());
        core.csr.set(CsrAddr::sepc, 0x3000);

        core.do_sret();

        assert_eq!(core.privilege, PrivilegeMode::User);
        assert_eq!(core.npc, VirtAddr::from(0x3000_usize));
        let ms = MStatus::from_bits_truncate(core.csr.get(CsrAddr::mstatus));
        assert!(ms.contains(MStatus::SIE));
        assert!(!ms.contains(MStatus::MPRV));
    }

    #[test]
    fn m_mode_trap_never_delegates_to_s_mode() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Machine;
        core.csr.set(CsrAddr::medeleg, 1 << 11);

        core.raise_trap(TrapCause::Exception(Exception::EcallFromM), 0);
        let trap = core.pending_trap.take().unwrap();
        core.commit_trap(trap);

        assert_eq!(core.privilege, PrivilegeMode::Machine);
        assert_eq!(core.npc, VirtAddr::from(0x8000_0000_usize));
    }

    #[test]
    fn vectored_mtvec_computes_interrupt_offset() {
        let mut core = setup_core();
        core.csr.set(CsrAddr::mtvec, 0x8000_0001);

        core.raise_trap(TrapCause::Interrupt(Interrupt::MachineTimer), 0);
        let trap = core.pending_trap.take().unwrap();
        core.commit_trap(trap);

        assert_eq!(core.npc, VirtAddr::from(0x8000_001C_usize));
    }

    #[test]
    fn vectored_mtvec_uses_base_for_exceptions() {
        let mut core = setup_core();
        core.csr.set(CsrAddr::mtvec, 0x8000_0001);

        core.raise_trap(TrapCause::Exception(Exception::IllegalInstruction), 0);
        let trap = core.pending_trap.take().unwrap();
        core.commit_trap(trap);

        assert_eq!(core.npc, VirtAddr::from(0x8000_0000_usize));
    }

    #[test]
    fn full_roundtrip_ecall_trap_mret() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::User;
        let original_pc = core.pc;

        core.raise_trap(TrapCause::Exception(Exception::EcallFromU), 0);
        let trap = core.pending_trap.take().unwrap();
        core.commit_trap(trap);

        assert_eq!(core.privilege, PrivilegeMode::Machine);
        assert_eq!(core.npc, VirtAddr::from(0x8000_0000_usize));
        assert_eq!(core.csr.get(CsrAddr::mepc), original_pc.as_usize() as Word);

        core.do_mret();

        assert_eq!(core.privilege, PrivilegeMode::User);
        assert_eq!(core.npc, VirtAddr::from(original_pc.as_usize()));
    }

    // ---- Interrupt sampling tests ----

    #[test]
    fn no_pending_interrupts_returns_false() {
        let mut core = setup_core();
        assert!(!core.check_pending_interrupts());
    }

    #[test]
    fn pending_but_not_enabled_returns_false() {
        let mut core = setup_core();
        // Set pending but not enabled in mie
        core.csr.set(CsrAddr::mip, Interrupt::MachineTimer.bit());
        core.csr.set(CsrAddr::mie, 0);
        assert!(!core.check_pending_interrupts());
    }

    #[test]
    fn pending_enabled_but_globally_disabled_in_m_mode() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Machine;
        core.csr.set(CsrAddr::mip, Interrupt::MachineTimer.bit());
        core.csr.set(CsrAddr::mie, Interrupt::MachineTimer.bit());
        // MIE is clear → globally disabled
        core.csr.set(CsrAddr::mstatus, 0);
        assert!(!core.check_pending_interrupts());
    }

    #[test]
    fn pending_enabled_and_globally_enabled_in_m_mode() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Machine;
        core.csr.set(CsrAddr::mip, Interrupt::MachineTimer.bit());
        core.csr.set(CsrAddr::mie, Interrupt::MachineTimer.bit());
        core.csr.set(CsrAddr::mstatus, MStatus::MIE.bits());
        assert!(core.check_pending_interrupts());
        assert_eq!(
            core.pending_trap.unwrap().cause,
            TrapCause::Interrupt(Interrupt::MachineTimer),
        );
    }

    #[test]
    fn m_level_interrupt_always_fires_from_s_mode() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Supervisor;
        core.csr.set(CsrAddr::mip, Interrupt::MachineTimer.bit());
        core.csr.set(CsrAddr::mie, Interrupt::MachineTimer.bit());
        // Even without MIE set, M-level interrupts preempt S-mode
        core.csr.set(CsrAddr::mstatus, 0);
        assert!(core.check_pending_interrupts());
    }

    #[test]
    fn s_level_interrupt_respects_sie_in_s_mode() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Supervisor;
        let irq = Interrupt::SupervisorTimer;
        core.csr.set(CsrAddr::mip, irq.bit());
        core.csr.set(CsrAddr::mie, irq.bit());
        core.csr.set(CsrAddr::mideleg, irq.bit());

        // SIE clear → blocked
        core.csr.set(CsrAddr::mstatus, 0);
        assert!(!core.check_pending_interrupts());

        // SIE set → fires
        core.csr.set(CsrAddr::mstatus, MStatus::SIE.bits());
        assert!(core.check_pending_interrupts());
    }

    #[test]
    fn s_level_interrupt_cannot_preempt_m_mode() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Machine;
        let irq = Interrupt::SupervisorTimer;
        core.csr.set(CsrAddr::mip, irq.bit());
        core.csr.set(CsrAddr::mie, irq.bit());
        core.csr.set(CsrAddr::mideleg, irq.bit());
        core.csr
            .set(CsrAddr::mstatus, MStatus::MIE.bits() | MStatus::SIE.bits());
        assert!(!core.check_pending_interrupts());
    }

    #[test]
    fn highest_priority_interrupt_wins() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::Machine;
        let both = Interrupt::MachineTimer.bit() | Interrupt::MachineExternal.bit();
        core.csr.set(CsrAddr::mip, both);
        core.csr.set(CsrAddr::mie, both);
        core.csr.set(CsrAddr::mstatus, MStatus::MIE.bits());
        assert!(core.check_pending_interrupts());
        // MachineExternal has higher priority than MachineTimer
        assert_eq!(
            core.pending_trap.unwrap().cause,
            TrapCause::Interrupt(Interrupt::MachineExternal),
        );
    }

    #[test]
    fn delegated_interrupt_routes_to_s_mode_handler() {
        let mut core = setup_core();
        core.privilege = PrivilegeMode::User;
        let irq = Interrupt::SupervisorTimer;
        core.csr.set(CsrAddr::mip, irq.bit());
        core.csr.set(CsrAddr::mie, irq.bit());
        core.csr.set(CsrAddr::mideleg, irq.bit());

        assert!(core.check_pending_interrupts());
        let trap = core.pending_trap.take().unwrap();
        core.commit_trap(trap);
        assert_eq!(core.privilege, PrivilegeMode::Supervisor);
        assert_eq!(core.npc, VirtAddr::from(0x4000_0000_usize));
    }
}
