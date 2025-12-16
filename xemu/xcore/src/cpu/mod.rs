mod core;

use std::sync::{LazyLock, Mutex};

use memory_addr::VirtAddr;

use self::core::CoreOps;
use crate::{config::Word, error::XResult};

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

#[allow(clippy::upper_case_acronyms)]
pub struct CPU<Core: CoreOps> {
    core: Core,
    state: State,
    halt_pc: VirtAddr,
    halt_ret: Word,
}

impl<Core: CoreOps> CPU<Core> {
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

    pub fn load(&mut self, file: String) -> XResult {
        println!("Loading ELF file : {}", file);
        Ok(())
    }

    pub fn step(&mut self) -> XResult {
        self.core
            .fetch()
            .and_then(|instr| self.core.decode(instr))
            .and_then(|decoded| self.core.execute(decoded))
    }

    pub fn run(&mut self, count: u32) -> XResult {
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
        if !self.is_exit_normal() {
            eprintln!(
                "Program {} with error: {} (exit code: {})",
                self.state.message(),
                error_msg,
                self.halt_ret
            );
        } else {
            println!("Program terminated with exit code {}", self.halt_ret);
        }
    }
}

#[macro_export]
macro_rules! with_xcpu {
    ($method:ident($($arg:expr),* $(,)?)) => {{
        $crate::XCPU.lock()
            .expect("Poisoned lock on CPU mutex")
            .$method($($arg),*)
    }};
}

#[macro_export]
macro_rules! terminate {
    ($e:expr) => {{
        match &$e {
            $crate::XError::ToTerminate => {
                $crate::XCPU
                    .lock()
                    .expect("Poisoned lock on CPU mutex")
                    .set_terminated($crate::State::HALTED)
                    .log_termination("No error message provided");
            }
            _ => {
                $crate::XCPU
                    .lock()
                    .expect("Poisoned lock on CPU mutex")
                    .set_terminated($crate::State::ABORT)
                    .log_termination(&$e.to_string());
            }
        }
    }};
    () => {{
        $crate::XCPU
            .lock()
            .expect("Posisoned lock on CPU mutex")
            .set_terminated($crate::State::HALTED)
            .log_termination("No error message provided");
    }};
}
