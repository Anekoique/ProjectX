# `archLayout` REVIEW `02`

> Status: Open
> Feature: `archLayout`
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

- Decision: Approved with Revisions
- Blocking Issues: `1`
- Non-Blocking Issues: `2`



## Summary

Round 02 is a well-targeted mechanical revision that correctly resolves four of
the five round-01 findings (R-012, R-013, R-014 partially, R-015). Response
Matrix is complete; inherited archModule MASTER directives (00-M-001, 00-M-002,
01-M-001 through 01-M-004) are faithfully applied; the plan is under the 350-
line budget (actual 341, per `wc -l`); phase merging matches the single-commit
compile constraint from round-01's review.

However, the core R-011 remediation — the absolute-path rewrite table — is
still materially incomplete. An independent audit using a complementary
substring regex (`arch::riscv::(csr|mm|trap|inst|isa)::` plus a paragraph-
level scan for nested `arch::riscv::{ … csr::, trap::, … }` groupings) finds
two absolute-path rewrite sites that break `cargo build` after Phase 1+2 if
left untouched: the top-of-file multi-line `use` in `arch/riscv/trap/handler.rs:5-12`
and the second test-module multi-line `use` in `arch/riscv/inst/float.rs:659-666`.
Both are multi-line nested-brace imports where the text `crate::arch::riscv::`
and the sub-topic `csr::`/`trap::` arms live on separate lines, so neither the
plan's two-pattern audit nor its PRIMARY `rg` gate (`rg 'crate::arch::riscv::
(csr|mm|trap|inst|isa)::' xemu/xcore/src`, single-line) detects them. `cargo
test` would still catch the breakage at gate time, so the iteration does not
risk a false-green merge, but the executor following row-counts verbatim will
hit two avoidable compile errors mid-PR, and the plan's claim that "no further
sites found" is not accurate.

Answers to the specific convergence questions:
- (a) R-011 is NOT fully resolved. Two multi-line nested-brace sites remain
  missing from the 20-row table; the audit regex used by the plan does not
  catch them, and the PRIMARY gate as written would report 0 hits even for
  incomplete rewrites of this shape. Recurring-HIGH R-016.
- (b) Inherited archModule MASTER directives are respected: no `trait Arch`,
  topic organisation preserved under `cpu/`, direct `pub type`/`pub use`
  seams, phase count ≤ round-01 budget, `build.rs` authoritative, seam
  surface narrowed to `arch::riscv::cpu::*`.
- (c) The plan is implementable with zero boot regressions once R-016 is
  addressed. No validation hole that would hide a difftest divergence.
- (d) Recommend: skip 02_MASTER, but require round-03 to land the 2-row
  table extension + a multiline-aware audit regex. The delta is three lines
  of plan body; convergence on round 03 is realistic.



---

## Findings

### R-016 `Two absolute-path rewrite sites missing from the Phase-1+2 table (recurring R-011)`

- Severity: HIGH
- Section: `Implement / Execution Flow / Phase 1+2 absolute-path rewrite table; Validation / V-F-2 PRIMARY`
- Type: Correctness
- Problem:
  The 20-row table at `02_PLAN.md:170-192` and the post-audit statement at
  `02_PLAN.md:43` ("no further sites found") are incomplete. An independent
  audit finds two absolute-path sites that must be rewritten for Phase 1+2 to
  compile:
  1. `xemu/xcore/src/arch/riscv/trap/handler.rs:5-12` (top-of-file `use`):
     ```rust
     use crate::{
         arch::riscv::{
             cpu::RVCore,
             csr::{CsrAddr, MStatus, PrivilegeMode, TrapCause},
             trap::{Exception, Interrupt, PendingTrap},
         },
         config::Word,
     };
     ```
     After the nest, `crate::arch::riscv::csr` and `crate::arch::riscv::trap`
     do not exist — they are `crate::arch::riscv::cpu::csr` and
     `crate::arch::riscv::cpu::trap`. The block must become something like
     `arch::riscv::cpu::{ RVCore, csr::{…}, trap::{…} }`. Plan's row #7
     covers only the test-module use at `handler.rs:179`, not this one.
  2. `xemu/xcore/src/arch/riscv/inst/float.rs:659-666` (second multi-line
     `use` inside the test module):
     ```rust
     use crate::{
         arch::riscv::{
             cpu::RVCore,
             csr::{CsrAddr, MStatus},
         },
         config::CONFIG_MBASE,
         isa::RVReg,
     };
     ```
     Same failure mode. Plan's row #17 covers only the top-of-file
     `float.rs:15` single-line multi-import, not this second block.

  Both were missed by the plan's audit command because the two-pattern regex
  (`rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)' …` and
  `rg 'crate::\s*\{[^}]*arch::riscv::(csr|mm|trap|inst|isa)' … --multiline`)
  is line-oriented / wrapper-oriented and does not span `arch::riscv::{\n …
  csr::{…}` groupings. The PRIMARY gate
  (`rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)::' xemu/xcore/src`,
  line-based) has the same blind spot.

- Why it matters:
  Round 01's HIGH R-011 was meant to close exactly this category of miss.
  After Phase 1+2 lands, these two files will fail `cargo build`. The
  executor will debug at the keyboard, not from the plan — the point of the
  rewrite table is to be exhaustive so that mechanical execution works.
  The round-01 review already established that "table completeness is the
  acceptance bar, not compile-success"; the plan is therefore not yet at
  that bar. Additionally, because the PRIMARY gate misses these sites, a
  plan-faithful executor who fixed only the 20 listed rows and ran the
  gate would see "0 hits" and believe the rewrite is complete — a
  false-success signal that undermines R-014's intent.

- Recommendation:
  In round 03:
  1. Extend the Phase-1+2 table with two new rows #21 and #22:
     - `arch/riscv/cpu/trap/handler.rs:5-12` (top-of-file `use`) —
       before: multi-line `arch::riscv::{ cpu::RVCore, csr::{…}, trap::{…} }`;
       after: `arch::riscv::cpu::{ RVCore, csr::{…}, trap::{…} }` (the three
       arms collapse under a single `cpu::` prefix).
     - `arch/riscv/cpu/inst/float.rs:659-666` (test-module `use`) — same
       shape, two arms: `cpu::{ RVCore, csr::{…} }`.
  2. Strengthen the audit command used to seed the table. Either:
     - (a) add a third line-based pattern that matches the inner arm after
       a nested-brace open, e.g.
       `rg -U --multiline-dotall 'arch::riscv::\s*\{[^}]*\b(csr|mm|trap|inst|isa)::' xemu/xcore/src`,
       or
     - (b) replace both audit patterns with a simpler substring scan that
       ignores wrappers altogether:
       `rg '\barch::riscv::(csr|mm|trap|inst|isa)\b' xemu/xcore/src` — this
       single pattern hits every site regardless of line wrapping or prefix,
       including both new ones. Cross-check against the table for every hit
       inside `arch/riscv/` vs `arch/**` seam files.
  3. Promote the single-pattern scan in (b) to the PRIMARY Phase-1+2 gate in
     place of the current `rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)::'`.
     The current pattern's reliance on the `crate::` prefix and the line-
     internal `::` is the root cause of R-016; dropping `crate::` and the
     trailing `::` (using `\b` instead) removes the blind spot without
     introducing false positives (the post-nest correct form is
     `arch::riscv::cpu::csr::…`, which does not match `\barch::riscv::csr\b`).



### R-017 `Seam-file secondary gate uses a permissive exclusion glob`

- Severity: LOW
- Section: `Validation / V-F-2 SECONDARY`
- Type: Validation
- Problem:
  The SECONDARY gate at `02_PLAN.md:306-310` is
  `rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)::' xemu/xcore/src --glob
  '!arch/**'`. The glob excludes the entire `arch/**` subtree, which is
  correct for a seam-file focus, but the plan describes the expected result
  as "0 hits outside the 4 seam files" while the glob actually excludes
  seam files living OUTSIDE `arch/**` — which are `cpu/mod.rs`,
  `isa/mod.rs`, `device/mod.rs`, `device/intc/mod.rs`. The four seam
  files are precisely the surfaces that SHOULD match (rows #1-#4 of the
  table), so a "0 hits" claim is wrong by construction; the correct
  expectation is "exactly 4 hits, all in the four seam files". The
  SECONDARY gate as worded will either be silently ignored or misinterpreted
  as a failure if executors read it literally.
- Why it matters:
  Low on its own (SECONDARY, not authoritative), but compounds R-016's
  gate-reliability concern. If the PRIMARY gate has a blind spot and the
  SECONDARY gate has a wording bug, neither is trustworthy as a green-bar
  signal.
- Recommendation:
  Reword V-F-2 SECONDARY to: "SECONDARY: `rg 'crate::arch::riscv::(csr|mm|
  trap|inst|isa)::' xemu/xcore/src` returns exactly 4 hits, all inside
  `cpu/mod.rs:45`, `isa/mod.rs:10`, `device/mod.rs:58` (doc) and :61,
  `device/intc/mod.rs:10`. Any other hit indicates a missed rewrite." This
  matches actual expected state and removes the glob-mismatch trap.



### R-018 `Ex-Phase-4 "isolation confirm" folded — green-bar pin is thin`

- Severity: LOW
- Section: `Implement / Execution Flow / Phase 4`
- Type: Validation
- Problem:
  `02_PLAN.md:216-219` collapses the round-01 Phase-4 "re-run
  `arch_isolation`" step into one bullet of the new Phase 4. The
  description is "doc-comment refresh; arrays invariant". Current
  `xcore/tests/arch_isolation.rs` uses a generic
  `contains("crate::arch::riscv::")` check (per `01_PLAN.md:273`) which is
  path-prefix-stable under the nest, so the arrays are indeed invariant.
  That reasoning is sound. What's thin is the proof-of-green: the plan
  lists "Re-run `xcore/tests/arch_isolation.rs::arch_isolation` to confirm
  no violations" as a bullet rather than an explicit gate assertion with a
  named expected outcome. `make test` covers it transitively, so this is
  not a hole, only an explicitness gap.
- Why it matters:
  Non-blocking. If I-1 / I-2 / I-3 break silently due to a missed rewrite
  somewhere, the executor needs a precise assertion to bisect against;
  "no violations" is ambiguous (zero new patterns? zero total hits?). This
  is a clarity nit, not a correctness one.
- Recommendation:
  Sharpen the bullet to: "Re-run `arch_isolation` — expected result: same
  pass/fail profile as pre-nest, i.e. the test passes with identical
  per-check counts. If the counts change, a seam boundary shifted and the
  rewrite table is incomplete." Optionally: include this in V-IT-1 rather
  than Phase-4 execution so the assertion becomes a gate.



---

## Trade-off Advice

No new trade-offs this round. Prior TR-1..TR-4 are correctly marked CLOSED
in the Response Matrix. The decision to fold ex-Phase-4 into the docs+boot
phase (TR-4 → R-013) is sound: the old phase was a 1-line doc-pin plus a
test re-run, not independently useful as a standalone commit under 01-M-002
(elegance). No objection.



---

## Positive Notes

- Response Matrix is complete: R-001..R-015 all appear with explicit
  Accepted / CLOSED decisions and resolutions. MASTER slot correctly notes
  "no new MASTER this round" and enumerates inherited directives.
- Phase 1+2 merge into a single commit correctly honours round-01's
  "cargo won't compile a bare nest" observation; phase-ordering rationale
  is stated inline at `02_PLAN.md:130-131` rather than buried in a footnote.
- Policy-A scope correction (R-012) is precise: plan now states "1 edit at
  `mm/tlb.rs:10`; two sister sites audit-confirmed wide" and the Data-
  Structure delta at `02_PLAN.md:105-112` cites the exact audit regex. An
  independent `rg 'pub\(in crate::arch::riscv' …` audit confirms the claim.
- `include_str!` hop (R-015) is called out as a distinct Phase-1+2 action
  entry with both before/after paths at `02_PLAN.md:152-158` and the path-
  depth arithmetic (four `../` segments) is stated explicitly. Failure-Flow
  step 4 pins the correct literal for disaster recovery.
- Body length discipline: 341 lines under the 350 budget, down from 394 in
  round 01, with no loss of load-bearing content.
- Seam-file rewrite rows (#1-#4) correctly identify all four seam surfaces
  and use consistent before/after text. Cross-checked against current code:
  `cpu/mod.rs:45`, `isa/mod.rs:10`, `device/mod.rs:58/61`,
  `device/intc/mod.rs:10` all present and matching.
- Failure Flow correctly pre-empts the `make debian` timeout case
  (step 6: "extend to 180s, do not relax `debian login:` grep") — this
  preserves the hard user constraint while allowing instrumentation slack.



---

## Approval Conditions

### Must Fix
- R-016

### Should Improve
- R-017
- R-018

### Trade-off Responses Required
- None (all prior TRs CLOSED)

### Ready for Implementation
- No
- Reason: R-016 HIGH is a recurring miss of R-011 with two concrete,
  auditable absolute-path sites still absent from the Phase-1+2 rewrite
  table (`arch/riscv/trap/handler.rs:5-12` and `arch/riscv/inst/float.rs:659-666`).
  The miss is also symptomatic of a line-oriented audit regex that
  cannot span multi-line nested-brace imports. Round 03 should (a) add
  the two rows, and (b) replace the audit/PRIMARY-gate pattern with one
  that uses `\barch::riscv::(csr|mm|trap|inst|isa)\b` (no `crate::` prefix,
  no trailing `::`), which catches every shape — single-line absolute,
  wrapper multi-import, and nested-brace grouping — without false
  positives against the correct post-nest `cpu::csr::…` form. Delta is
  ~3 plan lines; convergence realistic within round 03.
