# `archLayout` PLAN `02`

> Status: Draft
> Feature: `archLayout`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `01_MASTER.md` (blank — user skipped)

---

## Summary

Third iteration. Round-01 was approved with one blocking HIGH (R-011 — 5
multi-import sites missing from the Phase-2 rewrite table) plus four
non-blocking items. This revision: (a) extends the rewrite table with the 5
multi-import rows after an independent two-pattern `rg` audit (R-011);
(b) corrects the Response-Matrix overclaim for Policy A — 1 edit site, not 3
(R-012); (c) folds the former Phase 4 (no-op `arch_isolation` confirm) into the
docs+boot phase so the plan is now 4 phases (R-013 / TR-4); (d) promotes the
no-glob `rg` command to the primary Phase-1+2 gate (R-014); (e) adds an
explicit `include_str!` row in the Phase-1+2 action list (R-015). Spec sections
(Goals / Non-Goals / Architecture / Invariants / API Surface / Constraints)
are inherited verbatim from `01_PLAN.md`; this document only records deltas.
No new MASTER this round.

## Log

[**Feature Introduce**]

Round 02 is a mechanical revision. No architectural change vs round-01:
Direction A stands, rename stays dropped, Policy A stays. Deltas are table
extensions (R-011, R-015), phase-count reduction (R-013), Response-Matrix
accuracy fix (R-012), and gate-command promotion (R-014).

[**Review Adjustments**]

- R-011 HIGH (closes R-001 from round 00): two-pattern audit run; Phase-1+2
  table extended with the 5 multi-import sites the review named
  (`inst/privileged.rs:6`, `inst/float.rs:15`, `inst/compressed.rs:361`,
  `inst/atomic.rs:171`, `mm/pmp.rs:6`); no further sites found.
- R-012 LOW: R-002 Response-Matrix row reworded to "1 edit at `mm/tlb.rs:10`;
  two sister sites audit-confirmed wide (no edit)".
- R-013 LOW + TR-4: ex-Phase 4 folded into the docs+boot phase. Phases 5 → 4.
- R-014 MEDIUM: no-glob `rg` promoted to PRIMARY gate; `--glob '!arch/**'`
  variant demoted to supplementary seam-only check.
- R-015 LOW: `include_str!` hop change listed as a distinct entry in the
  Phase-1+2 action list.

[**Master Compliance**]

No new MASTER this round (user explicitly skipped `01_MASTER.md`). Inherited
archModule directives apply unchanged: 00-M-001 (no `trait Arch` added),
00-M-002 (nesting by CPU-concern is still topic-organised), 01-M-001 (direct
`pub type`/`pub use` seams), 01-M-002 (phase count 5 → 4, body ≤ 350 lines),
01-M-003 (`build.rs` authoritative; no new cfg), 01-M-004 CRITICAL (seam
surface narrowed to a single `arch::riscv::cpu::*` subtree, as in round 01).

### Changes from Previous Round

[**Added**] 5 new rows (#16–#20) in the Phase-1+2 `use`-path table
(multi-import sites); 1 new entry for the `include_str!` edit; PRIMARY /
SECONDARY gate labelling in Validation.

[**Changed**] Phase count 5 → 4 (ex-Phase 4 folded); R-002 matrix row now
"1 edit + 2 audit-confirmed wide" (was "3 widenings"); four PRs instead of
five; body budget ≤ 350 lines.

[**Removed**] Standalone old-Phase-4 confirmation PR.

[**Unresolved**] None blocking. Residual risk: `make debian` boot time under
difftest instrumentation (documented in Failure Flow step 6).

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 HIGH (round 00) | CLOSED | Fully resolved by R-011 in this round — Phase-1+2 table is now two-pattern `rg`-audited and covers 20 `use`-path sites + 1 `include_str!` site. |
| Review | R-002 HIGH (round 00) | CLOSED | Policy A: 1 edit at `mm/tlb.rs:10` widens `pub(in crate::arch::riscv::mm)` → `pub(in crate::arch::riscv)`. Sister sites `mm/tlb.rs:58` and `mm/mmu.rs:20` already use the wider scope (audited; no edit). |
| Review | R-003 HIGH (round 00) | CLOSED | Four phases (1+2 merged nest+rewrite; 3 visibility; 4 docs+isolation+boot). Each PR independently green; phase ordering respects `cargo build`'s inability to accept a bare nest. |
| Review | R-004..R-009 (round 00) | CLOSED | Resolved inline in round 01; no change this round. |
| Review | R-010 (round 00) | CLOSED | Body compressed further to ≤ 350 lines. |
| Review | R-011 HIGH (round 01) | Accepted | Phase-1+2 `use`-path table extended with 5 multi-import rows (#16–#20). Audit command updated to the two-pattern form; no further sites found. |
| Review | R-012 LOW (round 01) | Accepted | Response-Matrix wording for R-002 corrected as above. Data-Structure section and Phase 2 already said this correctly; only the matrix row is fixed. |
| Review | R-013 LOW (round 01) | Accepted | Former Phase 4 (1-line `arch_isolation` doc-comment pin) folded into Phase 3. Now 4 phases. |
| Review | R-014 MEDIUM (round 01) | Accepted | No-glob `rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)::' xemu/xcore/src` is the PRIMARY Phase-1+2 gate; `--glob '!arch/**'` variant demoted to supplementary seam-only check. |
| Review | R-015 LOW (round 01) | Accepted | `include_str!` path edit listed as a distinct action entry in Phase-1+2 adjacent to the `use`-path table; Phase-1+2 scope now explicitly "Rust `use`-paths + one `include_str!`". |
| Trade-off | TR-1 | CLOSED | Direction A. Closed round 01. |
| Trade-off | TR-2 | CLOSED | Drop rename. Closed round 01. |
| Trade-off | TR-3 | CLOSED | Policy A. Closed round 01. |
| Trade-off | TR-4 | CLOSED | Fold ex-Phase 4 into Phase 3. Accepted per R-013. |
| Master | — | — | No new MASTER this round. Inherited archModule directives (00-M-001..01-M-004) applied as in Master Compliance. |

---

## Spec

All Spec sections (Goals G-1..G-5, Non-Goals NG-1..NG-8, Architecture
before/after tree, Invariants I-1..I-6, Data Structure, API Surface,
Constraints C-1..C-7) are inherited verbatim from `01_PLAN.md` Spec. The
following deltas apply:

[**Data Structure — correction per R-012**]

One edit under Policy A: `mm/tlb.rs:10` `pub(in crate::arch::riscv::mm) struct
TlbEntry` → `pub(in crate::arch::riscv) struct TlbEntry`. Sister sites
`mm/tlb.rs:58` (`struct Tlb`) and `mm/mmu.rs:20` (`tlb: Tlb`) already use the
wider scope — audited this round via `rg 'pub\(in crate::arch::riscv'
xemu/xcore/src`. No other `pub(in crate::arch::riscv::{csr|mm|trap|inst|isa}…)`
sites exist.

[**Constraints — round-02 budget**]

- C-7 tightened: plan body ≤ 350 lines (was ≤ 400 in round 01).

Remaining Spec content is carried forward unchanged; see `01_PLAN.md`
lines 84–195.

---

## Implement

### Execution Flow

[**Main Flow**]

1. **Phase 1+2 — Nest + `use`-path + `include_str!` rewrite (one PR).**
   A bare nest cannot compile without path rewrites, so the nest and every
   path edit land in the same commit. Two-pattern `rg` audit seeds the table.

   Structural moves (`git mv`, five topic dirs + their sibling `.rs`):
   - `arch/riscv/{csr, csr.rs, mm, mm.rs, trap, trap.rs, inst, inst.rs, isa}`
     → `arch/riscv/cpu/…`.

   Module-declaration edits:
   - `arch/riscv/mod.rs`: children reduce to `pub mod cpu; pub mod device;`.
     Refresh doc-comment (old text says "flat topic layout").
   - `arch/riscv/cpu/mod.rs`: add `pub mod csr; pub(in crate::arch::riscv)
     mod mm; pub mod trap; mod inst; pub mod isa;`. Rewrite the lines-11–16
     use-block as (R-004, round 01):
     ```rust
     use self::{
         csr::{CsrAddr, CsrFile, MStatus, Mip, PrivilegeMode},
         mm::{Mmu, Pmp},
         trap::{PendingTrap, TrapCause, interrupt::HW_IP_MASK},
     };
     use super::device::intc::{aclint::Aclint, plic::Plic};
     ```

   `include_str!` edit (R-015, non-`use`-path):
   - `arch/riscv/cpu/isa/decoder.rs`:
     `include_str!("../../../isa/instpat/riscv.instpat")` →
     `include_str!("../../../../isa/instpat/riscv.instpat")` (four `../`
     segments, taking the decoder from `src/arch/riscv/cpu/isa/` back to
     `src/`, then down into `isa/instpat/`).

   Relative-path fixups (depth change only):
   - `arch/riscv/cpu/debug.rs:4-5`: `super::super::csr::{…}` → `super::csr::{…}`.
   - `arch/riscv/cpu/debug.rs:45`: `super::super::csr::DIFFTEST_CSRS` →
     `super::csr::DIFFTEST_CSRS`.
   - `arch/riscv/cpu/mm.rs:12-14`: drop redundant `cpu::` prefix
     (`super::{RVCore, csr::{…}, trap::{…}}`).

   Absolute-path `use` rewrite table (audited via both
   `rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)' xemu/xcore/src` and
   `rg 'crate::\s*\{[^}]*arch::riscv::(csr|mm|trap|inst|isa)' xemu/xcore/src
   --multiline`):

   | # | File:Line | Before | After |
   |---|-----------|--------|-------|
   | 1  | `cpu/mod.rs:45` (seam) | `crate::arch::riscv::trap::PendingTrap` | `crate::arch::riscv::cpu::trap::PendingTrap` |
   | 2  | `isa/mod.rs:10` (seam) | `crate::arch::riscv::isa::{…}` | `crate::arch::riscv::cpu::isa::{…}` |
   | 3  | `device/mod.rs:58` (seam doc) | `crate::arch::riscv::trap::interrupt` | `crate::arch::riscv::cpu::trap::interrupt` |
   | 4  | `device/mod.rs:61` (seam) | `crate::arch::riscv::trap::interrupt::{…}` | `crate::arch::riscv::cpu::trap::interrupt::{…}` |
   | 5  | `arch/riscv/cpu/mod.rs:277` (test) | `crate::arch::riscv::{csr::CsrAddr, trap::Exception}` | `crate::arch::riscv::cpu::{csr::CsrAddr, trap::Exception}` |
   | 6  | `arch/riscv/cpu/mod.rs:413` (test) | `crate::arch::riscv::{csr::MStatus, trap::Interrupt}` | `crate::arch::riscv::cpu::{csr::MStatus, trap::Interrupt}` |
   | 7  | `arch/riscv/cpu/trap/handler.rs:179` (test) | `crate::arch::riscv::{csr::{…}, trap::{…}}` | `crate::arch::riscv::cpu::{csr::{…}, trap::{…}}` |
   | 8  | `arch/riscv/cpu/inst/privileged.rs:102` (test) | `crate::arch::riscv::trap::{TrapCause, test_helpers::…}` | `crate::arch::riscv::cpu::trap::{TrapCause, test_helpers::…}` |
   | 9  | `arch/riscv/cpu/inst/zicsr.rs:106` (test) | `crate::arch::riscv::{csr::CsrAddr, trap::test_helpers::…}` | `crate::arch::riscv::cpu::{csr::CsrAddr, trap::test_helpers::…}` |
   | 10 | `arch/riscv/cpu/inst/zicsr.rs:234` | `crate::arch::riscv::csr::PrivilegeMode::User` | `crate::arch::riscv::cpu::csr::PrivilegeMode::User` |
   | 11 | `arch/riscv/cpu/inst/atomic.rs:9` | `crate::{arch::riscv::mm::MemOp, …}` | `crate::{arch::riscv::cpu::mm::MemOp, …}` |
   | 12 | `arch/riscv/cpu/inst/compressed.rs:7` | `crate::{arch::riscv::trap::Exception, …}` | `crate::{arch::riscv::cpu::trap::Exception, …}` |
   | 13 | `arch/riscv/cpu/csr/ops.rs:138` (test) | `crate::arch::riscv::trap::test_helpers::assert_illegal_inst` | `crate::arch::riscv::cpu::trap::test_helpers::assert_illegal_inst` |
   | 14 | `arch/riscv/cpu/mm.rs:340` (test) | `crate::{arch::riscv::trap::test_helpers::assert_trap, …}` | `crate::{arch::riscv::cpu::trap::test_helpers::assert_trap, …}` |
   | 15 | `arch/riscv/cpu/mm/mmu.rs:11` | `crate::{arch::riscv::csr::PrivilegeMode, …}` | `crate::{arch::riscv::cpu::csr::PrivilegeMode, …}` |
   | 16 | `arch/riscv/cpu/inst/privileged.rs:6` (multi) | `crate::{ arch::riscv::csr::{CsrAddr, Exception, MStatus, PrivilegeMode}, … }` | `crate::{ arch::riscv::cpu::csr::{CsrAddr, Exception, MStatus, PrivilegeMode}, … }` |
   | 17 | `arch/riscv/cpu/inst/float.rs:15` (multi) | `crate::{ arch::riscv::csr::CsrAddr, … }` | `crate::{ arch::riscv::cpu::csr::CsrAddr, … }` |
   | 18 | `arch/riscv/cpu/inst/compressed.rs:361` (test multi) | `crate::{ arch::riscv::trap::{TrapCause, test_helpers::assert_trap}, … }` | `crate::{ arch::riscv::cpu::trap::{TrapCause, test_helpers::assert_trap}, … }` |
   | 19 | `arch/riscv/cpu/inst/atomic.rs:171` (test multi) | `crate::{ arch::riscv::trap::{Exception, TrapCause, test_helpers::assert_trap}, … }` | `crate::{ arch::riscv::cpu::trap::{Exception, TrapCause, test_helpers::assert_trap}, … }` |
   | 20 | `arch/riscv/cpu/mm/pmp.rs:6` (multi) | `crate::{ arch::riscv::csr::PrivilegeMode, … }` | `crate::{ arch::riscv::cpu::csr::PrivilegeMode, … }` |

   Positive sanity (no edit, confirmed by audit): `cpu/csr/ops.rs:4` uses
   `crate::arch::riscv::cpu::RVCore` (path-stable); six of the seven
   `cpu/inst/` children (`base, float, mul, privileged, zicsr, …`) use
   `super::{RVCore, rv64_*}` via `cpu/inst.rs`, path-stable after nest.

   Gate: PRIMARY `rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)::'
   xemu/xcore/src` → 0 hits anywhere (pre-nest flat paths eliminated);
   `cargo test --workspace` → 344 tests pass; `make fmt && make clippy`.

2. **Phase 3 — Visibility widen (Policy A, one edit).**
   `arch/riscv/cpu/mm/tlb.rs:10`: `pub(in crate::arch::riscv::mm) struct
   TlbEntry` → `pub(in crate::arch::riscv) struct TlbEntry`. No other edits
   (confirmed by `rg 'pub\(in crate::arch::riscv' xemu/xcore/src` — `tlb.rs:58`
   and `mmu.rs:20` already wide). Gate: `cargo test --workspace` → 344 green.

3. **Phase 4 — Docs + `arch_isolation` pin + full boot verification.**
   (Absorbs the R-013-folded ex-Phase 4.)
   - `arch/riscv/mod.rs` doc: replace "Flat topic layout …" with a one-line
     description of the nested layout.
   - `arch/riscv/cpu/mod.rs` doc: add one line distinguishing `cpu/isa/`
     (encoding) from `cpu/inst/` (execution) — R-007 / TR-2 closure.
   - `xcore/tests/arch_isolation.rs` header doc-comment: refresh mentions of
     seam paths; `SEAM_FILES`, `SEAM_ALLOWED_SYMBOLS`, `BUS_DEBUG_STRING_PINS`
     arrays are invariant under the nest (I-3) — no value change.
   - Re-run `xcore/tests/arch_isolation.rs::arch_isolation` to confirm no
     violations.
   - Full boot gate:
     - `make fmt && make clippy && make test && make run`.
     - `timeout 60 make linux 2>&1 | tee /tmp/linux.log && grep -q 'Welcome
       to Buildroot' /tmp/linux.log`.
     - `timeout 120 make debian 2>&1 | tee /tmp/debian.log && grep -q
       'debian login:' /tmp/debian.log`.
     - Difftest regression corpus (the archModule-03 green set): zero new
       divergences.

[**Failure Flow**]

1. Phase-1+2 compile fails (missed path site): patch in the same commit; do
   not amend already-landed phases. The PRIMARY gate (`rg … xemu/xcore/src`,
   no glob) would catch any remaining site.
2. `arch_isolation` red on a new pattern: triage real I-1 leakage vs missed
   seam edit; fold seam fix into the same PR.
3. `make linux` / `make debian` diverges: bisect across the four phase commits.
4. `include_str!` compile error: verify the argument is exactly
   `"../../../../isa/instpat/riscv.instpat"` (four `../` segments).
5. `cargo build --features isa32` fails: cfg-gated imports in moved files —
   fix in-phase.
6. `make debian` timeout: extend to 180s under difftest instrumentation only;
   do not relax the `debian login:` success-marker grep.

[**State Transition**]

S0 (archModule-03 landed) → S1 (Phase 1+2 merged: nest + 20 `use`-path
rewrites + 1 `include_str!` edit + relative-path fixups; 344 green) → S2
(Phase 3: `tlb.rs:10` widened; 344 green) → S3 (Phase 4: docs + isolation
pin + `make linux` + `make debian` + difftest; refactor complete).

### Implementation Plan

Four PRs, each independently green-barable:

- **PR1 (Phase 1+2)** — all `git mv`s; module-decl moves; `include_str!` hop;
  relative-path fixups; 20-row absolute-path table. Deliverable: nested tree
  with every path-site resolved. Gate: `make fmt && make clippy && make
  test && make run`; PRIMARY `rg` 0-hit; SECONDARY `--glob '!arch/**'` `rg`
  0-hit-outside-seam.
- **PR2 (Phase 3)** — one-line edit to `cpu/mm/tlb.rs:10`. Gate: `cargo
  test --workspace`.
- **PR3 (Phase 4 docs + isolation pin)** — doc-comment refreshes in three
  files; re-run `arch_isolation`. Gate: `make fmt && make clippy && make
  test && make run`.
- **PR4 (Phase 4 boot)** — full boot suite: `make linux`, `make debian`,
  difftest regression corpus. Deliverable: refactor-complete signal.

## Trade-offs

- **TR-1 CLOSED — Direction A.** Nest under `cpu/`. Closed round 01.
  Direction B (move `isa` back to `xcore/src/isa/riscv/`) rejected on
  01-M-004 grounds.
- **TR-2 CLOSED — drop rename.** `cpu/isa/` next to `cpu/inst/` is
  structurally disambiguating; a one-line `cpu/mod.rs` doc-comment captures
  encoding-vs-execution. Closed round 01.
- **TR-3 CLOSED — Policy A.** Widen `pub(in …::mm)` (one site only) to
  `pub(in crate::arch::riscv)`. Closed round 01.
- **TR-4 CLOSED — fold ex-Phase 4.** `arch_isolation` confirm is a 1-line
  doc-comment pin and a test re-run; absorbing it into the docs+boot phase
  removes an otherwise-empty PR without loss of safety (01-M-002 elegance).
  Accepted this round (R-013).

## Validation

[**Unit Tests**]
- V-UT-1: `cargo test --workspace` — all 344 tests green at every phase
  boundary. Covers CSR / MMU / PMP / TLB / trap / decoder / dispatch / RVCore
  test modules.

[**Integration Tests**]
- V-IT-1: `xcore/tests/arch_isolation.rs::arch_isolation` passes unchanged
  under the nested layout. Locks in I-1, I-2, I-3. Runs as part of V-UT-1.
- V-IT-2: `make run` — default direct-boot reaches HIT GOOD TRAP.
- V-IT-3: `timeout 60 make linux 2>&1 | tee /tmp/linux.log && grep -q
  'Welcome to Buildroot' /tmp/linux.log` — Phase 4.
- V-IT-4: `timeout 120 make debian 2>&1 | tee /tmp/debian.log && grep -q
  'debian login:' /tmp/debian.log` — Phase 4.

[**Failure / Robustness Validation**]
- V-F-1: `git log --follow arch/riscv/cpu/<topic>/<file>` reaches the
  pre-nest ancestor for every moved file. Confirms `git mv` used.
- V-F-2 PRIMARY (R-014): `rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)::'
  xemu/xcore/src` — post-Phase-1+2 returns 0 hits anywhere. This is the
  authoritative proof that the rewrite is complete; covers `arch/**` and
  catches R-011's multi-import sites at gate time.
- V-F-2 SECONDARY (supplementary, seam-only): `rg
  'crate::arch::riscv::(csr|mm|trap|inst|isa)::' xemu/xcore/src --glob
  '!arch/**'` — returns 0 hits outside the 4 seam files (`cpu/mod.rs`,
  `isa/mod.rs`, `device/mod.rs`, `device/intc/mod.rs`). Useful as a seam-
  focused sanity check but NOT authoritative.
- V-F-3: Per-phase bisection — each of the four phase commits in isolation
  is green on `make test`.
- V-F-4: Difftest vs QEMU and Spike on the archModule-03 regression corpus
  — zero new divergences.

[**Edge Case Validation**]
- V-E-1: `cargo build --no-default-features --features isa32` — RV32 build
  still compiles.
- V-E-2: `cargo clippy --all-targets -- -D warnings` — no new warnings.
- V-E-3: `cargo fmt --check` — formatting invariant under moves.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|---|---|
| G-1 nest under `cpu/` | V-IT-1; V-F-1; V-F-2 PRIMARY; V-UT-1 |
| G-2 `device/` sibling | V-IT-1 |
| G-3 structural disambiguation | V-UT-1 (both `cpu::isa` and `cpu::inst` test modules pass) + Phase 4 doc |
| G-4 `instpat` at crate root | V-UT-1 (decoder loads); V-E-1 |
| G-5 byte-identical behaviour | V-IT-2, V-IT-3, V-IT-4; V-F-4; V-UT-1 |
| C-1 every phase green | V-F-3 |
| C-2 `git mv` only | V-F-1 |
| C-3 byte-identical | V-F-4; V-IT-2..V-IT-4 |
| C-4 no new deps | Cargo.toml diff |
| C-5 `include_str!` correct | V-UT-1 (decoder tests); `cargo build` on PR1 |
| C-6 pest grammar path | `cargo build` post-PR1 |
| C-7 ≤ 350 lines | Plan body self-review |
| I-1, I-2, I-3 seam invariants | V-IT-1 |
| I-4 history | V-F-1 |
| I-5 build.rs authoritative | Diff review |
| I-6 no `trait Arch` | Diff review |
