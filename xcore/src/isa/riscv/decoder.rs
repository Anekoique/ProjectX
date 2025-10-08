use std::sync::LazyLock;

use pest::Parser;
use pest_derive::Parser;

use crate::error::{XError, XResult};

pub static DECODER: LazyLock<RVDecoder> =
    LazyLock::new(|| RVDecoder::from_instpat(include_str!("../instpat/riscv.instpat")).unwrap());

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
        let patterns = RVParser::parse(Rule::file, instpat_code)
            .map_err(|_| XError::ParseError)?
            .next()
            .unwrap()
            .into_inner()
            .filter(|pair| pair.as_rule() == Rule::instpat_line)
            .map(|pair| -> XResult<InstPattern> {
                let mut inner_rules = pair.into_inner();
                let pattern_str = inner_rules.next().unwrap().as_str();
                let name = inner_rules.next().unwrap().as_str().to_string();
                let inst_type = inner_rules.next().unwrap().as_str().to_string();

                let pattern = InstPattern::from_pattern(pattern_str, name, inst_type)?;
                Ok(pattern)
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self { patterns })
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
    R { inst: String, rd: u8, rs1: u8, rs2: u8 },
    I { inst: String, rd: u8, rs1: u8, imm: i32 },
    S { inst: String, rs1: u8, rs2: u8, imm: i32 },
    B { inst: String, rs1: u8, rs2: u8, imm: i32 },
    U { inst: String, rd: u8, imm: i32 },
    J { inst: String, rd: u8, imm: i32 },
}

impl DecodedInst {
    fn decoded_from(inst_type: &str, inst: u32, name: String) -> XResult<Self> {
        let rd = ((inst >> 7) & 0x1F) as u8;
        let rs1 = ((inst >> 15) & 0x1F) as u8;
        let rs2 = ((inst >> 20) & 0x1F) as u8;

        match inst_type {
            "R" => Ok(DecodedInst::R {
                inst: name,
                rd,
                rs1,
                rs2,
            }),
            "I" => {
                let imm = (inst as i32) >> 20;
                Ok(DecodedInst::I {
                    inst: name,
                    rd,
                    rs1,
                    imm,
                })
            }
            "S" => {
                let imm_11_5 = (inst >> 25) & 0x7F;
                let imm_4_0 = (inst >> 7) & 0x1F;
                let imm = (imm_11_5 << 5) | imm_4_0;
                let imm = ((imm as i32) << 20) >> 20;
                Ok(DecodedInst::S {
                    inst: name,
                    rs2,
                    rs1,
                    imm,
                })
            }
            "B" => {
                let imm_12 = (inst >> 31) & 1;
                let imm_10_5 = (inst >> 25) & 0x3F;
                let imm_4_1 = (inst >> 8) & 0xF;
                let imm_11 = (inst >> 7) & 1;
                let imm = (imm_12 << 12) | (imm_11 << 11) | (imm_10_5 << 5) | (imm_4_1 << 1);
                let imm = ((imm as i32) << 19) >> 19;
                Ok(DecodedInst::B {
                    inst: name,
                    rs1,
                    rs2,
                    imm,
                })
            }
            "U" => {
                let imm = inst & 0xFFFFF000;
                Ok(DecodedInst::U {
                    inst: name,
                    rd,
                    imm: (imm as i32) >> 12,
                })
            }
            "J" => {
                let imm_20 = (inst >> 31) & 1;
                let imm_10_1 = (inst >> 21) & 0x3FF;
                let imm_11 = (inst >> 20) & 1;
                let imm_19_12 = (inst >> 12) & 0xFF;
                let imm = (imm_20 << 20) | (imm_19_12 << 12) | (imm_11 << 11) | (imm_10_1 << 1);
                let imm = ((imm as i32) << 11) >> 11;
                Ok(DecodedInst::J {
                    inst: name,
                    rd,
                    imm,
                })
            }
            _ => Err(XError::DecodeError),
        }
    }
}
