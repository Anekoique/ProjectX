use bitflags::bitflags;

use crate::config::Word;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Mip: Word {
        const SSIP = 1 << 1;
        const MSIP = 1 << 3;
        const STIP = 1 << 5;
        const MTIP = 1 << 7;
        const SEIP = 1 << 9;
        const MEIP = 1 << 11;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interrupt_pending_bits_match_mip_positions() {
        assert_eq!(Mip::SSIP.bits(), 1 << 1);
        assert_eq!(Mip::MSIP.bits(), 1 << 3);
        assert_eq!(Mip::STIP.bits(), 1 << 5);
        assert_eq!(Mip::MTIP.bits(), 1 << 7);
        assert_eq!(Mip::SEIP.bits(), 1 << 9);
        assert_eq!(Mip::MEIP.bits(), 1 << 11);
    }
}
