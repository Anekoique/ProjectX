//! Asynchronous interrupt codes and priority ordering (Privileged Spec §3.1.9).

use crate::config::Word;

/// RISC-V `mip` bit positions — shared vocabulary between CPU and devices.
#[allow(dead_code)]
pub const SSIP: u64 = 1 << 1; // Supervisor software interrupt pending
/// Machine software interrupt pending.
pub const MSIP: u64 = 1 << 3; // Machine software interrupt pending
/// Supervisor timer interrupt pending.
#[allow(dead_code)]
pub const STIP: u64 = 1 << 5; // Supervisor timer interrupt pending
/// Machine timer interrupt pending.
pub const MTIP: u64 = 1 << 7; // Machine timer interrupt pending
/// Supervisor external interrupt pending.
pub const SEIP: u64 = 1 << 9; // Supervisor external interrupt pending
/// Machine external interrupt pending.
pub const MEIP: u64 = 1 << 11; // Machine external interrupt pending

/// Hardware-wired mip bits managed via IrqState (excludes SSIP/STIP —
/// software-controlled). STIP is managed by Sstc stimecmp comparison.
pub const HW_IP_MASK: Word = (MSIP | MTIP | SEIP | MEIP) as Word;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
/// Asynchronous interrupt codes with priority ordering.
pub enum Interrupt {
    SupervisorSoftware = 1,
    MachineSoftware    = 3,
    SupervisorTimer    = 5,
    MachineTimer       = 7,
    SupervisorExternal = 9,
    MachineExternal    = 11,
}

impl Interrupt {
    /// All interrupt variants in descending priority.
    /// RISC-V spec order: MEI > MSI > MTI > SEI > SSI > STI.
    pub const PRIORITY_ORDER: &[Self] = &[
        Self::MachineExternal,
        Self::MachineSoftware,
        Self::MachineTimer,
        Self::SupervisorExternal,
        Self::SupervisorSoftware,
        Self::SupervisorTimer,
    ];

    /// Return the `mip`/`mie` bit mask for this interrupt.
    pub const fn bit(self) -> Word {
        1 << (self as Word)
    }

    /// True for M-level interrupts (MSI/MTI/MEI).
    pub const fn is_machine(self) -> bool {
        matches!(
            self,
            Self::MachineSoftware | Self::MachineTimer | Self::MachineExternal
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupt_codes() {
        assert_eq!(Interrupt::SupervisorSoftware as u8, 1);
        assert_eq!(Interrupt::MachineSoftware as u8, 3);
        assert_eq!(Interrupt::MachineExternal as u8, 11);
    }

    #[test]
    fn bit_returns_correct_mask() {
        assert_eq!(Interrupt::SupervisorSoftware.bit(), 1 << 1);
        assert_eq!(Interrupt::MachineTimer.bit(), 1 << 7);
        assert_eq!(Interrupt::MachineExternal.bit(), 1 << 11);
    }

    #[test]
    fn is_machine_classification() {
        assert!(Interrupt::MachineSoftware.is_machine());
        assert!(Interrupt::MachineTimer.is_machine());
        assert!(Interrupt::MachineExternal.is_machine());
        assert!(!Interrupt::SupervisorSoftware.is_machine());
        assert!(!Interrupt::SupervisorTimer.is_machine());
        assert!(!Interrupt::SupervisorExternal.is_machine());
    }

    #[test]
    fn priority_order_covers_all_variants() {
        assert_eq!(Interrupt::PRIORITY_ORDER.len(), 6);
    }
}
