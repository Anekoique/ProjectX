//! Trap handling: exception/interrupt causes, privilege delegation, and
//! vectored trap entry/return (`mret`/`sret`).

mod cause;
mod exception;
mod handler;
pub mod interrupt;

pub use cause::{PendingTrap, TrapCause};
pub use exception::Exception;
pub use interrupt::Interrupt;

use crate::{
    arch::riscv::cpu::RVCore,
    config::Word,
    device::bus::Bus,
    error::{XError, XResult},
};

impl RVCore {
    #[inline]
    pub(in crate::arch::riscv) fn trap(&self, cause: TrapCause, tval: Word) -> XError {
        XError::Trap(PendingTrap { cause, tval })
    }

    #[inline]
    pub(in crate::arch::riscv) fn trap_exception(&self, exc: Exception, tval: Word) -> XError {
        self.trap(TrapCause::Exception(exc), tval)
    }

    #[inline]
    pub(in crate::arch::riscv) fn illegal_inst(&self) -> XError {
        self.trap_exception(Exception::IllegalInstruction, 0)
    }

    pub(in crate::arch::riscv) fn trap_on_err(
        &mut self,
        bus: &mut Bus,
        f: impl FnOnce(&mut Self, &mut Bus) -> XResult,
    ) -> XResult {
        match f(self, bus) {
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
pub(in crate::arch::riscv) mod test_helpers {
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
