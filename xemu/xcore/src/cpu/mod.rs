mod core;
mod mem;

use std::sync::{LazyLock, Mutex};

use memory_addr::{PhysAddr, VirtAddr};
use xlogger::ColorCode;

use self::{core::CoreOps, mem::MemOps};
use crate::{
    config::Word,
    error::{XError, XResult},
    memory::with_mem,
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

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum State {
    RUNNING,
    STOP,
    ABORT,
    HALTED,
}

impl State {
    pub fn is_terminated(self) -> bool {
        matches!(self, State::HALTED | State::ABORT)
    }

    pub fn message(self) -> &'static str {
        match self {
            State::HALTED => "Halted",
            State::ABORT => "Aborted",
            State::RUNNING => "Running",
            State::STOP => "Stopped",
        }
    }
}

const RESET_VECTOR: usize = 0x80000000;

#[allow(clippy::upper_case_acronyms)]
pub struct CPU<Core: CoreOps + MemOps> {
    core: Core,
    state: State,
    halt_pc: VirtAddr,
    halt_ret: Word,
}

impl<Core: CoreOps + MemOps> CPU<Core> {
    pub fn new(core: Core) -> Self {
        Self {
            core,
            state: State::STOP,
            halt_pc: VirtAddr::from(0),
            halt_ret: 0,
        }
    }

    pub fn reset(&mut self) -> XResult {
        self.state = State::STOP;
        self.core.reset()
    }

    pub fn load(&mut self, file: Option<String>) -> XResult<&mut Self> {
        let addr = PhysAddr::from(RESET_VECTOR);
        file.map_or_else(
            || self.core.init_memory(addr),
            |path| {
                trace!("Loading file: {}", path);
                std::fs::read(path)
                    .map_err(|_| XError::FailedToRead)
                    .and_then(|bytes| {
                        with_mem!(load(addr, &bytes))?;
                        info!("Loaded {} bytes @ {:#x}", bytes.len(), addr);
                        Ok(())
                    })
            },
        )?;

        Ok(self)
    }

    pub fn step(&mut self) -> XResult {
        self.core
            .fetch()
            .and_then(|instr| self.core.decode(instr))
            .and_then(|decoded| self.core.execute(decoded))
    }

    pub fn run(&mut self, count: u64) -> XResult {
        if self.state.is_terminated() {
            info!("CPU is not running. Please reset or load a program first.");
            return Ok(());
        }

        self.state = State::RUNNING;
        for _ in 0..count {
            self.step()?;
        }
        Ok(())
    }

    pub fn terminate(&mut self, state: State, error_msg: &str) {
        self.set_terminated(state).log_termination(error_msg);
    }

    pub fn set_terminated(&mut self, state: State) -> &Self {
        self.state = state;
        self.halt_pc = self.core.pc();
        self.halt_ret = self.core.halt_ret();
        self
    }

    pub fn is_exit_normal(&self) -> bool {
        self.state == State::HALTED && self.halt_ret == 0
    }

    pub fn log_termination(&self, error_msg: &str) {
        let (color, msg) = if self.is_exit_normal() {
            (
                ColorCode::Green,
                format!("Program terminated with exit code {}", self.halt_ret),
            )
        } else {
            (
                ColorCode::Red,
                format!(
                    "Program {} with error: {} (exit code: {})",
                    self.state.message(),
                    error_msg,
                    self.halt_ret
                ),
            )
        };
        xprintln!(color, "{}", msg);
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
        $crate::with_xcpu(|cpu| match &$e {
            $crate::XError::ToTerminate => {
                cpu.set_terminated($crate::State::HALTED)
                    .log_termination("No error message provided");
            }
            err => {
                cpu.set_terminated($crate::State::ABORT)
                    .log_termination(&err.to_string());
            }
        });
    }};
    () => {{
        $crate::with_xcpu(|cpu| {
            cpu.set_terminated($crate::State::HALTED)
                .log_termination("No error message provided");
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
        assert!(!State::RUNNING.is_terminated());
        assert!(!State::STOP.is_terminated());
        assert!(State::HALTED.is_terminated());
        assert!(State::ABORT.is_terminated());
    }

    #[test]
    fn cpu_reset_sets_pc_to_reset_vector() {
        let mut cpu = new_cpu();
        assert_eq!(cpu.core.pc, VirtAddr::from(RESET_VECTOR));
        assert_eq!(cpu.state, State::STOP);

        // Reset again to verify idempotency
        cpu.state = State::RUNNING;
        cpu.reset().unwrap();
        assert_eq!(cpu.state, State::STOP);
        assert_eq!(cpu.core.pc, VirtAddr::from(RESET_VECTOR));
    }

    #[test]
    fn cpu_load_default_image() {
        let mut cpu = new_cpu();
        cpu.load(None).unwrap();
        // After loading default IMG, memory at RESET_VECTOR should have the first
        // instruction
        let word = with_mem!(read(PhysAddr::from(RESET_VECTOR), 4)).unwrap();
        assert_eq!(word as u32, crate::isa::IMG[0]);
    }

    #[test]
    fn cpu_run_skips_if_terminated() {
        let mut cpu = new_cpu();
        cpu.state = State::HALTED;
        // Should not error, just skip
        cpu.run(100).unwrap();
        assert_eq!(cpu.state, State::HALTED);
    }

    #[test]
    fn cpu_set_terminated_captures_state() {
        let mut cpu = new_cpu();
        // halt_ret reads from a0, which is 0 after reset
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
        cpu.state = State::RUNNING;

        // The first IMG instruction is `auipc t0, 0` (0x00000297), a 32-bit inst
        let pc_before = cpu.core.pc();
        cpu.step().unwrap();
        // PC should advance by 4
        assert_eq!(cpu.core.pc(), pc_before.wrapping_add(4));
    }

    #[test]
    fn cpu_run_executes_default_img_to_completion() {
        let mut cpu = new_cpu();
        cpu.load(None).unwrap();
        // The default IMG ends with ebreak which returns ToTerminate
        let result = cpu.run(100);
        assert!(matches!(result, Err(XError::ToTerminate)));
    }
}
