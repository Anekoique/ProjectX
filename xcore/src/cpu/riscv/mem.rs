use memory_addr::{PhysAddr, VirtAddr};

use super::RVCore;

impl RVCore {
    pub(super) fn virt_to_phys(&self, vaddr: VirtAddr) -> PhysAddr {
        // For simplicity, assume identity mapping for now
        PhysAddr::from(vaddr.as_usize())
    }

    pub(super) fn init_memory(&self, memory: &mut crate::memory::Memory) {
        // Initialize memory with some data if needed
        // For example, load a bootloader or kernel image
        let image_bytes: &[u8] = bytemuck::bytes_of(&crate::isa::IMG);
        memory
            .load(PhysAddr::from(0x80000000), image_bytes)
            .expect("Failed to load dummy image")
    }
}
