use memory_addr::{PhysAddr, VirtAddr};

use crate::XResult;

pub trait MemOps {
    fn virt_to_phys(&self, vaddr: VirtAddr) -> PhysAddr;
    fn init_memory(&self, start_addr: PhysAddr) -> XResult;
}
