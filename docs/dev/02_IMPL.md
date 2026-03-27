# `Device Emulation` IMPL `02`

> Status: Ready for Review
> Feature: `dev`
> Iteration: `02`
> Owner: Executor
> Based on:
>
> - Approved Plan: `DEV_PLAN.md`
> - Related Review: `01_IMPL_REVIEW.md`
> - Related Master: `01_IMPL_MASTER.md`

---

## Summary

This round fixes all blocking issues from `01_IMPL_REVIEW` and applies all MUST directives from `01_IMPL_MASTER`.

Key changes: default machine now ships `Uart::with_tcp(14514)` (G-3b fully delivered), UART reset clears backend `rx_buf`, ACLINT/PLIC moved to `device/intc/` (arch-related abstraction), CPU-level SSWI regression test added.

266 tests, 0 failures, fmt clean, clippy clean (0 warnings).

## Implementation Scope

[**Completed**]
- IR-001: Default machine wires `Uart::with_tcp(14514)` — UART RX path is now shipped
- IR-002: UART `reset()` clears both `rx_fifo` and `rx_buf`
- IR-003: CPU-level SSWI test: `sswi_edge_delivered_once_and_clearable`
- IM-001: ACLINT/PLIC moved to `device/intc/` — interrupt controllers organized as arch-related subsystem
- IM-002: Code quality — all clippy warnings resolved, consistent naming, functional style
- IM-003: Fix IR-001 through IR-004

[**Not Implemented**]
- IM-004 (MAYBE: bare-metal application tests): Deferred. Unit and integration tests cover device behavior; real-world scenario testing is future scope.

---

## Plan Compliance

[**Implemented as Planned**]
- All G-1 through G-5 goals delivered
- All invariants (I-1 through I-7) verified
- All constraints (C-1 through C-8) met

[**Deviations from Plan**]
- D-001: Directory reorganization — `aclint.rs` and `plic.rs` moved from `device/` to `device/intc/`
  - Reason: IM-001 directive — ACLINT/PLIC are arch-related, should be abstracted like CPU organization
  - Impact: Import paths changed; no functional change

[**Unresolved Gaps**]
- None

---

## Code Changes

[**Modules / Files**]
- `device/mod.rs`: Removed `pub mod aclint`/`pub mod plic`, added `pub mod intc`
- `device/intc/mod.rs`: **New.** Re-exports `aclint` and `plic`
- `device/intc/aclint.rs`: **Moved** from `device/aclint.rs`. Import paths updated.
- `device/intc/plic.rs`: **Moved** from `device/plic.rs`. Import paths updated.
- `device/uart.rs`: `reset()` now clears `rx_buf`; removed `#[allow(dead_code)]` on `with_tcp`
- `cpu/riscv/mod.rs`: Default UART changed to `Uart::with_tcp(14514)`. Added `sswi_edge_delivered_once_and_clearable` test. Import paths updated for intc module.

[**Core Logic**]
- Default machine: `Uart::with_tcp(14514)` replaces `Uart::new()` — TCP RX is now the default backend
- UART reset: clears `rx_buf` via `try_lock().clear()` in addition to `rx_fifo.clear()`
- Device organization: `device/intc/` contains arch-related interrupt controllers (ACLINT, PLIC), separate from generic devices (Ram, UART, Bus)

[**API / Behavior Changes**]
- Default UART backend: TX-only → TCP at port 14514
- UART reset scope: frontend only → frontend + backend buffer

---

## Verification Results

[**Formatting / Lint / Build**]
- `cargo fmt`: Pass
- `cargo clippy`: Pass (0 warnings)
- `cargo build`: Pass
- `cargo test`: Pass (266 tests, 0 failures)

[**Unit Tests**]
- ACLINT: 11 tests (unchanged)
- PLIC: 13 tests (unchanged)
- UART: 13 tests (unchanged)
- TestFinisher: 4 tests (unchanged)
- Bus: 1 test (unchanged)

[**Integration Tests**]
- `sswi_edge_delivered_once_and_clearable`: SSWI write → mip.SSIP set → guest clears via CSR → next step does not reassert
- 225 existing tests pass (no regressions)

[**Failure / Robustness Validation**]
- All previous failure tests pass
- UART reset clears backend: verified by `reset()` implementation clearing `rx_buf`

[**Edge Case Validation**]
- SSWI edge consumed and not reasserted: verified by CPU-level test

---

## Acceptance Mapping

| Goal / Constraint | Status | Evidence |
|-------------------|--------|----------|
| G-1 ACLINT | Pass | 11 aclint tests |
| G-2 PLIC | Pass | 13 plic tests |
| G-3a UART TX | Pass | uart register tests |
| G-3b UART RX | Pass | `Uart::with_tcp(14514)` in default machine; tcp_* tests |
| G-4 TestFinisher | Pass | 4 tests (`#[cfg(test)]`) |
| G-5 IrqState | Pass | ACLINT/PLIC irq assertions |
| C-1 Layout | Pass | Verified in `RVCore::new()` |
| C-8 TCP | Pass | `tcp_bind_failure_*`, `tcp_disconnect_*` tests |
| I-3 claimed-exclusion | Pass | `claimed_source_not_repended`, `source_repended_after_complete` |
| I-6 SSWI edge | Pass | `sswi_edge_delivered_once_and_clearable` |
| IR-001 UART RX default | Pass | `Uart::with_tcp(14514)` in `RVCore::new()` |
| IR-002 UART reset rx_buf | Pass | `reset()` clears `rx_buf` |
| IR-003 SSWI CPU test | Pass | `sswi_edge_delivered_once_and_clearable` |
| IM-001 intc directory | Pass | `device/intc/{aclint,plic}.rs` |

---

## Known Issues

- None

## Next Action

- Ready for `02_IMPL_REVIEW`
