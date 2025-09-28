use core::fmt;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum XError {
    BadAddress,
    AddrNotAligned,
    DecodeError,
}

pub type XResult<T = ()> = Result<T, XError>;

impl XError {
    pub fn as_str(&self) -> &'static str {
        match self {
            XError::BadAddress => "bad address",
            XError::AddrNotAligned => "address not aligned",
            XError::DecodeError => "decode error",
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
