# `Device Emulation` IMPL REVIEW `01`

> Status: Open
> Feature: `dev`
> Iteration: `01`
> Owner: Reviewer
> Target Impl: `01_IMPL.md`

---

## Verdict

- Decision: Rejected
- Blocking Issues: `2`
- Non-Blocking Issues: `2`

## Summary

Round `01` fixes several real problems from `00_IMPL_REVIEW`: the `ProgramExit` result is now preserved, SSWI is no longer wired through the sticky `irq_state` path, and PLIC claim now reevaluates external interrupt outputs. However, the implementation is still not acceptable as the delivered `DEV_PLAN.md` result. The default machine still does not expose the approved UART RX backend, and the new device-reset path remains incomplete for UART because backend-buffered input can survive reset. The supporting `01_IMPL.md` artifact also overstates scope closure and validation.

---

## Findings

### IR-001 `Default machine still does not ship the approved UART RX path`

- Severity: HIGH
- Section: `Implementation Scope / Plan Compliance / Acceptance Mapping`
- Type: Correctness | Plan Compliance
- Problem:
  `xemu/xcore/src/cpu/riscv/mod.rs` still wires `Box::new(Uart::new())` in `RVCore::new()`, while `Uart::with_tcp(...)` is only referenced from unit tests in `xemu/xcore/src/device/uart.rs`. `01_IMPL.md` records this as "Partially Completed", but it also says all four blocking issues are fixed, reports `Unresolved Gaps: None`, and marks `G-3b UART RX` as `Pass`.
- Why it matters:
  The shipped emulator still does not provide the approved `DEV_PLAN.md` UART RX feature in normal runs. This is the same scope gap identified in round `00`, and the implementation artifact currently understates that it remains unresolved.
- Recommendation:
  Either wire a configurable TCP-backed UART into the default machine, or reopen the approved scope and explicitly downgrade `G-3b` to deferred / partial instead of accepted.

### IR-002 `UART reset still leaks backend input across reset`

- Severity: HIGH
- Section: `Plan Compliance / Behavior`
- Type: Correctness | Regression
- Problem:
  `xemu/xcore/src/device/uart.rs` resets only the register state and `rx_fifo`, but it does not clear `rx_buf` and does not reset the TCP backend lifecycle created by `with_tcp()`. Bytes already accepted by the background thread can still be drained into `rx_fifo` after the next `tick()`, even after `RVCore::reset()` calls `bus.reset_devices()`.
- Why it matters:
  Round `01` claims IR-004 is fixed, but opt-in UART RX can still deliver stale pre-reset input into the guest after reset. That is exactly the kind of reset leakage the previous round rejected.
- Recommendation:
  Reset must clear both the frontend FIFO and the backend buffer, and the implementation needs an explicit policy for the TCP session on reset. If the connection is intentionally preserved, document that deviation and test it; otherwise, recreate or invalidate the backend so pre-reset input cannot reappear.

### IR-003 `SSWI fix still lacks a CPU-level regression test`

- Severity: MEDIUM
- Section: `Verification Results / Acceptance Mapping`
- Type: Validation
- Problem:
  The new ACLINT test only verifies that `SETSSIP` toggles an `AtomicBool`. It does not verify the actual round-01 contract: `RVCore::step()` consumes the edge once, sets `mip.SSIP`, and guest CSR clears of SSIP remain cleared on later steps.
- Why it matters:
  The original bug was not in ACLINT’s local write path alone; it was in the interaction between ACLINT, Bus, and `sync_interrupts()`. Without a CPU-level regression test, the most important behavioral guarantee of the round is still only inferred from the code.
- Recommendation:
  Add an `RVCore` integration test that triggers SSWI, observes a single SSIP delivery, clears SSIP via CSR state, and confirms that a later step does not reassert it without another `SETSSIP` write.

### IR-004 `01_IMPL.md still overstates closure and required verification`

- Severity: MEDIUM
- Section: `Summary / Verification Results / Acceptance Mapping`
- Type: Validation | Plan Compliance
- Problem:
  `01_IMPL.md` says "fixes all 4 blocking issues", reports `265 tests passing`, and lists no unresolved gaps, but IR-001 remains partial and the required repo-level `make fmt`, `make clippy`, `make run`, and `make test` workflow is not what the document records.
- Why it matters:
  `NN_IMPL.md` is supposed to be the reliable acceptance record for the round. Right now it is ahead of the actual implementation state and ahead of the AGENTS-required verification contract.
- Recommendation:
  After the code fixes land, update `01_IMPL.md` so its summary, acceptance mapping, and verification section match the actual shipped scope and the actual `make` results for this round.

---

## Positive Notes

- The round does fix several real correctness issues from `00_IMPL_REVIEW`, especially the `ProgramExit` overwrite and the PLIC post-claim reevaluation bug.
- `IrqState` and the named PLIC constants are cleaner than the previous raw atomic / hard-coded-offset version.

---

## Approval Conditions

### Must Fix
- IR-001
- IR-002

### Should Improve
- IR-003
- IR-004

### Ready for Merge / Release
- No
- Reason: The delivered implementation still does not match the approved UART RX scope, and reset behavior is still incomplete for the opt-in UART RX path.

---

## References

- Approved plan: `docs/dev/DEV_PLAN.md`
- QEMU `virt` machine docs: https://www.qemu.org/docs/master/system/riscv/virt.html
- RISC-V ACLINT specification: https://github.com/riscv/riscv-aclint/blob/main/riscv-aclint.adoc
