//! Trap cause encoding and `mcause`/`scause` value generation.

use super::{Exception, Interrupt};
use crate::config::Word;

/// Trap cause: either a synchronous exception or an asynchronous interrupt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrapCause {
    /// Synchronous fault or system call.
    Exception(Exception),
    /// Asynchronous interrupt (timer, software, external).
    Interrupt(Interrupt),
}

impl TrapCause {
    /// True if this is an asynchronous interrupt (not an exception).
    pub fn is_interrupt(&self) -> bool {
        matches!(self, Self::Interrupt(_))
    }

    /// Exception/interrupt code (lower bits of `mcause`/`scause`).
    pub fn code(&self) -> Word {
        match self {
            Self::Exception(e) => *e as Word,
            Self::Interrupt(i) => *i as Word,
        }
    }

    /// Encode as `mcause`/`scause` value (interrupt bit | code).
    pub fn to_mcause(self) -> Word {
        match self {
            Self::Exception(_) => self.code(),
            Self::Interrupt(_) => (1 as Word).wrapping_shl(Word::BITS - 1) | self.code(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
/// A trap awaiting delivery: cause + trap value (`mtval`/`stval`).
pub struct PendingTrap {
    pub cause: TrapCause,
    pub tval: Word,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trap_cause_is_interrupt() {
        let exc = TrapCause::Exception(Exception::IllegalInstruction);
        let int = TrapCause::Interrupt(Interrupt::MachineTimer);
        assert!(!exc.is_interrupt());
        assert!(int.is_interrupt());
    }

    #[test]
    fn trap_cause_code() {
        assert_eq!(TrapCause::Exception(Exception::Breakpoint).code(), 3);
        assert_eq!(TrapCause::Interrupt(Interrupt::MachineTimer).code(), 7);
    }

    #[test]
    fn to_mcause_exception_no_interrupt_bit() {
        let cause = TrapCause::Exception(Exception::EcallFromU);
        assert_eq!(cause.to_mcause(), 8);
    }

    #[test]
    fn to_mcause_interrupt_has_high_bit() {
        let cause = TrapCause::Interrupt(Interrupt::MachineTimer);
        let expected = (1 as Word).wrapping_shl(Word::BITS - 1) | 7;
        assert_eq!(cause.to_mcause(), expected);
    }

    #[test]
    fn to_mcause_all_exceptions() {
        let cases: &[(Exception, Word)] = &[
            (Exception::InstructionMisaligned, 0),
            (Exception::IllegalInstruction, 2),
            (Exception::Breakpoint, 3),
            (Exception::EcallFromU, 8),
            (Exception::EcallFromS, 9),
            (Exception::EcallFromM, 11),
            (Exception::StorePageFault, 15),
        ];
        for &(exc, code) in cases {
            assert_eq!(TrapCause::Exception(exc).to_mcause(), code);
        }
    }

    #[test]
    fn to_mcause_all_interrupts() {
        let high = (1 as Word).wrapping_shl(Word::BITS - 1);
        let cases: &[(Interrupt, Word)] = &[
            (Interrupt::SupervisorSoftware, high | 1),
            (Interrupt::MachineSoftware, high | 3),
            (Interrupt::SupervisorTimer, high | 5),
            (Interrupt::MachineTimer, high | 7),
            (Interrupt::SupervisorExternal, high | 9),
            (Interrupt::MachineExternal, high | 11),
        ];
        for &(int, expected) in cases {
            assert_eq!(TrapCause::Interrupt(int).to_mcause(), expected);
        }
    }
}
