# `OS Boot` REVIEW `00`

> Status: Open
> Feature: `boot`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
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
- Blocking Issues: `3`
- Non-Blocking Issues: `1`

## Summary

The overall direction is correct: `OpenSBI -> xv6 -> Linux` is the right boot ladder for this emulator, the `resource/` boundary is the right place to isolate external boot artifacts, and a boot ROM trampoline is a defensible way to model the reset path. The proposed memory layout also broadly fits the current machine shape: DRAM starts at `0x8000_0000`, spans 128 MiB, and `0x8020_0000` is a valid 2 MiB-aligned kernel entry point.

This round is still not ready for implementation. The main blockers are:

- the plan promises a standard xv6 shell while explicitly excluding the storage device that xv6 normally needs to reach that shell;
- the DT / firmware contract is not concrete enough for OpenSBI generic-platform bring-up or for later Linux compliance;
- the boot-mode API does not match the current `xcore` / `xdb` loader and reset architecture, which is still built around one fixed reset vector and one image path.

Those are design-stage issues, not implementation nits. If they are not resolved in the next PLAN, the executor will either approve an unachievable milestone, or ship a boot path that only works through ad hoc loader behavior and undocumented DT assumptions.

---

## Findings

### R-001 `xv6 shell scope conflicts with the plan's no-disk constraint`

- Severity: HIGH
- Section: `Goals / Constraints / Implementation Plan`
- Type: Spec Alignment
- Problem:
  `G-2` promises "xv6 shell prompt", but `NG-2` explicitly excludes disk and the implementation plan for Phase 7b only says "build xv6-riscv kernel as `kernel.bin`". Upstream `xv6-riscv`'s standard QEMU flow links `virtio_disk.o`, builds `fs.img`, and runs with a `virtio-blk-device`; that is how xv6 gets `/init` and `/sh`. As written, the plan mixes "standard xv6 shell" with a machine profile that does not yet provide xv6's normal storage path.
- Why it matters:
  This makes the approved goal set internally inconsistent. The executor could finish the OpenSBI plumbing and still be blocked on an unplanned storage requirement the first time Phase 7b starts. That is exactly the kind of architectural gap the PLAN round is supposed to surface before implementation.
- Recommendation:
  The next PLAN must choose one explicit scope:
  - either narrow `G-2` to "xv6 kernel reaches early boot / console output" for now;
  - or explicitly add the storage strategy required to reach an xv6 shell, including the device model and the external xv6 artifacts kept under `resource/`.

### R-002 `The DT / firmware contract is too underspecified for OpenSBI generic and later Linux boot`

- Severity: HIGH
- Section: `Invariants / Architecture / Validation`
- Type: Invariant
- Problem:
  The plan currently treats the DTB as a mostly static hardware summary (`ACLINT`, `PLIC`, `UART`, memory, ISA string, timebase) and states that the existing block is already compatible as `riscv,clint0`. That is not enough for an implementation-ready round. OpenSBI's generic platform requires the FDT passed by the previous stage to be in sync with its FDT-based drivers, to include the right timer / IPI description, and to provide `/chosen/stdout-path` when console selection matters. Linux boot also requires the firmware handoff to preserve the boot register contract, keep `satp = 0`, and correctly reserve resident firmware memory in the hardware description.
- Why it matters:
  Without a concrete DTS contract, the executor cannot know which compatible strings, node layout, and reserved regions are actually part of acceptance. This is especially risky here because the local machine is not just "QEMU virt copied verbatim": the emulator has its own MMIO wiring, its own ACLINT shape, and only one hart.
- Recommendation:
  The next PLAN must define the exact DT schema to be generated and validated, including:
  - the concrete timer / IPI / PLIC / UART compatible strings and required properties;
  - `/chosen` contents, at minimum `stdout-path`, and the Linux-facing bootargs / initrd story if Linux remains in feature scope;
  - the memory reservation story for resident OpenSBI firmware and any protected regions;
  - validation that kernel and DT placement do not overlap the real built image sizes, not just the nominal addresses.

### R-003 `Boot-mode integration does not fit the current loader/reset architecture`

- Severity: HIGH
- Section: `API Surface / Execution Flow / Constraints`
- Type: API
- Problem:
  The plan introduces `FW` / `KERNEL` / `FDT` boot mode and says "`RESET_VECTOR` changes from `0x8000_0000` to `0x1000` when booting firmware", but the current codebase is not structured that way. `xcore` has a fixed `RESET_VECTOR`, `CPU::reset()` always reloads the default image at that address, `CPU::load()` only loads a single file at that address, and `xdb` currently drives loading through one `X_FILE` path. The plan's `setup_boot(bus, ...)` sketch does not specify how those existing reset/load semantics change or how legacy mode remains stable.
- Why it matters:
  This is the main implementation boundary for the round. If the next PLAN does not define a real runtime boot configuration surface, the executor will either:
  - patch global behavior in a way that breaks legacy tests and debugger flows;
  - or build a boot path that only works in one ad hoc frontend entry point and is lost on reset.
- Recommendation:
  The next PLAN must define an explicit boot configuration contract, for example a `BootConfig` stored by the frontend or CPU, and then map the required code changes across:
  - reset PC selection;
  - firmware/kernel/DT loading;
  - legacy direct-load mode;
  - and any debugger / difftest assumptions that currently key off the fixed reset vector.

### R-004 `This round mixes long-term feature goals with a much smaller acceptance set`

- Severity: MEDIUM
- Section: `Goals / Implementation Plan / Acceptance Mapping`
- Type: Validation
- Problem:
  The top-level goals include OpenSBI console, xv6 shell, Linux shell, and reproducible resource artifacts, but the actual round-00 implementation plan is only Phase 7a and the acceptance mapping only covers `G-1`, `G-5`, and a few constraints. `G-2`, `G-3`, and `G-4` are effectively deferred without being removed from the round's approval target.
- Why it matters:
  That makes round approval ambiguous. A reviewer cannot tell whether accepting `00_PLAN` means "approve only OpenSBI smoke bring-up" or "approve the full feature contract". This repo's iteration rules are explicit that implementation follows an approved PLAN; that approval target must be precise.
- Recommendation:
  The next PLAN should choose one of these:
  - make round `00` explicitly an OpenSBI-only bring-up round and move xv6/Linux shell goals into later numbered PLAN files;
  - or keep the broader feature goals but split the acceptance mapping into "this round" vs "future rounds" with clear non-acceptance status for deferred goals.

---

## Trade-off Advice

### TR-1 `Prefer an explicit external-artifact contract over opaque prebuilt blobs`

- Related Plan Item: `G-4 / C-6 / Phase 7a`
- Topic: Reproducibility vs Convenience
- Reviewer Position: Prefer explicit source-pinned contract
- Advice:
  Keep external boot material under `resource/`, but do not make "pre-built binaries" the only stable artifact. The cleaner boundary is: pinned upstream source provenance plus local fetch/build scripts plus generated outputs.
- Rationale:
  That keeps the core codebase clean, matches the user's request to isolate external material under `resource/`, and gives later reviewers a deterministic way to rebuild OpenSBI, xv6, and Linux without guessing the provenance of opaque blobs.
- Required Action:
  The next PLAN should define the `resource/` layout explicitly: what is checked in, what is generated, how upstream revisions are pinned, and where any local patches live.

### TR-2 `Boot ROM is acceptable, but only if boot selection becomes an explicit runtime contract`

- Related Plan Item: `T-1`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Keep as is with clarification
- Advice:
  The reviewer does not object to the boot-ROM choice itself. The risk is not "Boot ROM vs direct register setup"; the risk is leaving boot selection implicit while the current code is still fixed to one reset vector and one image path.
- Rationale:
  A boot ROM at `0x1000` can be a clean design, but only after the plan defines how firmware mode is selected, how reset behaves, and how legacy direct-load execution remains unchanged.
- Required Action:
  Executor may keep `T-1` as-is, but the next PLAN must add a concrete boot-configuration and reset/load design around it.

---

## Positive Notes

- The high-level boot ladder is correct: using OpenSBI as the M-mode layer is the right way to avoid baking an SBI implementation directly into the kernel bring-up path.
- Choosing a kernel entry at `0x8020_0000` is aligned with Linux's documented 2 MiB physical alignment requirement on RV64.
- Isolating external firmware / kernel material under `resource/` is the right cleanliness boundary for this repo, provided the next round makes the provenance and rebuild contract explicit.

---

## Approval Conditions

### Must Fix
- R-001
- R-002
- R-003

### Should Improve
- R-004

### Trade-off Responses Required
- TR-1
- TR-2

### Ready for Implementation
- No
- Reason: No. The current round still approves an unachievable xv6 milestone, leaves the DT / firmware contract too vague for OpenSBI generic bring-up, and does not yet define how boot mode integrates with the existing fixed-vector loader/reset architecture.
