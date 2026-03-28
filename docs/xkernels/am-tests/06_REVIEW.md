# `am-tests` REVIEW `06`

> Status: Open
> Feature: `am-tests`
> Iteration: `06`
> Owner: Reviewer
> Target Plan: `06_PLAN.md`
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
- Blocking Issues: `1`
- Non-Blocking Issues: `2`

## Summary

`06_PLAN` is much closer to implementation than the previous rounds.

The conditional-`main()` structure fixes the old wrapper-name mismatch cleanly, the Rust 2024 `unsafe extern` issue is resolved, and weakening the run-all isolation claim is the right response to the shared-state concern from round 05. The plan is now coherent on the C-side build model and on the trap ABI itself.

One blocking issue remains, though: the new `trap.S` integration is still not actually implementable through the current `xhal` Cargo build path as documented. The round separates the assembly file correctly, but the proposed `build.rs` change still does not compile or link that file into `libxhal`.

Aside from that blocker, there are two medium issues worth tightening:

- run-all mode is now truthfully documented as shared-state, but the plan still does not make the ordering dependency explicit enough for future maintenance
- `run-menu` is now a failing target, but it is still only a local/manual validation path, not part of the gated contract

So this round is near-ready, but not yet ready for implementation approval.

---

## Findings

### R-001 `trap.S is still not actually linked into xhal`

- Severity: HIGH
- Section: `Implement / xhal build.rs / Architecture`
- Type: Flow
- Problem:
  `06_PLAN` moves trap entry into `xhal/src/platform/xemu/trap.S`, which is a good cleanup, but the documented integration step is still only:

  `println!("cargo:rerun-if-changed=src/platform/xemu/trap.S");`

  In Cargo, `rerun-if-changed` only tells the build script when to rerun. It does not assemble the file or add it to the Rust archive. The current local `xhal/build.rs` has no native build step, and `xhal/Cargo.toml` has no `cc` build-dependency or equivalent mechanism that would actually compile `trap.S` into `libxhal`.
- Why it matters:
  This is now the main remaining implementation blocker. The plan says `M-001` is applied, but the proposed changes still leave `__am_trap_entry` undefined at link time. As written, the separated `trap.S` path is not buildable.
- Recommendation:
  The next PLAN must define a real native-build path for `trap.S`, for example:
  - add a build dependency such as `cc`
  - compile `src/platform/xemu/trap.S` from `build.rs`
  - ensure the assembly object is linked only for the relevant target/platform combination

  A `rerun-if-changed` line can stay, but it is not sufficient by itself.

### R-002 `Run-all mode now depends on test ordering, but that dependency is still implicit`

- Severity: MEDIUM
- Section: `Invariants / src/main.c / amtest.h`
- Type: Invariant
- Problem:
  The round correctly weakens `I-1` and admits that run-all mode shares machine state. However, once that is true, the success of `src/main.c` depends on the specific sequence in `tests[]`. For example, `timer-irq` and `soft-irq` leave interrupt enables modified, and `csr-warl` rewrites `mtvec` before `trap-ecall` installs a fresh handler again.
- Why it matters:
  The current order may still be safe, but the plan no longer has a strong “tests are independent” invariant to protect future edits. A later reorder in `amtest.h` can silently turn the run-all binary from a smoke test into an order-sensitive failure.
- Recommendation:
  The next PLAN should either:
  - explicitly state that run-all correctness depends on the current test order in `tests[]`; or
  - add a minimal reset/restore discipline around trap and interrupt state between run-all invocations

### R-003 `run-menu validation is still non-gated despite being listed as an acceptance item`

- Severity: MEDIUM
- Section: `Build modes / Makefile / Validation`
- Type: Validation
- Problem:
  `run-menu` now fails correctly on missing `ALL PASSED`, which fixes the previous correctness issue. But `V-IT-8` still reads like a normal acceptance item even though the run-all binary is not part of the CI-gated path. The only gated contract remains `make run`, while `run-menu` is still effectively a manual smoke check.
- Why it matters:
  This is mostly a contract-precision issue. The round is better than 05, but the validation table still gives the one-binary path more weight than the actual automation guarantees.
- Recommendation:
  The next PLAN should either:
  - label `V-IT-8` explicitly as a manual smoke validation; or
  - add `run-menu` to the automated validation flow if it is meant to be part of implementation acceptance

---

## Trade-off Advice

### TR-1 `Keep the single-file conditional-main design`

- Related Plan Item: `G-8`
- Topic: Simplicity vs Build Indirection
- Reviewer Position: Prefer Option A
- Advice:
  The new `#ifndef AM_MENU` structure is cleaner than the old wrapper-directory approach and should be kept.
- Rationale:
  It removes the naming indirection that caused the last round's mismatch and makes the per-test/menu dual build model obvious from the test file itself.
- Required Action:
  Keep this design in the next round; do not reintroduce standalone wrappers just to fix `trap.S` integration.

### TR-2 `Treat run-all as a smoke path unless you are willing to pay the reset cost`

- Related Plan Item: `G-10`
- Topic: Convenience vs Strong Isolation
- Reviewer Position: Prefer Option A
- Advice:
  If the project does not want extra HAL/reset surface, keep run-all as a documented shared-state smoke binary rather than pretending it is equivalent to the per-test gated path.
- Rationale:
  The per-test runner already provides the strong correctness signal. Making run-all fully isolated would add more mechanics and state-reset rules than this feature currently needs.
- Required Action:
  The next PLAN should either formalize run-all as smoke-only or specify the exact reset sequence that upgrades it to a stronger acceptance path.

---

## Positive Notes

- Removing `standalone/` and using `#ifndef AM_MENU` is a real improvement. It resolves the previous naming problem without adding a new build indirection layer.
- The Rust 2024 FFI issue is fixed correctly with `unsafe extern "C"`.
- Separating trap assembly from `trap.rs` is the right cleanup direction once the build integration is made real.
- The round now states the shared-state reality of run-all mode much more honestly than round 05.

---

## Approval Conditions

### Must Fix
- R-001

### Should Improve
- R-002
- R-003

### Trade-off Responses Required
- TR-1
- TR-2

### Ready for Implementation
- No
- Reason: the round is close, but the separated `trap.S` path still is not actually buildable through the current `xhal` Cargo build pipeline as documented.
