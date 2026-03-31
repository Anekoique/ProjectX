mod core;
pub mod debug;

use std::sync::{LazyLock, Mutex};

use inherit_methods_macro::inherit_methods;
use memory_addr::VirtAddr;
use xlogger::ColorCode;

use self::{core::CoreOps, debug::DebugOps};
use crate::{
    config::Word,
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
const FDT_LOAD_ADDR: usize = 0x87F0_0000; // near top of 128MB DRAM

cfg_if::cfg_if! {
    if #[cfg(riscv)] {
        mod riscv;
        pub use self::riscv::*;
    } else if #[cfg(loongarch)] {
        mod loongarch;
        pub use self::loongarch::*;
    }
}

pub static XCPU: LazyLock<Mutex<CPU<Core>>> = LazyLock::new(|| Mutex::new(CPU::new(Core::new())));

pub const RESET_VECTOR: usize = 0x80000000;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum State {
    Idle,
    Halted,
    Abort,
}

impl State {
    pub fn is_terminated(self) -> bool {
        matches!(self, State::Halted | State::Abort)
    }
}

// TODO: support multi-core and add concurrent control.
#[allow(clippy::upper_case_acronyms)]
pub struct CPU<Core: CoreOps> {
    core: Core,
    state: State,
    halt_pc: VirtAddr,
    halt_ret: Word,
    boot_config: BootConfig,
}

impl<Core: CoreOps + DebugOps> CPU<Core> {
    pub fn new(core: Core) -> Self {
        Self {
            core,
            state: State::Idle,
            halt_pc: VirtAddr::from(0),
            halt_ret: 0,
            boot_config: BootConfig::Direct { file: None },
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
        self.load_file_at(&fw, RESET_VECTOR)?;
        if let Some(ref k) = kernel {
            self.load_file_at(k, KERNEL_LOAD_ADDR)?;
        }
        if let Some(ref rd) = initrd {
            self.load_file_at(rd, INITRD_LOAD_ADDR)?;
        }
        self.load_file_at(&fdt, FDT_LOAD_ADDR)?;
        self.core.setup_boot(core::BootMode::Firmware {
            fdt_addr: FDT_LOAD_ADDR,
        });
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

    pub fn set_terminated(&mut self, state: State) -> &mut Self {
        self.state = state;
        self.halt_pc = self.core.pc();
        self.halt_ret = self.core.halt_ret();
        self
    }

    pub fn pc(&self) -> usize {
        self.core.pc().as_usize()
    }

    pub fn is_terminated(&self) -> bool {
        self.state.is_terminated()
    }

    pub fn is_exit_normal(&self) -> bool {
        self.state == State::Halted && self.halt_ret == 0
    }

    /// Replace a named MMIO device on the bus (e.g. swap in a PTY-backed UART).
    pub fn replace_device(&mut self, name: &str, dev: Box<dyn crate::device::Device>) {
        self.core.bus_mut().replace_device(name, dev);
    }

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
    pub fn debug_ops(&self) -> &dyn DebugOps {
        &self.core
    }

    #[cfg(feature = "difftest")]
    pub fn bus_take_mmio_flag(&self) -> bool {
        self.core.bus().take_mmio_flag()
    }
}

#[inherit_methods(from = "self.core")]
impl<Core: CoreOps + DebugOps> CPU<Core> {
    pub fn add_breakpoint(&mut self, addr: usize) -> u32;
    pub fn remove_breakpoint(&mut self, id: u32) -> bool;
    pub fn list_breakpoints(&self) -> &[debug::Breakpoint];
    pub fn set_skip_bp(&mut self);
    pub fn context(&self) -> CoreContext;
}

pub fn with_xcpu<R>(f: impl FnOnce(&mut CPU<Core>) -> R) -> R {
    let mut guard = XCPU.lock().expect("Poisoned lock on CPU mutex");
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
        let mut cpu = CPU::new(Core::new());
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
