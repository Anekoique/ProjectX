# `difftest` REVIEW `00`

> Status: Closed
> Feature: `difftest`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
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

`00_PLAN` has the right direction, but it is not ready for implementation. The main problems are structural and correctness-related: difftest is designed inside `xcore` instead of `xdb`, the plan hard-codes a QEMU-only backend, it excludes CSR comparison even though Phase 6 requires it, and it does not define safe single-step behavior for interrupt/timer paths. In its current form, the implementation would either violate the monitor/core layering or produce misleading difftest failures on the exact workloads Phase 6 is supposed to protect.

---

## Findings

### R-001 `difftest ownership is placed in the wrong layer`

- Severity: HIGH
- Section: `Architecture / API Surface / Implementation Plan`
- Type: Maintainability
- Problem:
  The plan makes `xcore` own `DifftestContext`, the reference process, and the GDB client, and it inserts difftest directly into `RVCore::step()`.
- Why it matters:
  The existing architecture already separates execution (`xcore`) from monitor/debug policy (`xdb`). Putting reference lifecycle, mismatch reporting, and backend policy into `xcore` couples the hottest execution path to monitor concerns and makes future backend expansion awkward.
- Recommendation:
  Move difftest orchestration into `xdb`. Keep `xcore` limited to a small feature-gated probe surface such as architectural snapshot export and per-step MMIO/RAM-write events.

### R-002 `QEMU-only design does not match Phase 6 reference scope`

- Severity: HIGH
- Section: `Summary / Goals / Constraints / Implementation Plan`
- Type: Spec Alignment
- Problem:
  The plan is concretely built around one backend: QEMU gdbstub on `127.0.0.1:1234`, and it explicitly marks Spike out of scope.
- Why it matters:
  `docs/DEV.md` frames Phase 6 as comparison against `QEMU/Spike`, and the next boot phases need a framework that can swap references without redesign. A QEMU-only in-core client is not a reusable difftest framework.
- Recommendation:
  Introduce a monitor-side backend abstraction now and make both `qemu` and `spike` first-class backend kinds, even if they use different control mechanisms.

### R-003 `comparison scope is too weak for boot-stage correctness`

- Severity: HIGH
- Section: `Goals / Non-Goals / Invariants / Validation`
- Type: Spec Alignment
- Problem:
  `00_PLAN` narrows compared state to `pc + gpr`, and explicitly excludes CSR comparison from the initial scope.
- Why it matters:
  `docs/DEV.md` defines Phase 6 state diff as `{PC, GPR, CSR}`. For OpenSBI, xv6, and Linux bring-up, divergence in trap, delegation, privilege, and translation CSRs is exactly what difftest must catch. A `pc + gpr`-only checker will miss real architectural bugs.
- Recommendation:
  Add privilege mode plus a whitelist/mask-based CSR compare set in the next plan. Counters can still be excluded.

### R-004 `QEMU single-step behavior is underspecified for interrupt/timer paths`

- Severity: HIGH
- Section: `Execution Flow / Constraints / Validation`
- Type: Correctness
- Problem:
  The plan assumes that stepping QEMU one instruction at a time is enough for comparison, but it does not define how interrupt and timer delivery behave while the reference is in single-step mode.
- Why it matters:
  Timer and interrupt behavior is already part of the guest-side test corpus. If the reference changes or suppresses IRQ/timer delivery during stepping, difftest will report false divergences on precisely the workloads Phase 6 must validate.
- Recommendation:
  Make single-step interrupt/timer semantics an explicit backend requirement and validate it in the next plan and test strategy.

### R-005 `MMIO skip policy is incomplete after side-effecting steps`

- Severity: MEDIUM
- Section: `Goals / Invariants / Trade-offs`
- Type: Flow
- Problem:
  The plan says MMIO steps should skip comparison and sync DUT state to REF, but it does not define whether ordinary RAM writes from the same instruction are also synchronized.
- Why it matters:
  A step that both writes RAM and touches MMIO can leave the reference behind even if registers are synced. The next compared step then reports a secondary mismatch that hides the real cause.
- Recommendation:
  Track per-step events and explicitly synchronize RAM writes together with architectural state on MMIO-skipped steps.

### R-006 `runtime control model is too rigid for monitor workflows`

- Severity: LOW
- Section: `Constraints / Implementation Plan`
- Type: Maintainability
- Problem:
  The plan only describes `DIFFTEST=1 make run` as a startup mode and does not define attach/detach/reset behavior from `xdb`.
- Why it matters:
  Difftest is most useful as a monitor capability. Without monitor-facing control, the user cannot inspect, reset, or swap references cleanly during debugging sessions.
- Recommendation:
  Add `xdb`-level commands or runtime controls for attach, detach, status, and re-sync on `load` / `reset`.

---

## Trade-off Advice

### TR-1 `ownership boundary`

- Related Plan Item: `T-2`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option B
- Advice:
  Keep reference orchestration out of `RVCore` and make the monitor own it.
- Rationale:
  The current architecture already separates execution from debugging/monitor policy. Difftest belongs with breakpoints, watchpoints, and stepping strategy, not with instruction execution internals.
- Required Action:
  Executor should rewrite the plan around an `xdb`-owned harness and reduce `xcore` to probe hooks only.

### TR-2 `reference backend strategy`

- Related Plan Item: `NG-2`
- Topic: Flexibility vs Simplicity
- Reviewer Position: Need More Justification
- Advice:
  Do not keep Spike as a deferred non-goal if the framework is supposed to serve the next boot stage. At minimum, the abstraction for multiple backends must be part of the first approved plan.
- Rationale:
  Retrofitting backend abstraction later will force rework exactly where the current plan proposes hard-coded QEMU types and assumptions.
- Required Action:
  Executor should either add a backend trait now or explicitly justify why later backend addition would not change architecture, APIs, or invariants.

### TR-3 `compare scope`

- Related Plan Item: `NG-1`
- Topic: Flexibility vs Safety
- Reviewer Position: Prefer broader initial scope
- Advice:
  Prefer `pc + gpr + privilege + selected CSR whitelist` over `pc + gpr` only.
- Rationale:
  This keeps the checker usable for supervisor-boot debugging without paying the cost or instability of a full CSR dump.
- Required Action:
  Executor should expand compared architectural state and explain which CSRs are masked or excluded.

---

## Positive Notes

- The plan correctly identifies MMIO as a place where direct lock-step comparison is unsafe across different device models.
- The feature-gating requirement is good and should be preserved in the next round.
- The validation section already points at intentional-divergence testing, which is the right mindset for a difftest framework.

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
- Reason: The round00 plan still has unresolved structural and correctness blockers around ownership, backend scope, compare scope, and interrupt/timer stepping behavior.
