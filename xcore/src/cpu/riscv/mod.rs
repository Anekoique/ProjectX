mod mem;

use memory_addr::VirtAddr;

use super::CoreOps;
use crate::{
    config::{WSIZE, Word, XLEN},
    error::{XError, XResult},
    isa::DECODER,
    memory::Memory,
};

const RESET_VECTOR: usize = 0x80000000;

pub struct RVCore {
    gpr: [Word; 32],
}

impl RVCore {
    pub fn new() -> Self {
        Self { gpr: [0; 32] }
    }
}

impl Default for RVCore {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreOps for RVCore {
    fn reset(&mut self, memory: &mut Memory) -> VirtAddr {
        self.gpr.fill(0);
        self.init_memory(memory);
        VirtAddr::from(RESET_VECTOR)
    }

    fn fetch(&self, mem: &Memory, addr: VirtAddr) -> XResult<Word> {
        mem.read(self.virt_to_phys(addr), WSIZE)
    }

    fn decode(&self, instr: Word) -> XResult<String> {
        DECODER
            .decode_from_word(instr, XLEN)
            .inspect(|s| trace!("Decoded instruction: {}", s))
            .map_err(|_| XError::DecodeError)
    }

    fn execute(&mut self, pc: &mut VirtAddr, instr: String) -> XResult<()> {
        trace!("Executing instruction: {}", instr);
        *pc += 4;
        Ok(())
    }
}
