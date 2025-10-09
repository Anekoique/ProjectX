use super::RVCore;
use crate::{
    config::SWord,
    error::{XError, XResult},
    isa::RVReg,
};

impl RVCore {
    pub(super) fn csrrw(&mut self, _rd: RVReg, _rs1: RVReg, _imm: SWord) -> XResult {
        Err(XError::Unimplemented)
    }

    pub(super) fn csrrs(&mut self, _rd: RVReg, _rs1: RVReg, _imm: SWord) -> XResult {
        Err(XError::Unimplemented)
    }

    pub(super) fn csrrc(&mut self, _rd: RVReg, _rs1: RVReg, _imm: SWord) -> XResult {
        Err(XError::Unimplemented)
    }

    pub(super) fn csrrwi(&mut self, _rd: RVReg, _rs1: RVReg, _imm: SWord) -> XResult {
        Err(XError::Unimplemented)
    }

    pub(super) fn csrrsi(&mut self, _rd: RVReg, _rs1: RVReg, _imm: SWord) -> XResult {
        Err(XError::Unimplemented)
    }

    pub(super) fn csrrci(&mut self, _rd: RVReg, _rs1: RVReg, _imm: SWord) -> XResult {
        Err(XError::Unimplemented)
    }
}
