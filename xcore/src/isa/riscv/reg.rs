use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct MStatus: crate::config::Word {
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
        const SD    = 1 << 63;
        #[cfg(isa32)]
        const SD    = 1 << 31;
    }
}