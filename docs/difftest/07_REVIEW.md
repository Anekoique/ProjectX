# `difftest` REVIEW `07`

> Status: Closed
> Feature: `difftest`
> Iteration: `07`
> Owner: Reviewer
> Target Plan: `07_PLAN.md`
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

`07_PLAN` is the closest round so far. The descriptor-aware `read_register()` fix is the right answer to the shadow-CSR problem, the `continue` slow path is much more concrete, and the wiring sketch is finally close to implementation-level detail. But it still is not implementation-ready. The biggest remaining gap is that the plan only fixes `continue`; it still does not define how `cmd_step()` participates in difftest, even though the current shell has a separate step path with the same bypass pattern. The new named-GPR contract is also still only partially enforced, the RISC-V-only `CoreContext` gating still conflicts with the always-compiled debug API, and the claimed “same CSR addr set” validation is still only one-sided.

---

## Findings

### R-001 `the step path is still not wired through difftest`

- Severity: HIGH
- Section: `Summary / Main Flow / Phase 6`
- Type: Flow
- Problem:
  Round07 explicitly fixes `cmd_continue()`, but it never provides the corresponding `cmd_step()` redesign. The current shell has a separate `cmd_step(count, watch_mgr)` path, and today that path breaks on `done` before any difftest handling.
- Why it matters:
  The plan claims "`s` or `c`" both execute through per-step difftest, but only `continue` is actually specified. That leaves `step` with the same bypass hole the previous review called out for `continue`, especially on halting instructions and mismatch reporting.
- Recommendation:
  Add a concrete `cmd_step(..., diff)` sketch and make it follow the same hook order as the new `cmd_continue()`: capture `ctx`, run `check_step(&ctx, mmio, done)`, then handle halt/watchpoint exit conditions.

### R-002 `the new named-GPR contract is still only half-enforced`

- Severity: HIGH
- Section: `Summary / Phase 3 / Phase 5`
- Type: Correctness / Design
- Problem:
  The round07 plan now caches `gpr_names` in `QemuBackend`, but `diff_contexts()` still validates only vector length and then compares the two GPR lists positionally with `zip()`. It never checks that the names actually agree, and the Spike section never states how that backend reconstructs the named GPR list under the same contract.
- Why it matters:
  The refactor to `Vec<(&'static str, u64)>` was supposed to make the register contract self-describing and remove hidden ISA assumptions from xdb. Round07 still depends on positional identity, and on mismatch it will either silently compare the wrong registers or panic on length drift. That means the new metadata is not yet buying the safety it claims.
- Recommendation:
  Either validate full GPR identity `(name, position)` across DUT/REF and treat disagreement as a normal mismatch, or keep a fixed machine-order array for difftest and reserve the names for UI/reporting. The Spike backend needs the same explicit reconstruction rule as QEMU.

### R-003 `the RISC-V-only gating still does not match the always-compiled debug API`

- Severity: HIGH
- Section: `Summary / Review Adjustments / API Surface / Phase 1`
- Type: Build / Spec Alignment
- Problem:
  The plan says `CoreContext` is only dispatched under `#[cfg(riscv)]` and that LoongArch should fail only when difftest is enabled. But the proposed `DebugOps` trait still unconditionally declares `fn context(&self) -> super::CoreContext`, and the current project’s interactive build path enables `debug` by default.
- Why it matters:
  That means the compile boundary is still underspecified. If `CoreContext` only exists in the RISC-V branch, then a non-RISC-V build with debug support will fail long before “LoongArch + difftest” becomes the only unsupported combination. Round06 asked for a concrete API-level gating design; round07 still answers mostly in prose.
- Recommendation:
  Either gate the `context()` API itself on `#[cfg(riscv)]`/`#[cfg(feature = "difftest")]`, or provide a real `CoreContext` dispatch contract for every compiled architecture branch. Do not document the unsupported case more narrowly than the API actually enforces.

### R-004 `the CSR-set equality fix is still one-sided`

- Severity: HIGH
- Section: `Review Adjustments / Phase 5`
- Type: Correctness
- Problem:
  Round07 changes “missing CSR counterpart” to a mismatch, but `diff_contexts()` still only iterates over `dut.csrs`. If the REF context contains extra or duplicate CSR entries, the comparison still passes silently.
- Why it matters:
  The previous review asked for both contexts to carry the same CSR address set. Round07 now detects “DUT entry missing in REF”, but it still does not enforce symmetric equality of the set. A malformed backend context can therefore remain undetected.
- Recommendation:
  Validate the CSR address set symmetrically before value comparison, or perform a reverse check after the DUT loop. Do not leave REF-only entries outside the contract.

### R-005 `the M-001 response is still just a renamed whitelist constant`

- Severity: MEDIUM
- Section: `Summary / Master Compliance / Phase 1`
- Type: Spec Alignment
- Problem:
  The summary says round07 eliminates `DIFFTEST_CSRS`-style redundancy, but Phase 1 still introduces `const DIFF_CSR_SET`.
- Why it matters:
  This is cleaner than the earlier raw-address duplication because it now keys off `CsrAddr`, but it is still a dedicated difftest whitelist constant. The document should describe that honestly instead of claiming the constant is gone in substance rather than just by name.
- Recommendation:
  Rephrase the claim to “the whitelist now uses `CsrAddr` variants directly” instead of “no separate const”.

### R-006 `batch mode still bypasses hook-based execution and the scope should say so explicitly`

- Severity: LOW
- Section: `Phase 6`
- Type: Scope
- Problem:
  The new `xdb_mainloop()` sketch still routes `X_BATCH=y` straight to `run(u64::MAX)`.
- Why it matters:
  That is probably acceptable if round07 difftest is intentionally interactive-only, because `dt attach` is a monitor command. But the current wording does not make that scope boundary explicit, so the execution model still looks broader than it is.
- Recommendation:
  State explicitly that round07 difftest is monitor-attached interactive flow only, or sketch a batch-safe hook path if batch difftest is meant to be supported.

---

## Trade-off Advice

### TR-1 `separate UI-friendly register names vs machine-compare correctness`

- Related Plan Item: `CoreContext.gprs / diff_contexts()`
- Topic: Clarity vs Contract Safety
- Reviewer Position: Prefer a stricter contract
- Advice:
  If the compare path uses named GPR pairs, it should validate those names instead of discarding them.
- Rationale:
  Otherwise the extra allocation and metadata only serve the UI while the correctness path still depends on hidden positional assumptions.
- Required Action:
  Executor should either validate names/lengths or switch the compare path back to a fixed machine-order structure.

### TR-2 `documenting an unsupported arch vs actually gating the API`

- Related Plan Item: `R-004 response / Phase 1`
- Topic: Scope Honesty vs Compile Hygiene
- Reviewer Position: Prefer compile hygiene
- Advice:
  Unsupported architecture combinations should be enforced by the type/feature boundary, not just narrated in constraints text.
- Rationale:
  The current sketch still leaves too much ambiguity about which builds fail and where.
- Required Action:
  Executor should make the gating concrete in the trait/type surface.

### TR-3 `continue-path detail vs step-path symmetry`

- Related Plan Item: `Phase 6`
- Topic: Completeness vs Iteration Speed
- Reviewer Position: Prefer symmetry
- Advice:
  The same difftest hook rule should be specified for both `continue` and `step`.
- Rationale:
  These commands are two front doors to the same execution engine. Leaving one path implicit is how review escapes become runtime bugs.
- Required Action:
  Executor should add the `cmd_step()` hook order explicitly in the next plan.

---

## Positive Notes

- The descriptor-aware `read_register()` fix is correct and closes the shadow-CSR problem from round06.
- The `continue` slow path is finally concrete enough to review against the current `xdb` command flow.
- The Spike pin and the added `CsrAddr::name()` direction both improve the plan’s precision materially.

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
- Reason: Round07 still leaves the `step` difftest path unspecified, only partially enforces the new named-GPR contract, does not yet make the RISC-V-only `CoreContext` API boundary compile-clean, and still validates the compared CSR set in only one direction.
