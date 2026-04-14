//! RISC-V CPU core: RV32/RV64 IMAFDCZicsr with M/S/U privilege modes.
//!
//! [`RVCore`] implements the fetch–decode–execute–retire pipeline, with
//! interrupt synchronization, trap delivery, and hardware A/D bit updates.

pub mod context;
pub mod csr;
pub mod debug;
mod inst;
pub(crate) mod mm;
pub mod trap;

use std::sync::{Arc, Mutex};

use memory_addr::{MemoryAddr, VirtAddr};

use self::{
    csr::{CsrAddr, CsrFile, MStatus, Mip, PrivilegeMode},
    mm::{Mmu, Pmp},
    trap::{PendingTrap, TrapCause, interrupt::HW_IP_MASK},
};
use crate::{
    config::Word,
    cpu::{
        RESET_VECTOR,
        core::{CoreOps, HartId},
    },
    device::{IrqState, bus::Bus},
    error::{XError, XResult},
    isa::{DECODER, DecodedInst, RVReg},
};

/// RISC-V CPU core: registers, CSR file, MMU, PMP, bus, and trap state.
pub struct RVCore {
    pub(in crate::arch::riscv) id: HartId,
    pub(in crate::arch::riscv) gpr: [Word; 32],
    pub(in crate::arch::riscv) fpr: [u64; 32],
    pub(in crate::arch::riscv) pc: VirtAddr,
    pub(in crate::arch::riscv) npc: VirtAddr,
    pub(in crate::arch::riscv) csr: CsrFile,
    pub(in crate::arch::riscv) privilege: PrivilegeMode,
    pub(in crate::arch::riscv) pending_trap: Option<PendingTrap>,
    pub(in crate::arch::riscv) bus: Arc<Mutex<Bus>>,
    pub(in crate::arch::riscv) mmu: Mmu,
    pub(in crate::arch::riscv) pmp: Pmp,
    pub(in crate::arch::riscv) irq: IrqState,
    pub(in crate::arch::riscv) halted: bool,
    /// When true, ebreak traps to handler instead of halting (firmware mode).
    pub(in crate::arch::riscv) ebreak_as_trap: bool,
    pub(in crate::arch::riscv) breakpoints: Vec<crate::cpu::debug::Breakpoint>,
    pub(in crate::arch::riscv) next_bp_id: u32,
    pub(in crate::arch::riscv) skip_bp_once: bool,
}

impl RVCore {
    /// Create a default single-hart RVCore around a fresh single-hart bus.
    /// Convenience for tests; production builds use
    /// [`crate::cpu::CPU::from_config`].
    pub fn new() -> Self {
        use crate::config::{CONFIG_MBASE, CONFIG_MSIZE};
        let bus = Arc::new(Mutex::new(Bus::new(CONFIG_MBASE, CONFIG_MSIZE, 1)));
        Self::with_id(HartId(0), bus, IrqState::new())
    }

    /// Create an RVCore for `id` sharing the given bus and IRQ state.
    pub fn with_id(id: HartId, bus: Arc<Mutex<Bus>>, irq: IrqState) -> Self {
        let mut core = Self {
            id,
            gpr: [0; 32],
            fpr: [0; 32],
            pc: VirtAddr::from(0),
            npc: VirtAddr::from(0),
            csr: CsrFile::new(),
            privilege: PrivilegeMode::Machine,
            pending_trap: None,
            bus,
            mmu: Mmu::new(),
            pmp: Pmp::new(),
            irq,
            halted: false,
            ebreak_as_trap: false,
            breakpoints: Vec::new(),
            next_bp_id: 1,
            skip_bp_once: false,
        };
        core.csr.set(CsrAddr::mhartid, id.0 as Word);
        core
    }

    /// Merge hardware interrupt bits from devices into mip.
    /// SSIP handled via ACLINT SSWI; STIP driven by Sstc stimecmp.
    #[allow(clippy::unnecessary_cast)]
    fn sync_interrupts(&mut self) {
        let hw = self.irq.load() as Word;
        let stip: Word = if self.stip_asserted() {
            Mip::STIP.bits()
        } else {
            0
        };
        let mip = (self.csr.get(CsrAddr::mip) & !HW_IP_MASK & !Mip::STIP.bits())
            | (hw & HW_IP_MASK)
            | stip;
        self.csr.set(CsrAddr::mip, mip);
    }

    /// True iff `mip`'s STIP bit should be asserted for the current time.
    #[allow(clippy::unnecessary_cast)]
    fn stip_asserted(&self) -> bool {
        self.csr.get(CsrAddr::time) as u64 >= self.csr.get(CsrAddr::stimecmp) as u64
    }

    /// True iff `mip`'s current STIP bit matches the computed `stip_asserted`.
    fn stip_in_sync(&self) -> bool {
        (self.csr.get(CsrAddr::mip) & Mip::STIP.bits() != 0) == self.stip_asserted()
    }

    /// Trap if mstatus.FS == Off (floating-point disabled).
    pub(crate) fn require_fp(&self) -> XResult {
        ((self.csr.get(CsrAddr::mstatus) >> 13) & 0x3 != 0).ok_or(XError::InvalidInst)
    }

    /// Set mstatus.FS = Dirty (0b11) and SD = 1.
    pub(crate) fn dirty_fp(&mut self) {
        let ms = self.csr.get(CsrAddr::mstatus);
        self.csr.set(
            CsrAddr::mstatus,
            ms | MStatus::FS.bits() | MStatus::SD.bits(),
        );
    }

    /// Queue a trap for delivery during the next retire phase.
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
    fn id(&self) -> HartId {
        self.id
    }

    fn pc(&self) -> VirtAddr {
        self.pc
    }

    fn setup_boot(&mut self, mode: crate::cpu::core::BootMode) {
        use crate::cpu::core::BootMode;
        match mode {
            BootMode::Direct => {
                self.ebreak_as_trap = false;
            }
            BootMode::Firmware { fdt_addr } => {
                // SBI convention: a0 = hartid, a1 = FDT pointer.
                // Secondary harts get parked by OpenSBI until released via MSIP.
                self.gpr[RVReg::a0] = self.id.as_word();
                self.gpr[RVReg::a1] = fdt_addr as Word;
                self.ebreak_as_trap = true;
            }
        }
    }

    fn reset(&mut self) -> XResult {
        self.gpr.fill(0);
        self.fpr.fill(0);
        self.pc = VirtAddr::from(RESET_VECTOR);
        self.npc = self.pc;
        self.csr = CsrFile::new();
        self.csr.set(CsrAddr::mhartid, self.id.as_word());
        self.privilege = PrivilegeMode::Machine;
        self.pending_trap = None;
        self.mmu = Mmu::new();
        self.pmp = Pmp::new();
        self.irq.reset();
        self.halted = false;
        Ok(())
    }

    #[allow(clippy::unnecessary_cast)]
    fn step(&mut self) -> XResult {
        // Time always advances — rdtime + stimecmp comparisons need it.
        let mtime = self.bus.lock().unwrap().mtime();
        self.csr.set(CsrAddr::time, mtime as Word);

        // Event-driven `mip` sync: redo the merge only when a producer
        // published since the last step OR when STIP's time-driven bit is
        // out of sync with what `stimecmp` demands. `raise_ssip_edge` always
        // bumps the epoch, so the ssip_edge consume rides inside the same
        // gate without an extra atomic swap per step.
        if self.irq.take_epoch() || !self.stip_in_sync() {
            if self.irq.take_ssip_edge() {
                let mip = self.csr.get(CsrAddr::mip);
                self.csr.set(CsrAddr::mip, mip | Mip::SSIP.bits());
            }
            self.sync_interrupts();
        }

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
            trace!("{:#010x}: {:08x}  {}", core.pc.as_usize(), raw, inst,);

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
        arch::riscv::cpu::{csr::CsrAddr, trap::Exception},
        config::CONFIG_MBASE,
    };

    fn setup_core() -> RVCore {
        let mut core = RVCore::new();
        core.pc = VirtAddr::from(CONFIG_MBASE);
        core.npc = core.pc;
        core.csr.set(CsrAddr::mtvec, 0x8000_0000);
        core
    }

    fn write_inst(core: &mut RVCore, inst: u32) {
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
            write_inst(&mut core, raw);
            assert_eq!(core.fetch().unwrap(), expected);
        }
    }

    #[test]
    fn step_ebreak_halts_in_direct_mode() {
        let mut core = setup_core();
        write_inst(&mut core, 0x0010_0073); // ebreak
        core.step().unwrap();
        assert!(core.halted());
    }

    #[test]
    fn step_ebreak_traps_in_firmware_mode() {
        let mut core = setup_core();
        core.ebreak_as_trap = true;
        write_inst(&mut core, 0x0010_0073); // ebreak
        core.step().unwrap();
        assert!(!core.halted());
        assert_eq!(core.csr.get(CsrAddr::mcause), Exception::Breakpoint as Word);
    }

    #[test]
    fn step_normal_instruction_succeeds() {
        let mut core = setup_core();
        write_inst(&mut core, 0x0000_0297); // auipc t0, 0
        core.step().unwrap();
    }

    #[test]
    fn cycle_increments_on_trap_instret_does_not() {
        let mut core = setup_core();
        write_inst(&mut core, 0x0000_0073); // ecall
        core.step().unwrap();
        assert_eq!(core.csr.get(CsrAddr::cycle), 1);
        assert_eq!(core.csr.get(CsrAddr::instret), 0);
    }

    #[test]
    fn cycle_and_instret_both_increment_on_normal_step() {
        let mut core = setup_core();
        write_inst(&mut core, 0x0000_0297); // auipc t0, 0
        core.step().unwrap();
        assert_eq!(core.csr.get(CsrAddr::cycle), 1);
        assert_eq!(core.csr.get(CsrAddr::instret), 1);
    }

    #[test]
    fn sswi_edge_delivered_once_and_clearable() {
        let mut core = setup_core();
        let ssip = Mip::SSIP.bits();
        write_inst(&mut core, 0x0000_0297); // auipc t0, 0 (harmless NOP-like)

        // Raise the SSIP edge on the core's IrqState directly (equivalent to
        // a SETSSIP write hitting the hart's edge flag). RVCore::new() in
        // tests does not install ACLINT MMIO, so we drive the edge manually.
        core.irq.raise_ssip_edge();

        // Step: SSIP should be set in mip after step
        core.step().unwrap();
        assert_ne!(
            core.csr.get(CsrAddr::mip) & ssip,
            0,
            "SSIP should be set after SSWI"
        );

        // Guest clears SSIP via CSR write (mip bit 1 is software-writable)
        let mip = core.csr.get(CsrAddr::mip);
        core.csr.set(CsrAddr::mip, mip & !ssip);
        assert_eq!(
            core.csr.get(CsrAddr::mip) & ssip,
            0,
            "SSIP should be cleared"
        );

        // Re-write the instruction for next step
        write_inst(&mut core, 0x0000_0297);

        // Step again: SSIP should NOT be reasserted (no new SETSSIP write)
        core.step().unwrap();
        assert_eq!(
            core.csr.get(CsrAddr::mip) & ssip,
            0,
            "SSIP must not be reasserted without new SETSSIP"
        );
    }

    #[test]
    fn stip_delivered_in_s_mode_with_sie() {
        use crate::arch::riscv::cpu::{csr::MStatus, trap::Interrupt};

        let mut core = setup_core();
        // Switch to S-mode
        core.privilege = PrivilegeMode::Supervisor;
        // Set stvec for trap delivery
        core.csr.set(CsrAddr::stvec, 0x8000_4000);
        // Enable STIE in mie (bit 5)
        core.csr.set(CsrAddr::mie, 1 << 5);
        // Delegate STIP to S-mode
        core.csr.set(CsrAddr::mideleg, 1 << 5);
        // Set SIE in mstatus
        core.csr.set(CsrAddr::mstatus, MStatus::SIE.bits());
        // Set stimecmp=0 so mtime >= stimecmp → STIP fires via Sstc
        core.csr.set(CsrAddr::stimecmp, 0);

        // Write a NOP at PC
        write_inst(&mut core, 0x0000_0013); // addi x0, x0, 0 (NOP)

        // Step should deliver the timer interrupt
        core.step().unwrap();

        // After delivery: should have trapped to stvec, scause=SupervisorTimer
        let scause = core.csr.get(CsrAddr::scause);
        let expected = TrapCause::Interrupt(Interrupt::SupervisorTimer).to_mcause();
        assert_eq!(
            scause,
            expected,
            "Timer interrupt not delivered. scause={:#x} expected={:#x} mip={:#x} mie={:#x} \
             mstatus={:#x} priv={:?}",
            scause,
            expected,
            core.csr.get(CsrAddr::mip),
            core.csr.get(CsrAddr::mie),
            core.csr.get(CsrAddr::mstatus),
            core.privilege
        );
    }
}
