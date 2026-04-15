# `difftest` REVIEW `02`

> Status: Closed
> Feature: `difftest`
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
- Blocking Issues: `4`
- Non-Blocking Issues: `2`

## Summary

`02_PLAN` is more disciplined than round01 and it fixes the feature-boundary problem, but it still fails on the most important correctness boundary: it now bakes in a QEMU stepping mode that suppresses interrupts and timers, then treats the resulting DUT/REF divergence as something to sync away instead of something difftest should verify. At the same time, the new `DifftestOps` trait leaks QEMU-specific protocol details into `xcore`, and the interactive attach flow still reads the binary path through `option_env!("X_FILE")`, which is compile-time state rather than monitor runtime state. Those issues keep the plan from being implementation-ready.

---

## Findings

### R-001 `the chosen QEMU sstep mask makes interrupt correctness untestable`

- Severity: HIGH
- Section: `Summary / Invariants / Constraints / Main Flow / Interrupt Delivery Divergence Detection / Trade-offs`
- Type: Correctness
- Problem:
  The plan explicitly sends `Qqemu.sstep=0x7` and documents that this means `NOIRQ + NOTIMER`, then treats the resulting "DUT took interrupt, REF did not" case as an expected divergence that should be synchronized away.
- Why it matters:
  This turns interrupt and timer delivery into a blind spot. Phase 6 is supposed to protect the OpenSBI/xv6/Linux path, and interrupt/trap delivery is exactly the class of bug difftest must catch, not suppress. Under this plan, a real DUT bug in interrupt entry timing or masking can be silently hidden behind the same sync path as the expected QEMU behavior.
- Recommendation:
  Do not build round02 around `sstep=0x7` plus sync-away logic. Either use a stepping mode that preserves interrupt/timer delivery semantics for comparison, or fail closed and keep the blocker open until a correct backend mechanism is defined.

### R-002 `unsupported sstep falls back to an unverified mode`

- Severity: HIGH
- Section: `Failure Flow / Phase 3 / Validation`
- Type: Correctness
- Problem:
  The failure flow says `Qqemu.sstep` unsupported -> warn and continue with default behavior.
- Why it matters:
  The plan relies on `Qqemu.sstep` semantics to justify how difftest behaves on interrupt-heavy workloads. If the backend cannot confirm that stepping mode, falling back to some unknown default makes the comparison semantics undefined. A warning is not enough; the whole correctness model becomes unreviewable.
- Recommendation:
  Treat unsupported `Qqemu.sstep` as an attach failure. The next plan should not permit a fail-open path for the core backend stepping contract.

### R-003 `QEMU backend details are leaking into xcore's arch abstraction`

- Severity: HIGH
- Section: `Feature Introduce / Data Structure / API Surface / Phase 1`
- Type: API
- Problem:
  `DiffCsrEntry` stores `qemu_regnum`, `DifftestOps` exposes `qemu_priv_regnum()` and `qemu_bin()`, and the plan describes `DifftestOps` as the arch-level abstraction in `xcore`.
- Why it matters:
  This is not a backend-neutral arch abstraction; it is a QEMU-specific control surface moved into the core crate. That undermines the stated backend split (`DiffBackend` in xdb) and makes future backends pay for QEMU protocol knowledge in `xcore`.
- Recommendation:
  Keep `xcore` backend-neutral: snapshot shape, CSR whitelist, and maybe CSR addresses or names. Move QEMU register-number mapping and binary selection into the xdb QEMU driver or into xdb-side arch descriptors.

### R-004 `interactive attach still depends on compile-time `X_FILE``

- Severity: HIGH
- Section: `Constraints / Main Flow / Phase 5`
- Type: Flow
- Problem:
  The plan says `dt attach` requires a loaded binary, but the concrete `cmd_dt_attach()` snippet still reads the path from `option_env!("X_FILE")`.
- Why it matters:
  `option_env!()` is compile-time state, not runtime monitor state. That means the intended interactive flow:
  `load <file>` -> `dt attach`
  is still not actually defined by the plan. A binary loaded from the monitor would not automatically update `option_env!("X_FILE")`, so the attach path can fail or use stale configuration.
- Recommendation:
  Store the currently loaded binary path in xdb runtime state and have `dt attach` use that state. Do not use `option_env!()` for monitor-controlled difftest lifecycle.

### R-005 `the response to the RAM-write sync review is still too broad`

- Severity: MEDIUM
- Section: `Review Adjustments / Response Matrix`
- Type: Maintainability
- Problem:
  `R-006` from the previous review is rejected with a blanket claim that MMIO writes do not need RAM-write synchronization because they "go to devices not RAM".
- Why it matters:
  The review concern was about the plan's response discipline as much as the exact mechanism. The new rejection still does not explain how future extensions such as alternative backends, observer widening, or more complex side effects would be handled if that assumption stops holding.
- Recommendation:
  Either tighten the claim to "for the current RISC-V functional core, a single instruction does not need combined RAM+MMIO synchronization" or leave the item open as a documented limitation.

### R-006 `M-003 is not responded to literally`

- Severity: LOW
- Section: `Master Compliance / Response Matrix`
- Type: Spec Alignment
- Problem:
  `01_MASTER.md` says "difftest depend on test feature", but `02_PLAN` answers it as `difftest = ["debug"]`.
- Why it matters:
  The intent may be correct, but the response is not literally aligned with the master wording. If the directive text was mistaken, the plan should say so explicitly instead of silently rewriting it.
- Recommendation:
  Clarify whether `M-003` meant `debug` or some other feature, then answer it explicitly in the next plan.

---

## Trade-off Advice

### TR-1 `interrupt correctness vs backend workaround`

- Related Plan Item: `R-003 response / T-4`
- Topic: Correctness vs Convenience
- Reviewer Position: Prefer correctness
- Advice:
  Do not normalize an interrupt/timer suppression workaround into the base difftest semantics.
- Rationale:
  A framework that syncs away exactly the control-flow class it is supposed to validate will be fast to implement but weak where it matters most.
- Required Action:
  Executor should redesign the QEMU stepping strategy so interrupt delivery stays comparable, or keep the blocker open and narrow the validated scope honestly.

### TR-2 `arch abstraction vs backend abstraction`

- Related Plan Item: `M-001 / Phase 1`
- Topic: Clean Design vs Expedience
- Reviewer Position: Prefer backend-neutral xcore
- Advice:
  Let `xcore` describe architecture state, not QEMU protocol details.
- Rationale:
  Once `qemu_regnum` and `qemu_bin()` are part of the xcore-facing trait, the backend split is mostly nominal. That is the wrong long-term direction if Spike or other references are ever added.
- Required Action:
  Executor should move QEMU-specific mapping out of `DifftestOps` and into xdb-side backend/arch tables.

### TR-3 `runtime monitor state vs build-time environment`

- Related Plan Item: `R-005 response / Phase 5`
- Topic: Simplicity vs Reliability
- Reviewer Position: Prefer runtime state
- Advice:
  `dt attach` should consume the monitor's current loaded-image state, not a compile-time env macro.
- Rationale:
  The whole point of moving difftest into `xdb` is to make it monitor-owned. Using `option_env!()` undermines that goal and keeps the workflow brittle.
- Required Action:
  Executor should add explicit xdb runtime state for the currently loaded binary path and use that in the attach/reset flow.

---

## Positive Notes

- The plan is much more truthful than round01 about QEMU-first delivery and it no longer claims that a Spike stub is implemented support.
- Feature gating is now handled much more cleanly than before, and the `difftest -> debug` dependency resolves one of the previous major inconsistencies.
- Introducing an explicit arch-facing difftest trait is directionally right; the issue is the QEMU-specific content currently stored inside it.

---

## Approval Conditions

### Must Fix
- R-001
- R-002
- R-003
- R-004

### Should Improve
- R-005
- R-006

### Trade-off Responses Required
- TR-1
- TR-2
- TR-3

### Ready for Implementation
- No
- Reason: Round02 still makes interrupt delivery effectively untestable under its chosen QEMU stepping mode, keeps a fail-open backend path, leaks QEMU protocol details into `xcore`, and does not yet define `dt attach` from real runtime monitor state.
