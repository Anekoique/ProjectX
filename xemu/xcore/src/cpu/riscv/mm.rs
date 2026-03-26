mod mmu;
pub(super) mod pmp;
mod tlb;

use bitflags::bitflags;
use memory_addr::{MemoryAddr, VirtAddr};

pub use self::{mmu::Mmu, pmp::Pmp};
use super::RVCore;
use crate::{
    config::Word,
    cpu::riscv::{
        csr::{CsrAddr, MStatus, PrivilegeMode},
        trap::{Exception, PendingTrap, TrapCause},
    },
    device::bus::Bus,
    error::{XError, XResult},
};

// --- MemOp ---

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MemOp {
    Fetch,
    Load,
    Store,
    Amo,
}

// --- SvMode + SvConfig ---

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[rustfmt::skip]
pub enum SvMode {
    Bare,
    #[cfg(isa32)] Sv32,
    #[cfg(isa64)] Sv39,
    #[cfg(isa64)] Sv48,
    #[cfg(isa64)] Sv57,
}

impl SvMode {
    pub fn from_satp(satp: Word) -> Self {
        #[cfg(isa32)]
        if satp >> 31 == 1 {
            return Self::Sv32;
        }
        #[cfg(isa64)]
        match satp >> 60 {
            8 => return Self::Sv39,
            9 => return Self::Sv48,
            10 => return Self::Sv57,
            _ => {}
        }
        Self::Bare
    }

    #[inline]
    #[rustfmt::skip]
    pub fn config(self) -> SvConfig {
        match self {
            Self::Bare => unreachable!("config() called on Bare mode"),
            #[cfg(isa32)]
            Self::Sv32 => SvConfig { levels: 2, pte_size: 4, vpn_bits: 10, va_bits: 32, ppn_bits: 22 },
            #[cfg(isa64)]
            Self::Sv39 => SvConfig { levels: 3, pte_size: 8, vpn_bits: 9, va_bits: 39, ppn_bits: 44 },
            #[cfg(isa64)]
            Self::Sv48 => SvConfig { levels: 4, pte_size: 8, vpn_bits: 9, va_bits: 48, ppn_bits: 44 },
            #[cfg(isa64)]
            Self::Sv57 => SvConfig { levels: 5, pte_size: 8, vpn_bits: 9, va_bits: 57, ppn_bits: 44 },
        }
    }
}

pub struct SvConfig {
    pub levels: usize,
    pub pte_size: usize,
    pub vpn_bits: usize,
    pub va_bits: usize,
    pub ppn_bits: usize,
}

// --- Satp ---

pub(super) struct Satp {
    pub mode: SvMode,
    pub ppn: usize,
    pub asid: u16,
}

impl Satp {
    pub fn parse(raw: Word) -> Self {
        Self {
            mode: SvMode::from_satp(raw),
            ppn: Self::ppn(raw),
            asid: Self::asid(raw),
        }
    }

    #[cfg(isa64)]
    fn ppn(raw: Word) -> usize {
        (raw & ((1u64 << 44) - 1)) as usize
    }
    #[cfg(isa32)]
    fn ppn(raw: Word) -> usize {
        (raw & ((1u32 << 22) - 1)) as usize
    }
    #[cfg(isa64)]
    fn asid(raw: Word) -> u16 {
        ((raw >> 44) & 0xFFFF) as u16
    }
    #[cfg(isa32)]
    fn asid(raw: Word) -> u16 {
        ((raw >> 22) & 0x1FF) as u16
    }
}

// --- PteFlags + Pte ---

const PAGE_SHIFT: usize = 12;
const PAGE_SIZE: usize = 1 << PAGE_SHIFT;

bitflags! {
    #[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub(super) struct PteFlags: usize {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

impl PteFlags {
    pub(super) fn permits(
        &self,
        op: MemOp,
        priv_mode: PrivilegeMode,
        sum: bool,
        mxr: bool,
    ) -> bool {
        let perm_ok = match op {
            MemOp::Fetch => self.contains(Self::X),
            MemOp::Load => self.contains(Self::R) || (mxr && self.contains(Self::X)),
            _ => self.contains(Self::W),
        };
        let priv_ok = if self.contains(Self::U) {
            match priv_mode {
                PrivilegeMode::User => true,
                PrivilegeMode::Supervisor => op != MemOp::Fetch && sum,
                _ => false,
            }
        } else {
            priv_mode != PrivilegeMode::User
        };
        // Svade: A must be set; D must be set for writes
        let ad_ok = self.contains(Self::A)
            && (!matches!(op, MemOp::Store | MemOp::Amo) || self.contains(Self::D));
        perm_ok && priv_ok && ad_ok
    }
}

#[derive(Clone, Copy)]
pub(super) struct Pte(pub usize);

impl Pte {
    #[inline]
    pub fn flags(self) -> PteFlags {
        PteFlags::from_bits_truncate(self.0)
    }

    #[inline]
    pub fn is_valid(self) -> bool {
        self.flags().contains(PteFlags::V)
    }

    #[inline]
    pub fn is_leaf(self) -> bool {
        self.flags().intersects(PteFlags::R | PteFlags::X)
    }

    pub fn is_reserved(self) -> bool {
        self.flags().contains(PteFlags::W) && !self.flags().contains(PteFlags::R)
    }

    pub fn has_nonleaf_reserved_bits(self) -> bool {
        self.flags()
            .intersects(PteFlags::D | PteFlags::A | PteFlags::U)
    }

    pub fn has_high_reserved_bits(self, sv: &SvConfig) -> bool {
        let ppn_top = 10 + sv.ppn_bits;
        ppn_top < usize::BITS as usize && (self.0 >> ppn_top) != 0
    }

    #[inline]
    pub fn ppn(self, sv: &SvConfig) -> usize {
        (self.0 >> 10) & ((1 << sv.ppn_bits) - 1)
    }

    pub fn superpage_aligned(self, level: usize, sv: &SvConfig) -> bool {
        level == 0 || (self.ppn(sv) & ((1 << (level * sv.vpn_bits)) - 1)) == 0
    }
}

// --- RVCore memory access methods ---

impl RVCore {
    #[inline]
    fn effective_priv(&self) -> PrivilegeMode {
        let ms = MStatus::from_bits_truncate(self.csr.get(CsrAddr::mstatus));
        if self.privilege == PrivilegeMode::Machine && ms.contains(MStatus::MPRV) {
            ms.mpp()
        } else {
            self.privilege
        }
    }

    #[inline]
    fn to_trap(err: XError, vaddr: VirtAddr, op: MemOp) -> XError {
        let exc = match (err, op) {
            (XError::PageFault, MemOp::Fetch) => Exception::InstructionPageFault,
            (XError::PageFault, MemOp::Load) => Exception::LoadPageFault,
            (XError::PageFault, _) => Exception::StorePageFault,
            (XError::BadAddress, MemOp::Fetch) => Exception::InstructionAccessFault,
            (XError::BadAddress, MemOp::Load) => Exception::LoadAccessFault,
            (XError::BadAddress, _) => Exception::StoreAccessFault,
            (other, _) => return other,
        };
        XError::Trap(PendingTrap {
            cause: TrapCause::Exception(exc),
            tval: vaddr.as_usize() as Word,
        })
    }

    fn access_bus<T>(
        &mut self,
        addr: VirtAddr,
        op: MemOp,
        size: usize,
        f: impl FnOnce(&mut Bus, usize) -> XResult<T>,
    ) -> XResult<T> {
        let priv_mode = match op {
            MemOp::Fetch => self.privilege,
            _ => self.effective_priv(),
        };
        let mut bus = self.bus.lock().unwrap();
        self.mmu
            .translate(addr, op, priv_mode, &self.pmp, &bus)
            .and_then(|pa| self.pmp.check(pa, size, op, priv_mode).map(|_| pa))
            .and_then(|pa| f(&mut bus, pa))
            .map_err(|e| Self::to_trap(e, addr, op))
    }

    fn checked_read(&mut self, addr: VirtAddr, size: usize, op: MemOp) -> XResult<Word> {
        self.access_bus(addr, op, size, |bus, pa| bus.read(pa, size))
    }

    fn checked_write(&mut self, addr: VirtAddr, size: usize, value: Word, op: MemOp) -> XResult {
        self.access_bus(addr, op, size, |bus, pa| bus.write(pa, size, value))
    }

    pub(super) fn translate(&mut self, addr: VirtAddr, size: usize, op: MemOp) -> XResult<usize> {
        self.access_bus(addr, op, size, |_, pa| Ok(pa))
    }

    #[allow(clippy::unnecessary_cast)]
    pub(super) fn fetch(&mut self) -> XResult<u32> {
        self.validate_alignment(self.pc, 2, Exception::InstructionMisaligned)?;
        let lo = self.checked_read(self.pc, 2, MemOp::Fetch)? as u32;
        if lo & 0b11 != 0b11 {
            return Ok(lo & 0xFFFF);
        }
        let pc2 = VirtAddr::from(self.pc.as_usize() + 2);
        let hi = self.checked_read(pc2, 2, MemOp::Fetch)? as u32;
        Ok((lo & 0xFFFF) | ((hi & 0xFFFF) << 16))
    }

    pub(super) fn load(&mut self, addr: VirtAddr, size: usize) -> XResult<Word> {
        self.validate_alignment(addr, size, Exception::LoadMisaligned)?;
        self.checked_read(addr, size, MemOp::Load)
    }

    pub(super) fn store(&mut self, addr: VirtAddr, size: usize, value: Word) -> XResult {
        self.validate_alignment(addr, size, Exception::StoreMisaligned)?;
        self.checked_write(addr, size, value, MemOp::Store)
    }

    pub(super) fn amo_load(&mut self, addr: VirtAddr, size: usize) -> XResult<Word> {
        self.validate_alignment(addr, size, Exception::StoreMisaligned)?;
        self.checked_read(addr, size, MemOp::Amo)
    }

    pub(super) fn amo_store(&mut self, addr: VirtAddr, size: usize, value: Word) -> XResult {
        self.validate_alignment(addr, size, Exception::StoreMisaligned)?;
        self.checked_write(addr, size, value, MemOp::Amo)
    }

    #[inline]
    fn validate_alignment(&mut self, addr: VirtAddr, size: usize, exc: Exception) -> XResult {
        addr.is_aligned(size)
            .ok_or_else(|| self.trap_exception(exc, addr.as_usize() as Word))
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
