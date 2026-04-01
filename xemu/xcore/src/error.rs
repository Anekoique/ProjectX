//! Error types for the emulator core.

use core::fmt;

use crate::cpu::PendingTrap;

/// Emulator error type covering traps, memory faults, decode errors, and IO.
#[non_exhaustive]
#[derive(Clone, Copy, Debug)]
pub enum XError {
    /// RISC-V trap (exception or interrupt) to be delivered by the trap
    /// handler.
    Trap(PendingTrap),
    /// Physical address out of range or unmapped.
    BadAddress,
    /// Virtual address translation failed (PTE invalid/permissions).
    PageFault,
    /// Misaligned memory access.
    AddrNotAligned,
    /// Instruction pattern file parse error.
    PatternError,
    /// Instruction name/format string parse error.
    ParseError,
    /// Unrecognized instruction encoding.
    InvalidInst,
    /// Invalid register index.
    InvalidReg,
    /// File read failed.
    FailedToRead,
    /// File write failed.
    FailedToWrite,
    /// Guest program called exit (SiFive test finisher).
    ProgramExit(u32),
    /// Debugger breakpoint hit at the given PC.
    DebugBreak(usize),
    /// Feature not yet implemented.
    Unimplemented,
}

/// Convenience alias: `Result<T, XError>` with default `T = ()`.
pub type XResult<T = ()> = Result<T, XError>;

impl XError {
    /// Extract the pending trap if this is a `Trap` variant.
    pub fn as_trap(&self) -> Option<crate::cpu::PendingTrap> {
        match self {
            Self::Trap(t) => Some(*t),
            _ => None,
        }
    }

    /// Human-readable description of the error.
    pub fn as_str(&self) -> &'static str {
        match self {
            XError::Trap(_) => "trap triggered",
            XError::BadAddress => "bad address",
            XError::PageFault => "page fault",
            XError::AddrNotAligned => "address not aligned",

            XError::PatternError => "pattern error",
            XError::ParseError => "parse error",
            XError::InvalidInst => "invalid instruction",
            XError::InvalidReg => "invalid register",
            XError::FailedToRead => "failed to read",
            XError::FailedToWrite => "failed to write",
            XError::ProgramExit(_) => "program exit",
            XError::DebugBreak(_) => "breakpoint hit",
            XError::Unimplemented => "unimplemented",
        }
    }
}

#[macro_export]
macro_rules! ensure {
    ($predicate:expr, $context_selector:expr $(,)?) => {
        if !$predicate {
            return Err($context_selector);
        }
    };
}

impl fmt::Display for XError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            XError::Trap(trap) => write!(f, "trap {trap:?} thrown"),
            XError::ProgramExit(code) => write!(f, "program exit (code={code})"),
            XError::DebugBreak(pc) => write!(f, "breakpoint at {pc:#x}"),
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
            XError::PageFault,
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
