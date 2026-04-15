# `difftest` REVIEW `05`

> Status: Closed
> Feature: `difftest`
> Iteration: `05`
> Owner: Reviewer
> Target Plan: `05_PLAN.md`
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
- Blocking Issues: `4`
- Non-Blocking Issues: `2`

## Summary

`05_PLAN` resolves several real round04 issues. The raw-vs-masked CSR split is the right fix, separating Spike behind `difftest-spike` is the right direction, and using `ctx.csrs` as backend metadata is cleaner than duplicating the whitelist in xdb. But the plan still is not implementation-ready. The most serious remaining problem is that it proposes adding an inherent `impl CoreContext` block inside xdb even though `CoreContext` is defined in xcore, which is not legal Rust. On top of that, the `DebugOps` trim regresses debugger register coverage from "any CSR name" to only the difftest whitelist, the new architecture boundary still leaks RISC-V details into xdb, and the plan still does not specify how active difftest disables xdb’s current `continue` fast path that bypasses per-instruction hooks.

---

## Findings

### R-001 `xdb cannot add inherent methods to xcore's CoreContext type`

- Severity: HIGH
- Section: `API Surface / Phase 7`
- Type: Correctness
- Problem:
  The plan defines `CoreContext` in xcore and re-exports it, but then proposes `impl CoreContext { pub fn diff(...) ... }` inside xdb.
- Why it matters:
  Rust does not allow adding inherent impls to a type defined in another crate. That means the central `CoreContext::diff()` design in round05 does not compile as written. This is not a stylistic concern; it is a hard language boundary.
- Recommendation:
  Move `diff()` into xcore, or define it as an extension trait / free function / xdb-owned wrapper type. Do not rely on an inherent impl in xdb for a foreign type.

### R-002 `removing read_register regresses debugger CSR coverage to the difftest whitelist`

- Severity: HIGH
- Section: `Summary / Master Compliance / How xdb commands migrate to CoreContext / Phase 1 / Phase 2 / Phase 3`
- Type: Regression
- Problem:
  The current debugger can resolve arbitrary CSR names through `DebugOps::read_register()` and `CsrAddr::from_name(...)`. Round05 removes that API and replaces register lookup with `CoreContext::register_by_name()`, but that helper only searches `ctx.csrs`, which is built from the 14-entry difftest whitelist.
- Why it matters:
  This silently narrows debugger functionality. Commands like `info reg <csr>` and `p $<csr>` stop working for any CSR that is not in the difftest compare set. That is an avoidable regression caused by trimming the API past what `CoreContext` currently models. `04_MASTER.md` explicitly allowed keeping or adding `DebugOps` APIs for information that cannot be supplied from `CoreContext`.
- Recommendation:
  Keep a targeted register-read API for debugger lookups, or introduce a separate full debug-register view distinct from the difftest CSR whitelist. Do not make the difftest whitelist the debugger’s complete CSR namespace.

### R-003 `the claimed arch-dependent boundary still hardcodes RISC-V knowledge into xdb`

- Severity: HIGH
- Section: `Summary / Master Compliance / Architecture / Phase 1 / Phase 3`
- Type: Design
- Problem:
  The plan says `CoreContext` is arch-dispatched like `Core`, but xdb-side migration still hardcodes RISC-V details such as `xcore::isa::RVReg::from_u8(...)`, and the proposed CPU dispatch snippet only shows the `#[cfg(riscv)]` alias for `CoreContext`.
- Why it matters:
  This means the new boundary is not actually architecture-neutral. xdb still reconstructs ordered register naming from RISC-V types instead of consuming a fully self-describing context, and the plan leaves the non-RISC-V branch underspecified even though the tree still contains `cpu/loongarch` and `isa/loongarch`. That is exactly the sort of partial abstraction that becomes brittle during later feature work.
- Recommendation:
  Either gate the entire refactor explicitly to RISC-V for this round, or move register naming / ordered register iteration fully behind the arch layer so xdb does not name `RVReg` directly. If `CoreContext` is part of `cpu`’s cross-arch API, every architecture branch needs a concrete dispatch story.

### R-004 `continue-path difftest integration is still not specified against the current xdb fast path`

- Severity: HIGH
- Section: `Execution Flow / Phase 3 / File Summary`
- Type: Flow
- Problem:
  The plan says "`s` or `c` -> per-step", but it never describes how that replaces xdb’s current `cmd_continue()` fast path, which immediately calls `cpu.run(u64::MAX)` whenever there are no watchpoints.
- Why it matters:
  In the current shell, `continue` without watchpoints is the normal path. If that branch remains, active difftest is bypassed entirely during `c`, because per-instruction hooks only run in the step loop. Round05 needs an explicit integration point here, not just a high-level intention.
- Recommendation:
  State concretely that `cmd_continue()` must not use `cpu.run()` while difftest is attached, or refactor `run()` to accept a post-step hook so the fast path stays correct. Without that, the plan leaves a critical execution path undefined.

### R-005 `addr-based CSR comparison still ignores missing counterpart entries`

- Severity: MEDIUM
- Section: `Phase 7`
- Type: Correctness
- Problem:
  The new `diff()` loop compares a DUT CSR only if `other.csrs.iter().find(|c| c.addr == dut_csr.addr)` succeeds. If the REF context omits a CSR entry entirely, the code just skips it.
- Why it matters:
  That weakens the "single source of truth" guarantee. A backend omission or malformed context should be a loud mismatch, not silently treated as acceptable.
- Recommendation:
  Treat missing CSR counterparts as a mismatch, or pre-validate that both contexts contain the same CSR address set before value comparison.

### R-006 `the Spike pin is now concrete, but the documented provenance is inaccurate`

- Severity: LOW
- Section: `Review Adjustments / Constraints`
- Type: Spec Alignment
- Problem:
  The plan says Spike is pinned to riscv-isa-sim `v1.1.0` and describes it as a "2023-12 release".
- Why it matters:
  The upstream tag exists, so the placeholder problem is fixed, but the tag metadata is still wrong: upstream shows `v1.1.0` as commit `530af85` dated December 17, 2021. If the plan wants the pin to act as a real compatibility contract, the recorded provenance should be accurate.
- Recommendation:
  Record the exact tag/commit correctly, for example `v1.1.0` / `530af85`, and drop the incorrect release date.

---

## Trade-off Advice

### TR-1 `API reduction vs debugger completeness`

- Related Plan Item: `M-002 response / DebugOps trimming`
- Topic: Simplicity vs Capability
- Reviewer Position: Prefer keeping the missing capability
- Advice:
  Trim `DebugOps` only as far as `CoreContext` can faithfully replace it.
- Rationale:
  The point of the refactor is a clearer boundary, not a debugger regression. Today `CoreContext` does not carry the full CSR namespace, so removing `read_register()` outright is premature.
- Required Action:
  Executor should either preserve a narrow debugger lookup API or widen the context model so the old capability is not lost.

### TR-2 `clean arch dispatch vs xdb-side ISA assumptions`

- Related Plan Item: `M-001 response / Phase 3`
- Topic: Architectural cleanliness vs expedience
- Reviewer Position: Prefer a real boundary
- Advice:
  If `CoreContext` is the arch-dispatched API, xdb should not reconstruct RISC-V register semantics itself.
- Rationale:
  Hardcoding `RVReg` in xdb defeats the main value of the dispatch refactor and leaves non-RISC-V support in a half-migrated state.
- Required Action:
  Executor should move ordered register naming/iteration into xcore or explicitly narrow the scope to RISC-V-only delivery.

### TR-3 `preserving continue fast paths vs guaranteeing difftest coverage`

- Related Plan Item: `Execution Flow`
- Topic: Performance vs Correctness
- Reviewer Position: Prefer correctness
- Advice:
  It is acceptable to give up the existing `cpu.run()` fast path while difftest is attached.
- Rationale:
  A fast path that bypasses per-instruction comparison is not a valid difftest path. Correct step-hook coverage matters more than preserving the non-difftest execution shortcut.
- Required Action:
  Executor should explicitly redesign `cmd_continue()` / `run()` interaction for active difftest.

---

## Positive Notes

- The raw-vs-masked `CsrValue` split is the right answer to the round04 sync corruption issue.
- Splitting Spike behind `difftest-spike` is directionally correct and matches Cargo’s feature-environment model for build scripts.
- Removing the duplicated QEMU CSR table is the right cleanup; using `ctx.csrs` metadata is much closer to a defensible single-source design.

---

## Approval Conditions

### Must Fix
- R-001
- R-002
- R-003
- R-004

### Should Improve
- R-005
- R-006

### Trade-off Responses Required
- TR-1
- TR-2
- TR-3

### Ready for Implementation
- No
- Reason: Round05 still contains a non-compilable `CoreContext` API design, regresses debugger register coverage by collapsing it to the difftest whitelist, leaks RISC-V-specific knowledge back into xdb despite claiming an arch-dispatched boundary, and leaves the active-difftest `continue` path undefined against xdb’s current fast path.
