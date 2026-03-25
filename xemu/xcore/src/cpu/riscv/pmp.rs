use super::mmu::MemOp;
use crate::{cpu::riscv::csr::PrivilegeMode, error::XResult};

const PMP_COUNT: usize = 16;

#[allow(dead_code)]
#[derive(Clone, Copy, Default)]
struct PmpEntry {
    cfg: u8,
    addr: usize,
}

#[allow(dead_code)]
impl PmpEntry {
    fn is_off(self) -> bool {
        (self.cfg >> 3) & 3 == 0
    }
    fn locked(self) -> bool {
        self.cfg & 0x80 != 0
    }
    fn r(self) -> bool {
        self.cfg & 1 != 0
    }
    fn w(self) -> bool {
        self.cfg & 2 != 0
    }
    fn x(self) -> bool {
        self.cfg & 4 != 0
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

    /// Check physical address access. M-mode bypasses unless Locked.
    /// S/U-mode: first match wins; no match → deny.
    pub fn check(&self, _paddr: usize, _op: MemOp, priv_mode: PrivilegeMode) -> XResult {
        // M-mode bypasses PMP (unless locked entries exist — checked in Step 3)
        if priv_mode == PrivilegeMode::Machine {
            return Ok(());
        }

        // All entries are Off by default → no match → deny for S/U
        // Full matching will be implemented in Step 3.
        // For now: if any entry is configured, we'd check it.
        // Since all are Off in skeleton, and we only run M-mode currently,
        // this path won't be hit in practice.
        for entry in &self.entries {
            if !entry.is_off() {
                // TODO: implement TOR/NA4/NAPOT matching in Step 3
                continue;
            }
        }

        // No match in S/U-mode → access denied
        Err(crate::error::XError::BadAddress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn m_mode_bypasses_pmp() {
        let pmp = Pmp::new();
        assert!(
            pmp.check(0x8000_0000, MemOp::Load, PrivilegeMode::Machine)
                .is_ok()
        );
        assert!(
            pmp.check(0x8000_0000, MemOp::Store, PrivilegeMode::Machine)
                .is_ok()
        );
        assert!(
            pmp.check(0x8000_0000, MemOp::Fetch, PrivilegeMode::Machine)
                .is_ok()
        );
    }

    #[test]
    fn s_mode_denied_without_entries() {
        let pmp = Pmp::new();
        assert!(
            pmp.check(0x8000_0000, MemOp::Load, PrivilegeMode::Supervisor)
                .is_err()
        );
    }

    #[test]
    fn u_mode_denied_without_entries() {
        let pmp = Pmp::new();
        assert!(
            pmp.check(0x8000_0000, MemOp::Load, PrivilegeMode::User)
                .is_err()
        );
    }
}
