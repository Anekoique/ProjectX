# `am-tests` IMPL REVIEW `06`

> Status: Open
> Feature: `am-tests`
> Iteration: `06`
> Owner: Reviewer
> Target Impl: `06_IMPL.md`

---

## Verdict

- Decision: Accepted with Revisions
- Blocking Issues: `0`
- Non-Blocking Issues: `2`

## Summary

The implementation is substantially aligned with the approved direction from round 06.

The trap path is now buildable through `global_asm!(include_str!("trap.S"))`, which satisfies `06_MASTER.md` `M-001` and removes the old linker concern from the plan review. The new `am-tests` suite boots under `xemu`, the per-test `make run` path passes on the current snapshot, and the one-binary smoke path `make run TEST=a` also passes.

This review therefore uses the code as the primary artifact. `06_IMPL.md` is not present in the current tree, so plan-compliance and verification conclusions are derived from the implementation itself plus the local runs above.

Two non-blocking issues remain:

- the run-all (`TEST=a`) path is implemented but not part of the default or CI-gated validation contract
- the `am-tests` runner uses shared temp/result paths, so concurrent invocations in the same working tree can corrupt each other's output

---

## Findings

### IR-001 `Run-all mode is implemented but not covered by the gated validation path`

- Severity: MEDIUM
- Section: `Verification Results / Acceptance Mapping`
- Type: Validation
- Problem:
  The implementation adds a distinct run-all execution mode in `src/main.c` (`case 'a'`) and the Makefile can invoke it with `make run TEST=a`. However, the default `make run` path only iterates `ALL = u r t s p c e`, and CI only executes that default path. As a result, the shared-state run-all binary exists, but neither the local default contract nor the automated workflow verifies it.
- Why it matters:
  The run-all mode is behaviorally different from the per-test isolated runs because it preserves machine state across subtests. Bugs in that smoke path can therefore ship even when both the per-test runner and CI are green.
- Recommendation:
  Either add `make run TEST=a` to the automated validation flow, or explicitly downgrade run-all to a manual/local smoke path in the implementation record and acceptance mapping.

### IR-002 `am-tests runner uses shared artifact names and is not safe for concurrent invocations`

- Severity: MEDIUM
- Section: `Implementation Scope / Makefile`
- Type: Maintainability
- Problem:
  The runner writes to fixed paths in the working tree: `.result`, `.mk.$*`, `.output.$*`, and `build/`. If two `make run` invocations execute concurrently in `xkernels/tests/am-tests`, they reuse and truncate the same files. On the current snapshot, overlapping runs can mix pass/fail lines in `.result` and race on generated Makefiles and captured output.
- Why it matters:
  This does not affect the normal serialized CI job, but it makes local validation fragile and can produce misleading review results when two runs overlap in the same tree.
- Recommendation:
  Isolate per-invocation state with PID- or `mktemp`-scoped result/output paths, or document and enforce serialized execution for this runner.

---

## Positive Notes

- The `insert-arg.py` approach cleanly replaces the earlier `mainargs` multiple-definition problem without adding wrapper source files.
- Using `global_asm!` for `trap.S` is a simpler and cleaner fit for the current `xhal` build than the previously proposed `build.rs` native-build path.
- The `am-tests` scope is intentionally small and focused on the target subsystems: CSR, ACLINT timer/software interrupt, PLIC register access, trap handling, and UART output.
- Both the per-test path and the one-binary smoke path boot successfully on the current staged implementation.

---

## Approval Conditions

### Must Fix

- None

### Should Improve

- IR-001
- IR-002

### Ready for Merge / Release

- Yes
- Reason: no blocking correctness issue was found in the current implementation; the remaining issues are validation coverage and runner robustness.
