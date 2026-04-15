# `Debian Boot` REVIEW `00`

> Status: Open
> Feature: `debian`
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
- Blocking Issues: 4
- Non-Blocking Issues: 1

## Summary

The direction is reasonable: adding a block device plus a richer Linux userspace is the right next step after initramfs boot. But this round is not implementation-ready. The plan currently over-promises Debian/networking outcomes without a network device, proposes a guest-RAM access model that conflicts with both the current bus ownership model and its own invariant, and treats the machine description as if it can be changed globally for Debian without regressing the existing initramfs Linux path. The next round should tighten the machine contract first: one coherent source of truth for RAM size, DT contents, bootargs, and disk presence.

## Findings

### R-001 `Debian userspace goal exceeds the no-network scope`

- Severity: HIGH
- Section: `Goals / Validation`
- Type: `Correctness`
- Problem:
  `G-2` promises a Debian shell with working `apt`, `dpkg`, and networking tools, while `NG-1` explicitly excludes `virtio-net` and the plan does not define any alternate network path. In the current machine there is only UART, ACLINT, PLIC, and the proposed block device.
- Why it matters:
  This makes the acceptance target internally inconsistent. `dpkg` can be exercised offline, but `apt` against package repositories and general networking-tool validation cannot be satisfied without a network device or some other explicit transport.
- Recommendation:
  Narrow `G-2` for this iteration to Debian boot/login/local userspace execution plus filesystem-backed package database behavior, or add networking as explicit scope. The validation matrix must match the chosen scope.

### R-002 `The RAM access design contradicts the framework and the plan’s own invariant`

- Severity: HIGH
- Section: `Invariants / API Surface / Phase 2 / Trade-offs`
- Type: `Invariant`
- Problem:
  `I-1` says the device must access guest RAM only through the Bus’s existing methods and must avoid direct host-memory aliasing, but the chosen design in the RAM-sharing section and `T-3` is the opposite: `Arc<UnsafeCell<Vec<u8>>>` or a raw pointer to the RAM backing store. That also conflicts with the current architecture where [`bus.rs`](/Users/anekoique/ProjectX/xemu/xcore/src/device/bus.rs) owns [`Ram`](/Users/anekoique/ProjectX/xemu/xcore/src/device/ram.rs) directly and `Device::write()` has no bus handle.
- Why it matters:
  This is not a small local detail. It is a framework-level ownership change in the hot path, it weakens the current lock-free single-owner bus design, and it pushes memory-safety invariants into one MMIO device implementation.
- Recommendation:
  Pick a bus-mediated DMA design and document the concrete API change. For example, process queue notifications through a bus-owned helper that has `&mut Bus`, or introduce an explicit safe guest-memory accessor interface for DMA-capable devices. Remove the contradictory “no direct aliasing” wording if the plan intentionally chooses otherwise.

### R-003 `Runtime RAM sizing and disk boot are under-specified for the current machine model`

- Severity: HIGH
- Section: `Constraints / Phase 2 / Phase 3`
- Type: `Flow`
- Problem:
  The plan says “add `X_DISK`” and “add `X_MSIZE` or increase default”, but the current machine is constructed from a fixed `RVCore::new()` / `XCPU` path, uses compile-time `CONFIG_MSIZE = 128MB`, loads a static DTB whose `memory@80000000` node also says `128MB`, and uses fixed boot load addresses such as `FDT_LOAD_ADDR` near the top of that 128MB window. The round does not define how these pieces stay coherent when Debian needs more memory and an extra block device.
- Why it matters:
  Without a single machine-configuration contract, the emulator can easily end up with “more host RAM allocated but only 128MB described to the guest”, or with target-specific boot artifacts and DT contents drifting apart. That is a correctness problem, not just a cleanup issue.
- Recommendation:
  Add a concrete machine-configuration design to the next plan. One source of truth should drive bus construction, RAM size, optional devices, DT memory/device nodes, and any boot-time load addresses that depend on memory layout.

### R-004 `Hard-switching bootargs in the shared DTB would break the existing Linux path`

- Severity: HIGH
- Section: `Phase 3 / Constraints`
- Type: `Correctness`
- Problem:
  Phase 3 says to update `chosen.bootargs` to `root=/dev/vda rw`, but the current shared [`xemu.dts`](/Users/anekoique/ProjectX/resource/xemu.dts) is also used by the existing [`linux.mk`](/Users/anekoique/ProjectX/resource/linux.mk) initramfs boot flow, which depends on `rdinit=/init`. That directly conflicts with `C-6` (“must not break existing `make linux`”).
- Why it matters:
  A single static DTB cannot encode mutually exclusive boot contracts cleanly here. If the plan follows this step literally, it regresses the currently working Linux path.
- Recommendation:
  Define mode-specific DT handling in the next round: separate DTBs, a generated DT step per target, or a controlled bootargs/FDT patch step at launch. The plan should state exactly which artifact is authoritative for initramfs boot versus disk-root boot.

### R-005 `Disk persistence and acceptance mapping are still ambiguous`

- Severity: MEDIUM
- Section: `Data Structure / Trade-offs / Validation`
- Type: `Validation`
- Problem:
  `I-3` talks about a read-write disk image file, but the actual chosen backend is an in-memory `Vec<u8>` with no stated writeback path. At the same time, `V-IT-2` claims persistence, and the acceptance mapping for `G-3` incorrectly points to `V-IT-3` (“existing make linux still works”) instead of a Debian-target validation.
- Why it matters:
  The plan does not currently say whether this round is a persistent disk image or a throwaway snapshot model. That leaves both the implementation contract and the validation target unclear.
- Recommendation:
  Make the persistence contract explicit. If disk writes must survive reboot, add writeback semantics and a reboot-level validation. If this round is intentionally snapshot-only, say so and change the wording of `I-3`, `V-IT-2`, and the acceptance mapping.

## Trade-off Advice

### TR-1 `Prefer a bus-mediated DMA path over raw RAM aliasing`

- Related Plan Item: `T-3`
- Topic: `Flexibility vs Safety`
- Reviewer Position: `Prefer Option C, but redesigned around an explicit bus-mediated interface rather than raw aliasing`
- Advice:
  Keep the “single-threaded and simple” goal, but do not express it as `Arc<UnsafeCell<Vec<u8>>>` or a raw pointer shared into a device.
- Rationale:
  The current codebase is intentionally clean in this area: the bus owns RAM and devices are isolated behind the `Device` trait. Preserving that boundary is a better long-term trade-off than saving a small amount of plumbing with unsafe aliasing.
- Required Action:
  The next PLAN should either adopt a safe bus-mediated design or justify, in concrete architectural terms, why a framework-level unsafe RAM alias is unavoidable.

### TR-2 `Prefer a known-good Debian image before in-tree image construction`

- Related Plan Item: `T-4`
- Topic: `Compatibility vs Clean Design`
- Reviewer Position: `Prefer pre-built image first`
- Advice:
  Separate “boot a Debian disk image correctly” from “build Debian images locally with host tooling”. The first round should minimize host-environment dependencies and prove the emulator/device contract first.
- Rationale:
  `mmdebstrap` is useful, but it adds host-package, privilege-mode, and foreign-architecture setup questions that are orthogonal to proving `virtio-blk` correctness. A known-good image gives a tighter first signal and a smaller failure surface.
- Required Action:
  The next PLAN should either split the image-builder work into a follow-up round or justify why both emulator bring-up and local Debian image construction must land together.

## Positive Notes

- The overall feature direction is sound: a block-backed Debian userspace is a meaningful next validation step after the current initramfs Linux boot.
- The plan already identifies the right low-level areas to think about for VirtIO bring-up: queue bounds, malformed descriptor chains, interrupt semantics, and transport/backend trade-offs.

## Approval Conditions

### Must Fix
- R-001
- R-002
- R-003
- R-004

### Should Improve
- R-005

### Trade-off Responses Required
- TR-1
- TR-2

### Ready for Implementation
- No
- Reason: The round still has unresolved blocking contradictions in scope, memory ownership, and boot/machine configuration.
