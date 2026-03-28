# `benchmark-adaptation` REVIEW `02`

> Status: Open
> Feature: `benchmark-adaptation`
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
- Blocking Issues: 2
- Non-Blocking Issues: 1

## Summary

`02_PLAN` fixes most of the real blockers from round `01`.

The round now matches xam's `SRCS`-driven build model, the microbench `mainargs`
path is finally concrete, and the plan is much narrower now that `malloc/free`
has been deferred instead of being forced into the current scope.

Two implementation blockers still remain, both in the microbench/lzip/assert area:

- the plan claims all NJU header dependencies are removed, but it still misses
  `microbench/src/lzip/quicklz.h`, which directly includes `<am.h>` and `<klib.h>`;
- the proposed new `assert.h` is not C++-safe, while the plan still intends all
  C++ microbench sub-benchmarks to use it.

There is also one medium issue in the `alu-tests` generation flow: the plan writes
to `tests/test.c`, but that directory does not currently exist in the tree.

So this round is close, but not yet ready for implementation approval.

---

## Findings

### R-001 `microbench still has an unported NJU dependency in quicklz.h`

- Severity: HIGH
- Section: `Spec / Invariants / Phase 7`
- Type: Completeness
- Problem:
  `02_PLAN` claims no NJU headers remain in the adapted code and only patches
  `microbench/include/benchmark.h` plus `microbench/src/bench.c`. But
  [quicklz.h](/Users/anekoique/ProjectX/xkernels/benchmarks/microbench/src/lzip/quicklz.h)
  still directly includes `<am.h>` and `<klib.h>` at lines 4-5. The plan never
  mentions patching that file.
- Why it matters:
  This leaves the `lzip` sub-benchmark uncompilable under the proposed port and
  makes invariant `I-4` false as written. Since `G-7` explicitly includes all 10
  microbench sub-benchmarks, this is a real implementation blocker.
- Recommendation:
  Add an explicit `quicklz.h` migration step in the next plan. Either replace those
  includes with standard/xlib-compatible headers and keep the helper local, or
  justify a narrower benchmark subset if `lzip` is being deferred.

### R-002 `The proposed assert.h will break C++ microbench files at link time`

- Severity: HIGH
- Section: `Phase 3 / Phase 7`
- Type: API
- Problem:
  The new `xlib/include/assert.h` declares `halt()` and `printf()` as plain `extern`
  declarations with no `extern "C"` guards
  ([02_PLAN.md](/Users/anekoique/ProjectX/docs/xkernels/benchmark/02_PLAN.md):200-217).
  But the round still expects C++ files such as `15pz.cc`, `dinic.cc`, and `ssort.cc`
  to use `assert()`. In C++ translation units, those declarations acquire C++ linkage.
  A local compile probe with the planned header shape produces unresolved mangled
  references `_Z4halti` and `_Z6printfPKcz`, which do not match the actual C symbols.
- Why it matters:
  This is a direct correctness bug in the proposed compatibility surface. Even if the
  benchmark sources are otherwise ported correctly, C++ sub-benchmarks will still fail
  to link once `assert()` expands to those declarations.
- Recommendation:
  Make the planned `assert.h` C++-safe with `extern "C"` guards around the
  declarations, or move the assert shim into a header that is already inside a
  correct `extern "C"` context for the C++ consumers.

### R-003 `alu-tests generation path points to a directory that does not exist yet`

- Severity: MEDIUM
- Section: `Phase 4`
- Type: Flow
- Problem:
  The plan generates `alu-tests` output into `tests/test.c`
  ([02_PLAN.md](/Users/anekoique/ProjectX/docs/xkernels/benchmark/02_PLAN.md):223-249),
  but the current directory only contains `LICENSE` and `gen_alu_test.c`; there is no
  existing `tests/` directory under
  [xkernels/tests/alu-tests](/Users/anekoique/ProjectX/xkernels/tests/alu-tests).
- Why it matters:
  The plan's first concrete generation command fails as written unless the directory is
  created first. This is not architecturally serious, but it is still an execution-gap
  in the round.
- Recommendation:
  Either add an explicit `mkdir -p tests` step to the plan or simplify the layout and
  store the generated file directly under `xkernels/tests/alu-tests/`.

---

## Trade-off Advice

### TR-1 `Prefer a target-local quicklz fix over any broader compatibility revival`

- Related Plan Item: `G-7 / Phase 7`
- Topic: Clean Design vs Partial Compatibility Revival
- Reviewer Position: Prefer Option A
- Advice:
  Patch `quicklz.h` directly instead of reintroducing any broader NJU compatibility
  layer just to satisfy `lzip`.
- Rationale:
  The round is finally narrow again. One remaining header in one sub-benchmark does
  not justify widening the architecture.
- Required Action:
  Add a local `quicklz.h` migration step in the next plan.

### TR-2 `Keep assert support, but make it C/C++ safe before generalizing it`

- Related Plan Item: `G-3 / T-3`
- Topic: Reuse vs Safety
- Reviewer Position: Prefer Option A
- Advice:
  It is reasonable to add `xlib/include/assert.h`, but only if the header is written
  to work correctly in both C and C++ consumers.
- Rationale:
  This round already has C++ benchmark files that will consume the new assert surface.
  General reuse is fine, but not at the cost of a broken ABI.
- Required Action:
  Add C++ linkage guards or keep the assert shim in a header that already provides
  them.

---

## Positive Notes

- `02_PLAN` correctly resolves the prior Makefile `SRCS` issue.
- The `mainargs` plan is now concrete and consistent with xam's existing ABI pattern.
- Deferring `malloc/free` is the right narrowing move for this round.

---

## Approval Conditions

### Must Fix
- R-001
- R-002

### Should Improve
- R-003

### Trade-off Responses Required
- TR-1
- TR-2

### Ready for Implementation
- No
- Reason: the round is close, but the unported `quicklz.h` dependency and the
  C++-unsafe `assert.h` design would still break the microbench port as written.
