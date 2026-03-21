mod base;
mod compressed;
mod mul;
mod privileged;
mod zicsr;

use super::RVCore;
use crate::{
    config::Word,
    error::{XError, XResult},
    isa::{DecodedInst, InstKind, RVReg},
};

macro_rules! build_dispatch {
    ( $( ($fmt:ident, ($($arg:ident),*), [$($name:ident),*]) ),* $(,)? ) => {
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
