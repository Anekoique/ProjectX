//! Instruction dispatch and per-extension handlers (I/M/A/F/D/C/Zicsr).
//!
//! The [`build_dispatch!`] macro generates a single match on [`DecodedInst`]
//! that routes to per-instruction handler methods. Shared macros [`rv64_op!`]
//! and [`rv64_only!`] handle RV32/RV64 width gating.

mod atomic;
mod base;
mod compressed;
mod float;
mod mul;
mod privileged;
mod zicsr;

use super::RVCore;
use crate::{
    config::Word,
    error::{XError, XResult},
    isa::{DecodedInst, InstKind, RVReg},
};

/// RV64-only word-width operation. On RV32, returns `InvalidInst`.
/// Evaluates `$body`, sign-extends the result to 64-bit, and writes to `$rd`.
#[allow(clippy::needless_return)]
macro_rules! rv64_op {
    ($self:ident, $rd:ident, |$($param:ident),+| $body:expr) => {{
        #[cfg(isa32)]
        {
            let _ = ($rd, $($param),+);
            return Err($crate::error::XError::InvalidInst);
        }
        #[cfg(isa64)]
        {
            let value = { $body };
            $self.set_gpr($rd, value as i64 as $crate::config::Word)
        }
    }};
}
use rv64_op;

/// Guard + body wrapper for RV64-only instructions.
/// On RV32, returns `InvalidInst`; on RV64, executes the body.
#[allow(clippy::needless_return)]
macro_rules! rv64_only {
    ($body:expr; $($unused:expr),* $(,)?) => {{
        #[cfg(isa32)]
        {
            let _ = ($($unused),*);
            return Err($crate::error::XError::InvalidInst);
        }
        #[cfg(isa64)]
        { $body }
    }};
}
use rv64_only;

macro_rules! build_dispatch {
    ( $( ($fmt:ident, ($($arg:ident),*), [$($name:ident),*]) ),* $(,)? ) => {
        /// Route a decoded instruction to its handler method.
        #[inline]
        pub fn dispatch(&mut self, decoded: DecodedInst) -> XResult {
            match decoded {
                $(
                    DecodedInst::$fmt { kind, $($arg),* } => {
                        let handler = match kind {
                            $( InstKind::$name => Self::$name, )*
                            _ => return Err(XError::InvalidInst),
                        };
                        handler(self, $($arg),*)
                    }
                )*
            }
        }
    };
}

impl RVCore {
    crate::rv_inst_table!(build_dispatch);

    #[inline(always)]
    fn set_gpr(&mut self, reg: RVReg, value: Word) -> XResult {
        if reg == RVReg::zero {
            return Ok(());
        }
        self.gpr[reg] = value;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dispatch_executes_known_instruction() {
        let mut core = RVCore::new();
        core.gpr[RVReg::t0] = 3;
        core.gpr[RVReg::t1] = 4;

        let inst = DecodedInst::R {
            kind: InstKind::add,
            rd: RVReg::t2,
            rs1: RVReg::t0,
            rs2: RVReg::t1,
        };
        core.dispatch(inst).unwrap();
        assert_eq!(core.gpr[RVReg::t2], 7);
    }

    #[test]
    fn dispatch_rejects_unknown_instruction() {
        let mut core = RVCore::new();
        let inst = DecodedInst::R {
            kind: InstKind::addi,
            rd: RVReg::t0,
            rs1: RVReg::t1,
            rs2: RVReg::t2,
        };

        let err = core.dispatch(inst).unwrap_err();
        assert!(matches!(err, XError::InvalidInst));
    }
}
