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

use memory_addr::{MemoryAddr, VirtAddr};

pub use self::{RVCore as Core, context::RVCoreContext as CoreContext, trap::PendingTrap};
use self::{
    csr::{CsrAddr, CsrFile, MStatus, PrivilegeMode},
    mm::{Mmu, Pmp},
    trap::TrapCause,
};
use super::{CoreOps, RESET_VECTOR};
use crate::{
    config::{CONFIG_MBASE, MachineConfig, Word},
    device::{
        HW_IP_MASK, IrqState, SSIP, STIP,
        bus::Bus,
        intc::{aclint::Aclint, plic::Plic},
        test_finisher::TestFinisher,
        uart::Uart,
        virtio_blk::VirtioBlk,
    },
    error::{XError, XResult},
    isa::{DECODER, DecodedInst, RVReg},
};

/// RISC-V CPU core: registers, CSR file, MMU, PMP, bus, and trap state.
pub struct RVCore {
    gpr: [Word; 32],
    fpr: [u64; 32],
    pc: VirtAddr,
    npc: VirtAddr,
    pub(crate) csr: CsrFile,
    pub(crate) privilege: PrivilegeMode,
    pub(crate) pending_trap: Option<PendingTrap>,
    pub(crate) reservation: Option<usize>,
    pub(crate) bus: Bus,
    pub(crate) mmu: Mmu,
    pub(crate) pmp: Pmp,
    irq: IrqState,
    halted: bool,
    /// When true, ebreak traps to handler instead of halting (firmware mode).
    pub(crate) ebreak_as_trap: bool,
    breakpoints: Vec<crate::cpu::debug::Breakpoint>,
    next_bp_id: u32,
    skip_bp_once: bool,
}

impl RVCore {
    /// Create a new RVCore with default machine profile (128 MB, no disk).
    pub fn new() -> Self {
        Self::with_config(MachineConfig::default())
    }

    /// Create an RVCore from a machine configuration.
    pub fn with_config(config: MachineConfig) -> Self {
        let irq = IrqState::new();
        let mut bus = Bus::new(CONFIG_MBASE, config.ram_size);
        let aclint_idx = bus.mmio.len();
        bus.add_mmio(
            "aclint",
            0x0200_0000,
            0x1_0000,
            Box::new(Aclint::new(irq.clone(), bus.ssip_flag())),
            0,
        );
        bus.set_timer_source(aclint_idx);
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
        bus.add_mmio(
            "finisher",
            0x10_0000,
            0x1000,
            Box::new(TestFinisher::new()),
            0,
        );
        if let Some(disk) = config.disk {
            bus.add_mmio(
                "virtio-blk0",
                0x1000_1000,
                0x1000,
                Box::new(VirtioBlk::new(disk)),
                1,
            );
        }
        Self::with_bus(bus, irq)
    }

    /// Create an RVCore with an externally constructed bus and IRQ state.
    pub fn with_bus(bus: Bus, irq: IrqState) -> Self {
        Self {
            gpr: [0; 32],
            fpr: [0; 32],
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
            ebreak_as_trap: false,
            breakpoints: Vec::new(),
            next_bp_id: 1,
            skip_bp_once: false,
        }
    }

    /// Merge hardware interrupt bits from devices into mip.
    /// SSIP handled via ACLINT SSWI; STIP driven by Sstc stimecmp.
    #[allow(clippy::unnecessary_cast)]
    fn sync_interrupts(&mut self) {
        let hw = self.irq.load() as Word;
        let stip: Word =
            if self.csr.get(CsrAddr::time) as u64 >= self.csr.get(CsrAddr::stimecmp) as u64 {
                STIP as Word
            } else {
                0
            };
        let mip =
            (self.csr.get(CsrAddr::mip) & !HW_IP_MASK & !(STIP as Word)) | (hw & HW_IP_MASK) | stip;
        self.csr.set(CsrAddr::mip, mip);
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
    fn pc(&self) -> VirtAddr {
        self.pc
    }

    fn bus(&self) -> &Bus {
        &self.bus
    }

    fn bus_mut(&mut self) -> &mut Bus {
        &mut self.bus
    }

    fn setup_boot(&mut self, mode: super::core::BootMode) {
        use super::core::BootMode;
        match mode {
            BootMode::Direct => {
                self.ebreak_as_trap = false;
            }
            BootMode::Firmware { fdt_addr } => {
                // SBI convention: a0 = hartid, a1 = FDT pointer
                self.gpr[RVReg::a0] = 0;
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
        self.privilege = PrivilegeMode::Machine;
        self.pending_trap = None;
        self.reservation = None;
        self.mmu = Mmu::new();
        self.pmp = Pmp::new();
        self.irq.reset();
        self.bus.reset_devices();
        self.halted = false;
        Ok(())
    }

    #[allow(clippy::unnecessary_cast)]
    fn step(&mut self) -> XResult {
        self.bus.tick();
        self.csr.set(CsrAddr::time, self.bus.mtime() as Word);
        if self.bus.take_ssip() {
            let mip = self.csr.get(CsrAddr::mip);
            self.csr.set(CsrAddr::mip, mip | SSIP as Word);
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
    use crate::cpu::riscv::{csr::CsrAddr, trap::Exception};

    fn setup_core() -> RVCore {
        let mut core = RVCore::new();
        core.pc = VirtAddr::from(CONFIG_MBASE);
        core.npc = core.pc;
        core.csr.set(CsrAddr::mtvec, 0x8000_0000);
        core
    }

    fn write_inst(core: &mut RVCore, inst: u32) {
        core.bus.write(core.pc.as_usize(), 4, inst as Word).unwrap();
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
        use crate::device::SSIP;
        let mut core = setup_core();
        write_inst(&mut core, 0x0000_0297); // auipc t0, 0 (harmless NOP-like)

        // Write SETSSIP=1 via ACLINT MMIO (base=0x0200_0000, offset=0xC000)
        core.bus.write(0x0200_C000, 4, 1).unwrap();

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
        write_inst(&mut core, 0x0000_0297);

        // Step again: SSIP should NOT be reasserted (no new SETSSIP write)
        core.step().unwrap();
        assert_eq!(
            core.csr.get(CsrAddr::mip) as u64 & SSIP,
            0,
            "SSIP must not be reasserted without new SETSSIP"
        );
    }

    #[test]
    fn stip_delivered_in_s_mode_with_sie() {
        use crate::cpu::riscv::{csr::MStatus, trap::Interrupt};

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
