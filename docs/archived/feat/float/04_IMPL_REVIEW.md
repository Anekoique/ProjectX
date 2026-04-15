# `F/D Floating-Point Extension` IMPL REVIEW `04`

> Status: Open
> Feature: `float`
> Iteration: `04`
> Owner: Reviewer
> Target Impl: `04_IMPL.md`

---

## Verdict

- Decision: Rejected
- Blocking Issues: `2`
- Non-Blocking Issues: `1`

## Summary

The implementation is substantial and materially closer to merge than the plan rounds were. Decoder support, FP CSR interception, the new float handler module, and the debugger CSR-view fix are all present in code, and `cargo test -p xcore` currently passes locally.

It is still not ready for merge. Two architectural correctness issues remain in the floating-point state and `FMIN/FMAX` behavior:

- `mstatus.SD` is set when FP state becomes dirty, but it is never recomputed when software later writes `mstatus` / `sstatus` to clear or downgrade `FS`, so `SD` can remain permanently stale.
- the `FMIN/FMAX` implementation does not implement the required signed-zero tie-break, so operand order can change the result for `+0.0` / `-0.0`.

There is also a validation / implementation-record gap: the implementation note claims end-to-end FP CSR instruction coverage for the immediate CSR forms, but the actual test additions do not include those paths.

---

## Findings

### IR-001 `mstatus.SD becomes stale after software clears FS`

- Severity: HIGH
- Section: `Implementation Scope / Behavior`
- Type: Correctness
- Problem:
  `dirty_fp()` sets both `FS=Dirty` and `SD=1`, but the generic `mstatus` / `sstatus` write path never recomputes `SD` from the new `FS` state. `mstatus` writes cannot touch `SD` directly because `MSTATUS_WMASK` excludes it, and `csr_write_side_effects()` only updates MMU state. As a result, once any FP instruction sets `SD`, software can write `FS` back to `Initial` or `Clean` and `SD` will still remain set.
- Why it matters:
  That violates the architectural contract of `SD` as the dirty-state summary bit. It also breaks the approved plan’s stated `FS` / `SD` tracking model and can mislead OS save/restore code that relies on `SD` reflecting extension dirtiness.
- Recommendation:
  Recompute `SD` whenever `mstatus` or `sstatus` is written. The clean fix is in `csr_write_side_effects()`: after the descriptor write, rebuild `mstatus` so `SD` matches whether `FS` (or any future tracked extension state) is dirty.

### IR-002 `FMIN/FMAX mishandle the signed-zero tie-break`

- Severity: HIGH
- Section: `Implementation Scope / Acceptance Mapping`
- Type: Correctness
- Problem:
  `fminmax()` chooses between the two non-NaN operands purely from the quiet comparison result returned by `pick_a`, and the concrete `fmin_*` / `fmax_*` handlers pass `a.le_quiet(b)` and `b.le_quiet(a)` respectively. For `+0.0` and `-0.0`, both comparisons are true, so the result depends on operand order instead of following the spec rule that `FMIN` must return `-0.0` and `FMAX` must return `+0.0`.
- Why it matters:
  This is a direct ISA-visible result bug. For example, `fmax.s(-0.0, +0.0)` returns `-0.0` here, which is not compliant with the RISC-V F specification.
- Recommendation:
  Add an explicit signed-zero tie-break before the generic `pick_a` selection, for both single and double precision. Also add tests covering both operand orders for `fmin` and `fmax` with `+0.0` and `-0.0`.

### IR-003 `The implementation note claims FP CSR immediate-form coverage that is not present in code`

- Severity: MEDIUM
- Section: `Verification Results / Plan Compliance`
- Type: Validation
- Problem:
  `04_IMPL.md` claims that the round added validation for the immediate CSR forms (`csrrwi` / `csrrsi` / `csrrci`) on FP CSRs, but the actual code changes do not include any new FP-CSR-specific `zicsr` tests. The existing `zicsr.rs` tests still cover only `mscratch` and generic CSR semantics, while the float tests exercise helper views such as `fp_csr_read()` rather than end-to-end `csrr*i` execution on `fflags` / `frm` / `fcsr`.
- Why it matters:
  This leaves the previous review target only partially validated and makes the implementation record inaccurate. Implementation review should be able to trust `04_IMPL.md` as a truthful supplement to the code.
- Recommendation:
  Add explicit `csrrwi` / `csrrsi` / `csrrci` tests that target FP CSR addresses and verify both CSR values and `FS` / `SD` behavior, then update `04_IMPL.md` to match the real verification scope and test counts.

---

## Positive Notes

- The dedicated FP CSR read/write path is implemented cleanly, and debugger reads for `fflags` / `frm` now route through the architectural view instead of stale raw storage.
- The `FR` / `FR4` decode split is in place and integrates with the existing dispatch macro framework without contaminating integer `R` handlers.
- `cargo test -p xcore` passes locally, which gives the implementation a solid baseline despite the remaining architectural bugs.

---

## Approval Conditions

### Must Fix
- IR-001
- IR-002

### Should Improve
- IR-003

### Ready for Merge / Release
- No
- Reason: The FP state summary bit can become architecturally stale, and `FMIN/FMAX` still return the wrong signed zero in valid instruction cases.
