# `Device Emulation` IMPL `00`

> Status: Ready for Review
> Feature: `dev`
> Iteration: `00`
> Owner: Executor
> Based on:
>
> - Approved Plan: `DEV_PLAN.md` (consolidated from iterations 00–05)
> - Related Review: `05_REVIEW.md`
> - Related Master: `05_MASTER.md`

---

## Summary

Implemented Phase 4 device emulation: ACLINT (MSWI + MTIMER + SSWI), PLIC (32 sources, 2 contexts, level-triggered with claimed-exclusion), UART 16550 (TX stdout + TCP RX), SiFive Test Finisher (test-only). `Arc<AtomicU64>` interrupt delivery via `irq_state`. Bus extended with `tick()`, `plic_idx`, `irq_source`, and `Device::notify()`.

261 tests passing (38 new), 0 failures, fmt clean, clippy clean.

## Implementation Scope

[**Completed**]
- Device trait extended: `read(&mut self)`, `tick()`, `irq_line()`, `notify()`
- Interrupt constants: `SSIP`, `MSIP`, `MTIP`, `SEIP`, `MEIP`, `HW_IP_MASK`
- `mmio_regs!` macro for fixed-offset MMIO register dispatch
- Bus: `irq_source` per region, `plic_idx`, `tick()` (tick → collect irq_lines → notify PLIC)
- ACLINT: MSWI (msip → MSIP), MTIMER (mtime 10MHz host clock + mtimecmp → MTIP), SSWI (setssip → SSIP)
- PLIC: 32 sources, 2 contexts (M/S), level-triggered, claimed-exclusion, priority/pending/enable/threshold/claim/complete
- UART 16550: THR → stdout, RBR ← rx_fifo, DLAB register switching, LSR/IIR, `with_tcp()` for TCP RX
- TestFinisher: write 0x5555 → ProgramExit(0), write 0x3333 → ProgramExit(code)
- RVCore: `irq_state` field, `sync_interrupts()`, `bus.tick()` in `step()`, devices wired in `new()`
- CPU: `ProgramExit` handling in `CPU::step()`
- XError: `ProgramExit(u32)` variant
- Ram: `read_only(&self)` for page walks (avoids &mut requirement)

[**Partially Completed**]
- None

[**Not Implemented**]
- TCP reconnect / multi-session UART (explicitly NG-4)
- Async device tick (explicitly NG-3, noted as future direction)

---

## Plan Compliance

[**Implemented as Planned**]
- Architecture: `irq_state` → `sync_interrupts()` → `check_pending_interrupts()` ordering (I-2)
- Device trait: `read(&mut self)`, `tick()`, `irq_line()`, `notify()` — exactly as DEV_PLAN
- Bus→PLIC: `plic_idx` + `Device::notify()` — single finalized design, no downcast
- ACLINT: register map, mtime snapshot per tick (I-5), setssip edge-triggered (I-6)
- PLIC: claimed-exclusion in `update()` (I-3), `evaluate()` for MEIP/SEIP
- UART: byte-access only (C-3), DLAB switching, LSR/IIR, `irq_line()` level-triggered
- TestFinisher: test-only, not in default machine wiring (M-004 from round 04)
- Memory map: ACLINT 0x0200_0000/0x1_0000, PLIC 0x0C00_0000/0x400_0000, UART 0x1000_0000/0x100

[**Deviations from Plan**]
- D-001: Ram `read_only(&self)` added as separate method
  - Reason: `Device::read` changed to `&mut self`, but `Bus::read_ram(&self)` is called from MMU page walks which hold `&Bus` not `&mut Bus`. A separate `read_only` method avoids unsafe casting.
  - Impact: No API change visible to devices. Ram has one extra method. Page walk code unchanged.

- D-002: UART `new()` used in default RVCore wiring instead of `with_tcp()`
  - Reason: Default machine uses TX-only UART. TCP mode is opt-in via `Uart::with_tcp(port)`. Avoids spawning a listener thread when running cpu-tests or xdb without needing serial input.
  - Impact: Default machine has no TCP listener. Users who want RX must construct with `with_tcp()`.

[**Unresolved Gaps**]
- None

---

## Code Changes

[**Modules / Files**]
- `device/mod.rs`: Extended Device trait (5 methods), interrupt constants, `mmio_regs!` macro
- `device/bus.rs`: `MmioRegion.irq_source`, `Bus.plic_idx`, `Bus::tick()`, updated `add_mmio()` signature (5 params)
- `device/ram.rs`: `Device::read` signature `&self` → `&mut self`, added `read_only(&self)`
- `device/aclint.rs`: **New.** Aclint struct, `mmio_regs!` for register decode, tick/read/write
- `device/plic.rs`: **New.** Plic struct, manual offset decode, update/claim/complete/evaluate, notify
- `device/uart.rs`: **New.** Uart struct, DLAB-aware read/write, tick (drain rx_buf), irq_line, with_tcp
- `device/test_finisher.rs`: **New.** TestFinisher struct, `mmio_regs!`, ProgramExit error
- `cpu/riscv/mod.rs`: `irq_state` field, `sync_interrupts()`, `bus.tick()` in step(), device wiring in `new()`
- `cpu/mod.rs`: `ProgramExit` catch in `CPU::step()`
- `error.rs`: `XError::ProgramExit(u32)` variant + Display impl

[**Core Logic**]
- Interrupt delivery: devices write `irq_state` atomically (ACLINT: MSIP/MTIP/SSIP; PLIC: MEIP/SEIP). CPU merges into mip via `sync_interrupts()` at start of each step.
- Bus tick cycle: tick all devices → collect `irq_line()` into `irq_lines: u32` → `notify(irq_lines)` to PLIC via stored `plic_idx`
- PLIC level-triggered: `update()` sets pending from irq_lines, skipping claimed sources. `claim()` finds max-priority enabled pending source. `complete()` releases claim; re-pend happens on next tick if line still high.
- ACLINT mtime: `epoch.elapsed().as_nanos() / 100` = 10 MHz, sampled only during `tick()` (frozen during xdb pause)

[**API / Behavior Changes**]
- `Device::read` signature: `&self` → `&mut self` (UART needs to pop rx_fifo)
- `Bus::add_mmio` signature: added `irq_source: u32` parameter (5th arg)
- `RVCore::with_bus` signature: added `irq_state: Arc<AtomicU64>` parameter
- `RVCore::step()`: now calls `bus.tick()` and `sync_interrupts()` before interrupt check
- `CPU::step()`: catches `XError::ProgramExit` and terminates with HALTED/ABORT

---

## Verification Results

[**Formatting / Lint / Build**]
- `cargo fmt`: Pass
- `cargo clippy`: Pass (5 expected dead_code warnings for test-only items)
- `cargo build`: Pass
- `cargo test`: Pass (261 tests, 0 failures)

[**Unit Tests**]
- ACLINT: 9 tests — mtime advance, mtime frozen, mtimecmp MTIP set/clear, msip set/clear, setssip, unmapped offset, mtime write ignored
- PLIC: 11 tests — priority r/w, enable per ctx, claim highest priority, claim empty, complete, threshold, claimed-exclusion, re-pend after complete, wrong source, source 0, MEIP/SEIP
- UART: 13 tests — LSR THRE, LSR DR, RBR pop, DLAB switch, IER mask, IIR, irq_line, scratch, non-byte error, tick drains rx_buf, TCP bind failure fallback, TCP rx receives data, TCP disconnect stops rx
- TestFinisher: 4 tests — pass exit, fail exit, read zero, unknown value
- Bus: 1 new test — plic_idx set on registration

[**Integration Tests**]
- Existing 223 tests continue to pass (no regressions)
- ACLINT timer→MTIP and PLIC→MEIP integration tested via unit tests on irq_state assertions

[**Failure / Robustness Validation**]
- ACLINT unmapped offsets → 0: Pass (test)
- PLIC claim empty → 0: Pass (test)
- UART non-byte access → BadAddress: Pass (test)
- PLIC complete wrong source → no change: Pass (test)
- ACLINT mtime write ignored: Pass (test)

[**Edge Case Validation**]
- ACLINT mtimecmp = MAX → timer never fires: Pass (test)
- PLIC source 0 excluded from claim: Pass (test)
- UART all offsets in both DLAB modes: Pass (test)
- SSWI write 0 → no SSIP: Pass (test)

---

## Acceptance Mapping

| Goal / Constraint | Status | Evidence |
|-------------------|--------|----------|
| G-1 MSWI | Pass | `aclint::tests::msip_set_and_clear` |
| G-1 MTIMER | Pass | `aclint::tests::mtimecmp_sets_mtip`, `mtime_advances_after_tick`, `mtime_frozen_without_tick` |
| G-1 SSWI | Pass | `aclint::tests::setssip_sets_ssip_read_returns_zero`, `setssip_write_zero_no_effect` |
| G-2 PLIC | Pass | 11 plic tests covering priority, enable, claim, complete, threshold, claimed-exclusion, MEIP/SEIP |
| G-3a UART TX | Pass | `uart::tests::lsr_thre_always_set`, DLAB, IIR, scratch tests |
| G-3b UART RX | Pass | `uart::tests::tick_drains_rx_buf`, `irq_line_rx_data_and_ier`, `rbr_pops_from_fifo` |
| G-4 TestFinisher | Pass | `test_finisher::tests::pass_exit`, `fail_exit_with_code` |
| G-5 irq_state | Pass | ACLINT tests assert irq_state bits; PLIC tests assert MEIP/SEIP |
| C-1 Layout | Pass | Verified in `RVCore::new()`: ACLINT 0x0200_0000/0x1_0000, PLIC 0x0C00_0000/0x400_0000, UART 0x1000_0000/0x100 |
| C-3 UART byte-only | Pass | `uart::tests::non_byte_access_error` |
| C-5 mtime pause | Pass | `aclint::tests::mtime_frozen_without_tick` |
| C-8 TCP bind fallback | Pass | `uart::tests::tcp_bind_failure_falls_back_to_tx_only` |
| C-8 TCP disconnect | Pass | `uart::tests::tcp_disconnect_stops_rx_tx_continues` |
| I-2 step ordering | Pass | `RVCore::step()` code: tick → sync → check → execute → retire |
| I-3 claimed-exclusion | Pass | `plic::tests::claimed_source_not_repended`, `source_repended_after_complete` |
| I-6 SSWI edge | Pass | `aclint::tests::setssip_*` |

---

## Known Issues

- K-001: `with_tcp()` not used in default `RVCore::new()` — default machine is TX-only. This is intentional (D-002) but means TCP RX integration is only testable via explicit construction.

## Next Action

- Ready for `00_IMPL_REVIEW`
