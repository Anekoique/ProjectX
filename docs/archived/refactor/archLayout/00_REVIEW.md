# `archLayout` REVIEW `00`

> Status: Open
> Feature: `archLayout`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
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
- Blocking Issues: 3
- Non-Blocking Issues: 7



## Summary

This is the first iteration of `archLayout` — no prior PLAN, REVIEW, or MASTER
exists. The plan proposes a pure-reorganisation refactor that builds directly on
the landed `archModule` (rounds 00–03) and inherits its four binding MASTER
directives (00-M-001, 00-M-002, 01-M-001, 01-M-003, 01-M-004).

**(a) Direction A defence.** Direction A (nest under `cpu/`) is genuinely
defended against Direction B (move `isa` back to `xcore/src/isa/riscv/`). TR-1
correctly cites the parallel-tree pattern that archModule spent three rounds
eliminating and names the concrete 01-M-004 violation Direction B would
reintroduce: `xcore/src/isa/` would stop being a thin re-export seam and would
own arch-specific code. This framing is sound.

**(b) Inherited MASTER compliance.** 00-M-001, 00-M-002, 01-M-001, 01-M-003 are
correctly enumerated and faithfully preserved. 01-M-004 is **claimed** to be
strengthened (narrower seam subtree) but the plan is not specific enough about
every seam-reference update: three of the four seam diffs are shown, the fourth
(`device/intc/mod.rs`) is correctly noted as unchanged, but the seam-diff list
omits the inherited test-import surface that also must move. 01-M-002 is
mentioned obliquely ("clean/concise/elegant") — the plan body at 564 lines
exceeds the 400-line authorship budget, which is itself an elegance violation
worth flagging (R-010).

**(c) Implementability with zero test/boot regressions.** The phased landing is
credible: 4 phases, each gated by `make fmt && make clippy && make test`, plus
`make linux` / `make debian` at Phase 4. However, the per-phase file-edit
enumeration is **incomplete** — several concrete call sites that will break
under the rename are not listed (see R-001 below). An implementor following the
plan literally will discover these at `cargo build` time and either backfill
them under the current phase's commit (acceptable, matches the failure flow) or
invent paths ad hoc (risky). The plan must enumerate them up front.

**(d) `inst/` rename worth the churn?** Reviewer position: **the rename is
cosmetic noise and should be dropped** (see TR-2 advice). Once `inst/` is nested
at `arch/riscv/cpu/inst/` alongside `arch/riscv/cpu/isa/`, the reader has
unambiguous structural cues: `isa/inst.rs` is inside `isa/` (encoding) and
`inst/*.rs` is a sibling of `isa/` (execution). Renaming to `executor/` adds 7
more `git mv` calls, another seam-less commit to track, and breaks every
`super::` / `crate::arch::riscv::inst::` reference in the tree without adding
conceptual value that `cpu/inst/` alongside `cpu/isa/` does not already provide.
Recommend keeping `inst/` and documenting the distinction in `cpu/mod.rs`
doc-comment.

No CRITICAL findings. Three HIGH findings block approval. Once the plan
enumerates the missing import sites (R-001), widens the visibility-scope
discussion (R-002), and splits or justifies the combined nest+rename phase
(R-003), the plan is ready for implementation.

---

## Findings

### R-001 Incomplete enumeration of import sites that break under the nest

- Severity: HIGH
- Section: Execution Flow (Phases 1–3) / Implementation Plan
- Type: Correctness / Flow
- Problem:
  The plan's per-phase edit list covers the seam files correctly and catches
  `cpu/debug.rs` line 4 (`use super::super::csr::{CsrAddr, find_desc};` →
  `use super::csr::{CsrAddr, find_desc};`), but it omits the following concrete
  call sites that WILL fail to compile after the nest and must be rewritten in
  the same commit that moves the topic module:

  1. `arch/riscv/cpu/debug.rs:45` — `use super::super::csr::DIFFTEST_CSRS;`
     becomes `use super::csr::DIFFTEST_CSRS;` (inside `fn context`; the plan
     mentions the CsrAddr/find_desc sibling line but not this second one).
  2. `arch/riscv/cpu/mod.rs:277` — test module
     `use crate::arch::riscv::{csr::CsrAddr, trap::Exception};` must become
     `use crate::arch::riscv::cpu::{csr::CsrAddr, trap::Exception};`.
  3. `arch/riscv/cpu/mod.rs:413` — test module
     `use crate::arch::riscv::{csr::MStatus, trap::Interrupt};` must become
     `use crate::arch::riscv::cpu::{csr::MStatus, trap::Interrupt};`.
  4. `arch/riscv/trap/handler.rs:179` — `use crate::arch::riscv::{…}` absolute
     path references that traverse the moved topics (csr, trap's own sub-paths)
     must be updated to `crate::arch::riscv::cpu::…`.
  5. `arch/riscv/inst/privileged.rs:102` — test
     `use crate::arch::riscv::trap::{TrapCause, test_helpers::assert_trap};`
     becomes `use crate::arch::riscv::cpu::trap::{…}` after Phase 1.
  6. `arch/riscv/inst/zicsr.rs:106` — test
     `use crate::arch::riscv::{csr::CsrAddr, trap::test_helpers::assert_illegal_inst};`
     becomes
     `use crate::arch::riscv::cpu::{csr::CsrAddr, trap::test_helpers::assert_illegal_inst};`.
  7. `arch/riscv/inst/zicsr.rs:234` —
     `core.privilege = crate::arch::riscv::csr::PrivilegeMode::User;` becomes
     `crate::arch::riscv::cpu::csr::PrivilegeMode::User;`.
  8. `arch/riscv/csr/ops.rs:4` — `use crate::{arch::riscv::cpu::RVCore, …};` is
     path-stable (already anchored at `arch::riscv::cpu::`) and survives. This
     should be called out positively as a sanity anchor in the plan.
  9. `arch/riscv/csr/ops.rs:138` — test
     `use crate::arch::riscv::trap::test_helpers::assert_illegal_inst;` becomes
     `use crate::arch::riscv::cpu::trap::test_helpers::assert_illegal_inst;`.

- Why it matters:
  The plan currently claims Phase 1 is done after editing 5 files
  (`arch/riscv/mod.rs`, `cpu/mod.rs`, `cpu/debug.rs`, and two seam files). The
  above list shows at least 7 additional files whose absolute
  `crate::arch::riscv::` paths must be updated in the **same commit** to keep
  C-1 (every phase green). Without explicit enumeration an implementor will
  discover them one-by-one at `cargo build` time; while the failure flow allows
  recovery, the up-front plan should be specific enough to implement without
  reconstruction from multiple files.
- Recommendation:
  Extend each phase's Execution Flow step with a complete "absolute-path
  rewrites" sub-section listing every `crate::arch::riscv::{csr,mm,trap,inst,isa}::...`
  occurrence in the tree and its post-phase replacement. Use
  `rg "crate::arch::riscv::(csr|mm|trap|inst|isa)::" xemu/xcore/src` as the
  audit command and paste the output (or a distilled table) into the plan.



### R-002 Visibility-scope audit missing — `pub(in …::mm)` sites not addressed

- Severity: HIGH
- Section: Data Structure / Invariants
- Type: Invariant / API
- Problem:
  The plan's Data Structure section (lines 218–230) describes a single
  visibility relaxation: `arch/riscv/mm` becomes `pub(in crate::arch::riscv)`
  (previously `pub(crate)`). That is correct and sufficient for the `mm` module
  declaration itself. **But** the plan does not address module-path-scoped
  visibilities that name the *current* position of `mm` as an absolute path:

  - `arch/riscv/mm/tlb.rs:10` —
    `pub(in crate::arch::riscv::mm) struct TlbEntry { … }`. The scope path
    `crate::arch::riscv::mm` names a module that will no longer exist after
    the nest; it becomes `crate::arch::riscv::cpu::mm`. Unless rewritten, the
    restriction references a non-existent module and fails to compile.
  - `arch/riscv/mm/tlb.rs:58` — `pub(in crate::arch::riscv) struct Tlb { … }`
    is path-stable (the `arch::riscv` module still exists after the nest).
  - `arch/riscv/mm/mmu.rs:20` — `pub(in crate::arch::riscv) tlb: Tlb,` is
    path-stable.

  Similar audits are needed for `trap/`, `csr/`, `inst/` modules: any
  `pub(in crate::arch::riscv::{csr,mm,trap,inst,isa})` scope path must either
  be rewritten to the deeper `…::cpu::…` form or widened to
  `pub(in crate::arch::riscv)`.

- Why it matters:
  `pub(in <path>)` uses an **absolute module path**, not a relative one. Any
  restriction whose path traverses the moved subtree is sensitive to the nest
  and must be rewritten. The current tree has at least one such site
  (`tlb.rs:10`). Missing this produces a compile error that the plan's failure
  flow would need to handle ad hoc.
- Recommendation:
  Add a subsection under Data Structure titled "Visibility-scope rewrites"
  that lists every `pub(in crate::arch::riscv::{csr,mm,trap,inst,isa}…)`
  occurrence in the repository with its post-nest replacement. Audit command:
  `rg "pub\(in crate::arch::riscv::(csr|mm|trap|inst|isa)" xemu/xcore/src`.
  Pick and state one of (a) rewrite each site to the deeper path, or
  (b) widen each to `pub(in crate::arch::riscv)`.



### R-003 Phase 3 bundles nest and rename; cannot land nest without rename

- Severity: HIGH
- Section: Execution Flow (Phase 3) / Implementation Plan
- Type: Flow / Maintainability
- Problem:
  Phase 3 does two things in one PR: nests `inst/` under `cpu/` AND renames it
  to `executor/`. There is no intermediate green checkpoint at which the nest
  exists but the rename does not, so if the rename is rejected post-merge
  (see TR-2 reviewer pushback below) the only paths forward are (a) land the
  rename, (b) roll back Phase 3 entirely, or (c) open a follow-up "revert
  rename" iteration.
- Why it matters:
  The plan's C-1 invariant ("every phase green") is about compilation/tests,
  but each phase should also be *independently reviewable* so a reviewer can
  accept the nest while rejecting or deferring the rename. This matters more
  given TR-2 is genuinely contested (see this review's TR-2 advice below —
  reviewer recommends dropping the rename).
- Recommendation:
  Split Phase 3 into 3a (nest only, `inst/` → `cpu/inst/`) and 3b (rename,
  `cpu/inst/` → `cpu/executor/`). Make 3b explicitly conditional on
  Master/reviewer approval of TR-2. This also cleanly separates the atomic.rs
  `super::super::mm::MemOp` rewrite (belongs in 3a, forced by the nest) from
  the pure cosmetic rename (3b).



### R-004 `cpu/mod.rs` `use super::{…}` rewrite target underspecified

- Severity: MEDIUM
- Section: Execution Flow (Phase 1)
- Type: Correctness
- Problem:
  Plan line 318 says: "Rewrite the `use super::{ csr::…, mm::…, trap::… };`
  block into `use self::{ csr::…, mm::…, trap::… };` (or drop the `super::`
  since these are now children)." But the current `arch/riscv/cpu/mod.rs:11-16`
  import block is:

  ```rust
  use super::{
      csr::{CsrAddr, CsrFile, MStatus, Mip, PrivilegeMode},
      device::intc::{aclint::Aclint, plic::Plic},
      mm::{Mmu, Pmp},
      trap::{PendingTrap, TrapCause, interrupt::HW_IP_MASK},
  };
  ```

  The `device::intc::{aclint::Aclint, plic::Plic}` element is **not** moving —
  `device/` stays a sibling of `cpu/` per G-2. So the rewrite is not a clean
  "change `super` to `self`": the `csr`, `mm`, `trap` elements become `self::`
  while the `device::intc` element remains `super::device::intc::…`. A literal
  one-keyword substitution produces a compile error.
- Why it matters:
  A reader implementing the plan verbatim will either swap the whole block to
  `self::…` (breaking `device`) or leave it as `super::…` (breaking the three
  moved topics).
- Recommendation:
  Paste the exact post-edit `use` block in the plan, e.g.:

  ```rust
  use self::{
      csr::{CsrAddr, CsrFile, MStatus, Mip, PrivilegeMode},
      mm::{Mmu, Pmp},
      trap::{PendingTrap, TrapCause, interrupt::HW_IP_MASK},
  };
  use super::device::intc::{aclint::Aclint, plic::Plic};
  ```



### R-005 Plan claim about `inst/*.rs` `super::` depth is overbroad

- Severity: MEDIUM
- Section: Execution Flow (Phase 3) / Changes from Previous Round
- Type: Correctness
- Problem:
  The Unresolved / residual-risk framing implies every `inst/*.rs` needs a
  `super::` depth adjustment. On inspection this is **not true** for the
  `super::{RVCore, rv64_only, rv64_op}` imports in
  `inst/{atomic,base,compressed,float,mul,privileged,zicsr}.rs`: they resolve
  to `inst/mod.rs` (currently `inst.rs`), which re-exports those names. After
  renaming `inst.rs` → `executor.rs` (or simply nesting without rename),
  `super` from `cpu/{inst|executor}/*.rs` still resolves to the same
  re-export container. So the `super::{…}` imports in the seven child files
  are **path-stable** and require zero edits.

  The plan correctly mentions `atomic.rs`'s `crate::arch::riscv::mm::MemOp`
  rewrite (Phase 3 step 4), which IS required. But it does not positively
  confirm the other six `inst/` children are path-stable, inviting
  unnecessary edits.
- Why it matters:
  Minimising edit surface is part of 01-M-002 (clean/concise/elegant).
  Unnecessary churn on `base.rs`, `compressed.rs`, `float.rs`, `mul.rs`,
  `privileged.rs`, `zicsr.rs` inflates the diff and the git-log-follow chain.
- Recommendation:
  Add a positive sanity note in Phase 3: "Of the seven files under `inst/`,
  only `atomic.rs` requires an edit (its `crate::arch::riscv::mm::MemOp` path
  must be rewritten). The other six — `base.rs`, `compressed.rs`, `float.rs`,
  `mul.rs`, `privileged.rs`, `zicsr.rs` — use only `super::RVCore` and
  `super::rv64_*` which are path-stable after nest. Verified by `cargo build`
  after the single-file atomic.rs edit."



### R-006 `trap/handler.rs:179` test import not addressed explicitly

- Severity: MEDIUM
- Section: Execution Flow (Phase 1)
- Type: Correctness
- Problem:
  `arch/riscv/trap/handler.rs:179` contains a test import
  `use crate::arch::riscv::{…};`. After Phase 1 nests `trap/`, these paths
  must prefix with `cpu::`. Plan does not name this file.
- Why it matters:
  Phase 1 will build-fail on `cargo test` (but not on `cargo build --lib`)
  without this edit. The plan's validation gate includes `make test` so the
  failure will surface — but the plan should pre-empt it.
- Recommendation:
  Covered by R-001's audit command
  (`rg "crate::arch::riscv::(csr|mm|trap|inst|isa)::" xemu/xcore/src`). Call
  the line out explicitly in the Phase 1 edit list.



### R-007 `arch_isolation.rs` doc-comment references will go stale

- Severity: LOW
- Section: Execution Flow (Phase 4)
- Type: Maintainability
- Problem:
  `xemu/xcore/tests/arch_isolation.rs` lines 1–15 reference
  `docs/fix/archModule/03_PLAN.md` and legacy finding IDs (`R-019`, `R-022`,
  `R-024`). These references remain historically valid, but the test-file
  doc-comments also describe the seam file paths and re-export vocabulary —
  these descriptions must still match post-nest reality. Plan's Phase 4 notes
  a doc-comment refresh but does not explicitly mention this file's comment
  header.
- Why it matters:
  Stale references don't block correctness but degrade the test file's
  auditability. The `SEAM_FILES` and `SEAM_ALLOWED_SYMBOLS` arrays are
  genuinely unchanged (as the plan claims), but the surrounding comments
  should reflect the new paths.
- Recommendation:
  In Phase 4 step 2, state explicitly: "refresh `arch_isolation.rs`
  doc-comments to say: seam files now re-export from
  `crate::arch::riscv::cpu::…` (not the pre-nest flat topics). Do NOT change
  `SEAM_FILES`, `SEAM_ALLOWED_SYMBOLS`, or `BUS_DEBUG_STRING_PINS` — their
  values are invariant under the nest."



### R-008 `include_str!` hop-count language ambiguous in Failure Flow

- Severity: LOW
- Section: Execution Flow (Failure Flow step 4) / Constraints C-5
- Type: Correctness
- Problem:
  C-5 and Phase-2 step 2 both correctly specify the new path as
  `"../../../../isa/instpat/riscv.instpat"`. But Failure Flow step 4 says
  "Four `..` hops" — slightly ambiguous because a reader may count the four
  `../` segments plus the `isa/` segment and get confused. Minor.
- Why it matters:
  Low — `cargo build` fails fast if wrong.
- Recommendation:
  Rewrite Failure Flow step 4 as: "Verify the `include_str!` argument is
  exactly `"../../../../isa/instpat/riscv.instpat"` (four `../` segments,
  taking the decoder from `arch/riscv/cpu/isa/` back to `src/`, then down
  into the neutral `isa/instpat/` data dir)."



### R-009 Boot-gate success criteria underspecified

- Severity: LOW
- Section: Validation / Acceptance Mapping
- Type: Validation
- Problem:
  V-IT-4 / V-IT-5 require `make linux` and `make debian` to reach "shell
  prompt byte-for-byte equivalent to pre-refactor baseline". How is
  equivalence measured? By inspection? By hash? V-F-3 mentions "zero
  divergences on the default regression corpus" — where is the corpus defined?
- Why it matters:
  Without a concrete pass/fail test, "byte-for-byte equivalent" becomes
  inspection by eye, which is not reliable for a validation gate.
- Recommendation:
  Specify: `make linux` succeeds iff emulator reaches `/ #` prompt within N
  seconds and serial log contains a named boot marker (e.g.
  `Welcome to Buildroot` or `Debian GNU/Linux 13 trixie`). Capture via
  `timeout 60 make linux 2>&1 | tee` and grep. Similarly name the difftest
  corpus path for V-F-3.



### R-010 Plan body 564 lines — exceeds the 400-line authorship budget

- Severity: LOW
- Section: Plan Body / 01-M-002 Compliance
- Type: Maintainability
- Problem:
  The plan is 564 lines. The authorship directive (part of 01-M-002
  clean/concise/elegant) targets ≤ 400 lines. Compression is achievable: the
  Architecture before/after tree blocks are verbose; the Response Matrix row
  prose can be shortened; trade-off sections have long sub-bullets.
- Why it matters:
  Over-long plans become reference documents rather than implementation
  guides. 01-M-002 was raised precisely because past plans bloated.
- Recommendation:
  Target 400 lines in the next revision. Suggested compression: merge the
  before/after trees into a single diff-style block; collapse TR-1/TR-2/TR-3
  bullets to one bullet each (option + reasoning); shrink the Response Matrix
  prose to one clause per row.



---

## Trade-off Advice

### TR-1 Direction A (nest under `cpu/`) vs Direction B (`isa` back to crate-root `isa/`)

- Related Plan Item: `TR-1`
- Topic: Structural clarity vs Seam thinness (01-M-004)
- Reviewer Position: Prefer Option A (same as plan)
- Advice:
  Adopt Direction A. The plan's rejection of Direction B is well-grounded: B
  would reintroduce the parallel-tree pattern that archModule spent rounds
  00–03 dismantling, and would violate 01-M-004 by making
  `xcore/src/isa/mod.rs` (a neutral seam) own arch-specific code. Reviewer
  confirms Direction B is disqualified on inherited-MASTER grounds alone.
- Rationale:
  01-M-004 requires top-level `cpu/`, `device/`, `isa/` to be tiny
  `#[cfg(arch)]` seams with arch behaviour inside `arch/`. Direction B's
  `xcore/src/isa/riscv/{decoder,inst,reg}.rs` would necessarily contain
  arch-specific code at a path outside `arch/`, which is the structural
  pattern 01-M-004 exists to prevent. Direction A preserves this by keeping
  all arch code under `arch/riscv/` and strictly narrowing the seam's
  re-export surface to `arch::riscv::cpu::*` and `arch::riscv::device::*`.
- Required Action:
  Keep the current choice. No change required beyond the findings above.



### TR-2 Rename `inst/` → `executor/` vs keep `inst/` as-is

- Related Plan Item: `TR-2`
- Topic: Naming clarity vs Refactor churn
- Reviewer Position: Prefer keeping `inst/` (disagree with plan)
- Advice:
  **Do not rename**. Keep `inst/` as `arch/riscv/cpu/inst/` after Phase 3
  nesting. Document the distinction between `cpu/isa/` (encoding) and
  `cpu/inst/` (execution) in the `cpu/mod.rs` doc-comment instead.
- Rationale:
  The naming collision the kickoff raised (`isa/inst.rs` vs `inst/*.rs`) is
  real in the **flat** layout where both `isa/` and `inst/` are arch-root
  siblings. Once both move under `cpu/`, the collision resolves structurally:
  `cpu/isa/inst.rs` is unambiguously "encoding definitions for instructions"
  (inside the encoding module) and `cpu/inst/base.rs` is unambiguously
  "execution semantics" (sibling module whose doc-comment already says
  "instruction dispatch and per-extension handlers"). Renaming to `executor/`
  adds:

  - 7 additional `git mv` operations (one per child file) plus the module rename.
  - An additional commit with no behaviour change and no structural improvement
    beyond word choice.
  - Rewrites of every `crate::arch::riscv::inst::…` reference (test modules,
    `mod inst;` declaration, and any `#[cfg(test)]` absolute path in the
    children).

  In exchange, it gets: one module name that matches the
  `dispatch`/`build_dispatch!` vocabulary. That alignment is real but weak —
  `inst/mod.rs` already documents itself as "instruction dispatch", so a
  doc-comment capture suffices.

  The plan's own TR-2 bullet concedes "the nested layout `cpu/isa/` vs
  `cpu/inst/` is arguably already enough contextual disambiguation" and
  rejects this option "primarily on the grounds" that `cpu/isa/inst.rs`
  continues to exist literally. That literal persistence is not a real
  problem: a reader looking at `cpu/isa/inst.rs` sees it under `isa/` and
  reads it as "encoding-layer inst definitions"; a reader looking at
  `cpu/inst/base.rs` sees it as "execution-layer inst handlers". The tree
  structure carries the meaning.
- Required Action:
  Drop the rename. Split Phase 3 per R-003 and deliver only Phase 3a (nest).
  Update `cpu/mod.rs` doc-comment in Phase 4 with an explicit line such as:
  "`cpu/isa/` holds instruction **encoding** (decoder, formats, kinds,
  register enum). `cpu/inst/` holds instruction **execution semantics**
  (per-extension handlers that run one decoded instruction)." If Executor
  still wants the rename, expand the comparison with concrete reader-value
  examples showing where `inst/` (nested) is genuinely ambiguous and
  `executor/` resolves the ambiguity.



### TR-3 `cpu/mod.rs` visibility of nested topics

- Related Plan Item: `TR-3`
- Topic: Visibility discipline
- Reviewer Position: Prefer plan's choice
- Advice:
  Keep the plan's visibility plan: `pub mod csr;`, `pub mod trap;`,
  `pub mod isa;`, `pub(in crate::arch::riscv) mod mm;`, and
  `mod inst;` / `mod executor;` private. This is the minimum that lets the
  seam files re-export the vocabulary they need.
- Rationale:
  The seam at `xcore/src/cpu/mod.rs` names
  `crate::arch::riscv::cpu::trap::PendingTrap` and the seam at
  `xcore/src/isa/mod.rs` names
  `crate::arch::riscv::cpu::isa::{DECODER, DecodedInst, IMG, InstFormat, InstKind, RVReg}`.
  `trap` and `isa` must be `pub` at `cpu/mod.rs` for those to resolve. `mm`
  only needs reachability within `arch::riscv` (for the executor/atomic.rs
  `super::super::mm::MemOp` path), hence the tightened
  `pub(in crate::arch::riscv)`. `inst`/`executor` needs nothing external —
  keep it private. This is correct discipline.
- Required Action:
  Keep as-is; ensure the narrower `pub(in crate::arch::riscv)` chosen for
  `mm` is paired with the visibility-scope audit requested by R-002.



---

## Positive Notes

- Direction A vs Direction B trade-off analysis (TR-1) is genuinely grounded
  in inherited MASTER directives (01-M-004) rather than hand-waved. Naming the
  concrete violation Direction B would introduce is exactly the right framing.
- The `include_str!` hop-count change is flagged up front (C-5), the new path
  string is written verbatim, and the `CARGO_MANIFEST_DIR`-relative
  `#[grammar = "src/isa/instpat/riscv.pest"]` is correctly identified as
  unchanged (C-6). This level of explicit attention to build-script path
  semantics is exactly right for a rename refactor.
- Non-goals (NG-1..NG-7) are well-scoped and bound the blast radius correctly:
  `xcore/src/isa/instpat/` stays, `loongarch/` untouched, `arch_isolation`
  allow-list vocabulary unchanged, no new deps, no new cfgs. NG-7 explicitly
  forbids the rejected Direction B pattern, which prevents silent regression.
- The visibility tightening of `mm` from `pub(crate)` to
  `pub(in crate::arch::riscv)` is a genuine improvement (narrower blast
  radius) that the nest enables — this is the kind of side-benefit that
  justifies reorganisation beyond pure aesthetics.
- Validation section maps every goal/constraint/invariant to at least one
  test item in Acceptance Mapping. The V-F-1 git-history-preservation check
  is a nice touch.
- The `device/intc/mod.rs` seam is correctly identified as UNCHANGED because
  `device/` itself does not move — the plan catches this subtlety and avoids
  the natural mistake of "update all four seam files".

---

## Approval Conditions

### Must Fix
- R-001 (enumerate every breaking absolute-path import site per phase)
- R-002 (audit `pub(in crate::arch::riscv::…)` scopes and state rewrite policy)
- R-003 (split Phase 3 into 3a nest + 3b rename, or justify bundling against
  TR-2 pushback)

### Should Improve
- R-004 (show concrete post-edit `cpu/mod.rs` import block with `device/` split)
- R-005 (positive sanity note: only `atomic.rs` needs edits in `inst/`
  children)
- R-006 (`trap/handler.rs:179` explicit call-out — subsumed by R-001 audit)
- R-007 (`arch_isolation.rs` doc-comment refresh in Phase 4)
- R-010 (compress plan body to ≤ 400 lines)

### Trade-off Responses Required
- TR-1 (no change — continue with Direction A)
- TR-2 (reviewer recommends dropping the rename; Executor must either
  comply or expand the ambiguity-resolution argument with concrete reader-
  value examples)
- TR-3 (no change — visibility plan stands)

### Ready for Implementation
- No
- Reason: Three HIGH findings (R-001, R-002, R-003) block. None are CRITICAL,
  but each represents a concrete gap that would force mid-phase improvisation
  and risk breaking C-1 ("every phase green"). Fixing R-001 and R-002 is a
  mechanical audit (two `rg` commands, paste output into the plan); fixing
  R-003 is a one-line phase-split. Once the next revision lands those three
  and responds to TR-2, the plan is approvable without further iteration.
