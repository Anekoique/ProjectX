# `Device Emulation` IMPL `01`

> Status: Ready for Review
> Feature: `dev`
> Iteration: `01`
> Owner: Executor
> Based on:
>
> - Approved Plan: `DEV_PLAN.md`
> - Related Review: `00_IMPL_REVIEW.md`
> - Related Master: `00_IMPL_MASTER.md`

---

## Summary

This round fixes all 4 blocking issues and 2 non-blocking issues from `00_IMPL_REVIEW`, and applies all MUST/SHOULD directives from `00_IMPL_MASTER`.

Key fixes: SSWI edge-triggered delivery via shared `AtomicBool`, device reset on `RVCore::reset()`, PLIC evaluate after claim, ProgramExit code preserved, `IrqState` newtype, PLIC named constants, TestFinisher moved to `#[cfg(test)]`.

265 tests passing, 0 failures, fmt clean, clippy clean (0 warnings).

## Implementation Scope

[**Completed**]
- IR-002: ProgramExit code preserved — `halt_ret` set after `set_terminated()`
- IR-003: SSWI edge-triggered — `Arc<AtomicBool>` shared between Aclint and Bus; SSIP removed from `HW_IP_MASK`; one-shot delivery in `step()`
- IR-004: Device reset — `Device::reset()` trait method, `Bus::reset_devices()`, implemented for Aclint/Plic/Uart, called from `RVCore::reset()`
- IR-005: PLIC evaluate after claim — `claim()` calls `evaluate()` to clear stale MEIP/SEIP
- IM-001: ACLINT/PLIC wiring is arch-specific (in `RVCore::new()`), devices remain in `device/` module
- IM-002: PLIC named constants (`PRIORITY_END`, `PENDING_OFF`, `ENABLE_BASE/STRIDE/END`, `THRESHOLD_BASE`, `CLAIM_BASE`, `CTX_STRIDE`), helper functions (`is_threshold`, `is_claim`, `ctx_of`)
- IM-003: `IrqState` newtype wrapping `Arc<AtomicU64>` with `set`/`clear`/`load`/`reset` methods
- IM-005: TestFinisher behind `#[cfg(test)]`, zero clippy warnings
- Bug fix: PLIC `ctx_of` was using `CTX_STRIDE` for enable registers instead of `ENABLE_STRIDE`

[**Partially Completed**]
- IR-001: Default machine ships `Uart::new()` (TX-only). `Uart::with_tcp(port)` available for opt-in RX. This is intentional — avoids spawning TCP threads for cpu-tests/xdb. Documented as D-001.

[**Not Implemented**]
- IM-004 (MAYBE: remove XError::ProgramExit): Kept. It's the cleanest device-to-CPU halt signal without polling flags each step.

---

## Plan Compliance

[**Implemented as Planned**]
- Device trait: `read(&mut self)`, `tick()`, `irq_line()`, `notify()`, `reset()`
- Bus: `plic_idx`, `ssip_pending`, `tick()`, `reset_devices()`, `take_ssip()`
- ACLINT: MSWI/MTIMER/SSWI with edge-triggered SSIP via shared `AtomicBool`
- PLIC: level-triggered, claimed-exclusion, evaluate after claim
- UART: TX stdout, RX TCP (opt-in), DLAB, LSR/IIR, byte-only
- Memory map: ACLINT 0x0200_0000/0x1_0000, PLIC 0x0C00_0000/0x400_0000, UART 0x1000_0000/0x100

[**Deviations from Plan**]
- D-001: Default machine uses `Uart::new()` (TX-only) instead of `Uart::with_tcp()`
  - Reason: Avoids spawning TCP listener for cpu-tests/xdb. Users opt-in via `with_tcp()`.
  - Impact: Default machine has no TCP RX. G-3b tested via unit tests only.
- D-002: SSWI uses shared `Arc<AtomicBool>` instead of `irq_state` bit
  - Reason: IR-003 — `irq_state` merge in `sync_interrupts()` would override guest CSR clears of SSIP. Edge delivery requires one-shot path outside `HW_IP_MASK`.
  - Impact: Bus gains `ssip_pending: Arc<AtomicBool>` and `take_ssip()`. Aclint takes `ssip` parameter at construction.
- D-003: `IrqState` newtype replaces raw `Arc<AtomicU64>`
  - Reason: IM-003 directive.
  - Impact: All `irq_state` fields changed to `IrqState`. API: `set`/`clear`/`load`/`reset`.

[**Unresolved Gaps**]
- None

---

## Code Changes

[**Modules / Files**]
- `device/mod.rs`: Added `Device::reset()`, `IrqState` newtype, removed SSIP from `HW_IP_MASK`, `test_finisher` behind `#[cfg(test)]`
- `device/bus.rs`: Added `ssip_pending: Arc<AtomicBool>`, `ssip_flag()`, `take_ssip()`, `reset_devices()`
- `device/aclint.rs`: SSWI via `Arc<AtomicBool>` instead of `irq_state`, added `reset()`, uses `IrqState`
- `device/plic.rs`: Named constants, `ctx_of` takes stride param (fixes enable bug), `claim()` calls `evaluate()`, added `reset()`, uses `IrqState`
- `device/uart.rs`: Added `reset()`, `#[allow(dead_code)]` on `with_tcp`
- `cpu/riscv/mod.rs`: Uses `IrqState`, `bus.take_ssip()` for SSWI edge in `step()`, `bus.reset_devices()` in `reset()`
- `cpu/mod.rs`: ProgramExit `halt_ret` set after `set_terminated()`

[**Core Logic**]
- SSWI delivery: Aclint writes `ssip: Arc<AtomicBool>` on setssip write → Bus exposes `take_ssip()` → RVCore `step()` consumes flag and sets mip.SSIP once → guest can clear via CSR write
- `sync_interrupts()`: merges only MSIP/MTIP/SEIP/MEIP (not SSIP) from `IrqState` into mip
- PLIC `claim()` → `evaluate()`: clears MEIP/SEIP immediately when last pending source is claimed
- Device reset chain: `RVCore::reset()` → `bus.reset_devices()` → each `Device::reset()`

[**API / Behavior Changes**]
- `Device` trait: added `fn reset(&mut self) {}`
- `Bus`: added `ssip_flag()`, `take_ssip()`, `reset_devices()`
- `Aclint::new()`: takes `(IrqState, Arc<AtomicBool>)` instead of `(Arc<AtomicU64>)`
- `Plic::new()`: takes `IrqState` instead of `Arc<AtomicU64>`
- `RVCore::with_bus()`: takes `(Arc<Mutex<Bus>>, IrqState)` instead of `(Arc<Mutex<Bus>>, Arc<AtomicU64>)`
- `HW_IP_MASK`: no longer includes SSIP bit

---

## Verification Results

[**Formatting / Lint / Build**]
- `cargo fmt`: Pass
- `cargo clippy`: Pass (0 warnings)
- `cargo build`: Pass
- `cargo test`: Pass (265 tests, 0 failures)

[**Unit Tests**]
- ACLINT: 11 tests — mtime advance/frozen, mtimecmp MTIP, msip, setssip edge-triggered (via AtomicBool), unmapped, mtime write ignored, reset
- PLIC: 13 tests — priority, enable, claim highest, claim empty, complete, threshold, claimed-exclusion, re-pend, wrong source, source 0, MEIP/SEIP, claim clears MEIP, reset
- UART: 13 tests — LSR, DR, RBR pop, DLAB, IER, IIR, irq_line, scratch, non-byte error, tick drains, TCP bind failure, TCP rx, TCP disconnect
- TestFinisher: 4 tests (under `#[cfg(test)]`)
- Bus: plic_idx test

[**Integration Tests**]
- 224 existing tests continue passing (no regressions)

[**Failure / Robustness Validation**]
- ACLINT unmapped → 0: Pass
- PLIC claim empty → 0: Pass
- UART non-byte → BadAddress: Pass
- PLIC complete wrong source → no change: Pass
- ACLINT mtime write ignored: Pass
- TCP bind failure → TX-only: Pass

[**Edge Case Validation**]
- mtimecmp = MAX → no timer: Pass
- PLIC source 0 excluded: Pass
- UART all offsets both DLAB: Pass
- SSWI write 0 → no SSIP: Pass
- SSWI edge consumed after take: Pass

---

## Acceptance Mapping

| Goal / Constraint | Status | Evidence |
|-------------------|--------|----------|
| G-1 MSWI | Pass | `aclint::tests::msip_set_and_clear` |
| G-1 MTIMER | Pass | `aclint::tests::mtimecmp_*`, `mtime_*` |
| G-1 SSWI | Pass | `aclint::tests::setssip_is_edge_triggered`, `setssip_write_zero_no_effect` |
| G-2 PLIC | Pass | 13 plic tests |
| G-3a UART TX | Pass | uart LSR/DLAB/IIR/scratch tests |
| G-3b UART RX | Pass | `uart::tests::tcp_*`, `tick_drains_rx_buf`, `irq_line_rx_data_and_ier` |
| G-4 TestFinisher | Pass | 4 test_finisher tests (`#[cfg(test)]`) |
| G-5 IrqState | Pass | ACLINT/PLIC tests assert IrqState bits |
| C-1 Layout | Pass | Verified in `RVCore::new()` |
| C-3 UART byte-only | Pass | `uart::tests::non_byte_access_error` |
| C-5 mtime pause | Pass | `aclint::tests::mtime_frozen_without_tick` |
| C-8 TCP bind fallback | Pass | `uart::tests::tcp_bind_failure_falls_back_to_tx_only` |
| C-8 TCP disconnect | Pass | `uart::tests::tcp_disconnect_stops_rx_tx_continues` |
| I-2 step ordering | Pass | Code: tick → take_ssip → sync → check → execute → retire |
| I-3 claimed-exclusion | Pass | `plic::tests::claimed_source_not_repended`, `source_repended_after_complete` |
| I-6 SSWI edge | Pass | `aclint::tests::setssip_is_edge_triggered` |
| IR-002 ProgramExit | Pass | `halt_ret` set after `set_terminated()` |
| IR-003 SSWI | Pass | Edge delivery via `AtomicBool`, SSIP removed from `HW_IP_MASK` |
| IR-004 Device reset | Pass | `aclint::tests::reset_clears_state`, `plic::tests::reset_clears_state` |
| IR-005 Evaluate after claim | Pass | `plic::tests::claim_clears_meip_when_last_source` |

---

## Known Issues

- K-001: Default machine uses `Uart::new()` (TX-only). TCP RX requires explicit `Uart::with_tcp(port)` construction. This is intentional (D-001).

## Next Action

- Ready for `01_IMPL_REVIEW`
