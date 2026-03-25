use std::sync::MutexGuard;

use memory_addr::{MemoryAddr, VirtAddr};

use super::RVCore;
use crate::{
    config::{Word, word_to_u32},
    cpu::riscv::trap::Exception,
    device::bus::Bus,
    error::XResult,
};

struct Faults {
    misaligned: Exception,
    access: Exception,
}

const LOAD_FAULTS: Faults = Faults {
    misaligned: Exception::LoadMisaligned,
    access: Exception::LoadAccessFault,
};

const STORE_FAULTS: Faults = Faults {
    misaligned: Exception::StoreMisaligned,
    access: Exception::StoreAccessFault,
};

impl RVCore {
    fn bus(&self) -> MutexGuard<'_, Bus> {
        self.bus.lock().expect("bus lock poisoned")
    }

    /// Identity mapping — will be replaced by MMU translate in Step 2.
    pub(super) fn virt_to_phys(&self, vaddr: VirtAddr) -> usize {
        vaddr.as_usize()
    }

    fn checked_read(&mut self, addr: VirtAddr, size: usize, faults: &Faults) -> XResult<Word> {
        let tval = addr.as_usize() as Word;
        if !addr.is_aligned(size) {
            return self.trap_exception(faults.misaligned, tval);
        }
        let paddr = self.virt_to_phys(addr);
        self.bus()
            .read(paddr, size)
            .or_else(|_| self.trap_exception(faults.access, tval))
    }

    fn checked_write(
        &mut self,
        addr: VirtAddr,
        size: usize,
        value: Word,
        faults: &Faults,
    ) -> XResult {
        let tval = addr.as_usize() as Word;
        if !addr.is_aligned(size) {
            return self.trap_exception(faults.misaligned, tval);
        }
        let paddr = self.virt_to_phys(addr);
        self.bus()
            .write(paddr, size, value)
            .or_else(|_| self.trap_exception(faults.access, tval))
    }

    pub(super) fn fetch(&mut self) -> XResult<u32> {
        let tval = self.pc.as_usize() as Word;
        if !self.pc.is_aligned(2_usize) {
            return self.trap_exception(Exception::InstructionMisaligned, tval);
        }
        let paddr = self.virt_to_phys(self.pc);
        let word = self
            .bus()
            .read(paddr, 4)
            .or_else(|_| self.trap_exception(Exception::InstructionAccessFault, tval))?;
        let inst = word_to_u32(word);
        Ok(if inst & 0b11 != 0b11 {
            inst & 0xFFFF
        } else {
            inst
        })
    }

    pub(super) fn load(&mut self, addr: VirtAddr, size: usize) -> XResult<Word> {
        self.checked_read(addr, size, &LOAD_FAULTS)
    }

    pub(super) fn store(&mut self, addr: VirtAddr, size: usize, value: Word) -> XResult {
        self.checked_write(addr, size, value, &STORE_FAULTS)
    }

    pub(super) fn amo_load(&mut self, addr: VirtAddr, size: usize) -> XResult<Word> {
        self.checked_read(addr, size, &STORE_FAULTS)
    }

    pub(super) fn amo_store(&mut self, addr: VirtAddr, size: usize, value: Word) -> XResult {
        self.checked_write(addr, size, value, &STORE_FAULTS)
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
    fn fetch_access_fault() {
        let mut core = RVCore::new();
        core.pc = VirtAddr::from(CONFIG_MBASE - 4);
        assert_trap(
            core.fetch(),
            TrapCause::Exception(Exception::InstructionAccessFault),
            (CONFIG_MBASE - 4) as Word,
        );
    }

    #[test]
    fn load_misaligned() {
        let mut core = RVCore::new();
        assert_trap(
            core.load(VirtAddr::from(CONFIG_MBASE + 2), 4),
            TrapCause::Exception(Exception::LoadMisaligned),
            (CONFIG_MBASE + 2) as Word,
        );
    }

    #[test]
    fn load_unmapped() {
        let mut core = RVCore::new();
        assert_trap(
            core.load(VirtAddr::from(0x1000_usize), 4),
            TrapCause::Exception(Exception::LoadAccessFault),
            0x1000,
        );
    }
}
