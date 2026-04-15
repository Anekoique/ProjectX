# `Debian Boot` REVIEW `01`

> Status: Open
> Feature: `debian`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
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
- Non-Blocking Issues: 2

## Summary

Round 01 fixes the main scope problems from round 00. Narrowing Debian to offline userspace, dropping the in-tree image builder, and replacing raw RAM aliasing with a bus-mediated DMA design are all good corrections. But the plan still stops short of a coherent runtime contract. The proposed `MachineConfig` does not yet actually control the real `XCPU` construction path, and the claimed “single source of truth” is still split between runtime env parsing and static per-target DTS files. Those are still blocking because they affect whether the Debian target can boot correctly in the real launcher path rather than only in a hypothetical refactor.

## Findings

### R-001 `MachineConfig still does not reach the real runtime construction path`

- Severity: HIGH
- Section: `Architecture / Execution Flow / Phase 3`
- Type: `Flow`
- Problem:
  The plan says `MachineConfig` is constructed from `X_DISK` / `X_MSIZE` and passed to `RVCore::with_config(config)`, but it never defines how that reaches the actual top-level runtime object. In the current code, [`XCPU`](/Users/anekoique/ProjectX/xemu/xcore/src/cpu/mod.rs#L51) is a global `LazyLock` built from `CPU::new(Core::new())`, and [`init_xcore()`](/Users/anekoique/ProjectX/xemu/xcore/src/lib.rs#L43) touches that singleton before boot. Round 01 leaves `RVCore::new()` in place “for backward compatibility” without stating what replaces the existing singleton construction path.
- Why it matters:
  As written, the real launcher path can still come up as the old 128MB/no-disk machine, which means `G-3` and `G-4` are not actually wired into the runtime that `xdb` uses.
- Recommendation:
  The next PLAN should define one concrete top-level construction contract. Either:
  1. initialize the global CPU from `MachineConfig` before first access, or
  2. make machine config part of the boot/reset path and rebuild the core from it.
  Do not leave `with_config()` as an isolated side constructor with no specified integration point.

### R-002 `The “single source of truth” claim is still contradicted by static Debian DTS`

- Severity: HIGH
- Section: `Invariants / Phase 3 / Phase 4`
- Type: `Invariant`
- Problem:
  `I-6` and the Response Matrix claim that RAM size, devices, DT, and boot addresses are all derived from `MachineConfig`, but the plan simultaneously:
  - parses `X_MSIZE` at runtime and computes FDT placement from it, and
  - introduces a static [`xemu-debian.dts`](/Users/anekoique/ProjectX/resource/xemu.dts) variant hardcoded for 256MB.
  That means the guest-visible memory map and the emulator’s internal RAM size can diverge again if `X_MSIZE` changes, or if the Debian target later needs another RAM size.
- Why it matters:
  This reintroduces the exact coherence problem round 00 called out, only in a narrower form. A static DTB plus runtime-configurable RAM is not one source of truth.
- Recommendation:
  The next PLAN should choose one of these designs explicitly:
  1. Debian target is fixed at 256MB in this round, with no runtime RAM override for that target, or
  2. DT content is generated or patched from `MachineConfig` so the guest-visible memory/device description always matches runtime configuration.

### R-003 `MachineConfig’s proposed shape is internally inconsistent`

- Severity: MEDIUM
- Section: `Data Structure / Phase 3`
- Type: `API`
- Problem:
  The plan defines `MachineConfig` with a stored `fdt_load_addr` field, but Phase 3 also says `MachineConfig::fdt_addr()` computes that address from `ram_size`. At the same time, the summary says the struct drives DT selection, but the struct does not actually carry any DT artifact/variant choice.
- Why it matters:
  This leaves authority unclear inside the very abstraction that is supposed to remove drift. Duplicated derived state is an easy way to recreate the same mismatch between boot code, DT content, and launcher behavior.
- Recommendation:
  Normalize the config shape in the next round. Keep only the minimal independent inputs in `MachineConfig`, and compute derived values like FDT load address from them. If DT selection is part of the contract, model that explicitly.

### R-004 `Snapshot disk semantics still omit reset and reboot behavior`

- Severity: MEDIUM
- Section: `Invariants / Phase 2 / Validation`
- Type: `Validation`
- Problem:
  The plan now states that disk writes modify only the in-memory snapshot, which is good, but it still does not define what happens on emulator reset / `xdb reset` / guest reboot. The current CPU reset path reuses the same device graph and calls `Device::reset()`, and Phase 2 only says reset clears “mutable state” without saying whether the disk bytes revert to the original image or remain modified within the process.
- Why it matters:
  This affects both debugging workflow and the correctness contract of a “snapshot” disk. `V-IT-2` only checks same-session readback, so the most user-visible semantic boundary is still undefined.
- Recommendation:
  State the intended disk behavior across reset and reboot explicitly, then add a validation item for it. Either “reset restores the original snapshot” or “reset preserves in-process snapshot state until process exit” is defensible, but the plan needs to choose.

## Trade-off Advice

### TR-1 `Prefer a fixed Debian machine profile unless DT is generated from config`

- Related Plan Item: `G-4 / C-7 / Phase 4`
- Topic: `Compatibility vs Clean Design`
- Reviewer Position: `Prefer a fixed target profile for this round`
- Advice:
  If round 01 is trying to land both virtio-blk bring-up and a generalized machine-config layer, the safer near-term trade-off is to keep the Debian target fixed at one known-good machine profile, unless the round also includes DT generation from the same runtime config.
- Rationale:
  A fixed 256MB Debian target plus a separate unchanged Linux target is enough to validate block-device bring-up. General runtime configurability is useful, but it should not re-open guest-visible DT drift while the disk path is still being introduced.
- Required Action:
  The next PLAN should either adopt a fixed Debian machine profile for this round or justify and fully specify generated DT content from runtime machine configuration.

## Positive Notes

- The previous round’s main blockers were addressed directly rather than papered over. The scope, persistence contract, and DMA design are all materially better.
- The bus-mediated DMA direction is aligned with the current framework style and is a much cleaner fit than the unsafe RAM-sharing proposal from round 00.

## Approval Conditions

### Must Fix
- R-001
- R-002

### Should Improve
- R-003
- R-004

### Trade-off Responses Required
- TR-1

### Ready for Implementation
- No
- Reason: The runtime construction path and machine/DT source-of-truth contract are still unresolved.
