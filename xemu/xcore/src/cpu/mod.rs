mod core;

use std::sync::{Arc, LazyLock, Mutex};

use memory_addr::VirtAddr;
use xlogger::ColorCode;

use self::core::CoreOps;
use crate::{
    config::Word,
    device::bus::Bus,
    error::{XError, XResult},
};

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

const RESET_VECTOR: usize = 0x80000000;

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum State {
    IDLE,
    HALTED,
    ABORT,
}

impl State {
    pub fn is_terminated(self) -> bool {
        matches!(self, State::HALTED | State::ABORT)
    }
}

#[allow(clippy::upper_case_acronyms)]
pub struct CPU<Core: CoreOps> {
    core: Core,
    bus: Arc<Mutex<Bus>>,
    state: State,
    halt_pc: VirtAddr,
    halt_ret: Word,
}

impl<Core: CoreOps> CPU<Core> {
    pub fn new(core: Core) -> Self {
        let bus = core.bus().clone();
        Self {
            core,
            bus,
            state: State::IDLE,
            halt_pc: VirtAddr::from(0),
            halt_ret: 0,
        }
    }

    pub fn reset(&mut self) -> XResult {
        self.state = State::IDLE;
        self.core.reset()?;
        let image_bytes: &[u8] = bytemuck::bytes_of(&crate::isa::IMG);
        self.bus.lock().unwrap().load_ram(RESET_VECTOR, image_bytes)
    }

    pub fn load(&mut self, file: Option<String>) -> XResult<&mut Self> {
        match file {
            None => {
                let image_bytes: &[u8] = bytemuck::bytes_of(&crate::isa::IMG);
                self.bus
                    .lock()
                    .unwrap()
                    .load_ram(RESET_VECTOR, image_bytes)?;
            }
            Some(path) => {
                trace!("Loading file: {}", path);
                let bytes = std::fs::read(path).map_err(|_| XError::FailedToRead)?;
                self.bus.lock().unwrap().load_ram(RESET_VECTOR, &bytes)?;
                info!("Loaded {} bytes @ {:#x}", bytes.len(), RESET_VECTOR);
            }
        }
        Ok(self)
    }

    pub fn step(&mut self) -> XResult {
        match self.core.step() {
            Err(XError::ProgramExit(code)) => {
                let state = if code == 0 {
                    State::HALTED
                } else {
                    State::ABORT
                };
                self.set_terminated(state);
                self.halt_ret = code as Word; // override after set_terminated
                self.log_termination();
                Ok(())
            }
            result => {
                result?;
                if self.core.halted() {
                    self.set_terminated(State::HALTED).log_termination();
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

    pub fn is_exit_normal(&self) -> bool {
        self.state == State::HALTED && self.halt_ret == 0
    }

    pub fn log_termination(&self) {
        match self.state {
            State::ABORT => xprintln!(ColorCode::Red, "Error at pc={:#x}", self.halt_pc),
            State::HALTED if self.halt_ret == 0 => {
                xprintln!(ColorCode::Green, "HIT GOOD TRAP at pc={:#x}", self.halt_pc);
            }
            State::HALTED => {
                xprintln!(
                    ColorCode::Red,
                    "HIT BAD TRAP at pc={:#x} (exit code: {})",
                    self.halt_pc,
                    self.halt_ret
                );
            }
            State::IDLE => {}
        }
    }
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
            cpu.set_terminated($crate::State::ABORT).log_termination();
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
        assert!(!State::IDLE.is_terminated());
        assert!(State::HALTED.is_terminated());
        assert!(State::ABORT.is_terminated());
    }

    #[test]
    fn cpu_reset_sets_pc_to_reset_vector() {
        let mut cpu = new_cpu();
        assert_eq!(cpu.core.pc(), VirtAddr::from(RESET_VECTOR));
        assert_eq!(cpu.state, State::IDLE);

        cpu.state = State::HALTED;
        cpu.reset().unwrap();
        assert_eq!(cpu.state, State::IDLE);
        assert_eq!(cpu.core.pc(), VirtAddr::from(RESET_VECTOR));
    }

    #[test]
    fn cpu_load_default_image() {
        let mut cpu = new_cpu();
        cpu.load(None).unwrap();
        let word = cpu.bus.lock().unwrap().read(RESET_VECTOR, 4).unwrap();
        assert_eq!(word as u32, crate::isa::IMG[0]);
    }

    #[test]
    fn cpu_run_skips_if_terminated() {
        let mut cpu = new_cpu();
        cpu.state = State::HALTED;
        cpu.run(100).unwrap();
        assert_eq!(cpu.state, State::HALTED);
    }

    #[test]
    fn cpu_set_terminated_captures_state() {
        let mut cpu = new_cpu();
        cpu.set_terminated(State::HALTED);
        assert_eq!(cpu.state, State::HALTED);
        assert_eq!(cpu.halt_ret, 0);
        assert_eq!(cpu.halt_pc, cpu.core.pc());
    }

    #[test]
    fn cpu_is_exit_normal_only_when_halted_with_zero() {
        let mut cpu = new_cpu();
        cpu.state = State::HALTED;
        cpu.halt_ret = 0;
        assert!(cpu.is_exit_normal());

        cpu.halt_ret = 1;
        assert!(!cpu.is_exit_normal());

        cpu.state = State::ABORT;
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
        assert_eq!(cpu.state, State::HALTED);
    }
}
