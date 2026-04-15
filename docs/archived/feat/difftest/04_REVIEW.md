# `difftest` REVIEW `04`

> Status: Closed
> Feature: `difftest`
> Iteration: `04`
> Owner: Reviewer
> Target Plan: `04_PLAN.md`
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

`04_PLAN` is much closer than the previous rounds. It fixes the QEMU physical-memory gap, removes the old compile-time `option_env!("X_FILE")` dependency from difftest state, and the `CoreContext` direction is cleaner than the earlier trait-heavy design. But the plan still is not implementation-ready. The biggest correctness problem is that it stores masked CSR values in `CoreContext` and then writes those masked values back into the reference during sync, which will actively clear masked-off interrupt bits like `mip.MTIP/SSIP`. On top of that, the supposedly experimental Spike backend is still built unconditionally whenever `difftest` is enabled, the new `CoreContext` boundary is undermined by a duplicated QEMU CSR table plus index-based comparison, and the claimed Spike version pin is still only a placeholder.

---

## Findings

### R-001 `masked CSR snapshots are being used for state synchronization`

- Severity: HIGH
- Section: `Data Structure / Phase 1 / Phase 3 / Phase 5 / Invariants`
- Type: Correctness
- Problem:
  `CsrValue.value` is defined as the already-masked CSR value, and both QEMU and Spike `sync_state()` write that stored value back into the reference CSR state.
- Why it matters:
  Masking is appropriate for comparison, not for state replication. In the current whitelist, `mip` is masked with `!0x82`, so a DUT snapshot will drop `SSIP` and `MTIP`. Writing that masked value back into REF clears those bits in the reference machine. That directly interferes with the interrupt-preserving model the plan just worked to restore, and it can create or hide interrupt divergences after MMIO/halt sync.
- Recommendation:
  Split raw state from compare state. `CoreContext`/`ArchSnapshot` should either carry raw CSR values plus a separate compare mask, or `sync_state()` must preserve masked-out bits when writing REF state. Do not reuse masked comparison payloads as synchronization payloads.

### R-002 `the experimental Spike backend is still on the mandatory difftest build path`

- Severity: HIGH
- Section: `Summary / Goals / Phase 4 / build.rs / Cargo / Trade-offs`
- Type: Build / Scope
- Problem:
  The plan says QEMU is the production backend and Spike is experimental, but `xdb/build.rs` still builds and links the Spike wrapper whenever `CARGO_FEATURE_DIFFTEST` is set. There is no separate Spike feature.
- Why it matters:
  This means `DIFFTEST=1 make run` for the QEMU path still requires a local Spike source tree, headers, static libraries, and host C++ toolchain setup. That is the opposite of the stated scope split. An experimental backend should not make the production backend unavailable by default.
- Recommendation:
  Split the feature surface, for example `difftest` and `difftest-spike` (or `spike`). Build the wrapper only when the Spike-specific feature is enabled, and keep QEMU difftest usable without Spike installed.

### R-003 `CoreContext is not actually the single source of truth for the compared CSR set`

- Severity: HIGH
- Section: `Summary / Architecture / Phase 3 / Phase 5 / Trade-offs`
- Type: Correctness / Design
- Problem:
  The plan says `CoreContext` carries everything needed, but `QemuBackend` discards `ctx.csrs` and rebuilds a second manual whitelist as `DIFF_CSR_REF`. `ArchSnapshot::diff()` then compares CSR vectors by positional index, not by CSR identity.
- Why it matters:
  This reintroduces exactly the cross-crate drift risk the `CoreContext` design was supposed to eliminate. If xcore changes the CSR order, adds an entry, or changes a mask, xdb can silently compare the wrong CSR against the wrong CSR. Because the compare loop is index-based and does not validate name/address alignment, the failure mode is incorrect difftest behavior, not a loud configuration error.
- Recommendation:
  Make `ctx.csrs` or a single shared exported whitelist the sole source of truth. Backends should derive CSR register mappings from that data, and comparison should validate CSR identity by `addr` or `name` instead of assuming positional lockstep.

### R-004 `the claimed Spike compatibility pin is still not concrete`

- Severity: HIGH
- Section: `Summary / Review Adjustments / Constraints / Phase 4`
- Type: Maintainability
- Problem:
  The plan claims Spike is pinned to a known-good commit, but the actual document still uses the placeholder `<hash>`.
- Why it matters:
  The previous review asked for a real compatibility policy because Spike’s internal C++ interface is not a stable public API. A placeholder is not a support contract. It leaves the build and maintenance target undefined, so the fix is not actionable or reviewable.
- Recommendation:
  Replace `<hash>` with an exact commit or released version and state that this is the supported Spike baseline for the experimental backend. If there are wrapper assumptions tied to that revision, record them explicitly.

### R-005 `the ISA derivation is still hard-coded despite claiming to be runtime-derived`

- Severity: MEDIUM
- Section: `Review Adjustments / Phase 1`
- Type: Spec Alignment
- Problem:
  The review-adjustment text says the ISA string is now derived from `CoreContext`, but the proposed `context()` implementation still hard-codes it from `cfg!(isa64)` to `"rv64imac"` or `"rv32imac"`.
- Why it matters:
  This does not actually resolve the earlier concern; it just moves the hard-coding into xcore. If the DUT configuration grows beyond that subset, the REF backend configuration can drift while the plan still claims the ISA is being derived.
- Recommendation:
  Either derive the ISA string from real architectural/config state or state honestly that round04 still pins a specific ISA subset.

### R-006 `the response to the single-cfg / zero-cost goal is still overstated`

- Severity: LOW
- Section: `Summary / Master Compliance / Goals / Phase 1 / Phase 6`
- Type: Spec Alignment
- Problem:
  The plan says there is a single `cfg(feature = "difftest")` per crate and zero cost when disabled, but the implementation sketch still shows multiple `cfg(feature = "difftest")` call sites and moves `CoreContext` plus `DebugOps::context()` into the always-compiled API surface.
- Why it matters:
  This is no longer the major blocker it was in earlier rounds, but the document should not claim a stricter isolation boundary than it actually implements.
- Recommendation:
  Rephrase the claim more narrowly: xdb’s difftest module is single-gated, while xcore keeps a small always-compiled context API plus a few difftest-gated hooks.

---

## Trade-off Advice

### TR-1 `comparison masking vs synchronization fidelity`

- Related Plan Item: `CsrValue / sync_state()`
- Topic: Simplicity vs Correctness
- Reviewer Position: Prefer correctness
- Advice:
  Comparison masks should not be reused as synchronization payloads.
- Rationale:
  A compact snapshot format is attractive, but once masked values are written back into REF, the harness starts mutating architectural state in ways the DUT never performed.
- Required Action:
  Executor should separate raw CSR state from compare masks, or explicitly preserve masked-out bits on REF writes.

### TR-2 `experimental backend scope vs usable production path`

- Related Plan Item: `Phase 4 / build.rs / Cargo`
- Topic: Scope Honesty vs Simplicity
- Reviewer Position: Prefer a usable QEMU path
- Advice:
  If Spike is experimental, it should not be a mandatory dependency of `--features difftest`.
- Rationale:
  The value of narrowing Spike scope is lost if every difftest build still requires the Spike toolchain.
- Required Action:
  Executor should introduce a separate Spike build feature or otherwise defer Spike wrapper compilation unless that backend is explicitly enabled.

### TR-3 `single source of truth vs duplicated backend tables`

- Related Plan Item: `CoreContext / DIFF_CSR_REF / ArchSnapshot::diff`
- Topic: Clean Boundary vs Drift Risk
- Reviewer Position: Prefer a single source of truth
- Advice:
  Let xcore provide the compared CSR set once, and let xdb consume that exact set.
- Rationale:
  The `CoreContext` refactor only pays off if the backend stops rebuilding parallel copies of the same architectural description.
- Required Action:
  Executor should remove the duplicate QEMU CSR table or validate strict identity alignment before any compare/sync path uses it.

---

## Positive Notes

- The QEMU `PhyMemMode` correction is the right fix and addresses the most important blocker from round03.
- Moving away from compile-time `option_env!("X_FILE")` in the difftest lifecycle is a real improvement.
- The `CoreContext` idea is directionally cleaner than the previous `DifftestOps` design and gives the plan a much better crate boundary to build on.

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
- Reason: Round04 still corrupts REF state by syncing masked CSR values, keeps the experimental Spike backend on the mandatory difftest build path, duplicates the compared CSR definition across crates, and does not yet provide a concrete Spike compatibility pin.
