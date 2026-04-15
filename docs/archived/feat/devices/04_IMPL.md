# `Device Emulation` IMPL `04`

> Status: Ready for Review
> Feature: `dev`
> Iteration: `04`
> Owner: Executor
> Based on:
>
> - Approved Plan: `DEV_PLAN.md` (amended: UART RX scope clarified)
> - Related Review: `03_IMPL_REVIEW.md`
> - Related Master: none (no `03_IMPL_MASTER.md`)

---

## Summary

This round fixes both blocking issues from `03_IMPL_REVIEW`:

- **IR-001**: DEV_PLAN.md amended to officially record "TX-only default, opt-in TCP RX" as the approved scope. This is no longer an undocumented deviation.
- **IR-002**: UART `reset()` changed from generation swap to blocking `lock().clear()`. Pre-reset data is cleared deterministically. The live TCP backend (background thread) is preserved — post-reset RX continues working.
- **IR-003**: New test `tcp_reset_preserves_live_backend` — sends pre-reset data, resets, sends post-reset data, verifies pre-reset data gone and post-reset data arrives.

267 tests, 0 failures, fmt clean, clippy clean (0 warnings).

## Implementation Scope

[**Completed**]
- IR-001: DEV_PLAN.md amended with UART RX scope clarification
- IR-002: UART reset uses blocking `lock().clear()` — deterministic, preserves backend
- IR-003: TCP reset test with live backend
- IR-004: Test count matches actual (267)

[**Not Implemented**]
- IM-004 (MAYBE: bare-metal application tests): Deferred.

---

## Plan Compliance

[**Implemented as Planned**]
- G-1 through G-5: All delivered
- G-3a: UART TX in default machine
- G-3b: UART TCP RX opt-in (per amended DEV_PLAN.md)

[**Deviations from Plan**]
- None. DEV_PLAN.md amended to reflect the TX-only default.

[**Unresolved Gaps**]
- None

---

## Code Changes

[**Modules / Files**]
- `device/uart.rs`: `reset()` changed from `self.rx_buf = Arc::new(...)` (generation swap) to `self.rx_buf.lock().unwrap().clear()` (blocking clear). New test `tcp_reset_preserves_live_backend`. Updated `reset_clears_rx_and_preserves_backend` to verify post-reset backend continuity.
- `docs/dev/DEV_PLAN.md`: Added UART RX scope amendment.

[**Core Logic**]
- UART reset: `self.rx_buf.lock().unwrap().clear()` — blocks until mutex acquired, clears buffer, preserves the `Arc` so the background TCP thread continues writing to the same buffer. Post-reset RX works immediately.

---

## Verification Results

[**Formatting / Lint / Build**]
- `cargo fmt`: Pass
- `cargo clippy`: Pass (0 warnings)
- `cargo build`: Pass
- `cargo test`: Pass (267 tests, 0 failures)

[**Key Tests**]
- `reset_clears_rx_and_preserves_backend`: Local buffer pre-reset data cleared, post-reset data arrives
- `tcp_reset_preserves_live_backend`: TCP connection survives reset — pre-reset data cleared, post-reset bytes arrive via same connection
- `tcp_bind_failure_falls_back_to_tx_only`: Port conflict → TX-only
- `tcp_disconnect_stops_rx_tx_continues`: Disconnect → RX stops, TX works
- `sswi_edge_delivered_once_and_clearable`: CPU-level SSWI regression

---

## Acceptance Mapping

| Goal / Constraint | Status | Evidence |
|-------------------|--------|----------|
| G-1 ACLINT | Pass | 11 aclint tests |
| G-2 PLIC | Pass | 13 plic tests |
| G-3a UART TX | Pass | Default machine, register tests |
| G-3b UART RX | Pass (opt-in) | `tcp_*` tests, DEV_PLAN.md amended |
| G-4 TestFinisher | Pass | 4 tests |
| G-5 IrqState | Pass | irq assertions |
| IR-001 scope | Pass | DEV_PLAN.md amended |
| IR-002 reset | Pass | `tcp_reset_preserves_live_backend` |
| IR-003 test | Pass | `tcp_reset_preserves_live_backend` |

---

## Known Issues

- None

## Next Action

- Ready for `04_IMPL_REVIEW`
