use memory_addr::{PhysAddr, VirtAddr};

use crate::{XResult, config::Word};

pub trait MemOps {
    fn virt_to_phys(&self, vaddr: VirtAddr) -> PhysAddr;
    fn init_memory(&self, start_addr: PhysAddr) -> XResult;
    fn fetch(&self) -> XResult<u32>;
    fn load(&self, addr: VirtAddr, size: usize) -> XResult<Word>;
    fn store(&mut self, addr: VirtAddr, size: usize, value: Word) -> XResult;
}
