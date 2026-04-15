# `hotPath` REVIEW `01`

> Status: Open
> Feature: `hotPath`
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

- Decision: Approved with Revisions
- Blocking Issues: 2
- Non-Blocking Issues: 2

## Summary

The current worktree and `docs/PERF_DEV.md` show the project is in the
post-P1 stage: the bus fast-path already landed, the `docs/perf/*` path
rename is underway, and `hotPath` is still design-only. This round does
the administrative work well. It absorbs round-00 findings, follows the
`hotPath` rename and the `busFastPath` move, and expands scope per
`00_MASTER.md`.

The remaining blockers are both design-level, not editorial. First, the
P4 plan mixes up a decoded-raw cache with a translation-aware icache and
therefore adds a large invalidation lattice that is not justified by the
current code. Second, the Exit Gate allows the bundle to land even when an
individual PERF_DEV phase misses its own threshold, which conflicts with
the roadmap contract that each phase keeps its own measurable gate. Until
those are corrected, implementation should not begin.

---

## Findings

### R-001 `P4 invalidation model does not match what this cache actually memoizes`

- Severity: HIGH
- Section: `Summary`, `Architecture §P4`, `Invariants I-2/I-8/I-9/I-14/I-15`, `Implementation Plan / Phase 5`
- Type: Correctness | Spec Alignment
- Problem:
  The plan describes a decoded-instruction cache whose lookup happens only
  after `RVCore::step` has already fetched the current raw instruction word
  (`xemu/xcore/src/arch/riscv/cpu.rs:238-245`, `cpu/mm.rs:306-315`), and the
  proposed line key already includes `raw`. `DecodedInst::from_raw` depends
  on the raw bits plus static decode tables (`isa/riscv/decoder.rs:130-143`,
  `206-260`); privilege checks and memory-translation effects happen later in
  execute/fetch paths, not inside decode. Under that design, `satp`,
  `sfence.vma`, `fence.i`, privilege transitions, and especially
  "flush-on-every-RAM-store" are not load-bearing for decode correctness.
  The plan nevertheless threads `ctx_tag` through all of those events and
  proposes invalidating on every successful RAM store.

  That last part is especially problematic: a RAM store is the common case
  for ordinary stack/data traffic, not a rare self-modifying-code event, so
  Phase 5.6 would effectively cold-start the cache on normal workloads.
  This directly fights G-1/G-5 and makes the 95% hit-rate target hard to
  believe.

  The spec citations used by the plan also do not support the MPRV/SUM/MXR
  hooks. The current code already models MPRV as affecting loads/stores only
  (`cpu/mm.rs:228-235`, `261-264`), not fetch. The current privileged spec
  says instruction address translation/protection are unaffected by MPRV,
  and SUM/MXR describe load/store permissions rather than instruction fetch.
- Why it matters:
  This is the largest scope item in the round. If the invalidation model is
  wrong, the plan adds hot-path branches, CSR/trap hook churn, and a
  store-path cache flush that can erase the very P4 gain the round is trying
  to capture.
- Recommendation:
  In the next PLAN, either:
  1. Re-scope P4 as a pure decoded-raw cache keyed by `(pc, raw)` and delete
     the `ctx_tag` invalidation machinery, the RAM-store flush, and the
     MPRV/SUM/MXR-specific tests; or
  2. Explicitly prove that `DecodedInst` in this codebase has a semantic
     dependency on translation/privilege state that `raw` does not cover, and
     quantify why the extra invalidation traffic still preserves the exit
     gate.

### R-002 `The combined Exit Gate waives per-phase gates that PERF_DEV says are binding`

- Severity: HIGH
- Section: `Goals G-2..G-7`, `Exit Gate`
- Type: Spec Alignment | Validation
- Problem:
  `docs/PERF_DEV.md:186-190` defines each remaining phase as a separately
  measurable phase with its own exit gate. The final paragraph of this plan's
  Exit Gate says the round can still land when an individual phase misses its
  sub-threshold as long as the combined bundle passes. That turns the P3/P5/P6
  gates into advisory checks instead of binding ones.
- Why it matters:
  This breaks the roadmap contract the plan claims to implement. It also
  muddies attribution: a strong P4 result could hide an ineffective P3 or P5,
  yet the bundle would still be marked done. That makes follow-up planning and
  rollback decisions harder, not easier.
- Recommendation:
  Keep the bundled round if `00_MASTER.md` requires it, but make the per-phase
  gates binding inside the bundle. Acceptable shapes are:
  1. One branch / one doc set, but P3/P4/P5/P6 each must still meet their own
     PERF_DEV thresholds before the round is "ready for implementation"; or
  2. Keep the combined implementation work, but state explicitly that any phase
     missing its sub-gate is not considered landed and must be split into the
     next PLAN before approval.

### R-003 `P5 trap-slimming targets the wrong code path`

- Severity: MEDIUM
- Section: `Architecture §P5`, `Implementation Plan / Phase 7`
- Type: Correctness | Maintainability
- Problem:
  The plan says the "zero-pending-trap path through `commit_trap` should be a
  tight branch on a single field load." In the current code, the zero-pending
  branch already lives in `RVCore::retire` (`xemu/xcore/src/arch/riscv/cpu.rs:152-159`);
  `commit_trap` is only called after `pending_trap.take()` succeeded
  (`trap/handler.rs:58-75`). Extracting `has_pending_trap()` out of
  `commit_trap` therefore does not optimize the steady-state path the plan is
  trying to slim.
- Why it matters:
  P5 is one quarter of this bundled round. If its trap sub-plan is aimed at a
  non-existent hot path, the round carries extra complexity without a realistic
  path to the claimed trap-bucket gain.
- Recommendation:
  Re-scope the trap part of P5 to the actual per-step sites:
  `check_pending_interrupts`, the `pending_trap.take()` branch in `retire`,
  and any `mip`/`stimecmp` sync logic that still shows up in the profile.
  If no concrete hot branch remains after inspection, drop the trap-slimming
  subgoal from this round and keep P5 focused on the MMU fast path only.

### R-004 `Validation does not fully reflect the repo’s required verification contract`

- Severity: MEDIUM
- Section: `Constraints C-3`, `Implementation Plan`, `Exit Gate`
- Type: Validation
- Problem:
  The repository instructions require `make fmt`, `make clippy`, `make run`,
  and `make test` after coding modifications. The plan uses `cargo test --workspace`,
  `make m`, boot checks, and custom benchmark commands, but it never makes the
  required `make run` / `make test` pair a binding part of the step gate or
  the final Exit Gate.
- Why it matters:
  A round can satisfy the plan yet still fail the project’s mandatory
  verification workflow. That is a process bug, not just a wording issue.
- Recommendation:
  Update the next PLAN so both the per-step verification sentence and the Exit
  Gate explicitly include the repo-mandated command set:
  `make fmt`, `make clippy`, `make run`, and `make test`.
  Keep `cargo test --workspace`, `make m`, and the workload/boot checks as
  additional targeted evidence, not replacements.

---

## Trade-off Advice

### TR-1 `Prefer a simpler decoded-raw cache over a translation-context cache`

- Related Plan Item: `Architecture §P4`, `Invariants I-2/I-14/I-15`
- Topic: Simplicity vs Safety
- Reviewer Position: Prefer Option A
- Advice:
  Use the simplest structure that matches the real semantic dependency:
  cache decoded results of `(pc, raw)` only.
- Rationale:
  In the current implementation, fetch already resolves translation and
  returns the raw instruction bits before decode. That means the cache is
  memoizing decode work, not translation results. Carrying translation-context
  invalidation into this round buys very little and risks turning normal data
  stores into systematic cache flushes.
- Required Action:
  The executor should either adopt the simpler cache in the next PLAN or give
  a code-level proof that `raw` is insufficient for correctness.

### TR-2 `Keep the bundled round if required, but do not weaken phase accountability`

- Related Plan Item: `T-1`, `Exit Gate`
- Topic: Delivery Speed vs Measurability
- Reviewer Position: Prefer Option B
- Advice:
  One branch / one review cycle is acceptable, but the bundle still needs
  binding sub-gates per PERF_DEV phase.
- Rationale:
  The master directive justifies consolidation for workflow reasons. It does
  not justify declaring a phase done without evidence that its own hypothesis
  worked. Bundling without phase accountability makes later perf work less
  scientific and harder to maintain.
- Required Action:
  Keep the combined round only if the next PLAN states clearly how P3, P4, P5,
  and P6 each retain their own go/no-go criteria inside the shared artefact.

---

## Positive Notes

- Round-00 blocking findings were addressed concretely: the SMC path is now
  described through `checked_write`, the am-test wiring is explicit, and the
  response matrix is complete.
- The branch state is coherent with the docs: P1 is landed, the `docs/perf/*`
  reorg is in progress, and `hotPath` is correctly framed as the next planning
  step rather than disguised implementation.
- P3 and P6 are generally well-motivated by the post-P1 profile data in
  `docs/PERF_DEV.md`, and their expected wins remain plausible if the round is
  narrowed to changes that actually affect the measured hot path.

---

## Approval Conditions

### Must Fix
- R-001
- R-002

### Should Improve
- R-003
- R-004

### Trade-off Responses Required
- TR-1
- TR-2

### Ready for Implementation
- No
- Reason: P4’s invalidation model currently does not match the semantics of
  the proposed cache, and the Exit Gate weakens PERF_DEV’s per-phase contract.
  Both need to be corrected before implementation can start.
