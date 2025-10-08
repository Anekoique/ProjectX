use memory_addr::{PhysAddr, VirtAddr};

use super::RVCore;
use crate::{XResult, with_mem};

impl RVCore {
    pub(super) fn virt_to_phys(&self, vaddr: VirtAddr) -> PhysAddr {
        // For simplicity, assume identity mapping for now
        PhysAddr::from(vaddr.as_usize())
    }

    pub(super) fn init_memory(&self, start_addr: PhysAddr) -> XResult {
        // Initialize memory with some data if needed
        // For example, load a bootloader or kernel image
        let image_bytes: &[u8] = bytemuck::bytes_of(&crate::isa::IMG);
        with_mem!(load(start_addr, image_bytes))
    }
}
