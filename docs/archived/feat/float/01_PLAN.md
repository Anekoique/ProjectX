# F/D Floating-Point Extension PLAN 01

> Status: Revised
> Feature: `float`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `00_MASTER.md`

---

## Summary

Implement RISC-V F (single-precision) and D (double-precision) floating-point extensions for RV64GC Linux userspace (busybox/buildroot, `lp64d` ABI). This round resolves all blocking findings from REVIEW 00: brings compressed D instructions (C.FLD/C.FSD/C.FLDSP/C.FSDSP) into scope, corrects FCVT invalid-input semantics per the official spec table, refines NaN validation into qNaN/sNaN cases, and adds substantially more implementation detail with concrete code as directed by MASTER.

## Log

[**Feature Introduce**]

- Compressed D load/store instructions (4 ops) now in scope per R-001
- Full FCVT invalid-input table from official RISC-V spec per R-002
- Expanded qNaN vs sNaN validation matrix per R-004
- Concrete code examples for all core helpers, dispatch, and instruction handlers per M-001
- Reference emulator patterns from KXemu per M-002
- `softfloat-wrapper` with `riscv` feature confirmed as backend — exposes per-op rounding, exception flags, FMA, and RISC-V NaN canonicalization per TR-1

[**Review Adjustments**]

- R-001 (HIGH): C.FLD/C.FSD/C.FLDSP/C.FSDSP added to Phase 3 with full decode/execute/validation
- R-002 (HIGH): C-7 replaced with spec-compliant FCVT invalid-input table (per-conversion, per-input)
- R-003 (MEDIUM): Phase 4 expanded — difftest scoped as explicit deferral with narrowed claims
- R-004 (MEDIUM): NaN validation split into qNaN/sNaN cases; FMIN/FMAX and FEQ/FLT/FLE distinguished

[**Master Compliance**]

- M-001: All phases now include concrete Rust code (data structures, helpers, instruction handlers, decoder patterns, CSR wiring)
- M-002: Studied KXemu's F/D implementation — adopted NaN-boxing via fill-high pattern, FCVT saturation semantics, C.FLD/FSD handler structure, and FCSR field extraction
- M-003: Code follows functional patterns (closures for load/store, combinators for flag accumulation), minimal mutation, clean helper boundaries

### Changes from Previous Round

[**Added**]
- Compressed D instructions: `c_fld`, `c_fsd`, `c_fldsp`, `c_fsdsp` (decode + execute + validation)
- Full FCVT invalid-input table (8 conversions × 5 input classes)
- Concrete code for: `RVCore` FP helpers, `R4` decoder extraction, `float.rs` instruction handlers, CSR composite wiring, `rv_inst_table!` expansion, `fclass` implementation
- `softfloat-wrapper` backend capability requirements and feature flag selection

[**Changed**]
- C-5: Changed from "compressed FP deferred" to "C.FLW/C.FSW deferred (RV32 only); C.FLD/C.FSD/C.FLDSP/C.FSDSP in scope"
- C-7: Replaced informal prose with spec-compliant per-conversion table
- Phase 4: Difftest FP integration explicitly deferred (narrowed validation claims)
- Validation: NaN cases split into qNaN/sNaN; comparison instruction flag behavior distinguished

[**Removed**]
- Claim of difftest FP comparison readiness in this round (deferred to follow-up)

[**Unresolved**]
- Difftest FP state sync (QEMU/Spike backends need FP register getters) — deferred to post-boot follow-up
- Compressed single-precision (C.FLW/C.FSW/C.FLWSP/C.FSWSP) — only exists on RV32; not needed for RV64GC

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | C.FLD/C.FSD/C.FLDSP/C.FSDSP added to Phase 3 with decode, execute, and validation |
| Review | R-002 | Accepted | C-7 replaced with full spec-compliant FCVT invalid-input table |
| Review | R-003 | Accepted (narrowed) | Difftest FP extension explicitly deferred; validation claims narrowed to unit/integration tests |
| Review | R-004 | Accepted | NaN validation split: qNaN/sNaN for arithmetic, FEQ vs FLT/FLE flag rules, FMIN/FMAX one-NaN behavior |
| Review | TR-1 | Accepted | `softfloat-wrapper` with `riscv` feature — confirmed per-op rounding, exception flags, FMA, RISC-V NaN canonicalization |
| Review | TR-2 | Kept | `[u64; 32]` for FP storage, `DecodedInst::R4` for FMA — with concrete code showing `rv_inst_table!` and decoder updates |
| Master | M-001 | Applied | All phases now include concrete Rust code |
| Master | M-002 | Applied | KXemu patterns studied and adopted where applicable |
| Master | M-003 | Applied | Functional patterns, minimal mutation, clean helper boundaries |

---

## Spec

[**Goals**]
- G-1: Implement all RV64F instructions (26 ops)
- G-2: Implement all RV64D instructions (26 ops)
- G-3: Implement compressed D load/store (C.FLD, C.FSD, C.FLDSP, C.FSDSP — 4 ops)
- G-4: Strict IEEE 754-2008 compliance via `softfloat-wrapper` with `riscv` feature
- G-5: NaN-boxing: single-precision values NaN-boxed in 64-bit f-registers; invalid boxing reads produce canonical NaN
- G-6: `mstatus.FS` state machine (Off/Initial/Clean/Dirty) with illegal-instruction trap when FS=Off
- G-7: `misa` bits F(5) and D(3) advertised
- G-8: Boot Linux with buildroot/busybox `lp64d` initramfs to interactive shell

- NG-1: No Zfh (half-precision) or Q (quad-precision)
- NG-2: No FP exception trapping (RISC-V base spec: no trap support)
- NG-3: No compressed single-precision (C.FLW/C.FSW — RV32 only, not needed for RV64GC)
- NG-4: No difftest FP state comparison in this round (deferred to follow-up)

[**Architecture**]

```
                    RVCore
                    +--------------------------+
                    | gpr: [Word; 32]          |  (existing)
                    | fpr: [u64; 32]           |  (NEW: f0-f31, 64-bit NaN-boxed)
                    | pc, npc, csr, bus, ...   |
                    +--------------------------+
                           |
              +------------+-----------+
              |            |           |
         inst/float.rs  csr.rs    inst/compressed.rs
         (F/D ops)      (fcsr +   (C.FLD/FSD/FLDSP/FSDSP)
                         FS track)
              |
         softfloat-wrapper (riscv feature)
         └─ Berkeley softfloat-3 via FFI
            ├─ Per-op RoundingMode parameter
            ├─ ExceptionFlags after each op
            ├─ F32/F64 from_bits/to_bits
            └─ fused_mul_add for FMA
```

[**Invariants**]
- I-1: `fpr[i]` stores 64-bit raw bits. Single-precision values are NaN-boxed: `bits[63:32] = 0xFFFF_FFFF`.
- I-2: When an F-extension instruction reads `fpr[i]` and `bits[63:32] != 0xFFFF_FFFF`, the value is canonical single NaN (`0x7FC0_0000`).
- I-3: All FP instructions raise `IllegalInstruction` when `mstatus.FS == Off` (`bits[14:13] == 0b00`).
- I-4: Every FP instruction that modifies FP state sets `mstatus.FS = Dirty` (`0b11`).
- I-5: `fflags` are sticky (OR-accumulated). Only software clears them.
- I-6: `fcsr[7:5] = frm`, `fcsr[4:0] = fflags`. Writes to any one update the composite.
- I-7: Reserved rounding modes (5, 6) in `frm` cause illegal-instruction when `rm=7` (DYN) selects them.
- I-8: `mstatus.SD` (bit 63 on RV64) = `(FS == 0b11)`. Automatically maintained.
- I-9: FCVT invalid-input results follow the spec table exactly (see C-7 below).

[**Data Structure**]

Add `fpr` field to `RVCore`:

```rust
pub struct RVCore {
    gpr: [Word; 32],
    fpr: [u64; 32],       // NEW: f0-f31, raw 64-bit NaN-boxed storage
    pc: VirtAddr,
    npc: VirtAddr,
    pub(crate) csr: CsrFile,
    // ... rest unchanged
}
```

Add `R4` variant to `DecodedInst` for fused multiply-add:

```rust
pub enum DecodedInst {
    R  { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg },
    R4 { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg, rs3: RVReg }, // NEW
    I  { kind: InstKind, rd: RVReg, rs1: RVReg, imm: SWord },
    S  { kind: InstKind, rs1: RVReg, rs2: RVReg, imm: SWord },
    B  { kind: InstKind, rs1: RVReg, rs2: RVReg, imm: SWord },
    U  { kind: InstKind, rd: RVReg, imm: SWord },
    J  { kind: InstKind, rd: RVReg, imm: SWord },
    C  { kind: InstKind, inst: u32 },
}
```

`R4` extraction in `DecodedInst::from_raw`:

```rust
InstFormat::R4 => Ok(Self::R4 {
    kind,
    rd: reg(7)?,
    rs1: reg(15)?,
    rs2: reg(20)?,
    rs3: RVReg::try_from(((inst >> 27) & 0x1F) as u8).map_err(|_| XError::InvalidReg)?,
}),
```

[**API Surface**]

Core FP helpers on `RVCore` in `inst/float.rs`:

```rust
use softfloat_wrapper::{ExceptionFlags, Float, RoundingMode as SfRm, F32, F64};

/// Rounding mode — maps rm field to softfloat enum.
fn sf_rounding_mode(rm: u8) -> Option<SfRm> {
    match rm {
        0 => Some(SfRm::TiesToEven),
        1 => Some(SfRm::TowardZero),
        2 => Some(SfRm::TowardNegative),
        3 => Some(SfRm::TowardPositive),
        4 => Some(SfRm::TiesToAway),
        _ => None,
    }
}

impl RVCore {
    /// Trap if mstatus.FS == Off.
    fn require_fp(&self) -> XResult {
        let fs = (self.csr.get(CsrAddr::mstatus) >> 13) & 0x3;
        if fs == 0 {
            Err(XError::InvalidInst)  // caught by trap_on_err -> IllegalInstruction
        } else {
            Ok(())
        }
    }

    /// Set mstatus.FS = Dirty (0b11) and SD = 1.
    fn dirty_fp(&mut self) {
        let ms = self.csr.get(CsrAddr::mstatus);
        self.csr.set(CsrAddr::mstatus, ms | MStatus::FS.bits() | MStatus::SD.bits());
    }

    /// Read single-precision with NaN-boxing check.
    fn read_f32(&self, reg: RVReg) -> F32 {
        let bits = self.fpr[reg];
        let is_nanboxed = (bits >> 32) == 0xFFFF_FFFF;
        F32::from_bits(if is_nanboxed { bits as u32 } else { 0x7FC0_0000 })
    }

    /// Write single-precision with NaN-boxing (upper 32 = all 1s).
    fn write_f32(&mut self, reg: RVReg, val: F32) {
        self.fpr[reg] = 0xFFFF_FFFF_0000_0000 | val.to_bits() as u64;
    }

    /// Read double-precision (raw 64-bit).
    fn read_f64(&self, reg: RVReg) -> F64 {
        F64::from_bits(self.fpr[reg])
    }

    /// Write double-precision (raw 64-bit).
    fn write_f64(&mut self, reg: RVReg, val: F64) {
        self.fpr[reg] = val.to_bits();
    }

    /// Resolve rounding mode: rm=7 reads frm; reserved modes -> Err.
    fn resolve_rm(&self, rm: u8) -> XResult<SfRm> {
        let effective = if rm == 7 {
            (self.csr.get(CsrAddr::fcsr) >> 5) as u8 & 0x7
        } else {
            rm
        };
        sf_rounding_mode(effective).ok_or(XError::InvalidInst)
    }

    /// OR exception flags into fflags portion of fcsr.
    fn accrue_fflags(&mut self, flags: ExceptionFlags) {
        let bits = flags.to_bits() as Word;
        let fcsr = self.csr.get(CsrAddr::fcsr);
        self.csr.set(CsrAddr::fcsr, fcsr | (bits & 0x1F));
    }

    /// Clear softfloat exception flags, execute op, accumulate result flags.
    fn with_flags<T>(&mut self, rm: SfRm, op: impl FnOnce(SfRm) -> T) -> T {
        let mut flags = ExceptionFlags::default();
        flags.set();               // clear global flags
        let result = op(rm);
        flags.get();               // read accumulated flags
        self.accrue_fflags(flags);
        result
    }
}
```

[**Constraints**]

- C-1: FP and GPR share the 5-bit register encoding. The decoder extracts `rd`/`rs1`/`rs2` as `RVReg`; handlers index `fpr[]` using the same value.
- C-2: R4-type (FMA) needs `rs3` from bits `[31:27]`. New `DecodedInst::R4` variant added.
- C-3: `funct3` (bits `[14:12]`) is the `rm` field for arithmetic, but encodes sub-operation for FSGNJ (000/001/010) and comparison (000/001/010). Handlers interpret contextually.
- C-4: `softfloat-wrapper` dependency:
  ```toml
  [dependencies]
  softfloat-wrapper = { version = "0.3", default-features = false, features = ["riscv"] }
  ```
  The `riscv` feature selects RISC-V NaN canonicalization rules in the underlying Berkeley softfloat-3.
- C-5: Compressed D instructions (C.FLD/C.FSD/C.FLDSP/C.FSDSP) are in scope. These share the same opcode slots as C.FLW/C.FSW on RV32; on RV64 those slots are free and assigned to C.FLD/C.FSD. Compressed single-precision (C.FLW/C.FSW) is RV32-only and deferred (NG-3).
- C-6: `fcsr` (0x003) is the single storage slot. `fflags` (0x001) views `fcsr[4:0]`; `frm` (0x002) views `fcsr[7:5]`. Writes to one reflect in the composite.
- C-7: **FCVT invalid-input behavior** (per official RISC-V spec Table 11.1 / Table 12.1):

  | Conversion | +∞ input | -∞ input | NaN input | Positive overflow | Negative overflow |
  |---|---|---|---|---|---|
  | `FCVT.W.{S,D}` (signed 32) | 2^31 - 1 | -2^31 | 2^31 - 1 | 2^31 - 1 | -2^31 |
  | `FCVT.WU.{S,D}` (unsigned 32) | 2^32 - 1 | 0 | 2^32 - 1 | 2^32 - 1 | 0 |
  | `FCVT.L.{S,D}` (signed 64) | 2^63 - 1 | -2^63 | 2^63 - 1 | 2^63 - 1 | -2^63 |
  | `FCVT.LU.{S,D}` (unsigned 64) | 2^64 - 1 | 0 | 2^64 - 1 | 2^64 - 1 | 0 |

  All invalid-input cases set NV flag. `FCVT.W*` results on RV64 are sign-extended to 64 bits.

- C-8: `mstatus.SD` = `(FS == 0b11)`. Updated whenever FS changes (in `dirty_fp` and CSR write side-effects).

---

## Implement

### Execution Flow

[**Main Flow**] (example: `fadd.s rd, rs1, rs2`)
1. Decoder matches `0000000 ????? ????? ??? ????? 10100 11` -> `DecodedInst::R { kind: fadd_s, rd, rs1, rs2 }`
2. `dispatch()` routes to `Self::fadd_s(self, rd, rs1, rs2)` in `inst/float.rs`
3. `self.require_fp()?` — traps if FS=Off
4. `let rm = self.resolve_rm(funct3)?` — extract rounding mode (funct3 from raw inst bits stored in kind metadata, or extracted at decode time)
5. `let a = self.read_f32(rs1)` — NaN-boxing check
6. `let b = self.read_f32(rs2)` — NaN-boxing check
7. `let result = self.with_flags(rm, |rm| a.add(b, rm))` — softfloat add + flag accumulation
8. `self.write_f32(rd, result)` — NaN-box and store
9. `self.dirty_fp()` — mark FS=Dirty

[**Main Flow**] (example: `fmadd.s rd, rs1, rs2, rs3`)
1. Decoder matches R4-type: `????? 00 ????? ????? ??? ????? 10000 11` -> `DecodedInst::R4 { kind: fmadd_s, rd, rs1, rs2, rs3 }`
2. `dispatch()` routes to `Self::fmadd_s(self, rd, rs1, rs2, rs3)`
3. `self.require_fp()?`
4. `let rm = self.resolve_rm(bits(14,12) from raw)?`
5. `let (a, b, c) = (self.read_f32(rs1), self.read_f32(rs2), self.read_f32(rs3))`
6. `let result = self.with_flags(rm, |rm| a.fused_mul_add(b, c, rm))` — true FMA via softfloat
7. `self.write_f32(rd, result)` + `self.dirty_fp()`

[**Main Flow**] (example: `c_fld rd', rs1', offset`)
1. Decoder matches compressed: `001 ? ????? ????? 00` -> `DecodedInst::C { kind: c_fld, inst }`
2. Handler extracts `rd' = reg_prime(inst, 4, 2)`, `rs1' = reg_prime(inst, 9, 7)`, `imm` from bits
3. `self.require_fp()?`
4. `let addr = self.eff_addr(rs1', imm)`
5. `let val = self.load(addr, 8)?` — 8-byte load
6. `self.fpr[rd'] = val as u64` — raw 64-bit write (no NaN-boxing for D)
7. `self.dirty_fp()`

[**Failure Flow**]
1. `mstatus.FS == Off` -> `raise_trap(IllegalInstruction, raw_inst)`
2. `rm=7` (DYN) and `frm` contains 5/6/7 -> `raise_trap(IllegalInstruction, raw_inst)`
3. FP exception -> set `fflags` bits; no trap (spec: no FP trap support)
4. Invalid FCVT input (NaN/±∞/overflow) -> return saturated value per C-7 table + set NV flag

[**State Transition**]
- FS: Off -> any FP instruction -> IllegalInstruction trap (no state change)
- FS: Initial/Clean -> FP instruction modifies FP state -> FS: Dirty
- FS: Dirty -> stays Dirty until OS resets to Clean/Initial via CSR write
- SD bit = (FS == Dirty) — automatically updated by `dirty_fp()` and CSR side-effects

### Implementation Plan

[**Phase 1: Infrastructure**]

**1.1 Add `fpr` to `RVCore`**:
```rust
// In cpu/riscv/mod.rs
pub struct RVCore {
    gpr: [Word; 32],
    fpr: [u64; 32],  // NEW
    // ...
}

impl RVCore {
    pub fn new() -> Self {
        // ...
        Self {
            gpr: [0; 32],
            fpr: [0; 32],  // FS=Initial means zero-initialized
            // ...
        }
    }
}
```

Reset must clear `fpr`:
```rust
fn reset(&mut self) -> XResult {
    self.gpr.fill(0);
    self.fpr.fill(0);  // NEW
    // ...
}
```

**1.2 Add `R4` variant**:

In `isa/riscv/inst.rs`, add `R4` format:
```rust
pub enum InstFormat {
    R, R4,  // NEW
    I, S, B, U, J,
    CR, CI, CSS, CIW, CL, CS, CA, CB, CJ,
}
```

In `isa/riscv/decoder.rs`, add `R4` extraction:
```rust
InstFormat::R4 => Ok(Self::R4 {
    kind,
    rd: reg(7)?,
    rs1: reg(15)?,
    rs2: reg(20)?,
    rs3: RVReg::try_from(((inst >> 27) & 0x1F) as u8).map_err(|_| XError::InvalidReg)?,
}),
```

In `utils/macros.rs`, add R4 to the instruction table:
```rust
macro_rules! rv_inst_table {
    ($macro:ident) => {
        $macro! {
            (R, (rd, rs1, rs2), [add, ... /* existing */]),
            (R4, (rd, rs1, rs2, rs3), [fmadd_s, fmsub_s, fnmsub_s, fnmadd_s,
                                       fmadd_d, fmsub_d, fnmsub_d, fnmadd_d]),
            (I, (rd, rs1, imm), [addi, ... /* existing */, flw, fld]),
            (S, (rs1, rs2, imm), [sb, sh, sw, sd, fsw, fsd]),
            // ... rest unchanged
        }
    };
}
```

The `build_dispatch` macro in `cpu/riscv/inst.rs` will automatically handle R4 dispatch since it already matches on format variant and destructures the fields.

**1.3 Register FP CSRs**:

In `csr.rs`, add composite CSR entries:
```rust
csr_table! {
    // ... existing entries ...

    // ---- Floating-Point CSRs ----
    fcsr   = 0x003 => [RW(0xFF)],     // frm[7:5] | fflags[4:0]
    fflags = 0x001 => [RW(0x1F) => fcsr(0x1F)],   // shadow: low 5 bits of fcsr
    frm    = 0x002 => [RW(0x07)],     // independent 3-bit storage
}
```

Note: `fflags` is a view of `fcsr[4:0]` (shadow with view_mask=0x1F). `frm` needs special handling — it maps to `fcsr[7:5]`, which the current shadow mechanism doesn't support (no shift). Two options:

**Option A (preferred)**: Store `fcsr` as the canonical slot. Add CSR write side-effect for `fflags` and `frm` that re-compose `fcsr`:
```rust
// In csr/ops.rs — csr_write_side_effects
0x001 /* fflags */ => {
    let fcsr = self.csr.get(CsrAddr::fcsr);
    let fflags = self.csr.get_by_addr(0x001) & 0x1F;
    self.csr.set(CsrAddr::fcsr, (fcsr & !0x1F) | fflags);
}
0x002 /* frm */ => {
    let fcsr = self.csr.get(CsrAddr::fcsr);
    let frm = self.csr.get_by_addr(0x002) & 0x7;
    self.csr.set(CsrAddr::fcsr, (fcsr & !0xE0) | (frm << 5));
}
0x003 /* fcsr */ => {
    let fcsr = self.csr.get(CsrAddr::fcsr) & 0xFF;
    self.csr.set(CsrAddr::fflags, fcsr & 0x1F);
    self.csr.set(CsrAddr::frm, (fcsr >> 5) & 0x7);
}
```

This ensures reads of any one CSR always reflect writes to the others.

**1.4 Update `misa`**:
```rust
// In csr.rs — add F(5) and D(3) bits
#[cfg(isa64)]
const MISA_VALUE: Word = (2 << 62) | (1 << 20) | (1 << 18) | (1 << 12) | (1 << 8)
    | (1 << 5)  // F
    | (1 << 3)  // D
    | (1 << 2) | 1;
```

**1.5 Update `mstatus.FS` initialization**:
```rust
// In CsrFile::default()
regs[CsrAddr::mstatus as usize] = MStatus::FS.bits(); // FS = Initial (01 << 13)
```

Wait — `MStatus::FS` is `0b11 << 13` (the mask, not Initial). We need:
```rust
const FS_INITIAL: Word = 1 << 13;  // 0b01 << 13
regs[CsrAddr::mstatus as usize] = FS_INITIAL;
```

**1.6 Add `softfloat-wrapper` dependency**:
```toml
# xemu/xcore/Cargo.toml
[dependencies]
softfloat-wrapper = { version = "0.3", default-features = false, features = ["riscv"] }
```

[**Phase 2: F Extension Instructions (26 ops)**]

All handlers in `cpu/riscv/inst/float.rs`. Pattern: `require_fp` -> read operands -> softfloat op -> write result -> `dirty_fp`.

**2.1 Instruction patterns** (`riscv.instpat`):

```
// F extension — Load/Store (opcode 0000111/0100111)
INSTPAT("??????? ????? ????? 010 ????? 00001 11", flw        , I);
INSTPAT("??????? ????? ????? 010 ????? 01001 11", fsw        , S);

// F extension — R-type arithmetic (opcode 1010011, fmt=00)
INSTPAT("0000000 ????? ????? ??? ????? 10100 11", fadd_s     , R);
INSTPAT("0000100 ????? ????? ??? ????? 10100 11", fsub_s     , R);
INSTPAT("0001000 ????? ????? ??? ????? 10100 11", fmul_s     , R);
INSTPAT("0001100 ????? ????? ??? ????? 10100 11", fdiv_s     , R);
INSTPAT("0101100 00000 ????? ??? ????? 10100 11", fsqrt_s    , R);
INSTPAT("0010000 ????? ????? 000 ????? 10100 11", fsgnj_s    , R);
INSTPAT("0010000 ????? ????? 001 ????? 10100 11", fsgnjn_s   , R);
INSTPAT("0010000 ????? ????? 010 ????? 10100 11", fsgnjx_s   , R);
INSTPAT("0010100 ????? ????? 000 ????? 10100 11", fmin_s     , R);
INSTPAT("0010100 ????? ????? 001 ????? 10100 11", fmax_s     , R);
INSTPAT("1100000 00000 ????? ??? ????? 10100 11", fcvt_w_s   , R);
INSTPAT("1100000 00001 ????? ??? ????? 10100 11", fcvt_wu_s  , R);
INSTPAT("1110000 00000 ????? 000 ????? 10100 11", fmv_x_w    , R);
INSTPAT("1010000 ????? ????? 010 ????? 10100 11", feq_s      , R);
INSTPAT("1010000 ????? ????? 001 ????? 10100 11", flt_s      , R);
INSTPAT("1010000 ????? ????? 000 ????? 10100 11", fle_s      , R);
INSTPAT("1110000 00000 ????? 001 ????? 10100 11", fclass_s   , R);
INSTPAT("1101000 00000 ????? ??? ????? 10100 11", fcvt_s_w   , R);
INSTPAT("1101000 00001 ????? ??? ????? 10100 11", fcvt_s_wu  , R);
INSTPAT("1111000 00000 ????? 000 ????? 10100 11", fmv_w_x    , R);

// F extension — RV64 only
INSTPAT("1100000 00010 ????? ??? ????? 10100 11", fcvt_l_s   , R);
INSTPAT("1100000 00011 ????? ??? ????? 10100 11", fcvt_lu_s  , R);
INSTPAT("1101000 00010 ????? ??? ????? 10100 11", fcvt_s_l   , R);
INSTPAT("1101000 00011 ????? ??? ????? 10100 11", fcvt_s_lu  , R);

// F extension — R4-type FMA (opcode varies, fmt=00)
INSTPAT("????? 00 ????? ????? ??? ????? 10000 11", fmadd_s   , R4);
INSTPAT("????? 00 ????? ????? ??? ????? 10001 11", fmsub_s   , R4);
INSTPAT("????? 00 ????? ????? ??? ????? 10010 11", fnmsub_s  , R4);
INSTPAT("????? 00 ????? ????? ??? ????? 10011 11", fnmadd_s  , R4);
```

**2.2 Example instruction handlers**:

```rust
// inst/float.rs

impl RVCore {
    // --- Arithmetic ---

    pub(super) fn fadd_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(self.funct3())?;  // funct3 from decode
        let (a, b) = (self.read_f32(rs1), self.read_f32(rs2));
        let result = self.with_flags(rm, |rm| a.add(b, rm));
        self.write_f32(rd, result);
        self.dirty_fp();
        Ok(())
    }

    // fsub_s, fmul_s, fdiv_s follow the same pattern with sub/mul/div

    pub(super) fn fsqrt_s(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(self.funct3())?;
        let a = self.read_f32(rs1);
        let result = self.with_flags(rm, |rm| a.sqrt(rm));
        self.write_f32(rd, result);
        self.dirty_fp();
        Ok(())
    }

    // --- Sign injection (no rounding, no flags) ---

    pub(super) fn fsgnj_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.require_fp()?;
        let a = self.fpr[rs1] as u32;
        let b = self.fpr[rs2] as u32;
        let result = (a & 0x7FFF_FFFF) | (b & 0x8000_0000);
        self.fpr[rd] = 0xFFFF_FFFF_0000_0000 | result as u64;
        self.dirty_fp();
        Ok(())
    }

    pub(super) fn fsgnjn_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.require_fp()?;
        let a = self.fpr[rs1] as u32;
        let b = self.fpr[rs2] as u32;
        let result = (a & 0x7FFF_FFFF) | (!b & 0x8000_0000);
        self.fpr[rd] = 0xFFFF_FFFF_0000_0000 | result as u64;
        self.dirty_fp();
        Ok(())
    }

    pub(super) fn fsgnjx_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.require_fp()?;
        let a = self.fpr[rs1] as u32;
        let b = self.fpr[rs2] as u32;
        let result = a ^ (b & 0x8000_0000);
        self.fpr[rd] = 0xFFFF_FFFF_0000_0000 | result as u64;
        self.dirty_fp();
        Ok(())
    }

    // --- Comparison (writes GPR, sets fflags) ---

    pub(super) fn feq_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.require_fp()?;
        let (a, b) = (self.read_f32(rs1), self.read_f32(rs2));
        // FEQ: only signals NV for signaling NaN (not quiet NaN)
        let result = self.with_flags(SfRm::TiesToEven, |_| a.eq(b));
        self.set_gpr(rd, result as Word)
    }

    pub(super) fn flt_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
        self.require_fp()?;
        let (a, b) = (self.read_f32(rs1), self.read_f32(rs2));
        // FLT/FLE: signals NV for any NaN (quiet or signaling)
        let result = self.with_flags(SfRm::TiesToEven, |_| a.lt(b));
        self.set_gpr(rd, result as Word)
    }

    // fle_s: same pattern with a.le(b)

    // --- Classify (writes GPR, no flags) ---

    pub(super) fn fclass_s(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg) -> XResult {
        self.require_fp()?;
        let v = self.read_f32(rs1);
        let class = (v.is_negative_infinity() as Word)        // bit 0
            | ((v.is_negative_normal() as Word) << 1)         // bit 1
            | ((v.is_negative_subnormal() as Word) << 2)      // bit 2
            | ((v.is_negative_zero() as Word) << 3)           // bit 3
            | ((v.is_positive_zero() as Word) << 4)           // bit 4
            | ((v.is_positive_subnormal() as Word) << 5)      // bit 5
            | ((v.is_positive_normal() as Word) << 6)         // bit 6
            | ((v.is_positive_infinity() as Word) << 7)       // bit 7
            | ((v.is_signaling_nan() as Word) << 8)           // bit 8
            | (((v.is_nan() && !v.is_signaling_nan()) as Word) << 9); // bit 9: qNaN
        self.set_gpr(rd, class)
    }

    // --- Conversions (FCVT) ---

    pub(super) fn fcvt_w_s(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(self.funct3())?;
        let a = self.read_f32(rs1);
        let result = self.with_flags(rm, |rm| a.to_i32(rm, true));  // exact=true for NX
        // Sign-extend 32-bit result to Word on RV64
        self.set_gpr(rd, result as i32 as Word)
    }

    pub(super) fn fcvt_wu_s(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(self.funct3())?;
        let a = self.read_f32(rs1);
        let result = self.with_flags(rm, |rm| a.to_u32(rm, true));
        // RV64: sign-extend the 32-bit result
        self.set_gpr(rd, result as i32 as Word)
    }

    // fcvt_l_s, fcvt_lu_s: RV64 only, use a.to_i64/to_u64

    pub(super) fn fcvt_s_w(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(self.funct3())?;
        let val = self.gpr[rs1] as i32;
        let result = self.with_flags(rm, |rm| F32::from_i32(val, rm));
        self.write_f32(rd, result);
        self.dirty_fp();
        Ok(())
    }

    // --- Move (no conversion, no flags) ---

    pub(super) fn fmv_x_w(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg) -> XResult {
        self.require_fp()?;
        // Sign-extend lower 32 bits of fpr to XLEN
        let bits = self.fpr[rs1] as u32;
        self.set_gpr(rd, bits as i32 as Word)
    }

    pub(super) fn fmv_w_x(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg) -> XResult {
        self.require_fp()?;
        let bits = self.gpr[rs1] as u32;
        self.fpr[rd] = 0xFFFF_FFFF_0000_0000 | bits as u64;  // NaN-box
        self.dirty_fp();
        Ok(())
    }

    // --- Load/Store ---

    pub(super) fn flw(&mut self, rd: RVReg, rs1: RVReg, imm: SWord) -> XResult {
        self.require_fp()?;
        let addr = self.eff_addr(rs1, imm);
        let val = self.load(addr, 4)?;
        self.fpr[rd] = 0xFFFF_FFFF_0000_0000 | (val as u32 as u64);  // NaN-box
        self.dirty_fp();
        Ok(())
    }

    pub(super) fn fsw(&mut self, rs1: RVReg, rs2: RVReg, imm: SWord) -> XResult {
        self.require_fp()?;
        let addr = self.eff_addr(rs1, imm);
        let val = self.fpr[rs2] as u32;  // lower 32 bits only
        self.store(addr, 4, val as Word)?;
        Ok(())
    }

    // --- FMA (R4-type) ---

    pub(super) fn fmadd_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rs3: RVReg) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(self.funct3())?;
        let (a, b, c) = (self.read_f32(rs1), self.read_f32(rs2), self.read_f32(rs3));
        let result = self.with_flags(rm, |rm| a.fused_mul_add(b, c, rm));
        self.write_f32(rd, result);
        self.dirty_fp();
        Ok(())
    }

    pub(super) fn fmsub_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rs3: RVReg) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(self.funct3())?;
        let (a, b, c) = (self.read_f32(rs1), self.read_f32(rs2), self.read_f32(rs3));
        let neg_c = F32::from_bits(c.to_bits() ^ 0x8000_0000);  // negate rs3
        let result = self.with_flags(rm, |rm| a.fused_mul_add(b, neg_c, rm));
        self.write_f32(rd, result);
        self.dirty_fp();
        Ok(())
    }

    pub(super) fn fnmsub_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rs3: RVReg) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(self.funct3())?;
        let (a, b, c) = (self.read_f32(rs1), self.read_f32(rs2), self.read_f32(rs3));
        let neg_a = F32::from_bits(a.to_bits() ^ 0x8000_0000);  // negate rs1
        let result = self.with_flags(rm, |rm| neg_a.fused_mul_add(b, c, rm));
        self.write_f32(rd, result);
        self.dirty_fp();
        Ok(())
    }

    pub(super) fn fnmadd_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rs3: RVReg) -> XResult {
        self.require_fp()?;
        let rm = self.resolve_rm(self.funct3())?;
        let (a, b, c) = (self.read_f32(rs1), self.read_f32(rs2), self.read_f32(rs3));
        let neg_a = F32::from_bits(a.to_bits() ^ 0x8000_0000);
        let neg_c = F32::from_bits(c.to_bits() ^ 0x8000_0000);
        let result = self.with_flags(rm, |rm| neg_a.fused_mul_add(b, neg_c, rm));
        self.write_f32(rd, result);
        self.dirty_fp();
        Ok(())
    }
}
```

**2.3 Rounding mode access**: The `funct3` field (bits [14:12]) encodes `rm` for arithmetic instructions. Since the R-type decoder discards funct3 for instructions using it as a register disambiguator, we need to preserve it. Two approaches:

- **Option A**: Store raw instruction bits in `DecodedInst::R` and extract at handler time
- **Option B (preferred)**: The `funct3` is accessible from the raw instruction in `execute()`. Add a `last_inst_raw: u32` field to `RVCore` set during `execute()`, and expose `fn funct3(&self) -> u8`.

```rust
// In RVCore:
last_inst_raw: u32,

fn execute(&mut self, inst: DecodedInst, raw: u32) -> XResult {
    self.last_inst_raw = raw;
    // ...
}

fn funct3(&self) -> u8 {
    ((self.last_inst_raw >> 12) & 0x7) as u8
}
```

[**Phase 3: D Extension Instructions (26 ops) + Compressed D (4 ops)**]

**3.1 D instruction patterns** (`riscv.instpat`):

```
// D extension — Load/Store
INSTPAT("??????? ????? ????? 011 ????? 00001 11", fld        , I);
INSTPAT("??????? ????? ????? 011 ????? 01001 11", fsd        , S);

// D extension — R-type (opcode 1010011, fmt=01)
INSTPAT("0000001 ????? ????? ??? ????? 10100 11", fadd_d     , R);
INSTPAT("0000101 ????? ????? ??? ????? 10100 11", fsub_d     , R);
INSTPAT("0001001 ????? ????? ??? ????? 10100 11", fmul_d     , R);
INSTPAT("0001101 ????? ????? ??? ????? 10100 11", fdiv_d     , R);
INSTPAT("0101101 00000 ????? ??? ????? 10100 11", fsqrt_d    , R);
INSTPAT("0010001 ????? ????? 000 ????? 10100 11", fsgnj_d    , R);
INSTPAT("0010001 ????? ????? 001 ????? 10100 11", fsgnjn_d   , R);
INSTPAT("0010001 ????? ????? 010 ????? 10100 11", fsgnjx_d   , R);
INSTPAT("0010101 ????? ????? 000 ????? 10100 11", fmin_d     , R);
INSTPAT("0010101 ????? ????? 001 ????? 10100 11", fmax_d     , R);
INSTPAT("0100000 00001 ????? ??? ????? 10100 11", fcvt_s_d   , R);
INSTPAT("0100001 00000 ????? ??? ????? 10100 11", fcvt_d_s   , R);
INSTPAT("1010001 ????? ????? 010 ????? 10100 11", feq_d      , R);
INSTPAT("1010001 ????? ????? 001 ????? 10100 11", flt_d      , R);
INSTPAT("1010001 ????? ????? 000 ????? 10100 11", fle_d      , R);
INSTPAT("1110001 00000 ????? 001 ????? 10100 11", fclass_d   , R);
INSTPAT("1100001 00000 ????? ??? ????? 10100 11", fcvt_w_d   , R);
INSTPAT("1100001 00001 ????? ??? ????? 10100 11", fcvt_wu_d  , R);
INSTPAT("1101001 00000 ????? ??? ????? 10100 11", fcvt_d_w   , R);
INSTPAT("1101001 00001 ????? ??? ????? 10100 11", fcvt_d_wu  , R);

// D extension — RV64 only
INSTPAT("1100001 00010 ????? ??? ????? 10100 11", fcvt_l_d   , R);
INSTPAT("1100001 00011 ????? ??? ????? 10100 11", fcvt_lu_d  , R);
INSTPAT("1110001 00000 ????? 000 ????? 10100 11", fmv_x_d    , R);
INSTPAT("1101001 00010 ????? ??? ????? 10100 11", fcvt_d_l   , R);
INSTPAT("1101001 00011 ????? ??? ????? 10100 11", fcvt_d_lu  , R);
INSTPAT("1111001 00000 ????? 000 ????? 10100 11", fmv_d_x    , R);

// D extension — R4-type FMA (fmt=01)
INSTPAT("????? 01 ????? ????? ??? ????? 10000 11", fmadd_d   , R4);
INSTPAT("????? 01 ????? ????? ??? ????? 10001 11", fmsub_d   , R4);
INSTPAT("????? 01 ????? ????? ??? ????? 10010 11", fnmsub_d  , R4);
INSTPAT("????? 01 ????? ????? ??? ????? 10011 11", fnmadd_d  , R4);
```

**3.2 D handlers**: Mirror F handlers using `read_f64`/`write_f64` and `F64` type. No NaN-boxing on D writes (full 64-bit).

```rust
pub(super) fn fadd_d(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg) -> XResult {
    self.require_fp()?;
    let rm = self.resolve_rm(self.funct3())?;
    let (a, b) = (self.read_f64(rs1), self.read_f64(rs2));
    let result = self.with_flags(rm, |rm| a.add(b, rm));
    self.write_f64(rd, result);
    self.dirty_fp();
    Ok(())
}

// D precision-conversion:
pub(super) fn fcvt_s_d(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg) -> XResult {
    self.require_fp()?;
    let rm = self.resolve_rm(self.funct3())?;
    let a = self.read_f64(rs1);
    let result = self.with_flags(rm, |rm| a.to_f32(rm));  // narrow D->S
    self.write_f32(rd, result);  // NaN-boxed
    self.dirty_fp();
    Ok(())
}

pub(super) fn fcvt_d_s(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg) -> XResult {
    self.require_fp()?;
    let rm = self.resolve_rm(self.funct3())?;
    let a = self.read_f32(rs1);  // NaN-boxing check
    let result = self.with_flags(rm, |rm| a.to_f64(rm));  // widen S->D
    self.write_f64(rd, result);
    self.dirty_fp();
    Ok(())
}

// RV64 move (raw bits, no conversion):
pub(super) fn fmv_x_d(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg) -> XResult {
    self.require_fp()?;
    self.set_gpr(rd, self.fpr[rs1] as Word)
}

pub(super) fn fmv_d_x(&mut self, rd: RVReg, rs1: RVReg, _rs2: RVReg) -> XResult {
    self.require_fp()?;
    self.fpr[rd] = self.gpr[rs1] as u64;
    self.dirty_fp();
    Ok(())
}
```

**3.3 Compressed D instructions** (`riscv.instpat` + `inst/compressed.rs`):

Patterns:
```
INSTPAT("001 ? ????? ????? 00",   c_fld      , CL);
INSTPAT("101 ? ????? ????? 00",   c_fsd      , CS);
INSTPAT("001 ? ????? ????? 10",   c_fldsp    , CI);
INSTPAT("101 ? ????? ????? 10",   c_fsdsp    , CSS);
```

Handlers in `inst/compressed.rs`:
```rust
pub(super) fn c_fld(&mut self, inst: u32) -> XResult {
    self.require_fp()?;
    let rd = reg_prime(inst, 4, 2)?;
    let rs1 = reg_prime(inst, 9, 7)?;
    // Same immediate encoding as C.LD: bits[12:10]<<3 | bits[6:5]<<6
    let imm = (bits(inst, 12, 10) << 3) | (bits(inst, 6, 5) << 6);
    let addr = self.eff_addr(rs1, imm as SWord);
    let val = self.load(addr, 8)?;
    self.fpr[rd] = val as u64;
    self.dirty_fp();
    Ok(())
}

pub(super) fn c_fsd(&mut self, inst: u32) -> XResult {
    self.require_fp()?;
    let rs2 = reg_prime(inst, 4, 2)?;
    let rs1 = reg_prime(inst, 9, 7)?;
    let imm = (bits(inst, 12, 10) << 3) | (bits(inst, 6, 5) << 6);
    let addr = self.eff_addr(rs1, imm as SWord);
    self.store(addr, 8, self.fpr[rs2] as Word)?;
    Ok(())
}

pub(super) fn c_fldsp(&mut self, inst: u32) -> XResult {
    self.require_fp()?;
    let rd = reg(inst, 11, 7)?;
    // Same immediate encoding as C.LDSP: bits[12]<<5 | bits[6:5]<<3 | bits[4:2]<<6
    let imm = (bits(inst, 12, 12) << 5) | (bits(inst, 6, 5) << 3) | (bits(inst, 4, 2) << 6);
    let addr = self.eff_addr(RVReg::sp, imm as SWord);
    let val = self.load(addr, 8)?;
    self.fpr[rd] = val as u64;
    self.dirty_fp();
    Ok(())
}

pub(super) fn c_fsdsp(&mut self, inst: u32) -> XResult {
    self.require_fp()?;
    let rs2 = reg(inst, 6, 2)?;
    // Same immediate encoding as C.SDSP: bits[12:10]<<3 | bits[9:7]<<6
    let imm = (bits(inst, 12, 10) << 3) | (bits(inst, 9, 7) << 6);
    let addr = self.eff_addr(RVReg::sp, imm as SWord);
    self.store(addr, 8, self.fpr[rs2] as Word)?;
    Ok(())
}
```

Add to `rv_inst_table!`:
```rust
(C, (inst), [/* existing ... */, c_fld, c_fsd, c_fldsp, c_fsdsp]),
```

[**Phase 4: Integration**]

1. **DTS update**: `riscv,isa = "rv64imafdcsu_sstc"`
2. **`mstatus.FS` side-effects in `csr/ops.rs`**: update SD bit whenever FS field changes
3. **`CoreContext` update**: Add `fprs: Vec<(&'static str, u64)>` field for debug register inspection. Difftest FP comparison deferred.
4. **Debug ISA string**: Update `debug.rs` from `rv64imac` to `rv64imafdc`
5. **`format_mnemonic`**: Add FP instruction disassembly support
6. **Buildroot initramfs**: Replace minimal `init.c` with busybox rootfs built with `lp64d` ABI
7. **Run full validation**: `make fmt`, `make clippy`, `make test`, `make run`, am-tests, benchmarks, Linux boot

---

## Trade-offs

- T-1: **Softfloat backend** — `softfloat-wrapper` with `riscv` feature selected. Confirmed capabilities: per-op `RoundingMode` parameter, `ExceptionFlags` with `to_bits()/from_bits()`, `from_bits(u32)/to_bits()` for raw transfers, `fused_mul_add` for FMA, RISC-V NaN canonicalization via compile-time feature. Fallback: `softfloat-sys` (raw FFI) if wrapper API proves insufficient.

- T-2: **FP register storage** — `[u64; 32]` (raw bits). Matches NaN-boxing semantics, `FMV.*` bit preservation, and load/store raw transfer. Confirmed by KXemu's union approach (equivalent semantics, different syntax).

- T-3: **R4-type variant** — `DecodedInst::R4` with `rd, rs1, rs2, rs3`. Clean separation from R-type. 8 FMA instructions (4 F + 4 D) use this format. Updates: `rv_inst_table!` gains R4 row, `build_dispatch` macro handles it, `InstFormat::R4` added.

---

## Validation

[**Unit Tests**]
- V-UT-1: NaN-boxing roundtrip: `write_f32(f1, 1.0)` -> `read_f32(f1)` == 1.0; raw write without NaN-boxing -> `read_f32` returns canonical NaN
- V-UT-2: Rounding mode resolution: rm=0..4 valid; rm=7 reads frm; frm=5/6/7 with rm=7 -> IllegalInstruction
- V-UT-3: `fflags` sticky accumulation: add(qNaN, 1.0) sets NV; div(1.0, 0.0) sets DZ; both remain set
- V-UT-4: `fcsr` composite: write `fcsr=0xE5` -> `fflags=0x05`, `frm=0x7`; write `fflags=0x1F` -> `fcsr[4:0]=0x1F`; write `frm=0x3` -> `fcsr[7:5]=0x3`
- V-UT-5: FS gating: FS=Off -> `fadd.s` raises IllegalInstruction; FS=Initial -> executes, sets FS=Dirty
- V-UT-6: `mstatus.SD` = 1 when FS=Dirty; SD = 0 when FS reset to Clean/Initial
- V-UT-7: Decoder: all 56 FP instruction patterns (26 F + 26 D + 4 compressed) decode correctly including R4 rs3 extraction
- V-UT-8: `fclass` returns correct bit for all 10 categories (sNaN, qNaN, ±inf, ±normal, ±subnormal, ±zero)

[**Integration Tests**]
- V-IT-1: F arithmetic: `fadd.s`/`fsub.s`/`fmul.s`/`fdiv.s`/`fsqrt.s` with all 5 rounding modes
- V-IT-2: D arithmetic: `fadd.d`/`fsub.d`/`fmul.d`/`fdiv.d`/`fsqrt.d` with all 5 rounding modes
- V-IT-3: FMA: `fmadd.s(2.0, 3.0, 4.0) == 10.0`; `fmadd.d` likewise; verify single-rounding (not mul+add)
- V-IT-4: Load/store: `flw`/`fsw` roundtrip; `fld`/`fsd` roundtrip; `flw` NaN-boxes on load
- V-IT-5: Convert: `fcvt.d.s(1.5f32)` -> `1.5f64`; `fcvt.s.d(1.5f64)` -> `1.5f32` (NaN-boxed)
- V-IT-6: Compressed: `c_fld`/`c_fsd` roundtrip via stack; `c_fldsp`/`c_fsdsp` roundtrip
- V-IT-7: All existing tests unaffected: 278 unit tests, 31 cpu-tests, am-tests, benchmarks

[**Failure / Robustness Validation**]
- V-F-1: FS=Off trapping path: kernel thread with FS=Off executes FP -> trap -> OS sets FS=Initial -> retry succeeds
- V-F-2: Arithmetic NaN: `fadd.s(sNaN, 1.0)` -> canonical NaN + NV; `fadd.s(qNaN, 1.0)` -> canonical NaN, NV NOT set
- V-F-3: Division: `fdiv.s(1.0, 0.0)` -> +inf + DZ; `fdiv.s(0.0, 0.0)` -> canonical NaN + NV
- V-F-4: Overflow: `fmul.s(FLT_MAX, 2.0)` -> +inf + OF + NX
- V-F-5: FCVT invalid-input (per C-7 table):
  - `fcvt.w.s(+inf)` -> 2^31-1 + NV
  - `fcvt.w.s(-inf)` -> -2^31 + NV
  - `fcvt.w.s(NaN)` -> 2^31-1 + NV
  - `fcvt.wu.s(-1.0)` -> 0 + NV
  - `fcvt.wu.s(NaN)` -> 2^32-1 + NV

[**Edge Case Validation (NaN-sensitive)**]
- V-E-1: Signed zero: `fmin.s(-0.0, +0.0)` -> `-0.0`; `fmax.s(-0.0, +0.0)` -> `+0.0`
- V-E-2: `fmin.s(qNaN, 1.0)` -> `1.0` + NV flag (one-NaN rule); `fmin.s(sNaN, 1.0)` -> `1.0` + NV flag
- V-E-3: `fmin.s(qNaN, qNaN)` -> canonical NaN + NV flag (two-NaN rule)
- V-E-4: `feq.s(qNaN, qNaN)` -> 0, NV NOT set; `feq.s(sNaN, 1.0)` -> 0, NV SET
- V-E-5: `flt.s(qNaN, 1.0)` -> 0, NV SET (FLT/FLE signal on any NaN, unlike FEQ)
- V-E-6: `fle.s(sNaN, sNaN)` -> 0, NV SET
- V-E-7: NaN-boxing edge: `fmv.d.x` writes arbitrary bits, then `fadd.s` reads as canonical NaN
- V-E-8: `fmv.x.w` on RV64: sign-extends 32-bit value to 64-bit integer register
- V-E-9: `fsgnj.s(sNaN, x)` preserves sNaN payload (no canonicalization)
- V-E-10: FMA: `fmadd.s(+inf, 0.0, qNaN)` -> canonical NaN + NV flag (inf*0 is invalid even with qNaN addend)
- V-E-11: `fcvt.w.s(+0.5, RTZ)` -> 0 (truncation); `fcvt.w.s(+0.5, RUP)` -> 1 (round up)

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (F ext) | V-IT-1, V-IT-3, V-IT-4, V-IT-5, V-UT-7 |
| G-2 (D ext) | V-IT-2, V-IT-3, V-IT-4, V-IT-5, V-UT-7 |
| G-3 (Compressed D) | V-IT-6, V-UT-7 |
| G-4 (IEEE 754) | V-UT-1..3, V-F-2..5, V-E-1..6, V-E-10..11 |
| G-5 (NaN-boxing) | V-UT-1, V-E-7, V-E-9 |
| G-6 (mstatus.FS) | V-UT-5, V-UT-6, V-F-1 |
| G-7 (misa) | V-IT-7 (misa test updated) |
| G-8 (Linux boot) | V-IT-7 (full boot validation) |
| C-2 (R4-type) | V-UT-7, V-IT-3 |
| C-6 (fcsr composite) | V-UT-3, V-UT-4 |
| C-7 (FCVT table) | V-F-5 |
| C-8 (SD bit) | V-UT-6 |
