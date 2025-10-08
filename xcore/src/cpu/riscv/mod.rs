mod inst;
mod mem;

use memory_addr::VirtAddr;

use super::CoreOps;
use crate::{
    config::Word,
    error::XResult,
    isa::{DECODER, DecodedInst, RVReg},
    with_mem,
};

const RESET_VECTOR: usize = 0x80000000;

pub struct RVCore {
    gpr: [Word; 32],
    pub pc: VirtAddr,
}

impl RVCore {
    pub fn new() -> Self {
        Self {
            gpr: [0; 32],
            pc: VirtAddr::from(0),
        }
    }
}

impl Default for RVCore {
    fn default() -> Self {
        Self::new()
    }
}

impl CoreOps for RVCore {
    fn pc(&self) -> VirtAddr {
        self.pc
    }
    
    fn reset(&mut self) -> XResult {
        self.gpr.fill(0);
        self.init_memory(self.virt_to_phys(VirtAddr::from(RESET_VECTOR)))?;
        self.pc = VirtAddr::from(RESET_VECTOR);
        Ok(())
    }

    fn fetch(&self) -> XResult<u32> {
        with_mem!(read(self.virt_to_phys(self.pc), 4))
    }

    fn decode(&self, instr: u32) -> XResult<DecodedInst> {
        DECODER.decode(instr)
    }

    fn execute(&mut self, inst: DecodedInst) -> XResult {
        trace!("Executing instruction: {:?}", inst);
        self.dispatch(inst)?;
        self.pc += 4;
        Ok(())
    }

    fn halt_ret(&self) -> Word {
        self.gpr[RVReg::a0]
    }
}
