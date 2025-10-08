use super::RVCore;
use crate::{
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
            ebreak,
        }
        S(rs1, rs2, imm) { sb, sh, sw }
        B(rs1, rs2, imm) { beq, bne, blt, bge, bltu, bgeu }
        U(rd, imm) { lui, auipc }
        J(rd, imm) { jal }
    );

    fn add(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = self.gpr[rs1 as usize].wrapping_add(self.gpr[rs2 as usize]);
        Ok(())
    }

    fn sub(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = self.gpr[rs1 as usize].wrapping_sub(self.gpr[rs2 as usize]);
        Ok(())
    }

    fn sll(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        let shamt = self.gpr[rs2 as usize] & 0x1F;
        self.gpr[rd as usize] = self.gpr[rs1 as usize] << shamt;
        Ok(())
    }

    fn slt(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = if (self.gpr[rs1 as usize] as i32) < (self.gpr[rs2 as usize] as i32)
        {
            1
        } else {
            0
        };
        Ok(())
    }

    fn sltu(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = if self.gpr[rs1 as usize] < self.gpr[rs2 as usize] {
            1
        } else {
            0
        };
        Ok(())
    }

    fn xor(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = self.gpr[rs1 as usize] ^ self.gpr[rs2 as usize];
        Ok(())
    }

    fn srl(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        let shamt = self.gpr[rs2 as usize] & 0x1F;
        self.gpr[rd as usize] = self.gpr[rs1 as usize] >> shamt;
        Ok(())
    }

    fn sra(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        let shamt = self.gpr[rs2 as usize] & 0x1F;
        self.gpr[rd as usize] = ((self.gpr[rs1 as usize] as i32) >> shamt) as u32;
        Ok(())
    }

    fn or(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = self.gpr[rs1 as usize] | self.gpr[rs2 as usize];
        Ok(())
    }

    fn and(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = self.gpr[rs1 as usize] & self.gpr[rs2 as usize];
        Ok(())
    }

    fn mul(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = self.gpr[rs1 as usize].wrapping_mul(self.gpr[rs2 as usize]);
        Ok(())
    }

    fn mulh(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        let a = self.gpr[rs1 as usize] as i32 as i64;
        let b = self.gpr[rs2 as usize] as i32 as i64;
        self.gpr[rd as usize] = ((a * b) >> 32) as u32;
        Ok(())
    }

    fn mulhu(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        let a = self.gpr[rs1 as usize] as u64;
        let b = self.gpr[rs2 as usize] as u64;
        self.gpr[rd as usize] = ((a * b) >> 32) as u32;
        Ok(())
    }

    fn div(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        let dividend = self.gpr[rs1 as usize] as i32;
        let divisor = self.gpr[rs2 as usize] as i32;
        if divisor == 0 {
            self.gpr[rd as usize] = u32::MAX;
        } else if dividend == i32::MIN && divisor == -1 {
            self.gpr[rd as usize] = dividend as u32;
        } else {
            self.gpr[rd as usize] = (dividend / divisor) as u32;
        }
        Ok(())
    }

    fn divu(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        let dividend = self.gpr[rs1 as usize];
        let divisor = self.gpr[rs2 as usize];
        if divisor == 0 {
            self.gpr[rd as usize] = u32::MAX;
        } else {
            self.gpr[rd as usize] = dividend / divisor;
        }
        Ok(())
    }

    fn rem(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        let dividend = self.gpr[rs1 as usize] as i32;
        let divisor = self.gpr[rs2 as usize] as i32;
        if divisor == 0 {
            self.gpr[rd as usize] = dividend as u32;
        } else if dividend == i32::MIN && divisor == -1 {
            self.gpr[rd as usize] = 0;
        } else {
            self.gpr[rd as usize] = (dividend % divisor) as u32;
        }
        Ok(())
    }

    fn remu(&mut self, rd: u8, rs1: u8, rs2: u8) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        let dividend = self.gpr[rs1 as usize];
        let divisor = self.gpr[rs2 as usize];
        if divisor == 0 {
            self.gpr[rd as usize] = dividend;
        } else {
            self.gpr[rd as usize] = dividend % divisor;
        }
        Ok(())
    }

    fn mret(&mut self, _rd: u8, _rs1: u8, _rs2: u8) -> XResult {
        Ok(())
    }

    fn addi(&mut self, rd: u8, rs1: u8, imm: i32) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = self.gpr[rs1 as usize].wrapping_add(imm as _);
        Ok(())
    }

    fn slli(&mut self, rd: u8, rs1: u8, imm: i32) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        let shamt = (imm & 0x1F) as u32;
        self.gpr[rd as usize] = self.gpr[rs1 as usize] << shamt;
        Ok(())
    }

    fn slti(&mut self, rd: u8, rs1: u8, imm: i32) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = if (self.gpr[rs1 as usize] as i32) < imm {
            1
        } else {
            0
        };
        Ok(())
    }

    fn sltiu(&mut self, rd: u8, rs1: u8, imm: i32) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = if self.gpr[rs1 as usize] < (imm as u32) {
            1
        } else {
            0
        };
        Ok(())
    }

    fn xori(&mut self, rd: u8, rs1: u8, imm: i32) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = self.gpr[rs1 as usize] ^ (imm as u32);
        Ok(())
    }

    fn srli(&mut self, rd: u8, rs1: u8, imm: i32) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        let shamt = (imm & 0x1F) as u32;
        self.gpr[rd as usize] = self.gpr[rs1 as usize] >> shamt;
        Ok(())
    }

    fn srla(&mut self, rd: u8, rs1: u8, imm: i32) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        let shamt = (imm & 0x1F) as u32;
        self.gpr[rd as usize] = ((self.gpr[rs1 as usize] as i32) >> shamt) as u32;
        Ok(())
    }

    fn ori(&mut self, rd: u8, rs1: u8, imm: i32) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = self.gpr[rs1 as usize] | (imm as u32);
        Ok(())
    }

    fn andi(&mut self, rd: u8, rs1: u8, imm: i32) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = self.gpr[rs1 as usize] & (imm as u32);
        Ok(())
    }

    fn lb(&mut self, _rd: u8, _rs1: u8, _imm: i32) -> XResult {
        Err(XError::Unimplemented)
    }

    fn lh(&mut self, _rd: u8, _rs1: u8, _imm: i32) -> XResult {
        Err(XError::Unimplemented)
    }

    fn lw(&mut self, _rd: u8, _rs1: u8, _imm: i32) -> XResult {
        Err(XError::Unimplemented)
    }

    fn lbu(&mut self, _rd: u8, _rs1: u8, _imm: i32) -> XResult {
        Err(XError::Unimplemented)
    }

    fn lhu(&mut self, _rd: u8, _rs1: u8, _imm: i32) -> XResult {
        Err(XError::Unimplemented)
    }

    fn jalr(&mut self, _rd: u8, _rs1: u8, _imm: i32) -> XResult {
        Err(XError::Unimplemented)
    }

    fn csrrw(&mut self, _rd: u8, _rs1: u8, _imm: i32) -> XResult {
        Err(XError::Unimplemented)
    }

    fn csrrs(&mut self, _rd: u8, _rs1: u8, _imm: i32) -> XResult {
        Err(XError::Unimplemented)
    }

    fn csrrc(&mut self, _rd: u8, _rs1: u8, _imm: i32) -> XResult {
        Err(XError::Unimplemented)
    }

    fn sb(&mut self, _rs1: u8, _rs2: u8, _imm: i32) -> XResult {
        Err(XError::Unimplemented)
    }

    fn sh(&mut self, _rs1: u8, _rs2: u8, _imm: i32) -> XResult {
        Err(XError::Unimplemented)
    }

    fn sw(&mut self, _rs1: u8, _rs2: u8, _imm: i32) -> XResult {
        Err(XError::Unimplemented)
    }

    fn beq(&mut self, rs1: u8, rs2: u8, imm: i32) -> XResult {
        if self.gpr[rs1 as usize] == self.gpr[rs2 as usize] {
            self.gpr[RVReg::ra as usize] = self.gpr[RVReg::ra as usize]
                .wrapping_add(imm as u32)
                .wrapping_sub(4);
        }
        Ok(())
    }

    fn bne(&mut self, rs1: u8, rs2: u8, imm: i32) -> XResult {
        if self.gpr[rs1 as usize] != self.gpr[rs2 as usize] {
            self.gpr[RVReg::ra as usize] = self.gpr[RVReg::ra as usize]
                .wrapping_add(imm as u32)
                .wrapping_sub(4);
        }
        Ok(())
    }

    fn blt(&mut self, rs1: u8, rs2: u8, imm: i32) -> XResult {
        if (self.gpr[rs1 as usize] as i32) < (self.gpr[rs2 as usize] as i32) {
            self.gpr[RVReg::ra as usize] = self.gpr[RVReg::ra as usize]
                .wrapping_add(imm as u32)
                .wrapping_sub(4);
        }
        Ok(())
    }

    fn bge(&mut self, rs1: u8, rs2: u8, imm: i32) -> XResult {
        if (self.gpr[rs1 as usize] as i32) >= (self.gpr[rs2 as usize] as i32) {
            self.gpr[RVReg::ra as usize] = self.gpr[RVReg::ra as usize]
                .wrapping_add(imm as u32)
                .wrapping_sub(4);
        }
        Ok(())
    }

    fn bltu(&mut self, rs1: u8, rs2: u8, imm: i32) -> XResult {
        if self.gpr[rs1 as usize] < self.gpr[rs2 as usize] {
            self.gpr[RVReg::ra as usize] = self.gpr[RVReg::ra as usize]
                .wrapping_add(imm as u32)
                .wrapping_sub(4);
        }
        Ok(())
    }

    fn bgeu(&mut self, rs1: u8, rs2: u8, imm: i32) -> XResult {
        if self.gpr[rs1 as usize] >= self.gpr[rs2 as usize] {
            self.gpr[RVReg::ra as usize] = self.gpr[RVReg::ra as usize]
                .wrapping_add(imm as u32)
                .wrapping_sub(4);
        }
        Ok(())
    }

    fn lui(&mut self, rd: u8, imm: i32) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = (imm as u32) << 12;
        Ok(())
    }

    fn auipc(&mut self, rd: u8, imm: i32) -> XResult {
        if rd == 0 {
            return Ok(());
        }
        self.gpr[rd as usize] = self.gpr[RVReg::ra as usize].wrapping_add((imm as u32) << 12);
        Ok(())
    }

    fn jal(&mut self, rd: u8, imm: i32) -> XResult {
        if rd != 0 {
            self.gpr[rd as usize] = self.gpr[RVReg::ra as usize];
        }
        self.gpr[RVReg::ra as usize] = self.gpr[RVReg::ra as usize]
            .wrapping_add(imm as u32)
            .wrapping_sub(4);
        Ok(())
    }

    fn ebreak(&mut self, _rd: u8, _rs1: u8, _imm: i32) -> XResult {
        self.gpr[RVReg::a0] = 0;
        Err(XError::ToTerminate)
    }
}
