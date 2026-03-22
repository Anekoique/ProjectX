#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Exception {
    InstructionMisaligned = 0,
    InstructionAccessFault = 1,
    IllegalInstruction   = 2,
    Breakpoint           = 3,
    LoadMisaligned       = 4,
    LoadAccessFault      = 5,
    StoreMisaligned      = 6,
    StoreAccessFault     = 7,
    EcallFromU           = 8,
    EcallFromS           = 9,
    EcallFromM           = 11,
    InstructionPageFault = 12,
    LoadPageFault        = 13,
    StorePageFault       = 15,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exception_codes() {
        assert_eq!(Exception::InstructionMisaligned as u8, 0);
        assert_eq!(Exception::IllegalInstruction as u8, 2);
        assert_eq!(Exception::Breakpoint as u8, 3);
        assert_eq!(Exception::EcallFromU as u8, 8);
        assert_eq!(Exception::EcallFromS as u8, 9);
        assert_eq!(Exception::EcallFromM as u8, 11);
        assert_eq!(Exception::StorePageFault as u8, 15);
    }
}
