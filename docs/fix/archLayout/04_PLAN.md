# `archLayout` PLAN `04`

> Status: Draft
> Feature: `archLayout`
> Iteration: `04`
> Owner: Executor
> Depends on:
> - Previous Plan: `03_PLAN.md`
> - Review: `03_REVIEW.md`
> - Master Directive: `03_MASTER.md` (blank — user skipped)

---

## Summary

Fifth and final convergence pass. R-019 HIGH: `pub(in crate::arch::riscv::mm)`
at `mm/tlb.rs:10` hits rustc E0433 once Phase 1 nests `mm/` under `cpu/`
because the widen was deferred to Phase 3. Fix: fold widen into Phase 1;
delete standalone Phase 3; renumber Phase 4 → Phase 2. Audit confirms
`tlb.rs:10` is the sole narrow-path site. R-020 LOW: V-F-2 SECONDARY
annotated tautological. Spec inherited via `01/02/03_PLAN.md`. No new MASTER.

## Log

[**Feature Introduce**]

Phase 1 is now atomic — nest + absolute-path `use` rewrite +
`include_str!` hop + relative-path fixups + tlb.rs:10 visibility widen in
one PR. No intermediate state compiles against a path that ceases to
exist. Phase 2 handles docs + isolation pin + full boot.

Audit this round: `rg 'pub\(in crate::arch::riscv' xemu/xcore/src` → 27
hits, 26 wide (path-stable) and 1 narrow (`mm/tlb.rs:10`). R-019's
single-line fold suffices.

[**Review Adjustments**]

- R-019 HIGH: folded tlb.rs:10 widen into Phase 1 (23rd action). Deleted
  standalone Phase 3. Renumbered Phase 4 → Phase 2. State Transition
  S0→S1→S2. Re-audited.
- R-020 LOW: option (a) — kept V-F-2 SECONDARY form, annotated as
  tautological tripwire. Option (b) would broaden match surface, risking
  false positives; PRIMARY already catches every regression class.

[**Master Compliance**]

No new MASTER; inherited archModule directives still faithful (00-M-001
no `trait Arch`; 00-M-002 topic nesting; 01-M-001 direct seams; 01-M-002
≤ 4 phases, body ≤ 300 lines; 01-M-003 `build.rs` authoritative; 01-M-004
CRITICAL seams under `arch::riscv::cpu::*` via rows #1-#4).

### Changes from Previous Round

[**Added**] Phase-1 action row for `tlb.rs:10` widen; V-F-2 SECONDARY
tautological annotation.

[**Changed**] Phase count 4 → 2 (former Phase 3 merged into Phase 1;
former Phase 4 → Phase 2). State Transition S0→S1→S2. PR list 3 PRs.
C-7 ≤ 300 (was ≤ 320).

[**Removed**] Standalone Phase 3; its dedicated `cargo test` gate
(rolled into PR1).

[**Unresolved**] None blocking. Residual risks: `make debian` timing
under difftest (Failure Flow step 6 180s escape); novel import shape
missed by PRIMARY — mitigated by `cargo check`.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 HIGH (rnd 00) | CLOSED | 22-row rewrite table carried forward; PRIMARY audits coverage. |
| Review | R-002 HIGH (rnd 00) | CLOSED — MERGED | tlb.rs:10 widen folded into Phase 1 this round; split that produced R-019 eliminated. |
| Review | R-003 HIGH (rnd 00) | CLOSED | Phase execution collapsed 4 → 2; each green-barable. |
| Review | R-004..R-010 (rnd 00) | CLOSED | Resolved rnds 01/02 (use-block rewrite, doc refresh, include_str hop, body budget). |
| Review | R-011 HIGH (rnd 01) | CLOSED | Superseded by R-016 and its fix. |
| Review | R-012/R-013/R-015 LOW (rnd 01) | CLOSED | Matrix wording, ex-Phase 4 fold, include_str row — rnd 02. |
| Review | R-014 MEDIUM (rnd 01) | CLOSED | Superseded by R-016 multiline-dotall PRIMARY gate. |
| Review | R-016 HIGH (rnd 02) | CLOSED | Rows #21/#22 added; PRIMARY upgraded to multiline-dotall (rnd 03). |
| Review | R-017 LOW (rnd 02) | CLOSED | V-F-2 SECONDARY narrowed to 0-hit expectation (rnd 03). |
| Review | R-018 LOW (rnd 02) | CLOSED | V-F-3 asserts `cargo test --test arch_isolation -- --exact arch_isolation` exits 0. |
| Review | R-019 HIGH (rnd 03) | Accepted | Folded tlb.rs:10 widen into Phase 1 (Option 1). Phase 3 deleted; phases 3→2. Audit: tlb.rs:10 is sole narrow-path site. |
| Review | R-020 LOW (rnd 03) | Accepted | V-F-2 SECONDARY annotated "tautological tripwire — 0 hits structural, not a green-bar contributor." |
| Trade-off | TR-1..TR-4 | CLOSED | Direction A, drop rename, Policy A, fold ex-Phase 4 — rnds 01/02. |
| Master | — | — | No new MASTER; inherited archModule directives (00-M-001..01-M-004) faithful. |

---

## Spec

All Spec sections (Goals G-1..G-5, Non-Goals NG-1..NG-8, Architecture
before/after tree, Invariants I-1..I-6, Data Structure, API Surface,
Constraints C-1..C-7) inherited verbatim from `01_PLAN.md` (lines 84–194)
via `02/03_PLAN.md`. Only delta: C-7 tightened — body ≤ 300 lines (was
≤ 320). Remaining Spec unchanged.

---

## Implement

### Execution Flow

[**Main Flow**]

1. **Phase 1 — Nest + path rewrite + visibility widen (atomic PR).**
   A bare nest cannot compile without path rewrites; the widen cannot be
   deferred past the nest without tripping E0433 on
   `pub(in crate::arch::riscv::mm)`. All land together.

   Structural moves (`git mv`, five topic dirs + sibling `.rs`):
   `arch/riscv/{csr, csr.rs, mm, mm.rs, trap, trap.rs, inst, inst.rs, isa}`
   → `arch/riscv/cpu/…`.

   Module-decl edits:
   - `arch/riscv/mod.rs`: reduce children to `pub mod cpu; pub mod device;`
     + refresh doc.
   - `arch/riscv/cpu/mod.rs`: add `pub mod csr; pub(in crate::arch::riscv)
     mod mm; pub mod trap; mod inst; pub mod isa;`. Rewrite lines-11–16
     use-block (R-004):
     ```rust
     use self::{
         csr::{CsrAddr, CsrFile, MStatus, Mip, PrivilegeMode},
         mm::{Mmu, Pmp},
         trap::{PendingTrap, TrapCause, interrupt::HW_IP_MASK},
     };
     use super::device::intc::{aclint::Aclint, plic::Plic};
     ```

   `include_str!` edit (R-015): `arch/riscv/cpu/isa/decoder.rs`
   `"../../../isa/instpat/riscv.instpat"` →
   `"../../../../isa/instpat/riscv.instpat"` (four `../`, from
   `cpu/isa/` back to `src/`).

   Relative-path fixups (depth change):
   - `cpu/debug.rs:4-5,45`: `super::super::csr::{…}` → `super::csr::{…}`.
   - `cpu/mm.rs:12-14`: drop redundant `cpu::` prefix.

   Visibility widen (R-019 fold, 23rd action, Policy A):
   `cpu/mm/tlb.rs:10` `pub(in crate::arch::riscv::mm) struct TlbEntry` →
   `pub(in crate::arch::riscv) struct TlbEntry`. Audit: **sole**
   narrow-path site. Other `pub(in crate::arch::riscv)` sites
   (`cpu/mod.rs:27-44`, `csr.rs:95`, `csr/ops.rs:7,16`, `trap.rs:21-55`,
   `mm/tlb.rs:58`, `mm/mmu.rs:20`) target the path-stable ancestor
   `arch::riscv` — no edit.

   Absolute-path `use` rewrite table (seeded by
   `rg -U --multiline-dotall
   '\barch::riscv::(\s*\{[^}]*)?\b(csr|mm|trap|inst|isa)\b' xemu/xcore/src`;
   22 rows, unchanged from round 03):

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
   | 21 | `arch/riscv/cpu/trap/handler.rs:5-12` (top-of-file nested-brace) | `use crate::{ arch::riscv::{ cpu::RVCore, csr::{CsrAddr, MStatus, PrivilegeMode, TrapCause}, trap::{Exception, Interrupt, PendingTrap} }, config::Word };` | `use crate::{ arch::riscv::cpu::{ RVCore, csr::{CsrAddr, MStatus, PrivilegeMode, TrapCause}, trap::{Exception, Interrupt, PendingTrap} }, config::Word };` |
   | 22 | `arch/riscv/cpu/inst/float.rs:659-666` (test nested-brace) | `use crate::{ arch::riscv::{ cpu::RVCore, csr::{CsrAddr, MStatus} }, config::CONFIG_MBASE, isa::RVReg };` | `use crate::{ arch::riscv::cpu::{ RVCore, csr::{CsrAddr, MStatus} }, config::CONFIG_MBASE, isa::RVReg };` |

   Positive sanity (audit-confirmed): `cpu/csr/ops.rs:4` uses
   `crate::arch::riscv::cpu::RVCore`; six `cpu/inst/` children use
   `super::{…}` via `cpu/inst.rs` — path-stable.

   Gate: `make fmt && make clippy && make test && make run`; PRIMARY
   `rg -U --multiline-dotall
   '\barch::riscv::(\s*\{[^}]*)?\b(csr|mm|trap|inst|isa)\b' xemu/xcore/src`
   → 0 hits; 344 tests pass.

2. **Phase 2 — Docs + `arch_isolation` pin + full boot.**
   - `arch/riscv/mod.rs` doc: replace "Flat topic layout …" with one-line
     nested-layout description.
   - `arch/riscv/cpu/mod.rs` doc: one line distinguishing `cpu/isa/`
     (encoding) from `cpu/inst/` (execution) — R-007 / TR-2.
   - `xcore/tests/arch_isolation.rs` header: refresh seam-path mentions;
     `SEAM_FILES`, `SEAM_ALLOWED_SYMBOLS`, `BUS_DEBUG_STRING_PINS` arrays
     invariant under the nest (I-3).
   - Isolation pin: `cargo test --test arch_isolation -- --exact
     arch_isolation` exits 0. Any non-zero exit, new violation line, or
     per-check count drift fails the phase.
   - Full boot: `make fmt && make clippy && make test && make run`;
     `timeout 60 make linux … | grep -q 'Welcome to Buildroot'`;
     `timeout 120 make debian … | grep -q 'debian login:'`; difftest
     corpus (archModule-03 green set) zero new divergences.

[**Failure Flow**]

1. Phase-1 compile fails (missed path or narrow `pub(in)`): patch in same
   PR; PRIMARY catches every shape; `cargo check` blocks merge.
2. `arch_isolation` red (Phase 2): triage I-1 leakage vs missed seam; fold
   fix into same PR.
3. `make linux` / `make debian` diverges: bisect between PR1 and PR2.
4. `include_str!` error: confirm exactly `"../../../../isa/instpat/riscv.instpat"`.
5. `cargo build --features isa32`: cfg-gated imports — fix in-phase.
6. `make debian` timeout: extend to 180s under difftest only; do not
   relax `debian login:` grep.

[**State Transition**]

S0 (archModule-03 landed) → S1 (Phase 1: nest + 22 `use` rewrites + 1
`include_str!` + relative-path fixups + tlb.rs:10 widen; 344 green) → S2
(Phase 2: docs + isolation assertion + `make linux` + `make debian` +
difftest; refactor complete).

### Implementation Plan

Three PRs, each independently green-barable:

- **PR1 (Phase 1)** — `git mv`s, module-decls, `include_str!` hop,
  relative fixups, 22-row table, tlb.rs:10 widen. Gate: `make fmt && make
  clippy && make test && make run`; PRIMARY `rg` → 0; SECONDARY → 0.
- **PR2 (Phase 2 docs + isolation pin)** — doc refreshes; re-run
  `arch_isolation`. Gate: `cargo test --test arch_isolation -- --exact
  arch_isolation` exits 0; `make fmt && make clippy && make test && make run`.
- **PR3 (Phase 2 boot)** — `make linux`, `make debian`, difftest corpus.

## Trade-offs

All prior TR-1..TR-4 CLOSED (rnds 01/02). No new trade-offs. R-019's fold
was a correctness choice (03_REVIEW), not a trade-off; no open advice.

## Validation

[**Unit Tests**]
- V-UT-1: `cargo test --workspace` — all 344 tests green at every phase
  boundary (Phase 1 and Phase 2).

[**Integration Tests**]
- V-IT-1: `xcore/tests/arch_isolation.rs::arch_isolation` passes unchanged
  under the nested layout. Locks in I-1, I-2, I-3. Runs as part of V-UT-1.
- V-IT-2: `make run` — default direct-boot reaches HIT GOOD TRAP.
- V-IT-3: `timeout 60 make linux 2>&1 | tee /tmp/linux.log && grep -q
  'Welcome to Buildroot' /tmp/linux.log` — Phase 2.
- V-IT-4: `timeout 120 make debian 2>&1 | tee /tmp/debian.log && grep -q
  'debian login:' /tmp/debian.log` — Phase 2.

[**Failure / Robustness Validation**]
- V-F-1: `git log --follow arch/riscv/cpu/<topic>/<file>` reaches the
  pre-nest ancestor for every moved file. Confirms `git mv` used.
- V-F-2 PRIMARY (R-016): `rg -U --multiline-dotall
  '\barch::riscv::(\s*\{[^}]*)?\b(csr|mm|trap|inst|isa)\b' xemu/xcore/src`
  — post-Phase-1 returns 0 hits. Absolute-path agnostic +
  multiline-dotall + the `\s*\{[^}]*` gap catches all three shapes
  (single-line absolute, wrapper multi-import, nested-brace). No false
  positives: post-nest form `arch::riscv::cpu::csr::…` fails the
  `\b(csr|…)\b` adjacency test.
- V-F-2 SECONDARY (R-020 annotation — **tautological tripwire only**,
  not a green-bar contributor): `rg
  'crate::arch::riscv::(csr|mm|trap|inst|isa)::' xemu/xcore/src
  --glob '!arch/**' --glob '!cpu/mod.rs' --glob '!isa/mod.rs' --glob
  '!device/mod.rs' --glob '!device/intc/mod.rs'` → 0 hits. Pattern cannot
  match post-nest (topic arms under `cpu::`); 0 hits is structural.
  Retained for historical parity; PRIMARY carries actual signal.
- V-F-3 (R-018): `cargo test --test arch_isolation -- --exact
  arch_isolation` exits 0 at each phase boundary — explicit isolation-pin
  green bar for the docs+isolation phase (Main Flow step 2, PR2).
- V-F-4: Per-phase bisection — each phase commit green on `make test`.
- V-F-5: Difftest vs QEMU/Spike on archModule-03 corpus — zero new
  divergences.

[**Edge Case Validation**]
- V-E-1: `cargo build --no-default-features --features isa32` — RV32
  build still compiles.
- V-E-2: `cargo clippy --all-targets -- -D warnings` — no new warnings.
- V-E-3: `cargo fmt --check` — formatting invariant under moves.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|---|---|
| G-1 nest under `cpu/` | V-IT-1; V-F-1; V-F-2 PRIMARY; V-UT-1 |
| G-2 `device/` sibling | V-IT-1 |
| G-3 structural disambiguation | V-UT-1; Phase 2 doc |
| G-4 `instpat` at crate root | V-UT-1; V-E-1 |
| G-5 byte-identical | V-IT-2..V-IT-4; V-F-5; V-UT-1 |
| C-1 phase green | V-F-4 |
| C-2 `git mv` only | V-F-1 |
| C-3 byte-identical | V-F-5; V-IT-2..V-IT-4 |
| C-4 no new deps | Cargo.toml diff |
| C-5 `include_str!` | V-UT-1; `cargo build` PR1 |
| C-6 pest grammar path | `cargo build` post-PR1 |
| C-7 ≤ 300 lines | Plan body self-review |
| I-1..I-3 seam invariants | V-IT-1; V-F-3 |
| I-4 history | V-F-1 |
| I-5/I-6 build.rs / no `trait Arch` | Diff review |
