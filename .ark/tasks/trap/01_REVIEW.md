# `trap` REVIEW `01`

> Status: Open
> Feature: `trap`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved
- Blocking: 0
- Non-blocking: 3 (3 LOW)

## Summary

Iteration 01 lands every R-001..R-007 and TR-1..TR-4 disposition from 00_REVIEW with concrete spec / implementation changes. C-15 is now self-consistent and the framework-C-16 extension is captured in `## Log`; C-17 + Phase 3 step 3 wire the grep assertion V-IT-1 leans on; C-5b pins the restore set to `{sepc, sstatus}`; C-18 documents the OpenSBI-handoff UART invariant; `Cause` is extracted to `trap/cause.rs` (with `usize` inner type) and the host-gating hedge on V-UT-1 is demoted to a flagged fall-back rather than a deferral excuse; `pub(crate)` visibility on `trap_entry` is locked in by C-19; and the `sd zero, 0(sp)` instruction is now part of both C-2 and the Phase 1 assembly sketch. The `## Spec` block reads stand-alone (no "see 00_PLAN" or "as before") and the `trap.rs` → `trap/mod.rs` directory restructure is explicitly called out in Phase 1 step 1. Residual nits are cosmetic Makefile-sketch hygiene, a `pipefail`-on-`tee` exit-code-masking concern, and an offset-assert ergonomics note — all LOW. Recommend approval; proceed to EXECUTE under the iteration cap.

---

## Findings

### R-001 `Makefile sketch uses $(BIN) and re-passes -kernel, both inconsistent with the existing run target`

- **Severity:** LOW
- **Section:** `## Implementation` Phase 3 step 3 (Makefile sketch)
- **Problem:** The sketch is:
  ```
  trap-test:
  	cargo build --release --features trap-canary
  	$(QEMU_SYSTEM) $(QEMU_FLAGS) -kernel $(BIN) | tee /tmp/xvisor-trap.log
  	grep -E ... /tmp/xvisor-trap.log
  	grep -E ... /tmp/xvisor-trap.log
  ```
  Two minor inconsistencies with the on-disk `xvisor/Makefile` `run` target (which the sketch claims to mirror): (a) the existing Make variable is `$(OUT_BIN)`, not `$(BIN)`; (b) `$(QEMU_FLAGS)` already terminates with `-kernel $(OUT_BIN)`, so appending another `-kernel $(BIN)` double-passes the flag. The annotation "(variable names match the existing `run` target.)" is therefore inaccurate as written. Not a blocker — the executor will see this immediately when the rule fails to build — but the spec sketch should match what will actually go in.
- **Why it matters:** Iteration cap is `max_iterations = 3`; the iteration after EXECUTE is consumed by VERIFY. If the executor follows the literal sketch and `make trap-test` then fails on a typo, the only remediation path is a corrective commit, not another plan loop. The fix is trivial and worth absorbing here so it doesn't surface in VERIFY.
- **Recommendation:** During EXECUTE, drop the `-kernel $(BIN)` re-pass and rely on `$(QEMU_FLAGS)`'s existing `-kernel $(OUT_BIN)` clause (so the rule body collapses to `$(QEMU_SYSTEM) $(QEMU_FLAGS) | tee /tmp/xvisor-trap.log`). No spec edit required; treat the PLAN sketch as illustrative.

### R-002 `tee swallows QEMU's exit code, so a failure-path test (V-F-1 future) can't distinguish QEMU crash from clean exit`

- **Severity:** LOW
- **Section:** `## Spec` C-13, `## Implementation` Phase 3 step 3, `## Validation` V-IT-1
- **Problem:** `$(QEMU_SYSTEM) ... | tee /tmp/xvisor-trap.log` produces `tee`'s exit code (always 0 on successful write). If QEMU exits non-zero — e.g., the canary `terminate(HaltCode::Failure)` path or an unexpected halt — the pipe's tail is what `make` sees. V-IT-1's grep assertions still catch the missing-line case, which is the primary regression to detect in P1, so this isn't blocking. But V-F-1's "QEMU exits with non-success" check, even though P1-scratch-branch-only, cannot be wired through the same harness without `set -o pipefail` or `${PIPESTATUS[0]}` plumbing.
- **Why it matters:** The harness pattern this target sets will be reused by P2/P3's `hext-check` and beyond. Locking in pipefail-aware plumbing now is one extra `SHELL := /bin/bash` + `.SHELLFLAGS := -o pipefail -c` (or per-recipe `set -o pipefail;`) and keeps the regression-harness honest for future failure-path tests.
- **Recommendation:** Optional during EXECUTE: add `SHELL := /bin/bash` + `.SHELLFLAGS := -e -o pipefail -c` near the top of the Makefile (or prefix the recipe line with `set -o pipefail;`). No spec edit required — but note in the commit message if you adopt it so the pattern is discoverable in P2.

### R-003 `offset_of! and assert! invocations in Phase 1 need an import path`

- **Severity:** LOW
- **Section:** `## Implementation` Phase 1 step 1 (const offset checks)
- **Problem:** The sketch shows `const _: () = assert!(offset_of!(TrapFrame, regs) == 0);` etc., with no qualifier. `core::mem::offset_of!` was stabilised in 1.77 and is callable as `offset_of!` only inside `use core::mem::offset_of;`. The on-disk `trap.rs` already imports nothing — its single const-assert uses `core::mem::size_of::<TrapFrame>()` directly. The new asserts need either a `use core::mem::offset_of;` or fully-qualified paths. Cosmetic, but worth flagging so EXECUTE doesn't punt on it.
- **Why it matters:** Build break, easy fix.
- **Recommendation:** During EXECUTE, add `use core::mem::offset_of;` at the top of `trap/mod.rs` (or fully-qualify each call). No spec edit required.

---

## Trade-off Advice

### TR-1 `Makefile harness pattern for the family of future canary tests`

- **Related Plan Item:** T-2 (canary-as-feature) + C-17 (grep-assertion plumbing)
- **Topic:** Maintainability vs Scope Creep
- **Reviewer Position:** Prefer modest investment now
- **Advice:** Treat `make trap-test` as the prototype for a `xvisor-test-*` family (next-up `hext-check` in P2). If you add `pipefail` plumbing (per R-002), keep it crate-wide rather than per-recipe so subsequent canaries inherit it automatically.
- **Rationale:** T-2 already commits to repeating the canary-as-feature pattern (the PLAN's own Log says so). The harness side of the pattern — `tee` + grep — should be just as durable. One Makefile prelude (`SHELL` + `SHELLFLAGS`) is cheaper than three separate per-recipe `set -o pipefail` clauses across P1/P2/P3.
- **Required Action:** Keep with clarification — adopt during EXECUTE if convenient; defer if the recipe-only form is faster, and pick it up when `hext-check` lands.

### TR-2 `V-UT-1 fall-back wording`

- **Related Plan Item:** V-UT-1
- **Topic:** Validation Honesty vs Iteration Cap
- **Reviewer Position:** Prefer current direction (commit + flag), neutral on tightening further
- **Advice:** The PLAN's V-UT-1 commits to `cargo test --lib` and flags the sibling-crate fall-back rather than hiding it. That's the right shape. If `cargo test` of the `cause` module surfaces a `no_main` snag during EXECUTE, surface it in VERIFY.md rather than silently swapping to "inspection only" — the next phase should see the harness-evolution context.
- **Rationale:** The hedge in 00_PLAN was an *excuse*; the current wording is a *contingency*. The distinction matters for future iterations that look back at how P1's first-real-test landed.
- **Required Action:** Adopt — no change required.

### TR-3 `Eager offset asserts for stable layout vs minimal asserts that just guard the dispatcher`

- **Related Plan Item:** Phase 1 step 1 (const-offset checks)
- **Topic:** Clarity vs Brevity
- **Reviewer Position:** Prefer adding asserts for every field
- **Advice:** Add `offset_of!` asserts for all five field positions (`regs`, `sepc`, `scause`, `stval`, `sstatus`), not just `sepc` as the 00_PLAN sketch implied. The 01_PLAN sketch already lists `regs == 0` and "equivalents for scause, stval, sstatus" — make that "and equivalents" explicit in the file so a future field-order drift breaks the build the same way regardless of which field moved.
- **Rationale:** TrapFrame's field order is the binding contract carried into P2/P3's H-ext exit emulation (where guest-register decoding indexes `frame.regs[rd]`). One assert per field is five lines and zero runtime cost.
- **Required Action:** Adopt during EXECUTE.
