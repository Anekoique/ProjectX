# `difftest` REVIEW `01`

> Status: Closed
> Feature: `difftest`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
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

`01_PLAN` is much better than round00 on ownership and monitor integration, but it is still not ready for implementation. The major problem is that the document claims more than it actually designs: it says the framework now supports both QEMU and Spike, yet Spike is still only a stub; it says the feature is isolated to `xdb`, yet it permanently changes `xcore::Bus`; it says interrupt/timer semantics are addressed, yet the critical QEMU single-step configuration is still missing; and it says `difftest` is independent of `debug`, while the actual snapshot path is built on `DebugOps`. Those are all blocking because they affect correctness, scope truthfulness, or the promised architecture boundary.

---

## Findings

### R-001 `Spike support is still only nominal`

- Severity: HIGH
- Section: `Summary / Feature Introduce / Goals / Response Matrix / Validation`
- Type: Spec Alignment
- Problem:
  The plan repeatedly claims that round01 now supports both QEMU and Spike (`DiffBackend` with `QemuBackend` and `SpikeBackend`, `M-001 applied`, `R-002 accepted`), but the concrete implementation section and validation section still define Spike as a stub that returns `"not implemented"`.
- Why it matters:
  A stub is not backend support. As written, the document says the master directive is resolved when the actual deliverable still only runs QEMU. That makes the scope non-reviewable and would let implementation start under a false approval boundary.
- Recommendation:
  Narrow the round01 claims to "QEMU backend implemented, Spike interface reserved" and explicitly mark `M-001` as only partially satisfied, or extend the plan to include a real Spike backend design that is runnable in this round.

### R-002 `feature isolation is contradicted by the always-on Bus change`

- Severity: HIGH
- Section: `Summary / Goals / Constraints / Phase 4`
- Type: Invariant
- Problem:
  The summary says the harness lives entirely in `xdb` behind `cfg(feature = "difftest")`, but the actual design adds an always-compiled `AtomicBool` field plus extra stores/swaps in `xcore/src/device/bus.rs`.
- Why it matters:
  This breaks the stated feature boundary and the original zero-cost expectation for non-difftest builds. It also contradicts the user's preference to enable difftest through a feature gate rather than by permanently modifying the core hot path.
- Recommendation:
  Make the `Bus` hook feature-gated as well, or move to a separate difftest-specific observer path in `xcore`. The next plan must not claim "xdb-only" or "feature-gated" while retaining always-on core mutations.

### R-003 `interrupt/timer single-step behavior is still unresolved`

- Severity: HIGH
- Section: `Review Adjustments / Response Matrix / Interrupt/Timer Single-Step Semantics / Validation`
- Type: Correctness
- Problem:
  The plan says `R-004` is accepted and resolved, but the only concrete treatment is "timer CSRs excluded", `mip` masking, and "validated empirically". The QEMU backend section never specifies the required gdbstub single-step configuration needed to keep IRQ/timer delivery semantically correct during per-instruction stepping.
- Why it matters:
  This was the main correctness blocker from round00. Without a concrete backend mechanism, difftest can still false-diverge on timer and interrupt workloads, which are already part of the guest-side tests and are central to OpenSBI/xv6/Linux bring-up.
- Recommendation:
  Replace the "validated empirically" wording with an explicit backend requirement and concrete packet/configuration sequence, then map it to validation. If that mechanism is not yet known, `R-004` must stay open.

### R-004 `difftest/debug feature boundary is internally inconsistent`

- Severity: HIGH
- Section: `Summary / Invariants / Phase 5 / Phase 6`
- Type: API
- Problem:
  The plan says `difftest` is independent of `debug`, but the DUT snapshot path is defined through `xcore::DebugOps`, `ArchSnapshot::from_dut(ops: &dyn xcore::DebugOps)`, and updated `cmd_step`/`cmd_continue` code that assumes access to existing debug inspection APIs. At the same time, the Makefile snippet only adds `--features difftest`, not `debug`.
- Why it matters:
  As written, batch difftest either depends on debug features implicitly or becomes underdefined/unbuildable. That is a blocking interface problem because the monitor cannot reliably construct the DUT snapshot without a clear compiled API contract.
- Recommendation:
  Either make `difftest` explicitly depend on `debug`, or split the snapshot/export surface into a separate trait/feature that `difftest` owns directly. The next plan must pick one and align Summary, Cargo features, and code snippets with it.

### R-005 `runtime control is still tied to `X_FILE` and not fully monitor-owned`

- Severity: MEDIUM
- Section: `Constraints / Main Flow / Failure Flow`
- Type: Flow
- Problem:
  The review-adjustment text claims runtime control was added, but the constraints still define the binary path as coming from `X_FILE`, and the attach flow still assumes a pre-existing binary path without specifying how interactive monitor attach behaves if the user has not loaded an image.
- Why it matters:
  This weakens the claimed shift to a true monitor-owned workflow. It is not a blocker if round01 narrows itself to "attach only after load", but the current wording is broader than the actual flow.
- Recommendation:
  State the exact runtime contract: either `dt attach` requires a previously loaded binary, or the command accepts an explicit path and owns loading/reload itself.

### R-006 `response matrix overstates the resolution of R-005`

- Severity: LOW
- Section: `Review Adjustments / Response Matrix / Main Flow`
- Type: Maintainability
- Problem:
  The document says `R-005` is accepted because "MMIO-skip syncs registers to REF", but the previous review asked for explicit RAM-write synchronization together with architectural state. The current plan still only records a single `AtomicBool`.
- Why it matters:
  Even if the author believes RAM-write sync is unnecessary for the current instruction model, the response does not answer the review recommendation as written.
- Recommendation:
  Either add RAM-write synchronization to the design or explicitly reject that part of the prior recommendation with reasoning.

---

## Trade-off Advice

### TR-1 `feature boundary between debug and difftest`

- Related Plan Item: `G-5 / C-6 / Phase 6`
- Topic: Flexibility vs Safety
- Reviewer Position: Need More Justification
- Advice:
  Reusing `DebugOps` is reasonable, but only if the feature relationship is made explicit. Hiding that dependency under "independent features" is the wrong compromise.
- Rationale:
  The current plan tries to get both reuse and independence, and ends up with an interface contract that is unclear for batch builds.
- Required Action:
  Executor should choose one of:
  `difftest -> debug` dependency, or a separate `DifftestOps` export path. Then update Cargo features, snippets, and invariants consistently.

### TR-2 `minimal xcore change vs zero-cost disabled build`

- Related Plan Item: `T-1 / G-5 / C-4`
- Topic: Performance vs Simplicity
- Reviewer Position: Prefer a feature-gated hook
- Advice:
  One always-on `AtomicBool` is simple, but it is not compatible with the plan's own feature-gating claims.
- Rationale:
  If round01 wants to preserve "zero difftest code when disabled", the hook must be conditionally compiled as well. Otherwise the plan should narrow its claims and accept the cost explicitly.
- Required Action:
  Executor should either gate the hook or rewrite the Summary/Goals/Constraints to truthfully describe the always-on cost.

### TR-3 `QEMU-first delivery vs truthful multi-backend scope`

- Related Plan Item: `M-001 / NG-1 / V-F-4`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer truthful QEMU-first scope unless Spike is really designed
- Advice:
  It is acceptable to ship QEMU first, but not to present a stub as completed Spike support.
- Rationale:
  Truthful scoping is more valuable than nominal symmetry. The framework can still reserve a Spike backend slot without claiming that the backend exists.
- Required Action:
  Executor should either narrow the wording to "QEMU implemented, Spike reserved" or provide a real Spike backend plan for this round.

---

## Positive Notes

- Moving difftest ownership into `xdb` is the right architectural direction and clearly addresses the main layering problem from round00.
- The CSR whitelist is a meaningful improvement over the old `pc + gpr` scope and is much closer to what Phase 6 actually needs.
- Adding explicit monitor commands (`dt attach`, `dt detach`, `dt status`) is a useful step toward a real debugging workflow.

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
- Reason: The plan still overclaims backend support, leaves the key QEMU step-semantics blocker unresolved, and has an inconsistent feature/API boundary between `difftest`, `debug`, and the always-on `Bus` hook.
