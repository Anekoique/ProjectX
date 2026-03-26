use memory_addr::VirtAddr;

use super::{PAGE_SHIFT, PAGE_SIZE, PteFlags, SvConfig};

const TLB_SIZE: usize = 64;

#[derive(Clone, Copy, Default)]
pub(in crate::cpu::riscv) struct TlbEntry {
    vpn: usize,
    ppn: usize,
    asid: u16,
    level: u8,
    valid: bool,
    pub(super) flags: PteFlags,
}

impl TlbEntry {
    pub(super) fn new(
        pte_flags: PteFlags,
        ppn: usize,
        vaddr: VirtAddr,
        level: usize,
        asid: u16,
    ) -> Self {
        Self {
            vpn: vaddr.as_usize() >> PAGE_SHIFT,
            ppn,
            asid,
            flags: pte_flags,
            level: level as u8,
            valid: true,
        }
    }

    #[inline]
    pub(super) fn translate(&self, vaddr: VirtAddr, sv: &SvConfig) -> usize {
        if self.level > 0 {
            let mask = (1 << (self.level as usize * sv.vpn_bits + PAGE_SHIFT)) - 1;
            (self.ppn << PAGE_SHIFT) & !mask | (vaddr.as_usize() & mask)
        } else {
            self.ppn << PAGE_SHIFT | (vaddr.as_usize() & (PAGE_SIZE - 1))
        }
    }

    #[inline]
    fn is_global(&self) -> bool {
        self.flags.contains(PteFlags::G)
    }

    #[inline]
    pub(super) fn matches(&self, vpn: usize, asid: u16) -> bool {
        self.valid && self.vpn == vpn && (self.asid == asid || self.is_global())
    }
}

pub(in crate::cpu::riscv) struct Tlb {
    entries: Vec<TlbEntry>,
}

impl Tlb {
    pub fn new() -> Self {
        Self {
            entries: vec![TlbEntry::default(); TLB_SIZE],
        }
    }

    #[inline]
    pub(super) fn get(&self, vpn: usize) -> &TlbEntry {
        &self.entries[vpn & (TLB_SIZE - 1)]
    }

    pub(super) fn insert(&mut self, entry: TlbEntry) {
        self.entries[entry.vpn & (TLB_SIZE - 1)] = entry;
    }

    pub fn flush(&mut self, vpn: Option<usize>, asid: Option<u16>) {
        self.entries
            .iter_mut()
            .filter(|e| {
                e.valid
                    && match (vpn, asid) {
                        (None, None) => true,
                        (Some(v), None) => e.vpn == v,
                        (None, Some(a)) => !e.is_global() && e.asid == a,
                        (Some(v), Some(a)) => !e.is_global() && e.vpn == v && e.asid == a,
                    }
            })
            .for_each(|e| e.valid = false);
    }
}
