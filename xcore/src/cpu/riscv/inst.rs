mod atomic;
mod base;
mod compressed;
mod mul;
mod privileged;
mod zicsr;
mod zifence;

use super::RVCore;
use crate::{
    config::Word,
    error::{XError, XResult},
    isa::{DecodedInst, RVReg},
};

macro_rules! inst_dispatcher {
    (@inner $self:expr, $inst:expr, $args:tt, $( $f:ident ),* $(,)?) => {
        match $inst.as_str() {
            $( stringify!($f) => $self.$f $args, )*
            _ => Err(XError::InvalidInst),
        }
    };

    ($( $V:ident ( $($arg:ident),* $(,)? ) { $( $f:ident ),* $(,)? } )*) => {
        pub fn dispatch(&mut self, decoded: DecodedInst) -> XResult {
            match decoded {
                $(
                    DecodedInst::$V { inst, $($arg),* } => {
                        inst_dispatcher!(@inner self, inst, ( $($arg),* ), $($f),* )
                    }
                )*
            }
        }
    };
}

impl RVCore {
    inst_dispatcher!(
        R(rd, rs1, rs2) {
            add, sub, sll, slt, sltu, xor, srl, sra, or,
            and, mul, mulh, mulhu, div, divu, rem, remu, mret,
        }
        I(rd, rs1, imm) {
            addi, slli, slti, sltiu, xori, srli, srla, ori, andi,
            lb, lh, lw, lbu, lhu, jalr, csrrw, csrrs, csrrc,
            csrrwi, csrrsi, csrrci, ebreak,
        }
        S(rs1, rs2, imm) { sb, sh, sw }
        B(rs1, rs2, imm) { beq, bne, blt, bge, bltu, bgeu }
        U(rd, imm) { lui, auipc }
        J(rd, imm) { jal }
    );

    #[inline(always)]
    fn set_gpr(&mut self, reg: RVReg, value: Word) {
        if reg != RVReg::zero {
            self.gpr[reg] = value;
        }
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
            inst: "add".into(),
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
            inst: "nope".into(),
            rd: RVReg::t0,
            rs1: RVReg::t1,
            rs2: RVReg::t2,
        };

        let err = core.dispatch(inst).unwrap_err();
        assert!(matches!(err, XError::InvalidInst));
    }
}
