# `difftest` REVIEW `03`

> Status: Closed
> Feature: `difftest`
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
- Blocking Issues: `4`
- Non-Blocking Issues: `2`

## Summary

`03_PLAN` finally fixes the round02 stepping mistake and the backend-neutrality problem in `xcore`, which is real progress. But it still is not implementation-ready. The biggest remaining gap is that the QEMU backend never enables physical-memory mode even though the plan treats memory addresses as physical and targets OS-boot scenarios with address translation. On top of that, the new Spike build path is not buildable as written, the Spike backend is still based on a non-public upstream API without a compatibility strategy, and the "runtime path" fix still seeds state from `option_env!("X_FILE")`. The design is closer, but those issues are still too fundamental to approve.

---

## Findings

### R-001 `QEMU memory access is still wrong once address translation turns on`

- Severity: HIGH
- Section: `Goals / Main Flow / Phase 3 / DiffBackend API / Constraints`
- Type: Correctness
- Problem:
  The plan uses `gdb.write_mem(addr, data)` and generally describes memory addresses as physical, but the QEMU backend never switches the gdbstub into physical-memory mode.
- Why it matters:
  QEMU's documented gdbstub behavior is to access the current process memory by default, not guest physical memory. The same docs explicitly provide `qqemu.PhyMemMode` / `Qqemu.PhyMemMode:1` to switch to physical-memory access. Without that step, any memory sync or inspection done after `satp` or other translation changes can hit the wrong address space. That is a direct blocker for the OpenSBI/xv6/Linux path this feature is supposed to enable.
- Recommendation:
  Add explicit physical-memory mode negotiation to the QEMU attach flow and treat failure to enable it as an attach failure. If the plan wants to stay virtual-only, it must narrow scope away from translated-boot scenarios and stop describing these addresses as physical.

### R-002 `the Spike build plan is not buildable as written`

- Severity: HIGH
- Section: `Phase 4 / Phase 7`
- Type: Correctness
- Problem:
  The Cargo/build snippets are internally inconsistent:
  `xdb/Cargo.toml` is shown with two `[features]` tables, `cc` is declared as an optional build-dependency and then referenced like a normal feature item, and `xdb/build.rs` is gated with `#[cfg(feature = "difftest")]` even though build scripts do not consume package features that way.
- Why it matters:
  This is not a stylistic issue; the build plan will not work as written. If the backend cannot be built, the round cannot honestly claim a two-backend implementation.
- Recommendation:
  Rewrite the build integration so it is valid Cargo:
  one `[features]` table, `cc` as a normal build-dependency, and build-script gating via Cargo-provided environment such as `CARGO_FEATURE_DIFFTEST` rather than `#[cfg(feature = ...)]`.

### R-003 `Spike is promoted to a first-class backend on top of a non-public upstream API`

- Severity: HIGH
- Section: `Summary / Goals / Phase 4 / Validation`
- Type: Maintainability
- Problem:
  The plan elevates Spike to a real supported backend by wrapping Spike's C++ internals directly, but it does not record any compatibility strategy beyond "Spike source tree required".
- Why it matters:
  Spike's own upstream README states that the C++ interface to its internals is not considered a public API and may change incompatibly without a major-version bump. That makes a direct-wrapper backend inherently fragile. Without explicit version pinning, support matrix, or non-gating scope, the plan is overstating how reviewable and maintainable this backend really is.
- Recommendation:
  Either narrow Spike to an experimental/non-gating backend with a pinned supported version, or add a concrete compatibility policy to the plan. Do not present it as equivalent to the QEMU path without that containment.

### R-004 `the startup path still depends on compile-time `X_FILE``

- Severity: HIGH
- Section: `Summary / Review Adjustments / Phase 6`
- Type: Flow
- Problem:
  The round03 summary says runtime binary tracking no longer uses `option_env!`, but the actual `main.rs` sketch still seeds `loaded_binary_path` from `option_env!("X_FILE")`.
- Why it matters:
  That keeps compile-time environment coupling in the startup path. It is better than round02 because later `load` updates runtime state, but it still means the default attach path can depend on stale compile-time values and keeps `make run FILE=...` brittle.
- Recommendation:
  Remove `option_env!("X_FILE")` entirely from difftest state management. Use runtime env (`std::env::var`) or command-line/runtime state only.

### R-005 `the Spike ISA configuration is still too hard-coded`

- Severity: MEDIUM
- Section: `Phase 4`
- Type: Spec Alignment
- Problem:
  The Spike backend sketch hard-codes ISA strings like `rv64imac` / `rv32imac`.
- Why it matters:
  That makes the backend definition narrower than the rest of the plan implies. If the DUT configuration or future boot workloads depend on a different extension set, the REF may no longer describe the same machine.
- Recommendation:
  Derive the ISA string from xcore/xconfig state or document that round03 only supports a specific pinned ISA subset.

### R-006 `the response to R-005 is still only partially resolved`

- Severity: LOW
- Section: `Review Adjustments / Response Matrix / Constraints`
- Type: Maintainability
- Problem:
  The plan narrows the RAM+MMIO claim, which is better, but it still leaves the limitation sitting in prose without mapping it to a concrete future extension point or non-goal boundary.
- Why it matters:
  This is no longer a blocker, but the limitation is important enough that future iterations should not have to rediscover it from narrative text.
- Recommendation:
  Carry the limitation into a stable constraint/non-goal entry or a clearly named future work item.

---

## Trade-off Advice

### TR-1 `physical memory correctness vs simpler QEMU setup`

- Related Plan Item: `Phase 3 / DiffBackend::write_mem`
- Topic: Correctness vs Simplicity
- Reviewer Position: Prefer correctness
- Advice:
  If difftest operates on guest physical addresses, the QEMU backend must explicitly switch the gdbstub into physical-memory mode.
- Rationale:
  The simple setup is only safe while guest virtual and physical addresses coincide. That is not the long-term target of this feature.
- Required Action:
  Executor should add `Qqemu.PhyMemMode:1` (and verification) to the attach sequence or narrow the supported scope.

### TR-2 `full Spike delivery vs contained experimental backend`

- Related Plan Item: `G-2 / Phase 4`
- Topic: Scope vs Maintainability
- Reviewer Position: Prefer contained scope
- Advice:
  Given the upstream API instability, it is better to ship Spike as an explicitly experimental or pinned-version backend than to present it as equal to QEMU in the same round.
- Rationale:
  The problem is not that a wrapper is impossible; it is that the plan currently gives no compatibility contract for maintaining it.
- Required Action:
  Executor should either add version/support constraints or reduce the Spike claim accordingly.

### TR-3 `runtime state vs startup convenience`

- Related Plan Item: `Review Adjustments / Phase 6`
- Topic: Reliability vs Convenience
- Reviewer Position: Prefer runtime state only
- Advice:
  Do not keep `option_env!("X_FILE")` as a bootstrap shortcut.
- Rationale:
  It preserves exactly the class of stale build-time coupling that earlier review rounds were trying to remove.
- Required Action:
  Executor should move the startup default to true runtime configuration or require explicit `load` before attach.

---

## Positive Notes

- The round03 plan correctly fixes the round02 `sstep=0x7` problem by moving to an interrupt-preserving stepping mode and failing closed if QEMU does not support it.
- Moving QEMU register-number mapping out of `xcore` is the right architectural cleanup and materially improves the backend boundary.
- The plan is much more concrete now about file placement, trait boundaries, and attach flow, which makes the remaining issues easier to isolate.

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
- Reason: Round03 still leaves the QEMU backend in the wrong memory-access mode for translated workloads, the Spike build path is not buildable as written, the Spike integration lacks an upstream-compatibility contract, and the startup path still retains compile-time `X_FILE` coupling.
