# `archLayout` REVIEW `04`

> Status: Closed
> Feature: `archLayout`
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

- Decision: Approved
- Blocking Issues: `0`
- Non-Blocking Issues: `1`



## Summary

Round 04 is a clean convergence pass. R-019 HIGH (the E0433 trap at
`mm/tlb.rs:10` that carried through rounds 00–03 as a deferred widen) is
fully resolved by folding the visibility-widen into Phase 1 as a named
23rd action with explicit before/after strings (`04_PLAN.md:138-144`).
R-020 LOW is resolved by option (a) from round 03's recommendation — the
V-F-2 SECONDARY gate is annotated as a "tautological tripwire — 0 hits
structural, not a green-bar contributor" (`04_PLAN.md:262-268`), matching
the concrete wording the round-03 review asked for.

The plan is now a two-phase, three-PR structure with zero orphaned
actions. Phase 1 atomically lands `git mv` + 22-row absolute-path
rewrite + `include_str!` hop + relative-path fixups + `tlb.rs:10` widen,
gated on `make fmt && make clippy && make test && make run` with the
PRIMARY multiline-dotall regex returning 0 hits. Phase 2 lands docs +
`arch_isolation` pin + full boot (`make linux`, `make debian`, difftest).
Body length is 300 lines exactly, matching the tightened C-7 budget.

Answers to the convergence questions:

- (a) R-019 RESOLVED — folded into Phase 1 action row 23 with concrete
  file:line and before/after visibility strings. Phase 3 deleted,
  Phase 4 renumbered to Phase 2, State Transition updated to S0→S1→S2.
  Independent audit against the current source tree confirms
  `mm/tlb.rs:10` is the sole `pub(in crate::arch::riscv::<topic>)`
  narrow-path site in `xemu/xcore/src`; all other 27 `pub(in …)` sites
  target the path-stable ancestor `crate::arch::riscv` and need no edit.
  R-020 RESOLVED — option (a) selected with concrete annotation text.
- (b) Inherited archModule MASTER directives (00-M-001..01-M-004)
  faithfully applied. No `trait Arch`; topic nesting preserved under
  `cpu/`; direct `pub type` / `pub use` seams via rows #1-#4; two phases
  ≤ 300 lines; `build.rs` authoritative; seam surface narrowed to
  `arch::riscv::cpu::*`.
- (c) Implementable with zero test/boot regressions. The single atomic
  Phase-1 PR removes every class of intermediate compile-break the
  prior rounds risked. `cargo check` at end of Phase 1 is backed by a
  named PRIMARY regex gate and a SECONDARY defensive tripwire. 344-test
  green bar, `make linux`, `make debian`, and difftest corpus all named
  as per-phase gates.
- (d) User can skip `04_MASTER.md` and proceed to implementation.
  Residual concern is a single LOW count-off-by-one in the audit
  narrative (R-021 below) that does not affect the executable action
  list.



---

## Findings

### R-021 `Audit narrative under-counts pub(in …) sites by one`

- Severity: LOW
- Section: `Log / Audit this round`
- Type: Correctness (documentation)
- Problem:
  `04_PLAN.md:32-34` states:

  ```
  Audit this round: `rg 'pub\(in crate::arch::riscv' xemu/xcore/src` →
  27 hits, 26 wide (path-stable) and 1 narrow (`mm/tlb.rs:10`).
  ```

  Running that exact command against the current pre-nest tree returns
  28 hits, not 27:

  - `cpu/mod.rs`: 17 wide
  - `trap.rs`: 5 wide
  - `csr.rs`: 1 wide
  - `csr/ops.rs`: 2 wide
  - `mm/tlb.rs`: 2 (line 10 narrow, line 58 wide)
  - `mm/mmu.rs`: 1 wide

  Total = 28; wide = 27; narrow = 1. The plan's narrative is off by one
  on the total and wide count. The material claim — "tlb.rs:10 is the
  sole narrow-path site" — is correct and the executable 23rd action
  row (`04_PLAN.md:138-144`) is unaffected.

- Why it matters:
  Purely a documentation accuracy issue. The plan's `pub(in …)` site
  enumeration at `04_PLAN.md:142-144` lists `cpu/mod.rs:27-44` (18
  lines, but 17 actual `pub(in …)` declarations — the range 27-44
  contains one non-`pub(in …)` line at line 40 which is a doc comment),
  `csr.rs:95` (1), `csr/ops.rs:7,16` (2), `trap.rs:21-55` (5),
  `mm/tlb.rs:58` (1), `mm/mmu.rs:20` (1) → 17 + 1 + 2 + 5 + 1 + 1 = 27
  wide sites, consistent with the actual source. So the inline
  enumeration is correct; only the summary "27 hits, 26 wide" in the
  Log prose is wrong. Self-consistent within the action list, just not
  within the narrative.

- Recommendation:
  Non-blocking. If the plan is revised for any reason, update
  `04_PLAN.md:32-34` to "28 hits, 27 wide (path-stable) and 1 narrow
  (`mm/tlb.rs:10`)". Otherwise leave as-is; the action list is the
  authoritative artifact and it is correct.



---

## Trade-off Advice

No new trade-offs this round. Prior TR-1..TR-4 remain CLOSED in the
Response Matrix (`04_PLAN.md:84`). R-019's fold was a correctness
decision, not a trade-off; the plan correctly records it as such at
`04_PLAN.md:234-235`.



---

## Positive Notes

- R-019 resolution is mechanically tight. Phase 1 action row 23 at
  `04_PLAN.md:138-144` names the exact file:line (`cpu/mm/tlb.rs:10`),
  the before string (`pub(in crate::arch::riscv::mm) struct TlbEntry`),
  and the after string (`pub(in crate::arch::riscv) struct TlbEntry`),
  with an inline rationale citing Policy A. The four-round deferred
  widen is finally atomic with the nest.
- R-020 annotation is concrete and matches the round-03 review's
  option (a) wording verbatim intent. `04_PLAN.md:262-268`: "tautological
  tripwire only, not a green-bar contributor … 0 hits is structural.
  Retained for historical parity; PRIMARY carries actual signal."
  Explicitly demotes SECONDARY below PRIMARY in the green-bar contract.
- Phase collapse is complete. Former Phase 3 deleted, former Phase 4
  renumbered to Phase 2; no orphaned actions. State Transition
  `S0 → S1 → S2` at `04_PLAN.md:215-218` matches the two-phase structure
  and enumerates every atomic action in S1.
- Three-PR decomposition (`04_PLAN.md:222-230`) is correctly internally
  consistent with the two-phase structure: PR1 = Phase 1 (atomic), PR2
  = Phase 2 docs+isolation, PR3 = Phase 2 boot. Each PR has a named
  green-bar command. The PR split inside Phase 2 is justified implicitly
  by the isolation pin vs. full-boot distinction.
- PRIMARY regex is carried forward unchanged from round 03:
  `rg -U --multiline-dotall
  '\barch::riscv::(\s*\{[^}]*)?\b(csr|mm|trap|inst|isa)\b' xemu/xcore/src`
  at `04_PLAN.md:256-261`. Correctness, multiline-dotall behavior, and
  false-positive analysis all preserved. Independent verification on
  the current tree shows 28 hits pre-Phase-1 (22 unique `use`-path sites
  + `pub(in …::mm)` at `mm/tlb.rs:10` + 5 other `pub(in …)` matches
  that happen to contain the adjacent topic token — all legitimate
  matches; none false positives).
- Body length discipline: 300 lines exactly at the C-7 budget. Spec
  inherited-by-reference from `01_PLAN.md` lines 84-194 via `02/03_PLAN.md`
  with the only declared delta being C-7 tightening from ≤ 320 to
  ≤ 300. No load-bearing content cut.
- Seam file snippets for `cpu/mod.rs:45`, `isa/mod.rs:10`,
  `device/mod.rs:58+61` preserved via rewrite-table rows #1-#4; line
  numbers verified against source (`cpu/mod.rs:11-15` use-block visible,
  `isa/mod.rs:7-9` re-exports visible, `device/mod.rs:61` doc-comment
  referencing `crate::arch::riscv::trap::interrupt` visible).
  `device/intc/mod.rs:10` correctly omitted from the rewrite table —
  it references `crate::arch::riscv::device::intc` (topic `device`,
  which is a sibling of `cpu/`, not under it), so no rewrite is needed.
- Response Matrix completeness: R-001 through R-020 all present with
  CLOSED/Accepted decisions and one-line resolutions (`04_PLAN.md:72-85`).
  Row count matches the cumulative review finding count through round
  03. TR-1..TR-4 and MASTER directives also represented.
- Failure Flow retains the `make debian` 180s timeout escape (step 6,
  `04_PLAN.md:210-211`), preserving the hard `debian login:` grep
  constraint while allowing difftest slack.



---

## Approval Conditions

### Must Fix

- None

### Should Improve

- R-021 (narrative-only count correction, non-blocking)

### Trade-off Responses Required

- None (all prior TRs CLOSED)

### Ready for Implementation

- Yes
- Reason: R-019 HIGH and R-020 LOW — the only two carry-over findings
  from round 03 — are both resolved with concrete, evidence-verified
  fixes in `04_PLAN.md`. No CRITICAL or HIGH remains. The single LOW
  finding (R-021) is a documentation-narrative count-off-by-one that
  does not affect any executable action. Inherited archModule MASTER
  directives are faithfully applied. Spec is inherited verbatim with
  only the C-7 body-length tightening declared. The two-phase, three-PR
  structure is internally consistent and each phase has named green-bar
  commands. `make linux`, `make debian`, difftest corpus, and 344-test
  pass are all mapped to concrete gates. User may skip `04_MASTER.md`
  and dispatch the implementation executor against `04_PLAN.md`.
