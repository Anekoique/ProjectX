# `OS Boot` REVIEW `01`

> Status: Open
> Feature: `boot`
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
- Blocking Issues: `3`
- Non-Blocking Issues: `1`

## Summary

Round `01` is materially better than round `00`. The scope is now precise, the response matrix is complete, and moving the round target to "OpenSBI console bring-up only" is the right correction. The plan also improves the architecture discussion by introducing an explicit `BootConfig` and by treating the DTS as a first-class artifact under `resource/`.

It is still not ready for implementation. Three blocking issues remain:

- the proposed boot ROM trampoline does not correctly materialize the planned RV64 physical addresses;
- the claimed "reset-safe" `BootConfig` model is not actually defined against the current bus and debugger reset architecture;
- the published DTS / build contract is still internally inconsistent and not directly buildable as written.

These are not style problems. They affect whether the first firmware-mode boot can work at all, whether mode switching/reset can remain coherent in one process, and whether the advertised reproducible build path is actually executable.

---

## Findings

### R-001 `Boot ROM address materialization is incorrect for the planned RV64 addresses`

- Severity: HIGH
- Section: `Implementation Plan / Validation / Constraints`
- Type: Correctness
- Problem:
  Step 2 defines the trampoline as `lui`/`addi` for `a1 = FDT_ADDR` and `lui`/`jalr` for the entry address. That is not mechanically correct for the addresses used in this plan. On RV64, the `lui` result is sign-extended, so materializing `0x8000_0000` or `0x87F0_0000` that way yields a negative canonical value rather than the intended low 32-bit physical address. The current unit validation (`V-UT-4`) only says "decoding instructions yields ..." and would not catch this bad final register value.
- Why it matters:
  This is the first instruction path of firmware boot. If the trampoline computes `a1` or the jump target incorrectly, OpenSBI never receives the intended DTB pointer and the emulator jumps outside mapped DRAM. That blocks `G-1` directly.
- Recommendation:
  The next PLAN must replace the pseudocode with an address-construction sequence that is correct on RV64 for these exact addresses, and strengthen validation accordingly. For example:
  - use a zero-extension-safe constant materialization sequence;
  - or use a PC-relative sequence only if the offset range and final values are proven correct.

  Validation must check the resulting register values or execute the trampoline in a focused test, not just decode the intended mnemonics.

### R-002 `BootConfig is not yet reset-safe under the current bus and debugger lifecycle`

- Severity: HIGH
- Section: `Architecture / Invariants / API Surface / Execution Flow`
- Type: Flow
- Problem:
  The plan says `BootConfig` makes boot selection explicit and "reset-safe", and `I-1`/`I-2` claim exact mode-dependent behavior. But the current architecture does not yet support that lifecycle:
  - the bus can add MMIO regions and replace named devices, but it cannot remove a previously added boot ROM region;
  - the debugger already exposes `reset`, and that command currently calls bare `CPU::reset()`, not a boot-mode-aware path;
  - `xcore::init_xcore()` also performs an early reset before `xdb` boot selection happens.

  The plan never states how a process that has booted once in firmware mode returns to exact legacy direct mode, or how debugger reset preserves firmware mode coherently.
- Why it matters:
  This is the architectural contract that round `01` explicitly claimed to fix from `R-003` in round `00`. Without a concrete lifecycle, "exact legacy behavior" and "reset-safe" are not true yet, and the executor will be forced to improvise semantics during implementation.
- Recommendation:
  The next PLAN must define one explicit lifecycle and carry it through all reset paths. For example:
  - persist the active `BootConfig` and make reset reapply it;
  - rebuild or reinitialize the machine/bus when changing mode;
  - or add and use an explicit MMIO removal / boot-ROM teardown mechanism.

  It should also state how `xdb reset` and startup initialization interact with the chosen boot mode.

### R-003 `The DTS and build contract are still not self-consistent as written`

- Severity: HIGH
- Section: `Review Adjustments / Execution Flow / DTS File / Constraints`
- Type: Validation
- Problem:
  The round now treats `resource/xemu.dts` and `resource/Makefile` as the reproducible source of truth, but the published contract is still internally inconsistent:
  - the DTS snippet uses `interrupt-parent = <&plic>` but never defines a `plic:` label for the PLIC node, so the source does not compile as written;
  - the round is explicitly OpenSBI-only, yet the main firmware flow still invokes xemu with `KERNEL=kernel.bin`;
  - the "Removed" section says all `FW`/`KERNEL`/`FDT` env vars are removed from xemu's Makefile, but Step 5 later adds exactly those vars back for xemu-side pass-through.
- Why it matters:
  `G-2` is a reproducible-build goal in this round. If the documented DTS cannot compile and the make-path contract still contradicts itself about kernel inputs and Makefile responsibilities, then the plan has not actually delivered a buildable round-01 artifact model.
- Recommendation:
  The next PLAN must make the artifact contract executable as written:
  - fix the DTS references/labels so the sample source is compilable;
  - decide whether round-01 `make boot` is firmware-only or firmware-plus-optional-kernel, and document the command line consistently;
  - make the xemu/resource Makefile boundary internally consistent instead of saying the vars are both removed and reintroduced.

### R-004 `The documented misa literal still omits the C bit`

- Severity: MEDIUM
- Section: `Constraints / Validation / Log`
- Type: Correctness
- Problem:
  The round says `misa` must report `IMACSU`, but the literal in `C-3` and `V-UT-2` is still `0x8000_0000_0014_1101`, which does not include bit 2 for `C`. That value does not match the stated extension set.
- Why it matters:
  This weakens `G-3` directly and would make the planned implementation and test encode the wrong architectural contract.
- Recommendation:
  Correct the literal and all dependent validation to the real `IMACSU` value, and keep the DT ISA description aligned with the actual emulator capability.

---

## Trade-off Advice

### TR-1 `Prefer rebuilding boot-mode machine state over trying to undo ad hoc MMIO mutation`

- Related Plan Item: `T-1 / BootConfig lifecycle`
- Topic: Clean Design vs Mutation Risk
- Reviewer Position: Prefer explicit rebuild/reapply model
- Advice:
  Prefer a design where the active boot mode is reapplied through a single configuration path, even if that means rebuilding or reinitializing the machine state for mode changes or reset. Avoid a design that relies on "add boot ROM sometimes and somehow remove it later" without a first-class lifecycle.
- Rationale:
  The current bus abstraction is good at fixed machine composition. It is not currently a dynamic device hotplug/remove layer. A rebuild/reapply model is cleaner, easier to reason about, and less likely to leak firmware-mode state into direct mode.
- Required Action:
  The next PLAN should choose one explicit lifecycle and document it through startup, `load`, and `reset`.

---

## Positive Notes

- Narrowing the round to OpenSBI console bring-up is the right correction and fully addresses the biggest scope problem from round `00`.
- The response matrix is complete and does address all prior blocking review findings plus the master directives.
- Moving the external artifact discussion under `resource/` is the right cleanliness boundary for this project.

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

### Ready for Implementation
- No
- Reason: No. The round is much closer, but the first-instruction boot path, mode/reset lifecycle, and reproducible DTS/build contract are still not yet mechanically correct.
