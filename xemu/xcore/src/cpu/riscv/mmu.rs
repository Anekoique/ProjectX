use std::sync::MutexGuard;

use bitflags::bitflags;
use memory_addr::VirtAddr;

use super::pmp::Pmp;
use crate::{
    config::Word,
    cpu::riscv::csr::PrivilegeMode,
    device::bus::Bus,
    error::{XError, XResult},
};

// ---------------------------------------------------------------------------
// MemOp
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

pub struct SvMode {
    pub levels: usize,
    pub pte_size: usize,
    pub vpn_bits: usize,
    pub va_bits: usize,
}

pub static SV32: SvMode = SvMode {
    levels: 2,
    pte_size: 4,
    vpn_bits: 10,
    va_bits: 32,
};
pub static SV39: SvMode = SvMode {
    levels: 3,
    pte_size: 8,
    vpn_bits: 9,
    va_bits: 39,
};
pub static SV48: SvMode = SvMode {
    levels: 4,
    pte_size: 8,
    vpn_bits: 9,
    va_bits: 48,
};
pub static SV57: SvMode = SvMode {
    levels: 5,
    pte_size: 8,
    vpn_bits: 9,
    va_bits: 57,
};

const PAGE_SHIFT: usize = 12;
const PAGE_SIZE: usize = 1 << PAGE_SHIFT;

// ---------------------------------------------------------------------------
// PteFlags + Pte
// ---------------------------------------------------------------------------

bitflags! {
    struct PteFlags: usize {
        const V = 1 << 0; const R = 1 << 1; const W = 1 << 2; const X = 1 << 3;
        const U = 1 << 4; const G = 1 << 5; const A = 1 << 6; const D = 1 << 7;
    }
}

#[derive(Clone, Copy)]
struct Pte(usize);

impl Pte {
    fn flags(self) -> PteFlags {
        PteFlags::from_bits_truncate(self.0)
    }
    fn is_valid(self) -> bool {
        self.flags().contains(PteFlags::V)
    }
    fn is_leaf(self) -> bool {
        self.flags().intersects(PteFlags::R | PteFlags::X)
    }
    fn is_reserved(self) -> bool {
        self.flags().contains(PteFlags::W) && !self.flags().contains(PteFlags::R)
    }
    fn ppn(self, sv: &SvMode) -> usize {
        (self.0 >> 10) & ((1 << (sv.levels * sv.vpn_bits + 2)) - 1)
    }
    fn superpage_aligned(self, level: usize, sv: &SvMode) -> bool {
        level == 0 || (self.ppn(sv) & ((1 << (level * sv.vpn_bits)) - 1)) == 0
    }
    fn perm_bits(self) -> u8 {
        (self.flags() & (PteFlags::R | PteFlags::W | PteFlags::X | PteFlags::U | PteFlags::G))
            .bits() as u8
            >> 1
    }
}

// ---------------------------------------------------------------------------
// TlbEntry + Tlb (stub — Step 4 implements full TLB)
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Default)]
pub(super) struct TlbEntry {
    ppn: usize,
    level: u8,
    // Fields for Step 4:
    _vpn: usize,
    _asid: u16,
    _perm: u8,
    _valid: bool,
}

impl TlbEntry {
    fn from_pte(pte: Pte, vaddr: VirtAddr, level: usize, sv: &SvMode, asid: u16) -> Self {
        Self {
            ppn: pte.ppn(sv),
            level: level as u8,
            _vpn: vaddr.as_usize() >> PAGE_SHIFT,
            _asid: asid,
            _perm: pte.perm_bits(),
            _valid: true,
        }
    }

    fn translate(&self, vaddr: VirtAddr, sv: &SvMode) -> usize {
        if self.level > 0 {
            let mask = (1 << (self.level as usize * sv.vpn_bits + PAGE_SHIFT)) - 1;
            (self.ppn << PAGE_SHIFT) & !mask | (vaddr.as_usize() & mask)
        } else {
            self.ppn << PAGE_SHIFT | (vaddr.as_usize() & (PAGE_SIZE - 1))
        }
    }
}

pub(super) struct Tlb;

impl Tlb {
    pub fn new() -> Self {
        Self
    }
    pub fn flush(&mut self, _vpn: Option<usize>, _asid: Option<u16>) {}
}

// ---------------------------------------------------------------------------
// Mmu
// ---------------------------------------------------------------------------

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
            sv: None,
            ppn: 0,
            asid: 0,
            sum: false,
            mxr: false,
        }
    }

    pub fn update_satp(&mut self, satp: Word) {
        self.sv = satp_to_sv(satp);
        self.ppn = satp_ppn(satp);
        self.asid = satp_asid(satp);
        self.tlb.flush(None, None);
    }

    pub fn update_mstatus(&mut self, sum: bool, mxr: bool) {
        self.sum = sum;
        self.mxr = mxr;
    }

    pub fn translate(
        &mut self,
        vaddr: VirtAddr,
        op: MemOp,
        priv_mode: PrivilegeMode,
        pmp: &Pmp,
        bus: &MutexGuard<'_, Bus>,
    ) -> XResult<usize> {
        let Some(sv) = self.sv else {
            return Ok(vaddr.as_usize());
        };
        if priv_mode == PrivilegeMode::Machine {
            return Ok(vaddr.as_usize());
        }

        let entry = self.page_walk(vaddr, op, priv_mode, sv, pmp, bus)?;
        Ok(entry.translate(vaddr, sv))
    }

    fn page_walk(
        &self,
        vaddr: VirtAddr,
        op: MemOp,
        priv_mode: PrivilegeMode,
        sv: &SvMode,
        pmp: &Pmp,
        bus: &MutexGuard<'_, Bus>,
    ) -> XResult<TlbEntry> {
        if !is_canonical(vaddr, sv) {
            return Err(XError::PageFault);
        }

        let mut base = self.ppn * PAGE_SIZE;

        for level in (0..sv.levels).rev() {
            let pte_addr = base + vpn_index(vaddr, level, sv) * sv.pte_size;

            pmp.check(pte_addr, MemOp::Load, PrivilegeMode::Supervisor)?;

            let pte = bus
                .read_ram(pte_addr, sv.pte_size)
                .map(|w| Pte(w as usize))?;

            if !pte.is_valid() || pte.is_reserved() {
                return Err(XError::PageFault);
            }

            if pte.is_leaf() {
                if !pte.superpage_aligned(level, sv) || !self.check_perm(pte, op, priv_mode) {
                    return Err(XError::PageFault);
                }
                return Ok(TlbEntry::from_pte(pte, vaddr, level, sv, self.asid));
            }

            base = pte.ppn(sv) * PAGE_SIZE;
        }

        Err(XError::PageFault)
    }

    fn check_perm(&self, pte: Pte, op: MemOp, priv_mode: PrivilegeMode) -> bool {
        let f = pte.flags();

        let perm_ok = match op {
            MemOp::Fetch => f.contains(PteFlags::X),
            MemOp::Load => f.contains(PteFlags::R) || (self.mxr && f.contains(PteFlags::X)),
            _ => f.contains(PteFlags::W),
        };
        let priv_ok = if f.contains(PteFlags::U) {
            priv_mode == PrivilegeMode::User || (priv_mode == PrivilegeMode::Supervisor && self.sum)
        } else {
            priv_mode != PrivilegeMode::User
        };
        let ad_ok = f.contains(PteFlags::A)
            && (!matches!(op, MemOp::Store | MemOp::Amo) || f.contains(PteFlags::D));

        perm_ok && priv_ok && ad_ok
    }
}

// ---------------------------------------------------------------------------
// satp field extraction
// ---------------------------------------------------------------------------

fn satp_to_sv(satp: Word) -> Option<&'static SvMode> {
    #[cfg(isa32)]
    {
        if satp >> 31 == 1 {
            return Some(&SV32);
        }
    }
    #[cfg(isa64)]
    match satp >> 60 {
        8 => return Some(&SV39),
        9 => return Some(&SV48),
        10 => return Some(&SV57),
        _ => {}
    }
    None
}

fn satp_ppn(satp: Word) -> usize {
    #[cfg(isa64)]
    {
        (satp & ((1u64 << 44) - 1)) as usize
    }
    #[cfg(isa32)]
    {
        (satp & ((1u32 << 22) - 1)) as usize
    }
}

fn satp_asid(satp: Word) -> u16 {
    #[cfg(isa64)]
    {
        ((satp >> 44) & 0xFFFF) as u16
    }
    #[cfg(isa32)]
    {
        ((satp >> 22) & 0x1FF) as u16
    }
}

fn vpn_index(vaddr: VirtAddr, level: usize, sv: &SvMode) -> usize {
    (vaddr.as_usize() >> (PAGE_SHIFT + level * sv.vpn_bits)) & ((1 << sv.vpn_bits) - 1)
}

fn is_canonical(vaddr: VirtAddr, sv: &SvMode) -> bool {
    let va = vaddr.as_usize() as isize;
    let shift = usize::BITS as usize - sv.va_bits;
    (va << shift) >> shift == va
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use memory_addr::VirtAddr;

    use super::*;
    use crate::config::{CONFIG_MBASE, CONFIG_MSIZE};

    fn test_bus() -> Mutex<Bus> {
        Mutex::new(Bus::new(CONFIG_MBASE, CONFIG_MSIZE))
    }

    fn allow_all_pmp() -> Pmp {
        let mut pmp = Pmp::new();
        pmp.update_addr(1, usize::MAX >> 2);
        pmp.update_cfg(1, 0x0F); // TOR, R+W+X
        pmp
    }

    /// PTE helpers for readable test setup.
    fn ptr_pte(child_base: usize) -> usize {
        ((child_base >> PAGE_SHIFT) << 10) | 0x01
    }
    fn leaf_pte(ppn: usize, flags: usize) -> usize {
        (ppn << 10) | flags
    }

    const PTE_VRWXAD: usize = 0xCF; // V|R|W|X|A|D
    const PTE_VRAD: usize = 0xC3; // V|R|A|D (read-only)
    const PTE_VRWX: usize = 0x0F; // V|R|W|X (no A/D — Svade fault)

    /// Set up a full page table mapping vaddr 0x0 → target paddr.
    fn setup_page_table(bus: &mut MutexGuard<'_, Bus>, mmu: &mut Mmu, leaf_flags: usize) -> usize {
        let pt_base = CONFIG_MBASE + 0x1000;
        let target_paddr = CONFIG_MBASE + 0x2000;
        let target_ppn = target_paddr >> PAGE_SHIFT;

        #[cfg(isa64)]
        {
            let (l2, l1, l0) = (pt_base, CONFIG_MBASE + 0x3000, CONFIG_MBASE + 0x4000);
            bus.write(l2, 8, ptr_pte(l1) as Word).unwrap();
            bus.write(l1, 8, ptr_pte(l0) as Word).unwrap();
            bus.write(l0, 8, leaf_pte(target_ppn, leaf_flags) as Word)
                .unwrap();
            mmu.update_satp((8u64 << 60) | (l2 >> PAGE_SHIFT) as u64);
        }
        #[cfg(isa32)]
        {
            let (l1, l0) = (pt_base, CONFIG_MBASE + 0x3000);
            bus.write(l1, 4, ptr_pte(l0) as Word).unwrap();
            bus.write(l0, 4, leaf_pte(target_ppn, leaf_flags) as Word)
                .unwrap();
            mmu.update_satp(((1u32 << 31) | (l1 >> PAGE_SHIFT) as u32) as Word);
        }
        target_paddr
    }

    #[test]
    fn bare_mode_is_identity() {
        let mut mmu = Mmu::new();
        let pmp = Pmp::new();
        let bus = test_bus();
        let lock = bus.lock().unwrap();
        let r = mmu.translate(
            VirtAddr::from(0x8000_1234_usize),
            MemOp::Load,
            PrivilegeMode::Machine,
            &pmp,
            &lock,
        );
        assert_eq!(r.unwrap(), 0x8000_1234);
    }

    #[test]
    fn m_mode_identity_even_with_satp() {
        let mut mmu = Mmu::new();
        #[cfg(isa64)]
        mmu.update_satp(8 << 60);
        #[cfg(isa32)]
        mmu.update_satp(1 << 31);
        let pmp = Pmp::new();
        let bus = test_bus();
        let lock = bus.lock().unwrap();
        let r = mmu.translate(
            VirtAddr::from(CONFIG_MBASE),
            MemOp::Fetch,
            PrivilegeMode::Machine,
            &pmp,
            &lock,
        );
        assert_eq!(r.unwrap(), CONFIG_MBASE);
    }

    #[test]
    fn page_walk_translates_correctly() {
        let bus = test_bus();
        let mut lock = bus.lock().unwrap();
        let mut mmu = Mmu::new();
        let pmp = allow_all_pmp();
        let target = setup_page_table(&mut lock, &mut mmu, PTE_VRWXAD);
        let r = mmu.translate(
            VirtAddr::from(0x0_usize),
            MemOp::Load,
            PrivilegeMode::Supervisor,
            &pmp,
            &lock,
        );
        assert_eq!(r.unwrap(), target);
    }

    #[test]
    fn invalid_pte_faults() {
        let bus = test_bus();
        let lock = bus.lock().unwrap();
        let mut mmu = Mmu::new();
        let pmp = allow_all_pmp();
        #[cfg(isa64)]
        mmu.update_satp((8u64 << 60) | (CONFIG_MBASE >> PAGE_SHIFT) as u64);
        #[cfg(isa32)]
        mmu.update_satp(((1u32 << 31) | (CONFIG_MBASE >> PAGE_SHIFT) as u32) as Word);
        let r = mmu.translate(
            VirtAddr::from(0x0_usize),
            MemOp::Load,
            PrivilegeMode::Supervisor,
            &pmp,
            &lock,
        );
        assert!(matches!(r, Err(XError::PageFault)));
    }

    #[test]
    fn write_to_readonly_faults() {
        let bus = test_bus();
        let mut lock = bus.lock().unwrap();
        let mut mmu = Mmu::new();
        let pmp = allow_all_pmp();
        setup_page_table(&mut lock, &mut mmu, PTE_VRAD);

        assert!(
            mmu.translate(
                VirtAddr::from(0x0_usize),
                MemOp::Load,
                PrivilegeMode::Supervisor,
                &pmp,
                &lock
            )
            .is_ok()
        );
        assert!(matches!(
            mmu.translate(
                VirtAddr::from(0x0_usize),
                MemOp::Store,
                PrivilegeMode::Supervisor,
                &pmp,
                &lock
            ),
            Err(XError::PageFault)
        ));
    }

    #[test]
    fn svade_a_unset_faults() {
        let bus = test_bus();
        let mut lock = bus.lock().unwrap();
        let mut mmu = Mmu::new();
        let pmp = allow_all_pmp();
        setup_page_table(&mut lock, &mut mmu, PTE_VRWX);
        let r = mmu.translate(
            VirtAddr::from(0x0_usize),
            MemOp::Load,
            PrivilegeMode::Supervisor,
            &pmp,
            &lock,
        );
        assert!(matches!(r, Err(XError::PageFault)));
    }
}
