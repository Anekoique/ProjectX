# `am-tests` REVIEW `00`

> Status: Open
> Feature: `am-tests`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
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

This `00_PLAN` is directionally correct.

It is aiming at the right validation layer: real bare-metal guest programs built through the existing `xam` flow and run on `xemu`, rather than more unit tests inside `xcore`. That matches the stated goal of validating the guest-visible behavior of the CSR subsystem, ACLINT, PLIC, and UART.

The current draft also gets several important baseline choices right:

- it reuses the existing `xkernels/tests/cpu-tests` style runner pattern
- it keeps the round scoped to M-mode only
- its MMIO base addresses match the current `RVCore::new()` wiring in `xemu`

However, the plan is not implementation-ready yet.

Three issues are still blocking:

- trap and interrupt tests rely on `mtvec` handlers, but the plan does not define any actual trap-entry ABI or reusable entry stub for C tests
- the ACLINT timer portion is specified with helpers that do not match the current 64-bit `mtime` / `mtimecmp` programming model
- the PLIC goal is still split between a real UART-driven external interrupt test and a register-only fallback, which are not the same deliverable

There are also two non-blocking issues worth tightening before implementation:

- UART output and hang detection are not yet converted into machine-checkable runner behavior
- the CSR goal claims privilege-transition coverage that an M-mode-only round does not actually provide

---

## Findings

### R-001 `Trap-entry contract is missing for mtvec-based tests`

- Severity: HIGH
- Section: `Architecture / Invariants / Step 2 / Step 3 / Step 5`
- Type: Flow
- Problem:
  The plan says each test is a standalone C program, while `msip`, `timer-irq`, and `trap-ecall` all depend on installing a custom `mtvec` handler that verifies trap state and returns via `mret`. But the current `xam` / `xhal` environment only defines `_start -> _trm_init -> main() -> terminate()`. There is no shared trap-entry stub, no save/restore contract, no `mscratch` frame convention, and no specified way for a C test to act as a raw `mtvec` target safely.
- Why it matters:
  This is not a small implementation detail. Without a concrete trap-entry path, the core tests for software interrupts, timer interrupts, and `ecall` roundtrip are underdefined and likely to clobber guest state or fail nondeterministically. The plan currently assumes a normal C function can stand in for a trap vector, which is not a reliable contract.
- Recommendation:
  The next PLAN must define one shared trap harness explicitly:
  - where the trap entry code lives
  - which registers are saved
  - how the C-level checker is called
  - how `mepc` is advanced when required
  - how `mret` is executed
  - which tests reuse this harness

### R-002 `ACLINT timer helpers do not match the current 64-bit register model`

- Severity: HIGH
- Section: `Data Structure / Step 2 / Validation`
- Type: Correctness
- Problem:
  The plan only defines `MMIO_READ32` / `MMIO_WRITE32` and single symbolic addresses for `ACLINT_MTIME` and `ACLINT_MTIMECMP`. But the current ACLINT implementation and the ACLINT specification expose 64-bit `mtime` and `mtimecmp` values using low/high register offsets. As written, `timer-read` and `timer-irq` have no correct way to program or read the full compare value. In particular, writing only the low word of `mtimecmp` leaves the high word unchanged, which can prevent `MTIP` from ever asserting.
- Why it matters:
  `G-2` and `G-7` are central goals of this round. If the register model in the plan is wrong, the timer tests will either hang or validate the wrong thing, and the resulting failures will not distinguish plan bugs from emulator bugs.
- Recommendation:
  The next PLAN should define explicit low/high offsets and helpers for:
  - `MTIME_LO` / `MTIME_HI`
  - `MTIMECMP_LO` / `MTIMECMP_HI`

  It should also state the exact read/write sequence used by the tests and make the single-hart assumption explicit.

### R-003 `PLIC scope is still split between self-contained testing and host-assisted IRQ injection`

- Severity: HIGH
- Section: `Goals / Step 6 / Validation / Acceptance Mapping`
- Type: Validation
- Problem:
  `G-4` promises a real PLIC flow: configure priority/enable/threshold, trigger a UART interrupt, then claim and complete it. But Step 6 immediately weakens that into two incompatible paths: maybe inject UART RX from the host, or maybe only test PLIC register read/write without a live interrupt source. Those are materially different scopes. With the current `RVCore::new()` wiring, a UART external interrupt depends on RX data arriving from outside the guest, so the bare-metal app cannot trigger it by itself.
- Why it matters:
  This is a plan-contract issue, not just a test-detail issue. Either this round validates real external interrupt routing through UART -> PLIC -> `mip`, or it validates only internal PLIC register semantics. Leaving both paths open makes `G-4` impossible to approve cleanly and makes `V-IT-7` ambiguous.
- Recommendation:
  The next PLAN must choose one explicit contract and update goals, steps, and acceptance mapping consistently:
  1. Keep end-to-end UART IRQ validation and specify the host harness and runner behavior in detail; or
  2. Narrow `G-4` to deterministic PLIC register / gateway semantics and defer host-assisted UART IRQ validation to a later iteration.

### R-004 `UART output and hang behavior are not yet machine-checkable`

- Severity: MEDIUM
- Section: `Execution Flow / Failure Flow / Validation`
- Type: Validation
- Problem:
  `uart-hello` says it verifies console output, but the runner design only checks process success or failure. The failure flow also says hangs fail "if configured" or by manual kill, but there is no timeout policy in the proposed `Makefile`. So the two most important failure modes for these tests, missing UART bytes and infinite spin loops, are not yet automated.
- Why it matters:
  These tests are supposed to become regression assets, not just bring-up demos. Without explicit output capture and bounded runtime, the suite can report false positives or hang forever in CI and local batch runs.
- Recommendation:
  The next PLAN should define:
  - how UART output is captured and asserted
  - a per-test timeout policy
  - what exact runner signal counts as PASS or FAIL for timeout cases

### R-005 `CSR acceptance overstates what an M-mode-only round can prove`

- Severity: MEDIUM
- Section: `Goals / Constraints / Acceptance Mapping`
- Type: Spec Alignment
- Problem:
  `G-5` says this round verifies CSR privilege transitions, but the plan also explicitly constrains the round to M-mode and excludes S-mode / U-mode setup. Writing `medeleg` / `mideleg` or checking `mstatus`, `mie`, and `mtvec` read-back does not by itself validate delegated behavior or cross-privilege transitions.
- Why it matters:
  The review needs a truthful acceptance contract. If this round is really about M-mode CSR WARL behavior plus M-mode trap entry / return, the plan should say that directly rather than claiming broader privilege coverage than it will test.
- Recommendation:
  The next PLAN should either:
  - narrow `G-5` to M-mode CSR semantics; or
  - add a concrete supervisor-mode test design and the prerequisites needed to run it

---

## Trade-off Advice

### TR-1 `Keep raw MMIO, but add shared 64-bit ACLINT helpers`

- Related Plan Item: `T-1`
- Topic: Simplicity vs Safety
- Reviewer Position: Prefer Option A
- Advice:
  Keep the raw MMIO approach for device validation, but do not keep every timer access ad hoc. Add a small shared helper layer in `include/test.h` for split 64-bit ACLINT registers and CSR access patterns.
- Rationale:
  Raw MMIO is the right testing model here because the goal is to validate guest-visible devices directly. But the timer register layout is subtle enough that repeating low/high access logic in each test will create avoidable bugs in the tests themselves.
- Required Action:
  The next PLAN should keep raw MMIO as the testing model and explicitly add shared helpers for ACLINT timer accesses.

### TR-2 `Prefer a truthful first-round PLIC scope over pseudo end-to-end coverage`

- Related Plan Item: `T-3`
- Topic: Coverage vs Deterministic Automation
- Reviewer Position: Prefer Option B
- Advice:
  If the next PLAN does not fully specify a host-side UART RX harness, then it should narrow the first-round PLIC scope to deterministic register / gateway semantics instead of continuing to imply a real end-to-end UART external interrupt test.
- Rationale:
  AbstractMachine-style bare-metal tests are strongest when the guest program is the whole test. A half-specified host interaction path is brittle, hard to reproduce, and hard to review. A smaller but truthful first-round contract is better than nominally broader coverage that is not actually defined.
- Required Action:
  The next PLAN must either specify the host harness concretely or narrow `G-4`, `V-IT-7`, and the acceptance mapping accordingly.

---

## Positive Notes

- Reusing the existing `xkernels/tests/cpu-tests` runner pattern is the right starting point for `am-tests`.
- The plan keeps the round bounded: M-mode only, single hart only, and no unrelated devices.
- The proposed MMIO base addresses align with the current `RVCore::new()` machine wiring, so the direction is grounded in the actual implementation rather than an invented target.
- Using bare-metal guest programs to validate the emulator from the guest side is the correct layer for this feature.

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
- T-1
- T-3

### Ready for Implementation
- No
- Reason: the trap-entry contract, ACLINT timer programming model, and PLIC scope are all still unresolved in the current round.
