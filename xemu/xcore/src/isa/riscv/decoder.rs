use std::{
    fmt::{Debug, Formatter},
    sync::LazyLock,
};

use itertools::Itertools;
use pest::Parser;
use pest_derive::Parser;

use crate::{
    config::SWord,
    error::{XError, XResult},
    isa::{InstFormat, InstKind, RVReg},
    utils::{bit_u32, sext_u32},
};

pub static DECODER: LazyLock<RVDecoder> = LazyLock::new(|| {
    RVDecoder::from_instpat(include_str!("../instpat/riscv.instpat"))
        .expect("Failed to load instruction patterns")
});

#[derive(Parser)]
#[grammar = "src/isa/instpat/riscv.pest"]
struct RVParser;

#[derive(Debug, Clone, Copy)]
pub struct InstPattern {
    kind: InstKind,
    format: InstFormat,
    mask: u32,
    value: u32,
}

impl InstPattern {
    pub fn from_pattern(pattern_str: &str, name: &str, inst_type: &str) -> XResult<InstPattern> {
        let pattern_str = pattern_str.replace(" ", "");
        let kind = InstKind::from_name(name)?;
        let format = inst_type.parse::<InstFormat>()?;

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

        trace!(
            "Loaded instruction pattern: {:<32} => kind: {:<11}, format: {:<4}, mask: {:#034b}, \
             value: {:#034b}",
            pattern_str,
            format!("{:?}", kind),
            format!("{:?}", format),
            mask,
            value
        );
        Ok(InstPattern {
            kind,
            format,
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
                            InstPattern::from_pattern(pattern, name, inst_type)
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
            .map(|pattern| DecodedInst::decoded_from(pattern.format, instruction, pattern.kind))?
    }
}

#[derive(Clone, PartialEq, Eq)]
#[rustfmt::skip]
pub enum DecodedInst {
    R { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg },
    I { kind: InstKind, rd: RVReg, rs1: RVReg, imm: SWord },
    S { kind: InstKind, rs1: RVReg, rs2: RVReg, imm: SWord },
    B { kind: InstKind, rs1: RVReg, rs2: RVReg, imm: SWord },
    U { kind: InstKind, rd: RVReg, imm: SWord },
    J { kind: InstKind, rd: RVReg, imm: SWord },
    C { kind: InstKind, inst: u32 },
}

#[rustfmt::skip]
impl Debug for DecodedInst {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        use DecodedInst::*;
        match self {
            R { kind, rd, rs1, rs2 } => write!(f, "{:?} {:?}, {:?}, {:?}", kind, rd, rs1, rs2),
            I { kind, rd, rs1, imm } => write!(f, "{:?} {:?}, {:?}, {:#x}", kind, rd, rs1, imm),
            S { kind, rs1, rs2, imm } => write!(f, "{:?} {:?}, {:?}, {:#x}", kind, rs1, rs2, imm),
            B { kind, rs1, rs2, imm } => write!(f, "{:?} {:?}, {:?}, {:#x}", kind, rs1, rs2, imm),
            U { kind, rd, imm } => write!(f, "{:?} {:?}, {:#x}", kind, rd, imm),
            J { kind, rd, imm } => write!(f, "{:?} {:?}, {:#x}", kind, rd, imm),
            C { kind, inst } => write!(f, "{:?} {:?}", kind, inst),
        }
    }
}

impl DecodedInst {
    fn decoded_from(format: InstFormat, inst: u32, kind: InstKind) -> XResult<Self> {
        let reg = |shamt: u32| {
            RVReg::try_from(((inst >> shamt) & 0x1F) as u8).map_err(|_| XError::InvalidReg)
        };
        let bits = |hi: u8, lo: u8| bit_u32(inst, hi, lo);
        let sext = |val: u32, width: u8| sext_u32(val, width) as SWord;

        use DecodedInst::*;
        let decoded = match format {
            InstFormat::R => R {
                kind,
                rd: reg(7)?,
                rs1: reg(15)?,
                rs2: reg(20)?,
            },
            InstFormat::I => I {
                kind,
                rd: reg(7)?,
                rs1: reg(15)?,
                imm: sext(bits(31, 20), 12),
            },
            InstFormat::S => S {
                kind,
                rs1: reg(15)?,
                rs2: reg(20)?,
                imm: sext((bits(31, 25) << 5) | bits(11, 7), 12),
            },
            InstFormat::B => B {
                kind,
                rs1: reg(15)?,
                rs2: reg(20)?,
                imm: sext(
                    (bits(31, 31) << 12)
                        | (bits(7, 7) << 11)
                        | (bits(30, 25) << 5)
                        | (bits(11, 8) << 1),
                    13,
                ),
            },
            InstFormat::U => U {
                kind,
                rd: reg(7)?,
                imm: ((inst as i32 & !0xFFF) as SWord),
            },
            InstFormat::J => J {
                kind,
                rd: reg(7)?,
                imm: sext(
                    (bits(31, 31) << 20)
                        | (bits(19, 12) << 12)
                        | (bits(20, 20) << 11)
                        | (bits(30, 21) << 1),
                    21,
                ),
            },
            _ => C { kind, inst },
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
            DecodedInst::R { kind, rd, rs1, rs2 } => {
                assert_eq!(kind, InstKind::add);
                assert_eq!(rd, RVReg::ra);
                assert_eq!(rs1, RVReg::sp);
                assert_eq!(rs2, RVReg::gp);
            }
            _ => panic!("expected R-type add"),
        }

        // addi x5, x0, -1
        let addi = 0b111111111111_00000_000_00101_0010011_u32;
        match decoder.decode(addi).unwrap() {
            DecodedInst::I { kind, rd, rs1, imm } => {
                assert_eq!(kind, InstKind::addi);
                assert_eq!(rd, RVReg::t0);
                assert_eq!(rs1, RVReg::zero);
                assert_eq!(imm, -1);
            }
            _ => panic!("expected I-type addi"),
        }

        // jal x1, 0x20
        let jal = (0b0000010000 << 21) | ((RVReg::ra as u32) << 7) | 0b1101111;
        match decoder.decode(jal).unwrap() {
            DecodedInst::J { kind, rd, imm } => {
                assert_eq!(kind, InstKind::jal);
                assert_eq!(rd, RVReg::ra);
                assert_eq!(imm, 0x20);
            }
            _ => panic!("expected J-type jal"),
        }

        let unknown = 0xFFFF_FFFF;
        assert!(matches!(decoder.decode(unknown), Err(XError::DecodeError)));
    }
}
