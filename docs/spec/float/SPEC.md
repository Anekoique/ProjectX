# `float` SPEC

> Source: [`/docs/archived/feat/float/03_PLAN.md`](/docs/archived/feat/float/03_PLAN.md).
> Iteration history, trade-off analysis, and implementation
> plan live under `docs/archived/feat/float/`.

---


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
