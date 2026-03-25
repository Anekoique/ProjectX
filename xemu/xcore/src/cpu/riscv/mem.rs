use memory_addr::{PhysAddr, VirtAddr};

use super::RVCore;
use crate::{
    config::{Word, word_to_u32},
    cpu::{mem::MemOps, riscv::trap::Exception},
    error::{XError, XResult},
    memory::with_mem,
};

impl RVCore {
    fn map_mem_err<T>(
        &self,
        err: XError,
        addr: VirtAddr,
        misaligned: Exception,
        access_fault: Exception,
    ) -> XResult<T> {
        let tval = addr.as_usize() as Word;
        match err {
            XError::AddrNotAligned => self.trap_exception(misaligned, tval),
            XError::BadAddress => self.trap_exception(access_fault, tval),
            _ => Err(err),
        }
    }

    fn read_mem(
        &self,
        addr: VirtAddr,
        size: usize,
        misaligned: Exception,
        access_fault: Exception,
    ) -> XResult<Word> {
        with_mem!(read(self.virt_to_phys(addr), size))
            .or_else(|err| self.map_mem_err(err, addr, misaligned, access_fault))
    }

    fn write_mem(
        &self,
        addr: VirtAddr,
        size: usize,
        value: Word,
        misaligned: Exception,
        access_fault: Exception,
    ) -> XResult {
        with_mem!(write(self.virt_to_phys(addr), size, value))
            .or_else(|err| self.map_mem_err(err, addr, misaligned, access_fault))
    }

    pub(super) fn amo_load(&self, addr: VirtAddr, size: usize) -> XResult<Word> {
        self.read_mem(
            addr,
            size,
            Exception::StoreMisaligned,
            Exception::StoreAccessFault,
        )
    }

    pub(super) fn amo_store(&self, addr: VirtAddr, size: usize, value: Word) -> XResult {
        self.write_mem(
            addr,
            size,
            value,
            Exception::StoreMisaligned,
            Exception::StoreAccessFault,
        )
    }
}

impl MemOps for RVCore {
    fn virt_to_phys(&self, vaddr: VirtAddr) -> PhysAddr {
        // HACK: assume identity mapping for now
        PhysAddr::from(vaddr.as_usize())
    }

    fn init_memory(&self, start_addr: PhysAddr) -> XResult {
        // Initialize memory with some data if needed
        // For example, load a bootloader or kernel image
        let image_bytes: &[u8] = bytemuck::bytes_of(&crate::isa::IMG);
        with_mem!(load_img(start_addr, image_bytes))
    }

    fn fetch(&self) -> XResult<u32> {
        let inst = with_mem!(fetch(self.virt_to_phys(self.pc), 4))
            .or_else(|err| {
                self.map_mem_err(
                    err,
                    self.pc,
                    Exception::InstructionMisaligned,
                    Exception::InstructionAccessFault,
                )
            })
            .map(word_to_u32)?;
        if (inst & 0b11) != 0b11 {
            Ok(inst & 0xFFFF)
        } else {
            Ok(inst)
        }
    }

    fn load(&self, addr: VirtAddr, size: usize) -> XResult<Word> {
        self.read_mem(
            addr,
            size,
            Exception::LoadMisaligned,
            Exception::LoadAccessFault,
        )
    }

    fn store(&mut self, addr: VirtAddr, size: usize, value: Word) -> XResult {
        self.write_mem(
            addr,
            size,
            value,
            Exception::StoreMisaligned,
            Exception::StoreAccessFault,
        )
    }
}

#[cfg(test)]
mod tests {
    use memory_addr::VirtAddr;

    use super::*;
    use crate::{
        config::CONFIG_MBASE,
        cpu::riscv::trap::{TrapCause, test_helpers::assert_trap},
    };

    #[test]
    fn fetch_fault_uses_pc_as_tval() {
        let mut core = RVCore::new();
        let bad_pc = CONFIG_MBASE - 4;
        core.pc = VirtAddr::from(bad_pc);

        assert_trap(
            core.fetch(),
            TrapCause::Exception(Exception::InstructionAccessFault),
            bad_pc as Word,
        );
    }

    #[test]
    fn load_fault_uses_vaddr_as_tval() {
        let core = RVCore::new();
        let bad_addr = VirtAddr::from(CONFIG_MBASE + 2);

        assert_trap(
            core.load(bad_addr, 4),
            TrapCause::Exception(Exception::LoadMisaligned),
            bad_addr.as_usize() as Word,
        );
    }
}
