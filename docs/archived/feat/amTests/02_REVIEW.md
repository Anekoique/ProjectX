# `am-tests` REVIEW `02`

> Status: Open
> Feature: `am-tests`
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
- Blocking Issues: `2`
- Non-Blocking Issues: `2`

## Summary

`02_PLAN` is stronger than `01_PLAN`.

The round does make several real improvements:

- it finally aligns ACLINT helper access with the current split low/high MMIO model
- it moves trap handling into `xam`, which is directionally closer to the AbstractMachine-style layering requested by `01_MASTER`
- it narrows PLIC to register accessibility only instead of continuing to overclaim claim/complete coverage
- it adds a compile-only path for optional tests in principle, which is the right direction

However, the plan still should not be approved for implementation.

Two blocking issues remain:

- `G-1` still does not actually validate UART output, because the round dropped output assertion and now treats “exit 0” as sufficient evidence
- the new `xam` trap HAL publishes a `TrapContext` contract that does not match the entry assembly it proposes

There are also two non-blocking issues worth tightening in the next round:

- the optional compile-check target still masks failures, so `G-8` / `G-9` overstate the actual CI coverage
- the new console HAL bypasses the existing `_putch` boundary in `xlib`, which weakens the claimed “improve xam basic functions” direction by creating a second console ABI instead of connecting the existing one

---

## Findings

### R-001 `UART output is no longer actually validated`

- Severity: HIGH
- Section: `Summary / Goals / Validation / Makefile`
- Type: Validation
- Problem:
  `02_PLAN` says R-004 is resolved by switching `uart-putc` to exit-code-only validation because “UART output goes to stdout naturally via xemu”. But `V-IT-1` now only says `uart-putc` exits 0, and the runner only checks for `GOOD TRAP`. That means a broken console path can still pass as long as the test returns normally. The round has dropped the machine-checkable output assertion instead of fixing it.
- Why it matters:
  `G-1` is specifically a UART output goal. If the test does not assert that the expected bytes appeared, it is no longer validating the UART output path from guest to host. A stubbed or broken `putc()` implementation could still satisfy the current acceptance criteria.
- Recommendation:
  The next PLAN must either:
  1. Restore explicit output capture and assertion for `uart-putc`; or
  2. Narrow `G-1` and `V-IT-1` to a weaker goal such as “console HAL call path returns normally”.

  If the feature is meant to validate the UART device path, option 1 is the correct one.

### R-002 `The published TrapContext ABI does not match the proposed trap entry`

- Severity: HIGH
- Section: `xam HAL Functions / Data Structure / Invariants`
- Type: API
- Problem:
  The plan publishes a C-visible `TrapContext` with `gpr[32]`, `mepc`, and `mcause`, implying a complete architectural GPR snapshot. But the proposed `__am_trap_entry` does not save `x0`, and it saves `x2/sp` only after `sp` has already been decremented to allocate the trap frame. So the public structure is not actually the original register state the type name suggests. In addition, the plan leaves the final assembly shape unresolved with “implementation will use whichever works” between `.rept`-generated register names and an explicit save list.
- Why it matters:
  This is now part of the `xam` HAL contract, not just a local test helper. If the exposed trap ABI is internally inconsistent, future handlers and tests will build on a false model of what the saved register state means. That is exactly the kind of interface debt that becomes painful once multiple tests or higher-level code consume it.
- Recommendation:
  The next PLAN should finalize one precise trap ABI:
  - either document a deliberately partial context shape with exact slot meanings
  - or save the original register state in a layout that truly matches `TrapContext`

  It should also replace the “whichever works” assembly branch with one final entry implementation.

### R-003 `Optional compile-check still masks build failures`

- Severity: MEDIUM
- Section: `Log / Makefile / CI Addition / Validation`
- Type: Validation
- Problem:
  `02_PLAN` says R-005 is resolved because `make build-optional` compile-checks optional tests. But the proposed `Build.%` rule runs `make -s -f Makefile.$* kernel 2>&1 || true`, which suppresses build failures and always returns success. As written, CI can report green even if every optional test fails to compile.
- Why it matters:
  This means the plan still does not deliver the actual signal it claims. The optional tests remain effectively unchecked by CI, so `G-8`, `G-9`, and `V-IT-8` are stronger than the proposed workflow really provides.
- Recommendation:
  The next PLAN should remove the unconditional success path and aggregate optional build failures into a real pass/fail result, or explicitly downgrade `build-optional` to a best-effort non-checking step.

### R-004 `The new console HAL creates a second console ABI instead of wiring the existing one`

- Severity: MEDIUM
- Section: `Master Compliance / xam HAL Functions / Trade-offs`
- Type: Maintainability
- Problem:
  The new `console.rs` exports `putc()` and `puts()`, but the existing C library boundary already routes formatted output through `_putch` in `xlib/src/stdio.c`. The plan does not connect that boundary. As a result, `printf()` remains disconnected from the new console HAL, and the project ends up with two separate console interfaces instead of one coherent one.
- Why it matters:
  `01_MASTER` asked for `xam` to improve as a basic HAL. Adding a second console ABI is functional, but it is not the cleanest platform boundary. It increases surface area and keeps the existing `xlib` I/O path half-integrated.
- Recommendation:
  The next PLAN should either:
  - make `_putch` the primary `xhal` console hook and layer `putc`/`puts` on top if needed; or
  - justify explicitly why the HAL should own a separate console ABI from the existing `xlib` output boundary

---

## Trade-off Advice

### TR-1 `Prefer wiring xhal into xlib's existing output hook`

- Related Plan Item: `T-1`
- Topic: Clean Design vs Local Convenience
- Reviewer Position: Prefer Option A
- Advice:
  Keep the idea of improving `xam` as a HAL, but prefer integrating through `_putch` rather than standardizing a second ad hoc C console interface.
- Rationale:
  This gives the project one console boundary instead of two, immediately improves `printf()` for all C tests, and better matches the “basic HAL” direction requested by the master directive.
- Required Action:
  The next PLAN should either adopt this integration path or explain why it is intentionally rejected.

### TR-2 `If trap stays in xam, make the context contract intentionally minimal`

- Related Plan Item: `G-9`
- Topic: Flexibility vs Safety
- Reviewer Position: Prefer Option A
- Advice:
  If `xam` owns trap entry now, the safest design is a small, explicit, well-documented context contract rather than a vaguely “full register snapshot” contract that the assembly does not faithfully implement.
- Rationale:
  A smaller but truthful ABI is easier to keep correct and easier for future tests to consume than an oversized structure with ambiguous slot semantics.
- Required Action:
  The next PLAN should either tighten `TrapContext` to what is really saved, or save what the published structure claims.

---

## Positive Notes

- Switching ACLINT access to split low/high on both RV32 and RV64 is the right correction for the current `xemu` device model.
- Narrowing PLIC to register accessibility is much more honest than the previous round’s mixed claim/complete wording.
- Moving trap support into `xam` is a reasonable architectural direction for this project, even though the current ABI still needs tightening.
- Adding a compile-only path for optional tests is the right idea; it just needs to become a real check instead of a swallowed one.

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
- Reason: UART output is still not actually validated, and the new `xam` trap HAL publishes a context ABI that does not match its proposed entry assembly.
