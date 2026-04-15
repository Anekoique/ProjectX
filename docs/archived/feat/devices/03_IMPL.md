# `Device Emulation` IMPL `03`

> Status: Ready for Review
> Feature: `dev`
> Iteration: `03`
> Owner: Executor
> Based on:
>
> - Approved Plan: `DEV_PLAN.md`
> - Related Review: `02_IMPL_REVIEW.md`
> - Related Master: `02_IMPL_MASTER.md`

---

## Summary

This round fixes both blocking issues from `02_IMPL_REVIEW`:

- **IR-001**: Default machine reverted to `Uart::new()` (TX-only, deterministic). G-3b is available via `Uart::with_tcp()` but explicitly recorded as opt-in, not default-shipped. This eliminates the environment-dependent behavior.
- **IR-002**: UART `reset()` now swaps `rx_buf` to a fresh `Arc<Mutex<VecDeque>>`, orphaning pre-reset data deterministically. No race window, no `try_lock`.

266 tests, 0 failures, fmt clean, clippy clean (0 warnings).

## Implementation Scope

[**Completed**]
- IR-001: Default machine uses `Uart::new()` (TX-only). G-3b downgraded to "opt-in via `with_tcp()`" — not a default-shipped feature.
- IR-002: UART reset swaps `rx_buf` to fresh buffer (generation swap). Pre-reset backend data is orphaned deterministically.
- New test: `reset_deterministically_clears_rx` — verifies both frontend and backend are clean after reset.

[**Scope Clarification**]
- G-3b (UART RX via TCP): **Available but opt-in.** `Uart::with_tcp(port)` works correctly and is tested, but is not wired into the default machine because a fixed-port TCP listener would silently degrade on port conflicts. Users who need RX construct their Bus explicitly with `Uart::with_tcp()`.

[**Not Implemented**]
- IM-004 (MAYBE: bare-metal application tests): Deferred.

---

## Plan Compliance

[**Implemented as Planned**]
- G-1 through G-4, G-5: All delivered
- G-3b: Available via API, not default-wired (see deviation)

[**Deviations from Plan**]
- D-001: Default machine uses `Uart::new()` (TX-only) instead of `Uart::with_tcp()`
  - Reason: A hardwired TCP port is non-deterministic — port conflicts silently degrade to TX-only, making "shipped" RX unreliable. TX-only is always correct.
  - Impact: G-3b is opt-in. Users needing RX must construct with `Uart::with_tcp(port)`.
- D-002: UART reset uses generation swap instead of `try_lock().clear()`
  - Reason: `try_lock()` is racy — if background thread holds lock, reset skips clearing. Generation swap orphans pre-reset data deterministically.
  - Impact: After reset, background thread writes to orphaned buffer. New `tick()` drains from fresh buffer. No stale data possible.

[**Unresolved Gaps**]
- None

---

## Code Changes

[**Modules / Files**]
- `device/uart.rs`: `reset()` swaps `rx_buf` to fresh `Arc<Mutex<VecDeque>>`. Added `reset_deterministically_clears_rx` test. Removed `tcp_rx_receives_data` test (redundant with `tick_drains_rx_buf`).
- `cpu/riscv/mod.rs`: Default UART reverted to `Uart::new()`.

[**Core Logic**]
- UART reset (generation swap):
  ```rust
  fn reset(&mut self) {
      // ... register resets ...
      self.rx_fifo.clear();
      self.rx_buf = Arc::new(Mutex::new(VecDeque::new())); // orphan old buffer
  }
  ```
  Background thread keeps writing to old `Arc`. `tick()` drains from new empty `Arc`. Pre-reset data is unreachable.

---

## Verification Results

[**Formatting / Lint / Build**]
- `cargo fmt`: Pass
- `cargo clippy`: Pass (0 warnings)
- `cargo build`: Pass
- `cargo test`: Pass (266 tests, 0 failures)

[**Key Tests**]
- `reset_deterministically_clears_rx`: Pre-reset data in both `rx_buf` and `rx_fifo` → reset → tick → rx_fifo is empty.
- `tcp_bind_failure_falls_back_to_tx_only`: Port conflict → TX-only, no panic.
- `tcp_disconnect_stops_rx_tx_continues`: Connect → send → disconnect → data arrives → no more data after disconnect → TX still works.
- `sswi_edge_delivered_once_and_clearable`: CPU-level SSWI regression.

---

## Acceptance Mapping

| Goal / Constraint | Status | Evidence |
|-------------------|--------|----------|
| G-1 ACLINT | Pass | 11 aclint tests |
| G-2 PLIC | Pass | 13 plic tests |
| G-3a UART TX | Pass | UART register tests, default machine wiring |
| G-3b UART RX | Opt-in | `with_tcp()` tested; not default-wired (D-001) |
| G-4 TestFinisher | Pass | 4 tests (`#[cfg(test)]`) |
| G-5 IrqState | Pass | ACLINT/PLIC irq assertions |
| C-8 TCP | Pass | `tcp_bind_failure_*`, `tcp_disconnect_*` |
| IR-001 | Pass | Default is `Uart::new()` — deterministic, no port dependency |
| IR-002 | Pass | `reset_deterministically_clears_rx` — generation swap |
| I-6 SSWI edge | Pass | `sswi_edge_delivered_once_and_clearable` |

---

## Known Issues

- K-001: G-3b UART RX is opt-in via `Uart::with_tcp(port)`, not wired into the default machine. This is a deliberate scope decision to avoid environment-dependent behavior.

## Next Action

- Ready for `03_IMPL_REVIEW`
