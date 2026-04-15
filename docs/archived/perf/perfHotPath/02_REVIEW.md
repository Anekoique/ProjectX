# `hotPath` REVIEW `02`

> Status: Open
> Feature: `hotPath`
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

- Decision: Approved with Revisions
- Blocking Issues: 2
- Non-Blocking Issues: 2

## Summary

Round 02 fixes the most important design problem from round 01: the P4
cache is now modeled as decode memoization, not as a translation-context
cache. That is the right direction for the current codebase, where
`fetch()` produces raw instruction bits before `decode()` runs.

The remaining problems are in validation and process alignment, not in the
core P4 simplification. The plan weakens the P4 roadmap by demoting the
SMC am-test that `PERF_DEV.md` still marks as a pre-phase requirement, and
its new "mandatory" command block still does not correspond to executable
repo commands as written. Those need to be fixed before the round is ready
for implementation.

---

## Findings

### R-001 `Round 02 drops the P4 SMC am-test gate that PERF_DEV still requires`

- Severity: HIGH
- Section: `Log`, `Changes from Previous Round`, `Response Matrix`, `Validation`
- Type: Spec Alignment | Validation
- Problem:
  Round 02 explicitly demotes the SMC am-test from a gate to future work
  ([docs/perf/hotPath/02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:84),
  [02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:160),
  [02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:177),
  [02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:818)).
  But `docs/PERF_DEV.md` still defines that guest-modifies-text am-test as
  the P4 pre-phase test and includes "the new text-modifying am-test passes"
  in the phase exit gate
  ([docs/PERF_DEV.md](/Users/anekoique/ProjectX/docs/PERF_DEV.md:307),
  [docs/PERF_DEV.md](/Users/anekoique/ProjectX/docs/PERF_DEV.md:321)).

  The new `(pc, raw)` design makes the am-test easier to satisfy, but it
  does not let this round silently delete a roadmap requirement that the
  authoritative perf plan still treats as binding.
- Why it matters:
  This is a direct spec mismatch. The executor cannot claim P4 compliance
  while explicitly removing one of PERF_DEV's named gates.
- Recommendation:
  In the next PLAN, either keep the SMC am-test as a binding P4 gate, or
  explicitly update the authoritative contract first: add a MASTER waiver or
  revise `docs/PERF_DEV.md` so the pre-phase and exit-gate language no
  longer requires that test.

### R-002 `The new mandatory command block still does not map to real repo commands`

- Severity: HIGH
- Section: `Summary`, `Constraints C-2/C-4`, `Implementation Plan`, `Validation`, `Exit Gate`
- Type: Validation | Maintainability
- Problem:
  Round 02 says it adopted the repo-mandated block
  `make fmt && make clippy && make run && make test`
  ([02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:51),
  [02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:550),
  [02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:625),
  [02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:794),
  [02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:914)).
  But there is no top-level `Makefile` in the repository root, so those
  commands are not runnable there. The actual targets are split across
  subtrees:

  - [xemu/Makefile](/Users/anekoique/ProjectX/xemu/Makefile:44) has `run`,
    but it is `cargo run`, not "each benchmark: dhrystone / coremark /
    microbench".
  - [xemu/Makefile](/Users/anekoique/ProjectX/xemu/Makefile:53) has `test`,
    but it is `cargo test -p xcore`, not `cargo test --workspace + am-tests`.
  - [resource/Makefile](/Users/anekoique/ProjectX/resource/Makefile:30) and
    [resource/Makefile](/Users/anekoique/ProjectX/resource/Makefile:32) host
    `opensbi`/`linux`/`linux-2hart`, while `debian` comes from
    [resource/debian.mk](/Users/anekoique/ProjectX/resource/debian.mk:54).

  So the block is still conceptually and mechanically wrong as a binding
  gate.
- Why it matters:
  This is not a wording nit. A plan cannot claim a binding verification
  contract if the commands do not execute in the stated workspace or do not
  prove the stated behavior.
- Recommendation:
  Rewrite the next PLAN's verification section with exact runnable commands
  and working directories. Acceptable fixes are:
  1. Add a repo-root `Makefile` that forwards `fmt/clippy/run/test/linux/...`
     to the right subdirectories; or
  2. Spell out the real commands, e.g. `make -C xemu fmt`, `make -C xemu clippy`,
     `make -C xemu run`, `make -C xemu test`, `cargo test --workspace`,
     `make -C xkernels/tests/am-tests run`, `make -C resource linux`,
     `make -C resource linux-2hart`, `make -C resource debian`.

### R-003 `P5 uses cargo-asm as a binding gate, but the tool is not available in the current environment`

- Severity: MEDIUM
- Section: `Architecture §P5`, `Implementation Plan / Phase 3`, `Validation V-IT-5`, `Exit Gate §A P5`
- Type: Validation | Tooling
- Problem:
  P5 makes `cargo asm` part of the implementation flow and exit evidence
  ([02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:349),
  [02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:664),
  [02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:843),
  [02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:943)),
  but `cargo-asm` is not installed in the current workspace environment
  (`command -v cargo-asm` returns non-zero).
- Why it matters:
  A binding gate that depends on an unavailable external tool is not
  presently executable. Even if the design is sound, the round cannot be
  validated as written.
- Recommendation:
  Either add a bootstrap/install step for the required tool, or define an
  alternate evidence path that uses tools already present in the repo
  environment, such as emitted assembly, `objdump`, or profile-based proof.

### R-004 `The phase-split policy leaves combined-gate thresholds undefined after a split`

- Severity: MEDIUM
- Section: `Exit Gate §B`, `Policy`
- Type: Validation | Flow
- Problem:
  The new policy says that if a §A gate fails, the remaining phases may
  still land if §B passes "against the reduced bundle"
  ([02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:964)).
  But §B's thresholds are only specified for the full P3+P4+P5+P6 bundle
  ([02_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/02_PLAN.md:950)).
  There is no rule for how those thresholds are recalculated once one phase
  is removed.
- Why it matters:
  This invites post-hoc acceptance logic. If P5 or P6 drops out, the round
  needs an explicit reduced-bundle target, not an informal promise to
  "verify against the reduced bundle".
- Recommendation:
  In the next PLAN, either:
  1. Remove the reduced-bundle escape hatch and require a fresh PLAN when any
     §A phase fails; or
  2. Predefine the reduced-bundle thresholds for each allowed split case.

---

## Trade-off Advice

### TR-1 `Keep the decoded-raw simplification, but do not use it to waive explicit roadmap gates`

- Related Plan Item: `T-1`, `T-3`
- Topic: Simplicity vs Process Discipline
- Reviewer Position: Prefer Option A
- Advice:
  Keep the `(pc, raw)` cache design. It is cleaner and better aligned with
  the current fetch-decode boundary. But keep the explicit SMC am-test until
  the authoritative roadmap is updated.
- Rationale:
  The design simplification is technically strong. The problem is not the
  cache shape; the problem is deleting a named gate without updating the
  controlling roadmap.
- Required Action:
  Adopt the simpler cache, but restore or formally waive the am-test gate in
  the next PLAN.

### TR-2 `Prefer executable verification commands over inherited shorthand`

- Related Plan Item: `R-004 adoption`, `Validation`, `Exit Gate`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option B
- Advice:
  Use exact runnable commands and workdirs, even if that makes the validation
  block longer.
- Rationale:
  Reusing the shorthand `make fmt`, `make run`, `make test`, `make linux`
  reads cleanly, but it is false in this repo layout. A slightly longer plan
  with exact commands is better than a concise plan that cannot be executed.
- Required Action:
  Rewrite the next PLAN's validation and exit-gate command blocks with
  explicit `make -C ...` or `cargo ...` invocations.

---

## Positive Notes

- The round-02 P4 simplification is materially better than round 01. The plan
  now matches the actual `fetch -> decode` separation in the codebase and
  removes a large amount of unnecessary invalidation machinery.
- The round correctly accepts the round-01 feedback on phase accountability:
  splitting §A per-phase gates from §B bundle gates is a real improvement.
- Dropping the P5 trap-slim subgoal is the right call. The cited hot path in
  round 01 was not the real steady-state branch site.

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
- Reason: the core P4 design is now sound, but the round still weakens an
  explicit PERF_DEV P4 gate and its mandatory verification commands are not
  runnable as written.
