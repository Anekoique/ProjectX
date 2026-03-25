use core::fmt;

use crate::cpu::PendingTrap;

#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub enum XError {
    Trap(PendingTrap),
    // Memory management
    BadAddress,
    AddrNotAligned,
    // Instruction decoding
    PatternError,
    ParseError,
    InvalidInst,
    InvalidReg,
    // IO errors
    FailedToRead,
    FailedToWrite,
    // Not yet implemented
    Unimplemented,
}

pub type XResult<T = ()> = Result<T, XError>;

impl XError {
    pub fn as_trap(&self) -> Option<crate::cpu::PendingTrap> {
        match self {
            Self::Trap(t) => Some(*t),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            XError::Trap(_) => "trap triggered",
            XError::BadAddress => "bad address",
            XError::AddrNotAligned => "address not aligned",

            XError::PatternError => "pattern error",
            XError::ParseError => "parse error",
            XError::InvalidInst => "invalid instruction",
            XError::InvalidReg => "invalid register",
            XError::FailedToRead => "failed to read",
            XError::FailedToWrite => "failed to write",
            XError::Unimplemented => "unimplemented",
        }
    }
}

#[macro_export]
macro_rules! ensure {
    ($predicate:expr, $context_selector:expr $(,)?) => {
        if !$predicate {
            return $context_selector;
        }
    };
}

impl fmt::Display for XError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            XError::Trap(trap) => write!(f, "trap {trap:?} thrown"),
            _ => write!(f, "{}", self.as_str()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_variants_have_distinct_nonempty_messages() {
        let variants = [
            XError::BadAddress,
            XError::AddrNotAligned,
            XError::PatternError,
            XError::ParseError,
            XError::InvalidInst,
            XError::InvalidReg,
            XError::FailedToRead,
            XError::FailedToWrite,
            XError::Unimplemented,
        ];
        for v in &variants {
            assert!(!v.as_str().is_empty());
            assert_eq!(v.to_string(), v.as_str());
        }
        let msgs: std::collections::HashSet<_> = variants.iter().map(|v| v.as_str()).collect();
        assert_eq!(msgs.len(), variants.len());
    }
}
