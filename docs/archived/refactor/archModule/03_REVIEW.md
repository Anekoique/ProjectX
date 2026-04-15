# `archModule` REVIEW `03`

> Status: Closed
> Feature: `archModule`
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

- Decision: Approved with Revisions
- Blocking Issues: 0
- Non-Blocking Issues: 4



## Summary

Round 03 converges cleanly on the three round-02 blockers. All findings are
verified against the codebase:

- **R-016 is fully resolved.** The per-site table in Phase 2c enumerates
  exactly the 11 `pub(in crate::cpu::riscv)` sites that currently exist
  under `xemu/xcore/src/cpu/riscv/` (grep confirms: `trap.rs:21,26,31,35,55`;
  `csr.rs:95`; `csr/ops.rs:7,16`; `mm/tlb.rs:10,58`; `mm/mmu.rs:20`). The
  8-wide / 3-mm-local split is sound — `Tlb`, `TlbEntry`, and `Mmu::tlb` are
  the only truly topic-local items; every other site has a documented
  cross-topic consumer under `arch/riscv/`.
- **R-017 is resolved via option (b).** The chosen seam shape
  (cfg-gated `pub type` aliases for the concrete arch types named by
  upper-layer consumers) correctly covers the three types actually crossing
  the seam today — `Core`, `CoreContext`, `PendingTrap` (cross-referenced
  against `lib.rs:36`, `error.rs:5,13,47`, `xdb/src/difftest/{mod,qemu,spike}.rs`,
  and the existing glob at `cpu/riscv/mod.rs:15`). I-6 / C-5 relax
  honestly from "exactly one `#[cfg]` line" to "only cfg-gated aliases /
  re-exports." TR-6's argument against associated types on `CoreOps` is
  sound: xdb's difftest reads `CoreContext` by field, so associated types
  would force a concrete struct behind the alias anyway while forcing
  downstream edits — a wash for M-004 at material cost to I-4.
- **R-018 is resolved via option (a).** `TrapIntake` / `InterruptSink` /
  `IntcSet` are dropped; M-004's Response Matrix row correctly reads
  **Partial**; the bus-level residuals (`Bus::{aclint_idx, plic_idx, mtime,
  set_timer_source, set_irq_sink, ssip_flag, take_ssip}` — all confirmed
  present in `device/bus.rs`) are quarantined under NG-5 and mapped to the
  named follow-up tasks `aclintSplit` / `plicGateway` / `directIrq`.

(a) **Round-02 blocker resolution:** R-016 / R-017 / R-018 are all adequately
resolved, with the codebase grep matching the plan's enumeration exactly and
no silent architecture change. (b) **Round-00/01 MASTER compliance:**
00-M-001 (no global `trait Arch`), 00-M-002 (flat layout), 01-M-001 (no
`selected`), 01-M-003 (`build.rs` authoritative), 01-M-002 (plan length —
this plan is materially shorter than 02) all remain faithfully applied;
01-M-004 is honestly scoped as **Partial** with a concrete follow-up path.
(c) **Implementability with zero test/boot regressions:** the phased PR
plan, per-PR green-bar command list, and `sync_interrupts` byte-identity
commitment (now explicit in C-2 / R-021) are all compatible with the user's
five-gate preservation requirement. The Phase 4 import rewrite was
cross-checked against `aclint.rs:13` (imports `MSIP, MTIP`), `plic.rs:6`
(imports `MEIP, SEIP`), and `cpu/riscv/mod.rs:25,132-144` (imports
`HW_IP_MASK`; `sync_interrupts` body references it once) — the plan's
claim of pure import-rewrite is accurate. (d) **Recommendation on
03_MASTER:** the user can safely skip 03_MASTER again. No CRITICAL
remains; the four non-blocking items below are improvement notes for
round 04's plan body or the final implementation, not architectural
concerns.

Numbering: prior iteration ended at R-022 / TR-6. This round opens new
findings at R-023 (LOW) and R-024..R-026 (LOW). No new trade-off
advice — TR-6 is the sole open trade-off and the plan's decision is
endorsed.



---

## Findings

### R-023 `V-F-5 gate not expressed as a cargo-runnable assertion`

- Severity: LOW
- Section: Validation
- Type: Validation
- Problem:
  V-F-5 (the `pub(in …)` gate) is expressed as a shell one-liner
  (`rg 'pub\(in crate::(cpu|isa)::(riscv|loongarch)' xemu/xcore/src`
  returns 0 hits) rather than as an assertion inside
  `arch_isolation.rs`. The plan already builds a text-walking
  integration test that reads every `.rs` file under `src/`; folding
  V-F-5's regex into that same walk would make the gate fail fast in
  CI via `cargo test --workspace` at every PR boundary rather than
  relying on reviewer memory or a separate `rg` invocation.
- Why it matters:
  V-F-5 is the load-bearing verification for R-002 / R-016 (00
  CRITICAL + 02 CRITICAL). Leaving it as an out-of-band shell gate
  means a contributor who reintroduces `pub(in crate::cpu::riscv)`
  inside `arch/riscv/` (e.g. via copy-paste from round-02 history)
  will only be caught by manual review, not by `cargo test`.
- Recommendation:
  In Phase 5, add one clause to `arch_isolation.rs`: for every file
  under `src/arch/`, `content.contains("pub(in crate::cpu::")` and
  `content.contains("pub(in crate::isa::")` are both false. This is
  a one-line addition to the existing `std::fs` walk — no new
  dep, no NG-7 risk. Update the Acceptance Mapping row for V-F-5 /
  R-002 / R-016 to point at this assertion rather than the raw `rg`
  command.



### R-024 `arch_isolation.rs allow-list may miss aclint/plic strings in test_helpers or test modules`

- Severity: LOW
- Section: Validation / V-UT-1
- Type: Validation
- Problem:
  V-UT-1's vocabulary-isolation clause pins `info!("aclint: …")` /
  `info!("plic: …")` in `device/bus.rs` only. A quick grep of the
  tree shows those two substrings also appear inside the `#[cfg(test)]`
  module of `device/bus.rs` (line 425-430: `bus.plic_idx`,
  `"plic"` literal in `add_mmio`) and inside `device/intc/aclint.rs`
  / `plic.rs` tests. The arch/-prefixed file path filter will cover
  the intc tests once they relocate under `arch/riscv/device/intc/`,
  but `device/bus.rs`'s own `#[cfg(test)]` block uses the `"plic"`
  literal in `add_mmio("plic", …)` at line 428 — same file, same
  allow-list, but the plan lists only the `info!` log call sites
  explicitly.
- Why it matters:
  V-UT-1 is the structural guard for I-1 / I-2. If the allow-list
  under-enumerates the `device/bus.rs` occurrences, the test fails
  on first run and the contributor either broadens the allow-list
  without a clear anchor or disables the clause. The fix is trivial
  but the plan should say so before implementation, not during.
- Recommendation:
  In the plan's V-UT-1 allow-list for `device/bus.rs`, enumerate
  every `aclint` / `plic` substring occurrence (including
  `add_mmio("plic", …)` at `device/bus.rs:428`, and the existing
  `info!("bus: slow tick: plic")` if present), each with a short
  anchor (`// NG-5: plicGateway` etc.). An initial `grep -n aclint
  xemu/xcore/src/device/bus.rs; grep -n plic xemu/xcore/src/device/bus.rs`
  gives the full list; the plan's allow-list should reference the
  line-count ("N occurrences each of `aclint`/`plic`") rather than
  a fixed subset.



### R-025 `Phase 4 import-rewrite in arch/riscv/cpu/mod.rs is understated — more than two import lines change`

- Severity: LOW
- Section: Implementation / Phase 4
- Type: Correctness
- Problem:
  Phase 4 step 6 describes two import-line edits in
  `arch/riscv/cpu/mod.rs`:
  (a) `crate::device::intc::{aclint::Aclint, plic::Plic}` →
      `super::device::intc::{Intc, aclint::Aclint, plic::Plic}`;
  (b) `crate::device::HW_IP_MASK` →
      `super::trap::interrupt::HW_IP_MASK`.
  The current file (`cpu/riscv/mod.rs:22-34`) has a single grouped
  import that also pulls `IrqState`, `Bus`, `TestFinisher`, `Uart`,
  `VirtioBlk` from `crate::device`, plus `DECODER`, `DecodedInst`,
  `RVReg` from `crate::isa`. After 2c those `crate::isa::…` paths
  likely need to rewrite to `super::isa::…` (since `arch/riscv/isa/`
  exists and `isa/mod.rs`'s glob re-export is now cfg-gated to
  `crate::arch::riscv::isa::*`, not `crate::isa::*` glob). Also, the
  Intra-arch Imports block in the Data Structure section lists
  `super::csr::{CsrAddr, CsrFile, MStatus, Mip, PrivilegeMode}`,
  `super::mm::{Mmu, Pmp}`, `super::trap::{…}` — all of these are
  *new* import paths; the current file uses `use self::{csr::…,
  mm::…, trap::…}` at line 16-20.
- Why it matters:
  This is not a correctness gap (the edits are obvious and
  mechanical), but the plan's "two imports edited" phrasing in the
  PR 4 body at line 649 understates the diff. If the executor reads
  only the PR 4 body and not the Data Structure section's
  intra-arch import block, they may miss the `self::` →
  `super::` conversions that Phase 2c / 2b triggered. Flag as LOW
  because the full list does appear in Data Structure — this is a
  plan-readability concern, not a hidden behaviour change.
- Recommendation:
  In the Phase 4 body or in Phase 2c (where the relocation actually
  happens), add one sentence: "All `self::{csr,mm,trap}::…` imports
  in `arch/riscv/cpu/mod.rs` become `super::{csr,mm,trap}::…` as a
  mechanical consequence of the flat hoist; this rewrite is already
  shown in the Intra-arch Imports block of the Data Structure
  section." No new content — just cross-reference so the executor
  doesn't have to reconstruct the delta.



### R-026 `TR-6's associated-type option (a) risk statement is accurate but Trade-off framing slightly overclaims "zero M-004 gain"`

- Severity: LOW
- Section: Trade-offs / TR-6
- Type: Maintainability
- Problem:
  TR-6 argues that associated types on `CoreOps` give "zero M-004
  gain for real work" because xdb reads `CoreContext` by field.
  This is accurate for the **current** xdb, but associated types
  would enable a later migration where `CoreOps::Context` exposes
  a small accessor trait (e.g. `fn pc(&self) -> Word`, `fn gpr(&self,
  i: usize) -> Word`) and xdb's difftest reads through those
  accessors. The gain is not zero — it is "zero under today's xdb
  source," which is a different statement. Since round 03 explicitly
  scopes out xdb edits (NG-3) and defers M-004's residual ambition
  to a later refactor, the chosen option (b) is still correct; the
  rationale just over-claims.
- Why it matters:
  A future iteration (post-archModule) that revisits M-004 will
  re-read TR-6 as the last word on the choice. If it reads "zero
  gain," the iteration may skip associated types on `CoreOps`
  entirely, foreclosing a genuine M-004 path. This is a
  documentation-future-reader risk, not a round-03 implementation
  risk.
- Recommendation:
  One-line tweak to TR-6's option (a) "Con" bullet: change "Zero
  M-004 gain for real work" to "Zero M-004 gain **under today's
  xdb source**; the full trait-dispatch landing requires a
  coordinated xdb edit and is deferred to a later refactor." Keeps
  the decision unchanged, keeps the rationale honest for future
  readers.



---

## Trade-off Advice

No new trade-off advice this round. TR-6 (concrete alias vs
associated-type-on-`CoreOps` for `CoreContext` / `PendingTrap`) is the
sole open trade-off; the plan's choice of option (b) is endorsed —
TR-6's rationale is sound modulo the R-026 wording tweak above.

Closed trade-offs from prior rounds (TR-1 closed by 00-M-001; TR-2,
TR-3, TR-4 accepted in round 02; TR-5 closed by R-010; TR-6 of round
02 closed by R-018 option a) are correctly recorded in the Response
Matrix.



---

## Positive Notes

- **Per-site `pub(in …)` table is the right level of detail.** Eleven
  rows with `file:line`, old scope, new scope, and the specific
  cross-topic consumer for each. Grepping
  `pub(in crate::(cpu|isa)::(riscv|loongarch)` against the live tree
  returns exactly the 11 entries in the table, with matching line
  numbers. This is the correct resolution of R-016.
- **Seam enumeration (R-017) matches reality.** `Core`, `CoreContext`,
  `PendingTrap` are exactly the three concrete arch types crossing
  the seam today (`cpu/riscv/mod.rs:15`; `lib.rs:36`; `error.rs:5`;
  `xdb/src/difftest/*` all verified). No missed consumer. The
  LoongArch note ("add `CoreContext` / `PendingTrap` when that backend
  materialises") is honest.
- **R-018's Partial landing is the right call.** Introducing trait
  scaffolding (`TrapIntake`, `InterruptSink`, `IntcSet`) that only
  mediates arch-local dispatch would have absorbed review effort and
  left `Bus::{aclint_idx, plic_idx, mtime, set_timer_source,
  set_irq_sink, ssip_flag, take_ssip}` — the real leakage — untouched.
  Recording M-004 as **Partial** with the bus residuals queued under
  named follow-up tasks (`aclintSplit` / `plicGateway` / `directIrq`,
  pinned in the `arch_isolation.rs` allow-list as NG-5) is the honest
  and correct landing.
- **Test/boot preservation is first-class.** G-6 enumerates the five
  gates (336 unit tests, 31 cpu-tests-rs, 8 am-tests, `make linux`,
  `make debian`, difftest zero divergence) and the Acceptance Mapping
  maps each to a named V-IT / V-F row. The green-bar command block
  (X_ARCH matrix + DEBUG=n) is directly runnable from a terminal.
- **Plan is materially shorter than 02_PLAN** (01-M-002 compliance).
  Trade-offs collapse to TR-6; the ceremonial-trait material is
  excised; the `sync_interrupts` rewire language is gone.
- **Phase 2 merge-gating is tight.** 2a/2b/2c are commit-only inside
  PR 2; only PR 2 lands on trunk. No non-green commit reaches trunk,
  which keeps `git bisect` useful (R-020 resolved).
- **V-UT-1 is `std`-only.** No `aho-corasick` dev-dep; NG-7 preserved
  (R-022 resolved).

---

## Approval Conditions

### Must Fix
- (none)

### Should Improve
- R-023 (fold `pub(in …)` gate into `arch_isolation.rs`)
- R-024 (enumerate all `aclint` / `plic` substrings in
  `device/bus.rs` allow-list, not only the `info!` call sites)
- R-025 (cross-reference the intra-arch `self::` → `super::` import
  rewrite in the Phase 4 body)
- R-026 (soften TR-6 option-(a) "zero M-004 gain" wording)

### Trade-off Responses Required
- (none — TR-6 is endorsed as chosen)

### Ready for Implementation
- Yes
- Reason: No CRITICAL remains. R-016 / R-017 / R-018 (round-02
  blockers) are verified against the codebase as fully and
  correctly resolved. Round-00/01 MASTER directives remain faithfully
  applied (01-M-004 is honestly scoped as **Partial** with a
  concrete follow-up path, not silently weakened). The phased PR
  plan plus per-PR green-bar matrix plus byte-identical
  `sync_interrupts` commitment preserve the user's five-gate
  test/boot requirement at every trunk-bound PR boundary. R-023
  through R-026 are non-blocking improvements; the executor may
  fold them into Phase 5 during implementation or carry them as
  plan-body tweaks into round 04 if the user reopens the loop. The
  user can safely skip 03_MASTER.
