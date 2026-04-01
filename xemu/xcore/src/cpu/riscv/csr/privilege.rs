//! RISC-V privilege modes (Machine, Supervisor, User).

use crate::config::Word;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
/// RISC-V privilege level. Ordered: User < Supervisor < Machine.
pub enum PrivilegeMode {
    User       = 0,
    Supervisor = 1,
    // Reserved = 2,
    Machine    = 3,
}

impl PrivilegeMode {
    /// Decode from the 2-bit encoding. Reserved value 2 maps to Machine.
    pub fn from_bits(bits: Word) -> Self {
        match bits & 0x3 {
            0 => Self::User,
            1 => Self::Supervisor,
            3 => Self::Machine,
            _ => Self::Machine, // reserved → M per spec
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordering() {
        assert!(PrivilegeMode::User < PrivilegeMode::Supervisor);
        assert!(PrivilegeMode::Supervisor < PrivilegeMode::Machine);
        assert!(PrivilegeMode::User < PrivilegeMode::Machine);
    }

    #[test]
    fn from_bits_roundtrip() {
        assert_eq!(PrivilegeMode::from_bits(0), PrivilegeMode::User);
        assert_eq!(PrivilegeMode::from_bits(1), PrivilegeMode::Supervisor);
        assert_eq!(PrivilegeMode::from_bits(3), PrivilegeMode::Machine);
    }

    #[test]
    fn from_bits_reserved_defaults_to_machine() {
        assert_eq!(PrivilegeMode::from_bits(2), PrivilegeMode::Machine);
    }

    #[test]
    fn from_bits_masks_upper_bits() {
        assert_eq!(PrivilegeMode::from_bits(0xFF), PrivilegeMode::Machine);
        assert_eq!(PrivilegeMode::from_bits(0x100), PrivilegeMode::User);
    }
}
