# `archLayout` PLAN `00`

> Status: Draft
> Feature: `archLayout`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

The `archModule` refactor (rounds 00–03) achieved its topic-organised goal, but the
resulting flat layout under `arch/riscv/{cpu, csr, mm, trap, inst, isa, device}` is
structurally unbalanced once a reader looks at the tree: `cpu/` is a thin shell
(`context.rs`, `debug.rs`, `mod.rs`) while its conceptual peers (CSRs, MMU/PMP,
trap machinery, instruction decode, instruction execution) sit as top-level
siblings. The `isa/` (encoding) vs `inst/` (execution) split aggravates the
confusion — both names point at "instructions". This plan adopts **Direction A**:
nest all CPU-internal topics under `arch/riscv/cpu/`, keep `arch/riscv/device/`
as a sibling of `cpu/` (devices are not CPU-internal), and rename `inst/` →
`executor/` to eliminate the `isa/` vs `inst/` naming collision. The refactor is
pure reorganisation: no behaviour change, no new dependencies, git history
preserved via `git mv`, and every test/boot/difftest gate must stay green at
every phase boundary.

## Log {None in 00_PLAN}

[**Feature Introduce**]

First iteration — no prior PLAN, REVIEW, or MASTER exists. This plan proposes a
structural reorganisation of `arch/riscv/` that groups the six CPU-internal
topic modules (`csr`, `mm`, `trap`, `inst`, `isa`) under `cpu/`, renames `inst/`
to `executor/` to resolve the `isa/` vs `inst/` semantic collision, and updates
the four seam files and the `arch_isolation` integration test whose re-export
paths shift.

[**Review Adjustments**]

None in `00_PLAN`.

[**Master Compliance**]

The four binding directives inherited from `archModule` (00-M-001 no global
`trait Arch`, 00-M-002 topic organisation, 01-M-001 no `selected` alias,
01-M-003 no redundant arch-validity checks, 01-M-004 CRITICAL thin seam) are all
preserved. 00-M-002 in particular is still honoured: topic organisation does
not mandate a flat shape — nesting `{csr, mm, trap, inst→executor, isa}` under
`cpu/` is still by topic, only grouped by the superordinate "CPU-internal"
concern instead of flattened at the arch root. 01-M-004 is **strengthened**:
seam files re-export from a narrower subtree (`crate::arch::riscv::cpu::*`
instead of five distinct topic roots), which reduces the surface area the
`arch_isolation` test must police.

### Changes from Previous Round

[**Added**]

- New directory layout (`arch/riscv/cpu/{csr, mm, trap, executor, isa}`).
- New seam re-export paths in `cpu/mod.rs`, `isa/mod.rs`, `device/mod.rs`,
  `device/intc/mod.rs`.
- `inst/` → `executor/` rename.
- Four-hop `include_str!` path adjustment in `arch/riscv/cpu/isa/decoder.rs`
  (was three hops).

[**Changed**]

- `arch_isolation.rs` `SEAM_ALLOWED_SYMBOLS` vocabulary is unchanged (symbols
  keep their names), but the arch paths the seam files reference shift to
  `crate::arch::riscv::cpu::*`.

[**Removed**]

- Flat arch-root children `arch/riscv/{csr, mm, trap, inst, isa}` (now nested
  under `cpu/`).

[**Unresolved**]

- Whether to rename `inst/` → `executor/` or keep the name. Plan selects
  `executor/` (see TR-2); reviewer may prefer `semantics/` or keeping `inst/`.
- Whether `mm` should stay `pub(crate)` after the nest (it is currently reached
  from `inst/atomic.rs` via `crate::arch::riscv::mm::MemOp`). Plan adjusts the
  one call site to `super::super::mm::MemOp` so `mm` can become
  `pub(in crate::arch::riscv)` like its peers; see TR-3.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | — | — | None — first iteration. |
| Master | 00-M-001 | Applied | No global `trait Arch` introduced; the refactor only moves files and updates `super::` / `crate::` paths. |
| Master | 00-M-002 | Applied | Nesting by CPU-concern is still topic organisation — see Master Compliance note above. |
| Master | 01-M-001 | Applied | Seam aliases remain direct-path `pub type … = crate::arch::riscv::cpu::…` form; no `selected` indirection introduced. |
| Master | 01-M-003 | Applied | No new arch-validity check; `build.rs` remains authoritative. |
| Master | 01-M-004 | Applied | Seam files keep their thin-alias shape; narrower subtree is re-exported. Surface strictly shrinks. |

---

## Spec {Core specification}

[**Goals**]

- G-1: Nest the five CPU-internal topic modules (`csr`, `mm`, `trap`, `inst`,
  `isa`) under `arch/riscv/cpu/`, preserving git history via `git mv`.
- G-2: Keep `arch/riscv/device/` as a sibling of `cpu/`; devices are not
  CPU-internal and `device/intc/` already lives behind its own seam
  (`crate::device::intc`).
- G-3: Resolve the `isa/` (encoding) vs `inst/` (execution) naming collision by
  renaming `arch/riscv/inst/` → `arch/riscv/cpu/executor/` (see TR-2).
- G-4: Leave `xcore/src/isa/` at the crate root — it holds the neutral `pub use`
  seam plus the `instpat/` pest grammar (ISA-defining data). The existing
  `arch/riscv/isa/decoder.rs` → `../../../isa/instpat/riscv.instpat`
  `include_str!` path becomes a four-hop path after the nest.
- G-5: Behaviour is byte-identical. All 344 tests (343 unit/integration from
  archModule-03 + 1 `arch_isolation` integration test) must pass at every phase
  boundary; `make linux` boots to shell; `make debian` boots Debian 13 Trixie
  to shell; difftest vs QEMU/Spike shows zero divergence.

- NG-1: Do NOT move `xcore/src/isa/instpat/` (the pest grammar is neutral data
  that belongs at the crate root, not inside an arch backend).
- NG-2: Do NOT fold `arch/riscv/device/` into `arch/riscv/cpu/`.
- NG-3: Do NOT touch `arch/loongarch/` (the current stub is already minimal;
  when loongarch implementation lands it can adopt the same nested shape
  naturally).
- NG-4: Do NOT modify landed plan/review/master documents in `docs/fix/archModule/`.
- NG-5: Do NOT change any public API visible outside `xcore` (the xemu CLI sees
  `xcore::CPU`, `xcore::cpu::BootConfig`, etc. — none of those paths change).
- NG-6: Do NOT introduce new dependencies or new `cfg` flags.
- NG-7: Do NOT reintroduce the `arch/riscv/isa/` → `xcore/src/isa/riscv/`
  parallel-tree pattern (Direction B, rejected in TR-1).

[**Architecture**]

Before (archModule-03 landed):

```
xemu/xcore/src/
├── arch/
│   ├── mod.rs
│   └── riscv/
│       ├── mod.rs          (pub mod cpu; pub mod csr; pub mod device;
│       │                    mod inst; pub mod isa;
│       │                    pub(crate) mod mm; pub mod trap;)
│       ├── cpu/            (context.rs, debug.rs, mod.rs — RVCore)
│       ├── csr/            (mip.rs, mstatus.rs, ops.rs, privilege.rs) + csr.rs
│       ├── device/         (intc/{aclint, plic}/, mod.rs)
│       ├── inst/           (atomic.rs, base.rs, compressed.rs, float.rs,
│       │                    mul.rs, privileged.rs, zicsr.rs) + inst.rs
│       ├── isa/            (decoder.rs, inst.rs, reg.rs, mod.rs)
│       ├── mm/             (mmu.rs, pmp.rs, tlb.rs) + mm.rs
│       └── trap/           (cause.rs, exception.rs, handler.rs, interrupt.rs)
│                            + trap.rs
├── cpu/                    (seam: pub type Core = riscv::cpu::RVCore;)
├── device/                 (seam: intc/mod.rs re-exports Aclint, Plic;
│                            mod.rs re-exports SSIP/MSIP/…)
└── isa/                    (seam: pub use riscv::isa::{…}; + instpat/ pest)
```

After (target):

```
xemu/xcore/src/
├── arch/
│   ├── mod.rs              (unchanged)
│   └── riscv/
│       ├── mod.rs          (pub mod cpu; pub mod device;)
│       ├── cpu/
│       │   ├── mod.rs      (RVCore — unchanged code; super-path imports rewritten)
│       │   ├── context.rs  (unchanged)
│       │   ├── debug.rs    (use super::super::… → use super::… adjustments)
│       │   ├── csr/        (mip.rs, mstatus.rs, ops.rs, privilege.rs)
│       │   ├── csr.rs
│       │   ├── executor/   (atomic.rs, base.rs, compressed.rs, float.rs,
│       │   │                mul.rs, privileged.rs, zicsr.rs)  [renamed from inst/]
│       │   ├── executor.rs [renamed from inst.rs]
│       │   ├── isa/        (decoder.rs, inst.rs, reg.rs)
│       │   ├── isa.rs      [renamed from isa/mod.rs into the mod-in-file style
│       │   │                already used by siblings: csr.rs / mm.rs / trap.rs /
│       │   │                inst.rs — see TR-3]
│       │   ├── mm/         (mmu.rs, pmp.rs, tlb.rs)
│       │   ├── mm.rs
│       │   ├── trap/       (cause.rs, exception.rs, handler.rs, interrupt.rs)
│       │   └── trap.rs
│       └── device/         (unchanged — still a sibling of cpu/)
├── cpu/                    (seam: pub type Core = riscv::cpu::RVCore;
│                            pub type CoreContext = riscv::cpu::context::RVCoreContext;
│                            pub type PendingTrap = riscv::cpu::trap::PendingTrap;)
├── device/                 (seam: intc/mod.rs re-exports riscv::device::intc::…;
│                            mod.rs re-exports riscv::cpu::trap::interrupt::…)
└── isa/                    (seam: pub use riscv::cpu::isa::{…}; + instpat/ pest)
```

[**Invariants**]

- I-1: Outside `arch/` and outside the five seam files in `arch_isolation.rs`
  `SEAM_FILES`, **zero** source lines reference `crate::arch::riscv::` or
  `crate::arch::loongarch::`. Inherited verbatim from archModule-03.
- I-2: After the nest, the **only** arch subpaths the seam files name are under
  `crate::arch::riscv::cpu::*` (for CPU/ISA/trap vocabulary) and
  `crate::arch::riscv::device::*` (for interrupt controllers). No seam file
  references the old flat paths `crate::arch::riscv::{csr, mm, trap, inst, isa}::`.
- I-3: The `arch_isolation` integration test's `SEAM_ALLOWED_SYMBOLS` remains a
  name-level allow-list. Symbol names (`Core`, `Aclint`, `DECODER`, etc.) do
  not change; only the arch paths behind them shift.
- I-4: `git log --follow` from each moved file reaches its pre-nest ancestor
  (because `git mv` was used).
- I-5: The `build.rs`-emitted `cfg(riscv)` / `cfg(loongarch)` / `cfg(isa32)` /
  `cfg(isa64)` flags remain the sole source of truth for arch selection (01-M-003).
- I-6: No new `trait Arch` is introduced (00-M-001).

[**Data Structure**]

No structural changes. Every struct, enum, trait, and macro keeps its current
shape and visibility, with **one** visibility relaxation:

```rust
// Before (arch/riscv/mod.rs):
pub(crate) mod mm;

// After (arch/riscv/cpu/mod.rs adds mm in its own nest):
pub(in crate::arch::riscv) mod mm;
```

Rationale: the only non-arch caller of `mm::MemOp` is `inst/atomic.rs`, which
is being renamed to `cpu/executor/atomic.rs` and thus lives **inside** the same
`arch::riscv` subtree — so `pub(in crate::arch::riscv)` is sufficient. This
matches `trap`'s existing `pub(in crate::arch::riscv)` pattern.

[**API Surface**]

Seam file diffs. Only path roots change; symbol names are identical.

```rust
// xcore/src/cpu/mod.rs — type aliases
// Before:
#[cfg(riscv)] pub type Core        = crate::arch::riscv::cpu::RVCore;
#[cfg(riscv)] pub type CoreContext = crate::arch::riscv::cpu::context::RVCoreContext;
#[cfg(riscv)] pub type PendingTrap = crate::arch::riscv::trap::PendingTrap;
// After (only PendingTrap path changes):
#[cfg(riscv)] pub type Core        = crate::arch::riscv::cpu::RVCore;
#[cfg(riscv)] pub type CoreContext = crate::arch::riscv::cpu::context::RVCoreContext;
#[cfg(riscv)] pub type PendingTrap = crate::arch::riscv::cpu::trap::PendingTrap;
```

```rust
// xcore/src/isa/mod.rs — re-exports
// Before:
#[cfg(riscv)] pub use crate::arch::riscv::isa::{DECODER, DecodedInst, IMG, InstFormat, InstKind, RVReg};
// After:
#[cfg(riscv)] pub use crate::arch::riscv::cpu::isa::{DECODER, DecodedInst, IMG, InstFormat, InstKind, RVReg};
```

```rust
// xcore/src/device/mod.rs — mip bit re-export
// Before:
#[cfg(riscv)] pub use crate::arch::riscv::trap::interrupt::{HW_IP_MASK, MEIP, MSIP, MTIP, SEIP, SSIP, STIP};
// After:
#[cfg(riscv)] pub use crate::arch::riscv::cpu::trap::interrupt::{HW_IP_MASK, MEIP, MSIP, MTIP, SEIP, SSIP, STIP};
```

```rust
// xcore/src/device/intc/mod.rs — ACLINT / PLIC re-export
// Before:
#[cfg(riscv)] pub use crate::arch::riscv::device::intc::{Aclint, Plic};
// After: UNCHANGED (device/ did not move).
```

No other public surface changes.

[**Constraints**]

- C-1: Every phase must leave the tree `cargo build`-, `cargo test`-, and
  `cargo clippy`-clean. No intermediate "wip" commit may break `make test`.
- C-2: `git mv` is the only mechanism used to relocate files — no copy-and-delete.
- C-3: Behaviour is byte-identical. Difftest vs QEMU/Spike must show zero new
  divergences. `make linux` and `make debian` boots must reach the same shell
  prompt as pre-refactor.
- C-4: No new dependencies (NG-6). The integration test `arch_isolation` stays
  on its current `std::fs`-only text-level check.
- C-5: The `include_str!("../../../isa/instpat/riscv.instpat")` path in
  `isa/decoder.rs` must be updated to `"../../../../isa/instpat/riscv.instpat"`
  after decoder.rs moves from `arch/riscv/isa/decoder.rs` to
  `arch/riscv/cpu/isa/decoder.rs` (one additional `..` hop).
- C-6: `pest_derive::Parser` `#[grammar = "src/isa/instpat/riscv.pest"]` is
  `CARGO_MANIFEST_DIR`-relative, not file-relative; it does **not** need
  adjustment (verified against the current annotation in `isa/decoder.rs`).
- C-7: The `arch_isolation` integration test must pass after every phase. Its
  allow-list and pinned debug-string counts are updated in the same commit that
  changes the underlying paths.

---

## Implement {detail design}

### Execution Flow

[**Main Flow**]

1. **Phase 1 — Nest `csr`, `mm`, `trap` under `cpu/`.**
   These three have no cross-topic naming collisions and the smallest import
   blast radius.
   - `git mv arch/riscv/csr arch/riscv/cpu/csr`
   - `git mv arch/riscv/csr.rs arch/riscv/cpu/csr.rs`
   - `git mv arch/riscv/mm arch/riscv/cpu/mm`
   - `git mv arch/riscv/mm.rs arch/riscv/cpu/mm.rs`
   - `git mv arch/riscv/trap arch/riscv/cpu/trap`
   - `git mv arch/riscv/trap.rs arch/riscv/cpu/trap.rs`
   - Edit `arch/riscv/mod.rs`: drop `pub mod csr;`, `pub(crate) mod mm;`,
     `pub mod trap;` lines.
   - Edit `arch/riscv/cpu/mod.rs`:
     - Add `pub mod csr;`, `pub mod mm;`, `pub mod trap;` at the top.
     - Visibility: change `mm` from `pub(crate)` to
       `pub(in crate::arch::riscv)` (see Data Structure).
     - Rewrite the `use super::{ csr::…, mm::…, trap::… };` block into
       `use self::{ csr::…, mm::…, trap::… };` (or drop the `super::` since
       these are now children).
   - Edit `arch/riscv/cpu/debug.rs`: `use super::super::csr::{CsrAddr, find_desc};`
     → `use super::csr::{CsrAddr, find_desc};`; same pattern for `DIFFTEST_CSRS`.
   - Edit `arch/riscv/cpu/csr.rs`: `use super::trap::Interrupt as Irq;` is
     unchanged — `super` of `cpu::csr` is `cpu`, and `cpu::trap` exists after
     the move.
   - Edit `arch/riscv/cpu/mm.rs`: `use super::{cpu::RVCore, csr::…, trap::…};`
     → `use super::{RVCore, csr::…, trap::…};` (the `super` of `cpu::mm` is
     `cpu`, so `RVCore` is directly in scope via `cpu::mod.rs`).
   - Edit `arch/riscv/cpu/trap.rs`: `crate::arch::riscv::cpu::RVCore` path is
     unchanged (absolute path still resolves).
   - Edit `arch/riscv/cpu/csr/ops.rs`: `crate::arch::riscv::cpu::RVCore` is
     unchanged.
   - Edit seam `xcore/src/cpu/mod.rs`: `PendingTrap` alias path
     `trap::PendingTrap` → `cpu::trap::PendingTrap`.
   - Edit seam `xcore/src/device/mod.rs`: `trap::interrupt::{…}` path →
     `cpu::trap::interrupt::{…}`.
   - Update `xcore/tests/arch_isolation.rs` if any seam-file path references
     need to be widened (no: `SEAM_FILES` already lists the five seam paths and
     they are unchanged).
   - Run `make fmt && make clippy && make test` — must be green.

2. **Phase 2 — Nest `isa/` (encoding) under `cpu/` and fix `include_str!`.**
   - `git mv arch/riscv/isa arch/riscv/cpu/isa`
   - Edit `arch/riscv/cpu/isa/decoder.rs`: change
     `include_str!("../../../isa/instpat/riscv.instpat")` →
     `include_str!("../../../../isa/instpat/riscv.instpat")`. Verify
     `#[grammar = "src/isa/instpat/riscv.pest"]` is `CARGO_MANIFEST_DIR`-relative
     and requires no change.
   - Edit `arch/riscv/mod.rs`: drop `pub mod isa;`.
   - Edit `arch/riscv/cpu/mod.rs`: add `pub mod isa;`.
   - Edit seam `xcore/src/isa/mod.rs`: path `arch::riscv::isa::{…}` →
     `arch::riscv::cpu::isa::{…}`.
   - Run `make fmt && make clippy && make test`.

3. **Phase 3 — Rename `inst/` → `executor/` and nest under `cpu/`.**
   - `git mv arch/riscv/inst arch/riscv/cpu/executor`
   - `git mv arch/riscv/inst.rs arch/riscv/cpu/executor.rs`
   - Edit `arch/riscv/cpu/executor.rs`:
     - `use super::cpu::RVCore;` → `use super::RVCore;`
     - `use crate::{…, isa::{DecodedInst, InstKind, RVReg}};` — unchanged
       (neutral `crate::isa::` seam is path-stable).
   - Edit `arch/riscv/cpu/executor/atomic.rs`: `use crate::{arch::riscv::mm::…}`
     → `use super::super::mm::…;` (shorter and obeys I-2 — no non-seam
     file outside `cpu/` references the `crate::arch::riscv::` prefix, and
     non-seam files inside `arch/` are permitted by `arch_isolation` to
     reference sibling modules via `super::` chains).
   - Edit `arch/riscv/mod.rs`: drop `mod inst;`.
   - Edit `arch/riscv/cpu/mod.rs`: add `mod executor;` (remains private, matching
     the current private visibility of `inst`).
   - Update the `rv_inst_table!` macro call site in `cpu/executor.rs` only if it
     references a module path (it does not — `rv_inst_table!` is a top-level
     macro expansion).
   - Run `make fmt && make clippy && make test`.

4. **Phase 4 — Docs + `arch_isolation` sanity sweep.**
   - Update the doc-comment in `arch/riscv/mod.rs` to reflect the new nested
     layout (the previous comment mentioned flat topic layout).
   - Review `arch_isolation.rs`: confirm `SEAM_FILES` list is unchanged, confirm
     `SEAM_ALLOWED_SYMBOLS` is unchanged, confirm `BUS_DEBUG_STRING_PINS` count
     is unchanged (device/bus.rs was not touched). Re-run the test to confirm
     it passes against the new layout.
   - Run full validation gate: `make fmt && make clippy && make test && make run`.
   - Run `make linux` and `make debian`; difftest regression pass.

[**Failure Flow**]

1. If `cargo build` fails after a Phase N rename:
   - Do NOT amend landed phases. Diagnose the broken import and fix it in the
     current phase's commit.
   - The most common failure is a `super::X` chain broken because the topic
     moved one level deeper. Fix: add one `super::` hop, not by introducing a
     new re-export.
2. If `arch_isolation` test fails because a new path pattern shows up:
   - The violation message will name the file and line. Triage: is the line a
     **new** cross-arch leakage (stop and re-plan) or a **renamed** re-export
     from a known seam file (update the commit's seam file edit to land in the
     same commit)?
3. If `make linux` / `make debian` diverges from baseline:
   - Stop. Behaviour must be byte-identical. Bisect with `git bisect` across
     the phase commits to find the rename that introduced the regression.
4. If `include_str!` fails at compile time in Phase 2:
   - Verify the hop count: `arch/riscv/cpu/isa/decoder.rs` → `../../../../` →
     `xcore/src/` → `isa/instpat/riscv.instpat`. Four `..` hops.

[**State Transition**]

- S0 (pre-refactor) → S1: `csr`, `mm`, `trap` nested under `cpu/`. Seam files
  `cpu/mod.rs`, `device/mod.rs` updated.
- S1 → S2: `isa/` nested under `cpu/`. `include_str!` path extended. Seam file
  `isa/mod.rs` updated.
- S2 → S3: `inst/` renamed and nested as `cpu/executor/`.
- S3 → S4: Doc-comments and `arch_isolation` test finalised; full validation
  gate green.

### Implementation Plan

[**Phase 1 — `csr` / `mm` / `trap` nest**]

One PR (or four-commit stack). Smallest-risk starting phase because these three
topics have no name conflict and the fewest absolute-path callers. Deliverable:
- `arch/riscv/cpu/{csr, csr.rs, mm, mm.rs, trap, trap.rs}` in place.
- Seam updates to `xcore/src/cpu/mod.rs` and `xcore/src/device/mod.rs`.
- `mm` visibility tightened to `pub(in crate::arch::riscv)`.
- Validation: 344 tests green, `make run` green.

[**Phase 2 — `isa` nest + `include_str!` fix**]

One PR. Deliverable:
- `arch/riscv/cpu/isa/` in place, `include_str!` path extended to four hops.
- Seam update to `xcore/src/isa/mod.rs`.
- Validation: 344 tests green, decoder path covered by existing decode tests,
  `make linux` boot still OK.

[**Phase 3 — `inst` → `executor` rename + nest**]

One PR. Deliverable:
- `arch/riscv/cpu/executor/` and `arch/riscv/cpu/executor.rs` in place.
- `atomic.rs`'s `crate::arch::riscv::mm::MemOp` rewritten as
  `super::super::mm::MemOp`.
- Validation: 344 tests green including `dispatch_*` tests in `executor.rs`,
  `make linux` / `make debian` boot, difftest clean.

[**Phase 4 — doc + isolation sweep**]

Smallest PR. Deliverable:
- `arch/riscv/mod.rs` doc-comment updated.
- `arch/riscv/cpu/mod.rs` doc-comment updated.
- `arch_isolation.rs` doc-comment refreshed if paths in the comment differ
  from the post-nest reality (no allow-list vocabulary change expected).
- Validation: `make fmt && make clippy && make test && make run`;
  `make linux`, `make debian`.

## Trade-offs {ask reviewer for advice}

- **TR-1: Direction A (nest under `cpu/`) vs Direction B (move `isa` back
  under `xcore/src/isa/riscv/`).**
  - Direction A (chosen): strengthens 01-M-004 (narrower seam surface), makes
    the arch root tree self-documenting (CPU has one home), preserves the
    topic organisation master directive by grouping topics under CPU-concern.
    Cost: one more directory level when navigating to CSRs or MMU.
  - Direction B (rejected): reintroduces the parallel-tree pattern that
    MANUAL_REVIEW #3/#4 flagged and that archModule spent three rounds
    eliminating. It would create `xcore/src/isa/riscv/{decoder, inst, reg}.rs`
    siblinged against `xcore/src/cpu/` which then has to reach across the
    crate root to get ISA types — exactly the structural confusion
    archModule-03 removed. **Explicit rejection reason**: violates 01-M-004
    (seam stays thin) because the neutral `xcore/src/isa/` seam would no
    longer be a re-export-only file; it would own arch-specific code.
- **TR-2: Rename `inst/` → `executor/` vs keep `inst/` as-is vs
  `inst/` → `semantics/`.**
  - `executor/` (chosen): disambiguates from `isa/` (encoding/decoding) at a
    glance; matches the term used in the dispatch machinery (`dispatch`,
    `build_dispatch!`). Cost: one more rename in the git history.
  - `semantics/` (alternate): arguably more precise (these files encode what
    each instruction means). Cost: less common vocabulary for readers; the
    current module is already called "dispatch" in its doc-comment, which
    aligns with "executor" not "semantics".
  - Keep `inst/` as-is: the nested layout `cpu/isa/` vs `cpu/inst/` is arguably
    already enough contextual disambiguation because a reader who sees both
    under `cpu/` knows one is encoding and one is execution. Cost: the smell
    the user flagged in the kickoff ("`isa/inst.rs` vs `inst/*.rs`") persists
    literally — `cpu/isa/inst.rs` still exists after Phase 2. Plan rejects
    this option primarily on that grounds.
  - Reviewer advice requested: is `executor/` the right name, or should we
    use `semantics/` or keep `inst/`?
- **TR-3: `cpu/mod.rs` visibility of the nested topics.**
  - After the nest, `cpu/mod.rs` declares `pub mod csr;`, `pub mod trap;`,
    `pub mod mm;` (tightened to `pub(in crate::arch::riscv) mod mm;`), and
    `pub mod isa;`. `executor/` stays `mod executor;` (private) mirroring the
    current private `inst/`. This keeps the CPU's own code and the seam
    re-exports the only legitimate consumers. Alternative would be to make
    every nested topic `pub(in crate::arch::riscv)`, but `crate::cpu::mod.rs`
    (seam) needs `pub` visibility to name `crate::arch::riscv::cpu::trap::PendingTrap`
    etc. — so `pub` is the minimum that works.

## Validation {test design}

[**Unit Tests**]

- V-UT-1: Existing CSR unit tests (`xcore::arch::riscv::cpu::csr::tests::*`)
  pass unchanged — verifies Phase 1 didn't break the CSR module.
- V-UT-2: Existing MMU/PMP/TLB unit tests under `cpu::mm::*::tests` pass
  unchanged.
- V-UT-3: Existing trap unit tests under `cpu::trap::*::tests` pass unchanged.
- V-UT-4: Existing decoder unit tests under `cpu::isa::decoder::tests` pass
  unchanged — verifies the `include_str!` hop fix landed correctly.
- V-UT-5: Existing dispatch tests in `cpu::executor::tests` (`dispatch_executes_known_instruction`,
  `dispatch_rejects_unknown_instruction`) pass unchanged — verifies the rename
  didn't break dispatch.
- V-UT-6: Existing `cpu::tests::*` RVCore-level tests pass unchanged.

[**Integration Tests**]

- V-IT-1: `xcore/tests/arch_isolation.rs::arch_isolation` passes against the
  new layout with `SEAM_FILES` and `SEAM_ALLOWED_SYMBOLS` unchanged. This
  locks in I-1 and I-2.
- V-IT-2: Any other integration tests under `xcore/tests/` and `xemu/tests/`
  that exercise `make test` continue to pass; test count remains 344.
- V-IT-3: `make run` — default direct-boot of the built-in test image —
  completes with "HIT GOOD TRAP".
- V-IT-4: `make linux` — firmware boot of OpenSBI + Linux kernel — reaches
  shell prompt byte-for-byte equivalent to pre-refactor baseline.
- V-IT-5: `make debian` — Debian 13 Trixie boot — reaches shell prompt
  byte-for-byte equivalent to pre-refactor baseline (requires virtio-blk disk
  from the archModule-03 landing).

[**Failure / Robustness Validation**]

- V-F-1: `git log --follow arch/riscv/cpu/csr/mip.rs` reaches the pre-nest
  `arch/riscv/csr/mip.rs` commit history (same for every moved file). Verifies
  `git mv` was used and history is preserved.
- V-F-2: Bisection gate — each phase commit, in isolation, must be green on
  `make test`. Verified by checking out each commit and re-running `make test`.
- V-F-3: Difftest run vs QEMU and Spike shows **zero** new divergences on the
  default regression corpus (the test suite that ran green in archModule-03).

[**Edge Case Validation**]

- V-E-1: `cargo build --no-default-features --features isa32` — the RV32 build
  still compiles cleanly (important because `SWord` / `Word` cfg-gated imports
  inside the moved files could silently break on the alt-arch path).
- V-E-2: `cargo clippy --all-targets -- -D warnings` — no new clippy warnings
  introduced by the path changes.
- V-E-3: `cargo fmt --check` — the moved files keep their formatting.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (nest under cpu/)                 | V-IT-1 (`arch_isolation` proves no leaked old paths); V-F-1 (git history preserved); V-UT-1..V-UT-6 (nested modules still pass their tests). |
| G-2 (device/ stays sibling)           | V-IT-1 — `arch_isolation` would flag any unintended `device/` path change. |
| G-3 (resolve isa vs inst collision)   | Structural review of the diff + V-UT-4, V-UT-5 (both `cpu::isa` and `cpu::executor` test modules green). |
| G-4 (instpat stays at crate root)     | V-UT-4 (decoder loads from the `CARGO_MANIFEST_DIR`-anchored `src/isa/instpat/riscv.pest`); V-E-1 (cfg-gated builds still find the file). |
| G-5 (behaviour byte-identical)        | V-IT-3, V-IT-4, V-IT-5 (boots equivalent); V-F-3 (difftest clean); V-IT-2 (344-test count preserved). |
| C-1 (every phase green)               | V-F-2 (per-commit `make test`). |
| C-2 (git mv only)                     | V-F-1 (`git log --follow` reaches ancestors). |
| C-3 (byte-identical)                  | V-F-3; V-IT-3, V-IT-4, V-IT-5. |
| C-4 (no new deps)                     | Diff review: `Cargo.toml` unchanged; `arch_isolation.rs` still uses only `std::fs`. |
| C-5 (include_str path fix)            | V-UT-4 (decoder initialises) + `cargo build` on Phase-2 commit. |
| C-6 (pest grammar path unchanged)     | `cargo build` green after Phase 2 — if `CARGO_MANIFEST_DIR` path were wrong, `pest_derive` would fail at proc-macro expansion. |
| C-7 (arch_isolation always green)     | V-IT-1 after every phase. |
| I-1, I-2, I-3 (seam invariants)       | V-IT-1. |
| I-4 (history preservation)            | V-F-1. |
| I-5 (build.rs authoritative)          | Diff review: no new cfg introduced; `build.rs` unchanged. |
| I-6 (no global trait Arch)            | Diff review: no new trait added under `src/` or `src/arch/`. |
