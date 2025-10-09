use std::sync::LazyLock;

use itertools::Itertools;
use pest::Parser;
use pest_derive::Parser;

use crate::{
    config::SWord,
    error::{XError, XResult},
    isa::{
        riscv::util::{bit_slice_u32, sign_extend_u32},
        RVReg,
    },
};

pub static DECODER: LazyLock<RVDecoder> = LazyLock::new(|| {
    RVDecoder::from_instpat(include_str!("../instpat/riscv.instpat"))
        .expect("Failed to load instruction patterns")
});

#[derive(Parser)]
#[grammar = "src/isa/instpat/riscv.pest"]
struct RVParser;

#[derive(Debug, Clone)]
pub struct InstPattern {
    name: String,
    inst_type: String,
    mask: u32,
    value: u32,
}

impl InstPattern {
    pub fn from_pattern(
        pattern_str: &str,
        name: String,
        inst_type: String,
    ) -> XResult<InstPattern> {
        let pattern_str = pattern_str.replace(" ", "");
        if pattern_str.len() != 32 {
            return Err(XError::PatternError);
        }

        let (mask, value) = pattern_str
            .chars()
            .try_fold((0u32, 0u32), |(mask, value), ch| {
                let (new_mask, new_value) = match ch {
                    '0' => (mask << 1 | 1, value << 1),
                    '1' => (mask << 1 | 1, value << 1 | 1),
                    '?' => (mask << 1, value << 1),
                    _ => return Err(XError::PatternError),
                };
                Ok((new_mask, new_value))
            })?;

        Ok(InstPattern {
            name,
            inst_type,
            mask,
            value,
        })
    }

    pub fn matches(&self, instruction: u32) -> bool {
        (instruction & self.mask) == self.value
    }
}

#[derive(Debug, Default)]
pub struct RVDecoder {
    patterns: Vec<InstPattern>,
}

impl RVDecoder {
    pub fn from_instpat(instpat_code: &str) -> XResult<RVDecoder> {
        Ok(Self {
            patterns: RVParser::parse(Rule::file, instpat_code)
                .map_err(|_| XError::ParseError)?
                .next()
                .ok_or(XError::ParseError)?
                .into_inner()
                .filter(|p| p.as_rule() == Rule::instpat_line)
                .map(|pair| {
                    pair.into_inner()
                        .map(|p| p.as_str())
                        .collect_tuple::<(&str, &str, &str)>()
                        .ok_or(XError::ParseError)
                        .and_then(|(pattern, name, inst_type)| {
                            InstPattern::from_pattern(
                                pattern,
                                name.to_string(),
                                inst_type.to_string(),
                            )
                        })
                })
                .collect::<Result<_, _>>()?,
        })
    }

    pub fn decode(&self, instruction: u32) -> XResult<DecodedInst> {
        self.patterns
            .iter()
            .find(|p| p.matches(instruction))
            .ok_or(XError::DecodeError)
            .map(|pattern| {
                DecodedInst::decoded_from(&pattern.inst_type, instruction, pattern.name.clone())
            })?
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[rustfmt::skip]
pub enum DecodedInst {
    R { inst: String, rd: RVReg, rs1: RVReg, rs2: RVReg },
    I { inst: String, rd: RVReg, rs1: RVReg, imm: SWord },
    S { inst: String, rs1: RVReg, rs2: RVReg, imm: SWord },
    B { inst: String, rs1: RVReg, rs2: RVReg, imm: SWord },
    U { inst: String, rd: RVReg, imm: SWord },
    J { inst: String, rd: RVReg, imm: SWord },
}

impl DecodedInst {
    fn decoded_from(inst_type: &str, inst: u32, name: String) -> XResult<Self> {
        let bits = |hi: u8, lo: u8| bit_slice_u32(inst, hi, lo);
        let sext = |value: u32, width: u8| sign_extend_u32(value, width) as SWord;

        let rd = bits(11, 7)
            .try_into()
            .map_err(|_| XError::DecodeError)?;
        let rs1 = bits(19, 15)
            .try_into()
            .map_err(|_| XError::DecodeError)?;
        let rs2 = bits(24, 20)
            .try_into()
            .map_err(|_| XError::DecodeError)?;

        use DecodedInst::*;
        let decoded = match inst_type {
            "R" => R {
                inst: name,
                rd,
                rs1,
                rs2,
            },

            "I" => I {
                inst: name,
                rd,
                rs1,
                imm: sext(bits(31, 20), 12),
            },

            "S" => S {
                inst: name,
                rs1,
                rs2,
                imm: sext((bits(31, 25) << 5) | bits(11, 7), 12),
            },

            "B" => B {
                inst: name,
                rs1,
                rs2,
                imm: sext(
                    (bits(31, 31) << 12)
                        | (bits(7, 7) << 11)
                        | (bits(30, 25) << 5)
                        | (bits(11, 8) << 1),
                    13,
                ),
            },

            "U" => U {
                inst: name,
                rd,
                imm: ((bits(31, 12) as i32) << 12) as SWord,
            },

            "J" => J {
                inst: name,
                rd,
                imm: sext(
                    (bits(31, 31) << 20)
                        | (bits(19, 12) << 12)
                        | (bits(20, 20) << 11)
                        | (bits(30, 21) << 1),
                    21,
                ),
            },

            _ => return Err(XError::DecodeError),
        };

        Ok(decoded)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decode_basic_instructions() {
        let decoder = &*DECODER;

        // add x1, x2, x3
        let add = 0b0000000_00011_00010_000_00001_0110011_u32;
        match decoder.decode(add).unwrap() {
            DecodedInst::R { inst, rd, rs1, rs2 } => {
                assert_eq!(inst, "add");
                assert_eq!(rd, RVReg::ra);
                assert_eq!(rs1, RVReg::sp);
                assert_eq!(rs2, RVReg::gp);
            }
            _ => panic!("expected R-type add"),
        }

        // addi x5, x0, -1
        let addi = 0b111111111111_00000_000_00101_0010011_u32;
        match decoder.decode(addi).unwrap() {
            DecodedInst::I { inst, rd, rs1, imm } => {
                assert_eq!(inst, "addi");
                assert_eq!(rd, RVReg::t0);
                assert_eq!(rs1, RVReg::zero);
                assert_eq!(imm, -1);
            }
            _ => panic!("expected I-type addi"),
        }

        // jal x1, 0x20
        let jal = (0b0000010000 << 21) | ((RVReg::ra as u32) << 7) | 0b1101111;
        match decoder.decode(jal).unwrap() {
            DecodedInst::J { inst, rd, imm } => {
                assert_eq!(inst, "jal");
                assert_eq!(rd, RVReg::ra);
                assert_eq!(imm, 0x20);
            }
            _ => panic!("expected J-type jal"),
        }

        let unknown = 0xFFFF_FFFF;
        assert!(matches!(decoder.decode(unknown), Err(XError::DecodeError)));
    }
}
