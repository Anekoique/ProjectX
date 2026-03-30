pub mod context;
pub mod csr;
pub mod debug;
mod inst;
pub(crate) mod mm;
pub mod trap;

use std::sync::{Arc, Mutex};

use memory_addr::{MemoryAddr, VirtAddr};

pub use self::{RVCore as Core, context::RVCoreContext as CoreContext, trap::PendingTrap};
use self::{
    csr::{CsrAddr, CsrFile, PrivilegeMode},
    mm::{Mmu, Pmp},
    trap::TrapCause,
};
use super::{CoreOps, RESET_VECTOR};
use crate::{
    config::{CONFIG_MBASE, CONFIG_MSIZE, Word},
    device::{
        HW_IP_MASK, IrqState, SSIP,
        bus::Bus,
        intc::{aclint::Aclint, plic::Plic},
        uart::Uart,
    },
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
    pub(crate) bus: Arc<Mutex<Bus>>,
    pub(crate) mmu: Mmu,
    pub(crate) pmp: Pmp,
    irq: IrqState,
    halted: bool,
    breakpoints: Vec<crate::cpu::debug::Breakpoint>,
    next_bp_id: u32,
    skip_bp_once: bool,
}

impl RVCore {
    pub fn new() -> Self {
        let irq = IrqState::new();
        let mut bus = Bus::new(CONFIG_MBASE, CONFIG_MSIZE);
        bus.add_mmio(
            "aclint",
            0x0200_0000,
            0x1_0000,
            Box::new(Aclint::new(irq.clone(), bus.ssip_flag())),
            0,
        );
        let plic_idx = bus.mmio.len();
        bus.add_mmio(
            "plic",
            0x0C00_0000,
            0x400_0000,
            Box::new(Plic::new(irq.clone())),
            0,
        );
        bus.set_irq_sink(plic_idx);
        bus.add_mmio("uart0", 0x1000_0000, 0x100, Box::new(Uart::new()), 10);
        Self::with_bus(Arc::new(Mutex::new(bus)), irq)
    }

    pub fn with_bus(bus: Arc<Mutex<Bus>>, irq: IrqState) -> Self {
        Self {
            gpr: [0; 32],
            pc: VirtAddr::from(0),
            npc: VirtAddr::from(0),
            csr: CsrFile::new(),
            privilege: PrivilegeMode::Machine,
            pending_trap: None,
            reservation: None,
            bus,
            mmu: Mmu::new(),
            pmp: Pmp::new(),
            irq,
            halted: false,
            breakpoints: Vec::new(),
            next_bp_id: 1,
            skip_bp_once: false,
        }
    }

    /// Merge hardware interrupt bits from devices into mip.
    /// SSIP is handled separately via ACLINT edge-triggered SSWI.
    fn sync_interrupts(&mut self) {
        let ext = self.irq.load() as Word;
        let mip = self.csr.get(CsrAddr::mip);
        // Merge hardware-wired bits (MSIP, MTIP, SEIP, MEIP); SSIP stays
        // software-controlled
        self.csr
            .set(CsrAddr::mip, (mip & !HW_IP_MASK) | (ext & HW_IP_MASK));
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

    fn bus(&self) -> &Arc<Mutex<Bus>> {
        &self.bus
    }

    fn reset(&mut self) -> XResult {
        self.gpr.fill(0);
        self.pc = VirtAddr::from(RESET_VECTOR);
        self.npc = self.pc;
        self.csr = CsrFile::new();
        self.privilege = PrivilegeMode::Machine;
        self.pending_trap = None;
        self.reservation = None;
        self.mmu = Mmu::new();
        self.pmp = Pmp::new();
        self.irq.reset();
        self.bus.lock().unwrap().reset_devices(); // IR-004
        self.halted = false;
        Ok(())
    }

    fn step(&mut self) -> XResult {
        {
            let mut bus = self.bus.lock().unwrap();
            bus.tick();
            if bus.take_ssip() {
                let mip = self.csr.get(CsrAddr::mip);
                self.csr.set(CsrAddr::mip, mip | SSIP as Word);
            }
        }
        self.sync_interrupts();

        // Breakpoint check (before instruction execution)
        #[cfg(feature = "debug")]
        {
            let pc = self.pc.as_usize();
            if !self.skip_bp_once && self.breakpoints.iter().any(|bp| bp.addr == pc) {
                return Err(crate::error::XError::DebugBreak(pc));
            }
            self.skip_bp_once = false;
        }

        if self.check_pending_interrupts() {
            self.retire();
            return Ok(());
        }

        self.trap_on_err(|core| {
            let raw = core.fetch()?;
            let inst = core.decode(raw)?;

            #[cfg(feature = "debug")]
            trace!(
                "{:#010x}: {:08x}  {}",
                core.pc.as_usize(),
                raw,
                debug::format_mnemonic(&inst),
            );

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
    use crate::cpu::riscv::{csr::CsrAddr, trap::Exception};

    fn setup_core() -> RVCore {
        let mut core = RVCore::new();
        core.pc = VirtAddr::from(CONFIG_MBASE);
        core.npc = core.pc;
        core.csr.set(CsrAddr::mtvec, 0x8000_0000);
        core
    }

    fn write_inst(core: &RVCore, inst: u32) {
        core.bus
            .lock()
            .unwrap()
            .write(core.pc.as_usize(), 4, inst as Word)
            .unwrap();
    }

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
        core.raise_trap(TrapCause::Exception(Exception::EcallFromM), 0);
        let trap = core.pending_trap.unwrap();
        assert_eq!(trap.cause, TrapCause::Exception(Exception::EcallFromM));
        assert_eq!(trap.tval, 0);
    }

    #[test]
    fn fetch_distinguishes_standard_and_compressed_instructions() {
        let mut core = setup_core();
        for (raw, expected) in [
            (0xCAFEBABF_u32, 0xCAFEBABF_u32),
            (0xCAFEBABE_u32, 0xBABE_u32),
        ] {
            write_inst(&core, raw);
            assert_eq!(core.fetch().unwrap(), expected);
        }
    }

    #[test]
    fn step_ebreak_halts_without_trap() {
        let mut core = setup_core();
        write_inst(&core, 0x0010_0073); // ebreak
        core.step().unwrap();
        assert!(core.halted());
        assert_eq!(core.csr.get(CsrAddr::mepc), 0);
    }

    #[test]
    fn step_normal_instruction_succeeds() {
        let mut core = setup_core();
        write_inst(&core, 0x0000_0297); // auipc t0, 0
        core.step().unwrap();
    }

    #[test]
    fn cycle_increments_on_trap_instret_does_not() {
        let mut core = setup_core();
        write_inst(&core, 0x0000_0073); // ecall
        core.step().unwrap();
        assert_eq!(core.csr.get(CsrAddr::cycle), 1);
        assert_eq!(core.csr.get(CsrAddr::instret), 0);
    }

    #[test]
    fn cycle_and_instret_both_increment_on_normal_step() {
        let mut core = setup_core();
        write_inst(&core, 0x0000_0297); // auipc t0, 0
        core.step().unwrap();
        assert_eq!(core.csr.get(CsrAddr::cycle), 1);
        assert_eq!(core.csr.get(CsrAddr::instret), 1);
    }

    #[test]
    fn sswi_edge_delivered_once_and_clearable() {
        use crate::device::SSIP;
        let mut core = setup_core();
        write_inst(&core, 0x0000_0297); // auipc t0, 0 (harmless NOP-like)

        // Write SETSSIP=1 via ACLINT MMIO (base=0x0200_0000, offset=0xC000)
        core.bus.lock().unwrap().write(0x0200_C000, 4, 1).unwrap();

        // Step: SSIP should be set in mip after step
        core.step().unwrap();
        assert_ne!(
            core.csr.get(CsrAddr::mip) as u64 & SSIP,
            0,
            "SSIP should be set after SSWI"
        );

        // Guest clears SSIP via CSR write (mip bit 1 is software-writable)
        let mip = core.csr.get(CsrAddr::mip);
        core.csr.set(CsrAddr::mip, mip & !(SSIP as Word));
        assert_eq!(
            core.csr.get(CsrAddr::mip) as u64 & SSIP,
            0,
            "SSIP should be cleared"
        );

        // Re-write the instruction for next step
        write_inst(&core, 0x0000_0297);

        // Step again: SSIP should NOT be reasserted (no new SETSSIP write)
        core.step().unwrap();
        assert_eq!(
            core.csr.get(CsrAddr::mip) as u64 & SSIP,
            0,
            "SSIP must not be reasserted without new SETSSIP"
        );
    }
}
