use core::fmt;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum XError {
    // Memory management
    BadAddress,
    AddrNotAligned,
    // Instruction decoding
    PatternError,
    ParseError,
    DecodeError,
    InvalidInstType,
    InvalidInst,
    InvalidReg,
    // IO errors
    FailedToRead,
    FailedToWrite,
    // Control flow
    ToTerminate,
    // Not yet implemented
    Unimplemented,
}

pub type XResult<T = ()> = Result<T, XError>;

impl XError {
    pub fn as_str(&self) -> &'static str {
        match self {
            XError::BadAddress => "bad address",
            XError::AddrNotAligned => "address not aligned",
            XError::InvalidInstType => "invalid instruction type",
            XError::PatternError => "pattern error",
            XError::ParseError => "parse error",
            XError::DecodeError => "decode error",
            XError::InvalidInst => "invalid instruction",
            XError::InvalidReg => "invalid register",
            XError::FailedToRead => "failed to read",
            XError::FailedToWrite => "failed to write",
            XError::ToTerminate => "to be terminated",
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
        write!(f, "{}", self.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_xerror() {
        let err = XError::BadAddress;
        assert_eq!(err.to_string(), "bad address");
    }
}
