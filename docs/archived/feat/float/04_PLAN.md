# F/D Floating-Point Extension PLAN 04

> Status: Approved for Implementation
> Feature: `float`
> Iteration: `04`
> Owner: Executor
> Depends on:
> - Previous Plan: `03_PLAN.md`
> - Review: `03_REVIEW.md`
> - Master Directive: `03_MASTER.md`

---

## Summary

Final iteration — approved for implementation. Fixes the borrow-check issue in FP macros (R-001: read operands before `with_flags`), routes debugger FP CSR reads through `fp_csr_read` (R-002), adds `csrrwi`/`csrrsi`/`csrrci` validation (R-003), and adds AM-tests for bare-metal FP validation (M-001). All other design decisions carry forward from 03_PLAN unchanged.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Macros read operands into locals before `with_flags()` call |
| Review | R-002 | Accepted | Debugger register-read path routes FP CSRs through `fp_csr_read()` |
| Review | R-003 | Accepted | `csrrwi`/`csrrsi`/`csrrci` tests added (V-CSR-7..9) |
| Review | TR-1 | Accepted | `FR`/`FR4` kept; helper call shape fixed |
| Review | TR-2 | Accepted | `fflags`/`frm` kept in `csr_table!` for `CsrAddr` enum; debug read routed through `fp_csr_read` |
| Master | M-001 | Applied | AM-test for bare-metal FP validation added |

---

## Changes from 03_PLAN

**R-001 fix** — macros read operands before mutable borrow:

```rust
macro_rules! fp_binop {
    ($name_s:ident, $name_d:ident, $op:ident) => {
        pub(super) fn $name_s(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rm: u8) -> XResult {
            self.require_fp()?;
            let rm = self.resolve_rm(rm)?;
            let (a, b) = (self.read_f32(rs1), self.read_f32(rs2));  // read BEFORE with_flags
            let r = self.with_flags(rm, |rm| a.$op(b, rm));
            self.write_f32(rd, r);
            Ok(())
        }
        pub(super) fn $name_d(&mut self, rd: RVReg, rs1: RVReg, rs2: RVReg, rm: u8) -> XResult {
            self.require_fp()?;
            let rm = self.resolve_rm(rm)?;
            let (a, b) = (self.read_f64(rs1), self.read_f64(rs2));
            let r = self.with_flags(rm, |rm| a.$op(b, rm));
            self.write_f64(rd, r);
            Ok(())
        }
    };
}
```

Same pattern for `fp_cmp!` (read into locals first) and `fp_fma!`.

**R-002 fix** — debugger register-read routes through `fp_csr_read`:

In the `DebugOps` implementation (or wherever the debugger reads CSRs by name), intercept FP CSR addresses:

```rust
// In context snapshot or debug register read:
let value = if RVCore::is_fp_csr(addr) {
    self.fp_csr_read(addr)
} else {
    self.csr.read_with_desc(desc)
};
```

**R-003 fix** — additional CSR validation:
- V-CSR-7: `csrrwi fflags, 0x1F` → fflags=0x1F, FS=Dirty
- V-CSR-8: `csrrsi frm, 0x04` → frm |= 4, FS=Dirty
- V-CSR-9: `csrrci fcsr, 0x00` (uimm=0) → no write, FS unchanged

**M-001** — AM-test at `xkernels/tests/am-tests/src/tests/float.c`:
- Basic F arithmetic: `fadd.s`, `fmul.s`, `fdiv.s` via inline asm
- Basic D arithmetic: `fadd.d`, `fmul.d`
- NaN-boxing: write via `fmv.d.x`, read via `fadd.s` → canonical NaN
- FCVT: `fcvt.w.s` of integer, roundtrip
- FCLASS: classify known values
- FS state: verify FS transitions to Dirty after FP op

All other spec, invariants, architecture, constraints, execution flow, and validation carry forward from 03_PLAN unchanged.
