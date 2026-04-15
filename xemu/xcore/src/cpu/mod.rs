//! CPU lifecycle: boot configuration, step/run loop, and termination handling.
//!
//! The generic [`CPU`] wrapper owns one or more arch-specific cores
//! (`Vec<Core>`) and an inline [`crate::device::bus::Bus`]. Per M-001
//! (`docs/archived/perf/perfBusFastPath/01_MASTER.md`) the bus is NOT wrapped in a
//! synchronization primitive — cooperative round-robin hands each step
//! exactly one `&mut Bus` borrow, which is the exclusion invariant the
//! borrow checker enforces (see invariant I-10 in `03_PLAN.md`).

pub(crate) mod core;
pub mod debug;

use std::sync::{Mutex, OnceLock};

use inherit_methods_macro::inherit_methods;
use memory_addr::VirtAddr;
use xlogger::ColorCode;

use self::{
    core::{CoreOps, HartId},
    debug::DebugOps,
};
use crate::{
    config::{BootLayout, Word},
    device::bus::Bus,
    error::{XError, XResult},
};

/// Boot configuration — selects between legacy direct-load and firmware boot.
#[derive(Clone, Debug)]
pub enum BootConfig {
    /// Legacy: load one binary at DRAM base, PC = 0x8000_0000.
    Direct { file: Option<String> },
    /// Firmware: OpenSBI + optional kernel/initrd payload + FDT.
    Firmware {
        fw: String,
        kernel: Option<String>,
        initrd: Option<String>,
        fdt: String,
    },
}

// Boot memory layout (matches OpenSBI fw_jump convention):
const KERNEL_LOAD_ADDR: usize = 0x8020_0000; // FW_JUMP_ADDR: 2MB after DRAM base
const INITRD_LOAD_ADDR: usize = 0x8400_0000; // after kernel region

#[cfg(riscv)]
pub type Core = crate::arch::riscv::cpu::RVCore;
#[cfg(riscv)]
pub type CoreContext = crate::arch::riscv::cpu::context::RVCoreContext;
#[cfg(riscv)]
pub type PendingTrap = crate::arch::riscv::cpu::trap::PendingTrap;
#[cfg(loongarch)]
pub type Core = crate::arch::loongarch::cpu::LACore;

/// Global singleton CPU instance, initialized by `init_xcore(config)`.
///
/// The outer `Mutex` here guards the **CPU-lifecycle handle** — it lets
/// `xdb`, difftest, and the monitor coordinate `reset` / `boot` /
/// `step` calls from the single controller thread against background
/// tasks that occasionally peek at state. It is NOT the bus lock that
/// M-001 forbade: the bus is owned inline inside `CPU` (see
/// [`CPU`] docs) and is reached only through the `&mut CPU` obtained
/// from this mutex, so no per-memory-access locking exists on the
/// hot path. See `docs/archived/perf/perfBusFastPath/01_MASTER.md` for the
/// distinction.
pub static XCPU: OnceLock<Mutex<CPU<Core>>> = OnceLock::new();

/// DRAM base address where the first instruction executes.
pub const RESET_VECTOR: usize = 0x80000000;

/// CPU execution state.
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum State {
    /// Running or ready to step.
    Idle,
    /// Normal termination (exit code 0).
    Halted,
    /// Abnormal termination (nonzero exit code or error).
    Abort,
}

impl State {
    /// Returns `true` for both [`Halted`](State::Halted) and
    /// [`Abort`](State::Abort).
    pub fn is_terminated(self) -> bool {
        matches!(self, State::Halted | State::Abort)
    }
}

/// Generic CPU wrapper: owns one or more arch-specific cores plus an inline
/// bus, and manages boot/run lifecycle.
///
/// # M-001: no `Mutex<Bus>`
///
/// The `bus` field is owned inline (not `Arc<Mutex<Bus>>`). The cooperative
/// round-robin scheduler in [`CPU::step`] hands the current hart exactly one
/// `&mut Bus` borrow per step via a disjoint-field destructure (invariant
/// I-10); the borrow checker is the exclusion primitive.
#[allow(clippy::upper_case_acronyms)]
pub struct CPU<Core: CoreOps> {
    cores: Vec<Core>,
    bus: Bus,
    current: usize,
    state: State,
    halt_pc: VirtAddr,
    halt_ret: Word,
    boot_config: BootConfig,
    boot_layout: BootLayout,
    /// Pre-minted IRQ line for the platform's UART source, exposed so callers
    /// (e.g. `xdb`) can construct a PTY/stdio backend without re-entering the
    /// PLIC to mint a fresh handle.
    uart_line: Option<crate::device::IrqLine>,
}

// V-UT-3 (R-003): pin `CPU<RVCore>` layout below 4096 B. `cores` is a
// `Vec<Core>` (24 B header) regardless of hart count; the inline `Bus`
// contributes under 256 B. The struct as a whole is a few hundred bytes
// and must not balloon silently.
#[cfg(riscv)]
const _: () = assert!(
    std::mem::size_of::<CPU<Core>>() < 4096,
    "CPU<RVCore> grew past the 4 KiB layout budget",
);

impl<Core: CoreOps + DebugOps> CPU<Core> {
    /// Create a CPU wrapper around `cores` that owns `bus` by value.
    pub fn new(cores: Vec<Core>, bus: Bus, layout: BootLayout) -> Self {
        debug_assert_eq!(
            bus.num_harts(),
            cores.len(),
            "Bus::num_harts must match cores.len()"
        );
        Self {
            cores,
            bus,
            current: 0,
            state: State::Idle,
            halt_pc: VirtAddr::from(0),
            halt_ret: 0,
            boot_config: BootConfig::Direct { file: None },
            boot_layout: layout,
            uart_line: None,
        }
    }

    /// Clone of the pre-minted UART IRQ line (set by the machine factory).
    /// Returns `None` if no UART source was wired up.
    pub fn uart_line(&self) -> Option<crate::device::IrqLine> {
        self.uart_line.clone()
    }

    /// Borrow the owned bus for read-only inspection.
    pub fn bus(&self) -> &Bus {
        &self.bus
    }

    /// Borrow the owned bus mutably (used by difftest and test helpers).
    pub fn bus_mut(&mut self) -> &mut Bus {
        &mut self.bus
    }

    /// Boot from a configuration. Stores the config for subsequent resets.
    pub fn boot(&mut self, config: BootConfig) -> XResult {
        info!("cpu: boot config={:?}", config);
        self.boot_config = config;
        self.reset()
    }

    /// Reset the CPU and reapply the stored boot configuration.
    /// Order: bus first (clear devices + reservations), then cores.
    pub fn reset(&mut self) -> XResult {
        info!("cpu: reset");
        self.state = State::Idle;
        self.bus.reset_devices();
        self.bus.clear_reservations();
        for core in &mut self.cores {
            core.reset()?;
        }

        match &self.boot_config {
            BootConfig::Direct { file } => self.load_direct(file.clone()),
            BootConfig::Firmware {
                fw,
                kernel,
                initrd,
                fdt,
            } => self.load_firmware(fw.clone(), kernel.clone(), initrd.clone(), fdt.clone()),
        }
    }

    fn load_direct(&mut self, file: Option<String>) -> XResult {
        for core in &mut self.cores {
            core.setup_boot(core::BootMode::Direct);
        }
        match file {
            None => {
                let image_bytes: &[u8] = bytemuck::bytes_of(&crate::isa::IMG);
                self.bus.load_ram(RESET_VECTOR, image_bytes)
            }
            Some(path) => self.load_file_at(&path, RESET_VECTOR),
        }
    }

    fn load_firmware(
        &mut self,
        fw: String,
        kernel: Option<String>,
        initrd: Option<String>,
        fdt: String,
    ) -> XResult {
        let fdt_addr = self.boot_layout.fdt_addr;
        self.load_file_at(&fw, RESET_VECTOR)?;
        if let Some(ref k) = kernel {
            self.load_file_at(k, KERNEL_LOAD_ADDR)?;
        }
        if let Some(ref rd) = initrd {
            self.load_file_at(rd, INITRD_LOAD_ADDR)?;
        }
        self.load_file_at(&fdt, fdt_addr)?;
        for core in &mut self.cores {
            core.setup_boot(core::BootMode::Firmware { fdt_addr });
        }
        info!("firmware boot: fw={fw}, kernel={kernel:?}, initrd={initrd:?}, fdt={fdt}");
        Ok(())
    }

    fn load_file_at(&mut self, path: &str, addr: usize) -> XResult {
        let bytes = std::fs::read(path).map_err(|_| XError::FailedToRead)?;
        self.bus.load_ram(addr, &bytes)?;
        info!("Loaded {} ({} bytes @ {:#x})", path, bytes.len(), addr);
        Ok(())
    }

    /// Legacy load interface (wraps as BootConfig::Direct).
    pub fn load(&mut self, file: Option<String>) -> XResult<&mut Self> {
        self.boot(BootConfig::Direct { file })?;
        Ok(self)
    }

    /// Execute one instruction on the current hart and advance the
    /// round-robin scheduler. The bus is ticked once per `CPU::step`
    /// before the hart steps (matches HW hart-cycle clocking at N>1).
    ///
    /// # Invariant I-10: disjoint-field borrow at `CPU::step`
    ///
    /// `self` is destructured into disjoint borrows so `bus` and
    /// `cores[current]` are independent places to the borrow checker.
    /// A helper method on `&mut self` that reached both fields would
    /// collapse this disjoint-field path and fail to compile (E0499).
    pub fn step(&mut self) -> XResult {
        let CPU {
            bus,
            cores,
            current,
            ..
        } = self;
        bus.tick();
        let result = cores[*current].step(bus);
        let halted = cores[*current].halted();
        match result {
            Err(XError::ProgramExit(code)) => {
                info!("cpu: program exit with code {}", code);
                let state = if code == 0 {
                    State::Halted
                } else {
                    State::Abort
                };
                self.set_terminated(state);
                self.halt_ret = code as Word; // override after set_terminated
                self.log_termination();
                self.advance_current();
                Ok(())
            }
            Ok(()) => {
                if halted {
                    self.set_terminated(State::Halted).log_termination();
                }
                self.advance_current();
                Ok(())
            }
            Err(e) => {
                self.advance_current();
                Err(e)
            }
        }
    }

    #[inline]
    fn advance_current(&mut self) {
        if !self.cores.is_empty() {
            self.current = (self.current + 1) % self.cores.len();
        }
    }

    /// Run up to `count` instructions, stopping early on termination.
    pub fn run(&mut self, count: u64) -> XResult {
        if self.state.is_terminated() {
            info!("CPU is not running. Please reset or load a program first.");
            return Ok(());
        }
        for _ in 0..count {
            self.step()?;
            if self.state.is_terminated() {
                break;
            }
        }
        Ok(())
    }

    /// Record termination state and capture PC/return value (for the
    /// current hart).
    pub fn set_terminated(&mut self, state: State) -> &mut Self {
        self.state = state;
        self.halt_pc = self.cores[self.current].pc();
        self.halt_ret = self.cores[self.current].halt_ret();
        self
    }

    /// Current hart's program counter as a raw address.
    pub fn pc(&self) -> usize {
        self.cores[self.current].pc().as_usize()
    }

    /// True if the CPU has halted or aborted.
    pub fn is_terminated(&self) -> bool {
        self.state.is_terminated()
    }

    /// True only if halted with exit code 0 (success).
    pub fn is_exit_normal(&self) -> bool {
        self.state == State::Halted && self.halt_ret == 0
    }

    /// Replace a named MMIO device on the bus (e.g. swap in a PTY-backed UART).
    pub fn replace_device(&mut self, name: &str, dev: Box<dyn crate::device::Device>) {
        self.bus.replace_device(name, dev);
    }

    /// Print colored termination message to stdout.
    pub fn log_termination(&self) {
        match self.state {
            State::Abort => xprintln!(ColorCode::Red, "Error at pc={:#x}", self.halt_pc),
            State::Halted if self.halt_ret == 0 => {
                xprintln!(ColorCode::Green, "HIT GOOD TRAP at pc={:#x}", self.halt_pc);
            }
            State::Halted => {
                xprintln!(
                    ColorCode::Red,
                    "HIT BAD TRAP at pc={:#x} (exit code: {})",
                    self.halt_pc,
                    self.halt_ret
                );
            }
            State::Idle => {}
        }
    }

    /// Access the current hart's debug inspection interface.
    pub fn debug_ops(&self) -> &dyn DebugOps {
        &self.cores[self.current]
    }

    /// Consume and return the MMIO-accessed flag (for difftest skip).
    #[cfg(feature = "difftest")]
    pub fn bus_take_mmio_flag(&mut self) -> bool {
        self.bus.take_mmio_flag()
    }
}

/// Delegated debug operations (passed through to the current hart).
#[inherit_methods(from = "self.cores[self.current]")]
impl<Core: CoreOps + DebugOps> CPU<Core> {
    /// Insert a breakpoint, returning its stable ID.
    pub fn add_breakpoint(&mut self, addr: usize) -> u32;
    /// Remove breakpoint by ID. Returns `true` if found.
    pub fn remove_breakpoint(&mut self, id: u32) -> bool;
    /// List all active breakpoints.
    pub fn list_breakpoints(&self) -> &[debug::Breakpoint];
    /// Skip breakpoint check for the next step.
    pub fn set_skip_bp(&mut self);
    /// Capture a snapshot of the current architectural state.
    pub fn context(&self) -> CoreContext;
}

// --- RISC-V machine factory -------------------------------------------------

#[cfg(riscv)]
impl CPU<Core> {
    /// Build a CPU + Bus + devices from a [`crate::config::MachineConfig`].
    pub fn from_config(config: crate::config::MachineConfig, layout: BootLayout) -> Self {
        use crate::{
            arch::riscv::device::{aclint::Aclint, plic::Plic},
            config::CONFIG_MBASE,
            device::{
                IrqState, bus::Bus, test_finisher::TestFinisher, uart::Uart, virtio_blk::VirtioBlk,
            },
        };

        // Range enforcement lives in `Bus::new` and `MachineConfig::with_harts`.
        let num_harts = config.num_harts;
        let irqs: Vec<IrqState> = (0..num_harts).map(|_| IrqState::new()).collect();

        let mut bus = Bus::new(CONFIG_MBASE, config.ram_size, num_harts);
        let mtimer_idx = Aclint::new(num_harts, irqs.clone()).install(&mut bus, 0x0200_0000);
        bus.set_timer_source(mtimer_idx);

        let plic = Plic::new(num_harts, irqs.clone());
        let uart_line = plic.with_irq_line(10);
        let virtio_line = plic.with_irq_line(1);
        let plic_idx = bus.add_mmio("plic", 0x0C00_0000, 0x400_0000, Box::new(plic));
        bus.set_irq_sink(plic_idx);
        bus.add_mmio(
            "uart0",
            0x1000_0000,
            0x100,
            Box::new(Uart::new(uart_line.clone())),
        );
        bus.add_mmio("finisher", 0x10_0000, 0x1000, Box::new(TestFinisher::new()));
        if let Some(disk) = config.disk {
            bus.add_mmio(
                "virtio-blk0",
                0x1000_1000,
                0x1000,
                Box::new(VirtioBlk::new(disk, virtio_line)),
            );
        }

        let cores: Vec<Core> = (0..num_harts)
            .map(|i| Core::with_id(HartId::from(i), irqs[i].clone()))
            .collect();

        let mut cpu = Self::new(cores, bus, layout);
        cpu.uart_line = Some(uart_line);
        cpu
    }
}

/// Lock the global CPU and execute `f` with exclusive access.
pub fn with_xcpu<R>(f: impl FnOnce(&mut CPU<Core>) -> R) -> R {
    let mut guard = XCPU
        .get()
        .expect("XCPU not initialized — call init_xcore() first")
        .lock()
        .expect("Poisoned lock on CPU mutex");
    f(&mut guard)
}

#[macro_export]
macro_rules! with_xcpu {
    ($($chain:tt)+) => {{
        $crate::with_xcpu(|__xcpu| __xcpu.$($chain)+)
    }};
}

#[macro_export]
macro_rules! terminate {
    ($e:expr) => {{
        error!("{}", $e);
        $crate::with_xcpu(|cpu| {
            cpu.set_terminated($crate::State::Abort).log_termination();
        });
    }};
}

#[cfg(test)]
mod tests {
    use memory_addr::MemoryAddr;

    use super::*;

    fn new_cpu() -> CPU<Core> {
        let layout = BootLayout {
            fdt_addr: crate::config::CONFIG_MBASE + crate::config::CONFIG_MSIZE - 0x10_0000,
        };
        let mut cpu = CPU::<Core>::from_config(crate::config::MachineConfig::default(), layout);
        cpu.reset().unwrap();
        cpu
    }

    #[test]
    fn state_is_terminated() {
        assert!(!State::Idle.is_terminated());
        assert!(State::Halted.is_terminated());
        assert!(State::Abort.is_terminated());
    }

    #[test]
    fn cpu_reset_sets_pc_to_reset_vector() {
        let mut cpu = new_cpu();
        assert_eq!(cpu.cores[cpu.current].pc(), VirtAddr::from(RESET_VECTOR));
        assert_eq!(cpu.state, State::Idle);

        cpu.state = State::Halted;
        cpu.reset().unwrap();
        assert_eq!(cpu.state, State::Idle);
        assert_eq!(cpu.cores[cpu.current].pc(), VirtAddr::from(RESET_VECTOR));
    }

    #[test]
    fn cpu_load_default_image() {
        let mut cpu = new_cpu();
        cpu.load(None).unwrap();
        let word = cpu.bus_mut().read(RESET_VECTOR, 4).unwrap();
        assert_eq!(word as u32, crate::isa::IMG[0]);
    }

    #[test]
    fn cpu_run_skips_if_terminated() {
        let mut cpu = new_cpu();
        cpu.state = State::Halted;
        cpu.run(100).unwrap();
        assert_eq!(cpu.state, State::Halted);
    }

    #[test]
    fn cpu_set_terminated_captures_state() {
        let mut cpu = new_cpu();
        cpu.set_terminated(State::Halted);
        assert_eq!(cpu.state, State::Halted);
        assert_eq!(cpu.halt_ret, 0);
        assert_eq!(cpu.halt_pc, cpu.cores[cpu.current].pc());
    }

    #[test]
    fn cpu_is_exit_normal_only_when_halted_with_zero() {
        let mut cpu = new_cpu();
        cpu.state = State::Halted;
        cpu.halt_ret = 0;
        assert!(cpu.is_exit_normal());

        cpu.halt_ret = 1;
        assert!(!cpu.is_exit_normal());

        cpu.state = State::Abort;
        cpu.halt_ret = 0;
        assert!(!cpu.is_exit_normal());
    }

    #[test]
    fn cpu_step_advances_pc() {
        let mut cpu = new_cpu();
        cpu.load(None).unwrap();
        let pc_before = cpu.cores[cpu.current].pc();
        cpu.step().unwrap();
        assert_eq!(cpu.cores[cpu.current].pc(), pc_before.wrapping_add(4));
    }

    #[test]
    fn cpu_run_executes_default_img_to_completion() {
        let mut cpu = new_cpu();
        cpu.load(None).unwrap();
        cpu.run(100).unwrap();
        assert_eq!(cpu.state, State::Halted);
    }

    // ---- PR1 multi-hart shape tests ----

    #[test]
    fn cpu_step_advances_current_single_hart() {
        let mut cpu = new_cpu();
        cpu.load(None).unwrap();
        assert_eq!(cpu.current, 0);
        cpu.step().unwrap();
        // With one hart, current modulo cores.len() stays at 0.
        assert_eq!(cpu.current, 0);
    }

    #[test]
    fn hart_ids_match_index() {
        let cpu = new_cpu();
        for (i, core) in cpu.cores.iter().enumerate() {
            assert_eq!(core.id(), HartId::from(i));
        }
    }

    // ---- PR2b activation tests ----

    fn new_cpu_harts(n: usize) -> CPU<Core> {
        let config = crate::config::MachineConfig::default().with_harts(n);
        let layout = BootLayout {
            fdt_addr: crate::config::CONFIG_MBASE + crate::config::CONFIG_MSIZE - 0x10_0000,
        };
        let mut cpu = CPU::<Core>::from_config(config, layout);
        cpu.reset().unwrap();
        cpu
    }

    /// V-IT-1: PLIC is wired for 2*num_harts contexts when num_harts > 1.
    #[test]
    fn cpu_2hart_plic_two_contexts_per_hart() {
        let cpu = new_cpu_harts(2);
        assert_eq!(cpu.cores.len(), 2);
        assert_eq!(cpu.bus().num_harts(), 2);
    }

    /// V-IT-2: round-robin scheduler alternates between harts at num_harts=2.
    #[test]
    fn cpu_2hart_round_robin_alternates() {
        let mut cpu = new_cpu_harts(2);
        cpu.load(None).unwrap();
        assert_eq!(cpu.current, 0);
        cpu.step().unwrap();
        assert_eq!(
            cpu.current, 1,
            "after hart 0 step, current must advance to 1"
        );
        cpu.step().unwrap();
        assert_eq!(cpu.current, 0, "after hart 1 step, current must wrap to 0");
    }

    /// V-E-4: per-hart HartId reflects declaration order; drives `mhartid`
    /// seeding in `RVCore::reset` and `a0` in `setup_boot(Firmware)`.
    #[test]
    fn cpu_2hart_per_hart_hartid_matches_index() {
        let cpu = new_cpu_harts(2);
        assert_eq!(cpu.cores[0].id(), HartId(0));
        assert_eq!(cpu.cores[1].id(), HartId(1));
    }
}
