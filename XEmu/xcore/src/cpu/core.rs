use memory_addr::VirtAddr;

use crate::{config::Word, error::XResult, isa::DecodedInst};

pub trait CoreOps {
    fn pc(&self) -> VirtAddr;
    fn reset(&mut self) -> XResult;
    fn fetch(&self) -> XResult<u32>;
    fn decode(&self, instr: u32) -> XResult<DecodedInst>;
    fn execute(&mut self, inst: DecodedInst) -> XResult<()>;
    fn halt_ret(&self) -> Word;
}
// pub struct Core<C: CoreOps> {
//     pc: VirtAddr,
//     core: C,
// }
//
// impl<C: CoreOps> Core<C> {
//     pub fn new(core: C) -> Self {
//         Self {
//             pc: VirtAddr::from(0),
//             core,
//         }
//     }
//
//     pub fn pc(&self) -> VirtAddr {
//         self.pc
//     }
//
//     pub fn reset(&mut self, memory: &mut Memory) -> XResult {
//         self.pc = self.core.reset(memory);
//         Ok(())
//     }
//
//     pub fn step(&mut self, memory: &mut Memory) -> XResult {
//         let instr = self.core.fetch(memory, self.pc)?;
//         let decoded = self.core.decode(instr)?;
//         self.core.execute(&mut self.pc, decoded)?;
//         Ok(())
//     }
//
//     pub fn run(&mut self, memory: &mut Memory, count: u32) -> XResult {
//         for _ in 0..count {
//             self.step(memory)?;
//         }
//         Ok(())
//     }
// }
//
// #[inherit_methods(from = "self.core")]
// impl<C: CoreOps> CoreOps for Core<C> {
//     fn reset(&mut self, memory: &mut Memory) -> VirtAddr;
//     fn fetch(&self, mem: &Memory, addr: VirtAddr) -> XResult<u32>;
//     fn decode(&self, instr: u32) -> XResult<DecodedInst>;
//     fn execute(&mut self, pc: &mut VirtAddr, instr: DecodedInst) ->
// XResult<()>;     fn halt_ret(&self) -> Word;
// }
