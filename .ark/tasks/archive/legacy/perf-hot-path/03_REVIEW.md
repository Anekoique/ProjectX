# `hotPath` REVIEW `03`

> Status: Open
> Feature: `hotPath`
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

- Decision: Approved with Revisions
- Blocking Issues: 2
- Non-Blocking Issues: 2

## Summary

Round 03 materially improves round 02. It restores the missing SMC gate,
removes the reduced-bundle escape hatch, and replaces the unavailable
`cargo-asm` dependency with evidence that can actually be produced in this
repo.

The remaining blockers are both inside the newly restored P4 SMC gate.
First, the plan wires the new letter `m` only at the Makefile layer, but
the am-test harness is dispatch-driven from `src/main.c` and `include/amtest.h`;
without updating those files, the test can pass vacuously. Second, the
proposed `smc.c` body uses `x1`/`ra` as the observed register, which is not a
sound C-level contract for a self-modifying-code test. Until those are fixed,
the restored gate is not reliable enough to unblock implementation.

---

## Findings

### R-001 `The restored SMC gate is not fully wired into the am-test harness and can pass vacuously`

- Severity: HIGH
- Section: `Log`, `Implementation Plan / Phase 2a`, `Validation V-IT-1`, `Exit Gate §A P4`
- Type: Correctness | Validation
- Problem:
  Round 03 restores `smc.c` and adds letter `m` to the am-tests `Makefile`
  ([docs/perf/hotPath/03_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/03_PLAN.md:68),
  [03_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/03_PLAN.md:572)),
  but it never updates the actual test dispatch layer:

  - `xkernels/tests/am-tests/include/amtest.h` declares every test entrypoint
    and currently has no `test_smc(void)`.
  - `xkernels/tests/am-tests/src/main.c` owns the `MAINARGS` switch table and
    currently has no `CASE('m', test_smc)`, no description row, and no SMC
    mention anywhere in the selector table
    ([main.c](/Users/anekoique/ProjectX/xkernels/tests/am-tests/src/main.c:14),
    [main.c](/Users/anekoique/ProjectX/xkernels/tests/am-tests/src/main.c:34)).

  That omission is not just incomplete wiring. It creates a false-positive
  pass mode: when `MAINARGS="m"` falls into `main()`'s default branch, the
  program prints usage and returns `0`
  ([main.c](/Users/anekoique/ProjectX/xkernels/tests/am-tests/src/main.c:58),
  [main.c](/Users/anekoique/ProjectX/xkernels/tests/am-tests/src/main.c:66)).
  The xam runtime turns `main`'s return value into `terminate(ret)`
  ([xam/xhal/src/platform/xemu/mod.rs](/Users/anekoique/ProjectX/xam/xhal/src/platform/xemu/mod.rs:16),
  [misc.rs](/Users/anekoique/ProjectX/xam/xhal/src/platform/xemu/misc.rs:2)),
  which yields `HIT GOOD TRAP` for return code `0`
  ([xemu/xcore/src/cpu/mod.rs](/Users/anekoique/ProjectX/xemu/xcore/src/cpu/mod.rs:343)).
  The runner only greps for `GOOD TRAP`
  ([xkernels/tests/am-tests/Makefile](/Users/anekoique/ProjectX/xkernels/tests/am-tests/Makefile:36)),
  so an unwired `m` target can report PASS without ever executing `test_smc()`.
- Why it matters:
  This is the load-bearing validation fix in round 03. If the harness wiring
  is incomplete, the restored P4 gate is not actually restored.
- Recommendation:
  In the next PLAN, make Phase 2a explicitly update all three places:
  1. add `void test_smc(void);` to
     [amtest.h](/Users/anekoique/ProjectX/xkernels/tests/am-tests/include/amtest.h);
  2. add `CASE('m', test_smc);` and a description entry to
     [main.c](/Users/anekoique/ProjectX/xkernels/tests/am-tests/src/main.c);
  3. keep the Makefile `ALL` / `name` edits.
  Also state explicitly that this wiring is required to prevent vacuous
  `GOOD TRAP` passes from the default help path.

### R-002 `The proposed smc.c body is not sound as written because it uses x1/ra as the observed result`

- Severity: HIGH
- Section: `Log`, `Implementation Plan / Phase 2a`, `Validation V-IT-1`
- Type: Correctness | Design Soundness
- Problem:
  The round describes `smc.c` as writing `addi x1, x0, 0`, then
  `addi x1, x0, 42`, executing the RAM buffer, and checking `x1 == 42`
  in C
  ([03_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/03_PLAN.md:71),
  [03_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/03_PLAN.md:573),
  [03_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/03_PLAN.md:796)).
  `x1` is `ra`, the return-address register. That makes the sketch unsound:

  - If the RAM code is called like a normal function, `ra` is part of the
    call/return protocol and is not a stable post-call observation channel.
  - If the RAM code does not return normally, the plan does not specify how
    control gets back to the C test to run `check(...)`.
  - The plan never explains how C will read `x1` after execution.

  In other words, the test intent is valid, but the chosen observable is not.
- Why it matters:
  This is the actual semantics check for the restored gate. A test that is
  not mechanically executable as described is not an implementation-ready gate.
- Recommendation:
  Redesign the SMC test around an ABI-visible or memory-visible result.
  Preferred shapes:
  1. write a tiny RAM buffer equivalent to `addi a0, x0, 42; ret` and call it
     as a function pointer, then `check(ret == 42)` in C; or
  2. write a tiny RAM buffer that stores to a known memory slot, then verify
     that slot after execution.
  The next PLAN should name the exact instruction sequence and how control
  returns to the harness.

### R-003 `The validation block still does not preserve the repo-mandated make-run and make-test checks`

- Severity: MEDIUM
- Section: `Summary`, `Constraints C-2/C-4`, `Validation`, `Exit Gate`
- Type: Spec Alignment | Validation
- Problem:
  Round 03 correctly rewrites the workdirs, but it still replaces the
  repo-mandated `make run` / `make test` checks instead of mapping them to
  executable equivalents. `AGENTS.md` still says coding changes must run
  `make fmt`, `make clippy`, `make run`, and `make test`
  ([AGENTS.md](/Users/anekoique/ProjectX/AGENTS.md:8)).

  The current mandatory block includes `make -C xemu fmt` and
  `make -C xemu clippy`, but it omits the direct equivalents
  `make -C xemu run` and `make -C xemu test`, even though those targets do
  exist
  ([xemu/Makefile](/Users/anekoique/ProjectX/xemu/Makefile:44),
  [xemu/Makefile](/Users/anekoique/ProjectX/xemu/Makefile:53)).
  Instead it substitutes `cargo test --workspace`, perf scripts, and OS boots.
- Why it matters:
  The rewritten block is much better than round 02, but it still does not
  fully satisfy the repository’s stated verification contract.
- Recommendation:
  In the next PLAN, keep the stronger extra checks, but also add the direct
  xemu equivalents:
  `make -C xemu run` and `make -C xemu test`.
  If the intent is to supersede them, that needs an explicit MASTER waiver or
  an AGENTS update rather than silent substitution.

### R-004 `Acceptance Mapping still contains an impossible clippy command`

- Severity: MEDIUM
- Section: `Acceptance Mapping`
- Type: Validation | Maintainability
- Problem:
  The acceptance table still lists
  ``make -C xemu clippy --all-targets`` as the proof for C-9
  ([03_PLAN.md](/Users/anekoique/ProjectX/docs/perf/hotPath/03_PLAN.md:859)).
  That is not a valid invocation shape: `--all-targets` is a Cargo flag, not a
  Make flag, and `xemu/Makefile`'s `clippy` target does not forward extra args.
- Why it matters:
  This leaves one of the validation rows non-runnable even after round 03’s
  command-block rewrite.
- Recommendation:
  Replace that row with either:
  1. `cd xemu && cargo clippy --all-targets`; or
  2. `make -C xemu clippy` if `--all-targets` is not actually required.
  The acceptance mapping should match the executable gate text exactly.

---

## Trade-off Advice

### TR-1 `Prefer an ABI-visible SMC result over direct register poking`

- Related Plan Item: `Phase 2a`, `V-IT-1`
- Topic: Simplicity vs Safety
- Reviewer Position: Prefer Option A
- Advice:
  Use a tiny RAM-resident function that returns through `a0` or writes to a
  known memory slot. Do not design the test around `x1` / `ra`.
- Rationale:
  `a0` or a memory slot gives the harness a normal C-level assertion surface.
  `ra` does not. This keeps the test simple and avoids fragile inline-asm
  scaffolding.
- Required Action:
  Rewrite the next PLAN’s `smc.c` sketch around a concrete ABI-visible or
  memory-visible result channel.

### TR-2 `Prefer explicit xemu make-target coverage over cargo-only substitution`

- Related Plan Item: `Validation`, `Exit Gate`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option B
- Advice:
  Keep the exact workdirs, but include the direct `xemu` make targets as part
  of the mandatory block instead of replacing them entirely with Cargo and perf
  scripts.
- Rationale:
  The repo contract is written in terms of `make`. Translating that into exact
  subdir invocations is good; dropping the `run` / `test` make targets
  altogether is still a semantic change.
- Required Action:
  Add `make -C xemu run` and `make -C xemu test` in the next PLAN, or
  explicitly justify/waive their replacement.

---

## Positive Notes

- The round-03 response to round-02 R-003/R-004 is good: removing the
  `cargo-asm` gate and deleting the reduced-bundle escape hatch both improve
  plan executability.
- The round still keeps the stronger round-02 technical simplification: the
  `(pc, raw)` cache remains the right shape for this codebase.
- The rewritten workdir-qualified command block is a substantial improvement
  over round 02, even though it still needs one more pass to align fully with
  `AGENTS.md`.

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
- Reason: the round-03 architectural direction is sound, but the restored SMC
  gate is not yet implementation-ready because its harness wiring is incomplete
  and its proposed test body is not executable as written.
