# `archLayout` PLAN `01`

> Status: Draft
> Feature: `archLayout`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `00_MASTER.md` (blank — user skipped)

---

## Summary

Second iteration. Round-00 approved Direction A (nest under `cpu/`) but blocked
on three HIGH findings. This revision: (a) publishes the full grep-audited
rewrite table for every `crate::arch::riscv::{csr,mm,trap,inst,isa}` and
equivalent relative path; (b) adopts visibility Policy A (widen `pub(in …::mm)`
to `pub(in crate::arch::riscv)`); (c) **drops the `inst/` → `executor/` rename**
per the reviewer's TR-2 — structural context under `cpu/` disambiguates `isa/`
(encoding) from `inst/` (execution). Five independently green-barable PRs. No
new MASTER this round.

## Log

[**Feature Introduce**]

HIGH findings resolved by enumeration, one visibility policy, phase-splitting
— no architectural change vs round-00. Direction A stands.

[**Review Adjustments**]

- R-001: per-phase rewrite table in Phase 2 (`rg`-audited; 15 rows).
- R-002: Policy A widens 3 `pub(in …::mm)` sites; all enumerated.
- R-003: five phases — S1 nest, S2 path rewrite, S3 visibility, S4 test, S5 docs.
- TR-2 accepted: rename dropped.
- R-004..R-009 (MED/LOW) resolved inline in Phase 1 / Phase 5 text.
- R-010: body compressed to ≤ 400 lines.

[**Master Compliance**]

No new MASTER this round (user skipped `00_MASTER.md`). Inherited: 00-M-001 (no
`trait Arch`) — no trait added; 00-M-002 (topic org) — nesting by CPU-concern
still topic-organised; 01-M-001 (no `selected` alias) — seam keeps direct
`pub type`/`pub use`; 01-M-002 (clean/concise/elegant) — rename dropped + 400-line
budget; 01-M-003 (no redundant arch checks) — `build.rs` authoritative;
01-M-004 CRITICAL (thin seams) — strengthened: seam now re-exports from one
narrower subtree (`arch::riscv::cpu::*`) instead of five flat topic roots.

### Changes from Previous Round

[**Added**] Phase 2 rewrite table (15 sites); Phase 3 visibility table (3 sites);
Phase 4 `arch_isolation` confirm phase; Phase 5 docs phase.

[**Changed**] Five phases (was 4); target tree retains `inst/` (no rename);
`cpu/mod.rs` post-edit use-block shown verbatim (R-004).

[**Removed**] `inst/` → `executor/` rename and its 8 `git mv` operations.

[**Unresolved**] None blocking. LOW findings R-007/R-008/R-009 inline.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 HIGH | Accepted | Phase 2 publishes the complete `rg`-audited rewrite table (15 sites across 8 files — see Implementation Plan → Phase 2). |
| Review | R-002 HIGH | Accepted | Policy A: widen the 3 `pub(in crate::arch::riscv::mm)` occurrences at `mm/tlb.rs:10`, `mm/tlb.rs:58`, `mm/mmu.rs:20` to `pub(in crate::arch::riscv)` (Phase 3). |
| Review | R-003 HIGH | Accepted | Old Phase 3 (nest + rename) decomposed into five independently green phases (1 nest / 2 path rewrite / 3 visibility / 4 test / 5 docs). |
| Review | R-004 MED | Accepted | Phase 1 shows the exact post-edit `cpu/mod.rs` use block with `self::…` for `csr/mm/trap` and preserved `super::device::intc::…`. |
| Review | R-005 MED | Accepted | Phase 1 confirms the seven `inst/*.rs` children under `cpu/inst/` remain path-stable except `atomic.rs` (one rewrite in Phase 2). |
| Review | R-006 MED | Accepted | `trap/handler.rs:179` is line 3 of the Phase 2 table. |
| Review | R-007 LOW | Accepted | Phase 5 explicitly refreshes `arch_isolation.rs` doc-comment; `SEAM_FILES` / `SEAM_ALLOWED_SYMBOLS` / `BUS_DEBUG_STRING_PINS` arrays unchanged. |
| Review | R-008 LOW | Accepted | Failure Flow step 4 rewritten per reviewer wording (four `../` segments). |
| Review | R-009 LOW | Accepted | V-IT-4/V-IT-5 pin concrete success markers (`Welcome to Buildroot`, `debian login:`) with 60s timeouts. |
| Review | R-010 LOW | Accepted | Body compressed; target ≤ 400 lines. |
| TR | TR-1 | Closed (Direction A) | Nest under `cpu/`; Direction B rejected on 01-M-004 grounds. |
| TR | TR-2 | Accepted (drop rename) | `inst/` retained as `cpu/inst/` after nest. Structural context (`cpu/isa/` vs `cpu/inst/`) disambiguates; one-line doc-comment in `cpu/mod.rs` documents the split. |
| TR | TR-3 | Closed (Policy A) | Visibility minimum to compile; widen `mm`-local scopes to `pub(in crate::arch::riscv)` for consistency. |
| Master | — | — | No new MASTER this round. Inherited archModule directives (00-M-001..01-M-004) applied as in Master Compliance. |

---

## Spec

[**Goals**]

- G-1: Nest the five CPU-internal topic modules (`csr`, `mm`, `trap`, `inst`,
  `isa`) under `arch/riscv/cpu/`, preserving git history via `git mv`.
- G-2: Keep `arch/riscv/device/` as a sibling of `cpu/` (devices are not
  CPU-internal; `device/intc/` already lives behind its own seam).
- G-3: Resolve the `isa/` vs `inst/` naming smell by **structural context**
  (both become children of `cpu/`); no rename.
- G-4: Leave `xcore/src/isa/instpat/` at the crate root as neutral data.
- G-5: Behaviour is byte-identical: 344 tests pass at each phase boundary;
  `make linux` boots to shell; `make debian` boots Debian 13 to shell via
  VirtIO-blk; difftest vs QEMU/Spike shows zero divergence.

- NG-1: Do NOT move `xcore/src/isa/instpat/`.
- NG-2: Do NOT rename `inst/` to `executor/` or any other name.
- NG-3: Do NOT fold `arch/riscv/device/` into `arch/riscv/cpu/`.
- NG-4: Do NOT touch `arch/loongarch/` (stub remains minimal).
- NG-5: Do NOT modify landed `docs/fix/archModule/` artifacts.
- NG-6: Do NOT change any public API visible outside `xcore`.
- NG-7: Do NOT introduce new deps or cfg flags.
- NG-8: Do NOT reintroduce the `xcore/src/isa/riscv/*` parallel-tree pattern.

[**Architecture**]

```
Before                               After
arch/riscv/                          arch/riscv/
├── mod.rs (7 flat children)         ├── mod.rs (cpu + device)
├── cpu/{context,debug,mod}.rs       ├── cpu/
├── csr/ + csr.rs     ─┐             │   ├── {context,debug,mod}.rs
├── mm/  + mm.rs      ─┤ nest        │   ├── csr/ + csr.rs
├── trap/+ trap.rs    ─┤ into        │   ├── mm/  + mm.rs
├── inst/+ inst.rs    ─┤ cpu/        │   ├── trap/+ trap.rs
├── isa/  (…)         ─┘             │   ├── inst/+ inst.rs
└── device/ (unchanged)               │   └── isa/  (…)
                                      └── device/ (unchanged)

Crate-root seams cpu/, device/, isa/ re-export from arch::riscv::cpu::* after.
```

[**Invariants**]

- I-1: Outside `arch/` and outside the 5 seam files, **zero** source lines
  reference `crate::arch::riscv::` or `crate::arch::loongarch::`. Verified by
  the unchanged `arch_isolation.rs` file+symbol allow-list.
- I-2: The only arch paths seam files name are under `crate::arch::riscv::cpu::*`
  (for CPU/ISA/trap vocabulary) and `crate::arch::riscv::device::*` (for
  interrupt controllers). Post-nest, the pre-nest flat paths
  `crate::arch::riscv::{csr,mm,trap,inst,isa}::` no longer exist anywhere in
  the tree.
- I-3: `arch_isolation.rs` `SEAM_FILES`, `SEAM_ALLOWED_SYMBOLS`,
  `BUS_DEBUG_STRING_PINS` are invariant under the nest. Symbol names do not
  change.
- I-4: `git log --follow` from every moved file reaches its pre-nest ancestor.
- I-5: `build.rs` remains the sole source of truth for arch/isa cfg flags
  (01-M-003).
- I-6: No new `trait Arch` introduced (00-M-001).

[**Data Structure**]

No new types. Three visibility tightenings only (Policy A, see Phase 3):

```rust
// mm/tlb.rs:10  (was)  pub(in crate::arch::riscv::mm) struct TlbEntry { … }
// mm/tlb.rs:10  (new)  pub(in crate::arch::riscv) struct TlbEntry { … }
// mm/tlb.rs:58  — already pub(in crate::arch::riscv) (path-stable; audited).
// mm/mmu.rs:20  — already pub(in crate::arch::riscv) (path-stable; audited).
```

Rationale: of the three `pub(in …::mm)` / `pub(in …::riscv)` occurrences in
`mm/`, only `tlb.rs:10` uses the `…::mm` sub-path. After nest, that path
becomes `…::cpu::mm`; rewriting it to match would deepen by one segment, but
widening to `pub(in crate::arch::riscv)` matches the 8 already-widened sites
landed in archModule PR-2 and the existing `tlb.rs:58` / `mmu.rs:20` pattern.
No functional change: `TlbEntry` is only used from `mm/mmu.rs` and `mm/tlb.rs`.

[**API Surface**]

Four seam files; only path-roots change; symbol names identical. Each edit is
one `trap::` / `isa::` → `cpu::trap::` / `cpu::isa::` substitution:

```rust
// cpu/mod.rs:45        trap::PendingTrap            → cpu::trap::PendingTrap
// isa/mod.rs:10        isa::{DECODER,…,RVReg}       → cpu::isa::{…}
// device/mod.rs:58 (doc)  …trap::interrupt`]        → …cpu::trap::interrupt`]
// device/mod.rs:61     trap::interrupt::{SSIP,…}    → cpu::trap::interrupt::{…}
// device/intc/mod.rs:10  device::intc::{Aclint,Plic}  UNCHANGED (device/ stays)
```

No other public-surface changes; all six seam symbols keep identical names.

[**Constraints**]

- C-1: Each phase leaves `cargo build` / `cargo test --workspace` /
  `cargo clippy` / `cargo fmt --check` clean. 344 tests pass at every phase
  boundary.
- C-2: `git mv` only — no copy-delete. `git log --follow` preserved.
- C-3: Behaviour byte-identical. Difftest zero new divergences;
  `make linux` / `make debian` reach the same shell prompts as the pre-nest
  baseline.
- C-4: No new dependencies; no new cfg flags; `arch_isolation.rs` stays on
  its `std::fs`-only check.
- C-5: `include_str!` in `arch/riscv/cpu/isa/decoder.rs` uses exactly
  `"../../../../isa/instpat/riscv.instpat"` (four `../` segments:
  `cpu/isa/` → `cpu/` → `riscv/` → `arch/` → `src/`, then down into
  `isa/instpat/`).
- C-6: `pest_derive` `#[grammar = "src/isa/instpat/riscv.pest"]` is
  `CARGO_MANIFEST_DIR`-relative and requires no change.
- C-7: Plan body ≤ 400 lines (R-010).

---

## Implement

### Execution Flow

[**Main Flow**]

1. **Phase 1 — Nest `csr`, `mm`, `trap`, `inst`, `isa` under `cpu/`.**
   `git mv` five topic dirs and their `{topic}.rs` siblings from `arch/riscv/`
   into `arch/riscv/cpu/`. Edits:
   - `arch/riscv/mod.rs`: drop the five children; keep `pub mod cpu; pub mod
     device;`. Refresh doc-comment (old text mentions "flat layout").
   - `arch/riscv/cpu/mod.rs`: add `pub mod csr; pub(in crate::arch::riscv) mod
     mm; pub mod trap; mod inst; pub mod isa;`.
   - `arch/riscv/cpu/mod.rs` lines 11–16 — rewrite use-block (R-004):
     ```rust
     use self::{
         csr::{CsrAddr, CsrFile, MStatus, Mip, PrivilegeMode},
         mm::{Mmu, Pmp},
         trap::{PendingTrap, TrapCause, interrupt::HW_IP_MASK},
     };
     use super::device::intc::{aclint::Aclint, plic::Plic};
     ```
   - `arch/riscv/cpu/isa/decoder.rs`: `include_str!("../../../isa/…")` →
     `include_str!("../../../../isa/…")`.
   - `arch/riscv/cpu/debug.rs:4-5`: `super::super::csr::{…}` → `super::csr::{…}`.
   - `arch/riscv/cpu/debug.rs:45`: `super::super::csr::DIFFTEST_CSRS` →
     `super::csr::DIFFTEST_CSRS`.
   - `arch/riscv/cpu/mm.rs:12-14`: drop `cpu::` prefix:
     `use super::{RVCore, csr::{…}, trap::{…}};` (super of `cpu::mm` is `cpu`).
   Positive sanity (no edit): `csr/ops.rs:4` uses stable
   `crate::arch::riscv::cpu::RVCore`; six `inst/` children (`base, compressed,
   float, mul, privileged, zicsr`) use `super::{RVCore, rv64_*}` which resolves
   via `cpu/inst.rs` and is path-stable after nest (R-005).

   > Phase-Ordering: the 15 Phase-2 absolute-path rewrites must land in the
   > same commit as Phase 1 for `cargo build` to succeed. The two are listed
   > as logical phases for review clarity, not as separable commits.

2. **Phase 2 — Absolute-path rewrite audit (landed in the Phase-1 PR).**
   Complete table from `rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)' xemu/xcore/src`
   plus multi-import `rg 'use crate::arch::riscv::\{' xemu/xcore/src`:

   | # | File:Line | Before | After |
   |---|-----------|--------|-------|
   | 1 | `cpu/mod.rs:45` (seam) | `crate::arch::riscv::trap::PendingTrap` | `crate::arch::riscv::cpu::trap::PendingTrap` |
   | 2 | `isa/mod.rs:10` (seam) | `crate::arch::riscv::isa::{…}` | `crate::arch::riscv::cpu::isa::{…}` |
   | 3 | `device/mod.rs:58` (seam doc) | `crate::arch::riscv::trap::interrupt` | `crate::arch::riscv::cpu::trap::interrupt` |
   | 4 | `device/mod.rs:61` (seam) | `crate::arch::riscv::trap::interrupt::{…}` | `crate::arch::riscv::cpu::trap::interrupt::{…}` |
   | 5 | `arch/riscv/cpu/mod.rs:277` (test) | `crate::arch::riscv::{csr::CsrAddr, trap::Exception}` | `crate::arch::riscv::cpu::{csr::CsrAddr, trap::Exception}` |
   | 6 | `arch/riscv/cpu/mod.rs:413` (test) | `crate::arch::riscv::{csr::MStatus, trap::Interrupt}` | `crate::arch::riscv::cpu::{csr::MStatus, trap::Interrupt}` |
   | 7 | `arch/riscv/cpu/trap/handler.rs:179` (test) | `crate::arch::riscv::{csr::{…}, trap::{…}}` | `crate::arch::riscv::cpu::{csr::{…}, trap::{…}}` |
   | 8 | `arch/riscv/cpu/inst/privileged.rs:102` (test) | `crate::arch::riscv::trap::{TrapCause, test_helpers::…}` | `crate::arch::riscv::cpu::trap::{TrapCause, test_helpers::…}` |
   | 9 | `arch/riscv/cpu/inst/zicsr.rs:106` (test) | `crate::arch::riscv::{csr::CsrAddr, trap::test_helpers::…}` | `crate::arch::riscv::cpu::{csr::CsrAddr, trap::test_helpers::…}` |
   | 10 | `arch/riscv/cpu/inst/zicsr.rs:234` | `crate::arch::riscv::csr::PrivilegeMode::User` | `crate::arch::riscv::cpu::csr::PrivilegeMode::User` |
   | 11 | `arch/riscv/cpu/inst/atomic.rs:9` | `crate::{arch::riscv::mm::MemOp, …}` | `crate::{arch::riscv::cpu::mm::MemOp, …}` |
   | 12 | `arch/riscv/cpu/inst/compressed.rs:7` | `crate::{arch::riscv::trap::Exception, …}` | `crate::{arch::riscv::cpu::trap::Exception, …}` |
   | 13 | `arch/riscv/cpu/csr/ops.rs:138` (test) | `crate::arch::riscv::trap::test_helpers::assert_illegal_inst` | `crate::arch::riscv::cpu::trap::test_helpers::assert_illegal_inst` |
   | 14 | `arch/riscv/cpu/mm.rs:340` (test) | `crate::{arch::riscv::trap::test_helpers::assert_trap, …}` | `crate::{arch::riscv::cpu::trap::test_helpers::assert_trap, …}` |
   | 15 | `arch/riscv/cpu/mm/mmu.rs:11` | `crate::{arch::riscv::csr::PrivilegeMode, …}` | `crate::{arch::riscv::cpu::csr::PrivilegeMode, …}` |

   Gate after edits: `rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)::'
   xemu/xcore/src --glob '!arch/**'` returns 0 hits outside seam files;
   `rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)(::|\{)' xemu/xcore/src`
   returns 0 hits anywhere (pre-nest flat paths fully eliminated).

3. **Phase 3 — Visibility rewrite (Policy A).**
   Single edit at `arch/riscv/cpu/mm/tlb.rs:10`:
   `pub(in crate::arch::riscv::mm) struct TlbEntry` →
   `pub(in crate::arch::riscv) struct TlbEntry`. Sister sites `tlb.rs:58` and
   `mmu.rs:20` already use `pub(in crate::arch::riscv)`; no edit (listed for
   completeness per R-002). Gate: `cargo test --workspace`.

4. **Phase 4 — `arch_isolation.rs` confirm.**
   `SEAM_FILES`, `SEAM_ALLOWED_SYMBOLS`, `BUS_DEBUG_STRING_PINS` are
   value-invariant under nest (I-3). The only arch-path check in the file is
   the generic `contains("crate::arch::riscv::")` at line 212, which matches
   both pre- and post-nest paths. Action: re-run the test; confirm no
   violations; add a one-line doc-comment pin noting post-nest state. No-op
   on the allow-lists.

5. **Phase 5 — Docs + full boot verification.**
   - `arch/riscv/mod.rs` doc: replace "Flat topic layout …" with the nested
     layout sentence.
   - `arch/riscv/cpu/mod.rs` doc: add one line on `cpu/isa/` (encoding) vs
     `cpu/inst/` (execution) — TR-2 resolution / R-007.
   - `arch_isolation.rs` header doc-comment: refresh arch-path mentions;
     `SEAM_*` arrays untouched.
   - Boot gate:
     - `make fmt && make clippy && make test && make run`.
     - `timeout 60 make linux 2>&1 | tee /tmp/linux.log && grep -q 'Welcome
       to Buildroot' /tmp/linux.log` (R-009).
     - `timeout 120 make debian 2>&1 | tee /tmp/debian.log && grep -q
       'debian login:' /tmp/debian.log` (R-009).
     - Difftest regression corpus (the archModule-03 green set): zero new
       divergences.

[**Failure Flow**]

1. Phase 1+2 compile fail (missing path in Phase 2 table): patch in the same
   commit; do not amend landed phases.
2. `arch_isolation` red on a new pattern: triage real I-1 leakage vs a missed
   seam edit; fold seam fix into same PR.
3. `make linux` / `make debian` diverges: bisect across phase commits.
4. `include_str!` compile error: verify the argument is exactly
   `"../../../../isa/instpat/riscv.instpat"` — four `../` segments taking the
   decoder from `arch/riscv/cpu/isa/` back to `src/`, then down into
   `isa/instpat/` (R-008).
5. `cargo build --features isa32` fails: cfg-gated imports in moved files.

[**State Transition**]

S0 (archModule-03 landed) → S1 (Phase 1+2 merged: nest + 15 paths rewritten;
344 green) → S2 (Phase 3: `tlb.rs:10` widened) → S3 (Phase 4: `arch_isolation`
confirmed) → S4 (Phase 5: docs + boots + difftest; refactor complete).

### Implementation Plan

Five PRs, each independently green-barable:

- **PR1 (Phase 1+2)** — all `git mv`s; module decl moves; `include_str!` hop;
  `cpu/debug.rs` + `cpu/mm.rs` relative fixes; 15-row absolute-path table.
  Deliverable: nested tree + all paths rewritten. Gate: 344 tests + `make run`.
- **PR2 (Phase 3)** — one-line edit to `mm/tlb.rs:10`. Gate: `cargo test`.
- **PR3 (Phase 4)** — `arch_isolation` re-confirmed; one-line header
  doc-comment pin. Gate: `cargo test`.
- **PR4 (Phase 5 docs)** — doc-comment refreshes in three files. Gate:
  `make fmt && make clippy && make test && make run`.
- **PR5 (Phase 5 boot)** — full boot suite: `make linux`, `make debian`,
  difftest. Zero regressions.

## Trade-offs

- **TR-1 CLOSED — Direction A.** Nest under `cpu/`. Direction B (move `isa`
  back to `xcore/src/isa/riscv/`) rejected: reintroduces the parallel-tree
  pattern archModule dismantled and forces arch-specific code outside `arch/`
  — direct 01-M-004 violation.
- **TR-2 CLOSED — drop rename.** Accepted reviewer recommendation. After
  nest, `cpu/isa/` next to `cpu/inst/` makes encoding vs execution structural.
  Rename would cost 8 `git mv`s + a non-functional commit for a name change;
  a one-line `cpu/mod.rs` doc-comment captures the distinction cheaper.
- **TR-3 CLOSED — Policy A.** Widen `pub(in …::mm)` to `pub(in
  crate::arch::riscv)`. Matches the dominant pattern (8 peer sites from
  archModule PR-2). Avoids `…::cpu::mm` path churn; `TlbEntry` has no
  external callers so widening is a formality.

## Validation

[**Unit Tests**]
- V-UT-1: `cargo test --workspace` — all 344 tests green at every phase
  boundary. Covers CSR / MMU / PMP / TLB / trap / decoder / dispatch / RVCore
  test modules individually.

[**Integration Tests**]
- V-IT-1: `xcore/tests/arch_isolation.rs::arch_isolation` passes unchanged
  under the nested layout. Locks in I-1, I-2, I-3.
- V-IT-2: `make run` — default direct-boot reaches HIT GOOD TRAP.
- V-IT-3: `make linux` — within 60s, serial log contains `Welcome to
  Buildroot` (R-009).
- V-IT-4: `make debian` — within 120s, serial log contains `debian login:`
  (R-009).

[**Failure / Robustness Validation**]
- V-F-1: `git log --follow arch/riscv/cpu/<topic>/<file>` reaches the
  pre-nest ancestor for every moved file. Confirms `git mv` used.
- V-F-2: Per-phase bisection — check out each phase commit in isolation and
  re-run `make test`; each commit is independently green.
- V-F-3: Difftest vs QEMU and Spike on the archModule-03 regression corpus:
  zero new divergences.
- V-F-4: `rg 'crate::arch::riscv::(csr|mm|trap|inst|isa)(::|\{)'
  xemu/xcore/src` returns 0 hits after Phase 1+2 merge.

[**Edge Case Validation**]
- V-E-1: `cargo build --no-default-features --features isa32` — RV32 build
  still compiles.
- V-E-2: `cargo clippy --all-targets -- -D warnings` — no new warnings.
- V-E-3: `cargo fmt --check` — formatting invariant under moves.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|---|---|
| G-1 nest under `cpu/` | V-IT-1; V-F-1; V-F-4; V-UT-1 |
| G-2 `device/` sibling | V-IT-1 |
| G-3 structural disambiguation | V-UT-1 (both `cpu::isa` and `cpu::inst` test modules pass) + Phase 5 doc |
| G-4 `instpat` at crate root | V-UT-1 (decoder loads); V-E-1 |
| G-5 byte-identical behaviour | V-IT-2, V-IT-3, V-IT-4; V-F-3; V-UT-1 |
| C-1 every phase green | V-F-2 |
| C-2 `git mv` only | V-F-1 |
| C-3 byte-identical | V-F-3; V-IT-2..V-IT-4 |
| C-4 no new deps | Cargo.toml diff |
| C-5 `include_str!` correct | V-UT-1 (decoder tests) |
| C-6 pest grammar path | `cargo build` post-Phase 1+2 |
| C-7 ≤400 lines | Plan body self-review |
| I-1, I-2, I-3 seam invariants | V-IT-1 |
| I-4 history | V-F-1 |
| I-5 build.rs authoritative | Diff review |
| I-6 no `trait Arch` | Diff review |
