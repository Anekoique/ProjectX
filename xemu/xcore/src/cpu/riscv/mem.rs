use memory_addr::{MemoryAddr, VirtAddr};

use super::RVCore;
use crate::{
    config::{Word, word_to_u32},
    cpu::riscv::{csr::PrivilegeMode, mmu::MemOp, trap::Exception},
    error::{XError, XResult},
};

impl RVCore {
    /// Effective privilege for data accesses (accounts for MPRV).
    fn effective_priv(&self) -> PrivilegeMode {
        use crate::cpu::riscv::csr::MStatus;
        let ms =
            MStatus::from_bits_truncate(self.csr.get(crate::cpu::riscv::csr::CsrAddr::mstatus));
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
            .map_err(|e| Self::map_mem_err(e, vaddr, op))?;
        self.pmp
            .check(paddr, op, self.privilege)
            .map_err(|e| Self::map_mem_err(e, vaddr, op))?;
        Ok(paddr)
    }

    /// Map XError::{PageFault, BadAddress} → RISC-V trap.
    fn map_mem_err(err: XError, vaddr: VirtAddr, op: MemOp) -> XError {
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
        XError::Trap(crate::cpu::riscv::trap::PendingTrap {
            cause: crate::cpu::riscv::trap::TrapCause::Exception(exc),
            tval,
        })
    }

    pub(super) fn fetch(&mut self) -> XResult<u32> {
        let tval = self.pc.as_usize() as Word;
        if !self.pc.is_aligned(2_usize) {
            return self.trap_exception(Exception::InstructionMisaligned, tval);
        }
        let paddr = self.translate(self.pc, MemOp::Fetch)?;
        let word = self
            .bus
            .lock()
            .unwrap()
            .read(paddr, 4)
            .map_err(|e| Self::map_mem_err(e, self.pc, MemOp::Fetch))?;
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
        self.bus
            .lock()
            .unwrap()
            .read(paddr, size)
            .map_err(|e| Self::map_mem_err(e, addr, MemOp::Load))
    }

    pub(super) fn store(&mut self, addr: VirtAddr, size: usize, value: Word) -> XResult {
        if !addr.is_aligned(size) {
            return self.trap_exception(Exception::StoreMisaligned, addr.as_usize() as Word);
        }
        let paddr = self.translate(addr, MemOp::Store)?;
        self.bus
            .lock()
            .unwrap()
            .write(paddr, size, value)
            .map_err(|e| Self::map_mem_err(e, addr, MemOp::Store))
    }

    pub(super) fn amo_load(&mut self, addr: VirtAddr, size: usize) -> XResult<Word> {
        if !addr.is_aligned(size) {
            return self.trap_exception(Exception::StoreMisaligned, addr.as_usize() as Word);
        }
        let paddr = self.translate(addr, MemOp::Amo)?;
        self.bus
            .lock()
            .unwrap()
            .read(paddr, size)
            .map_err(|e| Self::map_mem_err(e, addr, MemOp::Amo))
    }

    pub(super) fn amo_store(&mut self, addr: VirtAddr, size: usize, value: Word) -> XResult {
        if !addr.is_aligned(size) {
            return self.trap_exception(Exception::StoreMisaligned, addr.as_usize() as Word);
        }
        let paddr = self.translate(addr, MemOp::Amo)?;
        self.bus
            .lock()
            .unwrap()
            .write(paddr, size, value)
            .map_err(|e| Self::map_mem_err(e, addr, MemOp::Amo))
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
