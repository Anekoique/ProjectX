# `Device Emulation` IMPL REVIEW `02`

> Status: Open
> Feature: `dev`
> Iteration: `02`
> Owner: Reviewer
> Target Impl: `02_IMPL.md`

---

## Verdict

- Decision: Rejected
- Blocking Issues: `2`
- Non-Blocking Issues: `2`

## Summary

Round `02` does make real progress: the default machine now attempts to expose UART RX, the ACLINT / PLIC code is organized more cleanly under `device/intc/`, and the missing CPU-level SSWI regression test has been added. However, the delivered UART RX path is still not reliable enough to count as a completed shipped feature. The current default backend hardwires a fixed TCP port and silently degrades to TX-only when that port is unavailable, and the reset fix still clears the backend buffer only on a best-effort basis. `02_IMPL.md` also still overstates completion and verification for the current code.

---

## Findings

### IR-001 `Hardwired default TCP UART makes G-3b non-deterministic`

- Severity: HIGH
- Section: `Implementation Scope / Core Logic / Acceptance Mapping`
- Type: Correctness | Plan Compliance
- Problem:
  `xemu/xcore/src/cpu/riscv/mod.rs` now hardwires the default machine to `Uart::with_tcp(14514)`, but `xemu/xcore/src/device/uart.rs` still treats bind failure as a silent TX-only fallback. Because the port is fixed and not configurable through the machine constructor, a second emulator instance, a still-running earlier instance, or any unrelated process already using `127.0.0.1:14514` causes the shipped UART RX feature to disappear without surfacing an error.
- Why it matters:
  `02_IMPL.md` claims `G-3b` is "fully delivered", but in practice that is only true when the host environment happens to leave port `14514` free. In the round-02 validation run, `make run` immediately logged `UART: TCP bind failed on port 14514, falling back to TX-only`. A shipped feature cannot be accepted when normal multi-instance or port-conflict scenarios silently downgrade it to a different behavior.
- Recommendation:
  Make the UART backend configurable at machine construction time and surface bind failure explicitly. If silent fallback remains intentional, `G-3b` should be recorded as optional / partial rather than accepted shipped scope.

### IR-002 `UART reset still clears backend state only on a best-effort basis`

- Severity: HIGH
- Section: `Plan Compliance / Behavior`
- Type: Correctness | Regression
- Problem:
  `xemu/xcore/src/device/uart.rs` now tries to clear `rx_buf` during `reset()`, but it does so with `try_lock()`. If the TCP reader thread holds the mutex at reset time, the clear is skipped and the method returns success anyway. The same implementation also leaves the live TCP session policy undefined after reset.
- Why it matters:
  Round `02` claims IR-002 is fixed, but stale or concurrently arriving host input can still survive reset under load. That keeps the exact reset-leak class from round `01`, only narrowed to a race window instead of removed.
- Recommendation:
  Reset needs deterministic backend handling. Either block until the buffer is cleared, or swap to a fresh backend buffer / generation so pre-reset input cannot reappear. The reset policy for an active TCP connection should also be documented and tested explicitly.

### IR-003 `Validation still does not exercise the shipped default UART wiring`

- Severity: MEDIUM
- Section: `Verification Results / Acceptance Mapping`
- Type: Validation
- Problem:
  The new tests still validate `Uart::with_tcp()` mostly in isolation. There is no regression test proving that `RVCore::new()` with the default UART port actually exposes the intended RX -> PLIC -> CPU path, and there is no test for the port-conflict case at the machine level.
- Why it matters:
  The main round-02 behavioral change is in the machine wiring, not only in `Uart` itself. Without a machine-level test, the strongest new acceptance claim is still only inferred.
- Recommendation:
  Add an integration test around `RVCore::new()` that verifies the default UART backend is active when bind succeeds, and another that defines / checks the machine-level behavior when the default port is already occupied.

### IR-004 `02_IMPL.md still overstates completion and verification`

- Severity: MEDIUM
- Section: `Summary / Verification Results / Acceptance Mapping`
- Type: Validation | Plan Compliance
- Problem:
  `02_IMPL.md` says all blocking issues are fixed, reports `266 tests`, and lists no known issues, but the shipped UART RX path still has the blocking problems above. The required repo-level validation for this review produced different results: `make run` showed an immediate UART bind fallback, and `make test` passed with `272` tests rather than `266`.
- Why it matters:
  `NN_IMPL.md` is the acceptance record for the round. If it declares full delivery while key runtime behavior is still conditional on host port availability and reset races, the document is not reliable enough to approve merge / release.
- Recommendation:
  Update `02_IMPL.md` after the code is fixed so the summary, acceptance mapping, known issues, and verification section match the actual shipped behavior and the actual `make` results.

---

## Positive Notes

- The CPU-level `sswi_edge_delivered_once_and_clearable` test is a meaningful improvement over round `01` and closes a real validation gap.
- Moving ACLINT / PLIC under `device/intc/` is a reasonable cleanup and is materially clearer than the flat device layout.

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
- Reason: The default UART RX feature is still environment-dependent, and reset semantics for the TCP backend are still not deterministic.

---

## References

- Approved plan: `docs/dev/DEV_PLAN.md`
- QEMU `virt` machine docs: https://www.qemu.org/docs/master/system/riscv/virt.html
- RISC-V ACLINT specification: https://github.com/riscv/riscv-aclint/blob/main/riscv-aclint.adoc
