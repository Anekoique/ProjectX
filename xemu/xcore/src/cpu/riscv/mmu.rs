use std::sync::MutexGuard;

use memory_addr::VirtAddr;

use super::pmp::Pmp;
use crate::{
    config::Word,
    cpu::riscv::csr::PrivilegeMode,
    device::bus::Bus,
    error::{XError, XResult},
};

// ---------------------------------------------------------------------------
// MemOp — what kind of memory access
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MemOp {
    Fetch,
    Load,
    Store,
    Amo,
}

// ---------------------------------------------------------------------------
// SvMode — page table format descriptor
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct SvMode {
    pub levels: usize,
    pub pte_size: usize,
    pub vpn_bits: usize,
    pub va_bits: usize,
}

#[allow(dead_code)]
pub static SV32: SvMode = SvMode {
    levels: 2,
    pte_size: 4,
    vpn_bits: 10,
    va_bits: 32,
};
#[allow(dead_code)]
pub static SV39: SvMode = SvMode {
    levels: 3,
    pte_size: 8,
    vpn_bits: 9,
    va_bits: 39,
};
#[allow(dead_code)]
pub static SV48: SvMode = SvMode {
    levels: 4,
    pte_size: 8,
    vpn_bits: 9,
    va_bits: 48,
};
#[allow(dead_code)]
pub static SV57: SvMode = SvMode {
    levels: 5,
    pte_size: 8,
    vpn_bits: 9,
    va_bits: 57,
};

// ---------------------------------------------------------------------------
// Tlb — stub (will be implemented in Step 4)
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub(super) struct Tlb;

#[allow(dead_code)]
impl Tlb {
    pub fn new() -> Self {
        Self
    }
    pub fn flush(&mut self, _vpn: Option<usize>, _asid: Option<u16>) {}
}

// ---------------------------------------------------------------------------
// Mmu — cached config + translate
// ---------------------------------------------------------------------------

#[allow(dead_code)]
pub struct Mmu {
    pub(super) tlb: Tlb,
    sv: Option<&'static SvMode>,
    ppn: usize,
    asid: u16,
    sum: bool,
    mxr: bool,
}

impl Mmu {
    pub fn new() -> Self {
        Self {
            tlb: Tlb::new(),
            sv: None, // Bare mode
            ppn: 0,
            asid: 0,
            sum: false,
            mxr: false,
        }
    }

    #[allow(dead_code)]
    pub fn update_satp(&mut self, satp: Word) {
        self.sv = match satp_mode(satp) {
            #[cfg(isa32)]
            1 => Some(&SV32),
            #[cfg(isa64)]
            8 => Some(&SV39),
            #[cfg(isa64)]
            9 => Some(&SV48),
            #[cfg(isa64)]
            10 => Some(&SV57),
            _ => None, // Bare or reserved
        };
        self.ppn = satp_ppn(satp);
        self.asid = satp_asid(satp);
        self.tlb.flush(None, None);
    }

    #[allow(dead_code)]
    pub fn update_mstatus(&mut self, sum: bool, mxr: bool) {
        self.sum = sum;
        self.mxr = mxr;
    }

    /// Translate virtual → physical. Bare mode and M-mode return identity.
    pub fn translate(
        &mut self,
        vaddr: VirtAddr,
        _op: MemOp,
        priv_mode: PrivilegeMode,
        _pmp: &Pmp,
        _bus: &MutexGuard<'_, Bus>,
    ) -> XResult<usize> {
        let Some(_sv) = self.sv else {
            return Ok(vaddr.as_usize()); // Bare mode
        };
        if priv_mode == PrivilegeMode::Machine {
            return Ok(vaddr.as_usize()); // M-mode identity
        }

        // Page walk will be implemented in Step 3.
        // For now, S/U mode with paging enabled is not yet supported.
        Err(XError::PageFault)
    }
}

// ---------------------------------------------------------------------------
// satp field extraction
// ---------------------------------------------------------------------------

#[cfg(isa64)]
fn satp_mode(satp: Word) -> usize {
    (satp >> 60) as usize
}

#[cfg(isa32)]
fn satp_mode(satp: Word) -> usize {
    (satp >> 31) as usize
}

#[cfg(isa64)]
fn satp_ppn(satp: Word) -> usize {
    (satp & ((1u64 << 44) - 1)) as usize
}

#[cfg(isa32)]
fn satp_ppn(satp: Word) -> usize {
    (satp & ((1u32 << 22) - 1)) as usize
}

#[cfg(isa64)]
fn satp_asid(satp: Word) -> u16 {
    ((satp >> 44) & 0xFFFF) as u16
}

#[cfg(isa32)]
fn satp_asid(satp: Word) -> u16 {
    ((satp >> 22) & 0x1FF) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_mode_is_identity() {
        let mut mmu = Mmu::new();
        let pmp = Pmp::new();
        let bus = crate::device::bus::Bus::new(0x8000_0000, 0x1000);
        let guard = std::sync::Mutex::new(bus);
        let lock = guard.lock().unwrap();

        let addr = VirtAddr::from(0x8000_1234_usize);
        let result = mmu.translate(addr, MemOp::Load, PrivilegeMode::Machine, &pmp, &lock);
        assert_eq!(result.unwrap(), 0x8000_1234);
    }

    #[test]
    fn m_mode_always_identity() {
        let mut mmu = Mmu::new();
        // Set satp to SV39 mode — but M-mode should still be identity
        #[cfg(isa64)]
        mmu.update_satp(8 << 60); // mode=8 (SV39)
        #[cfg(isa32)]
        mmu.update_satp(1 << 31); // mode=1 (SV32)

        let pmp = Pmp::new();
        let bus = crate::device::bus::Bus::new(0x8000_0000, 0x1000);
        let guard = std::sync::Mutex::new(bus);
        let lock = guard.lock().unwrap();

        let addr = VirtAddr::from(0x8000_0000_usize);
        let result = mmu.translate(addr, MemOp::Fetch, PrivilegeMode::Machine, &pmp, &lock);
        assert_eq!(result.unwrap(), 0x8000_0000);
    }
}
