//! F/D floating-point extension handlers.
//!
//! IEEE 754 arithmetic via `softfloat_pure` (pure Rust Berkeley softfloat-3).
//! FP registers are 64-bit NaN-boxed: single-precision values store
//! `0xFFFF_FFFF_xxxx_xxxx`; reads of improperly boxed values yield canonical
//! NaN (`0x7FC0_0000`) per RISC-V spec §12.2.

use softfloat_pure::{
    Float, RoundingMode as SfRm,
    softfloat::{float32_t, float64_t, softfloat_flag_invalid, softfloat_tininess_afterRounding},
};

use super::RVCore;
use crate::{
    config::{SWord, Word},
    cpu::riscv::csr::CsrAddr,
    error::{XError, XResult},
    isa::RVReg,
};

/// RV64-only FP instruction guard. Returns `InvalidInst` on RV32.
fn require_rv64() -> XResult {
    if cfg!(isa32) {
        Err(XError::InvalidInst)
    } else {
        Ok(())
    }
}

type F32 = float32_t;
type F64 = float64_t;

const TININESS: u8 = softfloat_tininess_afterRounding;
const NAN_BOX: u64 = 0xFFFF_FFFF_0000_0000;
const CANONICAL_NAN_F32: u32 = 0x7FC0_0000;

// ---------------------------------------------------------------------------
// Rounding mode resolution (§11.2: rm=7 delegates to frm CSR)
// ---------------------------------------------------------------------------

fn sf_rm(rm: u8) -> Option<SfRm> {
    [
        SfRm::RneTiesToEven,
        SfRm::RtzTowardZero,
        SfRm::RdnTowardNegative,
        SfRm::RupTowardPositive,
        SfRm::RmmTiesToAway,
    ]
    .get(rm as usize)
    .copied()
}

// ---------------------------------------------------------------------------
// NaN-boxing helpers
// ---------------------------------------------------------------------------

#[inline]
fn nan_box(bits: u32) -> u64 {
    NAN_BOX | bits as u64
}

#[inline]
fn unbox(bits: u64) -> u32 {
    if bits >> 32 == 0xFFFF_FFFF {
        bits as u32
    } else {
        CANONICAL_NAN_F32
    }
}

// ---------------------------------------------------------------------------
// FMIN/FMAX per spec §12.6 (sNaN signals, qNaN does not)
// ---------------------------------------------------------------------------

/// FMIN/FMAX per RISC-V spec §12.6.
/// `is_min`: true for FMIN, false for FMAX.
fn fminmax<F: Float>(a: &F, b: &F, is_min: bool) -> (F, u8) {
    let snan = if a.is_signaling_nan() || b.is_signaling_nan() {
        softfloat_flag_invalid
    } else {
        0
    };
    if a.is_nan() && b.is_nan() {
        return (F::from_bits(F::DEFAULT_NAN), snan);
    }
    if a.is_nan() {
        return (F::from_bits(b.to_bits()), snan);
    }
    if b.is_nan() {
        return (F::from_bits(a.to_bits()), snan);
    }
    // Signed-zero tie-break: -0 < +0 for FMIN/FMAX purposes.
    if a.is_zero() && b.is_zero() {
        let a_neg = a.is_negative();
        let pick_a = if is_min { a_neg } else { !a_neg };
        let r = if pick_a { a } else { b };
        return (F::from_bits(r.to_bits()), snan);
    }
    let (a_le_b, cmp_f) = a.le_quiet(b);
    let take_a = if is_min { a_le_b } else { !a_le_b };
    let r = if take_a { a } else { b };
    (F::from_bits(r.to_bits()), snan | cmp_f)
}

// ---------------------------------------------------------------------------
// Classify helper (§12.7)
// ---------------------------------------------------------------------------

fn classify<F: Float>(v: &F) -> Word {
    let predicates: [bool; 10] = [
        v.is_negative_infinity(),
        v.is_negative_normal(),
        v.is_negative_subnormal(),
        v.is_negative_zero(),
        v.is_positive_zero(),
        v.is_positive_subnormal(),
        v.is_positive_normal(),
        v.is_positive_infinity(),
        v.is_signaling_nan(),
        v.is_nan() && !v.is_signaling_nan(),
    ];
    predicates
        .iter()
        .enumerate()
        .fold(0, |acc, (i, &set)| acc | ((set as Word) << i))
}

// ---------------------------------------------------------------------------
// Higher-order operation helpers (modeled after base.rs binary_op/load_op)
// ---------------------------------------------------------------------------

impl RVCore {
    fn resolve_rm(&self, rm: u8) -> XResult<SfRm> {
        let eff = if rm == 7 {
            ((self.csr.get(CsrAddr::fcsr) >> 5) & 0x7) as u8
        } else {
            rm
        };
        sf_rm(eff).ok_or(XError::InvalidInst)
    }

    fn read_f32(&self, reg: RVReg) -> F32 {
        F32::from_bits(unbox(self.fpr[reg as usize]))
    }
    fn read_f64(&self, reg: RVReg) -> F64 {
        F64::from_bits(self.fpr[reg as usize])
    }

    fn write_f32(&mut self, reg: RVReg, val: F32) {
        self.fpr[reg as usize] = nan_box(val.to_bits());
    }
    fn write_f64(&mut self, reg: RVReg, val: F64) {
        self.fpr[reg as usize] = val.to_bits();
    }

    fn accrue(&mut self, flags: u8) {
        if flags != 0 {
            let fcsr = self.csr.get(CsrAddr::fcsr);
            self.csr.set(CsrAddr::fcsr, fcsr | flags as Word);
        }
        self.dirty_fp();
    }

    // --- Rounded binary/unary/cmp helpers (like binary_op in base.rs) ---

    fn f32_binop(
        &mut self,
        rd: RVReg,
        rs1: RVReg,
        rs2: RVReg,
        rm: u8,
        op: impl FnOnce(&F32, &F32, SfRm) -> (F32, u8),
    ) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(rm)?;
        let (r, f) = op(&self.read_f32(rs1), &self.read_f32(rs2), rm);
        self.write_f32(rd, r);
        self.accrue(f);
        Ok(())
    }

    fn f64_binop(
        &mut self,
        rd: RVReg,
        rs1: RVReg,
        rs2: RVReg,
        rm: u8,
        op: impl FnOnce(&F64, &F64, SfRm) -> (F64, u8),
    ) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(rm)?;
        let (r, f) = op(&self.read_f64(rs1), &self.read_f64(rs2), rm);
        self.write_f64(rd, r);
        self.accrue(f);
        Ok(())
    }

    fn f32_unary(
        &mut self,
        rd: RVReg,
        rs1: RVReg,
        rm: u8,
        op: impl FnOnce(&F32, SfRm) -> (F32, u8),
    ) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(rm)?;
        let (r, f) = op(&self.read_f32(rs1), rm);
        self.write_f32(rd, r);
        self.accrue(f);
        Ok(())
    }

    fn f64_unary(
        &mut self,
        rd: RVReg,
        rs1: RVReg,
        rm: u8,
        op: impl FnOnce(&F64, SfRm) -> (F64, u8),
    ) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(rm)?;
        let (r, f) = op(&self.read_f64(rs1), rm);
        self.write_f64(rd, r);
        self.accrue(f);
        Ok(())
    }

    fn f32_cmp(
        &mut self,
        rd: RVReg,
        rs1: RVReg,
        rs2: RVReg,
        op: impl FnOnce(&F32, &F32) -> (bool, u8),
    ) -> XResult {
        self.require_fp()?;
        let (r, f) = op(&self.read_f32(rs1), &self.read_f32(rs2));
        self.accrue(f);
        self.set_gpr(rd, r as Word)
    }

    fn f64_cmp(
        &mut self,
        rd: RVReg,
        rs1: RVReg,
        rs2: RVReg,
        op: impl FnOnce(&F64, &F64) -> (bool, u8),
    ) -> XResult {
        self.require_fp()?;
        let (r, f) = op(&self.read_f64(rs1), &self.read_f64(rs2));
        self.accrue(f);
        self.set_gpr(rd, r as Word)
    }

    // --- Load/Store/Convert helpers ---

    pub(super) fn fload_op(
        &mut self,
        rd: RVReg,
        rs1: RVReg,
        imm: SWord,
        size: usize,
        pack: impl FnOnce(Word) -> u64,
    ) -> XResult {
        self.require_fp()?;
        let addr = self.eff_addr(rs1, imm);
        self.fpr[rd as usize] = pack(self.load(addr, size)?);
        self.dirty_fp();
        Ok(())
    }

    pub(super) fn fstore_op(
        &mut self,
        rs1: RVReg,
        rs2: RVReg,
        imm: SWord,
        size: usize,
        unpack: impl FnOnce(u64) -> Word,
    ) -> XResult {
        self.require_fp()?;
        let addr = self.eff_addr(rs1, imm);
        self.store(addr, size, unpack(self.fpr[rs2 as usize]))
    }

    fn fcvt_f2i(&mut self, rd: RVReg, rm: u8, cvt: impl FnOnce(SfRm) -> (Word, u8)) -> XResult {
        self.require_fp()?;
        let (r, f) = cvt(self.resolve_rm(rm)?);
        self.accrue(f);
        self.set_gpr(rd, r)
    }

    fn fcvt_i2f32(&mut self, rd: RVReg, rm: u8, cvt: impl FnOnce(SfRm) -> (F32, u8)) -> XResult {
        self.require_fp()?;
        let (r, f) = cvt(self.resolve_rm(rm)?);
        self.write_f32(rd, r);
        self.accrue(f);
        Ok(())
    }

    fn fcvt_i2f64(&mut self, rd: RVReg, rm: u8, cvt: impl FnOnce(SfRm) -> (F64, u8)) -> XResult {
        self.require_fp()?;
        let (r, f) = cvt(self.resolve_rm(rm)?);
        self.write_f64(rd, r);
        self.accrue(f);
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Macro: generate S+D handler pairs calling the helpers above.
// ---------------------------------------------------------------------------

/// Binary arithmetic: `fadd`, `fsub`, `fmul`, `fdiv`.
macro_rules! fp_binop {
    ($s:ident, $d:ident, $op:ident) => {
        pub(super) fn $s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rm: u8) -> XResult {
            self.f32_binop(rd, rs1, rs2, rm, |a, b, rm| a.$op(b, rm, TININESS))
        }
        pub(super) fn $d(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rm: u8) -> XResult {
            self.f64_binop(rd, rs1, rs2, rm, |a, b, rm| a.$op(b, rm, TININESS))
        }
    };
}

/// Comparison (FLT/FLE signal on any NaN; FEQ handled separately).
macro_rules! fp_cmp {
    ($s:ident, $d:ident, $op:ident) => {
        pub(super) fn $s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _: u8) -> XResult {
            self.f32_cmp(rd, rs1, rs2, |a, b| a.$op(b))
        }
        pub(super) fn $d(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _: u8) -> XResult {
            self.f64_cmp(rd, rs1, rs2, |a, b| a.$op(b))
        }
    };
}

/// Fused multiply-add with optional sign negation of multiplicand/addend.
macro_rules! fp_fma {
    ($s:ident, $d:ident, $neg_a:expr, $neg_c:expr) => {
        pub(super) fn $s(
            &mut self,
            rd: RVReg,
            rs1: RVReg,
            rs2: RVReg,
            rs3: RVReg,
            rm: u8,
        ) -> XResult {
            self.require_fp()?;
            let rm = self.resolve_rm(rm)?;
            let (a, b, c) = (self.read_f32(rs1), self.read_f32(rs2), self.read_f32(rs3));
            let flip = |v: F32, neg: bool| {
                if neg {
                    F32::from_bits(v.to_bits() ^ 0x8000_0000)
                } else {
                    v
                }
            };
            let (r, f) = flip(a, $neg_a).fused_mul_add(&b, &flip(c, $neg_c), rm, TININESS);
            self.write_f32(rd, r);
            self.accrue(f);
            Ok(())
        }
        pub(super) fn $d(
            &mut self,
            rd: RVReg,
            rs1: RVReg,
            rs2: RVReg,
            rs3: RVReg,
            rm: u8,
        ) -> XResult {
            self.require_fp()?;
            let rm = self.resolve_rm(rm)?;
            let (a, b, c) = (self.read_f64(rs1), self.read_f64(rs2), self.read_f64(rs3));
            let flip = |v: F64, neg: bool| {
                if neg {
                    F64::from_bits(v.to_bits() ^ (1u64 << 63))
                } else {
                    v
                }
            };
            let (r, f) = flip(a, $neg_a).fused_mul_add(&b, &flip(c, $neg_c), rm, TININESS);
            self.write_f64(rd, r);
            self.accrue(f);
            Ok(())
        }
    };
}

/// Sign-injection (no flags, NaN-boxing checked on read).
macro_rules! fp_sgnj {
    ($s:ident, $d:ident, $op32:expr, $op64:expr) => {
        pub(super) fn $s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _: u8) -> XResult {
            self.require_fp()?;
            let r = ($op32)(self.read_f32(rs1).to_bits(), self.read_f32(rs2).to_bits());
            self.write_f32(rd, F32::from_bits(r));
            self.dirty_fp();
            Ok(())
        }
        pub(super) fn $d(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _: u8) -> XResult {
            self.require_fp()?;
            let r = ($op64)(self.read_f64(rs1).to_bits(), self.read_f64(rs2).to_bits());
            self.write_f64(rd, F64::from_bits(r));
            self.dirty_fp();
            Ok(())
        }
    };
}

/// FMIN/FMAX pairs.
macro_rules! fp_minmax {
    ($min_s:ident, $max_s:ident, $min_d:ident, $max_d:ident) => {
        pub(super) fn $min_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _: u8) -> XResult {
            self.f32_binop(rd, rs1, rs2, 0, |a, b, _| fminmax(a, b, true))
        }
        pub(super) fn $max_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _: u8) -> XResult {
            self.f32_binop(rd, rs1, rs2, 0, |a, b, _| fminmax(a, b, false))
        }
        pub(super) fn $min_d(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _: u8) -> XResult {
            self.f64_binop(rd, rs1, rs2, 0, |a, b, _| fminmax(a, b, true))
        }
        pub(super) fn $max_d(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _: u8) -> XResult {
            self.f64_binop(rd, rs1, rs2, 0, |a, b, _| fminmax(a, b, false))
        }
    };
}

// ---------------------------------------------------------------------------
// Instruction handlers — each is a one-liner (or near) calling a helper.
// ---------------------------------------------------------------------------

#[allow(clippy::unnecessary_cast)]
impl RVCore {
    // --- Arithmetic (§12.4) ---
    fp_binop!(fadd_s, fadd_d, add);
    fp_binop!(fsub_s, fsub_d, sub);
    fp_binop!(fmul_s, fmul_d, mul);
    fp_binop!(fdiv_s, fdiv_d, div);

    // --- Sqrt (§12.4, unary) ---
    pub(super) fn fsqrt_s(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        self.f32_unary(rd, rs1, rm, |a, rm| a.sqrt(rm, TININESS))
    }
    pub(super) fn fsqrt_d(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        self.f64_unary(rd, rs1, rm, |a, rm| a.sqrt(rm, TININESS))
    }

    // --- Sign injection (§12.5, no flags, NaN-boxing checked) ---
    fp_sgnj!(
        fsgnj_s,
        fsgnj_d,
        |a: u32, b: u32| (a & 0x7FFF_FFFF) | (b & 0x8000_0000),
        |a: u64, b: u64| (a & !(1u64 << 63)) | (b & (1u64 << 63))
    );
    fp_sgnj!(
        fsgnjn_s,
        fsgnjn_d,
        |a: u32, b: u32| (a & 0x7FFF_FFFF) | (!b & 0x8000_0000),
        |a: u64, b: u64| (a & !(1u64 << 63)) | (!b & (1u64 << 63))
    );
    fp_sgnj!(
        fsgnjx_s,
        fsgnjx_d,
        |a: u32, b: u32| a ^ (b & 0x8000_0000),
        |a: u64, b: u64| a ^ (b & (1u64 << 63))
    );

    // --- Min/Max (§12.6, sNaN signals, qNaN does not) ---
    fp_minmax!(fmin_s, fmax_s, fmin_d, fmax_d);

    // --- Comparison (§12.8) ---
    // FEQ: quiet — NV only on sNaN (uses Float::eq, not PartialEq::eq)
    pub(super) fn feq_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _: u8) -> XResult {
        self.f32_cmp(rd, rs1, rs2, |a, b| Float::eq(a, b))
    }
    pub(super) fn feq_d(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _: u8) -> XResult {
        self.f64_cmp(rd, rs1, rs2, |a, b| Float::eq(a, b))
    }
    // FLT/FLE: signaling — NV on any NaN
    fp_cmp!(flt_s, flt_d, lt);
    fp_cmp!(fle_s, fle_d, le);

    // --- Classify (§12.7) ---
    pub(super) fn fclass_s(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, _: u8) -> XResult {
        self.require_fp()?;
        self.set_gpr(rd, classify(&self.read_f32(rs1)))
    }
    pub(super) fn fclass_d(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, _: u8) -> XResult {
        self.require_fp()?;
        self.set_gpr(rd, classify(&self.read_f64(rs1)))
    }

    // --- FMA (§12.9) ---
    fp_fma!(fmadd_s, fmadd_d, false, false); // rs1*rs2 + rs3
    fp_fma!(fmsub_s, fmsub_d, false, true); // rs1*rs2 - rs3
    fp_fma!(fnmsub_s, fnmsub_d, true, false); // -rs1*rs2 + rs3
    fp_fma!(fnmadd_s, fnmadd_d, true, true); // -rs1*rs2 - rs3

    // --- Move (§12.10, transfer — raw bits, no NaN-boxing check) ---
    pub(super) fn fmv_x_w(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, _: u8) -> XResult {
        self.require_fp()?;
        self.set_gpr(rd, self.fpr[rs1 as usize] as u32 as i32 as Word)
    }
    pub(super) fn fmv_w_x(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, _: u8) -> XResult {
        self.require_fp()?;
        self.fpr[rd as usize] = nan_box(self.gpr[rs1] as u32);
        self.dirty_fp();
        Ok(())
    }
    pub(super) fn fmv_x_d(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, _: u8) -> XResult {
        require_rv64()?;
        self.require_fp()?;
        self.set_gpr(rd, self.fpr[rs1 as usize] as Word)
    }
    pub(super) fn fmv_d_x(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, _: u8) -> XResult {
        require_rv64()?;
        self.require_fp()?;
        self.fpr[rd as usize] = self.gpr[rs1] as u64;
        self.dirty_fp();
        Ok(())
    }

    // --- Load/Store (§12.3) ---
    pub(super) fn flw(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.fload_op(rd, rs1, imm, 4, |v| nan_box(v as u32))
    }
    pub(super) fn fsw(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.fstore_op(rs1, rs2, imm, 4, |v| v as u32 as Word)
    }
    pub(super) fn fld(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.fload_op(rd, rs1, imm, 8, |v| v as u64)
    }
    pub(super) fn fsd(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.fstore_op(rs1, rs2, imm, 8, |v| v as Word)
    }

    // --- FCVT float→int (§12.11) ---
    pub(super) fn fcvt_w_s(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        let a = self.read_f32(rs1);
        self.fcvt_f2i(rd, rm, |rm| {
            let (r, f) = a.to_i32(rm, true);
            (r as i32 as Word, f)
        })
    }
    pub(super) fn fcvt_wu_s(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        let a = self.read_f32(rs1);
        self.fcvt_f2i(rd, rm, |rm| {
            let (r, f) = a.to_u32(rm, true);
            (r as i32 as Word, f)
        })
    }
    pub(super) fn fcvt_l_s(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        require_rv64()?;
        let a = self.read_f32(rs1);
        self.fcvt_f2i(rd, rm, |rm| {
            let (r, f) = a.to_i64(rm, true);
            (r as Word, f)
        })
    }
    pub(super) fn fcvt_lu_s(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        require_rv64()?;
        let a = self.read_f32(rs1);
        self.fcvt_f2i(rd, rm, |rm| {
            let (r, f) = a.to_u64(rm, true);
            (r as Word, f)
        })
    }
    pub(super) fn fcvt_w_d(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        let a = self.read_f64(rs1);
        self.fcvt_f2i(rd, rm, |rm| {
            let (r, f) = a.to_i32(rm, true);
            (r as i32 as Word, f)
        })
    }
    pub(super) fn fcvt_wu_d(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        let a = self.read_f64(rs1);
        self.fcvt_f2i(rd, rm, |rm| {
            let (r, f) = a.to_u32(rm, true);
            (r as i32 as Word, f)
        })
    }
    pub(super) fn fcvt_l_d(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        require_rv64()?;
        let a = self.read_f64(rs1);
        self.fcvt_f2i(rd, rm, |rm| {
            let (r, f) = a.to_i64(rm, true);
            (r as Word, f)
        })
    }
    pub(super) fn fcvt_lu_d(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        require_rv64()?;
        let a = self.read_f64(rs1);
        self.fcvt_f2i(rd, rm, |rm| {
            let (r, f) = a.to_u64(rm, true);
            (r as Word, f)
        })
    }

    // --- FCVT int→float (§12.11) ---
    pub(super) fn fcvt_s_w(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        let v = self.gpr[rs1] as i32;
        self.fcvt_i2f32(rd, rm, |rm| F32::from_i32(v, rm, TININESS))
    }
    pub(super) fn fcvt_s_wu(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        let v = self.gpr[rs1] as u32;
        self.fcvt_i2f32(rd, rm, |rm| F32::from_u32(v, rm, TININESS))
    }
    pub(super) fn fcvt_s_l(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        require_rv64()?;
        let v = self.gpr[rs1] as i64;
        self.fcvt_i2f32(rd, rm, |rm| F32::from_i64(v, rm, TININESS))
    }
    pub(super) fn fcvt_s_lu(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        require_rv64()?;
        let v = self.gpr[rs1] as u64;
        self.fcvt_i2f32(rd, rm, |rm| F32::from_u64(v, rm, TININESS))
    }
    pub(super) fn fcvt_d_w(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        let v = self.gpr[rs1] as i32;
        self.fcvt_i2f64(rd, rm, |rm| F64::from_i32(v, rm, TININESS))
    }
    pub(super) fn fcvt_d_wu(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        let v = self.gpr[rs1] as u32;
        self.fcvt_i2f64(rd, rm, |rm| F64::from_u32(v, rm, TININESS))
    }
    pub(super) fn fcvt_d_l(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        require_rv64()?;
        let v = self.gpr[rs1] as i64;
        self.fcvt_i2f64(rd, rm, |rm| F64::from_i64(v, rm, TININESS))
    }
    pub(super) fn fcvt_d_lu(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        require_rv64()?;
        let v = self.gpr[rs1] as u64;
        self.fcvt_i2f64(rd, rm, |rm| F64::from_u64(v, rm, TININESS))
    }

    // --- Convert between precisions (§12.12) ---
    pub(super) fn fcvt_s_d(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(rm)?;
        let a = self.read_f64(rs1);
        let (r, f) = a.to_f32(rm, TININESS);
        self.write_f32(rd, r);
        self.accrue(f);
        Ok(())
    }
    pub(super) fn fcvt_d_s(&mut self, rd: RVReg, rs1: RVReg, _: RVReg, rm: u8) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(rm)?;
        let a = self.read_f32(rs1);
        let (r, f) = a.to_f64(rm, TININESS);
        self.write_f64(rd, r);
        self.accrue(f);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        config::CONFIG_MBASE,
        cpu::riscv::{
            RVCore,
            csr::{CsrAddr, MStatus},
        },
        isa::RVReg,
    };

    // IEEE 754 bit constants — ground truth independent of implementation.
    const F32_POS_ZERO: u32 = 0x0000_0000;
    const F32_NEG_ZERO: u32 = 0x8000_0000;
    const F32_ONE: u32 = 0x3F80_0000;
    const F32_TWO: u32 = 0x4000_0000;
    const F32_THREE: u32 = 0x4040_0000;
    const F32_FOUR: u32 = 0x4080_0000;
    const F32_TWELVE: u32 = 0x4140_0000;
    const F32_HALF: u32 = 0x3F00_0000;
    const F32_NEG_ONE: u32 = 0xBF80_0000;
    const F32_POS_INF: u32 = 0x7F80_0000;
    const F32_NEG_INF: u32 = 0xFF80_0000;
    const F32_QNAN: u32 = CANONICAL_NAN_F32;
    const F32_SNAN: u32 = 0x7F80_0001;

    const F64_ONE: u64 = 0x3FF0_0000_0000_0000;
    const F64_TWO: u64 = 0x4000_0000_0000_0000;
    const F64_THREE: u64 = 0x4008_0000_0000_0000;
    const F64_FOUR: u64 = 0x4010_0000_0000_0000;
    const F64_TEN: u64 = 0x4024_0000_0000_0000;

    const RNE: u8 = 0;
    const RTZ: u8 = 1;

    // FP register aliases — same 5-bit encoding as GPR (ft0=x0, ft1=x1, ...)
    const F0: RVReg = RVReg::zero; // ft0
    const F1: RVReg = RVReg::ra; // ft1
    const F2: RVReg = RVReg::sp; // ft2
    const F3: RVReg = RVReg::gp; // ft3
    const F4: RVReg = RVReg::tp; // ft4

    fn setup() -> RVCore {
        RVCore::new()
    }

    fn set_f32(core: &mut RVCore, reg: RVReg, bits: u32) {
        core.fpr[reg as usize] = nan_box(bits);
    }

    fn get_f32(core: &RVCore, reg: RVReg) -> u32 {
        core.fpr[reg as usize] as u32
    }

    fn set_f64(core: &mut RVCore, reg: RVReg, bits: u64) {
        core.fpr[reg as usize] = bits;
    }

    fn get_f64(core: &RVCore, reg: RVReg) -> u64 {
        core.fpr[reg as usize]
    }

    fn fflags(core: &RVCore) -> Word {
        core.csr.get(CsrAddr::fcsr) & 0x1F
    }

    fn fs(core: &RVCore) -> Word {
        (core.csr.get(CsrAddr::mstatus) >> 13) & 0x3
    }

    // === NaN-boxing ===

    #[test]
    fn nan_boxing_roundtrip() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_ONE);
        assert_eq!(get_f32(&core, F0), F32_ONE);
        assert_eq!(core.fpr[F0 as usize] >> 32, 0xFFFF_FFFF);
    }

    #[test]
    fn nan_boxing_invalid_yields_canonical_nan() {
        let mut core = setup();
        core.fpr[F0 as usize] = 0x0000_0000_3F80_0000; // not NaN-boxed
        assert_eq!(core.read_f32(F0).to_bits(), CANONICAL_NAN_F32);
    }

    // === F arithmetic ===

    #[test]
    fn fadd_s_basic() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_ONE);
        set_f32(&mut core, F1, F32_TWO);
        core.fadd_s(F2, F0, F1, RNE).unwrap();
        assert_eq!(get_f32(&core, F2), F32_THREE);
    }

    #[test]
    fn fsub_s_basic() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_THREE);
        set_f32(&mut core, F1, F32_ONE);
        core.fsub_s(F2, F0, F1, RNE).unwrap();
        assert_eq!(get_f32(&core, F2), F32_TWO);
    }

    #[test]
    fn fmul_s_basic() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_THREE);
        set_f32(&mut core, F1, F32_FOUR);
        core.fmul_s(F2, F0, F1, RNE).unwrap();
        assert_eq!(get_f32(&core, F2), F32_TWELVE);
    }

    #[test]
    fn fdiv_s_basic() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_FOUR);
        set_f32(&mut core, F1, F32_TWO);
        core.fdiv_s(F2, F0, F1, RNE).unwrap();
        assert_eq!(get_f32(&core, F2), F32_TWO);
    }

    #[test]
    fn fsqrt_s_basic() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_FOUR);
        core.fsqrt_s(F2, F0, RVReg::zero, RNE).unwrap();
        assert_eq!(get_f32(&core, F2), F32_TWO);
    }

    // === D arithmetic ===

    #[test]
    fn fadd_d_basic() {
        let mut core = setup();
        set_f64(&mut core, F0, F64_ONE);
        set_f64(&mut core, F1, F64_TWO);
        core.fadd_d(F2, F0, F1, RNE).unwrap();
        assert_eq!(get_f64(&core, F2), F64_THREE);
    }

    #[test]
    fn fmul_d_basic() {
        let mut core = setup();
        set_f64(&mut core, F0, F64_TWO);
        set_f64(&mut core, F1, F64_FOUR);
        core.fmul_d(F2, F0, F1, RNE).unwrap();
        assert_eq!(get_f64(&core, F2), 0x4020_0000_0000_0000); // 8.0
    }

    // === FMA ===

    #[test]
    fn fmadd_s_basic() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_TWO);
        set_f32(&mut core, F1, F32_THREE);
        set_f32(&mut core, F2, F32_FOUR);
        core.fmadd_s(F3, F0, F1, F2, RNE).unwrap();
        assert_eq!(get_f32(&core, F3), 0x4120_0000); // 10.0
    }

    #[test]
    fn fmadd_d_basic() {
        let mut core = setup();
        set_f64(&mut core, F0, F64_TWO);
        set_f64(&mut core, F1, F64_THREE);
        set_f64(&mut core, F2, F64_FOUR);
        core.fmadd_d(F3, F0, F1, F2, RNE).unwrap();
        assert_eq!(get_f64(&core, F3), F64_TEN);
    }

    #[test]
    fn fmsub_s_basic() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_THREE);
        set_f32(&mut core, F1, F32_TWO);
        set_f32(&mut core, F2, F32_TWO);
        core.fmsub_s(F3, F0, F1, F2, RNE).unwrap();
        assert_eq!(get_f32(&core, F3), F32_FOUR); // 3*2-2=4
    }

    // === Sign injection ===

    #[test]
    fn fsgnj_s_copies_sign() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_ONE);
        set_f32(&mut core, F1, F32_NEG_ONE);
        core.fsgnj_s(F2, F0, F1, 0).unwrap();
        assert_eq!(get_f32(&core, F2), F32_NEG_ONE);
    }

    #[test]
    fn fsgnjn_s_negates_sign() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_ONE);
        set_f32(&mut core, F1, F32_NEG_ONE);
        core.fsgnjn_s(F2, F0, F1, 0).unwrap();
        assert_eq!(get_f32(&core, F2), F32_ONE); // !neg = pos
    }

    #[test]
    fn fsgnjx_s_xors_sign() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_NEG_ONE);
        set_f32(&mut core, F1, F32_NEG_ONE);
        core.fsgnjx_s(F2, F0, F1, 0).unwrap();
        assert_eq!(get_f32(&core, F2), F32_ONE); // 1^1=0 -> positive
    }

    #[test]
    fn fsgnj_s_on_unboxed_uses_canonical_nan() {
        let mut core = setup();
        core.fpr[F0 as usize] = 0x0000_0000_3F80_0000; // not NaN-boxed
        set_f32(&mut core, F1, F32_NEG_ONE);
        core.fsgnj_s(F2, F0, F1, 0).unwrap();
        let r = get_f32(&core, F2);
        assert_eq!(r & 0x7FFF_FFFF, CANONICAL_NAN_F32 & 0x7FFF_FFFF);
        assert_ne!(r & 0x8000_0000, 0, "sign from rs2 must be negative");
    }

    // === Comparison ===

    #[test]
    fn feq_flt_fle_basic() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_ONE);
        set_f32(&mut core, F1, F32_TWO);

        core.feq_s(RVReg::t0, F0, F0, 0).unwrap();
        assert_eq!(core.gpr[RVReg::t0], 1); // 1==1

        core.flt_s(RVReg::t0, F0, F1, 0).unwrap();
        assert_eq!(core.gpr[RVReg::t0], 1); // 1<2

        core.fle_s(RVReg::t0, F1, F0, 0).unwrap();
        assert_eq!(core.gpr[RVReg::t0], 0); // !(2<=1)
    }

    // === Min/Max ===

    #[test]
    fn fmin_fmax_basic() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_THREE);
        set_f32(&mut core, F1, F32_ONE);
        core.fmin_s(F2, F0, F1, 0).unwrap();
        assert_eq!(get_f32(&core, F2), F32_ONE);
        core.fmax_s(F2, F0, F1, 0).unwrap();
        assert_eq!(get_f32(&core, F2), F32_THREE);
    }

    #[test]
    fn fmin_s_neg_zero_less_than_pos_zero() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_NEG_ZERO);
        set_f32(&mut core, F1, F32_POS_ZERO);
        core.fmin_s(F2, F0, F1, 0).unwrap();
        assert_eq!(get_f32(&core, F2), F32_NEG_ZERO);
    }

    // === NaN flag behavior ===

    #[test]
    fn fadd_s_snan_sets_nv() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_SNAN);
        set_f32(&mut core, F1, F32_ONE);
        core.fadd_s(F2, F0, F1, RNE).unwrap();
        assert_ne!(fflags(&core) & 0x10, 0, "sNaN must set NV");
    }

    #[test]
    fn fadd_s_qnan_no_nv() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_QNAN);
        set_f32(&mut core, F1, F32_ONE);
        core.fadd_s(F2, F0, F1, RNE).unwrap();
        assert!(F32::from_bits(get_f32(&core, F2)).is_nan());
        assert_eq!(fflags(&core) & 0x10, 0, "qNaN must not set NV");
    }

    #[test]
    fn feq_s_qnan_no_nv_snan_sets_nv() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_QNAN);
        set_f32(&mut core, F1, F32_QNAN);
        core.feq_s(RVReg::t0, F0, F1, 0).unwrap();
        assert_eq!(core.gpr[RVReg::t0], 0);
        assert_eq!(fflags(&core) & 0x10, 0, "FEQ(qNaN,qNaN) must not set NV");

        let mut core2 = setup();
        set_f32(&mut core2, F0, F32_SNAN);
        set_f32(&mut core2, F1, F32_ONE);
        core2.feq_s(RVReg::t0, F0, F1, 0).unwrap();
        assert_ne!(fflags(&core2) & 0x10, 0, "FEQ(sNaN,*) must set NV");
    }

    #[test]
    fn flt_s_any_nan_sets_nv() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_QNAN);
        set_f32(&mut core, F1, F32_ONE);
        core.flt_s(RVReg::t0, F0, F1, 0).unwrap();
        assert_ne!(fflags(&core) & 0x10, 0, "FLT with any NaN must set NV");
    }

    #[test]
    fn fmin_s_qnan_no_nv_snan_sets_nv() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_QNAN);
        set_f32(&mut core, F1, F32_ONE);
        core.fmin_s(F2, F0, F1, 0).unwrap();
        assert_eq!(get_f32(&core, F2), F32_ONE);
        assert_eq!(fflags(&core) & 0x10, 0, "FMIN(qNaN,x) must not set NV");

        let mut core2 = setup();
        set_f32(&mut core2, F0, F32_SNAN);
        set_f32(&mut core2, F1, F32_ONE);
        core2.fmin_s(F2, F0, F1, 0).unwrap();
        assert_ne!(fflags(&core2) & 0x10, 0, "FMIN(sNaN,x) must set NV");
    }

    #[test]
    fn fmin_s_both_qnan_canonical_no_nv() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_QNAN);
        set_f32(&mut core, F1, F32_QNAN);
        core.fmin_s(F2, F0, F1, 0).unwrap();
        assert!(F32::from_bits(get_f32(&core, F2)).is_nan());
        assert_eq!(fflags(&core) & 0x10, 0, "FMIN(qNaN,qNaN) must not set NV");
    }

    // === Division special cases ===

    #[test]
    fn fdiv_s_by_zero_sets_dz() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_ONE);
        set_f32(&mut core, F1, F32_POS_ZERO);
        core.fdiv_s(F2, F0, F1, RNE).unwrap();
        assert_eq!(get_f32(&core, F2), F32_POS_INF);
        assert_ne!(fflags(&core) & 0x08, 0, "DZ must be set");
    }

    #[test]
    fn fdiv_s_zero_over_zero_sets_nv() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_POS_ZERO);
        set_f32(&mut core, F1, F32_POS_ZERO);
        core.fdiv_s(F2, F0, F1, RNE).unwrap();
        assert!(F32::from_bits(get_f32(&core, F2)).is_nan());
        assert_ne!(fflags(&core) & 0x10, 0, "0/0 must set NV");
    }

    // === Classify ===

    #[test]
    fn fclass_s_all_categories() {
        for &(bits, expected) in &[
            (F32_NEG_INF, 1 << 0),
            (F32_NEG_ONE, 1 << 1),
            (F32_NEG_ZERO, 1 << 3),
            (F32_POS_ZERO, 1 << 4),
            (F32_ONE, 1 << 6),
            (F32_POS_INF, 1 << 7),
            (F32_SNAN, 1 << 8),
            (F32_QNAN, 1 << 9),
        ] {
            let mut core = setup();
            set_f32(&mut core, F0, bits);
            core.fclass_s(RVReg::t0, F0, RVReg::zero, 0).unwrap();
            assert_eq!(core.gpr[RVReg::t0], expected, "fclass({bits:#010x})");
        }
    }

    // === Move ===

    #[test]
    fn fmv_x_w_sign_extends() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_NEG_ONE);
        core.fmv_x_w(RVReg::t0, F0, RVReg::zero, 0).unwrap();
        #[cfg(isa64)]
        assert_eq!(core.gpr[RVReg::t0], 0xFFFF_FFFF_BF80_0000);
        #[cfg(isa32)]
        assert_eq!(core.gpr[RVReg::t0], 0xBF80_0000);
    }

    #[test]
    fn fmv_w_x_nan_boxes() {
        let mut core = setup();
        core.gpr[RVReg::t0] = F32_ONE as Word;
        core.fmv_w_x(F0, RVReg::t0, RVReg::zero, 0).unwrap();
        assert_eq!(core.fpr[F0 as usize], nan_box(F32_ONE));
    }

    #[test]
    #[cfg(isa64)]
    fn fmv_d_roundtrip() {
        let mut core = setup();
        core.gpr[RVReg::a0] = F64_ONE as Word;
        core.fmv_d_x(F1, RVReg::a0, RVReg::zero, 0).unwrap();
        assert_eq!(core.fpr[F1 as usize], F64_ONE);
        core.fmv_x_d(RVReg::a1, F1, RVReg::zero, 0).unwrap();
        assert_eq!(core.gpr[RVReg::a1] as u64, F64_ONE);
    }

    // === Load/Store ===

    #[test]
    fn flw_nan_boxes_and_fsw_roundtrip() {
        let mut core = setup();
        let addr = CONFIG_MBASE;
        core.bus.write(addr, 4, F32_ONE as Word).unwrap();
        core.gpr[RVReg::t0] = addr as Word;
        core.flw(F0, RVReg::t0, 0).unwrap();
        assert_eq!(core.fpr[F0 as usize], nan_box(F32_ONE));
        core.fsw(RVReg::t0, F0, 4).unwrap();
        assert_eq!(core.bus.read(addr + 4, 4).unwrap() as u32, F32_ONE);
    }

    #[test]
    fn f64_register_roundtrip() {
        let mut core = setup();
        set_f64(&mut core, F0, F64_ONE);
        assert_eq!(get_f64(&core, F0), F64_ONE);
        // fcvt_d_s exercises the D read/write path internally
        set_f32(&mut core, F1, F32_ONE);
        core.fcvt_d_s(F2, F1, RVReg::zero, RNE).unwrap();
        assert_eq!(get_f64(&core, F2), F64_ONE);
    }

    // === FCVT ===

    #[test]
    fn fcvt_w_s_and_s_w_roundtrip() {
        let mut core = setup();
        core.gpr[RVReg::t0] = 42;
        core.fcvt_s_w(F0, RVReg::t0, RVReg::zero, RNE).unwrap();
        assert_eq!(get_f32(&core, F0), 0x4228_0000); // 42.0
        core.fcvt_w_s(RVReg::t1, F0, RVReg::zero, RNE).unwrap();
        assert_eq!(core.gpr[RVReg::t1] as i64, 42);
    }

    #[test]
    fn fcvt_w_s_rtz_truncates_half() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_HALF);
        core.fcvt_w_s(RVReg::t0, F0, RVReg::zero, RTZ).unwrap();
        assert_eq!(core.gpr[RVReg::t0] as i64, 0);
    }

    #[test]
    fn fcvt_w_s_overflow_saturates() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_POS_INF);
        core.fcvt_w_s(RVReg::t0, F0, RVReg::zero, RNE).unwrap();
        assert_eq!(core.gpr[RVReg::t0] as i32, i32::MAX);
        assert_ne!(fflags(&core) & 0x10, 0);

        let mut core2 = setup();
        set_f32(&mut core2, F0, F32_NEG_INF);
        core2.fcvt_w_s(RVReg::t0, F0, RVReg::zero, RNE).unwrap();
        assert_eq!(core2.gpr[RVReg::t0] as i32, i32::MIN);

        let mut core3 = setup();
        set_f32(&mut core3, F0, F32_QNAN);
        core3.fcvt_w_s(RVReg::t0, F0, RVReg::zero, RNE).unwrap();
        assert_eq!(core3.gpr[RVReg::t0] as i32, i32::MAX);
    }

    #[test]
    fn fcvt_d_s_widens() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_ONE);
        core.fcvt_d_s(F1, F0, RVReg::zero, RNE).unwrap();
        assert_eq!(get_f64(&core, F1), F64_ONE);
    }

    #[test]
    fn fcvt_s_d_narrows() {
        let mut core = setup();
        set_f64(&mut core, F0, F64_ONE);
        core.fcvt_s_d(F1, F0, RVReg::zero, RNE).unwrap();
        assert_eq!(get_f32(&core, F1), F32_ONE);
    }

    // === mstatus.FS tracking ===

    #[test]
    fn fs_transitions_initial_to_dirty() {
        let mut core = setup();
        assert_eq!(fs(&core), 1);
        set_f32(&mut core, F0, F32_ONE);
        set_f32(&mut core, F1, F32_TWO);
        core.fadd_s(F2, F0, F1, RNE).unwrap();
        assert_eq!(fs(&core), 3);
        assert_ne!(core.csr.get(CsrAddr::mstatus) & MStatus::SD.bits(), 0);
    }

    #[test]
    fn fs_off_traps() {
        let mut core = setup();
        core.csr.set(CsrAddr::mstatus, 0); // FS=Off
        set_f32(&mut core, F0, F32_ONE);
        assert!(core.fadd_s(F2, F0, F0, RNE).is_err());
    }

    #[test]
    fn feq_dirties_fs() {
        let mut core = setup();
        core.csr.set(CsrAddr::mstatus, 1 << 13); // FS=Initial
        set_f32(&mut core, F0, F32_ONE);
        core.feq_s(RVReg::t0, F0, F0, 0).unwrap();
        assert_eq!(fs(&core), 3, "flag-only ops must dirty FS");
    }

    // === FP CSR ===

    #[test]
    fn fcsr_composite_views() {
        let mut core = setup();
        core.csr.set(CsrAddr::fcsr, 0xE5);
        assert_eq!(core.csr_read(CsrAddr::fflags as u16).unwrap(), 0x05);
        assert_eq!(core.csr_read(CsrAddr::frm as u16).unwrap(), 0x07);
        assert_eq!(core.csr_read(CsrAddr::fcsr as u16).unwrap(), 0xE5);
    }

    #[test]
    fn fflags_sticky() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_ONE);
        set_f32(&mut core, F1, F32_POS_ZERO);
        core.fdiv_s(F2, F0, F1, RNE).unwrap();
        assert_ne!(fflags(&core) & 0x08, 0, "DZ set");

        set_f32(&mut core, F0, F32_SNAN);
        set_f32(&mut core, F1, F32_ONE);
        core.fadd_s(F2, F0, F1, RNE).unwrap();
        assert_ne!(fflags(&core) & 0x08, 0, "DZ must remain (sticky)");
        assert_ne!(fflags(&core) & 0x10, 0, "NV also set");
    }

    // === Rounding mode ===

    #[test]
    fn resolve_rm_dynamic() {
        let mut core = setup();
        core.csr.set(CsrAddr::fcsr, 0x02 << 5); // frm=2 (RDN)
        assert!(matches!(
            core.resolve_rm(7).unwrap(),
            SfRm::RdnTowardNegative
        ));
    }

    #[test]
    fn resolve_rm_reserved_rejects() {
        let core = setup();
        assert!(core.resolve_rm(5).is_err());
        assert!(core.resolve_rm(6).is_err());
    }

    // === FMAX signed-zero tie-break (IR-002) ===

    #[test]
    fn fmax_s_pos_zero_greater_than_neg_zero() {
        let mut core = setup();
        // fmax(-0, +0) must return +0
        set_f32(&mut core, F0, F32_NEG_ZERO);
        set_f32(&mut core, F1, F32_POS_ZERO);
        core.fmax_s(F2, F0, F1, 0).unwrap();
        assert_eq!(get_f32(&core, F2), F32_POS_ZERO);
        // Reversed operand order: fmax(+0, -0) must also return +0
        core.fmax_s(F2, F1, F0, 0).unwrap();
        assert_eq!(get_f32(&core, F2), F32_POS_ZERO);
    }

    #[test]
    fn fmin_s_signed_zero_both_orders() {
        let mut core = setup();
        // fmin(+0, -0) must return -0
        set_f32(&mut core, F0, F32_POS_ZERO);
        set_f32(&mut core, F1, F32_NEG_ZERO);
        core.fmin_s(F2, F0, F1, 0).unwrap();
        assert_eq!(get_f32(&core, F2), F32_NEG_ZERO);
        // fmin(-0, +0) must also return -0
        core.fmin_s(F2, F1, F0, 0).unwrap();
        assert_eq!(get_f32(&core, F2), F32_NEG_ZERO);
    }

    // === SD recomputation on FS downgrade (IR-001) ===

    #[test]
    fn sd_clears_when_fs_downgraded() {
        let mut core = setup();
        // Make FS dirty
        set_f32(&mut core, F0, F32_ONE);
        set_f32(&mut core, F1, F32_TWO);
        core.fadd_s(F2, F0, F1, RNE).unwrap();
        assert_eq!(fs(&core), 3); // Dirty
        assert_ne!(core.csr.get(CsrAddr::mstatus) & MStatus::SD.bits(), 0);

        // OS writes mstatus to set FS=Initial (0b01)
        let ms = (core.csr.get(CsrAddr::mstatus) & !MStatus::FS.bits()) | (1 << 13);
        core.csr_write(CsrAddr::mstatus as u16, ms).unwrap();
        assert_eq!(fs(&core), 1); // Initial
        assert_eq!(
            core.csr.get(CsrAddr::mstatus) & MStatus::SD.bits(),
            0,
            "SD must be cleared when FS is no longer Dirty"
        );
    }

    // === FP CSR immediate instruction forms (IR-003) ===

    #[test]
    fn csrrwi_fflags_writes_and_dirties_fs() {
        let mut core = setup();
        core.csr.set(CsrAddr::mstatus, 1 << 13); // FS=Initial
        // csrrwi t0, fflags, 0x1F  (uimm=31 encoded as rs1=x31=t6)
        let fflags_addr = CsrAddr::fflags as SWord;
        core.csrrwi(RVReg::t0, RVReg::t6, fflags_addr).unwrap();
        assert_eq!(core.gpr[RVReg::t0], 0); // old fflags was 0
        assert_eq!(core.csr_read(CsrAddr::fflags as u16).unwrap(), 31); // new fflags
        assert_eq!(fs(&core), 3, "csrrwi fflags must dirty FS");
    }

    #[test]
    fn csrrsi_frm_sets_bits() {
        let mut core = setup();
        core.csr.set(CsrAddr::fcsr, 0x20); // frm=1 (RTZ)
        // csrrsi t0, frm, 0x04  (uimm=4 encoded as rs1=x4=tp)
        let frm_addr = CsrAddr::frm as SWord;
        core.csrrsi(RVReg::t0, RVReg::tp, frm_addr).unwrap();
        assert_eq!(core.gpr[RVReg::t0], 1); // old frm=1
        assert_eq!(core.csr_read(CsrAddr::frm as u16).unwrap(), 5); // 1 | 4 = 5
    }

    #[test]
    fn csrrci_fcsr_clears_bits() {
        let mut core = setup();
        core.csr.set(CsrAddr::fcsr, 0xFF);
        // csrrci t0, fcsr, 0x0F  (uimm=15 encoded as rs1=x15=a5)
        let fcsr_addr = CsrAddr::fcsr as SWord;
        core.csrrci(RVReg::t0, RVReg::a5, fcsr_addr).unwrap();
        assert_eq!(core.gpr[RVReg::t0] as u8, 0xFF); // old fcsr
        assert_eq!(core.csr_read(CsrAddr::fcsr as u16).unwrap(), 0xF0); // 0xFF & ~0x0F
    }

    #[test]
    fn csrrsi_fflags_uimm_zero_no_write() {
        let mut core = setup();
        core.csr.set(CsrAddr::fcsr, 0x15);
        core.csr.set(CsrAddr::mstatus, 1 << 13); // FS=Initial
        // csrrsi t0, fflags, 0  (uimm=0 encoded as rs1=x0=zero → no write)
        let fflags_addr = CsrAddr::fflags as SWord;
        core.csrrsi(RVReg::t0, RVReg::zero, fflags_addr).unwrap();
        assert_eq!(core.gpr[RVReg::t0], 0x15); // reads fflags=0x15
        assert_eq!(fs(&core), 1, "uimm=0 must NOT dirty FS (read-only access)");
    }

    // === RV64-only FP instructions rejected on RV32 ===

    #[test]
    #[cfg(isa32)]
    fn rv64_only_fp_instructions_rejected_on_rv32() {
        let mut core = setup();
        set_f32(&mut core, F0, F32_ONE);
        set_f64(&mut core, F1, F64_ONE);
        core.gpr[RVReg::t0] = 42;

        // FCVT.L[U].S, FCVT.S.L[U] — RV64F
        for op in [
            RVCore::fcvt_l_s,
            RVCore::fcvt_lu_s,
            RVCore::fcvt_s_l,
            RVCore::fcvt_s_lu,
        ] {
            assert!(
                matches!(
                    op(&mut core, F2, F0, RVReg::zero, RNE),
                    Err(XError::InvalidInst)
                ),
                "RV64F instruction must reject on RV32"
            );
        }

        // FCVT.L[U].D, FCVT.D.L[U], FMV.X.D, FMV.D.X — RV64D
        for op in [
            RVCore::fcvt_l_d,
            RVCore::fcvt_lu_d,
            RVCore::fcvt_d_l,
            RVCore::fcvt_d_lu,
            RVCore::fmv_x_d,
            RVCore::fmv_d_x,
        ] {
            assert!(
                matches!(
                    op(&mut core, F2, F0, RVReg::zero, RNE),
                    Err(XError::InvalidInst)
                ),
                "RV64D instruction must reject on RV32"
            );
        }
    }
}
