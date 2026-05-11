# `difftest` REVIEW `06`

> Status: Closed
> Feature: `difftest`
> Iteration: `06`
> Owner: Reviewer
> Target Plan: `06_PLAN.md`
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

`06_PLAN` is materially closer than the previous rounds. Moving `diff()` to a free function is the right fix for the foreign-type problem, restoring `read_register()` is the right direction for debugger coverage, the Spike pin is now concrete, and the explicit `continue` slow path is much better than the earlier hand-wave. But the plan still is not implementation-ready. The biggest remaining problem is that it still treats `read_register()` as “full CSR coverage” even though the current implementation reads raw CSR storage and therefore does not correctly model shadow/view CSRs like `sstatus`/`sie`/`sip`. On top of that, the new `continue` sketch skips difftest on the final halting instruction, the named-GPR contract is still incomplete in the QEMU backend, and the claimed non-RISC-V gating is still only stated in prose while the always-compiled API still depends on `CoreContext`.

---

## Findings

### R-001 `the restored read_register path still does not provide the full CSR semantics the plan claims`

- Severity: HIGH
- Section: `Review Adjustments / Invariants / Phase 2 / Phase 3`
- Type: Correctness
- Problem:
  Round06 resolves the debugger-regression item by restoring `DebugOps::read_register()`, and the plan then relies on that API for “full CSR coverage” in `info reg <name>` and `p $csr`. But the current implementation reads CSR values through `self.csr.get(a)` after `CsrAddr::from_name(name)`, which is a raw storage read.
- Why it matters:
  The CSR framework already models view aliases through descriptors in `csr_table!`, for example `sstatus`, `sie`, and `sip` are shadows of machine CSRs with masks. A raw storage read does not honor that descriptor/view behavior. That means the round06 fix still does not actually deliver the debugger semantics it claims for arbitrary CSR names, which matters for the OpenSBI/xv6/Linux path where supervisor-visible CSR state matters.
- Recommendation:
  Make `read_register()` descriptor-aware, for example via `find_desc()` + `read_with_desc()` or a dedicated `CsrFile` helper that resolves a named CSR through the existing descriptor framework instead of raw storage.

### R-002 `the new continue loop drops difftest on the final halting instruction`

- Severity: HIGH
- Section: `Main Flow / Continue path`
- Type: Correctness
- Problem:
  In the round06 `cmd_continue()` sketch, the loop breaks immediately when `done` is true, before the difftest branch calls `h.check_step(&ctx, mmio, false)`.
- Why it matters:
  That means the last instruction that halts the DUT is never stepped/synchronized on REF in the `continue` path. This contradicts the round06 main flow (`halted` goes through `check_step`) and the invariant that halting instructions should sync raw state and skip compare. It is exactly the kind of edge-case hole that will show up during trap/exit paths.
- Recommendation:
  Call `check_step()` before the `done` break and pass the real halted state through. Mirror the same rule in the step path, not just `continue`.

### R-003 `the named-GPR contract is still incomplete and the compare path still trusts positional zip`

- Severity: HIGH
- Section: `Data Structure / Phase 5 / Phase 7 / Trade-offs`
- Type: Design / Correctness
- Problem:
  The new design makes `CoreContext.gprs` a self-describing `Vec<(&'static str, u64)>`, but `QemuBackend` does not actually store the GPR-name metadata it says it will need to reconstruct REF contexts. At the same time, `diff_contexts()` ignores the names and just zips the DUT/REF vectors positionally.
- Why it matters:
  This leaves the new abstraction half-finished. If the REF-side GPR names/order are missing or drift, the compare path can silently pair the wrong registers or truncate on length mismatch. The whole point of switching away from `[u64; 32]` was to make the contract self-describing; round06 still does not enforce that contract.
- Recommendation:
  Either store and validate the ordered GPR-name list explicitly in the backend, or keep a fixed machine-order register array for difftest and reserve names for UI/reporting. In either case, `diff_contexts()` should validate length/name agreement instead of trusting `zip()`.

### R-004 `the RISC-V-only gating is still documented, not actually specified in the API surface`

- Severity: HIGH
- Section: `Review Adjustments / Constraints / Phase 1 / Phase 2`
- Type: Build / Spec Alignment
- Problem:
  The plan says non-RISC-V architectures are gated out for this round, but the always-compiled `DebugOps` trait still returns `super::CoreContext`, while the dispatch snippet only defines `CoreContext` in the `#[cfg(riscv)]` branch.
- Why it matters:
  The tree still has `cpu/loongarch` and `isa/loongarch` branches. A prose note in `C-12` is not enough if the public API shape still assumes that `cpu::CoreContext` exists unconditionally. Round06 needs to specify the actual compile-time gating strategy, not just the intended scope.
- Recommendation:
  Either gate the `context()` API surface itself on `#[cfg(riscv)]`, or provide parallel dispatch/type aliases for the other architecture branches. The review issue is the missing concrete gating design, not whether the project should support loongarch difftest today.

### R-005 `the xdb state-threading changes are still underspecified`

- Severity: MEDIUM
- Section: `Continue path / File Summary`
- Type: Completeness
- Problem:
  The plan updates `cmd_continue()` to take `diff: &mut Option<DiffHarness>`, but it does not show the corresponding `cli::respond(...)` / `xdb_mainloop(...)` signature and state-owner changes that make that compile, even though the current shell only threads `WatchManager`.
- Why it matters:
  The file summary says `main.rs` and `cli.rs` are part of the wiring, so this is probably intended, but the plan still leaves an important part of the state-flow implicit.
- Recommendation:
  Add a short concrete wiring sketch for `respond()` / `xdb_mainloop()` ownership of `loaded_binary_path` and `diff_harness` so the command integration is reviewable end-to-end.

### R-006 `the response to M-002 is still slightly overstated`

- Severity: LOW
- Section: `Master Compliance / Changes from Previous Round / Data Structure`
- Type: Spec Alignment
- Problem:
  The plan says it removed separate `CsrValue` / `DIFF_CSRS`-style structures and integrated the whitelist into the CSR framework, but the actual design still introduces a new `CsrSnapshot` struct and a dedicated `DIFFTEST_CSRS` whitelist constant.
- Why it matters:
  The design is cleaner than round05 because it now keys off `CsrAddr`, but it is not as fully integrated into the existing CSR framework as the summary language suggests.
- Recommendation:
  Rephrase the claim more narrowly: the whitelist now uses `CsrAddr` and `csr_table!` names instead of a raw-address shadow table, but a dedicated difftest snapshot type and whitelist still exist.

---

## Trade-off Advice

### TR-1 `restoring an API vs restoring the right semantics`

- Related Plan Item: `R-002 response / read_register`
- Topic: API Compatibility vs Correctness
- Reviewer Position: Prefer correctness
- Advice:
  Keeping `read_register()` is not enough if it still bypasses the descriptor/view logic already present in the CSR framework.
- Rationale:
  The debugger needs the right architectural value, not just a familiar function name.
- Required Action:
  Executor should route named CSR reads through descriptor-aware CSR lookup instead of raw storage access.

### TR-2 `self-describing contexts vs simple positional compare`

- Related Plan Item: `CoreContext.gprs / diff_contexts()`
- Topic: Clarity vs Simplicity
- Reviewer Position: Prefer an explicit contract
- Advice:
  If `gprs` become named vectors, the compare path should enforce that identity instead of discarding it.
- Rationale:
  Otherwise the added allocation and metadata do not buy real safety; the code still depends on hidden positional assumptions.
- Required Action:
  Executor should either validate names/lengths or step back to a fixed machine-order array for the machine-compare path.

### TR-3 `fast continue behavior vs correct terminal-step difftest`

- Related Plan Item: `cmd_continue()`
- Topic: Throughput vs Coverage
- Reviewer Position: Prefer coverage
- Advice:
  The halting instruction still needs to flow through difftest handling.
- Rationale:
  Exit/trap/ebreak edges are exactly where the design is most likely to diverge; skipping the last step leaves a blind spot.
- Required Action:
  Executor should explicitly run `check_step(..., halted=true)` on the final iteration before exiting the loop.

---

## Positive Notes

- Using a free `diff_contexts()` function is the right fix for the foreign-type impl problem from round05.
- Restoring `read_register()` and making `cmd_continue()` difftest-aware are both directionally correct responses to the previous review.
- The Spike pin is now concrete and accurately versioned enough to serve as a real experimental support target.

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
- Reason: Round06 still overstates the restored debugger CSR coverage, skips difftest handling on the final halting step in the new continue loop, leaves the named-GPR contract incomplete, and does not yet specify a concrete compile-time gating strategy for the RISC-V-only `CoreContext` API.
