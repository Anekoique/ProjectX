//! MMU: multi-level page table walker with hardware A/D bit updates.

use memory_addr::VirtAddr;

use super::{
    MemOp, PAGE_SHIFT, PAGE_SIZE, Pte, PteFlags, Satp, SvConfig, SvMode,
    pmp::Pmp,
    tlb::{Tlb, TlbEntry},
};
use crate::{
    arch::riscv::cpu::csr::PrivilegeMode,
    config::Word,
    device::bus::Bus,
    ensure,
    error::{XError, XResult},
};

/// Memory Management Unit: satp-driven address translation with TLB cache.
pub struct Mmu {
    pub(in crate::arch::riscv) tlb: Tlb,
    sv: SvMode,
    ppn: usize,
    asid: u16,
    sum: bool,
    mxr: bool,
}

impl Mmu {
    /// Create an MMU in bare (identity-mapped) mode.
    pub fn new() -> Self {
        Self {
            tlb: Tlb::new(),
            sv: SvMode::Bare,
            ppn: 0,
            asid: 0,
            sum: false,
            mxr: false,
        }
    }

    /// Update translation mode and root page table from satp.
    pub fn update_satp(&mut self, raw: Word) {
        let satp = Satp::parse(raw);
        self.sv = satp.mode;
        self.ppn = satp.ppn;
        self.asid = satp.asid;
        self.tlb.flush(None, None);
    }

    /// Update SUM and MXR flags from mstatus.
    pub fn update_mstatus(&mut self, sum: bool, mxr: bool) {
        self.sum = sum;
        self.mxr = mxr;
    }

    /// Translate a virtual address, consulting TLB then page walker.
    pub fn translate(
        &mut self,
        vaddr: VirtAddr,
        op: MemOp,
        priv_mode: PrivilegeMode,
        pmp: &Pmp,
        bus: &mut Bus,
    ) -> XResult<usize> {
        if self.sv == SvMode::Bare || priv_mode == PrivilegeMode::Machine {
            return Ok(vaddr.as_usize());
        }
        let sv = self.sv.config();
        let vpn = vaddr.as_usize() >> PAGE_SHIFT;

        let entry = self.tlb.get(vpn);
        if entry.matches(vpn, self.asid) && entry.flags.permits(op, priv_mode, self.sum, self.mxr) {
            return Ok(entry.translate(vaddr, &sv));
        }

        let entry = self.page_walk(vaddr, op, priv_mode, &sv, pmp, bus)?;
        let paddr = entry.translate(vaddr, &sv);
        self.tlb.insert(entry);
        Ok(paddr)
    }

    fn page_walk(
        &self,
        vaddr: VirtAddr,
        op: MemOp,
        priv_mode: PrivilegeMode,
        sv: &SvConfig,
        pmp: &Pmp,
        bus: &mut Bus,
    ) -> XResult<TlbEntry> {
        ensure!(is_canonical(vaddr, sv), XError::PageFault);

        let mut base = self.ppn * PAGE_SIZE;
        for level in (0..sv.levels).rev() {
            let pte_addr = base + vpn_index(vaddr, level, sv) * sv.pte_size;
            pmp.check(
                pte_addr,
                sv.pte_size,
                MemOp::Load,
                PrivilegeMode::Supervisor,
            )?;
            let pte = bus
                .read_ram(pte_addr, sv.pte_size)
                .map(|w| Pte(w as usize))?;

            ensure!(
                pte.is_valid() && !pte.is_reserved() && !pte.has_high_reserved_bits(sv),
                XError::PageFault
            );
            if pte.is_leaf() {
                ensure!(
                    pte.superpage_aligned(level, sv)
                        && pte.flags().permits(op, priv_mode, self.sum, self.mxr),
                    XError::PageFault
                );
                // Hardware A/D update (§4.3.1): set A, and D for stores/AMOs.
                let ad = PteFlags::A
                    | if matches!(op, MemOp::Store | MemOp::Amo) {
                        PteFlags::D
                    } else {
                        PteFlags::empty()
                    };
                if !pte.flags().contains(ad) {
                    let _ = bus.write(pte_addr, sv.pte_size, (pte.0 | ad.bits()) as Word);
                }
                return Ok(TlbEntry::new(
                    pte.flags() | ad,
                    pte.ppn(sv),
                    vaddr,
                    level,
                    self.asid,
                ));
            }
            ensure!(!pte.has_nonleaf_reserved_bits(), XError::PageFault);
            base = pte.ppn(sv) * PAGE_SIZE;
        }
        Err(XError::PageFault)
    }
}

#[inline]
fn vpn_index(vaddr: VirtAddr, level: usize, sv: &SvConfig) -> usize {
    (vaddr.as_usize() >> (PAGE_SHIFT + level * sv.vpn_bits)) & ((1 << sv.vpn_bits) - 1)
}

#[inline]
fn is_canonical(vaddr: VirtAddr, sv: &SvConfig) -> bool {
    let va = vaddr.as_usize() as isize;
    let shift = usize::BITS as usize - sv.va_bits;
    (va << shift) >> shift == va
}

#[cfg(test)]
mod tests {
    use memory_addr::VirtAddr;

    use super::*;
    use crate::config::{CONFIG_MBASE, CONFIG_MSIZE, Word};

    fn test_bus() -> Bus {
        Bus::new(CONFIG_MBASE, CONFIG_MSIZE)
    }

    fn allow_all_pmp() -> Pmp {
        let mut pmp = Pmp::new();
        pmp.update_addr(1, usize::MAX >> 2);
        pmp.update_cfg(1, 0x0F);
        pmp
    }

    fn ptr_pte(child: usize) -> usize {
        ((child >> PAGE_SHIFT) << 10) | 0x01
    }
    fn leaf_pte(ppn: usize, flags: usize) -> usize {
        (ppn << 10) | flags
    }

    const PTE_VRWXAD: usize = 0xCF;
    const PTE_VRAD: usize = 0xC3;
    const PTE_VRWX: usize = 0x0F;

    fn setup_pt(bus: &mut Bus, mmu: &mut Mmu, flags: usize) -> usize {
        let (pt, target) = (CONFIG_MBASE + 0x1000, CONFIG_MBASE + 0x2000);
        #[cfg(isa64)]
        {
            let (l2, l1, l0) = (pt, CONFIG_MBASE + 0x3000, CONFIG_MBASE + 0x4000);
            bus.write(l2, 8, ptr_pte(l1) as Word).unwrap();
            bus.write(l1, 8, ptr_pte(l0) as Word).unwrap();
            bus.write(l0, 8, leaf_pte(target >> PAGE_SHIFT, flags) as Word)
                .unwrap();
            mmu.update_satp((8u64 << 60) | (l2 >> PAGE_SHIFT) as u64);
        }
        #[cfg(isa32)]
        {
            let (l1, l0) = (pt, CONFIG_MBASE + 0x3000);
            bus.write(l1, 4, ptr_pte(l0) as Word).unwrap();
            bus.write(l0, 4, leaf_pte(target >> PAGE_SHIFT, flags) as Word)
                .unwrap();
            mmu.update_satp(((1u32 << 31) | (l1 >> PAGE_SHIFT) as u32) as Word);
        }
        target
    }

    fn zap_root(bus: &mut Bus) {
        let root = CONFIG_MBASE + 0x1000;
        #[cfg(isa64)]
        bus.write(root, 8, 0).unwrap();
        #[cfg(isa32)]
        bus.write(root, 4, 0).unwrap();
    }

    fn xlate(mmu: &mut Mmu, va: usize, op: MemOp, pmp: &Pmp, bus: &mut Bus) -> XResult<usize> {
        mmu.translate(VirtAddr::from(va), op, PrivilegeMode::Supervisor, pmp, bus)
    }

    #[test]
    fn bare_mode_identity() {
        let mut mmu = Mmu::new();
        let mut bus = test_bus();
        assert_eq!(
            mmu.translate(
                VirtAddr::from(0x8000_1234_usize),
                MemOp::Load,
                PrivilegeMode::Machine,
                &Pmp::new(),
                &mut bus
            )
            .unwrap(),
            0x8000_1234
        );
    }

    #[test]
    fn m_mode_identity_with_satp() {
        let mut mmu = Mmu::new();
        #[cfg(isa64)]
        mmu.update_satp(8 << 60);
        #[cfg(isa32)]
        mmu.update_satp(1 << 31);
        let mut bus = test_bus();
        assert_eq!(
            mmu.translate(
                VirtAddr::from(CONFIG_MBASE),
                MemOp::Fetch,
                PrivilegeMode::Machine,
                &Pmp::new(),
                &mut bus
            )
            .unwrap(),
            CONFIG_MBASE
        );
    }

    #[test]
    fn page_walk_ok() {
        let mut bus = test_bus();
        let mut mmu = Mmu::new();
        let pmp = allow_all_pmp();
        let target = setup_pt(&mut bus, &mut mmu, PTE_VRWXAD);
        assert_eq!(
            xlate(&mut mmu, 0, MemOp::Load, &pmp, &mut bus).unwrap(),
            target
        );
    }

    #[test]
    fn invalid_pte_faults() {
        let mut bus = test_bus();
        let mut mmu = Mmu::new();
        #[cfg(isa64)]
        mmu.update_satp((8u64 << 60) | (CONFIG_MBASE >> PAGE_SHIFT) as u64);
        #[cfg(isa32)]
        mmu.update_satp(((1u32 << 31) | (CONFIG_MBASE >> PAGE_SHIFT) as u32) as Word);
        assert!(matches!(
            xlate(&mut mmu, 0, MemOp::Load, &allow_all_pmp(), &mut bus),
            Err(XError::PageFault)
        ));
    }

    #[test]
    fn write_to_readonly_faults() {
        let mut bus = test_bus();
        let mut mmu = Mmu::new();
        let pmp = allow_all_pmp();
        setup_pt(&mut bus, &mut mmu, PTE_VRAD);
        assert!(xlate(&mut mmu, 0, MemOp::Load, &pmp, &mut bus).is_ok());
        assert!(matches!(
            xlate(&mut mmu, 0, MemOp::Store, &pmp, &mut bus),
            Err(XError::PageFault)
        ));
    }

    #[test]
    fn hw_ad_update_sets_bits() {
        let mut bus = test_bus();
        let mut mmu = Mmu::new();
        let pmp = allow_all_pmp();
        setup_pt(&mut bus, &mut mmu, PTE_VRWX);
        assert!(xlate(&mut mmu, 0, MemOp::Load, &pmp, &mut bus).is_ok());
    }

    #[test]
    fn tlb_caches() {
        let mut bus = test_bus();
        let mut mmu = Mmu::new();
        let pmp = allow_all_pmp();
        let target = setup_pt(&mut bus, &mut mmu, PTE_VRWXAD);
        assert_eq!(
            xlate(&mut mmu, 0, MemOp::Load, &pmp, &mut bus).unwrap(),
            target
        );
        zap_root(&mut bus);
        assert_eq!(
            xlate(&mut mmu, 0, MemOp::Load, &pmp, &mut bus).unwrap(),
            target
        );
    }

    #[test]
    fn tlb_flush_invalidates() {
        let mut bus = test_bus();
        let mut mmu = Mmu::new();
        let pmp = allow_all_pmp();
        setup_pt(&mut bus, &mut mmu, PTE_VRWXAD);
        xlate(&mut mmu, 0, MemOp::Load, &pmp, &mut bus).unwrap();
        zap_root(&mut bus);
        mmu.tlb.flush(None, None);
        assert!(matches!(
            xlate(&mut mmu, 0, MemOp::Load, &pmp, &mut bus),
            Err(XError::PageFault)
        ));
    }
}
