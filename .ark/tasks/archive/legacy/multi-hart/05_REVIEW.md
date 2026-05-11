# `multiHart` REVIEW `05`

> Status: Closed
> Feature: `multiHart`
> Iteration: `05`
> Owner: Reviewer
> Target Plan: `05_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved
- Blocking Issues: 0
- Non-Blocking Issues: 3



## Summary

Round 05 closes the multiHart pivot cleanly. Every round-04 finding
is absorbed with an explicit, ground-truth-anchored resolution:

- **R-026 (HIGH)**: defining `HartId` directly at `cpu/core.rs` as
  a plain newtype, with a *declared* two-token allow-list widening
  (`SEAM_FILES += "src/cpu/core.rs"`, `SEAM_ALLOWED_SYMBOLS +=
  "HartId"`). Verified against `xemu/xcore/tests/arch_isolation.rs:31-65`
  — the additions slot in cleanly. Because `HartId` is *defined* (not
  re-exported) in `cpu/core.rs`, the file body has zero
  `crate::arch::riscv::` references and the regex check trivially
  passes; the two allow-list tokens cover the symbol re-mention
  vector.
- **R-027 (MEDIUM)**: `<B>` method generic dropped; cfg-gated
  `pub type CoreBuilder` alias parallels the existing `Core` alias at
  `cpu/mod.rs`. Call sites (`xdb/src/main.rs`) use
  `CPU::from_config(config, layout)` with no turbofish.
- **R-028 (LOW)**: ordering-prose comment added in `CPU::step`
  pseudocode (lines 202-205) and I-8 (lines 296-302), plus
  `debug_assert!(src < self.cores.len())` in
  `invalidate_reservations_except`.
- **R-029 (LOW)**: plan body is **719 lines** vs C-7 ceiling 720
  — within budget. Trade-offs trimmed via T-2..T-11 collapsed bullet.
- **R-030 (LOW)**: V-IT-3 demoted to V-UT-15 in `cpu/mod.rs::tests`;
  Acceptance Mapping G-5 row updated to `V-UT-15, V-IT-4, V-E-3`.
- **TR-9** adopted (`CoreBuilder` alias). **TR-10** endorsed with the
  `setup_core_and_bus()` snippet now inline at lines 263-268.

The user-ordered concurrency audit lands as a top-level Concurrency
Matrix (CC-1..CC-10) bound to three new invariants (I-10/I-11/I-12)
and one new constraint (C-8). Each row states current shape, NG-2
safety basis ("one writer per hart at any point; tick and step are
temporally disjoint on the same thread"), and a future-MT escalation
delta. The matrix is implementation-actionable: CC-1 → I-11
identifies that `Mswi.irqs[h]`, `Mtimer.irqs[h]`, `Plic.irqs[h]` are
all `.clone()` of the *same* `Arc<AtomicU64>` per hart, producing
strictly-equal `.load()` results. CC-7 anchors the R-028 ordering
in the matrix as well as in I-8 prose, providing two independent
guards against future refactor reordering.

Carry-forward integrity is good: every prior R-001..R-025 lands in
the Response Matrix; inherited MASTER directives 00-M-001/002 +
01-M-001..004 are explicitly applied; C-7, R-013 (X_HARTS env var),
R-016 (granule overlap), R-020 (mm-layer hook), R-022 (14 PLIC
tests), TR-3 (3-PR split), TR-7 (PR2a +1 test) all survive
unchanged. No `Hart` noun appears as a struct anywhere in the body
— RVCore-is-hart is preserved.

Test arithmetic is internally consistent: 354 baseline + 11 PR1 →
365 lib + 1 + 6 = 372; +1 PR2a = 366 lib = 373; +3 PR2b = 369 lib
= 376. The 11-test PR1 enumeration (V-UT-3..7, 9, 11..15) is
explicit by file location, and the V-UT-1/V-UT-2 rebase-into-existing
explanation is unchanged from round 04.

Three LOW non-blocking observations remain (R-031 plan-body grazes
719/720 ceiling with literally one line of headroom; R-032
`access_bus` should be enumerated alongside the "8 mm.rs methods"
threading list to make the bus-parameter scope unambiguous; R-033
V-UT-15 single-hart fairness assertion is still degenerate per the
original R-030 logic — demoting from integration to unit test is a
classification change, not a coverage improvement). All three are
post-merge editorials, not blockers.

**Approve. Ready for Implementation: Yes.** Round 05 lands as
APPROVE / Ready=Yes within the 5-round cap. Implementation may
begin on PR1.



---

## Findings

### R-031 Plan body at 719/720 — one line of C-7 headroom

- Severity: LOW
- Section: Spec / Constraints / C-7
- Type: Maintainability
- Problem:
  `wc -l 05_PLAN.md` returns 719, against C-7 ceiling of 720
  (target ≤ 700 per R-029 Log entry). Exactly 1 line of headroom.
  R-029 was supposed to reclaim slack via Trade-offs trimming;
  T-2..T-11 are now a single-bullet collapse but the saved space is
  immediately consumed by the Concurrency Matrix and three new
  invariants. The audit-trail concern from rounds 03/04 (R-025,
  R-029) is unresolved — any in-flight amendment during PR1
  implementation will trigger another C-7 relaxation or a quiet
  violation.
- Why it matters:
  Pure hygiene. With the loop closing at round 05 and PR1 now
  green-lit, this is the last opportunity to either (a) accept that
  C-7 will continue to creep round-by-round and codify a "budget
  rises with scope" rule, or (b) factor verbose sub-sections (e.g.,
  the 14-row Acceptance Mapping table at lines 685-719 could move
  to an appendix or be expressed as `goal ↔ test_id` per-line
  shorthand). Non-blocking; the plan is ready to implement at 719.
- Recommendation:
  Document in the PR1 description that C-7 was hit at 719/720 and
  any plan amendments during implementation should land as
  appendix material rather than inline edits. No plan change
  required.



### R-032 `access_bus` is the 9th mm.rs method touching `self.bus` but is not listed in step 4

- Severity: LOW
- Section: Implement / step 4 (line 456-459) + §API Surface (line 372-374)
- Type: Spec Alignment
- Problem:
  Step 4 says "Thread `bus: &mut Bus` through 8 `mm.rs` methods";
  §API Surface comment lists "checked_read, checked_write, fetch,
  load, store, amo_load, amo_store, translate". Ground truth at
  `xemu/xcore/src/arch/riscv/cpu/mm.rs:251-326` shows
  `access_bus(&mut self, addr, op, size) -> XResult<usize>` is
  *also* part of the bus-touching call graph (it's the callee of
  `checked_read` / `checked_write` / `translate` and uses
  `self.pmp.check`, not `self.bus.read/write` directly — so it may
  not strictly need a `bus` parameter). The plan's count of "8" is
  ambiguous: if `access_bus` doesn't need `bus` threading (because
  it doesn't dereference `self.bus`), then 8 is correct and a
  one-line clarifier would prevent executor over-threading. If
  `access_bus` *does* need it (e.g., for PMP coverage of MMIO
  regions via bus-resolved physical addresses), then the count
  should be 9 and the API Surface enumeration should include it.
- Why it matters:
  Step 4 is the single largest mechanical edit in PR1. An off-by-one
  in the method enumeration costs an executor 5-10 minutes of
  re-grepping during implementation. Verified at
  `arch/riscv/cpu/mm.rs:251` that `access_bus` returns
  `XResult<usize>` and is the funnel through which `checked_*`
  resolve `pa` — the R-020 hook prose at line 250 of the plan
  correctly notes "`pa` is in scope from `access_bus`", so the
  callee is implicitly understood, but the step-4 list should
  state it.
- Recommendation:
  Add one line to step 4 (or a sub-bullet under the threading
  enumeration in §API Surface) clarifying that `access_bus` is the
  funnel; whether it takes `&mut Bus` depends on whether PMP
  resolution needs the bus (likely no — PMP is purely a CSR check),
  in which case `access_bus` keeps its current signature and only
  its callees thread the parameter. One sentence suffices.



### R-033 V-UT-15 single-hart fairness is the same degenerate property as round-04 V-IT-3

- Severity: LOW
- Section: Validation / Unit Tests — PR1 (line 629)
- Type: Validation
- Problem:
  V-UT-15 (`cpu_step_advances_current_single_hart`) at
  `cpu/mod.rs::tests` was the round-04 R-030 resolution: V-IT-3
  was rejected as a degenerate fairness check (with `num_harts = 1`
  the assertion is `(0 + 1) % 1 == 0`). The fix demoted V-IT-3 from
  integration to unit, which is a *classification* change but the
  underlying assertion is identical: at `num_harts = 1` the only
  property is "current stays at 0". This is a 1-line property test
  that proves nothing about round-robin behaviour — it proves the
  modulo arithmetic isn't broken, which is already covered by
  V-IT-4 (`round_robin_fairness_two_harts`) and V-E-3 (wraparound
  at N=2). The Acceptance Mapping at line 690 lists
  G-5 → "V-UT-15, V-IT-4, V-E-3" — the latter two carry the
  meaningful coverage.
- Why it matters:
  Test-count integrity. V-UT-15 inflates the PR1 lib count by 1
  (365 vs 364) for an assertion that single-handedly cannot fail
  unless modulo is broken. If V-UT-15 is dropped, PR1 arithmetic
  becomes 364 + 1 + 6 = 371; the gate matrix at line 567 needs the
  number adjusted. Non-blocking — the test passes trivially and
  the inflated count doesn't hurt anything except signal value.
- Recommendation:
  Either (a) keep V-UT-15 with a one-line comment stating "smoke
  test for `(0 + 1) % 1 == 0` modulo arithmetic — meaningful
  fairness coverage at V-IT-4 / V-E-3", or (b) drop V-UT-15 and
  adjust the gate-matrix arithmetic to 364/365/368 lib (371/372/375
  total). Reviewer leans (a) — keeping a passing assertion costs
  nothing and the comment would prevent a future reviewer from
  re-raising the same point. Editorial; do not block PR1 on this.



---

## Trade-off Advice

(No new trade-offs raised by round 05 require reviewer position;
TR-9 / TR-10 from round 04 were both addressed in the plan.
Concurrency Matrix CC-1..CC-10 is the user-ordered audit and not a
trade-off proposal — it's a constraint codification, mapped to
I-10/I-11/I-12 + C-8.)



---

## Positive Notes

- **R-026 resolution is structurally clean.** Defining `HartId` at
  `cpu/core.rs` as `pub struct HartId(pub u32)` rather than
  re-exporting from arch keeps the seam vocabulary tight: the file
  body has zero `crate::arch::riscv::` references, so the regex
  check at `arch_isolation.rs` trivially passes. The two-token
  allow-list widening (`"src/cpu/core.rs"` to `SEAM_FILES`,
  `"HartId"` to `SEAM_ALLOWED_SYMBOLS`) is the minimum-surface fix.
  Both edits land in PR1 step 1 as a single atomic action — no
  ordering hazard. Verified against
  `xemu/xcore/tests/arch_isolation.rs:31-65`: the new entries slot
  in cleanly without disrupting the existing 5-file / 17-symbol
  vocabulary.
- **CoreBuilder alias is the right shape.** Parallels the existing
  `Core` alias at `cpu/mod.rs`, lives in an existing seam file (no
  new `SEAM_FILES` entries beyond R-026's), and reduces
  `CPU::from_config` to a no-turbofish call site. The trait
  `MachineBuilder` itself stays arch-agnostic at `cpu/core.rs`;
  only the *binding* lives at the seam — exactly the round-04 TR-9
  recommendation.
- **R-028 ordering is double-guarded.** The pseudocode comment at
  lines 202-205 explains the `take_last_store` → cursor-advance
  ordering, I-8 (lines 296-302) restates it as a post-condition,
  and `debug_assert!(src < self.cores.len())` in
  `invalidate_reservations_except` traps the wildly-wrong-`src`
  case at runtime. A future refactor that swaps the two lines
  would have to defeat both the prose anchor and the assertion to
  silently break G-10 — that's the right amount of belt-and-braces
  for a load-bearing invariant.
- **Concurrency Matrix is implementation-actionable, not abstract.**
  Each CC-row names the concrete shape (`Per-hart IrqState =
  Arc<AtomicU64>` for CC-1, `Vec<Arc<AtomicBool>>` for CC-2,
  load-then-set on `Mtimer.check_timer` for CC-3, etc.), states
  the NG-2 safety basis ("tick thread sole writer", "swap-to-false
  atomic"), and points to the future-MT escalation (Relaxed →
  AcqRel for IRQ state, CAS loop or per-hart mutex for MTIMER).
  Bound to three new invariants (I-10 reservation isolation; I-11
  IrqState clone equivalence; I-12 SSIP per-hart edge), the matrix
  is testable rather than narrative. CC-4 (XCPU poison) is correctly
  classified KNOWN-LIMITATION (NG-11) because lock-shape changes
  would force a `parking_lot` dep, violating C-6.
- **NG-2 rationale paragraph (lines 106-113) is precise.** "All
  harts run on one OS thread via `CPU::step` round-robin … each
  hart's `IrqState` / `ssip_pending[h]` / `reservation` /
  `mhartid` / GPRs has exactly one writer at a time." This is the
  load-bearing safety argument for the entire matrix and it
  explicitly identifies the two temporally-disjoint writers on the
  IrqState (`bus.tick` before `core.step`, then `sync_interrupts`
  inside `core.step`) — the right level of detail for a reviewer
  to falsify if wrong.
- **Test-fixture snippet eliminates round-04 ambiguity.** The
  `setup_core_and_bus()` template at lines 263-268 with the
  enumerated 8 site list (`mm.rs`, `mm/{sv39,sv48,pmp}.rs`,
  `inst/atomic.rs`, `inst/base.rs`, `csr.rs`,
  `trap/handler.rs`) gives executor an unambiguous migration
  anchor for the ~60 mechanical edits of TR-10.
- **Response Matrix is complete and traceable.** Every prior-round
  R-001..R-025, every R-026..R-030 from round 04, all four MASTER
  directives (00-M-001/002, 01-M-001/002/003/004), and the user
  Concurrency directive are enumerated with a section pointer or
  inline resolution. CC-4 explicitly listed as Unresolved with
  scope rationale ("outside round-05 scope, single-hart limitation
  carried"). No silent drift.
- **PR boundary integrity preserved.** PR1 stays at `num_harts = 1`
  byte-identical (I-4); PR2a stays at `num_harts = 1`
  byte-identical with PLIC reshape (V-IT-6 14-test regression
  block); PR2b is the only PR that activates `num_harts > 1` and
  pins difftest to N=1 via the driver assert (NG-3 / CC-6). Each
  PR independently passes the 6-gate matrix.
- **R-020 hook placement verified.** `arch/riscv/cpu/mm.rs:271`
  confirmed as the single chokepoint for all stores: `Hart::store`
  / `Hart::amo_store` (now `RVCore::store` / `RVCore::amo_store`
  at lines 306, 323) both funnel through `checked_write`, and
  `checked_write` calls `access_bus` to resolve `pa` (line 272).
  The R-020 hook (`if matches!(op, MemOp::Store | MemOp::Amo) {
  self.last_store = Some((pa, size)); }`) lands cleanly *after*
  `bus.write` succeeds, with `pa` in scope. The op-gate defends
  against future Fetch / Load callers as in earlier rounds.
- **Plan length within ceiling.** 719 lines vs C-7 720; one line
  of headroom. Tight but not violated.



---

## Approval Conditions

### Must Fix
(none)

### Should Improve
- **R-031** (LOW) — Plan body at 719/720; document C-7 strategy in
  PR1 description rather than amend inline.
- **R-032** (LOW) — Step-4 method count "8" is ambiguous w.r.t.
  `access_bus`; one-sentence clarifier prevents executor
  over-threading.
- **R-033** (LOW) — V-UT-15 single-hart fairness is degenerate;
  add a one-line comment naming it as a modulo smoke test, or drop
  and adjust gate-matrix arithmetic.

### Trade-off Responses Required
(none — TR-9 and TR-10 from round 04 both addressed in this plan)

### Ready for Implementation
- Yes
- Reason: All round-04 blocking findings (R-026 HIGH, R-027
  MEDIUM) are resolved with ground-truth-anchored fixes verified
  against `xemu/xcore/tests/arch_isolation.rs:31-65` and
  `xemu/xcore/src/arch/riscv/cpu/mm.rs:271`. R-028 (ordering),
  R-029 (line budget), R-030 (V-IT-3 demotion), TR-9 (CoreBuilder
  alias), and TR-10 (test-fixture snippet) all absorbed. The
  user-ordered Concurrency Matrix lands as a 10-row top-level
  section bound to three new invariants and one new constraint.
  Carry-forward integrity verified for R-001..R-025, all four
  MASTER directives, and prior trade-offs. Test arithmetic
  internally consistent (372 / 373 / 376). Plan body at 719 lines
  within the 720 ceiling. Three remaining LOW findings (R-031,
  R-032, R-033) are post-merge editorials, not blockers.
  Implementation may begin on PR1.
