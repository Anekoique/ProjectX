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
        self.npc = self.pc;
        Ok(())
    }

    fn fetch(&self) -> XResult<u32> {
        let low = with_mem!(read(self.virt_to_phys(self.pc), 2))?;
        let low_u32 = word_to_u32(low);
        if (low_u32 & 0b11) != 0b11 {
            return Ok(low_u32);
        }
        let high = with_mem!(read(self.virt_to_phys(self.pc.wrapping_add(2)), 2))?;
        let high_u32 = word_to_u32(high);
        Ok((high_u32 << 16) | (low_u32 & 0xFFFF))
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
    fn fetch_returns_32bit_value() {
        let mut core = RVCore::new();
        core.pc = VirtAddr::from(CONFIG_MBASE);
        let inst: u32 = 0xCAFEBABE;
        with_mem!(write(core.virt_to_phys(core.pc), 4, inst as Word)).unwrap();

        assert_eq!(core.fetch().unwrap(), inst);
    }
}
