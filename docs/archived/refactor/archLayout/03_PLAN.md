# `archLayout` PLAN `03`

> Status: Draft
> Feature: `archLayout`
> Iteration: `03`
> Owner: Executor
> Depends on:
> - Previous Plan: `02_PLAN.md`
> - Review: `02_REVIEW.md`
> - Master Directive: `02_MASTER.md` (blank — user skipped)

---

## Summary

Fourth iteration. Round-02 was approved with one HIGH (R-016 — two multi-line
nested-brace sites missing from the rewrite table; line-oriented audit regex
cannot span them) and two LOW (R-017 SECONDARY-gate wording bug; R-018
thin Phase-3 assertion). This revision (a) replaces audit + PRIMARY gate
with a multiline-dotall regex that crosses line boundaries; (b) adds rows
#21/#22 for the two cited sites; (c) narrows V-F-2 SECONDARY so expected
result is unambiguously 0 hits; (d) names the exact `arch_isolation`
pass/fail assertion. Spec (Goals/Non-Goals/Architecture/Invariants/API
Surface/Constraints) inherited verbatim from `01_PLAN.md` via `02_PLAN.md`.
No new MASTER this round.

## Log

[**Feature Introduce**]

Round 03 is a final convergence pass. No architectural change vs round-02:
Direction A, rename dropped, Policy A, four phases — all stand. Audit
rationale: the round-02 line-oriented regex
`crate::arch::riscv::(csr|mm|trap|inst|isa)::` only matches when the topic
arm and the `crate::` prefix sit on the same line. Multi-line nested-brace
imports (`use crate::{ arch::riscv::{ cpu::…, csr::{…}, trap::{…} } }`)
split the prefix and topic arms across lines. The new regex
`\barch::riscv::(\s*\{[^}]*)?\b(csr|mm|trap|inst|isa)\b` run with
multiline-dotall is absolute-path agnostic (matches `arch::riscv::csr`
regardless of whether it is inside a `crate::{ … }` grouping) and the
optional `\s*\{[^}]*` arm spans the nested-brace gap, so all three shapes
(single-line absolute, wrapper multi-import, nested-brace) are caught
without false positives against the correct post-nest `cpu::csr::…` form.

[**Review Adjustments**]

- R-016 HIGH (recurring R-011): rows #21 (`cpu/trap/handler.rs:5-12`) and
  #22 (`cpu/inst/float.rs:659-666`) added; PRIMARY gate regex replaced with
  the multiline-dotall form. Re-audit confirms no further sites.
- R-017 LOW: V-F-2 SECONDARY narrowed to exclude the four seam files as
  well as `arch/**`, so the expected result is 0 hits (wording trap gone).
- R-018 LOW: Phase-3 green bar now asserts `cargo test --test
  arch_isolation -- --exact arch_isolation` exits 0.

[**Master Compliance**]

No new MASTER this round (user skipped `02_MASTER.md`). Inherited archModule
directives unchanged: 00-M-001 (no `trait Arch`), 00-M-002 (topic nesting),
01-M-001 (direct seams), 01-M-002 (four phases, body ≤ 320 lines), 01-M-003
(`build.rs` authoritative), 01-M-004 CRITICAL (seams under
`arch::riscv::cpu::*`).

### Changes from Previous Round

[**Added**] Rows #21/#22; audit rationale; `arch_isolation` assertion.

[**Changed**] Audit + PRIMARY gate replaced with multiline-dotall form;
SECONDARY narrowed to 0-hit expectation; body ≤ 320 lines (was 350).

[**Removed**] Nothing.

[**Unresolved**] None blocking. Residual risk: `make debian` boot time
under difftest (documented in Failure Flow step 6).

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001..R-003 HIGH (round 00) | CLOSED | Table (22 sites + `include_str!`), Policy A, four phases — all carried forward unchanged. |
| Review | R-004..R-010 (round 00) | CLOSED | Resolved inline in round 01. Body budget tightened to ≤ 320 lines. |
| Review | R-011 HIGH (round 01) | CLOSED | Resolved round 02; R-016 continues the audit chain. |
| Review | R-012, R-013, R-015 LOW (round 01) | CLOSED | Matrix wording, Phase-4 fold, `include_str!` row — all resolved round 02. |
| Review | R-014 MEDIUM (round 01) | CLOSED | Superseded this round by the R-016 tighter regex (still PRIMARY). |
| Review | R-016 HIGH (round 02) | Accepted | Audit + PRIMARY gate replaced with `rg -U --multiline-dotall '\barch::riscv::(\s*\{[^}]*)?\b(csr\|mm\|trap\|inst\|isa)\b' xemu/xcore/src`; rows #21 (`cpu/trap/handler.rs:5-12`) and #22 (`cpu/inst/float.rs:659-666`) added. |
| Review | R-017 LOW (round 02) | Accepted | V-F-2 SECONDARY narrowed with seam-file excludes so expected result is 0 hits. |
| Review | R-018 LOW (round 02) | Accepted | Phase 3 green bar asserts `cargo test --test arch_isolation -- --exact arch_isolation` exits 0. |
| Trade-off | TR-1..TR-4 | CLOSED | Direction A, drop rename, Policy A, fold ex-Phase 4 — all closed rounds 01/02. |
| Master | — | — | No new MASTER this round. Inherited archModule directives (00-M-001..01-M-004) applied as in Master Compliance. |

---

## Spec

All Spec sections (Goals G-1..G-5, Non-Goals NG-1..NG-8, Architecture
before/after tree, Invariants I-1..I-6, Data Structure, API Surface,
Constraints C-1..C-7) are inherited verbatim from `01_PLAN.md` Spec
(lines 84–195) as carried through `02_PLAN.md`. The following deltas apply:

[**Constraints — round-03 budget**]

- C-7 tightened: plan body ≤ 320 lines (was ≤ 350 in round 02).

Remaining Spec content unchanged.

---

## Implement

### Execution Flow

[**Main Flow**]

1. **Phase 1+2 — Nest + `use`-path + `include_str!` rewrite (one PR).**
   A bare nest cannot compile without path rewrites, so the nest and every
   path edit land in the same commit. Audit regex now crosses line
   boundaries (see Validation V-F-2 PRIMARY).

   Structural moves (`git mv`, five topic dirs + sibling `.rs`):
   - `arch/riscv/{csr, csr.rs, mm, mm.rs, trap, trap.rs, inst, inst.rs, isa}`
     → `arch/riscv/cpu/…`.

   Module-declaration edits:
   - `arch/riscv/mod.rs`: children reduce to `pub mod cpu; pub mod device;`.
     Refresh doc-comment (old text says "flat topic layout").
   - `arch/riscv/cpu/mod.rs`: add `pub mod csr; pub(in crate::arch::riscv)
     mod mm; pub mod trap; mod inst; pub mod isa;`. Rewrite the
     lines-11–16 use-block (R-004):
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
     `include_str!("../../../../isa/instpat/riscv.instpat")` (four `../`,
     from `src/arch/riscv/cpu/isa/` back to `src/`, then `isa/instpat/`).

   Relative-path fixups (depth change only):
   - `arch/riscv/cpu/debug.rs:4-5`: `super::super::csr::{…}` →
     `super::csr::{…}`.
   - `arch/riscv/cpu/debug.rs:45`: `super::super::csr::DIFFTEST_CSRS` →
     `super::csr::DIFFTEST_CSRS`.
   - `arch/riscv/cpu/mm.rs:12-14`: drop redundant `cpu::` prefix
     (`super::{RVCore, csr::{…}, trap::{…}}`).

   Absolute-path `use` rewrite table (seeded by
   `rg -U --multiline-dotall
   '\barch::riscv::(\s*\{[^}]*)?\b(csr|mm|trap|inst|isa)\b' xemu/xcore/src`;
   22 rows total, audited this round):

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

   Positive sanity (no edit, confirmed by audit): `cpu/csr/ops.rs:4` uses
   `crate::arch::riscv::cpu::RVCore` (path-stable); six of the seven
   `cpu/inst/` children (`base, float, mul, privileged, zicsr, …`) use
   `super::{RVCore, rv64_*}` via `cpu/inst.rs`, path-stable after nest.

   Gate: PRIMARY `rg -U --multiline-dotall
   '\barch::riscv::(\s*\{[^}]*)?\b(csr|mm|trap|inst|isa)\b' xemu/xcore/src`
   — post-rewrite returns 0 hits; `cargo test --workspace` → 344 pass;
   `make fmt && make clippy`.

2. **Phase 3 — Visibility widen (Policy A, one edit).**
   `arch/riscv/cpu/mm/tlb.rs:10`: `pub(in crate::arch::riscv::mm) struct
   TlbEntry` → `pub(in crate::arch::riscv) struct TlbEntry`. No other edits
   (confirmed by `rg 'pub\(in crate::arch::riscv' xemu/xcore/src` — `tlb.rs:58`
   and `mmu.rs:20` already wide). Gate: `cargo test --workspace` → 344 green.

3. **Phase 4 — Docs + `arch_isolation` pin + full boot verification.**
   - `arch/riscv/mod.rs` doc: replace "Flat topic layout …" with a one-line
     description of the nested layout.
   - `arch/riscv/cpu/mod.rs` doc: add one line distinguishing `cpu/isa/`
     (encoding) from `cpu/inst/` (execution) — R-007 / TR-2 closure.
   - `xcore/tests/arch_isolation.rs` header doc-comment: refresh mentions
     of seam paths; `SEAM_FILES`, `SEAM_ALLOWED_SYMBOLS`,
     `BUS_DEBUG_STRING_PINS` arrays are invariant under the nest (I-3).
   - Isolation pin (R-018 assertion): `cargo test --test arch_isolation --
     --exact arch_isolation` exits 0. Any non-zero exit, any new violation
     line, or any per-check count drift signals a seam-boundary shift and
     fails the phase.
   - Full boot gate:
     - `make fmt && make clippy && make test && make run`.
     - `timeout 60 make linux 2>&1 | tee /tmp/linux.log && grep -q
       'Welcome to Buildroot' /tmp/linux.log`.
     - `timeout 120 make debian 2>&1 | tee /tmp/debian.log && grep -q
       'debian login:' /tmp/debian.log`.
     - Difftest regression corpus (archModule-03 green set): zero new
       divergences.

[**Failure Flow**]

1. Phase-1+2 compile fails (missed path site): patch in same commit;
   PRIMARY gate (multiline-dotall) catches every shape.
2. `arch_isolation` red: triage real I-1 leakage vs missed seam edit; fold
   seam fix into the same PR.
3. `make linux` / `make debian` diverges: bisect across the four phase
   commits.
4. `include_str!` compile error: verify argument is exactly
   `"../../../../isa/instpat/riscv.instpat"` (four `../`).
5. `cargo build --features isa32` fails: cfg-gated imports in moved files —
   fix in-phase.
6. `make debian` timeout: extend to 180s under difftest only; do not relax
   the `debian login:` success-marker grep.

[**State Transition**]

S0 (archModule-03 landed) → S1 (Phase 1+2: nest + 22 `use` rewrites + 1
`include_str!` + relative-path fixups; 344 green) → S2 (Phase 3: `tlb.rs:10`
widened; 344 green) → S3 (Phase 4: docs + isolation assertion + `make linux`
+ `make debian` + difftest; refactor complete).

### Implementation Plan

Four PRs, each independently green-barable:

- **PR1 (Phase 1+2)** — `git mv`s; module-decl moves; `include_str!` hop;
  relative-path fixups; 22-row table. Gate: `make fmt && make clippy &&
  make test && make run`; PRIMARY `rg` → 0 hits; SECONDARY `rg` → 0 hits.
- **PR2 (Phase 3)** — one-line edit to `cpu/mm/tlb.rs:10`. Gate:
  `cargo test --workspace`.
- **PR3 (Phase 4 docs + isolation pin)** — doc-comment refreshes; re-run
  `arch_isolation`. Gate: `cargo test --test arch_isolation -- --exact
  arch_isolation` exits 0; `make fmt && make clippy && make test && make run`.
- **PR4 (Phase 4 boot)** — `make linux`, `make debian`, difftest corpus.
  Deliverable: refactor-complete signal.

## Trade-offs

All prior TR-1..TR-4 CLOSED (Direction A, drop rename, Policy A, fold
ex-Phase 4; rounds 01/02). No new trade-offs this round.

## Validation

[**Unit Tests**]
- V-UT-1: `cargo test --workspace` — all 344 tests green at every phase
  boundary.

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
- V-F-2 PRIMARY (R-016): `rg -U --multiline-dotall
  '\barch::riscv::(\s*\{[^}]*)?\b(csr|mm|trap|inst|isa)\b' xemu/xcore/src`
  — post-Phase-1+2 returns 0 hits. Absolute-path agnostic + multiline-dotall
  + the `\s*\{[^}]*` gap catches all three shapes (single-line absolute,
  wrapper multi-import, nested-brace). No false positives: post-nest form
  `arch::riscv::cpu::csr::…` fails the `\b(csr|…)\b` adjacency test.
- V-F-2 SECONDARY (R-017): `rg
  'crate::arch::riscv::(csr|mm|trap|inst|isa)::' xemu/xcore/src --glob
  '!arch/**' --glob '!cpu/mod.rs' --glob '!isa/mod.rs' --glob
  '!device/mod.rs' --glob '!device/intc/mod.rs'` — returns 0 hits.
- V-F-3 (R-018): `cargo test --test arch_isolation -- --exact
  arch_isolation` exits 0 at each phase boundary — explicit isolation-pin
  green bar for the docs+isolation phase (Main Flow step 3, PR3).
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
| G-3 structural disambiguation | V-UT-1 + Phase 4 doc |
| G-4 `instpat` at crate root | V-UT-1 (decoder loads); V-E-1 |
| G-5 byte-identical behaviour | V-IT-2, V-IT-3, V-IT-4; V-F-5; V-UT-1 |
| C-1 every phase green | V-F-4 |
| C-2 `git mv` only | V-F-1 |
| C-3 byte-identical | V-F-5; V-IT-2..V-IT-4 |
| C-4 no new deps | Cargo.toml diff |
| C-5 `include_str!` correct | V-UT-1 (decoder tests); `cargo build` on PR1 |
| C-6 pest grammar path | `cargo build` post-PR1 |
| C-7 ≤ 320 lines | Plan body self-review |
| I-1, I-2, I-3 seam invariants | V-IT-1; V-F-3 (explicit assertion) |
| I-4 history | V-F-1 |
| I-5 build.rs authoritative | Diff review |
| I-6 no `trait Arch` | Diff review |
