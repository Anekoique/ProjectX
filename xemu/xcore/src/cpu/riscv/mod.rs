mod inst;
mod mem;

use memory_addr::{MemoryAddr, VirtAddr};

pub use self::RVCore as Core;
use super::{CoreOps, MemOps, RESET_VECTOR};
use crate::{
    config::{Word, word_to_u32},
    error::XResult,
    isa::{DECODER, DecodedInst, RVReg},
    memory::with_mem,
};

pub struct RVCore {
    gpr: [Word; 32],
    pub pc: VirtAddr,
    npc: VirtAddr,
}

impl RVCore {
    pub fn new() -> Self {
        Self {
            gpr: [0; 32],
            pc: VirtAddr::from(0),
            npc: VirtAddr::from(0),
        }
    }
}

impl CoreOps for RVCore {
    fn pc(&self) -> VirtAddr {
        self.pc
    }

    fn reset(&mut self) -> XResult {
        self.gpr.fill(0);
        self.pc = VirtAddr::from(RESET_VECTOR);
        self.npc = self.pc;
        Ok(())
    }

    fn fetch(&self) -> XResult<u32> {
        let word = with_mem!(fetch_u32(self.virt_to_phys(self.pc), 4))?;
        let inst = word_to_u32(word);
        if (inst & 0b11) != 0b11 {
            Ok(inst & 0xFFFF)
        } else {
            Ok(inst)
        }
    }

    fn decode(&self, instr: u32) -> XResult<DecodedInst> {
        DECODER.decode(instr)
    }

    fn execute(&mut self, inst: DecodedInst) -> XResult {
        trace!("PC: {:?} Executing instruction: {:?}", self.pc, inst);
        let is_compressed = matches!(&inst, DecodedInst::C { .. });
        let step = if is_compressed { 2 } else { 4 };
        self.npc = self.pc.wrapping_add(step);
        self.dispatch(inst)?;
        self.pc = self.npc;
        Ok(())
    }

    fn halt_ret(&self) -> Word {
        self.gpr[RVReg::a0]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::CONFIG_MBASE;

    #[test]
    fn fetch_distinguishes_standard_and_compressed_instructions() {
        let mut core = RVCore::new();
        core.pc = VirtAddr::from(CONFIG_MBASE);

        let cases = [
            (0xCAFEBABF_u32, 0xCAFEBABF_u32),
            (0xCAFEBABE_u32, 0xBABE_u32),
        ];

        for (inst, expected) in cases {
            with_mem!(write(core.virt_to_phys(core.pc), 4, inst as Word)).unwrap();
            assert_eq!(core.fetch().unwrap(), expected);
        }
    }
}
