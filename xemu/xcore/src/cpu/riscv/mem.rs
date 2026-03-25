use memory_addr::{MemoryAddr, VirtAddr};

use super::RVCore;
use crate::{
    config::{Word, word_to_u32},
    cpu::riscv::{
        csr::{CsrAddr, MStatus, PrivilegeMode},
        mmu::MemOp,
        trap::{Exception, PendingTrap, TrapCause},
    },
    error::{XError, XResult},
};

impl RVCore {
    fn effective_priv(&self) -> PrivilegeMode {
        let ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
        if self.privilege == PrivilegeMode::Machine && ms.contains(MStatus::MPRV) {
            ms.mpp()
        } else {
            self.privilege
        }
    }

    /// Full access path: vaddr → MMU translate → PMP check → paddr.
    pub(super) fn translate(&mut self, vaddr: VirtAddr, op: MemOp) -> XResult<usize> {
        let bus = self.bus.lock().expect("bus lock poisoned");
        let priv_mode = match op {
            MemOp::Fetch => self.privilege,
            _ => self.effective_priv(),
        };
        let paddr = self
            .mmu
            .translate(vaddr, op, priv_mode, &self.pmp, &bus)
            .map_err(|e| Self::to_trap(e, vaddr, op))?;
        self.pmp
            .check(paddr, op, self.privilege)
            .map_err(|e| Self::to_trap(e, vaddr, op))?;
        Ok(paddr)
    }

    fn to_trap(err: XError, vaddr: VirtAddr, op: MemOp) -> XError {
        let tval = vaddr.as_usize() as Word;
        let exc = match err {
            XError::PageFault => match op {
                MemOp::Fetch => Exception::InstructionPageFault,
                MemOp::Load => Exception::LoadPageFault,
                _ => Exception::StorePageFault,
            },
            XError::BadAddress => match op {
                MemOp::Fetch => Exception::InstructionAccessFault,
                MemOp::Load => Exception::LoadAccessFault,
                _ => Exception::StoreAccessFault,
            },
            other => return other,
        };
        XError::Trap(PendingTrap {
            cause: TrapCause::Exception(exc),
            tval,
        })
    }

    fn bus_read(&self, paddr: usize, size: usize, vaddr: VirtAddr, op: MemOp) -> XResult<Word> {
        self.bus
            .lock()
            .unwrap()
            .read(paddr, size)
            .map_err(|e| Self::to_trap(e, vaddr, op))
    }

    fn bus_write(
        &self,
        paddr: usize,
        size: usize,
        value: Word,
        vaddr: VirtAddr,
        op: MemOp,
    ) -> XResult {
        self.bus
            .lock()
            .unwrap()
            .write(paddr, size, value)
            .map_err(|e| Self::to_trap(e, vaddr, op))
    }

    pub(super) fn fetch(&mut self) -> XResult<u32> {
        if !self.pc.is_aligned(2_usize) {
            return self
                .trap_exception(Exception::InstructionMisaligned, self.pc.as_usize() as Word);
        }
        let paddr = self.translate(self.pc, MemOp::Fetch)?;
        let word = self.bus_read(paddr, 4, self.pc, MemOp::Fetch)?;
        let inst = word_to_u32(word);
        Ok(if inst & 0b11 != 0b11 {
            inst & 0xFFFF
        } else {
            inst
        })
    }

    pub(super) fn load(&mut self, addr: VirtAddr, size: usize) -> XResult<Word> {
        if !addr.is_aligned(size) {
            return self.trap_exception(Exception::LoadMisaligned, addr.as_usize() as Word);
        }
        let paddr = self.translate(addr, MemOp::Load)?;
        self.bus_read(paddr, size, addr, MemOp::Load)
    }

    pub(super) fn store(&mut self, addr: VirtAddr, size: usize, value: Word) -> XResult {
        if !addr.is_aligned(size) {
            return self.trap_exception(Exception::StoreMisaligned, addr.as_usize() as Word);
        }
        let paddr = self.translate(addr, MemOp::Store)?;
        self.bus_write(paddr, size, value, addr, MemOp::Store)
    }

    pub(super) fn amo_load(&mut self, addr: VirtAddr, size: usize) -> XResult<Word> {
        if !addr.is_aligned(size) {
            return self.trap_exception(Exception::StoreMisaligned, addr.as_usize() as Word);
        }
        let paddr = self.translate(addr, MemOp::Amo)?;
        self.bus_read(paddr, size, addr, MemOp::Amo)
    }

    pub(super) fn amo_store(&mut self, addr: VirtAddr, size: usize, value: Word) -> XResult {
        if !addr.is_aligned(size) {
            return self.trap_exception(Exception::StoreMisaligned, addr.as_usize() as Word);
        }
        let paddr = self.translate(addr, MemOp::Amo)?;
        self.bus_write(paddr, size, value, addr, MemOp::Amo)
    }
}

#[cfg(test)]
mod tests {
    use memory_addr::VirtAddr;

    use super::*;
    use crate::{config::CONFIG_MBASE, cpu::riscv::trap::test_helpers::assert_trap};

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
