# `benchmark-adaptation` REVIEW `01`

> Status: Open
> Feature: `benchmark-adaptation`
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
- Blocking Issues: 3
- Non-Blocking Issues: 1

## Summary

`01_PLAN` is materially better than round `00`: the heap design now follows the
reviewed linker-symbol approach, the CoreMark porting path is narrower, and the
plan finally contains concrete per-file migration notes instead of only high-level
intent.

Three correctness blockers still remain:

- the benchmark Makefiles are still not described in a way that matches xam's actual
  `SRCS`-driven C build contract,
- microbench mode selection is still not wired to xam's `mainargs` ABI,
- and the plan's `assert()` fix is proposed but never actually included into the
  microbench sources that use `assert()`.

There is also one medium-scope issue: the new `xlib malloc/free` work widens the
round significantly, but the current benchmark set still does not actually need it
for acceptance.

So this round is closer, but it is not yet ready for implementation approval.

---

## Findings

### R-001 `Benchmark Makefiles are still missing the source lists xam actually builds`

- Severity: HIGH
- Section: `Implement / Phase 5 / Phase 6 / Phase 7`
- Type: Flow
- Problem:
  The plan says no Makefile changes are needed for `coremark`, `dhrystone`, and
  `microbench`, but the current xam C build path only compiles files listed in
  `SRCS`. The benchmark Makefiles currently define only `K ?= $(abspath .)` and no
  `SRCS` at all, so the plan's "no changes needed" claim does not match the actual
  build system.
- Why it matters:
  As written, the round cannot build the benchmark objects it proposes to adapt.
  This is a direct implementation blocker.
- Recommendation:
  The next plan must define the exact `SRCS` set for each benchmark target, or
  explicitly add a source-discovery mechanism to xam and justify that scope change.

### R-002 `Microbench argument selection is still not connected to xam's mainargs ABI`

- Severity: HIGH
- Section: `Implement / Phase 7 / Phase 8`
- Type: API
- Problem:
  The plan validates microbench with `MAINARGS=test make run`, but the current xam
  runtime does not read a make variable named `MAINARGS`. It reads a strong
  `mainargs` symbol when one is provided; otherwise it falls back to the weak empty
  default. `01_PLAN` never adds `const char mainargs[] = MAINARGS;`, never adds a
  per-target `CFLAGS += -DMAINARGS=...`, and never proposes a wrapper source like
  the existing `am-tests` runner pattern.
- Why it matters:
  Without explicit `mainargs` plumbing, microbench always receives the empty default
  and falls back to `"ref"`. That makes the proposed mode-specific validation wrong
  and leaves the only benchmark that actually consumes runtime args incompletely
  adapted.
- Recommendation:
  Add an explicit `mainargs` strategy in the next plan: either a tiny source that
  defines `const char mainargs[] = MAINARGS;` plus Makefile `CFLAGS`, or a per-mode
  wrapper build pattern. Then update the validation commands to the real interface.

### R-003 `The assert() fix is proposed, but the plan still never includes it into microbench`

- Severity: HIGH
- Section: `Implement / Phase 7`
- Type: Correctness
- Problem:
  The plan correctly notices that removing `am.h` / `klib-macros.h` leaves
  `microbench` without `assert()`, and it proposes a new `xlib/include/assert.h`.
  But the actual "AFTER" include block for `benchmark.h` does not include
  `<assert.h>`, and the "AFTER" include block for `bench.c` keeps only
  `<benchmark.h>` and `<limits.h>`. Several microbench sources still call
  `assert()`.
- Why it matters:
  This means the plan's own proposed diffs still leave microbench uncompilable after
  the NJU headers are removed. The compatibility surface is still incomplete.
- Recommendation:
  Make `<assert.h>` part of the actual planned include path, or add an explicit local
  assert shim to `benchmark.h` so every C and C++ microbench source sees it.

### R-004 `The new xlib malloc/free surface widens the round without a current consumer`

- Severity: MEDIUM
- Section: `Goals / Phase 3 / Trade-offs`
- Type: Maintainability
- Problem:
  The plan elevates `malloc/free` in xlib to a primary goal, but the same plan also
  states that the current benchmarks either do not use malloc or use their own
  allocators. CoreMark stays `MEM_STATIC`; Dhrystone uses `myalloc`; MicroBench uses
  `bench_alloc`.
- Why it matters:
  This is not a correctness blocker, but it widens the implementation and validation
  surface of `xlib` without helping the round clear its current blockers.
- Recommendation:
  Either justify `malloc/free` with a concrete round-01 consumer and validation path,
  or defer it to a later round while still responding to M-002 explicitly.

---

## Trade-off Advice

### TR-1 `Prefer target-local mainargs plumbing over a new xam-wide build variable`

- Related Plan Item: `Phase 7 / Phase 8`
- Topic: Simplicity vs Build-System Scope
- Reviewer Position: Prefer Option A
- Advice:
  Wire microbench's mode selection through a target-local `mainargs` definition and
  `CFLAGS` injection, similar to the existing `am-tests` pattern, instead of
  teaching the whole xam build system about a new `MAINARGS` variable.
- Rationale:
  The build-system surface is currently small and predictable. A local pattern solves
  the real problem without widening the round into a general xam Makefile redesign.
- Required Action:
  Formalize a target-local `mainargs` mechanism in the next plan.

### TR-2 `Treat xlib malloc as optional unless this round actually needs it`

- Related Plan Item: `T-3`
- Topic: Future Flexibility vs Current Scope
- Reviewer Position: Need More Justification
- Advice:
  Keep `malloc/free` only if the next plan can show why round `01` needs it now.
- Rationale:
  Adding allocator API to shared `xlib` is a larger surface-area change than the rest
  of the benchmark port, and the current benchmark set still does not consume it in
  the acceptance path.
- Required Action:
  Either justify the allocator as part of round `01`, or move it to future work with
  an explicit response to M-002.

---

## Positive Notes

- The heap-symbol redesign is the right response to round `00`'s blocking issue.
- The CoreMark plan is much tighter now that it uses `MAIN_HAS_NOARGC=1` instead of
  promising a vague `main()` rewrite.
- The per-file NJU→xam migration tables are materially more useful than the previous
  round's high-level bullets.

---

## Approval Conditions

### Must Fix
- R-001
- R-002
- R-003

### Should Improve
- R-004

### Trade-off Responses Required
- TR-1
- TR-2

### Ready for Implementation
- No
- Reason: the round is more detailed than `00_PLAN`, but the benchmark build
  contract, microbench argument plumbing, and microbench assert path are still
  incomplete in ways that would block implementation as written.
