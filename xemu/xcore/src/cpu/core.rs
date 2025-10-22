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