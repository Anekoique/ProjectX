# `archLayout` REVIEW `03`

> Status: Open
> Feature: `archLayout`
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
- Blocking Issues: `1`
- Non-Blocking Issues: `1`



## Summary

Round 03 is a tight, well-targeted convergence pass. R-016 (HIGH) and R-018
(LOW) are fully resolved; R-017 (LOW) is resolved with one residual sub-bug
covered by R-020 below. The plan body is 320 lines (matches C-7 budget
exactly, down from 341 in round 02). Response Matrix covers R-001..R-018
with explicit decisions; inherited archModule MASTER directives
(00-M-001..01-M-004) are faithfully applied; no new MASTER this round.

Independent audit verification:

- PRIMARY regex `rg -U --multiline-dotall
  '\barch::riscv::(\s*\{[^}]*)?\b(csr|mm|trap|inst|isa)\b' xemu/xcore/src`
  executed against the current pre-nest tree. It flags every unique
  rewrite-table site including the round-02 blind-spots
  (`arch/riscv/trap/handler.rs:5-12` top-of-file nested-brace and
  `arch/riscv/inst/float.rs:659-666` test-module nested-brace). 22 unique
  `use`-path sites found + one `pub(in crate::arch::riscv::mm)` site at
  `mm/tlb.rs:10` that is a legitimate match but handled by Phase 3 rather
  than the rewrite table. The table count matches exactly.
- Regex correctness spot-check: post-nest form `arch::riscv::cpu::csr::…`
  does NOT match (`cpu` sits between `arch::riscv::` and any topic arm, so
  `\b(csr|mm|trap|inst|isa)\b` adjacency fails). No false positives
  observed on the current tree.
- R-018 assertion `cargo test --test arch_isolation -- --exact
  arch_isolation` is executable as-written (verified: integration test at
  `xcore/tests/arch_isolation.rs`, function `arch_isolation` at line 172).

Answers to the convergence questions:

- (a) R-016/R-017/R-018 — R-016 RESOLVED (table now 22 rows; PRIMARY gate
  catches all three import shapes). R-018 RESOLVED (exact pass/fail
  assertion named). R-017 PARTIALLY RESOLVED (expected-result wording
  fixed; SECONDARY glob list is slightly over-restrictive, see R-020 LOW).
- (b) Inherited archModule MASTER — faithfully applied. No `trait Arch`;
  topic organisation preserved; direct `pub type`/`pub use` seams; four
  phases ≤ 320 lines; `build.rs` authoritative; seam surface narrowed to
  `arch::riscv::cpu::*`.
- (c) Implementable with zero test/boot regressions — almost. R-019 below
  is a recurring, evidence-backed compile-break in Phase 1+2 that the
  plan has deferred to Phase 3 for three rounds running. It is mechanical
  to fix (fold the `tlb.rs:10` `pub(in …::mm)` edit into Phase 1+2 or
  collapse Phase 3 into Phase 1+2). Without the fix, PR1 will hit rustc
  E0433 on the `pub(in crate::arch::riscv::mm)` scope path — reproduced
  against a minimal crate during this review.
- (d) User cannot skip 03_MASTER cleanly: R-019 is a concrete HIGH
  blocker. The delta is one line (add `tlb.rs:10` widen to Phase 1+2
  action list OR restate Phase 3 as a no-op post-PR1). Round 04 is
  realistic and should be the final iteration.



---

## Findings

### R-019 `Phase 1+2 will fail E0433 on mm/tlb.rs:10's pub(in ::mm) after git mv`

- Severity: HIGH
- Section: `Implement / Execution Flow / Phase 1+2 and Phase 3`
- Type: Correctness
- Problem:
  `03_PLAN.md:192-195` places the `tlb.rs:10` visibility edit in Phase 3
  (a separate PR after Phase 1+2 lands):

  ```
  2. Phase 3 — Visibility widen (Policy A, one edit).
     arch/riscv/cpu/mm/tlb.rs:10: pub(in crate::arch::riscv::mm) struct
     TlbEntry → pub(in crate::arch::riscv) struct TlbEntry.
  ```

  The starting-form `pub(in crate::arch::riscv::mm)` shown there is the
  PRE-nest form. Phase 1+2 does not touch this line (see action list at
  `03_PLAN.md:113-189` — `git mv`, module decls, `include_str!`,
  relative-path fixups, 22-row absolute `use` table; no `tlb.rs:10` row).
  After Phase 1+2 lands, the file lives at
  `xemu/xcore/src/arch/riscv/cpu/mm/tlb.rs` but line 10 still textually
  says `pub(in crate::arch::riscv::mm)`. That path no longer exists —
  post-nest, `mm` is at `crate::arch::riscv::cpu::mm`. `pub(in …)`
  requires the path to name an ancestor module, and Rust raises E0433
  "failed to resolve: unresolved import" when it does not.

  Reproduction (minimal crate written during this review):

  ```
  // src/lib.rs           pub mod cpu;
  // src/cpu/mod.rs       pub mod mm;
  // src/cpu/mm/mod.rs    pub mod tlb;
  // src/cpu/mm/tlb.rs    pub(in crate::mm) struct TlbEntry {}
  // cargo build          → error[E0433]: failed to resolve: unresolved import
  //                        help: a similar path exists: `cpu::mm`
  ```

  The identical failure mode applies to Phase 1+2 of this plan. This
  concern was raised in round 00 R-002 and was "accepted" by widening to
  `pub(in crate::arch::riscv)` — but the widen was placed in Phase 3, a
  separate PR. Rounds 01, 02, and 03 all keep Phase 3 as a standalone PR
  (`02_PLAN.md:260`, `03_PLAN.md:247`: "PR2 (Phase 3) — one-line edit to
  `cpu/mm/tlb.rs:10`. Gate: `cargo test --workspace`"). For PR2's gate to
  be meaningful, PR1 must already compile — which it will not.

  The plan's own V-UT-1 invariant ("344 tests green at every phase
  boundary") and C-1 ("each phase leaves `cargo build` / `cargo test
  --workspace` clean") are therefore violated by the plan's structure.
  Failure Flow step 1 ("missed path site: patch in same commit") could be
  stretched to cover this, but the step names "path site" (use-path), not
  a `pub(in …)` visibility site. A plan-faithful executor following the
  22-row table verbatim does not edit `tlb.rs:10` in PR1.

- Why it matters:
  Round 00 explicitly flagged this ("Unless rewritten, the restriction
  references a non-existent module and fails to compile"). It has carried
  through three rounds under the label "accepted / Policy A / widen in
  Phase 3". The mechanical fix is trivial — one line — but the plan must
  say so explicitly, otherwise PR1 fails `cargo build` and the executor
  has to improvise a deviation mid-PR. Given this is round 03 of 5 with
  one round remaining, failing to surface this now means the executor
  will either (i) silently merge Phase 3 into Phase 1+2 without a plan
  amendment (deviation from approved PLAN), or (ii) land PR1 red and
  retro-fit `tlb.rs:10`. Both outcomes bypass the iteration discipline
  the feature has followed for three rounds.

- Recommendation:
  In round 04, pick exactly one of:

  1. (Preferred — simplest delta) Fold Phase 3 into Phase 1+2. Add
     `tlb.rs:10` as a 23rd row in the Phase-1+2 action list:
     `pub(in crate::arch::riscv::mm) struct TlbEntry` →
     `pub(in crate::arch::riscv) struct TlbEntry`. Remove Phase 3;
     renumber Phase 4 → Phase 3; update State Transition and PR list
     (three PRs: PR1 = Phase 1+2 incl. widen, PR2 = docs+isolation,
     PR3 = boot). Widen target matches Policy A's existing decision, so
     no Spec/Invariant drift.

  2. (Alternative — keep Phase 3 separate) Add a non-widening depth-
     preserve edit to `tlb.rs:10` in Phase 1+2:
     `pub(in crate::arch::riscv::mm)` → `pub(in crate::arch::riscv::cpu::mm)`.
     Phase 3 then widens from the nested form to
     `pub(in crate::arch::riscv)` for the one-PR visibility rationalisation.
     This preserves the four-PR structure but adds a row to Phase 1+2 and
     updates Phase 3's before-form.

  Either fix eliminates the E0433. Option (1) is strictly smaller and
  matches the spirit of 01-M-002 (elegance — one physical change, one
  PR).



### R-020 `SECONDARY gate glob list misses xcore/src/device/ sub-paths`

- Severity: LOW
- Section: `Validation / V-F-2 SECONDARY`
- Type: Validation
- Problem:
  `03_PLAN.md:284-287` defines the SECONDARY gate as:

  ```
  rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)::' xemu/xcore/src
      --glob '!arch/**' --glob '!cpu/mod.rs' --glob '!isa/mod.rs'
      --glob '!device/mod.rs' --glob '!device/intc/mod.rs'
  ```

  The glob `!device/intc/mod.rs` is path-relative and matches BOTH
  `xemu/xcore/src/device/intc/mod.rs` (intended — the seam) AND
  `xemu/xcore/src/arch/riscv/device/intc/mod.rs` (unintended — but
  already excluded by `!arch/**`, so no double-exclusion harm).
  However, the check SHOULD allow precisely the four seam files and
  report any stray hit anywhere else outside `arch/`. The current
  `xemu/xcore/src/device/intc/mod.rs` DOES contain
  `crate::arch::riscv::device::intc::{Aclint, Plic}` (verified
  independently) — that matches `crate::arch::riscv::` but the topic
  after is `device::intc::`, which is NOT in `(csr|mm|trap|inst|isa)`,
  so it correctly does not match the pattern. So the `!device/intc/mod.rs`
  exclude is redundant and the gate returns 0 hits either way.

  The real wording concern: post-nest, the PATTERN itself
  `crate::arch::riscv::(csr|mm|trap|inst|isa)::` cannot match anywhere
  (the topic arms now sit under `cpu::`, so every valid reference reads
  `crate::arch::riscv::cpu::<topic>::`). The pattern returns 0 hits on a
  fully-rewritten tree regardless of glob excludes, including inside the
  seam files. The glob list is therefore defensive but not load-bearing —
  it would only matter if the SECONDARY pattern were broader. The "0
  hits" expectation is trivially satisfied.

  Net effect: the gate is correct but tautological. It does not add
  genuine signal over PRIMARY.

- Why it matters:
  Low on its own (SECONDARY is explicitly non-authoritative). The
  plan's own wording acknowledges this by demoting it to "supplementary"
  in round 02 and keeping it as-is in round 03. If the gate is purely
  defensive, it is fine; if it is meant to catch a real class of regression
  that PRIMARY cannot, the pattern should be strengthened (e.g., drop the
  trailing `::` to catch the `tlb.rs:10`-style `pub(in …)` site, or drop
  the `crate::` prefix to align with PRIMARY).

- Recommendation:
  Either (a) keep the current form and annotate: "SECONDARY is a
  tautological tripwire — succeeds iff PRIMARY succeeds; retained for
  historical parity with archModule gating. Not a green-bar contributor."
  Or (b) strengthen to
  `rg '\bcrate::arch::riscv::(csr|mm|trap|inst|isa)\b' xemu/xcore/src
  --glob '!arch/**'` with expected result "exactly 4 hits inside the
  four named seam files" — matches actual post-nest state and provides
  orthogonal signal to PRIMARY. Either option is non-blocking; the
  current wording does not mislead.



---

## Trade-off Advice

No new trade-offs this round. Prior TR-1..TR-4 are correctly marked
CLOSED in the Response Matrix. The round-03 decision to retain the
four-phase split (vs fold Phase 3 into Phase 1+2) surfaces as R-019 and
is analysed there rather than as a trade-off; it is a correctness issue,
not a design choice.



---

## Positive Notes

- PRIMARY regex design is solid. The multiline-dotall form
  `\barch::riscv::(\s*\{[^}]*)?\b(csr|mm|trap|inst|isa)\b` correctly
  handles single-line absolute paths, wrapper multi-imports, and
  multi-line nested-brace imports in one pattern, without false positives
  on the correct post-nest `cpu::csr::…` form. Rationale at
  `03_PLAN.md:36-43` states the design intent precisely.
- Rewrite table grew from 20 to 22 rows with exactly the two round-02
  blind-spots named. Row #21 (`cpu/trap/handler.rs:5-12`) and row #22
  (`cpu/inst/float.rs:659-666`) cite the correct line ranges and use the
  post-nest path prefix `cpu/` consistent with rows #5-#20. Independent
  re-audit of the current pre-nest tree with the new regex confirms no
  further `use`-path sites are missing.
- R-018 assertion is executable as-written. `cargo test --test
  arch_isolation -- --exact arch_isolation` names the integration test
  binary and the exact test function — `xcore/tests/arch_isolation.rs`
  contains `#[test] fn arch_isolation()` at line 172. No ambiguity; the
  gate is now a concrete pass/fail.
- Response Matrix compresses rounds 00/01 findings into row groups
  (R-001..R-003, R-004..R-010, R-012/R-013/R-015) while keeping the
  round-02 items (R-016/R-017/R-018) and trade-offs itemised. This is a
  good balance between brevity and traceability.
- Body length discipline: 320 lines exactly at the tightened C-7
  budget (down from 341 in round 02). No load-bearing content cut —
  Spec is inherited-by-reference from `01_PLAN.md` with explicit line
  ranges cited (`03_PLAN.md:96-97`).
- Failure Flow retains the `make debian` timeout escape hatch
  (step 6: "extend to 180s, do not relax `debian login:` grep"),
  preserving the hard user constraint while allowing difftest slack.
- Seam-file before/after snippets for `cpu/mod.rs`, `isa/mod.rs`,
  `device/mod.rs`, `device/intc/mod.rs` are preserved via rows #1-#4 of
  the table. Spot-check against current source confirms line numbers
  (45, 10, 58, 61) and symbol lists are accurate.



---

## Approval Conditions

### Must Fix

- R-019

### Should Improve

- R-020

### Trade-off Responses Required

- None (all prior TRs CLOSED)

### Ready for Implementation

- No
- Reason: R-019 HIGH is a concrete, reproducible rustc E0433 compile
  break in PR1 (Phase 1+2). The `pub(in crate::arch::riscv::mm)` at
  `mm/tlb.rs:10` references a module path that ceases to exist after
  the nest, and the plan defers the edit to a separate Phase 3 PR. This
  is a recurring miss of round-00 R-002. Minimal round-04 fix: fold the
  one-line `tlb.rs:10` widen into the Phase 1+2 action list (preferred)
  or split into a depth-preserve edit in Phase 1+2 + final widen in
  Phase 3. Delta is ≤ 3 plan lines; convergence within round 04 is
  realistic and should be the final iteration before implementation.
