# `am-tests` REVIEW `01`

> Status: Open
> Feature: `am-tests`
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
- Blocking Issues: `3`
- Non-Blocking Issues: `2`

## Summary

`01_PLAN` is materially better than `00_PLAN`.

It does address the last round’s biggest problems in the right direction:

- it introduces a shared trap harness instead of hand-waving trap entry
- it narrows the PLIC scope instead of leaving host-assisted IRQ injection ambiguous
- it tries to make the runner automatable with timeout and output checks
- it adds a real Response Matrix and keeps the round focused on implemented features

Those are substantive improvements.

However, this round is still not implementation-ready.

Three blocking issues remain:

- the plan still does not fit the current `xam` C build contract, because trap-based tests now require linking `src/trap.S` but the existing pipeline only builds a single `K` source
- the ACLINT “RV64 direct 64-bit access” fix does not match the current `xemu` ACLINT MMIO implementation, which still exposes split low/high registers
- after narrowing PLIC to register-only validation, the new `plic-regs.c` no longer proves the claim/complete behavior that `G-4` and `V-IT-7` still say is covered

Two non-blocking issues should also be tightened before implementation:

- the runner sketch still does not fully implement the stated required/optional selection and UART-output assertion behavior
- the optional tests are described as compile-checked, but the proposed CI flow never actually builds them

---

## Findings

### R-001 `Trap-based tests still do not fit the current xam C build contract`

- Severity: HIGH
- Section: `Architecture / Constraints / Step 0 / Makefile`
- Type: API
- Problem:
  `01_PLAN` resolves the old trap-entry underdefinition by adding `src/trap.S` and saying trap-based tests “link `trap.S` as additional source”. But the current `xam` C pipeline does not expose such a contract. Today `xam/scripts/build_c.mk` builds one object derived from `K`, and the generated per-test Makefile pattern still only feeds a single kernel source into `$(AM_HOME)/Makefile`. There is no defined `SRCS`, `EXTRA_OBJS`, or equivalent interface that would actually pull `src/trap.S` into `timer-irq`, `msip`, or `trap-ecall`.
- Why it matters:
  This is now on the critical path. The required trap-based tests are central to the feature, and they cannot be implemented cleanly until the build surface is made explicit. Right now the plan describes a multi-source test model that the current `xam` build contract does not support.
- Recommendation:
  The next PLAN must choose and document one concrete build model:
  1. Extend `xam` with an explicit multi-source C kernel interface such as `SRCS += src/trap.S`; or
  2. Restructure each trap-based test into a form that already fits the existing pipeline, such as a per-test directory or another single-entry build shape.

  The plan should also say exactly which `xam` files change if option 1 is chosen.

### R-002 `The new RV64 ACLINT helper still mismatches xemu's actual MMIO behavior`

- Severity: HIGH
- Section: `Log / Data Structure / Step 2 / Step 5`
- Type: Correctness
- Problem:
  The plan says R-002 is resolved by using low/high split access on RV32 and direct 64-bit `REG64` access on RV64. But the current `xemu` ACLINT device still decodes `mtime` and `mtimecmp` as separate low/high registers at offsets `0xBFF8/0xBFFC` and `0x4000/0x4004`. On the current implementation, an 8-byte MMIO access at `0x4000` or `0xBFF8` does not behave like a real 64-bit register access; it only goes through the low-half register path.
- Why it matters:
  This means the plan’s “fixed” timer helpers are still wrong for the actual target under test. On RV64, `mtimecmp_write()` can silently update only the low word, and `ACLINT_MTIMECMP` / `ACLINT_MTIME` read-backs can silently observe only the low word. That directly threatens `timer-read` and `timer-irq`, which are required validations.
- Recommendation:
  The next PLAN must make one contract explicit:
  1. For this round, use split low/high helper accesses on both RV32 and RV64 to match the current `xemu` device model; or
  2. Expand the feature scope to include `xemu` ACLINT support for true 8-byte MMIO accesses, then update validation and write-set accordingly.

  It should not continue to claim that the current RV64 direct `REG64` path is already correct.

### R-003 `PLIC validation no longer proves the claim/complete contract it still advertises`

- Severity: HIGH
- Section: `Goals / Step 7 / Validation / Acceptance Mapping`
- Type: Validation
- Problem:
  Narrowing PLIC to register-level validation is the right direction for determinism, but `plic-regs.c` now only proves register read/write plus “empty claim returns 0”. It does not prove a non-zero claim, does not prove `complete`, and does not prove any pending-source behavior. Yet `G-4` and `V-IT-7` still say the round validates priority/enable/threshold/claim/complete.
- Why it matters:
  The plan can no longer claim it validated the only non-trivial part of the remaining PLIC scope. If the review approves `G-4` in its current wording, this round could report “PLIC validated” without ever exercising a real claim/complete cycle.
- Recommendation:
  The next PLAN must either:
  1. Narrow `G-4`, `V-IT-7`, and the acceptance mapping to “register accessibility plus empty-claim semantics”; or
  2. Add a deterministic way to create a pending source and then validate non-zero claim plus complete behavior.

  The current wording overstates what the new test actually covers.

### R-004 `The runner sketch still does not implement the stated test-selection and UART-output contract`

- Severity: MEDIUM
- Section: `Log / Step 0 / Makefile / Validation`
- Type: Validation
- Problem:
  The plan text says the runner supports required/optional split, per-test selection, timeout, and UART output assertion. But the included Makefile sketch still has two concrete gaps:
  - `Makefile.%: tests/%.c optional/%.c` requires both files to exist, which does not match the claimed “either required or optional” model
  - the sketch only checks for `GOOD TRAP` on the last line and does not show any per-test expected-output check for `uart-hello`
- Why it matters:
  This leaves the “resolved” runner behavior only partially specified. Under the current sketch, required/optional routing is still not a valid build rule, and `G-1` can pass without any explicit assertion that the expected UART string was emitted.
- Recommendation:
  The next PLAN should split required and optional build rules explicitly and define one concrete output-check mechanism for `uart-hello`, such as per-test expected-string metadata or a dedicated rule path.

### R-005 `Optional tests are not actually compile-checked by the proposed CI flow`

- Severity: MEDIUM
- Section: `Goals / Step 8 / Validation / Acceptance Mapping`
- Type: Validation
- Problem:
  `G-9` and the acceptance mapping say optional tests are compile-checked but not gated. But the proposed CI job runs only `make run`, and `make run` explicitly excludes `optional/`. There is no separate compile-only target for the optional tree.
- Why it matters:
  As written, the optional tests are manual artifacts, not CI-checked artifacts. That is a real difference in maintenance value: they can silently stop compiling and the pipeline will never notice.
- Recommendation:
  The next PLAN should either:
  - add a `make build-optional` or similar CI step; or
  - stop claiming compile-checked status for `G-9` and describe them as manual-only optional tests

---

## Trade-off Advice

### TR-1 `Prefer an explicit xam multi-source interface over test-local build hacks`

- Related Plan Item: `Step 0`
- Topic: Clean Design vs Short-Term Convenience
- Reviewer Position: Prefer Option A
- Advice:
  If trap-based tests remain required, prefer extending `xam` with a small explicit multi-source contract rather than hiding extra assembly linkage inside one-off per-test tricks.
- Rationale:
  This round is already using `am-tests` to define a reusable testing model. A small, explicit `xam` build-surface improvement is more maintainable than building a special-case mechanism into the `am-tests` Makefile only.
- Required Action:
  The next PLAN should either adopt this and name the `xam` changes, or justify why the feature should stay entirely test-local.

### TR-2 `Be truthful about optional-test coverage`

- Related Plan Item: `T-3`
- Topic: CI Cost vs Validation Signal
- Reviewer Position: Prefer Option A
- Advice:
  If optional tests are kept as a goal, compile them in CI even if their runtime results are non-gating. If CI cost is not worth it, then stop describing them as compile-checked coverage.
- Rationale:
  The current plan is trying to get both benefits at once: low CI cost and ongoing compile-signal for future tests. But the proposed workflow only gives the first one. The contract should match the actual chosen trade-off.
- Required Action:
  The next PLAN should either add compile-only coverage for `optional/` or narrow the wording of `G-9`.

---

## Positive Notes

- The new Response Matrix is much better than round 00 and makes the executor’s intent reviewable.
- Narrowing PLIC away from host-assisted UART IRQ injection is the right correction for this round.
- Introducing a shared trap harness is the correct architectural direction, even though the build contract around it is still unfinished.
- The required/optional split is a useful idea for keeping the main bring-up path deterministic while still recording future test targets.

---

## Approval Conditions

### Must Fix
- R-001
- R-002
- R-003

### Should Improve
- R-004
- R-005

### Trade-off Responses Required
- T-3

### Ready for Implementation
- No
- Reason: the trap harness still lacks a real `xam` build contract, the ACLINT helper still mismatches the current `xemu` MMIO model, and the narrowed PLIC test no longer proves the contract it still claims.
