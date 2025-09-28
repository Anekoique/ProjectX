crate::import_modules!(riscv, loongarch);
mod core;

use memory_addr::VirtAddr;

use self::core::{Core, CoreOps};
use crate::{error::XResult, memory::Memory};

crate::define_cpu!(
    riscv => self::riscv::RVCore,
    loongarch => self::loongarch::LACore
);

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum State {
    RUNNING,
    STOP,
    ABORT,
    HALTED,
}

impl State {
    fn is_terminal(self) -> bool {
        matches!(self, State::HALTED | State::ABORT)
    }

    fn message(self) -> &'static str {
        match self {
            State::HALTED => "Halted",
            State::ABORT => "Aborted",
            State::RUNNING => "Running",
            State::STOP => "Stopped",
        }
    }
}

#[allow(clippy::upper_case_acronyms)]
pub struct CPU<C: CoreOps> {
    core: Core<C>,
    memory: Memory,

    state: State,
    halt_pc: VirtAddr,
    halt_ret: u32,
}

impl<C: CoreOps> CPU<C> {
    pub fn new(inner: C) -> Self {
        Self {
            core: Core::new(inner),
            memory: Memory::new(),
            state: State::STOP,
            halt_pc: VirtAddr::from(0),
            halt_ret: 0,
        }
    }

    pub fn reset(&mut self) {
        self.core.reset(&mut self.memory);
    }

    pub fn load(&mut self, file: String) -> XResult {
        println!("Loading ELF file : {}", file);
        Ok(())
    }

    pub fn run(&mut self, count: u32) -> XResult {
        if self.state.is_terminal() {
            println!("CPU is not running. Please reset or load a program first.");
            return Ok(());
        }

        self.state = State::RUNNING;
        self.core.execute(&mut self.memory, count)?;

        match self.state {
            State::RUNNING => self.state = State::STOP,
            _ => {
                info!(
                    "XEMU: {} at pc = {:#x}, return code = {}",
                    self.state.message(),
                    self.halt_pc.as_usize(),
                    self.halt_ret
                );
            }
        }
        Ok(())
    }

    pub fn is_exit_status_bad(&self) -> bool {
        self.state == State::HALTED && self.halt_ret != 0
    }
}
