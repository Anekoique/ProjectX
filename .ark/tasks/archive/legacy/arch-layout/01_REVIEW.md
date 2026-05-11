# `archLayout` REVIEW `01`

> Status: Open
> Feature: `archLayout`
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

- Decision: Approved with Revisions
- Blocking Issues: 1
- Non-Blocking Issues: 4



## Summary

Round-01 is a targeted revision addressing round-00's three HIGH findings plus
the nine lower-severity items. Direction A (nest under `cpu/`) is preserved;
the `inst/` → `executor/` rename is dropped per reviewer TR-2; the plan body
is recompressed to 394 lines (target ≤ 400, R-010 satisfied); R-005/R-006
sanity notes and `include_str!` hop wording (R-008) are resolved inline. The
Response Matrix covers every R-001..R-010 and TR-1..TR-3, no new MASTER is
claimed, and inherited archModule directives are faithfully restated.

**(a) Round-00 HIGH resolution.** R-002 and R-003 are adequately resolved.
R-001 is partially resolved — the plan publishes a 15-row absolute-path
rewrite table, but an independent audit finds **five additional sites** the
table misses (see R-011). All five are absolute `crate::{arch::riscv::…}`
imports inside files that will move under the nest; each fails to compile
unless rewritten in the Phase-1+2 commit. Because the plan explicitly merges
Phase-1 and Phase-2 into one commit (resolving its own sequencing concern)
the miss does not break the "every phase green" contract *as a sequencing
matter* — but it breaks the literal "complete `rg`-audited table" claim and
forces the implementor to re-run the audit during Phase-1 cargo-build-fail.
For a plan gate this is borderline; classifying HIGH (not CRITICAL) because
(a) the failure surfaces at `cargo build` and is mechanical to fix, (b) the
plan's Failure Flow step 1 explicitly anticipates "missing path in Phase-2
table" and prescribes folding fixes into the same commit.

**(b) Inherited archModule MASTER directives.** 00-M-001 (no global `trait
Arch`), 00-M-002 (topic organisation preserved — CPU-concern nesting is
still topic-organised), 01-M-001 (no `selected` alias — seam keeps direct
`pub type`/`pub use`), 01-M-002 (clean/concise/elegant — rename dropped,
body at 394 lines), 01-M-003 (`build.rs` authoritative — no new cfg), and
01-M-004 CRITICAL (thin seams — narrower `arch::riscv::cpu::*` subtree) are
all faithfully applied. 01-M-004 is legitimately strengthened by nesting
because seam files now re-export from a single subtree root
(`arch::riscv::cpu`) instead of five flat topic roots.

**(c) Implementability with zero test/boot regressions.** With R-011 fixed,
the plan is implementable. Phase merging (1+2 in one commit) is the right
call — a bare nest without path rewrites cannot compile, so the round-00
"five independent green commits" framing was always aspirational for
Phases 1–2. The plan correctly reduces to one Phase-1+2 PR plus four
smaller follow-ups. `arch_isolation.rs` is text-level and uses substring
`contains("crate::arch::riscv::")` plus a symbol allow-list — both are
invariant under the nest (confirmed by reading `xcore/tests/arch_isolation.rs`).
The `include_str!` hop-count arithmetic in C-5 is correct: from
`arch/riscv/cpu/isa/decoder.rs` to `src/isa/instpat/riscv.instpat` is four
`../` segments. Boot markers (`Welcome to Buildroot`, `debian login:`) are
concrete and grep-testable.

**(d) Should the user skip 01_MASTER?** Yes — recommend skipping. The only
blocker (R-011) is a pure enumeration gap, not a design call. The three
trade-offs (TR-1..TR-3) are closed by reviewer agreement in round-00 and
the plan reflects each cleanly. No strategic override is required. Round-02
can converge with a single mechanical table extension plus a re-run of the
`rg arch::riscv::(csr|mm|trap|inst|isa)` audit command.

One blocking HIGH (R-011). R-012..R-015 are non-blocking and should be
folded into the next revision if cheap, otherwise waived.

---

## Findings

### R-011 Absolute-path rewrite table still misses 5 sites

- Severity: HIGH
- Section: Implementation Plan → Phase 2 (rewrite table)
- Type: Correctness / Flow
- Problem:
  Phase 2's 15-row table claims to be the complete `rg`-audited list of
  absolute-path rewrites. The audit command given in the plan
  (`rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)' xemu/xcore/src` +
  `rg 'use crate::arch::riscv::\{' …`) misses a third pattern: absolute
  `crate::arch::riscv::…` paths that appear as sub-paths inside
  `use crate::{ … };` multi-import blocks where `crate::arch::…` is a
  nested element. A broader audit
  (`rg 'arch::riscv::(csr|mm|trap|inst|isa)' xemu/xcore/src`) reveals
  **five** such sites not in the plan's table:

  1. `arch/riscv/inst/privileged.rs:6` —
     `use crate::{ arch::riscv::csr::{CsrAddr, Exception, MStatus, PrivilegeMode}, … };`
     must become `arch::riscv::cpu::csr::{…}`.
  2. `arch/riscv/inst/float.rs:15` —
     `use crate::{ arch::riscv::csr::CsrAddr, … };` must become
     `arch::riscv::cpu::csr::CsrAddr`.
  3. `arch/riscv/inst/compressed.rs:361` (test module) —
     `use crate::{ arch::riscv::trap::{TrapCause, test_helpers::assert_trap}, … };`
     must become `arch::riscv::cpu::trap::{…}`.
  4. `arch/riscv/inst/atomic.rs:171` (test module) —
     `use crate::{ arch::riscv::trap::{Exception, TrapCause, test_helpers::assert_trap}, … };`
     must become `arch::riscv::cpu::trap::{…}`. (The table's row 11
     captures `atomic.rs:9` but not `:171`.)
  5. `arch/riscv/mm/pmp.rs:6` —
     `use crate::{ arch::riscv::csr::PrivilegeMode, … };` must become
     `arch::riscv::cpu::csr::PrivilegeMode`.

  All five are absolute paths (rooted at `crate::`), not relative
  `super::`-chains, so they will compile-fail in the Phase-1+2 commit
  unless rewritten. The plan's Phase-2 final gate command
  (`rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)(::|\{)' xemu/xcore/src`)
  would catch these at verification time but not at authoring time.
- Why it matters:
  R-001's round-00 recommendation was to "enumerate every breaking
  absolute-path import site per phase." The plan responded by enumerating
  15 sites but ran only one of two necessary `rg` patterns, so the table is
  not complete. An implementor following the plan verbatim will hit 5
  compile errors and must re-audit mid-Phase-1. The plan's Failure Flow
  step 1 ("missing path in Phase-2 table: patch in the same commit")
  explicitly anticipates this, so the practical fallout is small — but the
  "complete table" claim is literally false, and the stated gate command
  in Phase 2 (`rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)::'
  xemu/xcore/src --glob '!arch/**'` returns 0 hits outside seam files)
  misses these sites entirely because they live inside `arch/**` and are
  excluded by the glob. The second gate (`rg … xemu/xcore/src` without
  `!arch/**`) catches them, so the plan's verification is sound — only the
  **authoring table** is short.
- Recommendation:
  Extend the Phase-2 table with the 5 missing rows. Update the audit
  command in the plan prose to the broader pattern
  `rg 'arch::riscv::(csr|mm|trap|inst|isa)' xemu/xcore/src` (drop the
  leading `crate::` anchor; the pattern still excludes `use super::…`
  relative paths because `super::` does not contain `arch::`). One sentence
  confirming the gate command already covers these sites is sufficient.



### R-012 Phase 3 visibility edit is one site, not three — Response Matrix wording overstates

- Severity: LOW
- Section: Response Matrix (row R-002) / Phase 3
- Type: Maintainability / Accuracy
- Problem:
  The Response Matrix row for R-002 says "Policy A: widen the 3 `pub(in
  crate::arch::riscv::mm)` occurrences at `mm/tlb.rs:10`, `mm/tlb.rs:58`,
  `mm/mmu.rs:20` to `pub(in crate::arch::riscv)`". In reality only
  `tlb.rs:10` uses `pub(in crate::arch::riscv::mm)`; `tlb.rs:58` and
  `mmu.rs:20` already use `pub(in crate::arch::riscv)` (path-stable, no
  edit needed). The plan's Data Structure and Phase 3 bodies do say this
  correctly ("Sister sites `tlb.rs:58` and `mmu.rs:20` already use
  `pub(in crate::arch::riscv)`; no edit (listed for completeness per
  R-002)."), so the substantive plan is fine. The Response Matrix wording
  is just misleading.
- Why it matters:
  Low. A reviewer or future Executor reading only the Response Matrix
  might conclude three sites change when only one does. This is a
  stylistic cleanup, not a correctness issue.
- Recommendation:
  Reword the R-002 row to: "Policy A: widen the one true `pub(in
  crate::arch::riscv::mm)` occurrence at `mm/tlb.rs:10` to `pub(in
  crate::arch::riscv)`; `mm/tlb.rs:58` and `mm/mmu.rs:20` already use the
  wider scope (verified, no edit)."



### R-013 `arch_isolation` is strictly invariant — Phase 4 could downgrade to a gate check

- Severity: LOW
- Section: Phase 4 / Validation
- Type: Validation / Maintainability
- Problem:
  Phase 4 ("`arch_isolation.rs` confirm") is described as a separate PR
  with its own gate, but the work it performs is: "re-run the test; confirm
  no violations; add a one-line doc-comment pin noting post-nest state. No-op
  on the allow-lists." After verifying `xcore/tests/arch_isolation.rs`:
  the substring check at line 212 (`raw.contains("crate::arch::riscv::")`)
  and the symbol allow-list at line 42 are genuinely invariant under the
  nest — the substring `crate::arch::riscv::` is still present in post-nest
  paths (just with `cpu::` appended), and no allow-listed symbol name
  changes. So Phase 4 is one doc-comment line plus a `cargo test` pass
  that was already gated by Phase-1+2 (V-UT-1 runs the full workspace). A
  separate PR for this is over-engineering.
- Why it matters:
  Low. Splitting work into more PRs than the changes warrant adds review
  overhead without adding safety. The doc-comment line could fold into
  Phase 5's "docs + full boot verification" PR trivially.
- Recommendation:
  Collapse Phase 4 into Phase 5, or state explicitly in Phase 4 that it is
  a 1-line doc-comment edit landing as its own trivial PR purely for
  historical traceability. Either is acceptable; the plan should not imply
  the confirmation step involves non-trivial work.



### R-014 Phase-2 gate command excludes `arch/**`, silently failing to catch 10 of the 15 sites

- Severity: MEDIUM
- Section: Phase 2 gate (lines 258–262)
- Type: Validation
- Problem:
  The Phase-2 gate command as written:

  ```
  rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)::' xemu/xcore/src
     --glob '!arch/**' returns 0 hits outside seam files;
  ```

  excludes `arch/**`. But 10 of the 15 rows in the Phase-2 table are at
  paths under `arch/**` (rows 5–15). The first gate command only verifies
  the seam files (rows 1–4). The second gate command
  (`rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)(::|\{)' xemu/xcore/src`
  without glob) does catch everything — but the plan frames both as
  gates. If an implementor reads the first command as the primary gate,
  they may miss 10 unrewritten sites.

  (Aside: when the 5 additional sites from R-011 are added, the second
  gate's regex `(::|\{)` still catches them because they end in `::` after
  the topic name. Good — the gate is sound as stated, just needs to be
  emphasised as the primary one.)
- Why it matters:
  Medium. The gate commands are the post-condition proof that Phase 2 is
  complete. A reader who treats the first command as primary will declare
  victory on an incomplete rewrite. Risk magnified by R-011's missing rows.
- Recommendation:
  Mark the broader `rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)(::|\{)'
  xemu/xcore/src` (no `!arch/**` glob) as the **primary** gate, and
  demote the `!arch/**` variant to a secondary "seam-only" sanity check.
  One sentence swap.



### R-015 `include_str!` path string specified in C-5 but not in the rewrite table

- Severity: LOW
- Section: Phase 1 / Phase 2 table / Constraints C-5
- Type: Correctness / Traceability
- Problem:
  The exact post-nest `include_str!` string
  (`"../../../../isa/instpat/riscv.instpat"`) is given verbatim in C-5
  (line 191) and in Phase 1 prose (line 221: "`include_str!("../../../isa/…")`
  → `include_str!("../../../../isa/…")`") and in Failure Flow step 4
  (line 302). But the Phase-2 absolute-path rewrite table does **not**
  include it as a numbered row. That makes the Phase-2 gate command
  genuinely complete for `crate::arch::riscv::` imports but silent about
  the `include_str!` path, which is arguably a different kind of edit
  (relative filesystem path, not Rust `use`-path). An implementor executing
  "Phase 2" as just "apply the table" would miss the `include_str!` edit.
- Why it matters:
  Low — Phase 1 prose covers the edit explicitly, and
  `arch/riscv/cpu/isa/decoder.rs` will panic at crate init if the path is
  wrong (not at compile time — `include_str!` is a literal file read at
  macro-expansion time, so a missing file is a hard compile error). So
  the failure surfaces at `cargo build`, matching Failure Flow step 4's
  explicit guard.
- Recommendation:
  Either (a) add a 16th row to the Phase-2 table for the `include_str!`
  path edit, or (b) state in the Phase-1 text that the Phase-2 table is
  scoped to Rust `use`-paths and the `include_str!` edit is a Phase-1
  prose item. Current plan implicitly does (b); making it explicit
  prevents confusion.



---

## Trade-off Advice

### TR-4 Phase 4 standalone PR vs fold into Phase 5

- Related Plan Item: Implementation Plan PRs
- Topic: PR granularity vs Review overhead
- Reviewer Position: Prefer merging into Phase 5
- Advice:
  Fold Phase 4 (`arch_isolation` re-confirm) into Phase 5 (docs + boot).
  Phase 4 is a 1-line doc-comment pin; a separate PR adds review cycles
  without adding safety.
- Rationale:
  The `arch_isolation.rs` test is strictly invariant under the nest (as
  confirmed by independent reading of `xcore/tests/arch_isolation.rs`
  lines 31–75 and 208–230). Splitting a no-op confirmation into its own
  PR dilutes the signal of the phase-gate structure. 01-M-002
  (clean/concise/elegant) applies: fewer PRs, same safety.
- Required Action:
  Either merge Phase 4 into Phase 5 and note in the Execution Flow that
  the confirmation step runs as part of Phase 5's gate, or keep the
  split and add a one-sentence rationale for why a standalone PR is
  worth the overhead (e.g. "historical traceability of the invariant
  pin"). Both are acceptable.



---

## Positive Notes

- TR-2 closure (drop the rename) is cleanly applied: zero `executor/`
  references appear in the plan body except where the rejected rename is
  explicitly cited (NG-2, Removed section, TR-2 closed row). Response
  Matrix, Architecture diagram, and Phases are all consistent. Good
  follow-through.
- Phase 1+2 merger is the right call. Acknowledging that a bare nest
  cannot compile without path rewrites — and folding the two into one
  commit with a clearly-labelled "Phase-Ordering" note (lines 233–235) —
  turns the round-00 "five independently green phases" aspiration into a
  realistic "one combined PR + four follow-ups" structure. This matches
  `cargo build`'s actual constraints.
- `include_str!` hop-count arithmetic in C-5 (line 188–191) is correct
  and explicit. Four `../` segments from `arch/riscv/cpu/isa/` to `src/`
  is verified by counting the segments (`isa/` → `cpu/` → `riscv/` →
  `arch/` → `src/`).
- Body compressed from 564 → 394 lines while retaining all technical
  content. R-010 fully satisfied.
- `arch_isolation.rs` invariance under the nest (I-3) is correctly
  identified — the test's substring check and symbol allow-list are both
  robust to the path lengthening because the substring `crate::arch::riscv::`
  is preserved and no symbol names change.
- `pub(in crate::arch::riscv)` carry-over is correctly handled: the 8
  sites widened in archModule PR-2 require zero edits (module-path scope
  stays valid since `arch::riscv` is still the ancestor of the nested
  topics). The plan does not redundantly rewrite them.
- Boot markers (`Welcome to Buildroot`, `debian login:`) with concrete
  timeouts (60s / 120s) replace round-00's vague "byte-for-byte equivalent"
  — R-009 properly closed.



---

## Approval Conditions

### Must Fix
- R-011 — extend the Phase-2 rewrite table with the 5 missing sites at
  `inst/{privileged:6, float:15, compressed:361, atomic:171}`,
  `mm/pmp.rs:6`; update the plan's audit command to the broader
  `arch::riscv::(csr|mm|trap|inst|isa)` pattern.

### Should Improve
- R-012 — reword Response Matrix row for R-002 to say 1 edit, 2 sister
  sites confirmed invariant.
- R-014 — mark the no-glob `rg` command as the primary Phase-2 gate;
  demote the `!arch/**` variant to a secondary seam-only check.
- R-015 — explicitly scope the Phase-2 table to Rust `use`-paths and
  cross-reference the Phase-1 `include_str!` edit.

### Trade-off Responses Required
- TR-4 — either fold Phase 4 into Phase 5 (recommended), or add a
  one-sentence rationale for the standalone PR.

### Ready for Implementation
- No
- Reason: One blocking HIGH (R-011) — the Phase-2 rewrite table is
  incomplete by 5 sites. The miss is mechanical and recoverable via the
  Failure Flow, but the plan's literal "complete `rg`-audited table"
  claim is false as written. Non-blocking R-012/R-014/R-015/TR-4 are
  quality improvements; R-013 is informational. Once R-011 is addressed,
  the plan converges — recommend the user skip `01_MASTER.md` and
  dispatch `plan-executor` for round 02.
