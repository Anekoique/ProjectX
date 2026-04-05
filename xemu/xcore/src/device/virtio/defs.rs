//! VirtIO protocol constants: status, block request types, descriptor flags.

/// Block request type (guest → device).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum BlkReqType {
    In  = 0,
    Out = 1,
}

impl BlkReqType {
    pub fn from_u32(v: u32) -> Option<Self> {
        match v {
            0 => Some(Self::In),
            1 => Some(Self::Out),
            _ => None,
        }
    }
}

/// Block request status (device → guest).
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum BlkStatus {
    Ok     = 0,
    IoErr  = 1,
    Unsupp = 2,
}

/// Descriptor flags (parsed from guest memory).
#[derive(Clone, Copy, Debug)]
pub struct DescFlags(pub u16);

impl DescFlags {
    const NEXT: u16 = 1;
    const WRITE: u16 = 2;

    pub fn has_next(self) -> bool {
        self.0 & Self::NEXT != 0
    }
    /// True if the descriptor buffer is device-writable.
    pub fn is_write(self) -> bool {
        self.0 & Self::WRITE != 0
    }
}
