# `F/D Floating-Point Extension` REVIEW `03`

> Status: Open
> Feature: `float`
> Iteration: `03`
> Owner: Reviewer
> Target Plan: `03_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Rejected
- Blocking Issues: `1`
- Non-Blocking Issues: `2`

## Summary

Round `03` is materially better than `02`. It fixes the previously blocking FP-CSR dirty-state hole, removes the contradictory `DecodedInst` sketches, and strengthens the validation story around the architectural CSR path. The direction on `FR` / `FR4` is also cleaner than pushing an unused `rm` parameter through every integer `R` handler.

This round is still not ready for implementation. The new macro abstraction for arithmetic and compare handlers is not implementable as written in Rust, because it calls `with_flags(&mut self, ...)` while the closure re-borrows `self` to read operands. In addition, the proposed `fflags` / `frm` debug-name fix still does not return the architecturally correct values on the current debugger path, and the new CSR validation still omits the immediate CSR instruction family.

## Findings

### R-001 `The new fp_binop/fp_cmp abstraction cannot compile as written`

- Severity: HIGH
- Section: `API Surface / Phase 2`
- Type: Correctness
- Problem:
  The proposed macro abstraction calls `self.with_flags(...)` and then reads operands from `self` inside the closure, for example `self.with_flags(rm, |rm| self.read_f32(rs1).add(self.read_f32(rs2), rm))` and the analogous compare form. In the current design, `with_flags` takes `&mut self`, so the closure cannot also capture `&self` to call `read_f32()` / `read_f64()` during the same call. That is an immediate borrow-check conflict in Rust, and it appears in the core handler-generation pattern for both arithmetic and compare instructions.
- Why it matters:
  This is now the main execution-path abstraction for the feature. If approved as written, the executor either hits a compile wall immediately or has to deviate from the approved plan in the most central part of the implementation.
- Recommendation:
  The next PLAN must restructure the abstraction so operands are read before calling `with_flags()`, or otherwise make the callback pure with respect to `self`. For example: read `a`/`b` into locals first, then call `with_flags(rm, |rm| a.add(b, rm))`.

### R-002 `The claimed frm/fflags debug-name fix still reads the wrong storage`

- Severity: MEDIUM
- Section: `API Surface / Constraints / Phase 4`
- Type: API
- Problem:
  The plan keeps `fflags` and `frm` in `csr_table!` “for name resolution only”, but the current debugger register-read path does not call `csr_read()` or `fp_csr_read()`. It resolves the name through `CsrAddr::from_name()`, then calls `find_desc()` and `read_with_desc()`. With the proposed descriptors, that path reads raw slots `0x001` / `0x002`, not computed views of the canonical `fcsr` slot. Since the actual architectural reads and writes are intercepted before the descriptor path, those raw slots will not track the true FP CSR state.
- Why it matters:
  This means R-003 is not actually resolved. Debug register inspection would show stale or zero values for `fflags` / `frm`, exactly when the plan claims those names are preserved.
- Recommendation:
  The next PLAN should preserve debug visibility with an explicit hook, not a fake storage descriptor. The clean fix is to route debugger reads of `fflags` / `frm` through `fp_csr_read()`, or add a dedicated name-resolution path for FP CSR views.

### R-003 `End-to-end CSR validation still omits the immediate CSR instruction family`

- Severity: MEDIUM
- Section: `Validation`
- Type: Validation
- Problem:
  The new `V-CSR-*` coverage is an improvement, but it only covers `csrrw`, `csrrs`, and `csrrc` plus `x0` suppression cases. It still omits `csrrwi`, `csrrsi`, and `csrrci`, even though the current codebase has separate handlers for those instructions and the prior review recommendation explicitly called out all six CSR instruction forms.
- Why it matters:
  The immediate forms are architecturally distinct entry points into the same FP CSR logic, with their own zero-immediate no-write cases. Leaving them untested weakens the acceptance story around exactly the path that caused the previous blocking issue.
- Recommendation:
  Add explicit end-to-end tests for `csrrwi`, `csrrsi`, and `csrrci` on `fflags`, `frm`, and `fcsr`, including `uimm=0` no-write behavior and `FS` / `SD` state expectations.

## Trade-off Advice

### TR-1 `Keep FR/FR4, but shape the helper API around Rust borrowing rules`

- Related Plan Item: `T-3`
- Topic: Clean Design vs Implementability
- Reviewer Position: Keep as is with redesign of the helper call shape
- Advice:
  The `FR` / `FR4` split is still a reasonable trade-off. The problem is the proposed handler shape, not the decoded-format choice.
- Rationale:
  Separate FP decoded forms avoid contaminating every integer `R` handler with an unused argument. That part is sound. What needs to change is how FP helpers interact with `with_flags()`.
- Required Action:
  Keep `FR` / `FR4` if desired, but revise the macro patterns so operand reads happen before the mutable-borrowing helper call.

### TR-2 `Prefer explicit debug hooks over descriptor entries that are not semantically real`

- Related Plan Item: `C-6`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer explicit tooling hook
- Advice:
  A single canonical `fcsr` slot remains the right storage model, but the debugger compatibility layer should be explicit rather than pretending `fflags` / `frm` have real backing slots.
- Rationale:
  Fake descriptors make the API surface look cleaner on paper while returning the wrong value on the current code path. An explicit hook is simpler and more honest.
- Required Action:
  Keep the canonical `fcsr` storage model, but change the debugger/name-resolution strategy for `fflags` / `frm`.

## Positive Notes

- The round genuinely fixes the prior blocking issue by making FP CSR writes transition `FS` to Dirty in the architectural CSR path.
- Collapsing the plan to a single authoritative decoded-format design is a real improvement over round `02`.
- The added end-to-end CSR-path validation is directionally correct and materially better than helper-only coverage.

---

## Approval Conditions

### Must Fix
- R-001

### Should Improve
- R-002
- R-003

### Trade-off Responses Required
- TR-1
- TR-2

### Ready for Implementation
- No
- Reason: The central FP macro abstraction still cannot be implemented as written because the proposed `with_flags()` call pattern conflicts with Rust’s borrowing rules.
