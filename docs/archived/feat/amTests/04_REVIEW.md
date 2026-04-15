# `am-tests` REVIEW `04`

> Status: Open
> Feature: `am-tests`
> Iteration: `04`
> Owner: Reviewer
> Target Plan: `04_PLAN.md`
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

`04_PLAN` is materially better than `03_PLAN`.

The round finally narrows the timer goal to what the tests can really prove, keeps the `_putch` boundary instead of adding another console ABI, and uses the right structural fix for the old multi-`main` problem by splitting shared test bodies from standalone wrappers.

However, the plan still should not be approved.

Two blocking issues remain:

- the proposed `EXTRA_SRCS` build-surface change still collapses `standalone/X.c` and `tests/X.c` into the same object path, so the primary per-test runner is not actually buildable as written
- the published trap ABI is still not truthful, because the entry sequence overwrites `t0` before saving it and then restores the wrong value on `mret`

There are also two medium issues to tighten:

- the plan still promises a selectable test menu, but the documented `main.c` only prints choices and then runs everything
- the menu-mode validation is still described as a validated path without machine-checkable pass/fail criteria

---

## Findings

### R-001 `Per-test multi-source builds still collide on object names`

- Severity: HIGH
- Section: `Summary / Architecture / Makefile / xam build_c.mk`
- Type: Flow
- Problem:
  `04_PLAN` resolves the source-layout side of the old multi-`main` problem, but the new `xam` build contract is still incorrect for the actual `Run.%` shape. The runner builds `K = standalone/$*.c` and `EXTRA_SRCS = tests/$*.c`, while the current `xam` kernel path derives `KERNEL_NAME := $(basename $(notdir $(KERNEL)))`. The proposed `EXTRA_OBJS = $(addprefix $(OUT_DIR)/, $(notdir ...))` therefore maps both sources to the same object path `$(OUT_DIR)/$*.o`. With `VPATH` still seeded from `KERNEL_DIR`, both object entries resolve through the same basename and the test body is not a distinct object.
- Why it matters:
  This breaks the core CI path the plan is trying to approve. The per-test build either compiles the wrapper twice or fails to link because `test_$*()` is never provided from a distinct `tests/$*.c` object. So `R-001` from the previous round is not actually resolved yet.
- Recommendation:
  The next PLAN must define a path-preserving multi-source build contract. For example, object names should keep enough directory structure to distinguish `standalone/uart-putc.c` from `tests/uart-putc.c`, or the build surface should accept an explicit object list instead of deriving everything from `notdir`.

### R-002 `Trap entry still clobbers t0 before saving the interrupted register state`

- Severity: HIGH
- Section: `Summary / TrapFrame ABI / Trap entry assembly`
- Type: Correctness
- Problem:
  The round claims every published `TrapFrame` field is truthful, but the documented assembly still reuses `t0` before the original `x5/t0` value is saved. The sequence computes `addi t0, sp, FRAME_SIZE`, stores that value as the saved `sp`, and only then performs `STORE x5, 5`. At that point, `x5` no longer holds the interrupted `t0`; it holds the synthetic original-`sp` value. The return path then restores `x5` from that corrupted slot.
- Why it matters:
  This is a real trap-corruption bug, not just a documentation mismatch. Any interrupted code that had live state in `t0` can resume with the wrong register value. It also means the plan still does not satisfy its own “every field truthful” invariant, and it does not match the local `xark-core` save-first pattern it cites as inspiration.
- Recommendation:
  The next PLAN must save the interrupted `t0` before reusing it, or use a different scratch mechanism for recovering the original `sp` value. The save/restore order needs to be documented precisely enough that `TrapFrame.t0` and the resumed machine state are both correct.

### R-003 `The menu goal still promises selection, but the plan only implements run-all`

- Severity: MEDIUM
- Section: `Goals / Architecture / src/main.c / Validation`
- Type: Flow
- Problem:
  `G-10` and the architecture text still describe `main.c` as “selection + run-all” and “select by index or run all”, but the sample `src/main.c` only prints the test list and then unconditionally runs every test. The round does not define any input HAL, UART RX contract, or runner-side selector to make actual test selection possible.
- Why it matters:
  This leaves the plan internally inconsistent. If the real intent is a manual smoke binary that always runs all tests, the goal should say that. If test selection is still required, the execution path and validation need to be specified.
- Recommendation:
  The next PLAN should either narrow `G-10` to “single binary that lists tests and runs all”, or add a concrete input/selection contract and corresponding validation for choosing one test.

### R-004 `Menu-mode validation is still not machine-checkable`

- Severity: MEDIUM
- Section: `Makefile / Validation`
- Type: Validation
- Problem:
  `V-IT-8` treats `run-menu` as a validated path, but the documented target only runs the binary. It has no timeout, no asserted success marker such as `=== ALL PASSED ===`, and no explicit failure contract for hangs or partial output. In contrast, the per-test runner does define output-based acceptance.
- Why it matters:
  That makes the one-binary path weaker than the rest of the validation story. A future regression in the menu binary could still look “manually runnable” without satisfying any objective acceptance rule.
- Recommendation:
  The next PLAN should either narrow `V-IT-8` to a manual smoke check, or define concrete automation for menu mode with timeout and output assertions.

---

## Trade-off Advice

### TR-1 `Prefer path-preserving object mapping over basename-only extras`

- Related Plan Item: `G-8`
- Topic: Simplicity vs Build Hygiene
- Reviewer Position: Prefer Option A
- Advice:
  Keep the multi-source build surface honest by preserving source-path identity in object names.
- Rationale:
  The current basename-only shortcut looks small, but it breaks immediately on the exact wrapper/body split this feature now depends on. A slightly more explicit object mapping is safer and scales to helper `.S` files later.
- Required Action:
  The next PLAN should adopt a path-preserving object strategy or justify a different approach that still makes `standalone/X.c` and `tests/X.c` coexist reliably.

### TR-2 `Prefer narrowing menu scope over inventing half-specified input support`

- Related Plan Item: `G-10`
- Topic: Feature Scope vs Delivery Risk
- Reviewer Position: Prefer Option A
- Advice:
  If interactive test selection is not essential to validating CSR, ACLINT, PLIC, and UART, narrow the menu goal to a non-interactive run-all binary.
- Rationale:
  The current round has no defined input boundary in `xam`/`xlib`, so adding “selection” without a full input contract just creates another under-specified interface. A truthful run-all binary is better than a pseudo-menu that cannot actually select.
- Required Action:
  The next PLAN should either narrow the goal and validation accordingly, or specify the full input path and prove it.

---

## Positive Notes

- Splitting shared test bodies from standalone wrappers is the right structural direction for eliminating multi-`main` conflicts.
- Narrowing `G-2` to `mtime` read advancement fixes the previous overclaim around `mtimecmp` read-back.
- Keeping console output on the existing `_putch` hook is still the cleanest interface boundary for this stack.
- The plan is closer to the local `xark-core` trap structure than earlier rounds, even though the register-save order still needs one more correction.

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
- Reason: the primary per-test build path is still not actually implementable as documented, and the trap-entry design still corrupts saved register state.
