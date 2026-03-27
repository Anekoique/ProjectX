# `Device Emulation` IMPL REVIEW `00`

> Status: Open
> Feature: `dev`
> Iteration: `00`
> Owner: Reviewer
> Target Impl: `00_IMPL.md`

---

## Verdict

- Decision: Rejected
- Blocking Issues: `4`
- Non-Blocking Issues: `2`

## Summary

The implementation adds the planned device modules and materially improves local test coverage, but the round is not yet acceptable as the shipped Phase 4 result. The default machine still does not expose the approved UART RX path, the SSWI behavior is incorrect against ACLINT semantics, reset leaves MMIO device state behind, and the `ProgramExit` path can misreport guest exit status. `00_IMPL.md` also overstates delivered scope and validation for the current code.

---

## Findings

### IR-001 `Default machine still ships TX-only UART`

- Severity: HIGH
- Section: `Implementation Scope / Plan Compliance / Acceptance Mapping`
- Type: Correctness | Plan Compliance
- Problem:
  `RVCore::new()` registers `Box::new(Uart::new())` in `xemu/xcore/src/cpu/riscv/mod.rs`, so the default machine never enables the TCP RX backend that `DEV_PLAN.md` approved and that `00_IMPL.md` marks as `G-3b Pass`.
- Why it matters:
  The actual emulator binary does not provide the approved UART RX -> PLIC -> CPU path in normal runs. This is a missing feature in the default machine, not only a test limitation, and it makes the acceptance mapping in `00_IMPL.md` inaccurate.
- Recommendation:
  Either wire a configurable `Uart::with_tcp(...)` backend into the default machine, or reopen / amend the implementation scope and downgrade `G-3b` to partial with an explicit unresolved gap.

### IR-002 `ProgramExit code is overwritten before termination is recorded`

- Severity: HIGH
- Section: `Code Changes / API / Behavior Changes`
- Type: Correctness
- Problem:
  In `xemu/xcore/src/cpu/mod.rs`, `CPU::step()` stores `ProgramExit(code)` into `self.halt_ret` and then immediately calls `set_terminated(...)`, but `set_terminated()` overwrites `halt_ret` with `self.core.halt_ret()` (`a0`). The explicit device exit code is therefore lost.
- Why it matters:
  Test-finisher exits can be reported with the wrong status or wrong exit code. This affects `log_termination()`, `is_exit_normal()`, and any workflow that relies on the emulator's reported pass / fail result.
- Recommendation:
  Preserve the explicit `ProgramExit` code when terminating. The cleanest fix is to add a helper that records `state + halt_pc + halt_ret` together, instead of reusing `set_terminated()` for both core-halting and device-exit paths.

### IR-003 `SSWI is modeled as a sticky level instead of an edge`

- Severity: HIGH
- Section: `Core Logic / API / Behavior Changes`
- Type: Correctness | Behavior
- Problem:
  `xemu/xcore/src/device/aclint.rs` handles `SETSSIP` by setting `irq_state |= SSIP`, and `xemu/xcore/src/cpu/riscv/mod.rs` merges `irq_state` back into `mip` on every step. Under this design, once `SETSSIP` is written, SSIP is reasserted forever unless the device state is reset.
- Why it matters:
  The ACLINT SSWI register is edge-sensitive: writing `1` causes the hart to set SSIP, and SSIP itself is then software-clearable through `mip` / `sip`. The current implementation prevents software from clearing SSIP and can leave the guest stuck with a permanent supervisor software interrupt pending state.
- Recommendation:
  Do not model `SETSSIP` as a persistent external level in `irq_state`. It needs a one-shot delivery path that lets the hart set SSIP once while still allowing guest software to clear the writable SSIP CSR bit afterward. This likely requires reopening the approved architecture, because the current `irq_state -> sync_interrupts()` design is not correct for SSWI semantics.

### IR-004 `Reset leaves MMIO device state stale`

- Severity: HIGH
- Section: `Plan Compliance / Behavior`
- Type: Correctness | Regression
- Problem:
  `RVCore::reset()` in `xemu/xcore/src/cpu/riscv/mod.rs` resets CPU-local state and clears `irq_state`, but it does not reset device state in the `Bus`. ACLINT registers, PLIC pending / claimed / enable state, and UART FIFOs / RX buffers can survive a debugger reset.
- Why it matters:
  `xdb reset` no longer approximates a machine reset. State can leak across runs, producing stale interrupts, stale timer state, or stale serial input that did not exist in the guest image being rerun.
- Recommendation:
  Add an explicit reset path for devices, such as `Device::reset()` plus `Bus::reset()`, or rebuild the MMIO graph during reset. The fix should include reset-focused tests for ACLINT, PLIC, and UART.

### IR-005 `PLIC does not reevaluate external interrupt outputs after claim`

- Severity: MEDIUM
- Section: `Core Logic`
- Type: Correctness | Behavior
- Problem:
  In `xemu/xcore/src/device/plic.rs`, `claim()` clears `pending` and records the claimed source, but only `complete()` and `notify()` call `evaluate()`. If software claims the last pending source, `MEIP` / `SEIP` can remain asserted in `irq_state` until some later complete or tick path happens.
- Why it matters:
  The guest-visible external interrupt pending state becomes stale after a claim. Even if many handlers complete quickly, this is still a visible divergence from the controller's pending state.
- Recommendation:
  Recompute context outputs immediately after claim and add a regression test for "single pending source claimed, no completion yet".

### IR-006 `00_IMPL.md overstates acceptance and required verification`

- Severity: MEDIUM
- Section: `Verification Results / Acceptance Mapping`
- Type: Validation | Plan Compliance
- Problem:
  `00_IMPL.md` reports `Unresolved Gaps: None` and marks `G-3b` and `C-8 TCP disconnect` as fully passing, but the shipped default machine omits the TCP RX backend and there is still no end-to-end proof of UART RX through `RVCore -> Bus -> PLIC -> mip`. Its verification section also reports `cargo` commands instead of the required `make fmt`, `make clippy`, `make run`, and `make test` workflow from `AGENTS.md`.
- Why it matters:
  `NN_IMPL.md` is supposed to be a reliable acceptance record for the round. Right now it overclaims delivered behavior and does not reflect the required repository-level validation contract.
- Recommendation:
  Update `00_IMPL.md` after fixes land so it reflects the actual shipped behavior, records the UART default-machine gap accurately, and includes the required `make` verification results.

---

## Positive Notes

- The `Bus::tick() + plic_idx + Device::notify()` structure is materially cleaner than the earlier downcast-based alternatives and keeps device coupling low.
- The new ACLINT / PLIC / UART unit tests make the local device logic much easier to audit than in earlier rounds.

---

## Approval Conditions

### Must Fix
- IR-001
- IR-002
- IR-003
- IR-004

### Should Improve
- IR-005
- IR-006

### Ready for Merge / Release
- No
- Reason: The current code still has blocking correctness issues in shipped device behavior, reset semantics, and exit reporting.

---

## References

- Approved plan: `docs/dev/DEV_PLAN.md`
- QEMU `virt` machine docs: https://www.qemu.org/docs/master/system/riscv/virt.html
- RISC-V ACLINT specification: https://github.com/riscv/riscv-aclint/blob/main/riscv-aclint.adoc
- RISC-V Privileged Architecture manual: https://riscv.github.io/riscv-isa-manual/snapshot/privileged
