# F/D Floating-Point Extension PLAN 02

> Status: Revised
> Feature: `float`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `01_MASTER.md`

---

## Summary

Third iteration. Resolves all blocking findings: redesigns FP CSR composition as first-class specialized read/write (R-001), centralizes FS dirtiness in `with_flags` (R-002), routes all narrow non-transfer operands through NaN-boxing check (R-003), fixes FMIN/FMAX qNaN flag expectations (R-005), and carries `rm` as an explicit decoded field instead of hidden state (TR-1). Per M-001, Linux boot validation deferred to a follow-up round. Per M-003, reduces handler redundancy via macro-generated dispatch and shared helper closures.

## Log

[**Feature Introduce**]

- FP CSR redesigned: `fcsr` as single canonical slot with specialized `fp_csr_read`/`fp_csr_write` methods bypassing generic alias path
- `with_flags` now always calls `dirty_fp` — single responsibility for FS transition
- `rm` carried explicitly in `DecodedInst::R` and `R4` via embedded field, no hidden `last_inst_raw`
- Macro-generated handler families (`fp_arith!`, `fp_cmp!`, `fp_fma!`) eliminate handler repetition

[**Review Adjustments**]

- R-001 (HIGH): FP CSRs use dedicated read/write path outside generic alias model
- R-002 (HIGH): `with_flags` owns `dirty_fp` call — every flag-producing instruction transitions FS
- R-003 (HIGH): Sign-injection reads operands via `read_f32` (NaN-boxing check), then manipulates sign bits on the canonicalized payload
- R-005 (MEDIUM): FMIN/FMAX NaN tests split into 4 cases; only sNaN inputs expect NV

[**Master Compliance**]

- M-001: Linux boot validation deferred; acceptance scope narrowed to instruction-level correctness
- M-002: R-001/R-002/R-003 fixed as directed
- M-003: Handler macros reduce ~50 near-identical handlers to ~6 macro invocations + unique handlers

### Changes from Previous Round

[**Added**]
- `fp_csr_read`/`fp_csr_write` specialized methods in `csr/ops.rs`
- `with_flags` now calls `dirty_fp` unconditionally
- `rm` field embedded in decoded instructions
- Handler macros: `fp_arith!`, `fp_cmp!`, `fp_fma!`, `fp_cvt_f2i!`, `fp_cvt_i2f!`

[**Changed**]
- CSR composition: from alias+side-effect mix to dedicated first-class path
- Sign-injection: from raw `fpr[]` access to `read_f32()` -> sign-bit manipulation
- `with_flags`: now always marks FS dirty (was caller responsibility)
- FMIN/FMAX validation: qNaN-only cases no longer expect NV

[**Removed**]
- `last_inst_raw` field and `funct3()` method
- Linux boot acceptance item (deferred per M-001)
- Redundant per-handler code (replaced by macros)

[**Unresolved**]
- Difftest FP state sync (deferred)
- Linux boot with buildroot (deferred per M-001)

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | FP CSRs use dedicated `fp_csr_read`/`fp_csr_write` outside generic alias path |
| Review | R-002 | Accepted | `with_flags` always calls `dirty_fp` — centralized FS transition |
| Review | R-003 | Accepted | All narrow non-transfer ops read via `read_f32` with NaN-boxing check |
| Review | R-004 | Deferred | Linux boot validation deferred per M-001 |
| Review | R-005 | Accepted | FMIN/FMAX tests split: qNaN-only → no NV; sNaN-containing → NV |
| Review | TR-1 | Accepted | `rm` carried as explicit decoded field; no hidden state |
| Review | TR-2 | Accepted | FP CSR state model redesigned independently of softfloat choice |
| Master | M-001 | Applied | Linux boot deferred to follow-up |
| Master | M-002 | Applied | R-001/R-002/R-003 all fixed |
| Master | M-003 | Applied | Macro-generated handlers; shared helpers; minimal repetition |

---

## Spec

[**Goals**]
- G-1: Implement all RV64F instructions (26 ops)
- G-2: Implement all RV64D instructions (26 ops)
- G-3: Implement compressed D load/store (C.FLD, C.FSD, C.FLDSP, C.FSDSP — 4 ops)
- G-4: Strict IEEE 754-2008 compliance via `softfloat-wrapper` with `riscv` feature
- G-5: NaN-boxing: single-precision values NaN-boxed in 64-bit f-registers
- G-6: `mstatus.FS` state machine with illegal-instruction trap when FS=Off
- G-7: `misa` bits F(5) and D(3) advertised

- NG-1: No Zfh/Q extensions
- NG-2: No FP exception trapping
- NG-3: No compressed single-precision (RV32 only)
- NG-4: No difftest FP state comparison (deferred)
- NG-5: No Linux boot validation this round (deferred per M-001)

[**Architecture**]

```
                    RVCore
                    +------------------------------+
                    | gpr: [Word; 32]              |
                    | fpr: [u64; 32]               |  NaN-boxed 64-bit storage
                    | csr: CsrFile                 |  fcsr at slot 0x003
                    +------------------------------+
                           |
              +------------+-----------+
              |            |           |
         inst/float.rs  csr/ops.rs  inst/compressed.rs
         ├ fp_arith!    fp_csr_read   c_fld, c_fsd
         ├ fp_cmp!      fp_csr_write  c_fldsp, c_fsdsp
         ├ fp_fma!      dirty_fp
         ├ fp_cvt!      require_fp
         └ fsgnj, fclass, fmv, flw/fsw, fld/fsd
              |
         softfloat-wrapper (riscv feature)
```

**FP CSR data flow** (resolves R-001):
```
  fcsr (0x003) — canonical storage slot, 8 bits [frm:fflags]
       │
       ├── read 0x001 (fflags) → fp_csr_read → fcsr & 0x1F
       ├── read 0x002 (frm)    → fp_csr_read → (fcsr >> 5) & 0x7
       ├── read 0x003 (fcsr)   → fp_csr_read → fcsr & 0xFF
       │
       ├── write 0x001 (fflags) → fp_csr_write → fcsr = (fcsr & !0x1F) | (val & 0x1F)
       ├── write 0x002 (frm)    → fp_csr_write → fcsr = (fcsr & !0xE0) | ((val & 0x7) << 5)
       └── write 0x003 (fcsr)   → fp_csr_write → fcsr = val & 0xFF
```

No alias descriptors. No shadow registers. Single storage slot. Specialized read/write methods handle the bit-field extraction/insertion.

[**Invariants**]
- I-1: `fpr[i]` stores 64-bit raw bits. Single-precision values NaN-boxed: `bits[63:32] = 0xFFFF_FFFF`.
- I-2: All narrow non-transfer F-extension reads go through `read_f32()`, which returns canonical single NaN if not properly NaN-boxed. Only `FLW`/`FSW`/`FMV.W.X`/`FMV.X.W` bypass this (transfer instructions).
- I-3: All FP instructions raise `IllegalInstruction` when `mstatus.FS == Off`.
- I-4: `with_flags()` always calls `dirty_fp()` — every flag-producing instruction transitions FS to Dirty. Non-flag instructions (sign-inject, move, classify, load/store) call `dirty_fp()` explicitly when they write FP state.
- I-5: `fflags` are sticky (OR-accumulated).
- I-6: `fcsr` is the single canonical storage slot. `fflags` and `frm` are computed views, not separate storage.
- I-7: Reserved rounding modes (5, 6) in `frm` + `rm=7` → illegal-instruction.
- I-8: `mstatus.SD` = `(FS == 0b11)`. Updated in `dirty_fp()` and CSR side-effects.
- I-9: FCVT invalid-input results follow the spec table (see C-7).
- I-10: FMIN/FMAX: qNaN-only inputs do NOT set NV; sNaN inputs set NV.

[**Data Structure**]

```rust
// RVCore gains fpr field
pub struct RVCore {
    gpr: [Word; 32],
    fpr: [u64; 32],  // f0-f31, 64-bit NaN-boxed
    // ... rest unchanged
}
```

`DecodedInst` carries `rm` explicitly (resolves TR-1):
```rust
pub enum DecodedInst {
    R  { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg },
    R4 { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg, rs3: RVReg },
    I  { kind: InstKind, rd: RVReg, rs1: RVReg, imm: SWord },
    S  { kind: InstKind, rs1: RVReg, rs2: RVReg, imm: SWord },
    B  { kind: InstKind, rs1: RVReg, rs2: RVReg, imm: SWord },
    U  { kind: InstKind, rd: RVReg, imm: SWord },
    J  { kind: InstKind, rd: RVReg, imm: SWord },
    C  { kind: InstKind, inst: u32 },
}
```

Note: `rm` is NOT added as a separate field. Instead, FP R-type instructions that need `rm` extract it from `funct3`, which is already available in the matching process. The key insight: FP instructions use `funct7` to distinguish operations (not `funct3`), so FP instructions that use `rm` all share the same decoder bucket. The `rm` value equals `funct3` bits, which the handler extracts from `self.last_inst_raw`.

**Revised approach** per TR-1: Rather than `last_inst_raw`, we store the raw instruction in the `DecodedInst` itself. Since R-type FP instructions need both `funct7` (for operation selection) and `funct3` (for rm), and R4-type also needs `rs3` from bits[31:27], the cleanest approach is:

```rust
pub enum DecodedInst {
    R  { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg },
    R4 { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg, rs3: RVReg, rm: u8 },
    I  { kind: InstKind, rd: RVReg, rs1: RVReg, imm: SWord },
    // ... rest unchanged
}
```

Only R4 needs explicit `rm` because:
- R-type FP instructions that use `rm` as rounding mode have `funct3` as don't-care in the pattern, so they are broadcast to all 8 funct3 buckets. But R-type FP instructions where `funct3` selects the sub-operation (FSGNJ, FMIN/FMAX, FEQ/FLT/FLE) have fixed `funct3` and are decoded as **separate InstKind values** — no ambiguity.
- R-type FP arithmetic (FADD, FSUB, FMUL, FDIV, FSQRT, FCVT) need `rm`, but they are decoded via the R format which doesn't carry `funct3`. Solution: **embed `rm` in `DecodedInst::R` for FP instructions**.

Simplest clean solution — add `rm` to R-type:

```rust
pub enum DecodedInst {
    R  { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg, rm: u8 },
    R4 { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg, rs3: RVReg, rm: u8 },
    // ... I, S, B, U, J, C unchanged
}
```

For non-FP R-type instructions (add, sub, etc.), `rm` is simply 0 and ignored. The field costs nothing at runtime (1 byte) and eliminates all hidden state.

`from_raw` extraction:
```rust
InstFormat::R => Ok(Self::R {
    kind,
    rd: reg(7)?,
    rs1: reg(15)?,
    rs2: reg(20)?,
    rm: ((inst >> 12) & 0x7) as u8,
}),
InstFormat::R4 => Ok(Self::R4 {
    kind,
    rd: reg(7)?,
    rs1: reg(15)?,
    rs2: reg(20)?,
    rs3: RVReg::try_from(((inst >> 27) & 0x1F) as u8).map_err(|_| XError::InvalidReg)?,
    rm: ((inst >> 12) & 0x7) as u8,
}),
```

`rv_inst_table!` update:
```rust
(R, (rd, rs1, rs2, rm), [add, sub, ..., fadd_s, fsub_s, ..., fsgnj_s, ...]),
(R4, (rd, rs1, rs2, rs3, rm), [fmadd_s, fmsub_s, ...]),
```

Existing integer R-type handlers gain an `_rm: u8` parameter they ignore:
```rust
pub(super) fn add(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _rm: u8) -> XResult {
    self.binary_op(rd, rs1, rs2, |a, b| a.wrapping_add(b))
}
```

[**API Surface**]

```rust
// inst/float.rs — core helpers

impl RVCore {
    fn require_fp(&self) -> XResult {
        let fs = (self.csr.get(CsrAddr::mstatus) >> 13) & 0x3;
        (fs != 0).ok_or(XError::InvalidInst)
    }

    fn dirty_fp(&mut self) {
        let ms = self.csr.get(CsrAddr::mstatus);
        self.csr.set(CsrAddr::mstatus, ms | MStatus::FS.bits() | MStatus::SD.bits());
    }

    fn read_f32(&self, reg: RVReg) -> F32 {
        let bits = self.fpr[reg];
        let valid = (bits >> 32) == 0xFFFF_FFFF;
        F32::from_bits(if valid { bits as u32 } else { 0x7FC0_0000 })
    }

    fn write_f32(&mut self, reg: RVReg, val: F32) {
        self.fpr[reg] = 0xFFFF_FFFF_0000_0000 | val.to_bits() as u64;
    }

    fn read_f64(&self, reg: RVReg) -> F64 {
        F64::from_bits(self.fpr[reg])
    }

    fn write_f64(&mut self, reg: RVReg, val: F64) {
        self.fpr[reg] = val.to_bits();
    }

    fn resolve_rm(&self, rm: u8) -> XResult<SfRm> {
        let eff = if rm == 7 {
            ((self.csr.get(CsrAddr::fcsr) >> 5) & 0x7) as u8
        } else {
            rm
        };
        sf_rounding_mode(eff).ok_or(XError::InvalidInst)
    }

    /// Execute FP operation, accumulate flags, mark FS dirty.
    /// Centralizes I-4: every flag-producing path transitions FS.
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
        self.dirty_fp();  // Always — resolves R-002
        result
    }
}
```

```rust
// csr/ops.rs — FP CSR specialized path (resolves R-001)

impl RVCore {
    /// Check if addr is an FP CSR (0x001, 0x002, 0x003).
    fn is_fp_csr(addr: u16) -> bool {
        matches!(addr, 0x001 | 0x002 | 0x003)
    }

    /// Read FP CSR — all three are views into fcsr (slot 0x003).
    fn fp_csr_read(&self, addr: u16) -> Word {
        let fcsr = self.csr.get(CsrAddr::fcsr);
        match addr {
            0x001 => fcsr & 0x1F,          // fflags
            0x002 => (fcsr >> 5) & 0x7,    // frm
            0x003 => fcsr & 0xFF,           // fcsr
            _ => unreachable!(),
        }
    }

    /// Write FP CSR — merge into fcsr (slot 0x003).
    fn fp_csr_write(&mut self, addr: u16, val: Word) {
        let fcsr = self.csr.get(CsrAddr::fcsr);
        let new = match addr {
            0x001 => (fcsr & !0x1F) | (val & 0x1F),          // fflags
            0x002 => (fcsr & !0xE0) | ((val & 0x7) << 5),    // frm
            0x003 => val & 0xFF,                               // fcsr
            _ => unreachable!(),
        };
        self.csr.set(CsrAddr::fcsr, new);
    }
}
```

Integration into `csr_read`/`csr_write` in ops.rs:
```rust
pub(in crate::cpu::riscv) fn csr_read(&self, addr: u16) -> XResult<Word> {
    if Self::is_fp_csr(addr) {
        self.check_fp_csr_access()?;
        return Ok(self.fp_csr_read(addr));
    }
    // ... existing generic path
}

pub(in crate::cpu::riscv) fn csr_write(&mut self, addr: u16, val: Word) -> XResult {
    if Self::is_fp_csr(addr) {
        self.check_fp_csr_access()?;
        self.fp_csr_write(addr, val);
        return Ok(());
    }
    // ... existing generic path
}

fn check_fp_csr_access(&self) -> XResult {
    // FP CSRs are user-accessible (addr bits [9:8] = 0b00)
    // but require FS != Off
    self.require_fp()
}
```

`fcsr` still needs a `csr_table!` entry for `get(CsrAddr::fcsr)` / `set(CsrAddr::fcsr)` raw access:
```rust
csr_table! {
    // ... existing ...
    fcsr = 0x003 => [RW(0xFF)],
    // fflags and frm are NOT in csr_table — handled by fp_csr_read/write
}
```

[**Constraints**]
- C-1: FP and GPR share 5-bit encoding. Handlers index `fpr[]` with the same `RVReg`.
- C-2: R4-type (FMA) uses `DecodedInst::R4` with `rd, rs1, rs2, rs3, rm`.
- C-3: `funct3` = `rm` for arithmetic, = sub-op for FSGNJ/compare. Decoded as separate `InstKind`.
- C-4: `softfloat-wrapper = { version = "0.3", default-features = false, features = ["riscv"] }`.
- C-5: Compressed D (C.FLD/C.FSD/C.FLDSP/C.FSDSP) in scope. Compressed single-precision RV32-only, deferred.
- C-6: `fcsr` (0x003) is single canonical slot. `fflags`/`frm` handled by dedicated read/write. NOT in `csr_table!` as separate entries.
- C-7: FCVT invalid-input table (unchanged from 01_PLAN — spec-compliant).
- C-8: `mstatus.SD` maintained by `dirty_fp()` and mstatus side-effects.

---

## Implement

### Execution Flow

[**Main Flow**] (arithmetic: `fadd.s rd, rs1, rs2`)
1. Decoder → `DecodedInst::R { kind: fadd_s, rd, rs1, rs2, rm }`
2. Dispatch → `Self::fadd_s(self, rd, rs1, rs2, rm)`
3. `require_fp()?`
4. `let rm = resolve_rm(rm)?`
5. `let (a, b) = (read_f32(rs1), read_f32(rs2))` — NaN-boxing checked
6. `let result = with_flags(rm, |rm| a.add(b, rm))` — flags accumulated, FS marked dirty
7. `write_f32(rd, result)` — NaN-boxed

[**Main Flow**] (comparison: `feq.s rd, rs1, rs2`)
1. Decoder → `DecodedInst::R { kind: feq_s, rd, rs1, rs2, rm: 010 }`
2. Dispatch → `Self::feq_s(self, rd, rs1, rs2, _rm)`
3. `require_fp()?`
4. `let (a, b) = (read_f32(rs1), read_f32(rs2))` — NaN-boxing checked
5. `let eq = with_flags(SfRm::TiesToEven, |_| a.eq(b))` — flags + FS dirty
6. `set_gpr(rd, eq as Word)`

[**Main Flow**] (sign-inject: `fsgnj.s rd, rs1, rs2`)  — resolves R-003
1. `require_fp()?`
2. `let a = read_f32(rs1).to_bits()` — NaN-boxing checked, then raw bits
3. `let b = read_f32(rs2).to_bits()` — NaN-boxing checked
4. `let result = (a & 0x7FFF_FFFF) | (b & 0x8000_0000)` — sign injection on canonicalized bits
5. `write_f32(rd, F32::from_bits(result))` — NaN-boxed
6. `dirty_fp()` — writes FP state, no flags

[**Failure Flow**]
1. `mstatus.FS == Off` → `IllegalInstruction` trap
2. `rm=7` + reserved `frm` → `IllegalInstruction` trap
3. FP exception → `fflags` bits set; no trap
4. Invalid FCVT → saturated value per C-7 table + NV flag

[**State Transition**]
- FS=Off + FP instruction → IllegalInstruction (no state change)
- FS=Initial/Clean + `with_flags()` → FS=Dirty (via `dirty_fp`)
- FS=Initial/Clean + non-flag FP write (load/store/move/sgnj) → FS=Dirty (explicit `dirty_fp`)
- FS=Dirty → remains Dirty until OS resets via CSR write

### Implementation Plan

[**Phase 1: Infrastructure**]

1. `fpr: [u64; 32]` in `RVCore`, initialized to 0, cleared on reset
2. `DecodedInst::R` gains `rm: u8` field; `DecodedInst::R4` added with `rs3` + `rm`
3. `InstFormat::R4` added; decoder `from_raw` extracts `rs3` from bits[31:27], `rm` from bits[14:12]
4. `rv_inst_table!` gains R4 row; R row gains `rm` parameter
5. All existing R-type handlers gain `_rm: u8` parameter (ignored)
6. `fcsr = 0x003 => [RW(0xFF)]` in `csr_table!`; `fp_csr_read`/`fp_csr_write` for 0x001/0x002/0x003
7. `misa` bits F(5) + D(3); `mstatus` FS initialized to Initial (0b01 << 13)
8. `softfloat-wrapper` dependency

[**Phase 2: F Extension (26 ops)**]

Macro-generated handlers (resolves M-003):

```rust
/// Generate F-extension arithmetic handler: read_f32 → softfloat op → write_f32
macro_rules! fp_arith_s {
    ($name:ident, $op:ident) => {
        pub(super) fn $name(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rm: u8) -> XResult {
            self.require_fp()?;
            let rm = self.resolve_rm(rm)?;
            let (a, b) = (self.read_f32(rs1), self.read_f32(rs2));
            let r = self.with_flags(rm, |rm| a.$op(b, rm));
            self.write_f32(rd, r);
            Ok(())
        }
    };
}

fp_arith_s!(fadd_s, add);
fp_arith_s!(fsub_s, sub);
fp_arith_s!(fmul_s, mul);
fp_arith_s!(fdiv_s, div);

/// Generate comparison handler: read_f32 → compare → write GPR
macro_rules! fp_cmp_s {
    ($name:ident, $op:ident) => {
        pub(super) fn $name(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _rm: u8) -> XResult {
            self.require_fp()?;
            let (a, b) = (self.read_f32(rs1), self.read_f32(rs2));
            let r = self.with_flags(SfRm::TiesToEven, |_| a.$op(b));
            self.set_gpr(rd, r as Word)
        }
    };
}

fp_cmp_s!(feq_s, eq);
fp_cmp_s!(flt_s, lt);
fp_cmp_s!(fle_s, le);

/// Generate FMA handler: read_f32 × 3 → fused_mul_add → write_f32
macro_rules! fp_fma_s {
    ($name:ident, $neg_a:expr, $neg_c:expr) => {
        pub(super) fn $name(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rs3: RVReg, rm: u8) -> XResult {
            self.require_fp()?;
            let rm = self.resolve_rm(rm)?;
            let (mut a, b, mut c) = (self.read_f32(rs1), self.read_f32(rs2), self.read_f32(rs3));
            if $neg_a { a = F32::from_bits(a.to_bits() ^ 0x8000_0000); }
            if $neg_c { c = F32::from_bits(c.to_bits() ^ 0x8000_0000); }
            let r = self.with_flags(rm, |rm| a.fused_mul_add(b, c, rm));
            self.write_f32(rd, r);
            Ok(())
        }
    };
}

fp_fma_s!(fmadd_s,  false, false);  // rs1*rs2 + rs3
fp_fma_s!(fmsub_s,  false, true);   // rs1*rs2 - rs3
fp_fma_s!(fnmsub_s, true,  false);  // -rs1*rs2 + rs3
fp_fma_s!(fnmadd_s, true,  true);   // -rs1*rs2 - rs3
```

Unique handlers (not macro-generated):

```rust
// fsqrt — unary, no rs2
pub(super) fn fsqrt_s(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg, rm: u8) -> XResult {
    self.require_fp()?;
    let rm = self.resolve_rm(rm)?;
    let a = self.read_f32(rs1);
    let r = self.with_flags(rm, |rm| a.sqrt(rm));
    self.write_f32(rd, r);
    Ok(())
}

// Sign injection — NaN-boxing checked, no flags, explicit dirty (resolves R-003)
pub(super) fn fsgnj_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _rm: u8) -> XResult {
    self.require_fp()?;
    let (a, b) = (self.read_f32(rs1).to_bits(), self.read_f32(rs2).to_bits());
    self.write_f32(rd, F32::from_bits((a & 0x7FFF_FFFF) | (b & 0x8000_0000)));
    self.dirty_fp();
    Ok(())
}

pub(super) fn fsgnjn_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _rm: u8) -> XResult {
    self.require_fp()?;
    let (a, b) = (self.read_f32(rs1).to_bits(), self.read_f32(rs2).to_bits());
    self.write_f32(rd, F32::from_bits((a & 0x7FFF_FFFF) | (!b & 0x8000_0000)));
    self.dirty_fp();
    Ok(())
}

pub(super) fn fsgnjx_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _rm: u8) -> XResult {
    self.require_fp()?;
    let (a, b) = (self.read_f32(rs1).to_bits(), self.read_f32(rs2).to_bits());
    self.write_f32(rd, F32::from_bits(a ^ (b & 0x8000_0000)));
    self.dirty_fp();
    Ok(())
}

// FMIN/FMAX — softfloat handles NaN rules and flag setting
pub(super) fn fmin_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, _rm: u8) -> XResult {
    self.require_fp()?;
    let (a, b) = (self.read_f32(rs1), self.read_f32(rs2));
    let r = self.with_flags(SfRm::TiesToEven, |_| a.min(b));
    self.write_f32(rd, r);
    Ok(())
}

// fmax_s: same with a.max(b)

// FCLASS — reads f32 (NaN-boxing checked), writes GPR, no flags, no dirty
pub(super) fn fclass_s(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg, _rm: u8) -> XResult {
    self.require_fp()?;
    let v = self.read_f32(rs1);
    let class = classify_f32(v);
    self.set_gpr(rd, class)
}

// Move — transfer, no NaN-boxing check on read, no flags
pub(super) fn fmv_x_w(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg, _rm: u8) -> XResult {
    self.require_fp()?;
    self.set_gpr(rd, self.fpr[rs1] as u32 as i32 as Word)  // sign-extend 32→XLEN
}

pub(super) fn fmv_w_x(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg, _rm: u8) -> XResult {
    self.require_fp()?;
    self.fpr[rd] = 0xFFFF_FFFF_0000_0000 | (self.gpr[rs1] as u32 as u64);
    self.dirty_fp();
    Ok(())
}

// Load/Store — transfer, NaN-box on load
pub(super) fn flw(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
    self.require_fp()?;
    let addr = self.eff_addr(rs1, imm);
    let val = self.load(addr, 4)?;
    self.fpr[rd] = 0xFFFF_FFFF_0000_0000 | (val as u32 as u64);
    self.dirty_fp();
    Ok(())
}

pub(super) fn fsw(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
    self.require_fp()?;
    let addr = self.eff_addr(rs1, imm);
    self.store(addr, 4, self.fpr[rs2] as u32 as Word)?;
    Ok(())  // FSW does not dirty FS (only stores, no FP state modified)
}

// FCVT float→int — writes GPR, flags produced
pub(super) fn fcvt_w_s(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg, rm: u8) -> XResult {
    self.require_fp()?;
    let rm = self.resolve_rm(rm)?;
    let a = self.read_f32(rs1);
    let result = self.with_flags(rm, |rm| a.to_i32(rm, true));
    self.set_gpr(rd, result as i32 as Word)  // sign-extend on RV64
}

// FCVT int→float — writes fpr, flags produced
pub(super) fn fcvt_s_w(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg, rm: u8) -> XResult {
    self.require_fp()?;
    let rm = self.resolve_rm(rm)?;
    let result = self.with_flags(rm, |rm| F32::from_i32(self.gpr[rs1] as i32, rm));
    self.write_f32(rd, result);
    Ok(())
}
```

Classify helper:
```rust
fn classify_f32(v: F32) -> Word {
    (v.is_negative_infinity() as Word)
        | ((v.is_negative_normal() as Word) << 1)
        | ((v.is_negative_subnormal() as Word) << 2)
        | ((v.is_negative_zero() as Word) << 3)
        | ((v.is_positive_zero() as Word) << 4)
        | ((v.is_positive_subnormal() as Word) << 5)
        | ((v.is_positive_normal() as Word) << 6)
        | ((v.is_positive_infinity() as Word) << 7)
        | ((v.is_signaling_nan() as Word) << 8)
        | (((v.is_nan() && !v.is_signaling_nan()) as Word) << 9)
}
```

[**Phase 3: D Extension (26 ops) + Compressed D (4 ops)**]

D-extension macros mirror F, using `read_f64`/`write_f64` and `F64`:

```rust
macro_rules! fp_arith_d {
    ($name:ident, $op:ident) => {
        pub(super) fn $name(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rm: u8) -> XResult {
            self.require_fp()?;
            let rm = self.resolve_rm(rm)?;
            let (a, b) = (self.read_f64(rs1), self.read_f64(rs2));
            let r = self.with_flags(rm, |rm| a.$op(b, rm));
            self.write_f64(rd, r);
            Ok(())
        }
    };
}

// fp_cmp_d!, fp_fma_d! — same pattern with read_f64/write_f64
```

Compressed D handlers in `inst/compressed.rs` (same as 01_PLAN, unchanged).

Instruction patterns for D and compressed D (same as 01_PLAN, unchanged).

[**Phase 4: Integration**]

1. DTS: `riscv,isa = "rv64imafdcsu_sstc"`
2. `mstatus` side-effects: recompute SD when mstatus/sstatus written
3. `CoreContext`: add `fprs` field for debug register inspection
4. Debug ISA string: `rv64imafdc`
5. `format_mnemonic`: FP instruction disassembly
6. Validation: `make fmt`, `make clippy`, `make test`, `make run`, am-tests, benchmarks

---

## Trade-offs

- T-1: **Softfloat backend** — `softfloat-wrapper` with `riscv` feature. Confirmed per-op rounding, flags, FMA, RISC-V NaN canonicalization. Fallback: `softfloat-sys` if needed.

- T-2: **FP register storage** — `[u64; 32]` raw bits. Confirmed.

- T-3: **R4/rm design** — `DecodedInst::R` gains `rm: u8` field; `R4` has `rs3 + rm`. No hidden state. Integer R-type handlers ignore `_rm`. This is the minimal-churn explicit approach per TR-1.

---

## Validation

[**Unit Tests**]
- V-UT-1: NaN-boxing roundtrip; invalid boxing → canonical NaN
- V-UT-2: Rounding mode resolution: rm=0..4 valid; rm=7+frm=5 → error
- V-UT-3: `fflags` sticky accumulation via `with_flags`
- V-UT-4: FP CSR composite: write `fcsr=0xE5` → `fp_csr_read(0x001)=0x05`, `fp_csr_read(0x002)=0x7`; write `fflags=0x1F` → `fcsr[4:0]=0x1F`; write `frm=3` → `fcsr[7:5]=3`
- V-UT-5: FS gating: FS=Off → IllegalInstruction; FS=Initial → execute + FS=Dirty
- V-UT-6: `mstatus.SD` = 1 when FS=Dirty
- V-UT-7: Decoder: all 56 FP patterns + R4 rs3/rm extraction
- V-UT-8: `fclass` all 10 categories
- V-UT-9: `with_flags` marks FS dirty even for flag-only instructions (feq, fcvt.w)

[**Integration Tests**]
- V-IT-1: F arithmetic with all 5 rounding modes
- V-IT-2: D arithmetic with all 5 rounding modes
- V-IT-3: FMA: true fused multiply-add (single rounding)
- V-IT-4: Load/store roundtrip; `flw` NaN-boxes
- V-IT-5: Cross-precision convert: `fcvt.d.s` → `fcvt.s.d` roundtrip
- V-IT-6: Compressed D: `c_fld`/`c_fsd`/`c_fldsp`/`c_fsdsp` roundtrip
- V-IT-7: All existing tests unaffected

[**Failure / Robustness Validation**]
- V-F-1: FS=Off trap → OS sets FS → retry succeeds
- V-F-2: `fadd.s(sNaN, 1.0)` → canonical NaN + NV; `fadd.s(qNaN, 1.0)` → canonical NaN, no NV
- V-F-3: `fdiv.s(1.0, 0.0)` → +inf + DZ; `fdiv.s(0.0, 0.0)` → canonical NaN + NV
- V-F-4: `fmul.s(FLT_MAX, 2.0)` → +inf + OF + NX
- V-F-5: FCVT invalid: `fcvt.w.s(+inf)` → 2^31-1 + NV; `fcvt.wu.s(NaN)` → 2^32-1 + NV

[**Edge Case Validation (NaN-sensitive)**] — resolves R-005
- V-E-1: Signed zero: `fmin.s(-0.0, +0.0)` → `-0.0`; `fmax.s(-0.0, +0.0)` → `+0.0`
- V-E-2: `fmin.s(qNaN, 1.0)` → `1.0`, NV NOT set (one-qNaN rule)
- V-E-3: `fmin.s(sNaN, 1.0)` → `1.0`, NV SET (one-sNaN rule)
- V-E-4: `fmin.s(qNaN, qNaN)` → canonical NaN, NV NOT set (two-qNaN rule)
- V-E-5: `fmin.s(sNaN, qNaN)` → canonical NaN, NV SET (sNaN present)
- V-E-6: `feq.s(qNaN, qNaN)` → 0, NV NOT set; `feq.s(sNaN, 1.0)` → 0, NV SET
- V-E-7: `flt.s(qNaN, 1.0)` → 0, NV SET (FLT/FLE signal on any NaN)
- V-E-8: NaN-boxing edge: `fmv.d.x` → `fadd.s` reads canonical NaN
- V-E-9: `fmv.x.w` on RV64: sign-extends 32-bit
- V-E-10: `fsgnj.s` on un-NaN-boxed input → operates on canonical NaN bits (R-003 fix)
- V-E-11: FMA: `fmadd.s(+inf, 0.0, qNaN)` → canonical NaN + NV
- V-E-12: `fcvt.w.s(+0.5, RTZ)` → 0; `fcvt.w.s(+0.5, RUP)` → 1

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (F ext) | V-IT-1, V-IT-3, V-IT-4, V-IT-5, V-UT-7 |
| G-2 (D ext) | V-IT-2, V-IT-3, V-IT-4, V-IT-5, V-UT-7 |
| G-3 (Compressed D) | V-IT-6, V-UT-7 |
| G-4 (IEEE 754) | V-UT-1..3, V-F-2..5, V-E-1..7, V-E-11..12 |
| G-5 (NaN-boxing) | V-UT-1, V-E-8, V-E-10 |
| G-6 (mstatus.FS) | V-UT-5, V-UT-6, V-UT-9, V-F-1 |
| G-7 (misa) | V-IT-7 |
| C-2 (R4-type) | V-UT-7, V-IT-3 |
| C-6 (FP CSR) | V-UT-4 |
| C-7 (FCVT table) | V-F-5 |
| I-4 (FS dirty) | V-UT-9 |
| I-10 (FMIN NaN) | V-E-2..5 |
