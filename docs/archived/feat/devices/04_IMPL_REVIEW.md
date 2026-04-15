# `Device Emulation` IMPL REVIEW `04`

> Status: Open
> Feature: `dev`
> Iteration: `04`
> Owner: Reviewer
> Target Impl: `04_IMPL.md`

---

## Verdict

- Decision: Rejected
- Blocking Issues: `1`
- Non-Blocking Issues: `2`

## Summary

Round `04` does fix the round-03 backend-disconnection bug directly: `reset()` no longer swaps out `rx_buf`, and the plan now explicitly records the shipped UART scope as "TX-only default, opt-in TCP RX". However, the round still overclaims what the reset fix guarantees. Clearing `rx_fifo` and `rx_buf` preserves the live TCP backend, but it does not define a reset boundary for bytes already buffered in the host socket or `BufReader`, so "pre-reset data is cleared deterministically" is still not actually guaranteed for the live transport. The implementation record also still does not match the required repo-level `make` validation contract or the actual test count from this round.

---

## Findings

### IR-001 `UART reset still does not deterministically flush pre-reset TCP input`

- Severity: HIGH
- Section: `Summary / Core Logic / Verification Results`
- Type: Correctness | Contract
- Problem:
  `Uart::reset()` in `xemu/xcore/src/device/uart.rs` now clears `rx_fifo` and the shared `rx_buf` in place, which correctly preserves the background TCP reader thread. But the reader thread is still an independent `BufReader<TcpStream>` loop. Bytes that were already sitting in the socket receive buffer, or already prefetched into the `BufReader`, are not discarded by `reset()`. They can still be delivered into `rx_buf` after reset and then reach the guest on the next `tick()`.
- Why it matters:
  `04_IMPL.md` explicitly claims "Pre-reset data is cleared deterministically" and uses that claim to close the previous blocker. The current implementation only clears emulator-side buffers; it does not establish a real reset boundary for live TCP input. Under an active sender, stale host input can still leak into the post-reset guest session.
- Recommendation:
  Either weaken the documented contract to "emulator-side buffers are cleared; in-flight TCP bytes are not guaranteed to be dropped", or make reset tear down / recreate the TCP backend so the reset boundary is actually enforceable. If deterministic flush remains the intended behavior, add a regression test that sends multiple bytes across the reset boundary without first draining them all into `rx_fifo`.

### IR-002 `04_IMPL.md still does not match the required verification workflow or actual results`

- Severity: MEDIUM
- Section: `Summary / Verification Results`
- Type: Validation | Documentation
- Problem:
  `04_IMPL.md` reports `267 tests` and lists `cargo fmt / cargo clippy / cargo build / cargo test`. The repository instructions for this feature require `make fmt`, `make clippy`, `make run`, and `make test`, and this round's actual `make test` result was `273 passed`.
- Why it matters:
  The implementation artifact is supposed to be the acceptance record for the round. If it does not match the required workflow or the actual observed results, it is not reliable enough to approve as the round's verification evidence.
- Recommendation:
  Update `04_IMPL.md` so the verification section records the actual `make` workflow and the actual round-04 results, including the current `make run` behavior when quitting from `xdb>`.

### IR-003 `reset_clears_rx_and_preserves_backend` is still described more strongly than it tests

- Severity: MEDIUM
- Section: `Code Changes / Verification Results`
- Type: Validation
- Problem:
  `04_IMPL.md` describes `reset_clears_rx_and_preserves_backend` as verifying post-reset backend continuity, but the test still uses `setup() -> Uart::new()` and direct buffer mutation. It proves local buffer clearing on the TX-only UART, not behavior of a live TCP backend.
- Why it matters:
  This makes the round's evidence look broader than it really is. The live-backend coverage comes from `tcp_reset_preserves_live_backend`, while `reset_clears_rx_and_preserves_backend` is only a local-buffer reset test.
- Recommendation:
  Narrow the wording in `04_IMPL.md` so each test is described according to what it really covers, or rename the local-buffer test so it does not imply live backend semantics.

---

## Positive Notes

- The round does directly remove the round-03 `Arc` swap bug: post-reset TCP RX can continue because the background thread and the device still share the same `rx_buf`.
- Amending `docs/dev/DEV_PLAN.md` resolves the previous scope mismatch between the shipped default machine and the approved Phase 4 contract.
- The new live-backend test is materially better than the round-03 local-only reset coverage.

---

## Approval Conditions

### Must Fix
- IR-001

### Should Improve
- IR-002
- IR-003

### Ready for Merge / Release
- No
- Reason: The round still does not meet its own documented "deterministic pre-reset clear" guarantee for a live TCP-backed UART.

---

## References

- Approved plan: `docs/dev/DEV_PLAN.md`
- QEMU `virt` machine docs: https://www.qemu.org/docs/master/system/riscv/virt.html
- RISC-V ACLINT specification: https://github.com/riscv/riscv-aclint/blob/main/riscv-aclint.adoc
