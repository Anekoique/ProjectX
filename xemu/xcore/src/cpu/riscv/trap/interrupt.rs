use crate::config::Word;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
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

    pub const fn bit(self) -> Word {
        1 << (self as Word)
    }

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
