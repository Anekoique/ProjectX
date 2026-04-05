//! CPU lifecycle: boot configuration, step/run loop, and termination handling.
//!
//! The generic [`CPU`] wrapper dispatches to an arch-specific core (e.g.
//! [`RVCore`](super::cpu::riscv::RVCore)) via the [`CoreOps`] trait.

mod core;
pub mod debug;

use std::sync::{Mutex, OnceLock};

use inherit_methods_macro::inherit_methods;
use memory_addr::VirtAddr;
use xlogger::ColorCode;

use self::{core::CoreOps, debug::DebugOps};
use crate::{
    config::{BootLayout, Word},
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

cfg_if::cfg_if! {
    if #[cfg(riscv)] {
        mod riscv;
        pub use self::riscv::*;
    } else if #[cfg(loongarch)] {
        mod loongarch;
        pub use self::loongarch::*;
    }
}

/// Global singleton CPU instance, initialized by `init_xcore(config)`.
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

/// Generic CPU wrapper: owns an arch-specific core and manages boot/run
/// lifecycle.
#[allow(clippy::upper_case_acronyms)]
pub struct CPU<Core: CoreOps> {
    core: Core,
    state: State,
    halt_pc: VirtAddr,
    halt_ret: Word,
    boot_config: BootConfig,
    boot_layout: BootLayout,
}

impl<Core: CoreOps + DebugOps> CPU<Core> {
    /// Create a CPU wrapper around an arch-specific core.
    pub fn new(core: Core, layout: BootLayout) -> Self {
        Self {
            core,
            state: State::Idle,
            halt_pc: VirtAddr::from(0),
            halt_ret: 0,
            boot_config: BootConfig::Direct { file: None },
            boot_layout: layout,
        }
    }

    /// Boot from a configuration. Stores the config for subsequent resets.
    pub fn boot(&mut self, config: BootConfig) -> XResult {
        info!("cpu: boot config={:?}", config);
        self.boot_config = config;
        self.reset()
    }

    /// Reset the CPU and reapply the stored boot configuration.
    pub fn reset(&mut self) -> XResult {
        info!("cpu: reset");
        self.state = State::Idle;
        self.core.reset()?;

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
        self.core.setup_boot(core::BootMode::Direct);
        match file {
            None => {
                let image_bytes: &[u8] = bytemuck::bytes_of(&crate::isa::IMG);
                self.core.bus_mut().load_ram(RESET_VECTOR, image_bytes)
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
        self.core.setup_boot(core::BootMode::Firmware { fdt_addr });
        info!("firmware boot: fw={fw}, kernel={kernel:?}, initrd={initrd:?}, fdt={fdt}");
        Ok(())
    }

    fn load_file_at(&mut self, path: &str, addr: usize) -> XResult {
        let bytes = std::fs::read(path).map_err(|_| XError::FailedToRead)?;
        self.core.bus_mut().load_ram(addr, &bytes)?;
        info!("Loaded {} ({} bytes @ {:#x})", path, bytes.len(), addr);
        Ok(())
    }

    /// Legacy load interface (wraps as BootConfig::Direct).
    pub fn load(&mut self, file: Option<String>) -> XResult<&mut Self> {
        self.boot(BootConfig::Direct { file })?;
        Ok(self)
    }

    /// Execute one instruction, handling program exit and halt conditions.
    pub fn step(&mut self) -> XResult {
        match self.core.step() {
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
                Ok(())
            }
            result => {
                result?;
                if self.core.halted() {
                    self.set_terminated(State::Halted).log_termination();
                }
                Ok(())
            }
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

    /// Record termination state and capture PC/return value.
    pub fn set_terminated(&mut self, state: State) -> &mut Self {
        self.state = state;
        self.halt_pc = self.core.pc();
        self.halt_ret = self.core.halt_ret();
        self
    }

    /// Current program counter as a raw address.
    pub fn pc(&self) -> usize {
        self.core.pc().as_usize()
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
        self.core.bus_mut().replace_device(name, dev);
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

    /// Access the core's debug inspection interface.
    pub fn debug_ops(&self) -> &dyn DebugOps {
        &self.core
    }

    /// Consume and return the MMIO-accessed flag (for difftest skip).
    #[cfg(feature = "difftest")]
    pub fn bus_take_mmio_flag(&self) -> bool {
        self.core.bus().take_mmio_flag()
    }
}

/// Delegated debug operations (passed through to the arch-specific core).
#[inherit_methods(from = "self.core")]
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
        let mut cpu = CPU::new(Core::new(), layout);
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
        assert_eq!(cpu.core.pc(), VirtAddr::from(RESET_VECTOR));
        assert_eq!(cpu.state, State::Idle);

        cpu.state = State::Halted;
        cpu.reset().unwrap();
        assert_eq!(cpu.state, State::Idle);
        assert_eq!(cpu.core.pc(), VirtAddr::from(RESET_VECTOR));
    }

    #[test]
    fn cpu_load_default_image() {
        let mut cpu = new_cpu();
        cpu.load(None).unwrap();
        let word = cpu.core.bus_mut().read(RESET_VECTOR, 4).unwrap();
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
        assert_eq!(cpu.halt_pc, cpu.core.pc());
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
        let pc_before = cpu.core.pc();
        cpu.step().unwrap();
        assert_eq!(cpu.core.pc(), pc_before.wrapping_add(4));
    }

    #[test]
    fn cpu_run_executes_default_img_to_completion() {
        let mut cpu = new_cpu();
        cpu.load(None).unwrap();
        cpu.run(100).unwrap();
        assert_eq!(cpu.state, State::Halted);
    }
}
