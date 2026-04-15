# `am-tests` REVIEW `03`

> Status: Open
> Feature: `am-tests`
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
- Blocking Issues: `2`
- Non-Blocking Issues: `2`

## Summary

`03_PLAN` is a clear improvement over `02_PLAN`.

The round fixes several real problems in the right direction:

- it restores machine-checkable UART output assertions for `uart-putc`
- it finally routes console output through the existing `_putch` boundary instead of inventing a second console API
- it cleans up the trap design by borrowing more of the local `xark-core` structure
- it removes the swallowed `|| true` from the local `build-optional` rule

Those are meaningful corrections.

However, the plan still should not be approved yet.

Two blocking issues remain:

- the new menu-binary design is still not coherent with the per-test build model and does not actually satisfy the “choose a test to run” directive as currently written
- the trap ABI is still not truly finalized, because the published `TrapFrame` claims fields that the documented entry code still does not actually populate correctly

There are also two medium issues worth tightening:

- the CI contract still overstates optional compile coverage, because the workflow explicitly allows that step to fail without affecting the run
- the timer goal still claims `mtimecmp` read/write validation, but the current HAL and tests only show writes, not read-back validation

---

## Findings

### R-001 `Menu mode is still underdefined and currently unbuildable as written`

- Severity: HIGH
- Section: `Summary / Architecture / src/main.c / Makefile`
- Type: Flow
- Problem:
  `03_PLAN` says `run-menu` builds `src/main.c` together with all `tests/*.c`, and it also says each test file remains both a standalone `main()` program and a `test_xxx()` function. As written, that creates multiple `main` definitions in one binary. The sample `main.c` also does not actually implement selection logic; it prints a menu and then unconditionally runs all tests sequentially. In addition, `run-menu` still relies on `EXTRA_SRCS`, but the required `xam` build-surface support is no longer carried as an explicit change item in this round.
- Why it matters:
  This leaves `G-10` and `M-003` unresolved in practice. The current design either fails to link or falls short of the stated manual dispatch behavior. Because `M-003` is a binding master directive, “prints a menu, then runs all” is not equivalent to “choose a test to run.”
- Recommendation:
  The next PLAN should define one coherent menu-build model, for example:
  - split shared test bodies from standalone per-test wrappers, so menu mode links only the shared bodies; or
  - compile out the per-file `main()` functions under a menu-build macro

  It should also show actual selection behavior in `main.c`, not just a printed menu followed by unconditional “run all”.

### R-002 `TrapFrame still does not match the documented trap entry semantics`

- Severity: HIGH
- Section: `Summary / Data Structure / Trap entry assembly / Constraints`
- Type: API
- Problem:
  The round says the trap ABI is now final, but the documented entry path still conflicts with the published `TrapFrame`:
  - `GeneralRegs` includes `zero` and `sp`
  - the assembly explicitly says slot 0 is unused
  - the assembly also explicitly says the pre-trap `sp` is not saved in the `sp` slot

  So the public C-visible `TrapFrame` still presents fields that are not actually populated with the architectural values their names imply. This also does not really match the cited `xark-core` pattern, where the trap path explicitly stores the original stack pointer via scratch-register handling.
- Why it matters:
  This is exactly the class of ABI mismatch raised in the previous round, just with named fields instead of an array. Once `init_trap()` becomes a reusable `xam` HAL surface, future tests or higher-level code will treat `TrapFrame.sp` and `TrapFrame.zero` as real saved state. If those fields are garbage or synthetic, the interface is still misleading and not safe to build on.
- Recommendation:
  The next PLAN must choose one truthful ABI and document it precisely:
  - either save real values for every published field, including a defined value for `zero` and the original `sp`; or
  - remove / repurpose fields that are not truly saved and stop claiming the layout is final

  It should also stop saying this exactly matches the `xark-core` pattern unless the stack-pointer handling really does.

### R-003 `CI still overstates optional compile coverage`

- Severity: MEDIUM
- Section: `Log / CI / Validation`
- Type: Validation
- Problem:
  The local `build-optional` rule now propagates failures, which is good, but the proposed CI step uses `continue-on-error: true`. According to GitHub’s workflow semantics, that allows the job to pass even when the step fails. So `V-IT-9: CI passes` is not evidence that optional builds succeeded, only that the workflow did not treat them as gating.
- Why it matters:
  This is mostly a contract-precision problem. The plan is close, but it still blends “compile-checked” with “non-gating” in a way that can mislead later reviewers about what a green CI result really guarantees.
- Recommendation:
  The next PLAN should either:
  - narrow the wording to “optional builds are attempted and reported, but non-gating”; or
  - add an explicit follow-up status check on the step outcome if compile-checked status is meant to remain part of acceptance

### R-004 `Timer validation still overclaims mtimecmp coverage`

- Severity: MEDIUM
- Section: `Goals / Validation / tests/timer-read.c`
- Type: Validation
- Problem:
  `G-2` still says “mtime advances, mtimecmp read/write,” but the documented timer HAL exposes `mtime()` and `set_mtimecmp()` only, and `tests/timer-read.c` only checks that `mtime` advances. The round no longer documents a `mtimecmp` read-back path or a test that verifies compare-register visibility directly.
- Why it matters:
  This makes the acceptance contract looser than the stated goal. The plan may still be sufficient for functional interrupt validation, but it should not keep advertising `mtimecmp` read/write validation unless there is an actual read-back contract and corresponding test.
- Recommendation:
  The next PLAN should either:
  - narrow `G-2` to what the tests really prove; or
  - add a concrete `mtimecmp` read-back helper or raw-MMIO read path and validate it explicitly

---

## Trade-off Advice

### TR-1 `Prefer shared test bodies plus separate wrappers over dual-purpose source files`

- Related Plan Item: `G-10`
- Topic: Simplicity vs Build Hygiene
- Reviewer Position: Prefer Option A
- Advice:
  If the project wants both per-test standalone binaries and a menu binary, the cleanest design is shared test bodies with thin standalone wrappers, not the same source file defining both a test function and a `main()`.
- Rationale:
  That structure keeps the linker model obvious, avoids preprocessor tricks around duplicate `main` definitions, and makes the menu binary a first-class build target instead of a special-case workaround.
- Required Action:
  The next PLAN should either adopt this split or justify a different mechanism that still yields one unambiguous `main` at link time.

### TR-2 `If TrapFrame is public, make every published field truthful`

- Related Plan Item: `G-9`
- Topic: Rich ABI vs Safe ABI
- Reviewer Position: Prefer Option A
- Advice:
  Keep the trap ABI as small and truthful as possible. If a field is published in `TrapFrame`, the documented entry path should save that exact value.
- Rationale:
  A slightly smaller but precise trap ABI is much safer than a richer-looking structure with synthetic or undefined slots. Public trap state becomes part of the platform contract very quickly.
- Required Action:
  The next PLAN should either save `zero` / original `sp` exactly, or narrow the public frame to the data it really provides.

---

## Positive Notes

- Restoring explicit UART output assertions is the right fix for the last round’s biggest validation gap.
- Wiring console output through `_putch` is a much cleaner platform boundary than the separate `putc` / `puts` API from round 02.
- Borrowing the local `xark-core` structure improves the direction of the trap design, even though the ABI details still need one more tightening pass.
- Removing the swallowed failure in the local optional-build rule is an actual improvement.

---

## Approval Conditions

### Must Fix
- R-001
- R-002

### Should Improve
- R-003
- R-004

### Trade-off Responses Required
- T-1

### Ready for Implementation
- No
- Reason: the menu binary design is still inconsistent with the per-test build model, and the published `TrapFrame` still does not match the documented trap-entry semantics.
