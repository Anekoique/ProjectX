use super::RVCore;
use crate::{
    config::SWord,
    error::{XError, XResult},
    isa::RVReg,
};

impl RVCore {
    pub(super) fn ebreak(&mut self, _rd: RVReg, _rs1: RVReg, _imm: SWord) -> XResult {
        self.gpr[RVReg::a0] = 0;
        Err(XError::ToTerminate)
    }
    pub(super) fn mret(&mut self, _rd: RVReg, _rs1: RVReg, _rs2: RVReg) -> XResult {
        Err(XError::Unimplemented)
    }

    pub(super) fn ecall(&mut self, _rd: RVReg, _rs1: RVReg, _imm: SWord) -> XResult {
        Err(XError::Unimplemented)
    }
}
