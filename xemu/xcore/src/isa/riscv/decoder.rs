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
struct InstPattern {
    kind: InstKind,
    format: InstFormat,
    mask: u32,
    value: u32,
}

impl InstPattern {
    fn parse(pattern_str: &str, name: &str, inst_type: &str) -> XResult<Self> {
        let kind = InstKind::from_name(name)?;
        let format = inst_type.parse::<InstFormat>()?;
        let (mask, value) = pattern_str.bytes().filter(|&b| b != b' ').try_fold(
            (0u32, 0u32),
            |(m, v), ch| match ch {
                b'0' => Ok((m << 1 | 1, v << 1)),
                b'1' => Ok((m << 1 | 1, v << 1 | 1)),
                b'?' => Ok((m << 1, v << 1)),
                _ => Err(XError::PatternError),
            },
        )?;
        Ok(Self {
            kind,
            format,
            mask,
            value,
        })
    }

    #[inline]
    fn matches(&self, inst: u32) -> bool {
        (inst & self.mask) == self.value
    }

    /// Whether funct3 (bits[14:12]) is fully fixed in the mask.
    fn has_fixed_funct3(&self) -> bool {
        (self.mask >> 12) & 0x7 == 0x7
    }
}

// ---------------------------------------------------------------------------
// Decoder: two-level lookup tables
//
// 32-bit instructions:  key = (opcode[6:2] << 3) | funct3   → 256 buckets
// 16-bit compressed:    key = (funct3 << 2) | quadrant[1:0]  →  32 buckets
//
// Most buckets hold 0–1 patterns (O(1) decode). R-type buckets that share
// opcode+funct3 (add/sub, srl/sra, mul/div) hold 2–3 patterns resolved by
// a short linear scan on funct7.
// ---------------------------------------------------------------------------

const TABLE_32_SIZE: usize = 256; // 5-bit opcode[6:2] × 3-bit funct3
const TABLE_16_SIZE: usize = 32; // 3-bit funct3 × 2-bit quadrant

pub struct RVDecoder {
    table_32: Box<[Vec<InstPattern>; TABLE_32_SIZE]>,
    table_16: Box<[Vec<InstPattern>; TABLE_16_SIZE]>,
}

impl RVDecoder {
    pub fn from_instpat(instpat_code: &str) -> XResult<Self> {
        let table_32 = Box::new(std::array::from_fn(|_| Vec::new()));
        let table_16 = Box::new(std::array::from_fn(|_| Vec::new()));
        let mut decoder = Self { table_32, table_16 };

        for pair in RVParser::parse(Rule::file, instpat_code)
            .map_err(|_| XError::ParseError)?
            .next()
            .ok_or(XError::ParseError)?
            .into_inner()
            .filter(|p| p.as_rule() == Rule::instpat_line)
        {
            let (pat, name, fmt) = pair
                .into_inner()
                .map(|p| p.as_str())
                .collect_tuple()
                .ok_or(XError::ParseError)?;
            decoder.insert(InstPattern::parse(pat, name, fmt)?);
        }
        Ok(decoder)
    }

    fn insert(&mut self, pat: InstPattern) {
        if pat.format.is_compressed() {
            self.table_16[Self::key_16(pat.value)].push(pat);
        } else if pat.has_fixed_funct3() {
            self.table_32[Self::key_32(pat.value)].push(pat);
        } else {
            // U/J-type: funct3 bits are don't-care, broadcast to all 8 funct3 slots
            let opcode_hi = (pat.value >> 2) & 0x1F;
            for funct3 in 0..8u32 {
                self.table_32[((opcode_hi << 3) | funct3) as usize].push(pat);
            }
        }
    }

    #[inline]
    pub fn decode(&self, inst: u32) -> XResult<DecodedInst> {
        let table = if (inst & 0b11) != 0b11 {
            &self.table_16[Self::key_16(inst)]
        } else {
            &self.table_32[Self::key_32(inst)]
        };
        table
            .iter()
            .find(|p| p.matches(inst))
            .ok_or(XError::DecodeError)
            .and_then(|p| DecodedInst::from_raw(p.format, inst, p.kind))
    }

    #[inline]
    fn key_32(inst: u32) -> usize {
        let opcode_hi = (inst >> 2) & 0x1F; // bits[6:2]
        let funct3 = (inst >> 12) & 0x7; // bits[14:12]
        ((opcode_hi << 3) | funct3) as usize
    }

    #[inline]
    fn key_16(inst: u32) -> usize {
        let quadrant = inst & 0b11;
        let funct3 = (inst >> 13) & 0b111;
        ((funct3 << 2) | quadrant) as usize
    }
}

// ---------------------------------------------------------------------------
// Decoded instruction
// ---------------------------------------------------------------------------

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
            R { kind, rd, rs1, rs2 } => write!(f, "{kind:?} {rd:?}, {rs1:?}, {rs2:?}"),
            I { kind, rd, rs1, imm } => write!(f, "{kind:?} {rd:?}, {rs1:?}, {imm:#x}"),
            S { kind, rs1, rs2, imm } => write!(f, "{kind:?} {rs1:?}, {rs2:?}, {imm:#x}"),
            B { kind, rs1, rs2, imm } => write!(f, "{kind:?} {rs1:?}, {rs2:?}, {imm:#x}"),
            U { kind, rd, imm }       => write!(f, "{kind:?} {rd:?}, {imm:#x}"),
            J { kind, rd, imm }       => write!(f, "{kind:?} {rd:?}, {imm:#x}"),
            C { kind, inst }          => write!(f, "{kind:?} {inst:?}"),
        }
    }
}

impl DecodedInst {
    fn from_raw(format: InstFormat, inst: u32, kind: InstKind) -> XResult<Self> {
        let reg = |pos: u32| {
            RVReg::try_from(((inst >> pos) & 0x1F) as u8).map_err(|_| XError::InvalidReg)
        };
        let bits = |hi: u8, lo: u8| bit_u32(inst, hi, lo);
        let sext = |val: u32, width: u8| sext_u32(val, width) as SWord;

        match format {
            InstFormat::R => Ok(Self::R {
                kind,
                rd: reg(7)?,
                rs1: reg(15)?,
                rs2: reg(20)?,
            }),
            InstFormat::I => Ok(Self::I {
                kind,
                rd: reg(7)?,
                rs1: reg(15)?,
                imm: sext(bits(31, 20), 12),
            }),
            InstFormat::S => Ok(Self::S {
                kind,
                rs1: reg(15)?,
                rs2: reg(20)?,
                imm: sext((bits(31, 25) << 5) | bits(11, 7), 12),
            }),
            InstFormat::B => Ok(Self::B {
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
            }),
            InstFormat::U => Ok(Self::U {
                kind,
                rd: reg(7)?,
                imm: sext(inst & 0xFFFFF000, 32),
            }),
            InstFormat::J => Ok(Self::J {
                kind,
                rd: reg(7)?,
                imm: sext(
                    (bits(31, 31) << 20)
                        | (bits(19, 12) << 12)
                        | (bits(20, 20) << 11)
                        | (bits(30, 21) << 1),
                    21,
                ),
            }),
            _ => Ok(Self::C { kind, inst }),
        }
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
