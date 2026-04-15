# F/D Floating-Point Extension PLAN 00

> Status: Draft
> Feature: `float`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

Implement RISC-V F (single-precision) and D (double-precision) floating-point extensions to enable standard Linux userspace (busybox/buildroot with `lp64d` ABI). This adds 32 floating-point registers (`f0`-`f31`), 3 FP CSRs (`fcsr`, `fflags`, `frm`), ~52 new instruction patterns, `mstatus.FS` state tracking, and NaN-boxing semantics. IEEE 754 arithmetic is delegated to the `softfloat-wrapper` crate (Berkeley softfloat via FFI) for strict compliance.

---

## Spec

[**Goals**]
- G-1: Implement all RV64F instructions (26 ops: arithmetic, load/store, convert, compare, classify, sign-inject, move)
- G-2: Implement all RV64D instructions (26 ops: same categories for double-precision)
- G-3: Strict IEEE 754-2008 compliance via Berkeley softfloat (rounding modes, exception flags, NaN canonicalization)
- G-4: NaN-boxing: single-precision values in 64-bit f-registers must be NaN-boxed (upper 32 bits = 1); invalid NaN-boxing reads produce canonical NaN
- G-5: `mstatus.FS` state machine (Off/Initial/Clean/Dirty) with illegal-instruction trap when FS=Off
- G-6: `misa` bits F(5) and D(3) advertised; D implies F
- G-7: Boot Linux with buildroot/busybox `lp64d` initramfs to interactive shell

- NG-1: No Zfh (half-precision) or Q (quad-precision) extensions
- NG-2: No FP exception trapping (RISC-V base spec does not support this)
- NG-3: No performance optimization of FP pipeline (correctness first)

[**Architecture**]

```
                    RVCore
                    +--------------------------+
                    | gpr: [Word; 32]          |  (existing)
                    | fpr: [u64; 32]           |  (NEW: f0-f31, 64-bit storage)
                    | pc, npc, csr, bus, ...   |
                    +--------------------------+
                           |
                    +------+------+
                    |             |
               inst/float.rs   csr.rs
               (F/D handlers)  (fcsr/fflags/frm + FS tracking)
                    |
               softfloat-wrapper  (IEEE 754 backend)
```

FP instruction flow:
1. Decoder matches pattern in `riscv.instpat` -> `DecodedInst::R` / `I` / `S` with FP `InstKind`
2. Dispatch routes to `inst/float.rs` handler on `RVCore`
3. Handler checks `mstatus.FS != Off` (else raise IllegalInstruction)
4. Handler reads `fpr[]`, resolves rounding mode, invokes softfloat op
5. Handler writes result to `fpr[]` (NaN-boxed for single), accumulates `fflags`
6. Handler sets `mstatus.FS = Dirty`

[**Invariants**]
- I-1: `fpr[i]` always holds a 64-bit value. Single-precision values are stored NaN-boxed: `bits[63:32] = 0xFFFF_FFFF`, `bits[31:0]` = IEEE 754 single.
- I-2: When a single-precision instruction reads `fpr[i]` and upper 32 bits are not all-ones, the value is treated as canonical single NaN (`0x7FC00000`).
- I-3: All FP instructions raise `IllegalInstruction` if `mstatus.FS == Off` (bits `[14:13] == 0b00`).
- I-4: Every FP instruction that modifies FP state (f-regs, fcsr) sets `mstatus.FS = Dirty`.
- I-5: `fflags` are sticky (OR-accumulated). Software must explicitly clear them.
- I-6: `fcsr` is a composite view: `fcsr = (frm << 5) | fflags`. Writes to `fcsr` update both; writes to `fflags`/`frm` update their portion of `fcsr`.
- I-7: Reserved rounding modes (5, 6) in `frm` cause illegal-instruction when an instruction uses dynamic rounding (`rm=7`).
- I-8: `misa` is read-only in this implementation. F(5) and D(3) bits are always set when compiled with FP support.

[**Data Structure**]

```rust
// In RVCore (mod.rs) — add fpr field
pub struct RVCore {
    gpr: [Word; 32],
    fpr: [u64; 32],       // NEW: floating-point registers (64-bit for D)
    pc: VirtAddr,
    // ... existing fields unchanged
}

// FP register index type (reuse RVReg for f0-f31, same 5-bit encoding)
// No new type needed — FP register indices use the same 0..31 range as GPR.
```

```rust
// Rounding mode enum (inst/float.rs)
#[repr(u8)]
enum RoundingMode {
    RNE = 0, // Round to Nearest, ties to Even
    RTZ = 1, // Round towards Zero
    RDN = 2, // Round Down (-inf)
    RUP = 3, // Round Up (+inf)
    RMM = 4, // Round to Nearest, ties to Max Magnitude
}
```

```rust
// Exception flags (inst/float.rs)
const NV: u32 = 1 << 4;  // Invalid Operation
const DZ: u32 = 1 << 3;  // Divide by Zero
const OF: u32 = 1 << 2;  // Overflow
const UF: u32 = 1 << 1;  // Underflow
const NX: u32 = 1 << 0;  // Inexact
```

[**API Surface**]

```rust
// Core FP helpers on RVCore (inst/float.rs)
impl RVCore {
    /// Check FS != Off, else raise IllegalInstruction.
    fn require_fp(&self) -> XResult;

    /// Mark mstatus.FS = Dirty.
    fn dirty_fp(&mut self);

    /// Read single-precision from fpr with NaN-boxing check.
    /// Returns canonical NaN if not properly NaN-boxed.
    fn read_f32(&self, reg: RVReg) -> f32;

    /// Write single-precision to fpr with NaN-boxing.
    fn write_f32(&mut self, reg: RVReg, val: f32);

    /// Read double-precision from fpr (raw 64-bit).
    fn read_f64(&self, reg: RVReg) -> f64;

    /// Write double-precision to fpr (raw 64-bit).
    fn write_f64(&mut self, reg: RVReg, val: f64);

    /// Resolve rounding mode: if rm=7 (DYN), read frm CSR.
    /// Returns Err(IllegalInstruction) for reserved modes.
    fn resolve_rm(&self, rm: u8) -> XResult<RoundingMode>;

    /// Accumulate exception flags into fflags CSR.
    fn accrue_fflags(&mut self, flags: u32);
}
```

[**Constraints**]
- C-1: FP register encoding shares the same 5-bit field as GPR in the instruction encoding. The decoder extracts `rd`, `rs1`, `rs2` as `RVReg`; FP instructions reinterpret these as FP register indices.
- C-2: R4-type (fused multiply-add) requires a `rs3` field extracted from bits `[31:27]`. The current `DecodedInst::R` only has `rd`, `rs1`, `rs2`. A new variant `R4` is needed.
- C-3: The `rm` field (bits `[14:12]`) is overloaded: for arithmetic instructions it encodes rounding mode; for FSGNJ/FSGNJN/FSGNJX and FEQ/FLT/FLE it encodes the sub-operation. The handler must interpret `funct3` contextually.
- C-4: `softfloat-wrapper` provides IEEE 754 operations. Dependency added to `xcore/Cargo.toml`.
- C-5: Compressed FP loads/stores (C.FLW, C.FLD, C.FLWSP, C.FLDSP, etc.) exist but are **deferred** — they share opcode space with C.LW/C.LD on RV64 (only relevant on RV32). For RV64, these slots are already used by integer C extension instructions.
- C-6: `fcsr`/`fflags`/`frm` are composite CSRs sharing storage. Writes to one must reflect in the others. Implemented as a single `u32` storage slot with view masks.
- C-7: All FP-to-integer conversions that overflow or receive NaN produce a defined result (max/min int for overflow, 0 for unsigned NaN, max-negative for signed NaN) and set NV flag.
- C-8: `mstatus.SD` (bit 63 on RV64) is read-only summary of FS==Dirty. Must be updated as a side-effect of FS changes.

---

## Implement

### Execution Flow

[**Main Flow**]
1. Decoder matches FP instruction pattern -> `DecodedInst::R { kind: fadd_s, rd, rs1, rs2 }` (or `R4` for FMA)
2. `dispatch()` routes to `Self::fadd_s(self, rd, rs1, rs2)` in `inst/float.rs`
3. Handler calls `self.require_fp()?` — traps if FS=Off
4. Handler reads operands: `let a = self.read_f32(rs1); let b = self.read_f32(rs2);`
5. Handler resolves rounding mode: `let rm = self.resolve_rm(funct3)?;`
6. Handler invokes softfloat: `let (result, flags) = softfloat::f32_add(a, b, rm);`
7. Handler writes result: `self.write_f32(rd, result);`
8. Handler accumulates flags: `self.accrue_fflags(flags);`
9. Handler marks state: `self.dirty_fp();`

[**Failure Flow**]
1. `mstatus.FS == Off` -> `raise_trap(IllegalInstruction, raw_inst)` — FP disabled by OS
2. `rm == 7` (DYN) and `frm` contains reserved value (5/6/7) -> `raise_trap(IllegalInstruction, raw_inst)`
3. FP exception (NaN, overflow, etc.) -> set `fflags` bits; no trap (RISC-V spec: no FP trap support in base)

[**State Transition**]

- `FS: Off` — any FP instruction -> trap (no state change)
- `FS: Initial/Clean` — FP instruction writes f-reg/fcsr -> `FS: Dirty`
- `FS: Dirty` — stays Dirty until OS context-switches and resets to Clean/Initial
- `SD` bit = `(FS == Dirty)` — automatically updated

### Implementation Plan

[**Phase 1: Infrastructure**]
1. Add `fpr: [u64; 32]` to `RVCore`
2. Add `R4` variant to `DecodedInst` with `rd`, `rs1`, `rs2`, `rs3` fields
3. Add `R4` to `InstFormat` enum and decoder `from_raw()` extraction
4. Register FP CSRs in `csr_table!`: `fflags = 0x001`, `frm = 0x002`, `fcsr = 0x003`
   - `fcsr` is the single storage slot (8 bits: `frm[7:5] | fflags[4:0]`)
   - `fflags` and `frm` are shadow views into `fcsr`
5. Add `misa` bits: F(5) and D(3) to `MISA_VALUE`
6. Add FP helper methods: `require_fp`, `dirty_fp`, `read_f32`, `write_f32`, `read_f64`, `write_f64`, `resolve_rm`, `accrue_fflags`
7. Add `softfloat-wrapper` dependency to `xcore/Cargo.toml`

[**Phase 2: F Extension Instructions**]

Add 26 instruction patterns to `riscv.instpat` and handlers in `inst/float.rs`:

Load/Store (I/S format, reuse existing decoder):
- `flw` (I-type, opcode=0000111, funct3=010)
- `fsw` (S-type, opcode=0100111, funct3=010)

Arithmetic (R-type, opcode=1010011):
- `fadd_s`, `fsub_s`, `fmul_s`, `fdiv_s`, `fsqrt_s`

Sign-injection (R-type):
- `fsgnj_s`, `fsgnjn_s`, `fsgnjx_s`

Min/Max (R-type):
- `fmin_s`, `fmax_s`

Comparison (R-type -> writes GPR):
- `feq_s`, `flt_s`, `fle_s`

Classify (R-type -> writes GPR):
- `fclass_s`

Convert float<->int (R-type):
- `fcvt_w_s`, `fcvt_wu_s`, `fcvt_s_w`, `fcvt_s_wu` (RV32/64)
- `fcvt_l_s`, `fcvt_lu_s`, `fcvt_s_l`, `fcvt_s_lu` (RV64 only)

Move (R-type):
- `fmv_x_w` (FP->GPR, no conversion)
- `fmv_w_x` (GPR->FP, no conversion)

Fused Multiply-Add (R4-type):
- `fmadd_s`, `fmsub_s`, `fnmsub_s`, `fnmadd_s`

[**Phase 3: D Extension Instructions**]

Add 26 instruction patterns and handlers, mirroring F with `fmt=01`:

Load/Store:
- `fld` (I-type, funct3=011)
- `fsd` (S-type, funct3=011)

Arithmetic: `fadd_d`, `fsub_d`, `fmul_d`, `fdiv_d`, `fsqrt_d`
Sign-injection: `fsgnj_d`, `fsgnjn_d`, `fsgnjx_d`
Min/Max: `fmin_d`, `fmax_d`
Comparison: `feq_d`, `flt_d`, `fle_d`
Classify: `fclass_d`
Convert float<->int: `fcvt_w_d`, `fcvt_wu_d`, `fcvt_d_w`, `fcvt_d_wu`, `fcvt_l_d`, `fcvt_lu_d`, `fcvt_d_l`, `fcvt_d_lu` (RV64)
Move (RV64): `fmv_x_d`, `fmv_d_x`
Convert between precisions: `fcvt_s_d`, `fcvt_d_s`
FMA: `fmadd_d`, `fmsub_d`, `fnmsub_d`, `fnmadd_d`

[**Phase 4: Integration**]
1. Update `mstatus.FS` side-effects in `csr/ops.rs` — SD bit computation
2. Update DTS: `riscv,isa = "rv64imafdcsu_sstc"`
3. Update `mstatus.FS` initialization to `Initial` (01) in `CsrFile::default()`
4. Add `fpr` to `CoreContext` for difftest comparison
5. Replace minimal `init.c` with buildroot/busybox `lp64d` initramfs
6. Run full validation: am-tests, cpu-tests, benchmarks, Linux boot

---

## Trade-offs

- T-1: **Softfloat backend: `softfloat-wrapper` crate vs native Rust `softfloat-sys` vs hand-rolled**
  - Option A: `softfloat-wrapper` — Rust wrapper around Berkeley softfloat-3. Battle-tested, IEEE 754 compliant, used by Spike/QEMU references. Slight FFI overhead.
  - Option B: Native Rust `softfloat-sys` — raw FFI to C softfloat. Less ergonomic but zero wrapper overhead.
  - Option C: Use Rust `f32`/`f64` native ops — fast but not guaranteed IEEE 754 on all platforms (e.g., flush-to-zero, non-standard NaN).
  - **Recommendation**: Option A. Correctness is paramount for OS boot; FFI overhead is negligible in an emulator that already does bus dispatch per instruction.

- T-2: **FP register storage: `[u64; 32]` vs `[f64; 32]` vs newtype**
  - Option A: `[u64; 32]` — raw bit storage. NaN-boxing is explicit bit manipulation. No implicit float semantics.
  - Option B: `[f64; 32]` — natural for D extension. Single-precision requires transmute for NaN-boxing.
  - Option C: Newtype `FpReg(u64)` — type safety but verbose.
  - **Recommendation**: Option A (`[u64; 32]`). FP registers are fundamentally bit containers for NaN-boxing. `f64` semantics would be misleading for single-precision values. Conversion happens only at the softfloat boundary.

- T-3: **R4-type variant: new `DecodedInst::R4` vs encode rs3 in existing R-type**
  - Option A: Add `DecodedInst::R4 { kind, rd, rs1, rs2, rs3 }` — clean, explicit, matches spec's R4-type.
  - Option B: Overload `DecodedInst::R` and extract `rs3` from `rs2` field at dispatch time — saves variant but obscures semantics.
  - **Recommendation**: Option A. Four FMA instructions justify a dedicated variant. The dispatch macro already handles multiple formats cleanly.

---

## Validation

[**Unit Tests**]
- V-UT-1: NaN-boxing: write single via `write_f32`, read back via `read_f32` — roundtrip preserves value; read from un-NaN-boxed register returns canonical NaN
- V-UT-2: Rounding mode resolution: `rm=0..4` valid, `rm=7` reads `frm`, `frm=5/6/7` with `rm=7` -> error
- V-UT-3: `fflags` accumulation: multiple ops OR flags together, explicit clear resets
- V-UT-4: `fcsr` composite: write `fcsr` -> read `fflags`/`frm` reflects; write `fflags` -> `fcsr` updated
- V-UT-5: `mstatus.FS` gating: FS=Off -> FP instruction raises IllegalInstruction; FS=Initial -> executes and sets Dirty
- V-UT-6: `mstatus.SD` tracks FS=Dirty (set when FS becomes Dirty, cleared when FS reset)
- V-UT-7: Decoder correctly decodes all 52 FP instruction patterns including R4-type `rs3` extraction

[**Integration Tests**]
- V-IT-1: F arithmetic: `fadd.s`, `fmul.s`, `fdiv.s`, `fsqrt.s` produce correct IEEE 754 results with all rounding modes
- V-IT-2: D arithmetic: `fadd.d`, `fmul.d`, `fdiv.d`, `fsqrt.d` produce correct results
- V-IT-3: FMA: `fmadd.s`/`fmadd.d` produce correctly fused result (not separate mul+add)
- V-IT-4: Load/store: `flw`/`fsw`/`fld`/`fsd` read/write correct memory values with proper NaN-boxing on `flw`
- V-IT-5: Convert: `fcvt.w.s`, `fcvt.d.s`, `fcvt.s.d` chain produces correct results
- V-IT-6: Existing tests pass: all 278 unit tests, 31 cpu-tests, am-tests, benchmarks unaffected

[**Failure / Robustness Validation**]
- V-F-1: FS=Off trapping: Linux sets FS=Off for kernel threads; first FP instruction in userspace triggers trap -> OS sets FS=Initial and retries
- V-F-2: NaN propagation: `fadd.s(NaN, 1.0)` -> canonical NaN + NV flag
- V-F-3: Division by zero: `fdiv.s(1.0, 0.0)` -> +inf + DZ flag
- V-F-4: Overflow: `fmul.s(FLT_MAX, 2.0)` -> +inf + OF + NX flags
- V-F-5: Float-to-int overflow: `fcvt.w.s(1e20)` -> INT_MAX + NV flag

[**Edge Case Validation**]
- V-E-1: Signed zero: `fmin.s(-0.0, +0.0)` -> `-0.0`; `fmax.s(-0.0, +0.0)` -> `+0.0`
- V-E-2: NaN-boxing edge: `fmv.d.x` writes arbitrary bits to fpr, then `fadd.s` reads as canonical NaN (not NaN-boxed)
- V-E-3: `fmv.x.w` on RV64: sign-extends 32-bit value to 64-bit integer register
- V-E-4: `fclass` on all 10 categories: -inf, -normal, -subnormal, -0, +0, +subnormal, +normal, +inf, sNaN, qNaN
- V-E-5: FMA with `inf * 0 + qNaN` -> canonical NaN + NV flag (not just qNaN passthrough)
- V-E-6: `fsgnj.s(sNaN, x)` preserves sNaN payload (sign-injection does not canonicalize)

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (F ext) | V-IT-1, V-IT-3, V-IT-4, V-IT-5 |
| G-2 (D ext) | V-IT-2, V-IT-3, V-IT-4, V-IT-5 |
| G-3 (IEEE 754) | V-UT-1..3, V-F-2..5, V-E-1, V-E-5 |
| G-4 (NaN-boxing) | V-UT-1, V-E-2, V-E-6 |
| G-5 (mstatus.FS) | V-UT-5, V-UT-6, V-F-1 |
| G-6 (misa) | V-IT-6 (misa test updated) |
| G-7 (Linux boot) | V-IT-6 (full boot validation) |
| C-2 (R4-type) | V-UT-7, V-IT-3 |
| C-6 (fcsr composite) | V-UT-3, V-UT-4 |
| C-7 (float-to-int overflow) | V-F-5 |
| C-8 (SD bit) | V-UT-6 |
