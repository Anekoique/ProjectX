use super::MemOp;
use crate::{
    cpu::riscv::csr::PrivilegeMode,
    ensure,
    error::{XError, XResult},
};

const PMP_COUNT: usize = 16;

bitflags::bitflags! {
    #[derive(Default, Debug, Clone, Copy)]
    struct PmpFlags: u8 {
        const R = 1 << 0;
        const W = 1 << 1;
        const X = 1 << 2;
        const L = 1 << 7;
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum AddrMatch {
    Off,
    Tor,
    Na4,
    Napot,
}

#[derive(Clone, Copy, Default)]
struct PmpEntry {
    cfg: u8,
    addr: usize,
}

impl PmpEntry {
    #[inline]
    fn match_mode(self) -> AddrMatch {
        match (self.cfg >> 3) & 3 {
            1 => AddrMatch::Tor,
            2 => AddrMatch::Na4,
            3 => AddrMatch::Napot,
            _ => AddrMatch::Off,
        }
    }

    #[inline]
    fn locked(self) -> bool {
        self.cfg & PmpFlags::L.bits() != 0
    }

    #[inline]
    fn permits(self, op: MemOp) -> bool {
        match op {
            MemOp::Fetch => self.cfg & PmpFlags::X.bits() != 0,
            MemOp::Load => self.cfg & PmpFlags::R.bits() != 0,
            MemOp::Store | MemOp::Amo => self.cfg & PmpFlags::W.bits() != 0,
        }
    }

    fn napot_range(self) -> (usize, usize) {
        let trailing = (!self.addr).trailing_zeros() as usize;
        let size = 1usize.wrapping_shl((trailing + 3) as u32);
        let mask = if trailing + 1 >= usize::BITS as usize {
            usize::MAX
        } else {
            (1usize << (trailing + 1)) - 1
        };
        ((self.addr & !mask) << 2, size)
    }

    /// Match result: does [paddr, paddr+size) overlap this entry?
    /// - `None` = no overlap → try next entry
    /// - `Some(true)` = fully contained → this entry decides
    /// - `Some(false)` = partial overlap → immediate fail (spec §3.7.1)
    #[inline]
    fn overlap(self, paddr: usize, size: usize, prev_addr: usize) -> Option<bool> {
        let (acc_lo, acc_hi) = (paddr, paddr + size);
        let (rgn_lo, rgn_hi) = match self.match_mode() {
            AddrMatch::Off => return None,
            AddrMatch::Tor => (prev_addr << 2, self.addr << 2),
            AddrMatch::Na4 => {
                let b = self.addr << 2;
                (b, b + 4)
            }
            AddrMatch::Napot => {
                let (b, s) = self.napot_range();
                (b, b + s)
            }
        };
        // Empty/inverted range (e.g. TOR with prev >= cur) matches nothing
        if rgn_hi <= rgn_lo {
            return None;
        }
        if acc_hi <= rgn_lo || acc_lo >= rgn_hi {
            None // no overlap
        } else {
            Some(acc_lo >= rgn_lo && acc_hi <= rgn_hi) // true = full, false = partial
        }
    }
}

pub struct Pmp {
    entries: [PmpEntry; PMP_COUNT],
}

impl Pmp {
    pub fn new() -> Self {
        Self {
            entries: [PmpEntry::default(); PMP_COUNT],
        }
    }

    pub fn get_cfg(&self, index: usize) -> u8 {
        if index < PMP_COUNT {
            self.entries[index].cfg
        } else {
            0
        }
    }

    pub fn get_addr(&self, index: usize) -> usize {
        if index < PMP_COUNT {
            self.entries[index].addr
        } else {
            0
        }
    }

    /// Update cfg byte. Ignores write if entry is locked (spec §3.7.1).
    pub fn update_cfg(&mut self, index: usize, cfg: u8) {
        if index < PMP_COUNT && !self.entries[index].locked() {
            self.entries[index].cfg = cfg;
        }
    }

    /// Update addr. Ignores write if entry is locked, or if next entry
    /// is locked+TOR (spec §3.7.1: TOR locks pmpaddr[i-1]).
    pub fn update_addr(&mut self, index: usize, addr: usize) {
        if index >= PMP_COUNT || self.entries[index].locked() {
            return;
        }

        let next_locked_tor = self
            .entries
            .get(index + 1)
            .is_some_and(|next| next.locked() && next.match_mode() == AddrMatch::Tor);

        if !next_locked_tor {
            self.entries[index].addr = addr;
        }
    }

    /// Check [paddr, paddr+size) access.
    /// M-mode: bypass unless Locked. S/U-mode: first match wins, no match →
    /// deny.
    pub fn check(&self, paddr: usize, size: usize, op: MemOp, priv_mode: PrivilegeMode) -> XResult {
        let mut prev_addr: usize = 0;
        for entry in &self.entries {
            if entry.match_mode() == AddrMatch::Off {
                prev_addr = entry.addr;
                continue;
            }
            match entry.overlap(paddr, size, prev_addr) {
                None => {
                    prev_addr = entry.addr;
                    continue;
                } // no overlap
                Some(false) => return Err(XError::BadAddress), // partial overlap → fail
                Some(true) => {
                    if priv_mode == PrivilegeMode::Machine {
                        ensure!(!entry.locked() || entry.permits(op), XError::BadAddress);
                        return Ok(());
                    }
                    ensure!(entry.permits(op), XError::BadAddress);
                    return Ok(());
                }
            }
        }
        (priv_mode == PrivilegeMode::Machine).ok_or(XError::BadAddress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m_mode_bypasses_pmp() {
        let pmp = Pmp::new();
        for op in [MemOp::Load, MemOp::Store, MemOp::Fetch] {
            assert!(
                pmp.check(0x8000_0000, 4, op, PrivilegeMode::Machine)
                    .is_ok()
            );
        }
    }

    #[test]
    fn s_u_denied_without_entries() {
        let pmp = Pmp::new();
        assert!(
            pmp.check(0x8000_0000, 4, MemOp::Load, PrivilegeMode::Supervisor)
                .is_err()
        );
        assert!(
            pmp.check(0x8000_0000, 4, MemOp::Load, PrivilegeMode::User)
                .is_err()
        );
    }

    #[test]
    fn napot_region() {
        let mut pmp = Pmp::new();
        pmp.update_addr(0, 0x20FF_FFFF); // 128MB at 0x8000_0000
        pmp.update_cfg(0, 0x1F); // NAPOT, R+W+X
        assert!(
            pmp.check(0x8000_0000, 4, MemOp::Load, PrivilegeMode::Supervisor)
                .is_ok()
        );
        assert!(
            pmp.check(0x8400_0000, 4, MemOp::Store, PrivilegeMode::Supervisor)
                .is_ok()
        );
        assert!(
            pmp.check(0x8800_0000, 4, MemOp::Load, PrivilegeMode::Supervisor)
                .is_err()
        );
    }

    #[test]
    fn tor_region() {
        let mut pmp = Pmp::new();
        pmp.update_addr(1, 0x1000_0000 >> 2);
        pmp.update_cfg(1, 0x0B); // TOR, R+W (no X)
        assert!(
            pmp.check(0x0, 4, MemOp::Load, PrivilegeMode::Supervisor)
                .is_ok()
        );
        assert!(
            pmp.check(0x0FFF_FFFC, 4, MemOp::Store, PrivilegeMode::Supervisor)
                .is_ok()
        );
        assert!(
            pmp.check(0x1000_0000, 4, MemOp::Load, PrivilegeMode::Supervisor)
                .is_err()
        );
        assert!(
            pmp.check(0x0, 4, MemOp::Fetch, PrivilegeMode::Supervisor)
                .is_err()
        );
    }

    #[test]
    fn locked_enforced_in_m_mode() {
        let mut pmp = Pmp::new();
        pmp.update_addr(0, 0x20FF_FFFF);
        pmp.update_cfg(0, 0x99); // L=1, NAPOT, R only
        assert!(
            pmp.check(0x8000_0000, 4, MemOp::Load, PrivilegeMode::Machine)
                .is_ok()
        );
        assert!(
            pmp.check(0x8000_0000, 4, MemOp::Store, PrivilegeMode::Machine)
                .is_err()
        );
    }

    #[test]
    fn locked_entry_ignores_writes() {
        let mut pmp = Pmp::new();
        pmp.update_addr(0, 0x20FF_FFFF);
        pmp.update_cfg(0, 0x99); // L=1, NAPOT, R only
        // Try to change — should be ignored
        pmp.update_cfg(0, 0x1F);
        pmp.update_addr(0, 0);
        // Still locked R-only at original address
        assert!(
            pmp.check(0x8000_0000, 4, MemOp::Load, PrivilegeMode::Machine)
                .is_ok()
        );
        assert!(
            pmp.check(0x8000_0000, 4, MemOp::Store, PrivilegeMode::Machine)
                .is_err()
        );
    }

    #[test]
    fn tor_locks_prev_addr() {
        let mut pmp = Pmp::new();
        pmp.update_addr(0, 0x1000);
        pmp.update_addr(1, 0x2000);
        pmp.update_cfg(1, 0x88 | 0x01); // L=1, TOR, R
        // Entry 1 is locked+TOR → pmpaddr[0] should be locked too
        pmp.update_addr(0, 0xFFFF); // should be ignored
        assert_eq!(pmp.entries[0].addr, 0x1000);
    }

    #[test]
    fn cross_boundary_access_denied() {
        let mut pmp = Pmp::new();
        // TOR region [0, 0x100)
        pmp.update_addr(1, 0x100 >> 2);
        pmp.update_cfg(1, 0x0F); // TOR, R+W+X
        // 4-byte access at 0xFE: [0xFE, 0x102) crosses boundary
        assert!(
            pmp.check(0xFE, 4, MemOp::Load, PrivilegeMode::Supervisor)
                .is_err()
        );
        // 4-byte at 0xFC: [0xFC, 0x100) fits
        assert!(
            pmp.check(0xFC, 4, MemOp::Load, PrivilegeMode::Supervisor)
                .is_ok()
        );
    }

    #[test]
    fn partial_overlap_fails_even_if_lower_entry_covers() {
        let mut pmp = Pmp::new();
        // entry0: TOR [0, 0x100) R+W
        pmp.update_addr(1, 0x100 >> 2);
        pmp.update_cfg(1, 0x0B); // TOR, R+W
        // entry1: TOR [0, 0x200) R+W+X
        pmp.update_addr(2, 0x200 >> 2);
        pmp.update_cfg(2, 0x0F); // TOR, R+W+X
        // 8-byte access at 0xFC: partially in entry0 [0,0x100), rest in entry1
        // entry0 matches some bytes → partial overlap → must fail
        assert!(
            pmp.check(0xFC, 8, MemOp::Load, PrivilegeMode::Supervisor)
                .is_err()
        );
        // 4-byte at 0xFC: fully in entry0 → ok
        assert!(
            pmp.check(0xFC, 4, MemOp::Load, PrivilegeMode::Supervisor)
                .is_ok()
        );
    }

    #[test]
    fn empty_tor_range_matches_nothing() {
        let mut pmp = Pmp::new();
        // entry0: Off, addr=0x200 (sets prev_addr for entry1)
        pmp.update_addr(0, 0x200 >> 2);
        // entry1: TOR with prev(0x200) >= cur(0x100) → empty range, matches nothing
        pmp.update_addr(1, 0x100 >> 2);
        pmp.update_cfg(1, 0x0B); // TOR, R+W
        // entry2: TOR [0, 0x1000) R+W+X — should be reached since entry1 is empty
        pmp.update_addr(2, 0x1000 >> 2);
        pmp.update_cfg(2, 0x0F); // TOR, R+W+X
        // Access at 0x150 should fall through empty entry1 and match entry2
        assert!(
            pmp.check(0x150, 4, MemOp::Load, PrivilegeMode::Supervisor)
                .is_ok()
        );
    }
}
