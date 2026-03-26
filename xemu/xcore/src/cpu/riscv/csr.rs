mod mstatus;
mod ops;
mod privilege;

pub use mstatus::MStatus;
pub use privilege::PrivilegeMode;

// Re-export trap types for convenience (used by csr/ops and external consumers)
pub use super::trap::{Exception, TrapCause};
use crate::config::Word;

// ---------------------------------------------------------------------------
// AccessRule — per-CSR dynamic access rules
// ---------------------------------------------------------------------------

#[derive(Clone, Copy, Debug)]
pub enum AccessRule {
    /// Privilege level from addr bits [9:8] only
    Standard,
    /// Blocked when a specific mstatus flag is set and privilege == Supervisor
    BlockedByMstatus(MStatus),
    /// Gated by mcounteren / scounteren
    CounterGated,
}

// ---------------------------------------------------------------------------
// CsrDesc — descriptor for a single CSR
// ---------------------------------------------------------------------------

#[derive(Clone, Copy)]
pub struct CsrDesc {
    pub wmask: Word,
    pub storage: u16,
    pub view_mask: Word,
    pub access: AccessRule,
}

/// Extract the counteren bit index for a counter CSR address.
pub(super) fn counteren_bit(addr: u16) -> u32 {
    (addr & 0x1F) as u32
}

// ---------------------------------------------------------------------------
// csr_table! macro — generates CsrAddr enum + find_desc match
// ---------------------------------------------------------------------------

macro_rules! csr_table {
    ( $( $name:ident = $addr:expr => [ $($spec:tt)* ] ),* $(,)? ) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        #[repr(u16)]
        #[allow(non_camel_case_types, dead_code)]
        pub enum CsrAddr {
            $( $name = $addr ),*
        }

        pub(in crate::cpu::riscv) fn find_desc(addr: u16) -> Option<CsrDesc> {
            match addr {
                $( $addr => Some(csr_table!(@desc $addr, $($spec)*)), )*
                _ => None,
            }
        }
    };

    // RW(wmask) — normal register
    (@desc $addr:expr, RW($wmask:expr)) => {
        CsrDesc { wmask: $wmask, storage: $addr, view_mask: !0, access: AccessRule::Standard }
    };
    // RW(wmask) => alias(vmask) — shadow register
    (@desc $addr:expr, RW($wmask:expr) => $alias:ident($vmask:expr)) => {
        CsrDesc { wmask: $wmask, storage: CsrAddr::$alias as u16, view_mask: $vmask, access: AccessRule::Standard }
    };
    // RO — read-only register
    (@desc $addr:expr, RO) => {
        CsrDesc { wmask: 0, storage: $addr, view_mask: !0, access: AccessRule::Standard }
    };
    // RW + blocked_by(flag)
    (@desc $addr:expr, RW($wmask:expr), blocked_by($flag:ident)) => {
        CsrDesc { wmask: $wmask, storage: $addr, view_mask: !0, access: AccessRule::BlockedByMstatus(MStatus::$flag) }
    };
    // RO + counter_gated
    (@desc $addr:expr, RO, counter_gated) => {
        CsrDesc { wmask: 0, storage: $addr, view_mask: !0, access: AccessRule::CounterGated }
    };
}

// ---------------------------------------------------------------------------
// CSR Registration Table
// ---------------------------------------------------------------------------

const MSTATUS_WMASK: Word = MStatus::WRITABLE.bits();
const SSTATUS_VMASK: Word = MStatus::SSTATUS.bits();
const SSTATUS_WMASK: Word = SSTATUS_VMASK & !MStatus::SD.bits();
const MIE_WMASK: Word = (1 << 1) | (1 << 3) | (1 << 5) | (1 << 7) | (1 << 9) | (1 << 11);
const MIP_WMASK: Word = (1 << 1) | (1 << 3); // Only SSIP, MSIP software-writable
const SIE_MASK: Word = (1 << 1) | (1 << 5) | (1 << 9); // SSI, STI, SEI
const SIP_MASK: Word = 1 << 1; // Only SSIP writable from S-mode

csr_table! {
    // ---- M-mode Trap Setup ----
    mstatus    = 0x300 => [RW(MSTATUS_WMASK)],
    misa       = 0x301 => [RO],
    medeleg    = 0x302 => [RW(0xB3FF)],
    mideleg    = 0x303 => [RW(0x222)],
    mie        = 0x304 => [RW(MIE_WMASK)],
    mtvec      = 0x305 => [RW(!(0x2 as Word))],
    mcounteren = 0x306 => [RW(0x7)],

    // ---- M-mode Trap Handling ----
    mscratch   = 0x340 => [RW(!0)],
    mepc       = 0x341 => [RW(!(0x1 as Word))],
    mcause     = 0x342 => [RW(!0)],
    mtval      = 0x343 => [RW(!0)],
    mip        = 0x344 => [RW(MIP_WMASK)],

    // ---- S-mode Shadows ----
    sstatus    = 0x100 => [RW(SSTATUS_WMASK) => mstatus(SSTATUS_VMASK)],
    sie        = 0x104 => [RW(SIE_MASK) => mie(SIE_MASK)],
    sip        = 0x144 => [RW(SIP_MASK) => mip(SIP_MASK)],

    // ---- S-mode Own Registers ----
    stvec      = 0x105 => [RW(!(0x2 as Word))],
    scounteren = 0x106 => [RW(0x7)],
    sscratch   = 0x140 => [RW(!0)],
    sepc       = 0x141 => [RW(!(0x1 as Word))],
    scause     = 0x142 => [RW(!0)],
    stval      = 0x143 => [RW(!0)],
    satp       = 0x180 => [RW(!0), blocked_by(TVM)],

    // ---- PMP ----
    pmpcfg0    = 0x3A0 => [RW(!0)],
    pmpcfg1    = 0x3A1 => [RW(!0)],
    pmpcfg2    = 0x3A2 => [RW(!0)],
    pmpcfg3    = 0x3A3 => [RW(!0)],
    pmpaddr0   = 0x3B0 => [RW(!0)],
    pmpaddr1   = 0x3B1 => [RW(!0)],
    pmpaddr2   = 0x3B2 => [RW(!0)],
    pmpaddr3   = 0x3B3 => [RW(!0)],
    pmpaddr4   = 0x3B4 => [RW(!0)],
    pmpaddr5   = 0x3B5 => [RW(!0)],
    pmpaddr6   = 0x3B6 => [RW(!0)],
    pmpaddr7   = 0x3B7 => [RW(!0)],
    pmpaddr8   = 0x3B8 => [RW(!0)],
    pmpaddr9   = 0x3B9 => [RW(!0)],
    pmpaddr10  = 0x3BA => [RW(!0)],
    pmpaddr11  = 0x3BB => [RW(!0)],
    pmpaddr12  = 0x3BC => [RW(!0)],
    pmpaddr13  = 0x3BD => [RW(!0)],
    pmpaddr14  = 0x3BE => [RW(!0)],
    pmpaddr15  = 0x3BF => [RW(!0)],

    // ---- Machine Information (read-only) ----
    mvendorid  = 0xF11 => [RO],
    marchid    = 0xF12 => [RO],
    mimpid     = 0xF13 => [RO],
    mhartid    = 0xF14 => [RO],

    // ---- Counters ----
    cycle      = 0xC00 => [RO, counter_gated],
    instret    = 0xC02 => [RO, counter_gated],
}

// ---------------------------------------------------------------------------
// CsrFile — dumb storage + descriptor-driven read/write
// ---------------------------------------------------------------------------

pub struct CsrFile {
    regs: [Word; 4096],
}

impl Default for CsrFile {
    fn default() -> Self {
        Self { regs: [0; 4096] }
    }
}

impl CsrFile {
    pub fn new() -> Self {
        Self::default()
    }

    /// Raw get by CsrAddr — used by trap handler internals.
    pub fn get(&self, addr: CsrAddr) -> Word {
        self.regs[addr as usize]
    }

    /// Raw get by numeric address — used by side-effect handlers.
    pub fn get_by_addr(&self, addr: u16) -> Word {
        self.regs[addr as usize]
    }

    /// Raw set by CsrAddr — used by trap handler internals.
    pub fn set(&mut self, addr: CsrAddr, val: Word) {
        self.regs[addr as usize] = val;
    }

    /// Read using a pre-resolved descriptor.
    pub fn read_with_desc(&self, desc: CsrDesc) -> Word {
        self.regs[desc.storage as usize] & desc.view_mask
    }

    /// Write using a pre-resolved descriptor.
    pub fn write_with_desc(&mut self, desc: CsrDesc, val: Word) {
        let slot = &mut self.regs[desc.storage as usize];
        let mask = desc.view_mask & desc.wmask;
        *slot = (*slot & !mask) | (val & mask);
    }

    /// Convenience: lookup + read. Returns None for unknown CSR.
    #[cfg(test)]
    pub fn read_masked(&self, addr: u16) -> Option<Word> {
        Some(self.read_with_desc(find_desc(addr)?))
    }

    /// Convenience: lookup + write. Returns false for unknown CSR.
    #[cfg(test)]
    pub fn write_masked(&mut self, addr: u16, val: Word) -> bool {
        let Some(desc) = find_desc(addr) else {
            return false;
        };
        self.write_with_desc(desc, val);
        true
    }

    pub fn increment_cycle(&mut self) {
        self.regs[CsrAddr::cycle as usize] = self.regs[CsrAddr::cycle as usize].wrapping_add(1);
    }

    pub fn increment_instret(&mut self) {
        self.regs[CsrAddr::instret as usize] = self.regs[CsrAddr::instret as usize].wrapping_add(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_csr_returns_none() {
        let csr = CsrFile::new();
        assert!(csr.read_masked(0xFFF).is_none());
        assert!(csr.read_masked(0x123).is_none());
    }

    #[test]
    fn read_write_basic() {
        let mut csr = CsrFile::new();
        assert!(csr.write_masked(CsrAddr::mscratch as u16, 0xDEAD_BEEF));
        assert_eq!(csr.read_masked(CsrAddr::mscratch as u16), Some(0xDEAD_BEEF));
    }

    #[test]
    fn warl_masking() {
        let mut csr = CsrFile::new();
        csr.write_masked(CsrAddr::mip as u16, !0);
        assert_eq!(csr.read_masked(CsrAddr::mip as u16), Some(MIP_WMASK));
    }

    #[test]
    fn mstatus_warl() {
        let mut csr = CsrFile::new();
        csr.write_masked(CsrAddr::mstatus as u16, !0);
        let val = csr.read_masked(CsrAddr::mstatus as u16).unwrap();
        assert_eq!(val, MStatus::WRITABLE.bits());
    }

    #[test]
    fn shadow_sstatus_reads_mstatus() {
        let mut csr = CsrFile::new();
        let bits = MStatus::MIE.bits() | MStatus::SIE.bits();
        csr.write_masked(CsrAddr::mstatus as u16, bits);

        let sstatus = csr.read_masked(CsrAddr::sstatus as u16).unwrap();
        assert_ne!(sstatus & MStatus::SIE.bits(), 0);
        assert_eq!(sstatus & MStatus::MIE.bits(), 0);
    }

    #[test]
    fn sstatus_write_cannot_set_sd_bit() {
        let mut csr = CsrFile::new();
        csr.write_masked(CsrAddr::sstatus as u16, MStatus::SD.bits());
        assert_eq!(
            csr.read_masked(CsrAddr::mstatus as u16).unwrap() & MStatus::SD.bits(),
            0
        );
    }

    #[test]
    fn sstatus_read_shows_sd_from_mstatus() {
        let mut csr = CsrFile::new();
        // Set SD directly in mstatus (simulating hardware setting it)
        csr.set(CsrAddr::mstatus, MStatus::SD.bits());
        let sstatus = csr.read_masked(CsrAddr::sstatus as u16).unwrap();
        assert_ne!(sstatus & MStatus::SD.bits(), 0);
    }

    #[test]
    fn shadow_sstatus_write_updates_mstatus() {
        let mut csr = CsrFile::new();
        csr.write_masked(CsrAddr::mstatus as u16, MStatus::MIE.bits());
        csr.write_masked(CsrAddr::sstatus as u16, MStatus::SPIE.bits());

        let mstatus = csr.read_masked(CsrAddr::mstatus as u16).unwrap();
        assert_ne!(mstatus & MStatus::MIE.bits(), 0);
        assert_ne!(mstatus & MStatus::SPIE.bits(), 0);
    }

    #[test]
    fn shadow_sie_reads_mie() {
        let mut csr = CsrFile::new();
        csr.write_masked(CsrAddr::mie as u16, MIE_WMASK);
        let sie = csr.read_masked(CsrAddr::sie as u16).unwrap();
        assert_eq!(sie, SIE_MASK);
    }

    #[test]
    fn shadow_sip_write_only_ssip() {
        let mut csr = CsrFile::new();
        csr.write_masked(CsrAddr::sip as u16, !0);
        let mip = csr.read_masked(CsrAddr::mip as u16).unwrap();
        assert_eq!(mip, 1 << 1);
    }

    #[test]
    fn read_only_csr_ignores_writes() {
        let mut csr = CsrFile::new();
        csr.write_masked(CsrAddr::misa as u16, 0xFFFF);
        assert_eq!(csr.read_masked(CsrAddr::misa as u16), Some(0));
    }

    #[test]
    fn mtvec_allows_direct_and_vectored_mode() {
        let mut csr = CsrFile::new();
        csr.write_masked(CsrAddr::mtvec as u16, 0x8000_0000);
        assert_eq!(csr.read_masked(CsrAddr::mtvec as u16).unwrap() & 0x3, 0);

        csr.write_masked(CsrAddr::mtvec as u16, 0x8000_0001);
        assert_eq!(csr.read_masked(CsrAddr::mtvec as u16).unwrap(), 0x8000_0001);

        csr.write_masked(CsrAddr::mtvec as u16, 0x8000_0003);
        assert_eq!(csr.read_masked(CsrAddr::mtvec as u16).unwrap(), 0x8000_0001);
    }

    #[test]
    fn get_set_raw() {
        let mut csr = CsrFile::new();
        csr.set(CsrAddr::mepc, 0x1234);
        assert_eq!(csr.get(CsrAddr::mepc), 0x1234);
    }

    #[test]
    fn write_masked_unknown_returns_false() {
        let mut csr = CsrFile::new();
        assert!(!csr.write_masked(0xFFF, 42));
    }

    #[test]
    fn find_desc_known_csr() {
        assert!(find_desc(CsrAddr::mstatus as u16).is_some());
        assert!(find_desc(CsrAddr::cycle as u16).is_some());
        assert!(find_desc(CsrAddr::satp as u16).is_some());
    }

    #[test]
    fn find_desc_returns_correct_access_rule() {
        let satp = find_desc(CsrAddr::satp as u16).unwrap();
        assert!(matches!(satp.access, AccessRule::BlockedByMstatus(_)));

        let cycle = find_desc(CsrAddr::cycle as u16).unwrap();
        assert!(matches!(cycle.access, AccessRule::CounterGated));

        let mscratch = find_desc(CsrAddr::mscratch as u16).unwrap();
        assert!(matches!(mscratch.access, AccessRule::Standard));
    }

    #[test]
    fn shadow_storage_points_to_m_register() {
        let sstatus = find_desc(CsrAddr::sstatus as u16).unwrap();
        assert_eq!(sstatus.storage, CsrAddr::mstatus as u16);

        let sie = find_desc(CsrAddr::sie as u16).unwrap();
        assert_eq!(sie.storage, CsrAddr::mie as u16);

        let sip = find_desc(CsrAddr::sip as u16).unwrap();
        assert_eq!(sip.storage, CsrAddr::mip as u16);
    }

    #[test]
    fn increment_cycle_only() {
        let mut csr = CsrFile::new();
        csr.increment_cycle();
        csr.increment_cycle();
        assert_eq!(csr.get(CsrAddr::cycle), 2);
        assert_eq!(csr.get(CsrAddr::instret), 0);
    }

    #[test]
    fn increment_instret_only() {
        let mut csr = CsrFile::new();
        csr.increment_instret();
        assert_eq!(csr.get(CsrAddr::instret), 1);
        assert_eq!(csr.get(CsrAddr::cycle), 0);
    }

    #[test]
    fn mepc_sepc_clear_low_bit() {
        let mut csr = CsrFile::new();
        csr.write_masked(CsrAddr::mepc as u16, 0x1001);
        assert_eq!(csr.read_masked(CsrAddr::mepc as u16).unwrap(), 0x1000);

        csr.write_masked(CsrAddr::sepc as u16, 0x2003);
        assert_eq!(csr.read_masked(CsrAddr::sepc as u16).unwrap(), 0x2002);
    }

    #[test]
    fn counteren_bit_extracts_index() {
        assert_eq!(counteren_bit(0xC00), 0);
        assert_eq!(counteren_bit(0xC02), 2);
        assert_eq!(counteren_bit(0xC1F), 31);
    }

    #[test]
    fn medeleg_wmask() {
        let mut csr = CsrFile::new();
        csr.write_masked(CsrAddr::medeleg as u16, !0);
        let val = csr.read_masked(CsrAddr::medeleg as u16).unwrap();
        assert_eq!(val, 0xB3FF);
    }

    #[test]
    fn mideleg_wmask() {
        let mut csr = CsrFile::new();
        csr.write_masked(CsrAddr::mideleg as u16, !0);
        let val = csr.read_masked(CsrAddr::mideleg as u16).unwrap();
        assert_eq!(val, 0x222);
    }
}
