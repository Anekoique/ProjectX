use memory_addr::VirtAddr;

use crate::{config::Word, error::XResult, memory::Memory};

pub trait CoreOps {
    fn reset(&mut self, memory: &mut Memory) -> VirtAddr;
    fn fetch(&self, mem: &Memory, addr: VirtAddr) -> XResult<Word>;
    fn decode(&self, instr: Word) -> XResult<String>;
    fn execute(&mut self, pc: &mut VirtAddr, instr: String) -> XResult<()>;
}

pub struct Core<C: CoreOps> {
    pc: VirtAddr,
    core: C,
}

impl<C: CoreOps> Core<C> {
    pub fn new(core: C) -> Self {
        Self {
            pc: VirtAddr::from(0),
            core,
        }
    }

    pub fn reset(&mut self, memory: &mut Memory) {
        self.pc = self.core.reset(memory);
    }

    pub fn step(&mut self, memory: &mut Memory) -> XResult {
        let instr = self.core.fetch(memory, self.pc)?;
        let decoded = self.core.decode(instr)?;
        self.core.execute(&mut self.pc, decoded)?;
        Ok(())
    }

    pub fn execute(&mut self, memory: &mut Memory, count: u32) -> XResult {
        for _ in 0..count {
            self.step(memory)?;
        }
        Ok(())
    }
}
