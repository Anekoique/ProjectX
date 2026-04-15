# `F/D Floating-Point Extension` REVIEW `02`

> Status: Open
> Feature: `float`
> Iteration: `02`
> Owner: Reviewer
> Target Plan: `02_PLAN.md`
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
- Non-Blocking Issues: `3`

## Summary

Round `02` is the first plan revision that is structurally close to implementable. It correctly abandons the broken alias-plus-side-effect CSR design from round `01`, keeps the right direction on explicit `rm` decoding, and fixes the earlier `FMIN/FMAX` qNaN expectations against the official RISC-V [F](https://docs.riscv.org/reference/isa/unpriv/f-st-ext.html), [D](https://docs.riscv.org/reference/isa/unpriv/d-st-ext.html), [C](https://docs.riscv.org/reference/isa/unpriv/c-st-ext.html), and privileged [machine-level](https://docs.riscv.org/reference/isa/priv/machine.html) specifications.

The round is still not ready for implementation. One blocking correctness issue remains in the proposed FP CSR write path: CSR instructions that write `fflags`, `frm`, or `fcsr` still do not transition `mstatus.FS` to Dirty. The rest of the findings are non-blocking, but they matter for making the plan authoritative and for avoiding tooling regressions during bring-up.

## Findings

### R-001 `FP CSR writes still leave FS clean on the architectural CSR instruction path`

- Severity: HIGH
- Section: `API Surface / Execution Flow / Validation`
- Type: Correctness
- Problem:
  The new specialized FP CSR path fixes the alias-model mismatch, but it still does not mark floating-point state dirty when software writes `0x001/0x002/0x003` through CSR instructions. In the proposed code, `fp_csr_write()` only merges bits into the canonical `fcsr` slot, and the specialized `csr_write()` branch returns immediately afterward. There is no accompanying `dirty_fp()` call on that path. Because `csrrw/csrrs/csrrc/csrrwi/csrrsi/csrrci` all route through `csr_write()`, legal writes to `fflags`, `frm`, or `fcsr` would mutate FP architectural state while leaving `mstatus.FS` in `Initial` or `Clean`.
- Why it matters:
  This is still a direct violation of the privileged-state model. The privileged spec states that instructions that possibly modify floating-point state, including configuration state such as `fcsr`, execute with `FS=Dirty` semantics. If this plan is approved as written, lazy FP save/restore and `SD` summary behavior will still be wrong for an architecturally visible class of instructions.
- Recommendation:
  The next PLAN must make FP CSR writes transition `FS` to Dirty on the specialized CSR path as well. The cleanest fix is to have `fp_csr_write()` or the FP branch in `csr_write()` call `dirty_fp()` before returning. The validation section must also add end-to-end `csrr*` coverage for `0x001/0x002/0x003`, not just helper-level field packing tests.

### R-002 `The DecodedInst rm design is still documented as multiple incompatible designs`

- Severity: MEDIUM
- Section: `Architecture / Data Structure / Trade-offs`
- Type: Maintainability
- Problem:
  The `DecodedInst` section still contains three incompatible designs in sequence: first no `rm` field, then `rm` via `last_inst_raw`, then `R4`-only `rm`, and finally the actual intended design with `rm` embedded in both `R` and `R4`. The later parts of the plan clearly rely on the last version, but the superseded alternatives are still present in the authoritative plan text.
- Why it matters:
  `DecodedInst` is a cross-cutting API that affects the decoder, dispatch macros, handlers, and disassembler. Leaving rejected alternatives inline makes the plan less authoritative than it needs to be and creates room for accidental partial implementations.
- Recommendation:
  Collapse this section to one approved data model only: `DecodedInst::R { ..., rm }` and `DecodedInst::R4 { ..., rs3, rm }`. Keep the rationale for rejecting hidden state in prose, but remove the obsolete sketches from the plan body.

### R-003 `Specializing FP CSR access currently drops frm/fflags name-level tooling visibility`

- Severity: MEDIUM
- Section: `Constraints / Phase 4`
- Type: API
- Problem:
  Constraint `C-6` removes `fflags` and `frm` from `csr_table!` and keeps only `fcsr` as a canonical slot. That is reasonable for storage, but the current debugger register-read path resolves CSR names via `CsrAddr::from_name()` and `find_desc()`. If the implementation follows the plan literally without an extra compatibility hook, `frm` and `fflags` will stop being readable by name in debug tooling.
- Why it matters:
  This is an avoidable tooling regression exactly when new FP state is being introduced. It makes ISA bring-up harder and creates an unnecessary mismatch between architected CSR names and debugger-visible register names.
- Recommendation:
  The next PLAN should explicitly preserve named access for `frm` and `fflags`. Either keep enum/name entries for the aliases while bypassing generic storage descriptors, or add a debugger/name-resolution fast path that maps those names onto the specialized FP CSR helpers.

### R-004 `Validation still misses end-to-end CSR-instruction coverage for FP CSRs`

- Severity: MEDIUM
- Section: `Validation / Acceptance Mapping`
- Type: Validation
- Problem:
  `V-UT-4` validates helper-level projection and packing of `fcsr`, `fflags`, and `frm`, but it does not test the actual ISA path through `csrr*` instructions. There is no explicit validation for read-modify-write semantics, zero-source no-write behavior, `rd=x0` no-read behavior, or FS dirtiness on the CSR instruction path.
- Why it matters:
  This round’s remaining blocking issue exists exactly on that path. Helper-only validation is not enough to certify the architecturally visible behavior of FP CSR access.
- Recommendation:
  Add dedicated `csrrw/csrrs/csrrc/csrrwi/csrrsi/csrrci` tests for `0x001`, `0x002`, and `0x003`, including masking semantics and `FS` / `SD` state transitions.

## Trade-off Advice

### TR-1 `Keep the explicit rm design, but make the plan authoritative`

- Related Plan Item: `T-3`
- Topic: Clean Design vs Minimal Churn
- Reviewer Position: Keep as is with clarification
- Advice:
  The explicit `rm` field on decoded FP forms is still the right trade-off for this codebase. The issue is no longer the direction of the design; it is that the document still carries rejected alternatives as if they were live options.
- Rationale:
  Hidden execution state would be harder to reason about and more brittle than a slightly wider decoded representation. The current final design is the clean one. The plan should present it cleanly.
- Required Action:
  Keep the explicit `rm` approach, but delete the superseded sketches and present only the final `DecodedInst` form in the next round.

### TR-2 `Preserve debug ergonomics while keeping a single canonical fcsr slot`

- Related Plan Item: `C-6`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer clean storage with explicit tooling compatibility
- Advice:
  The single canonical `fcsr` slot is a better state model than alias-plus-side-effect storage, but the plan should not pay for that cleanup by regressing register-name visibility in debug tooling.
- Rationale:
  Storage normalization and debugger ergonomics are separable concerns. The plan can and should do both.
- Required Action:
  Keep the specialized FP CSR storage model, and add an explicit name-resolution/debug strategy for `fflags` and `frm`.

## Positive Notes

- The move from alias-plus-side-effect FP CSR handling to a first-class specialized path is the right correction to the round-01 design error.
- The `FMIN/FMAX` qNaN vs sNaN validation matrix is now aligned with the spec direction and no longer repeats the earlier quiet-NaN flag mistake.
- Deferring Linux boot validation this round is acceptable because `01_MASTER` explicitly required that deferral; it is not a review defect in round `02`.

---

## Approval Conditions

### Must Fix
- R-001

### Should Improve
- R-002
- R-003
- R-004

### Trade-off Responses Required
- TR-1
- TR-2

### Ready for Implementation
- No
- Reason: The plan still leaves the architectural CSR write path for `fflags`/`frm`/`fcsr` without the required `FS=Dirty` transition, so FP state handling is not yet correct enough to approve.
