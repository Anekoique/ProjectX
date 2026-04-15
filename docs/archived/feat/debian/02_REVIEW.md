# `Debian Boot` REVIEW `02`

> Status: Open
> Feature: `debian`
> Iteration: `02`
> Owner: Reviewer
> Target Plan: `02_PLAN.md`
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

Round 02 is materially stronger than round 01. The fixed-profile decision is cleaner, the runtime construction path is now addressed directly, and the machine/DT coherence story is much more believable. But two blocking issues remain. One is a concrete integration gap in the launcher path: the new profile selection depends on `X_DISK`, while the current build/run plumbing still only exports `X_FW`/`X_KERNEL`/`X_INITRD`/`X_FDT`. The other is a spec and semantics problem: the plan makes guest-visible virtio device reset restore the original disk snapshot, which conflates transport reset with emulator-level media rollback.

## Findings

### R-001 `The launcher path still does not actually deliver disk config to xdb`

- Severity: HIGH
- Section: `Execution Flow / Phase 1 / Phase 5`
- Type: `Flow`
- Problem:
  The new startup flow depends on `machine_config()` reading `X_DISK`, and `run-debian` is described as invoking xemu with `DISK=$(DEBIAN_IMG)`. But the current launcher make layer in [`xemu/Makefile`](/Users/anekoique/ProjectX/xemu/Makefile#L1) only exports `X_FILE`, `X_FW`, `X_KERNEL`, `X_INITRD`, and `X_FDT`; there is no `DISK` variable or `X_DISK` export. Round 02 does not list any corresponding `xemu/Makefile` change or an alternative direct-env launch path.
- Why it matters:
  Without this handoff, `xdb::main()` will never see the disk image path, `MachineConfig::with_disk(...)` will never be selected, and the Debian profile will not be activated in the actual runtime path.
- Recommendation:
  The next PLAN should explicitly wire the disk path through the real launcher path. Either update [`xemu/Makefile`](/Users/anekoique/ProjectX/xemu/Makefile) to accept `DISK` and export `X_DISK`, or state that `resource/debian.mk` bypasses that wrapper and invokes xemu with `X_DISK=...` directly.

### R-002 `The reset model is not compliant with virtio block-device semantics`

- Severity: HIGH
- Section: `Invariants / State Transition / Phase 3`
- Type: `Spec Alignment`
- Problem:
  The plan says writing `0` to the virtio `Status` register performs a “full reset” that restores the original disk snapshot, and it equates that with `Device::reset()`. Under the virtio spec, writing `0` resets device state and queue/interrupt state; it does not imply rollback of the backing block medium. Round 02 is therefore conflating a guest-visible virtio device reset with an emulator/session-level snapshot restore.
- Why it matters:
  A guest driver reset must not silently discard already-written sectors from the virtual disk. That would violate guest-visible device semantics and could produce incorrect Linux behavior if the device is reset and reinitialized.
- Recommendation:
  Split these two concepts in the next PLAN. Virtio `Status=0` / device reset should clear transport state only. If the emulator wants “reset to original snapshot” for debugger or whole-machine reset, define that as a separate emulator-level reset path and validate it separately.

### R-003 `The new init API still needs an explicit repeated-call contract`

- Severity: MEDIUM
- Section: `Phase 1 / API Surface`
- Type: `API`
- Problem:
  `init_xcore(config)` moves from a reset-style helper to `OnceLock`-backed one-time initialization, but the plan does not define what happens if it is called more than once in the same process. That is a behavior change from the current API surface, where `init_xcore()` resets the existing global CPU.
- Why it matters:
  This affects tests, library embedding, and future tooling. A silent no-op, panic, or error on second init each has different operational consequences.
- Recommendation:
  State the repeated-call contract explicitly in the next PLAN. Either reject re-init with a documented error, or keep reinitialization support via a separate reset/rebuild path.

## Trade-off Advice

None.

## Positive Notes

- The previous round’s two conceptual blockers were addressed in the right direction. Fixed machine profiles are a cleaner fit than half-general runtime configurability here.
- Replacing the old “single source of truth” claim with a narrower fixed-profile contract is an improvement in precision.

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
- Reason: The disk-launch integration path is still incomplete, and the current reset semantics are not virtio-correct.
