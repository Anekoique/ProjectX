use super::mmu::MemOp;
use crate::{
    cpu::riscv::csr::PrivilegeMode,
    error::{XError, XResult},
};

const PMP_COUNT: usize = 16;

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
    fn match_mode(self) -> AddrMatch {
        match (self.cfg >> 3) & 3 {
            1 => AddrMatch::Tor,
            2 => AddrMatch::Na4,
            3 => AddrMatch::Napot,
            _ => AddrMatch::Off,
        }
    }
    fn locked(self) -> bool {
        self.cfg & 0x80 != 0
    }
    fn permits(self, op: MemOp) -> bool {
        match op {
            MemOp::Fetch => self.cfg & 4 != 0,
            MemOp::Load => self.cfg & 1 != 0,
            MemOp::Store | MemOp::Amo => self.cfg & 2 != 0,
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

    pub fn update_cfg(&mut self, index: usize, cfg: u8) {
        if index < PMP_COUNT {
            self.entries[index].cfg = cfg;
        }
    }

    pub fn update_addr(&mut self, index: usize, addr: usize) {
        if index < PMP_COUNT {
            self.entries[index].addr = addr;
        }
    }

    /// Check physical address access.
    /// M-mode: bypass unless Locked. S/U-mode: first match wins, no match →
    /// deny.
    pub fn check(&self, paddr: usize, op: MemOp, priv_mode: PrivilegeMode) -> XResult {
        let mut prev_addr: usize = 0;

        for entry in &self.entries {
            let matched = match entry.match_mode() {
                AddrMatch::Off => {
                    prev_addr = entry.addr;
                    continue;
                }
                AddrMatch::Tor => {
                    let (lo, hi) = (prev_addr << 2, entry.addr << 2);
                    paddr >= lo && paddr < hi
                }
                AddrMatch::Na4 => {
                    let base = entry.addr << 2;
                    paddr >= base && paddr < base + 4
                }
                AddrMatch::Napot => {
                    let (base, size) = entry.napot_range();
                    paddr >= base && paddr < base + size
                }
            };
            prev_addr = entry.addr;
            if !matched {
                continue;
            }

            if priv_mode == PrivilegeMode::Machine {
                return if entry.locked() && !entry.permits(op) {
                    Err(XError::BadAddress)
                } else {
                    Ok(())
                };
            }
            return if entry.permits(op) {
                Ok(())
            } else {
                Err(XError::BadAddress)
            };
        }

        if priv_mode == PrivilegeMode::Machine {
            Ok(())
        } else {
            Err(XError::BadAddress)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m_mode_bypasses_pmp() {
        let pmp = Pmp::new();
        for op in [MemOp::Load, MemOp::Store, MemOp::Fetch] {
            assert!(pmp.check(0x8000_0000, op, PrivilegeMode::Machine).is_ok());
        }
    }

    #[test]
    fn s_u_denied_without_entries() {
        let pmp = Pmp::new();
        assert!(
            pmp.check(0x8000_0000, MemOp::Load, PrivilegeMode::Supervisor)
                .is_err()
        );
        assert!(
            pmp.check(0x8000_0000, MemOp::Load, PrivilegeMode::User)
                .is_err()
        );
    }

    #[test]
    fn napot_region() {
        let mut pmp = Pmp::new();
        // 128MB at 0x8000_0000: pmpaddr = 0x2000_0000 | 0x00FF_FFFF
        pmp.update_addr(0, 0x20FF_FFFF);
        pmp.update_cfg(0, 0x1F); // NAPOT, R+W+X
        assert!(
            pmp.check(0x8000_0000, MemOp::Load, PrivilegeMode::Supervisor)
                .is_ok()
        );
        assert!(
            pmp.check(0x8400_0000, MemOp::Store, PrivilegeMode::Supervisor)
                .is_ok()
        );
        assert!(
            pmp.check(0x8800_0000, MemOp::Load, PrivilegeMode::Supervisor)
                .is_err()
        );
    }

    #[test]
    fn tor_region() {
        let mut pmp = Pmp::new();
        pmp.update_addr(1, 0x1000_0000 >> 2);
        pmp.update_cfg(1, 0x0B); // TOR, R+W (no X)
        assert!(
            pmp.check(0x0, MemOp::Load, PrivilegeMode::Supervisor)
                .is_ok()
        );
        assert!(
            pmp.check(0x0FFF_FFFF, MemOp::Store, PrivilegeMode::Supervisor)
                .is_ok()
        );
        assert!(
            pmp.check(0x1000_0000, MemOp::Load, PrivilegeMode::Supervisor)
                .is_err()
        );
        assert!(
            pmp.check(0x0, MemOp::Fetch, PrivilegeMode::Supervisor)
                .is_err()
        );
    }

    #[test]
    fn locked_enforced_in_m_mode() {
        let mut pmp = Pmp::new();
        pmp.update_addr(0, 0x20FF_FFFF);
        pmp.update_cfg(0, 0x99); // L=1, NAPOT, R only
        assert!(
            pmp.check(0x8000_0000, MemOp::Load, PrivilegeMode::Machine)
                .is_ok()
        );
        assert!(
            pmp.check(0x8000_0000, MemOp::Store, PrivilegeMode::Machine)
                .is_err()
        );
    }
}
