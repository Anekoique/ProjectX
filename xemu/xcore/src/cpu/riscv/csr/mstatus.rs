//! Machine/Supervisor status register (`mstatus`) bitfield definitions.

use bitflags::bitflags;

use super::PrivilegeMode;
use crate::config::Word;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    /// Machine/Supervisor status register bitfields (Privileged Spec §3.1.6).
    pub struct MStatus: Word {
        const SIE   = 1 << 1;
        const MIE   = 1 << 3;
        const SPIE  = 1 << 5;
        const MPIE  = 1 << 7;
        const SPP   = 1 << 8;
        const MPP   = 0b11 << 11;
        const FS    = 0b11 << 13;
        const XS    = 0b11 << 15;
        const MPRV  = 1 << 17;
        const SUM   = 1 << 18;
        const MXR   = 1 << 19;
        const TVM   = 1 << 20;
        const TW    = 1 << 21;
        const TSR   = 1 << 22;

        #[cfg(isa64)]
        const SD = 1 << 63;
        #[cfg(isa32)]
        const SD = 1 << 31;

        // Composite: S-mode visible bits of mstatus
        const SSTATUS = Self::SIE.bits() | Self::SPIE.bits() | Self::SPP.bits()
                      | Self::FS.bits()  | Self::XS.bits()
                      | Self::SUM.bits() | Self::MXR.bits() | Self::SD.bits();

        // Composite: writable bits for mstatus
        const WRITABLE = Self::SIE.bits() | Self::MIE.bits()
                       | Self::SPIE.bits() | Self::MPIE.bits()
                       | Self::SPP.bits() | Self::MPP.bits()
                       | Self::FS.bits()
                       | Self::MPRV.bits() | Self::SUM.bits() | Self::MXR.bits()
                       | Self::TVM.bits() | Self::TW.bits() | Self::TSR.bits();
    }
}

impl MStatus {
    /// Extract the Machine Previous Privilege (MPP) field.
    pub fn mpp(self) -> PrivilegeMode {
        PrivilegeMode::from_bits((self.bits() >> 11) & 0x3)
    }

    /// Return a copy with MPP set to `mode`.
    pub fn with_mpp(self, mode: PrivilegeMode) -> Self {
        Self::from_bits_truncate((self.bits() & !Self::MPP.bits()) | ((mode as Word) << 11))
    }

    /// Extract the Supervisor Previous Privilege (SPP) field.
    pub fn spp(self) -> PrivilegeMode {
        PrivilegeMode::from_bits((self.bits() >> 8) & 0x1)
    }

    /// Return a copy with SPP set to `mode` (must not be Machine).
    pub fn with_spp(self, mode: PrivilegeMode) -> Self {
        debug_assert!(
            mode != PrivilegeMode::Machine,
            "SPP is a 1-bit field; Machine mode cannot be encoded"
        );
        Self::from_bits_truncate((self.bits() & !Self::SPP.bits()) | (((mode as Word) & 1) << 8))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_bit_flags() {
        let ms = MStatus::SIE | MStatus::MIE;
        assert!(ms.contains(MStatus::SIE));
        assert!(ms.contains(MStatus::MIE));
        assert!(!ms.contains(MStatus::MPRV));
    }

    #[test]
    fn insert_remove() {
        let mut ms = MStatus::empty();
        ms.insert(MStatus::MPIE);
        assert!(ms.contains(MStatus::MPIE));
        ms.remove(MStatus::MPIE);
        assert!(!ms.contains(MStatus::MPIE));
    }

    #[test]
    fn mpp_roundtrip() {
        for mode in [
            PrivilegeMode::User,
            PrivilegeMode::Supervisor,
            PrivilegeMode::Machine,
        ] {
            let ms = MStatus::empty().with_mpp(mode);
            assert_eq!(ms.mpp(), mode);
        }
    }

    #[test]
    fn spp_roundtrip() {
        // SPP is 1 bit: only User (0) and Supervisor (1)
        let ms_u = MStatus::empty().with_spp(PrivilegeMode::User);
        assert_eq!(ms_u.spp(), PrivilegeMode::User);

        let ms_s = MStatus::empty().with_spp(PrivilegeMode::Supervisor);
        assert_eq!(ms_s.spp(), PrivilegeMode::Supervisor);
    }

    #[test]
    fn mpp_does_not_clobber_other_bits() {
        let ms = MStatus::SIE | MStatus::MPRV;
        let ms2 = ms.with_mpp(PrivilegeMode::Machine);
        assert!(ms2.contains(MStatus::SIE));
        assert!(ms2.contains(MStatus::MPRV));
        assert_eq!(ms2.mpp(), PrivilegeMode::Machine);
    }

    #[test]
    fn sstatus_mask_includes_expected_bits() {
        let sstatus = MStatus::SSTATUS;
        // SSTATUS must include SIE, SPIE, SPP, FS, XS, SUM, MXR, SD
        assert!(sstatus.contains(MStatus::SIE));
        assert!(sstatus.contains(MStatus::SPIE));
        assert!(sstatus.contains(MStatus::SPP));
        assert!(sstatus.contains(MStatus::FS));
        assert!(sstatus.contains(MStatus::XS));
        assert!(sstatus.contains(MStatus::SUM));
        assert!(sstatus.contains(MStatus::MXR));
        assert!(sstatus.contains(MStatus::SD));
        // SSTATUS must NOT include M-mode bits
        assert!(!sstatus.contains(MStatus::MIE));
        assert!(!sstatus.contains(MStatus::MPIE));
        assert!(!sstatus.contains(MStatus::MPRV));
        assert!(!sstatus.contains(MStatus::TSR));
    }

    #[test]
    fn from_bits_truncate_enforces_warl() {
        // Writing garbage should only keep defined bits
        let ms = MStatus::from_bits_truncate(!0);
        // Undefined bits should be zero
        assert_eq!(ms.bits() & !MStatus::all().bits(), 0);
    }
}
