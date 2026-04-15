# `Debian Boot` REVIEW `03`

> Status: Open
> Feature: `debian`
> Iteration: `03`
> Owner: Reviewer
> Target Plan: `03_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Rejected
- Blocking Issues: 2
- Non-Blocking Issues: 1

## Summary

Round 03 resolves the two blockers from round 02 in the right direction. The launcher chain is now explicit, and splitting guest-visible virtio reset from emulator-level snapshot restore is the correct model. The remaining issues are narrower, but they are still implementation-blocking because they sit in the detailed code paths this round now treats as authoritative: the plan still does not say where the per-profile FDT load address lives at runtime after `MachineConfig` has been consumed, and the MMIO feature-selector model is still not compliant with the virtio transport.

## Findings

### R-001 `The FDT load-address path is still not closed in the runtime design`

- Severity: HIGH
- Section: `Phase 1 / Phase 3 / Execution Flow`
- Type: `Flow`
- Problem:
  The plan says `load_firmware` in [`cpu/mod.rs`](/Users/anekoique/ProjectX/xemu/xcore/src/cpu/mod.rs) will use `MachineConfig::fdt_addr()` and that `MachineConfig` is consumed during `init_xcore(config)` to build `Core::with_config(config)`. But the proposed API surface does not store `MachineConfig` or the derived FDT address anywhere in [`CPU`](/Users/anekoique/ProjectX/xemu/xcore/src/cpu/mod.rs), and the current `CoreOps` interface does not expose machine-profile metadata back to `cpu/mod.rs`. In other words, the round removes the old hardcoded `FDT_LOAD_ADDR` but does not yet define where the replacement value actually lives at boot/reset time.
- Why it matters:
  This is still a real runtime gap, not a wording issue. Debian and default profiles need different FDT placement, and `load_firmware()` cannot implement that cleanly unless the chosen address survives past initialization.
- Recommendation:
  The next PLAN should choose one concrete ownership point for the derived FDT address: store it in `CPU`, store it in the arch core and expose an accessor, or keep an explicit boot-layout struct alongside `MachineConfig`. Do not leave `load_firmware()` depending on a config object that no longer exists in scope.

### R-002 `DeviceFeaturesSel and DriverFeaturesSel are still conflated in the detailed MMIO model`

- Severity: HIGH
- Section: `Data Structure / Detailed Code`
- Type: `Spec Alignment`
- Problem:
  The detailed `VirtioBlk` design still uses a single `dev_features_sel` field for both `REG_DEV_FEATURES_SEL` and `REG_DRV_FEATURES_SEL`, and `REG_DRV_FEATURES` indexes `drv_features` through that same selector. In the virtio MMIO transport these are separate registers with separate state.
- Why it matters:
  This is architecturally wrong even if the device currently advertises zero optional feature bits. The plan claims official-spec compliance, and this register model would bake in a non-conformant control path for feature negotiation.
- Recommendation:
  Add a separate `drv_features_sel` field and keep device-feature and driver-feature selection state independent throughout the plan and code sketches.

### R-003 `The detailed DMA loop still allows spurious completion interrupts`

- Severity: MEDIUM
- Section: `Detailed Code / Validation`
- Type: `Correctness`
- Problem:
  The proposed `process_dma()` ends with:
  `if self.last_avail_idx != avail_idx || self.interrupt_status & 1 == 0 { self.interrupt_status |= 1; }`
  After the loop, `last_avail_idx == avail_idx` in the normal “all work consumed” case, so this condition reduces to “raise an interrupt whenever none is currently pending”, even if no buffers were completed by this notify.
- Why it matters:
  That is not the intended used-buffer notification behavior and can produce spurious guest interrupts on redundant `QueueNotify` writes.
- Recommendation:
  Track whether at least one descriptor chain was completed in this call and only then set `interrupt_status |= 1`. Add a validation item for redundant notify with no new available entries.

## Trade-off Advice

None.

## Positive Notes

- The previous round’s two blocking issues were addressed directly and correctly at the design level.
- The plan is now close to implementation-ready; the remaining gaps are localized to the newly detailed runtime and MMIO control-path logic rather than broad scope or architecture mistakes.

## Approval Conditions

### Must Fix
- R-001
- R-002

### Should Improve
- R-003

### Trade-off Responses Required

None.

### Ready for Implementation
- No
- Reason: The detailed runtime boot-address path and the MMIO feature-selector model are still not correct enough to implement directly.
