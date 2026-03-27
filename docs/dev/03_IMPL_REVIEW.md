# `Device Emulation` IMPL REVIEW `03`

> Status: Open
> Feature: `dev`
> Iteration: `03`
> Owner: Reviewer
> Target Impl: `03_IMPL.md`

---

## Verdict

- Decision: Rejected
- Blocking Issues: `2`
- Non-Blocking Issues: `2`

## Summary

Round `03` does address the two round-02 complaints directly: the default machine is deterministic again, and stale pre-reset UART bytes are no longer visible after reset. However, the implementation still is not acceptable as the delivered Phase 4 result. The round resolves the default-port problem by downgrading the approved UART RX scope inside `03_IMPL.md`, which is a material contract change that was not re-approved in a new plan iteration. The new UART reset strategy also leaves the opt-in TCP RX backend permanently disconnected after reset.

---

## Findings

### IR-001 `Approved UART RX scope was downgraded only in the IMPL artifact`

- Severity: HIGH
- Section: `Implementation Scope / Plan Compliance / Acceptance Mapping`
- Type: Plan Compliance | Behavior
- Problem:
  The live code now wires `Uart::new()` in the default machine at `xemu/xcore/src/cpu/riscv/mod.rs`, and `03_IMPL.md` explicitly downgrades `G-3b` from shipped behavior to "opt-in via `with_tcp()`". But the approved implementation contract in `docs/dev/DEV_PLAN.md` still describes Phase 4 as `UART 16550 (TX + TCP RX)` in the delivered machine configuration, and `02_IMPL_MASTER.md` required the reviewer-raised issues to be fixed, not the approved scope to be narrowed in-place.
- Why it matters:
  This is not a small implementation deviation; it changes the user-visible behavior of the default machine. Under the current workflow, that kind of contract change needs a new approved plan iteration, not only a note in `NN_IMPL.md`.
- Recommendation:
  Either reopen the plan and explicitly re-approve "TX-only default machine + opt-in TCP RX" as the new contract, or restore an approved default / configurable UART RX path that satisfies the existing `DEV_PLAN.md` scope.

### IR-002 `UART reset now disconnects the TCP RX backend permanently`

- Severity: HIGH
- Section: `Core Logic / Plan Compliance`
- Type: Correctness | Regression
- Problem:
  `Uart::with_tcp()` in `xemu/xcore/src/device/uart.rs` spawns a background thread that captures the original `rx_buf` `Arc`. `reset()` then replaces `self.rx_buf` with a fresh `Arc`, so the background thread keeps writing to the orphaned old buffer while `tick()` drains the new one. After reset, the opt-in TCP RX path no longer has any live producer connected to the buffer the device reads from.
- Why it matters:
  The round-03 reset change removes stale pre-reset bytes, but it also breaks all future RX for that UART instance after reset. For any user opting into `with_tcp()`, reset effectively kills UART input until the whole device is rebuilt.
- Recommendation:
  Reset must either preserve the live backend and clear it deterministically, or recreate / rebind the backend so post-reset RX remains functional. The intended reset policy for a live TCP session needs an explicit regression test.

### IR-003 `The new UART reset test does not exercise the live TCP backend`

- Severity: MEDIUM
- Section: `Verification Results`
- Type: Validation
- Problem:
  `reset_deterministically_clears_rx` in `xemu/xcore/src/device/uart.rs` uses `setup() -> Uart::new()` and manually pushes bytes into local buffers. It never exercises `with_tcp()`, an active reader thread, or post-reset input delivery.
- Why it matters:
  The round’s main behavioral change is specifically about the TCP-backed reset path, but the new test only proves local buffer cleanup on a TX-only UART. It does not protect against the regression in IR-002.
- Recommendation:
  Add a reset test for `Uart::with_tcp()` that defines the intended post-reset behavior and verifies it with a live backend, not only direct buffer mutation.

### IR-004 `03_IMPL.md still overstates verification results`

- Severity: MEDIUM
- Section: `Summary / Verification Results`
- Type: Validation
- Problem:
  `03_IMPL.md` reports `266 tests` and uses `cargo`-style verification bullets, but the required repository workflow for this review produced different results: `make test` passed with `272` tests, and the required validation contract is `make fmt`, `make clippy`, `make run`, `make test`.
- Why it matters:
  `NN_IMPL.md` is supposed to be the acceptance record for the round. If its verification summary does not match the actual repo-level workflow, it is not reliable enough to approve merge / release.
- Recommendation:
  Update `03_IMPL.md` so its verification section and test counts match the actual `make` results for this round.

---

## Positive Notes

- Reverting the default machine away from a hardwired fixed TCP port does remove the environment-dependent bind failure that blocked round `02`.
- The new reset approach is conceptually cleaner than the previous `try_lock().clear()` path for preventing stale pre-reset bytes.

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
- Reason: The round changes the approved UART delivery contract without re-approval, and the opt-in TCP UART backend no longer survives reset correctly.

---

## References

- Approved plan: `docs/dev/DEV_PLAN.md`
- QEMU `virt` machine docs: https://www.qemu.org/docs/master/system/riscv/virt.html
- RISC-V ACLINT specification: https://github.com/riscv/riscv-aclint/blob/main/riscv-aclint.adoc
