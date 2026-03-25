mod cause;
mod exception;
mod handler;
mod interrupt;

pub use cause::{PendingTrap, TrapCause};
pub use exception::Exception;
pub use interrupt::Interrupt;

use crate::{
    config::Word,
    cpu::riscv::RVCore,
    error::{XError, XResult},
};

impl RVCore {
    #[inline]
    pub(in crate::cpu::riscv) fn trap<T>(&self, cause: TrapCause, tval: Word) -> XResult<T> {
        Err(XError::Trap(PendingTrap { cause, tval }))
    }

    #[inline]
    pub(in crate::cpu::riscv) fn trap_exception<T>(
        &self,
        exc: Exception,
        tval: Word,
    ) -> XResult<T> {
        self.trap(TrapCause::Exception(exc), tval)
    }

    #[inline]
    pub(in crate::cpu::riscv) fn illegal_inst<T>(&self) -> XResult<T> {
        self.trap_exception(Exception::IllegalInstruction, 0)
    }

    pub(in crate::cpu::riscv) fn trap_on_err(
        &mut self,
        f: impl FnOnce(&mut Self) -> XResult,
    ) -> XResult {
        match f(self) {
            Ok(()) => Ok(()),
            Err(XError::Trap(trap)) => {
                self.raise_trap(trap.cause, trap.tval);
                Ok(())
            }
            Err(XError::InvalidInst | XError::InvalidReg) => {
                self.raise_trap(TrapCause::Exception(Exception::IllegalInstruction), 0);
                Ok(())
            }
            Err(err) => Err(err),
        }
    }
}

#[cfg(test)]
pub(in crate::cpu::riscv) mod test_helpers {
    use super::*;

    /// Assert that a result is an `XError::Trap` with the given cause and tval.
    pub fn assert_trap<T: std::fmt::Debug>(result: XResult<T>, cause: TrapCause, tval: Word) {
        assert_eq!(
            result.unwrap_err().as_trap(),
            Some(PendingTrap { cause, tval }),
            "expected trap {cause:?} with tval={tval:#x}",
        );
    }

    /// Assert that a result is an illegal-instruction trap.
    pub fn assert_illegal_inst<T: std::fmt::Debug>(result: XResult<T>) {
        assert_trap(
            result,
            TrapCause::Exception(Exception::IllegalInstruction),
            0,
        );
    }
}
