# `am-tests` REVIEW `05`

> Status: Open
> Feature: `am-tests`
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
- Blocking Issues: `2`
- Non-Blocking Issues: `2`

## Summary

`05_PLAN` is the first round that actually resolves the two blockers from `04_REVIEW`.

The rename split does remove the old basename collision in principle, and the trap-entry sequence now saves `t0` before reusing it as scratch. Narrowing the menu goal to run-all is also the right direction.

However, the plan still is not ready for implementation.

Two blocking issues remain:

- the wrapper naming contract is still inconsistent between the documented file layout and the Makefile expansion, so the core per-test runner still does not point at the files the plan says will exist
- the new `trap.rs` snippet does not compile under the current `xam/xhal` Rust 2024 edition because the extern block is not marked `unsafe`

There are also two medium issues worth tightening:

- `run-menu` now prints pass/fail status, but it still does not fail the target when the run-all binary fails
- the plan still overstates test isolation in run-all mode even though several tests mutate global machine state and do not restore it

---

## Findings

### R-001 `Wrapper filenames still do not match the per-test Makefile expansion`

- Severity: HIGH
- Section: `Architecture / Implement / Makefile`
- Type: Flow
- Problem:
  `05_PLAN` says the standalone wrappers are named `run_uart_putc.c`, `run_timer_read.c`, `run_timer_irq.c`, and so on, but the Makefile derives wrapper paths from the hyphenated test basenames: `K = standalone/run_$*.c` with `$*` coming from `tests/uart-putc.c`, `tests/timer-read.c`, etc. That expands to paths such as `standalone/run_uart-putc.c` and `standalone/run_timer-read.c`, which are not the files the plan documents.
- Why it matters:
  This keeps the main `make run` path inconsistent on paper. The old object-collision problem is fixed, but the runner still cannot be implemented exactly as written unless the naming scheme is made consistent end to end.
- Recommendation:
  The next PLAN should choose one naming convention and use it everywhere:
  - either rename the wrapper files to the hyphenated form implied by `$*`; or
  - introduce an explicit mapping from test name to wrapper filename instead of deriving wrapper paths mechanically

### R-002 `The proposed trap.rs does not compile in xam's Rust 2024 edition`

- Severity: HIGH
- Section: `Implement / xam/xhal/src/platform/xemu/trap.rs`
- Type: API
- Problem:
  The new `trap.rs` snippet declares `extern "C" { fn __am_trap_entry(); }`, but the current `xam/xhal` crate is on Rust 2024, where extern blocks must be written as `unsafe extern "C" { ... }`. This is a compile error in the exact code the plan proposes.
- Why it matters:
  This is now a direct implementation blocker. The round fixes the trap save-order bug, but the new HAL surface still cannot compile as written, so `G-9` is not yet implementable.
- Recommendation:
  The next PLAN must update the declaration to the Rust 2024 form and keep the rest of the FFI surface consistent with the current `xhal` edition and style.

### R-003 `run-menu still is not a failing validation target`

- Severity: MEDIUM
- Section: `Makefile / Validation`
- Type: Validation
- Problem:
  `run-menu` now captures output and checks for `ALL PASSED`, but the recipe ends with `... && echo "run-menu: OK" || echo "run-menu: FAIL"`. That means the target still exits successfully even when the binary times out or the grep fails, because the final `echo` returns zero.
- Why it matters:
  `V-IT-8` describes menu mode as a validated path, but the target is still only advisory. A broken run-all binary can print `run-menu: FAIL` and still leave the make target green.
- Recommendation:
  The next PLAN should make `run-menu` fail the recipe on timeout or missing success marker, for example by ending the line with `test $$? -eq 0` style status propagation or an explicit `exit 1` on failure.

### R-004 `Run-all mode still overclaims inter-test isolation`

- Severity: MEDIUM
- Section: `Invariants / Test Source Code / Run-all binary`
- Type: Invariant
- Problem:
  `I-1` says tests are standalone with no inter-test dependencies, but the documented run-all binary executes them sequentially in one machine context while several tests mutate global state and do not restore it. `timer-irq` and `soft-irq` leave interrupt-enable state changed, `csr-warl` rewrites `mie` and `mtvec`, and `init_trap()` replaces the global handler pointer each time.
- Why it matters:
  The current test order may still work, but the invariant is not truthful. Future reordering or adding a new test between these cases can introduce order-dependent failures that the plan currently says should not exist.
- Recommendation:
  The next PLAN should either:
  - weaken `I-1` for run-all mode and document that tests share machine state; or
  - add explicit reset / restore steps so run-all mode really preserves inter-test independence

---

## Trade-off Advice

### TR-1 `Prefer one canonical test-name scheme over derived filename tricks`

- Related Plan Item: `G-8`
- Topic: Simplicity vs Build Hygiene
- Reviewer Position: Prefer Option A
- Advice:
  Pick one canonical identifier form for each test and use it consistently across filenames, make targets, and user-facing names.
- Rationale:
  Round 05 removes the object collision, but it also shows how easy it is for hyphenated target names and underscored wrapper files to drift apart. A single naming scheme is simpler than carrying silent transformations through the build.
- Required Action:
  The next PLAN should either normalize all filenames to the make-target form or add an explicit mapping table and justify that extra indirection.

### TR-2 `Prefer truthful shared-state documentation over a stronger but false isolation invariant`

- Related Plan Item: `G-10`
- Topic: Simplicity vs Strict Isolation
- Reviewer Position: Prefer Option A
- Advice:
  If the run-all binary is meant as a convenience path, document the shared-state reality instead of claiming full test independence unless the plan is willing to pay the reset cost.
- Rationale:
  Resetting trap and interrupt state between tests is possible, but it adds more HAL surface and more moving pieces. A weaker but truthful invariant is better than an isolation guarantee the design does not currently enforce.
- Required Action:
  The next PLAN should either relax `I-1` for run-all mode or specify the exact reset sequence that restores independence.

---

## Positive Notes

- The old basename-collision blocker from round 04 is resolved in principle by giving wrappers distinct basenames.
- The trap-entry save order is materially better: `t0` is now saved before scratch use, which fixes the last round's real corruption bug.
- Narrowing the menu goal from “selection” to “run all” is the right simplification for the current xam/xemu surface.
- The optional-build CI wording is now aligned with what `continue-on-error` actually guarantees.

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
- Reason: the per-test runner still has a wrapper-name contract mismatch, and the proposed `trap.rs` FFI block does not compile in the current Rust 2024 `xhal` crate.
