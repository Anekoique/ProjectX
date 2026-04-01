# F/D Floating-Point Extension PLAN 03

> Status: Revised
> Feature: `float`
> Iteration: `03`
> Owner: Executor
> Depends on:
> - Previous Plan: `02_PLAN.md`
> - Review: `02_REVIEW.md`
> - Master Directive: `02_MASTER.md`

---

## Summary

Fourth iteration. Resolves the final blocking finding: FP CSR writes via `csrrw`/`csrrs`/`csrrc` now transition `mstatus.FS` to Dirty (R-001). Cleans up the `DecodedInst` design to a single authoritative model (R-002), preserves `fflags`/`frm` debug name visibility (R-003), adds end-to-end CSR instruction tests (R-004). Per M-001, extends macro abstraction to cover FP-specific decoded formats (`FR`/`FR4`) that carry `rm`, avoiding polluting existing integer R-type handlers. Per M-003, verified all behavior against official RISC-V ISA manual.

## Log

[**Feature Introduce**]

- New `DecodedInst::FR` and `FR4` variants for FP R-type with `rm` — zero churn on existing 50+ integer R-type handlers
- `fp_csr_write` now calls `dirty_fp()` — CSR instruction path transitions FS
- `fflags`/`frm` preserved in `CsrAddr` enum with `from_name()` for debug tooling
- Unified FP macro system: `fp_binop!` generates both F32 and F64 variants in one invocation

[**Review Adjustments**]

- R-001 (HIGH): `fp_csr_write` calls `dirty_fp()` before return
- R-002 (MEDIUM): Single authoritative `DecodedInst` — only `FR`/`FR4` design, no superseded alternatives
- R-003 (MEDIUM): `fflags`/`frm` kept in `CsrAddr` enum for name resolution; storage handled by specialized path
- R-004 (MEDIUM): End-to-end `csrrw/csrrs/csrrc` tests for 0x001/0x002/0x003 including FS dirtiness

[**Master Compliance**]

- M-001: `FR`/`FR4` decoded formats avoid adding `_rm` to 50+ existing integer handlers. FP macros generate both S and D variants from one invocation (`fp_binop!`, `fp_cmp!`, `fp_fma!`).
- M-002: R-001 fixed — `fp_csr_write` transitions FS to Dirty
- M-003: Verified against RISC-V ISA manual: FS transitions on FP CSR writes confirmed in privileged spec §3.1.6.5; FSW does NOT dirty FS (read-only); FMIN/FMAX qNaN behavior per F spec §12.6

### Changes from Previous Round

[**Added**]
- `DecodedInst::FR` and `FR4` — FP-specific formats carrying `rm: u8`
- `dirty_fp()` call in `fp_csr_write`
- `CsrAddr::fflags`/`frm` enum variants for debug name resolution
- `fp_binop!` macro generating both `_s` and `_d` variants
- End-to-end CSR instruction tests for FP CSRs

[**Changed**]
- `DecodedInst` section collapsed to single authoritative design (removed superseded sketches)
- Existing integer R-type handlers unchanged (no `_rm` parameter)

[**Removed**]
- `rm` field from `DecodedInst::R` — integer handlers untouched
- All superseded `DecodedInst` design alternatives from plan body

[**Unresolved**]
- Difftest FP state sync (deferred)
- Linux boot with buildroot (deferred per 01_MASTER M-001)

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | `fp_csr_write` calls `dirty_fp()` |
| Review | R-002 | Accepted | Single `DecodedInst` with `FR`/`FR4`; superseded alternatives removed |
| Review | R-003 | Accepted | `CsrAddr::fflags`/`frm` kept for debug; storage via specialized path |
| Review | R-004 | Accepted | End-to-end `csrrw`/`csrrs`/`csrrc` tests added for 0x001/0x002/0x003 |
| Review | TR-1 | Accepted | Explicit `rm` in decoded FP forms; clean authoritative design |
| Review | TR-2 | Accepted | Single `fcsr` storage + debug name hooks for `fflags`/`frm` |
| Master | M-001 | Applied | `FR`/`FR4` formats + `fp_binop!` dual-precision macro; no integer handler churn |
| Master | M-002 | Applied | R-001 fixed |
| Master | M-003 | Applied | Verified against official RISC-V spec |

---

## Spec

[**Goals**]
- G-1: Implement all RV64F instructions (26 ops)
- G-2: Implement all RV64D instructions (26 ops)
- G-3: Implement compressed D load/store (C.FLD, C.FSD, C.FLDSP, C.FSDSP)
- G-4: Strict IEEE 754-2008 compliance via `softfloat-wrapper` with `riscv` feature
- G-5: NaN-boxing with canonical NaN on invalid boxing reads
- G-6: `mstatus.FS` state machine including CSR instruction paths
- G-7: `misa` bits F(5) and D(3) advertised

- NG-1: No Zfh/Q extensions
- NG-2: No FP exception trapping
- NG-3: No compressed single-precision (RV32 only)
- NG-4: No difftest FP state comparison (deferred)
- NG-5: No Linux boot validation this round (deferred)

[**Architecture**]

```
                    RVCore
                    +------------------------------+
                    | gpr: [Word; 32]              |
                    | fpr: [u64; 32]               |  NaN-boxed 64-bit
                    | csr: CsrFile                 |  fcsr at slot 0x003
                    +------------------------------+
                           |
              +------------+-----------+
              |            |           |
         inst/float.rs  csr/ops.rs  inst/compressed.rs
         fp_binop!      fp_csr_read   c_fld, c_fsd
         fp_cmp!        fp_csr_write  c_fldsp, c_fsdsp
         fp_fma!        + dirty_fp()
              |
         softfloat-wrapper (riscv feature)
```

**Decoded instruction flow**:
```
  riscv.instpat          decoder.rs             dispatch          float.rs
  ─────────────          ──────────             ────────          ────────
  "0000000 ..." R  ──→  DecodedInst::R   ──→  add(rd,rs1,rs2)
  "0000001 ..." R  ──→  DecodedInst::R   ──→  mul(rd,rs1,rs2)
  "0000000 ..." FR ──→  DecodedInst::FR  ──→  fadd_s(rd,rs1,rs2,rm)
  "????? 00 ..." FR4──→  DecodedInst::FR4 ──→  fmadd_s(rd,rs1,rs2,rs3,rm)
```

Integer R-type → `DecodedInst::R` (no `rm`) → existing handlers unchanged.
FP R-type → `DecodedInst::FR` (with `rm`) → FP handlers receive `rm`.
FP R4-type → `DecodedInst::FR4` (with `rs3` + `rm`) → FMA handlers.

**FP CSR data flow** (with FS dirtiness — resolves R-001):
```
  CSR instruction (csrrw x1, fflags, x2)
       │
       └──→ csr_write(0x001, val)
              │
              ├── is_fp_csr(0x001) → true
              ├── check_fp_csr_access()? → require_fp()
              ├── read old value via fp_csr_read(0x001)  [for csrrw swap]
              ├── fp_csr_write(0x001, new_val) → merge into fcsr slot
              ├── dirty_fp()  ← NEW: transitions FS to Dirty
              └── return Ok(old_value)
```

[**Invariants**]
- I-1: `fpr[i]` stores 64-bit raw bits. Single values NaN-boxed: `bits[63:32] = 0xFFFF_FFFF`.
- I-2: All narrow non-transfer F reads go through `read_f32()` → canonical NaN if not NaN-boxed.
- I-3: All FP instructions trap when `mstatus.FS == Off`.
- I-4: `with_flags()` always calls `dirty_fp()`. Non-flag FP writes call `dirty_fp()` explicitly. FP CSR writes call `dirty_fp()` on the CSR instruction path.
- I-5: `fflags` are sticky (OR-accumulated).
- I-6: `fcsr` (0x003) is single canonical storage. `fflags`/`frm` are computed views.
- I-7: Reserved rounding modes (5, 6) in `frm` + `rm=7` → illegal-instruction.
- I-8: `mstatus.SD` = `(FS == 0b11)`. Maintained by `dirty_fp()`.
- I-9: FCVT invalid-input per spec table.
- I-10: FMIN/FMAX: qNaN-only → no NV; sNaN-containing → NV.

[**Data Structure**]

```rust
/// Decoded instruction — single authoritative design.
/// R: integer R-type. FR: FP R-type (carries rm). FR4: FP R4-type (carries rs3 + rm).
pub enum DecodedInst {
    R  { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg },
    FR { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg, rm: u8 },
    FR4 { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg, rs3: RVReg, rm: u8 },
    I  { kind: InstKind, rd: RVReg, rs1: RVReg, imm: SWord },
    S  { kind: InstKind, rs1: RVReg, rs2: RVReg, imm: SWord },
    B  { kind: InstKind, rs1: RVReg, rs2: RVReg, imm: SWord },
    U  { kind: InstKind, rd: RVReg, imm: SWord },
    J  { kind: InstKind, rd: RVReg, imm: SWord },
    C  { kind: InstKind, inst: u32 },
}
```

**Rationale**: FP R-type needs `rm` but integer R-type does not. Separate `FR`/`FR4` variants avoid adding an unused `_rm: u8` to 50+ existing integer handlers (add, sub, mul, div, atomic, privileged). Zero churn on existing code.

`from_raw` extraction:
```rust
InstFormat::FR => Ok(Self::FR {
    kind,
    rd: reg(7)?,
    rs1: reg(15)?,
    rs2: reg(20)?,
    rm: ((inst >> 12) & 0x7) as u8,
}),
InstFormat::FR4 => Ok(Self::FR4 {
    kind,
    rd: reg(7)?,
    rs1: reg(15)?,
    rs2: reg(20)?,
    rs3: RVReg::try_from(((inst >> 27) & 0x1F) as u8).map_err(|_| XError::InvalidReg)?,
    rm: ((inst >> 12) & 0x7) as u8,
}),
```

`rv_inst_table!`:
```rust
macro_rules! rv_inst_table {
    ($macro:ident) => {
        $macro! {
            (R, (rd, rs1, rs2), [add, sub, sll, ..., amomaxu_d]),  // unchanged
            (FR, (rd, rs1, rs2, rm), [fadd_s, fsub_s, fmul_s, fdiv_s, fsqrt_s,
                fsgnj_s, fsgnjn_s, fsgnjx_s, fmin_s, fmax_s,
                fcvt_w_s, fcvt_wu_s, fmv_x_w, feq_s, flt_s, fle_s, fclass_s,
                fcvt_s_w, fcvt_s_wu, fmv_w_x, fcvt_l_s, fcvt_lu_s, fcvt_s_l, fcvt_s_lu,
                fadd_d, fsub_d, fmul_d, fdiv_d, fsqrt_d,
                fsgnj_d, fsgnjn_d, fsgnjx_d, fmin_d, fmax_d,
                fcvt_s_d, fcvt_d_s, feq_d, flt_d, fle_d, fclass_d,
                fcvt_w_d, fcvt_wu_d, fcvt_d_w, fcvt_d_wu,
                fcvt_l_d, fcvt_lu_d, fmv_x_d, fcvt_d_l, fcvt_d_lu, fmv_d_x]),
            (FR4, (rd, rs1, rs2, rs3, rm), [fmadd_s, fmsub_s, fnmsub_s, fnmadd_s,
                fmadd_d, fmsub_d, fnmsub_d, fnmadd_d]),
            (I, (rd, rs1, imm), [addi, ..., flw, fld]),  // FP loads as I-type
            (S, (rs1, rs2, imm), [sb, sh, sw, sd, fsw, fsd]),  // FP stores as S-type
            // B, U, J, C unchanged
        }
    };
}
```

[**API Surface**]

```rust
// === inst/float.rs — core helpers ===

impl RVCore {
    /// Trap if mstatus.FS == Off.
    fn require_fp(&self) -> XResult {
        ((self.csr.get(CsrAddr::mstatus) >> 13) & 0x3 != 0)
            .ok_or(XError::InvalidInst)
    }

    /// Set mstatus.FS = Dirty (0b11) and SD = 1.
    fn dirty_fp(&mut self) {
        let ms = self.csr.get(CsrAddr::mstatus);
        self.csr.set(CsrAddr::mstatus, ms | MStatus::FS.bits() | MStatus::SD.bits());
    }

    /// Read single-precision with NaN-boxing check.
    fn read_f32(&self, reg: RVReg) -> F32 {
        let bits = self.fpr[reg];
        F32::from_bits(if bits >> 32 == 0xFFFF_FFFF { bits as u32 } else { 0x7FC0_0000 })
    }

    fn write_f32(&mut self, reg: RVReg, val: F32) {
        self.fpr[reg] = 0xFFFF_FFFF_0000_0000 | val.to_bits() as u64;
    }

    fn read_f64(&self, reg: RVReg) -> F64 { F64::from_bits(self.fpr[reg]) }
    fn write_f64(&mut self, reg: RVReg, val: F64) { self.fpr[reg] = val.to_bits(); }

    fn resolve_rm(&self, rm: u8) -> XResult<SfRm> {
        let eff = if rm == 7 { ((self.csr.get(CsrAddr::fcsr) >> 5) & 0x7) as u8 } else { rm };
        sf_rounding_mode(eff).ok_or(XError::InvalidInst)
    }

    /// Execute FP op, accumulate flags, mark FS dirty. Centralizes I-4.
    fn with_flags<T>(&mut self, rm: SfRm, op: impl FnOnce(SfRm) -> T) -> T {
        let mut flags = ExceptionFlags::default();
        flags.set();
        let result = op(rm);
        flags.get();
        let bits = flags.to_bits() as Word;
        if bits != 0 {
            let fcsr = self.csr.get(CsrAddr::fcsr);
            self.csr.set(CsrAddr::fcsr, fcsr | (bits & 0x1F));
        }
        self.dirty_fp();
        result
    }
}
```

```rust
// === csr/ops.rs — FP CSR specialized path (resolves R-001) ===

impl RVCore {
    fn is_fp_csr(addr: u16) -> bool {
        matches!(addr, 0x001 | 0x002 | 0x003)
    }

    fn fp_csr_read(&self, addr: u16) -> Word {
        let fcsr = self.csr.get(CsrAddr::fcsr);
        match addr {
            0x001 => fcsr & 0x1F,
            0x002 => (fcsr >> 5) & 0x7,
            _     => fcsr & 0xFF,
        }
    }

    fn fp_csr_write(&mut self, addr: u16, val: Word) {
        let fcsr = self.csr.get(CsrAddr::fcsr);
        let new = match addr {
            0x001 => (fcsr & !0x1F) | (val & 0x1F),
            0x002 => (fcsr & !0xE0) | ((val & 0x7) << 5),
            _     => val & 0xFF,
        };
        self.csr.set(CsrAddr::fcsr, new);
        self.dirty_fp();  // R-001 fix: CSR writes transition FS to Dirty
    }

    /// Integrated into csr_read:
    pub(in crate::cpu::riscv) fn csr_read(&self, addr: u16) -> XResult<Word> {
        if Self::is_fp_csr(addr) {
            self.require_fp()?;
            return Ok(self.fp_csr_read(addr));
        }
        // ... existing generic path unchanged
    }

    /// Integrated into csr_write — handles CSR read-modify-write semantics:
    pub(in crate::cpu::riscv) fn csr_write(&mut self, addr: u16, val: Word) -> XResult {
        if Self::is_fp_csr(addr) {
            self.require_fp()?;
            self.fp_csr_write(addr, val);
            return Ok(());
        }
        // ... existing generic path unchanged
    }
}
```

```rust
// === csr.rs — fcsr in csr_table + debug name entries (resolves R-003) ===

csr_table! {
    // ... existing M/S/PMP/counter entries ...

    // FP CSR — fcsr is canonical storage; fflags/frm kept for name resolution only
    fflags = 0x001 => [RW(0x1F)],   // debug name; actual r/w via fp_csr_read/write
    frm    = 0x002 => [RW(0x07)],   // debug name; actual r/w via fp_csr_read/write
    fcsr   = 0x003 => [RW(0xFF)],   // canonical storage slot
}
```

Note: `fflags` and `frm` appear in `csr_table!` so that `CsrAddr::from_name("fflags")` and `find_desc(0x001)` succeed for debug tooling. But `csr_read`/`csr_write` intercept these addresses before the generic descriptor path, so the descriptor's `wmask`/`storage` are never used for actual reads/writes. This preserves named access (R-003) without conflicting with the specialized storage model (R-001).

[**Constraints**]
- C-1: FP and GPR share 5-bit encoding.
- C-2: `DecodedInst::FR4` for FMA (8 instructions).
- C-3: `funct3` = `rm` in `FR`; = sub-op in separate `InstKind` values for FSGNJ/compare.
- C-4: `softfloat-wrapper = { version = "0.3", default-features = false, features = ["riscv"] }`.
- C-5: Compressed D in scope. Compressed S deferred (RV32).
- C-6: `fcsr` canonical storage. `fflags`/`frm` views via `fp_csr_read`/`fp_csr_write`. Entries in `csr_table!` for name resolution only.
- C-7: FCVT invalid-input table (unchanged from 01_PLAN).
- C-8: `mstatus.SD` maintained by `dirty_fp()`.

---

## Implement

### Execution Flow

[**Main Flow**] unchanged from 02_PLAN except:
- FP CSR write path now includes `dirty_fp()` call
- FP instructions decoded as `FR`/`FR4` (not `R`)
- Integer instructions decoded as `R` (unchanged)

[**Failure Flow**] unchanged.
[**State Transition**] unchanged, plus:
- `CSR write to 0x001/0x002/0x003` → `fp_csr_write` → `dirty_fp()` → FS=Dirty

### Implementation Plan

[**Phase 1: Infrastructure**]

1. `fpr: [u64; 32]` in `RVCore`, zero-init, cleared on reset
2. `DecodedInst::FR` (rd, rs1, rs2, rm) and `FR4` (rd, rs1, rs2, rs3, rm) added
3. `InstFormat::FR` and `FR4` in enum; decoder extracts `rm` and `rs3`
4. `rv_inst_table!` gains `FR` and `FR4` rows; `R` row unchanged
5. `build_dispatch` macro handles `FR`/`FR4` variants automatically
6. `fcsr`/`fflags`/`frm` in `csr_table!`; `fp_csr_read`/`fp_csr_write` with `dirty_fp()` in ops.rs
7. `misa` gains F(5) + D(3); mstatus FS initialized to Initial (0b01 << 13)
8. `softfloat-wrapper` dependency

[**Phase 2: F Extension (26 ops)**]

Macro-generated dual-precision handlers (per M-001):

```rust
/// Generate arithmetic handlers for both single and double precision.
macro_rules! fp_binop {
    ($name_s:ident, $name_d:ident, $op:ident) => {
        pub(super) fn $name_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rm: u8) -> XResult {
            self.require_fp()?;
            let rm = self.resolve_rm(rm)?;
            let r = self.with_flags(rm, |rm| self.read_f32(rs1).$op(self.read_f32(rs2), rm));
            self.write_f32(rd, r);
            Ok(())
        }
        pub(super) fn $name_d(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rm: u8) -> XResult {
            self.require_fp()?;
            let rm = self.resolve_rm(rm)?;
            let r = self.with_flags(rm, |rm| self.read_f64(rs1).$op(self.read_f64(rs2), rm));
            self.write_f64(rd, r);
            Ok(())
        }
    };
}

fp_binop!(fadd_s, fadd_d, add);
fp_binop!(fsub_s, fsub_d, sub);
fp_binop!(fmul_s, fmul_d, mul);
fp_binop!(fdiv_s, fdiv_d, div);

/// Comparison handlers — both precisions.
macro_rules! fp_cmp {
    ($name_s:ident, $name_d:ident, $op:ident) => {
        pub(super) fn $name_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _rm: u8) -> XResult {
            self.require_fp()?;
            let r = self.with_flags(SfRm::TiesToEven, |_| self.read_f32(rs1).$op(self.read_f32(rs2)));
            self.set_gpr(rd, r as Word)
        }
        pub(super) fn $name_d(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _rm: u8) -> XResult {
            self.require_fp()?;
            let r = self.with_flags(SfRm::TiesToEven, |_| self.read_f64(rs1).$op(self.read_f64(rs2)));
            self.set_gpr(rd, r as Word)
        }
    };
}

fp_cmp!(feq_s, feq_d, eq);
fp_cmp!(flt_s, flt_d, lt);
fp_cmp!(fle_s, fle_d, le);

/// FMA handlers — both precisions, with sign-flip flags.
macro_rules! fp_fma {
    ($name_s:ident, $name_d:ident, $neg_a:expr, $neg_c:expr) => {
        pub(super) fn $name_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rs3: RVReg, rm: u8) -> XResult {
            self.require_fp()?;
            let rm = self.resolve_rm(rm)?;
            let (a, b, c) = (self.read_f32(rs1), self.read_f32(rs2), self.read_f32(rs3));
            let a = if $neg_a { F32::from_bits(a.to_bits() ^ 0x8000_0000) } else { a };
            let c = if $neg_c { F32::from_bits(c.to_bits() ^ 0x8000_0000) } else { c };
            let r = self.with_flags(rm, |rm| a.fused_mul_add(b, c, rm));
            self.write_f32(rd, r);
            Ok(())
        }
        pub(super) fn $name_d(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rs3: RVReg, rm: u8) -> XResult {
            self.require_fp()?;
            let rm = self.resolve_rm(rm)?;
            let (a, b, c) = (self.read_f64(rs1), self.read_f64(rs2), self.read_f64(rs3));
            let a = if $neg_a { F64::from_bits(a.to_bits() ^ (1u64 << 63)) } else { a };
            let c = if $neg_c { F64::from_bits(c.to_bits() ^ (1u64 << 63)) } else { c };
            let r = self.with_flags(rm, |rm| a.fused_mul_add(b, c, rm));
            self.write_f64(rd, r);
            Ok(())
        }
    };
}

fp_fma!(fmadd_s,  fmadd_d,  false, false);
fp_fma!(fmsub_s,  fmsub_d,  false, true);
fp_fma!(fnmsub_s, fnmsub_d, true,  false);
fp_fma!(fnmadd_s, fnmadd_d, true,  true);
```

Unique handlers (sign-inject, classify, move, load/store, convert) — same as 02_PLAN, with the `read_f32()` NaN-boxing fix for sign-injection confirmed.

[**Phase 3: D Extension (26 ops) + Compressed D (4 ops)**]

D handlers generated by the same macros above (the `_d` variants). Unique D handlers mirror F handlers using `read_f64`/`write_f64`.

Compressed D patterns and handlers unchanged from 01_PLAN.

[**Phase 4: Integration**]

1. DTS: `riscv,isa = "rv64imafdcsu_sstc"`
2. `mstatus` side-effects: recompute SD on mstatus/sstatus write
3. `CoreContext`: add `fprs` field
4. Debug ISA string: `rv64imafdc`
5. `format_mnemonic`: FP disassembly
6. Validation

---

## Trade-offs

- T-1: **Softfloat backend** — `softfloat-wrapper` with `riscv` feature. Confirmed.
- T-2: **FP register storage** — `[u64; 32]` raw bits. Confirmed.
- T-3: **Decoded FP format** — `FR`/`FR4` variants instead of adding `rm` to existing `R`. This avoids modifying 50+ integer R-type handler signatures. `InstFormat::FR`/`FR4` added to decoder. Macro dispatch handles them naturally. Zero churn on existing code.

---

## Validation

[**Unit Tests**]
- V-UT-1: NaN-boxing roundtrip; invalid boxing → canonical NaN
- V-UT-2: Rounding mode resolution
- V-UT-3: `fflags` sticky accumulation
- V-UT-4: FP CSR composite via helpers: `fp_csr_write(0x003, 0xE5)` → `fp_csr_read(0x001)==0x05`, `fp_csr_read(0x002)==0x07`
- V-UT-5: FS gating: FS=Off → IllegalInstruction; FS=Initial → execute + FS=Dirty
- V-UT-6: `mstatus.SD` = 1 when FS=Dirty
- V-UT-7: Decoder: all 56 FP patterns, FR/FR4 rm/rs3 extraction
- V-UT-8: `fclass` all 10 categories
- V-UT-9: `with_flags` marks FS dirty even for flag-only instructions

[**Integration Tests**] — (V-IT-1 through V-IT-7 unchanged from 02_PLAN)

[**End-to-End CSR Instruction Tests**] — (resolves R-004)
- V-CSR-1: `csrrw x1, fflags, x2` — reads old fflags, writes new; verify fcsr composite updated
- V-CSR-2: `csrrs x1, frm, x2` — sets bits in frm; verify fcsr composite
- V-CSR-3: `csrrc x1, fcsr, x2` — clears bits in fcsr; verify fflags/frm views
- V-CSR-4: `csrrw x0, fflags, x2` — no-read variant (rd=x0); still writes and dirties FS
- V-CSR-5: `csrrs x1, fflags, x0` — no-write variant (rs1=x0); does NOT dirty FS (read-only access)
- V-CSR-6: FS=Initial + `csrrw fflags` → FS=Dirty + SD=1

[**Failure / Robustness Validation**] — (V-F-1 through V-F-5 unchanged from 02_PLAN)

[**Edge Case Validation (NaN-sensitive)**] — (V-E-1 through V-E-12 unchanged from 02_PLAN)

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (F ext) | V-IT-1, V-IT-3, V-IT-4, V-IT-5, V-UT-7 |
| G-2 (D ext) | V-IT-2, V-IT-3, V-IT-4, V-IT-5, V-UT-7 |
| G-3 (Compressed D) | V-IT-6, V-UT-7 |
| G-4 (IEEE 754) | V-UT-1..3, V-F-2..5, V-E-1..7, V-E-11..12 |
| G-5 (NaN-boxing) | V-UT-1, V-E-8, V-E-10 |
| G-6 (mstatus.FS) | V-UT-5, V-UT-6, V-UT-9, V-F-1, V-CSR-4, V-CSR-5, V-CSR-6 |
| G-7 (misa) | V-IT-7 |
| C-2 (R4-type) | V-UT-7, V-IT-3 |
| C-6 (FP CSR) | V-UT-4, V-CSR-1..6 |
| C-7 (FCVT table) | V-F-5 |
| I-4 (FS dirty) | V-UT-9, V-CSR-4, V-CSR-6 |
| I-10 (FMIN NaN) | V-E-2..5 |
