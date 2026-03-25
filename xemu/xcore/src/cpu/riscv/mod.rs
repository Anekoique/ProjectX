pub mod csr;
mod inst;
mod mem;
pub mod trap;

use memory_addr::{MemoryAddr, VirtAddr};

pub use self::{RVCore as Core, trap::PendingTrap};
use self::{
    csr::{CsrFile, PrivilegeMode},
    trap::TrapCause,
};
use super::{CoreOps, MemOps, RESET_VECTOR};
use crate::{
    config::Word,
    error::XResult,
    isa::{DECODER, DecodedInst, RVReg},
};

pub struct RVCore {
    gpr: [Word; 32],
    pc: VirtAddr,
    npc: VirtAddr,
    pub(crate) csr: CsrFile,
    pub(crate) privilege: PrivilegeMode,
    pub(crate) pending_trap: Option<PendingTrap>,
    pub(crate) reservation: Option<usize>,
    halted: bool,
}

impl RVCore {
    pub fn new() -> Self {
        Self {
            gpr: [0; 32],
            pc: VirtAddr::from(0),
            npc: VirtAddr::from(0),
            csr: CsrFile::new(),
            privilege: PrivilegeMode::Machine,
            pending_trap: None,
            reservation: None,
            halted: false,
        }
    }

    pub fn raise_trap(&mut self, cause: TrapCause, tval: Word) {
        debug_assert!(
            self.pending_trap.is_none(),
            "raise_trap called while trap already pending: {:?}",
            self.pending_trap
        );
        self.pending_trap = Some(PendingTrap { cause, tval });
    }

    fn decode(&self, raw: u32) -> XResult<DecodedInst> {
        DECODER.decode(raw)
    }

    fn execute(&mut self, inst: DecodedInst) -> XResult {
        trace!("Executing instruction at pc={:#x}: {:?}", self.pc, inst);
        let is_compressed = matches!(&inst, DecodedInst::C { .. });
        self.npc = self.pc.wrapping_add(if is_compressed { 2 } else { 4 });
        self.dispatch(inst)
    }

    /// Commit any pending trap, advance pc and counters.
    fn retire(&mut self) {
        if let Some(trap) = self.pending_trap.take() {
            self.commit_trap(trap);
        } else {
            self.csr.increment_instret();
        }

        self.pc = self.npc;
        self.csr.increment_cycle();
    }
}

impl CoreOps for RVCore {
    fn pc(&self) -> VirtAddr {
        self.pc
    }

    fn reset(&mut self) -> XResult {
        self.gpr.fill(0);
        self.pc = VirtAddr::from(RESET_VECTOR);
        self.npc = self.pc;
        self.csr = CsrFile::new();
        self.privilege = PrivilegeMode::Machine;
        self.pending_trap = None;
        self.reservation = None;
        self.halted = false;
        Ok(())
    }

    fn step(&mut self) -> XResult {
        if self.check_pending_interrupts() {
            self.retire();
            return Ok(());
        }

        self.trap_on_err(|core| {
            let raw = core.fetch()?;
            let inst = core.decode(raw)?;
            core.execute(inst)
        })?;

        self.retire();
        Ok(())
    }

    fn halted(&self) -> bool {
        self.halted
    }

    fn halt_ret(&self) -> Word {
        self.gpr[RVReg::a0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::CONFIG_MBASE,
        cpu::riscv::{csr::CsrAddr, trap::Exception},
        memory::with_mem,
    };

    #[test]
    fn new_core_starts_in_machine_mode() {
        let core = RVCore::new();
        assert_eq!(core.privilege, PrivilegeMode::Machine);
        assert!(core.pending_trap.is_none());
    }

    #[test]
    fn reset_restores_machine_mode() {
        let mut core = RVCore::new();
        core.privilege = PrivilegeMode::User;
        core.raise_trap(TrapCause::Exception(Exception::Breakpoint), 0);
        core.reset().unwrap();
        assert_eq!(core.privilege, PrivilegeMode::Machine);
        assert!(core.pending_trap.is_none());
        assert_eq!(core.pc, VirtAddr::from(RESET_VECTOR));
    }

    #[test]
    fn raise_trap_sets_pending() {
        let mut core = RVCore::new();
        assert!(core.pending_trap.is_none());
        core.raise_trap(TrapCause::Exception(Exception::EcallFromM), 0);
        let trap = core.pending_trap.unwrap();
        assert_eq!(trap.cause, TrapCause::Exception(Exception::EcallFromM));
        assert_eq!(trap.tval, 0);
    }

    #[test]
    fn fetch_distinguishes_standard_and_compressed_instructions() {
        let mut core = RVCore::new();
        core.pc = VirtAddr::from(CONFIG_MBASE);

        let cases = [
            (0xCAFEBABF_u32, 0xCAFEBABF_u32),
            (0xCAFEBABE_u32, 0xBABE_u32),
        ];

        for (inst, expected) in cases {
            with_mem!(write(core.virt_to_phys(core.pc), 4, inst as Word)).unwrap();
            assert_eq!(core.fetch().unwrap(), expected);
        }
    }

    fn setup_core() -> RVCore {
        let mut core = RVCore::new();
        core.pc = VirtAddr::from(CONFIG_MBASE);
        core.npc = core.pc;
        core.csr.set(CsrAddr::mtvec, 0x8000_0000);
        core
    }

    #[test]
    fn step_ebreak_halts_without_trap() {
        let mut core = setup_core();
        // ebreak encoding: 0x00100073
        with_mem!(write(
            core.virt_to_phys(core.pc),
            4,
            0x0010_0073_u32 as Word
        ))
        .unwrap();
        core.step().unwrap();
        assert!(core.halted());
        assert_eq!(core.csr.get(CsrAddr::mepc), 0);
    }

    #[test]
    fn step_normal_instruction_succeeds() {
        let mut core = setup_core();
        // auipc t0, 0 → 0x00000297
        with_mem!(write(
            core.virt_to_phys(core.pc),
            4,
            0x0000_0297_u32 as Word
        ))
        .unwrap();
        core.step().unwrap();
    }

    #[test]
    fn cycle_increments_on_trap_instret_does_not() {
        let mut core = setup_core();
        // ecall encoding: 0x00000073
        with_mem!(write(
            core.virt_to_phys(core.pc),
            4,
            0x0000_0073_u32 as Word
        ))
        .unwrap();
        core.step().unwrap();
        assert_eq!(core.csr.get(CsrAddr::cycle), 1);
        assert_eq!(core.csr.get(CsrAddr::instret), 0);
    }

    #[test]
    fn cycle_and_instret_both_increment_on_normal_step() {
        let mut core = setup_core();
        // auipc t0, 0
        with_mem!(write(
            core.virt_to_phys(core.pc),
            4,
            0x0000_0297_u32 as Word
        ))
        .unwrap();
        core.step().unwrap();
        assert_eq!(core.csr.get(CsrAddr::cycle), 1);
        assert_eq!(core.csr.get(CsrAddr::instret), 1);
    }
}
