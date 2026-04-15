# `archModule` REVIEW `01`

> Status: Open
> Feature: `archModule`
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
- Blocking Issues: `1`
- Non-Blocking Issues: `5`



## Summary

Round 01 is a substantial, honest improvement over round 00. Numbering
convention: R- / TR- IDs are re-used from round 00 where the concern
persists (resolutions audited in the matrix below); new concerns get
fresh IDs starting at R-010. Round 00's findings R-001..R-009 are all
either resolved in the `01_PLAN.md` Response Matrix or rolled into the
new R-010 question below.

Resolution audit against the round-00 blocking set:

- **R-001 (round 00, CRITICAL — `device/intc` leak):** adequately
  resolved in principle. Plan Option (a) is accepted:
  `device/intc/aclint.rs` and `device/intc/plic.rs` move to
  `arch/riscv/device/intc/` behind a `device/intc/mod.rs` `cfg_if`
  seam, and the Summary is honestly narrowed to *partially* address
  MANUAL_REVIEW #3. The Bus-field residuals (`aclint_idx`,
  `plic_idx`, `mtime`, `set_timer_source`, `set_irq_sink`,
  `ssip_flag`/`take_ssip`) are explicitly queued as NG-5 follow-ups
  with named successor plans (`aclintSplit`, `plicGateway`,
  `directIrq`) and tracked via the V-F-2 allow-list. The scoping is
  honest — the plan does not claim to close MANUAL_REVIEW #3.
- **R-002 (round 00, CRITICAL — `pub(in …)` paths):** resolved. The
  plan enumerates all 11 call sites explicitly; cross-checked against
  `rg 'pub\(in crate::(cpu|isa)::(riscv|loongarch)' xemu/xcore/src` —
  exactly 11 hits: 5 in `cpu/riscv/trap.rs` (lines 21, 26, 31, 35, 55),
  2 in `cpu/riscv/mm/tlb.rs` (lines 10, 58), 1 in `cpu/riscv/mm/mmu.rs`
  (line 20), 1 in `cpu/riscv/csr.rs` (line 95), 2 in
  `cpu/riscv/csr/ops.rs` (lines 7, 16). No `pub(in crate::isa::riscv)`
  sites exist today (confirmed). Phase 2 adds the rewrite rule and a
  grep gate of 0 hits post-rewrite; Phase 3 inherits the same rule
  defensively. Enumeration matches reality.
- **R-003 (round 00, HIGH — `--features loongarch` is fiction):**
  resolved. All validators now invoke `X_ARCH=<name> cargo check -p
  xcore`, which matches the real mechanism in `xemu/xcore/build.rs:19–33`
  (env var `X_ARCH` → `rustc-cfg` emission of the `X_ARCH` value plus
  `riscv`/`loongarch` plus `isa32`/`isa64`). V-E-1 is retargeted
  sensibly to the `compile_error!(all(riscv, loongarch))` gate plus
  `RUSTFLAGS` manual injection — the only runnable form given
  `build.rs` structurally cannot emit both cfgs from one `X_ARCH`.
- **M-001 (MUST keep cfg-if, no `trait Arch`):** faithfully applied.
  The Data Structure and API Surface sections show `cfg_if` at every
  seam (`arch/mod.rs`, `cpu/mod.rs`, `isa/mod.rs`, `device/intc/mod.rs`,
  plus a one-armed `#[cfg(riscv)]` re-export in `device/mod.rs`). No
  `trait Arch { type Word; … }`, `Bus<A: Arch>`, or `Cpu<A: Arch>`
  generics are introduced. `CoreOps` and `DebugOps` remain the upper-
  layer contract. T-1 is formally closed. No sneak-in observed.
- **M-002 (MUST rename `irq_bits.rs` → `irq.rs`; topic-organised
  `arch/`):** the rename part is satisfied — no `irq_bits.rs` exists
  in the target layout; the mip-bit constants land in
  `…/trap/interrupt.rs` alongside the existing `Interrupt` enum, which
  is better than a generic `irq.rs` because it reads as "topic:
  interrupt vocabulary." However, the plan's T-5 equivocates on
  *physical* topic layout: G-5 (`01_PLAN.md:268–290`) draws flat
  topics directly under `arch/riscv/` (`cpu`, `isa`, `inst`, `csr`,
  `mm`, `trap`, `device`), but Phase 2's Execution Flow
  (`01_PLAN.md:641–652`) keeps the topics nested under
  `arch/riscv/cpu/{csr, mm, trap, inst}` and hoists them via
  `pub use cpu::{csr, inst, mm, trap}` at `arch/riscv/mod.rs`. M-002's
  example paths are `riscv/trap/interrupt` and `riscv/csr` — flat.
  "The file in arch should be topic/theme leaded or abstracted them in
  riscv/trap/interrupt or riscv/csr" cannot be read as "topic via
  `pub use` over a `cpu/` prefix." See R-010 — this ambiguity is the
  one remaining CRITICAL for round 02.

Other positives in round 01: the vocabulary allow-list grep (V-F-2)
is the right shape for enforcing I-1/I-2 and catches the class of
leak the round-00 path grep missed. The compile-time seam check in
V-UT-2 is strictly better than round 00's `type_name` canary. The
per-phase PR titles plus I-6's green-bar invariant give the executor
a defensible landing pattern. Trade-off advice on T-2/T-3/T-4 stays
aligned with round 00.

Blocking concerns collapse to one: R-010 (M-002 layout ambiguity).
HIGH non-blocking concerns are R-011 (the hoist in Phase 2 requires
visibility edits that C-2 forbids) and R-012 (the mip-bit paste
target is named two different ways). The rest are MEDIUM / LOW.
Ready for Implementation: No, until R-010 is settled — a single
round-02 delta.

---

## Findings

### R-010 `M-002 topic layout: nested-via-pub-use (Option A) vs flat (Option B)`

- Severity: CRITICAL
- Section: Spec / G-5, Implementation Plan / Phase 2, Trade-offs / T-5
- Type: Spec Alignment
- Problem:
  The plan's G-5 (`01_PLAN.md:268–290`) draws a flat topic layout:
  ```
  arch/riscv/
  ├── cpu/         (context, debug, mod only)
  ├── isa/
  ├── inst/
  ├── csr/
  ├── mm/
  ├── trap/        (cause, exception, handler, interrupt)
  └── device/intc/
  ```
  but Phase 2 in Execution Flow (`01_PLAN.md:641–652`) does a single
  `git mv cpu/riscv → arch/riscv/cpu` that leaves the topics *nested*
  under `arch/riscv/cpu/{csr, csr.rs, mm, mm.rs, trap, trap.rs, inst,
  inst.rs, context.rs, debug.rs, mod.rs}`, then hoists them via
  `pub use cpu::{csr, inst, mm, trap}` at `arch/riscv/mod.rs`. T-5
  Option A is recommended; Option B (a second `git mv` wave to
  physically flatten) is listed as "reviewer-decidable."
  M-002 says explicitly: "every file or behaviour in arch directory
  should be highly abstracted by the upper dir of cpu/isa/device. The
  file in arch should be topic/theme leaded or abstracted them in
  **riscv/trap/interrupt** or **riscv/csr**." The example paths are
  physically flat at `arch/<arch>/<topic>/…`. A nested layout
  re-exposed by `pub use` makes the import path topical but leaves
  the on-disk layout arranged by the old upper-module grouping, which
  is precisely what M-002 is pushing back on.
  Phase 4 compounds the ambiguity: `01_PLAN.md:897` says "paste into
  the existing `arch/riscv/cpu/trap/interrupt.rs` module" (nested),
  while the API Surface pseudocode (`01_PLAN.md:489–501`) and the
  Response Matrix M-002 row say the canonical path is
  `arch/riscv/trap/interrupt.rs` (flat). The plan cannot be
  simultaneously both; one of these is wrong at implementation time.
- Why it matters:
  M-002 is a MASTER MUST directive. Even partial deviation (the
  on-disk tree reads as `arch/riscv/cpu/trap/interrupt.rs`) is a
  MASTER violation and blocks "Ready for Implementation." More
  pragmatically, the path shows up in every `use` statement in the
  crate, in `git log --follow` paths, and in every rustdoc link —
  "the upper layer re-exports it as if it were flat" is a
  not-quite-fix because the in-repo reality is what future
  contributors read.
- Recommendation:
  Pick Option B (flat) and commit to it in round 02. Concretely:
  - Phase 2a: `git mv xemu/xcore/src/cpu/riscv xemu/xcore/src/arch/riscv/cpu`
    (as-is).
  - Phase 2b: a second `git mv` wave hoisting
    `arch/riscv/cpu/{csr, csr.rs}` → `arch/riscv/{csr, csr.rs}` (and
    similarly for `mm`, `trap`, `inst`; leave `context.rs`, `debug.rs`,
    `mod.rs` under `arch/riscv/cpu/` since those are the "CPU core"
    topic). History is still preserved because `git mv` composes.
  - Fix cross-topic imports that the hoist exposes (e.g.
    `arch/riscv/trap/handler.rs` will need
    `use crate::arch::riscv::cpu::RVCore` where today it uses
    `use crate::cpu::riscv::RVCore`). That fixup is mechanical.
  - Drop T-5 entirely in the next round; the decision is settled by
    M-002. Remove the "reviewer-decidable" escape hatch from Phase 2.
  - Update G-5, API Surface, Response Matrix, and Phase 4's "paste
    into" reference so every mention of the interrupt vocabulary
    module says `arch/riscv/trap/interrupt.rs` consistently.
  If the executor genuinely believes Option A satisfies M-002, they
  must obtain an explicit MASTER waiver for round 02 (i.e. a MASTER
  directive updating M-002 to accept re-export-only topicality).
  Absent that, Option B is the only compliant landing.



### R-011 `Phase 2 hoist requires visibility changes that C-2 forbids`

- Severity: HIGH
- Section: Implementation Plan / Phase 2, Constraints / C-2
- Type: Correctness
- Problem:
  Phase 2's Option-A re-export (`01_PLAN.md:641–647`) is
  ```rust
  // arch/riscv/mod.rs  (phase 2)
  pub mod cpu;
  pub mod isa;
  pub use cpu::{csr, inst, mm, trap};
  ```
  The current `xemu/xcore/src/cpu/riscv/mod.rs:6–11` declares:
  ```
  pub mod context;
  pub mod csr;
  pub mod debug;
  mod inst;          // ← PRIVATE
  pub(crate) mod mm; // ← pub(crate), not pub
  pub mod trap;
  ```
  After `git mv` this file becomes `arch/riscv/cpu/mod.rs`. Writing
  `pub use cpu::{csr, inst, mm, trap}` in `arch/riscv/mod.rs` will
  fail to compile — `inst` is private to its parent
  (`E0603: module inst is private` on the re-export). And `pub use
  cpu::mm` is stricter than `mod mm`'s `pub(crate)` visibility, so it
  should be `pub(crate) use cpu::mm` to match, not `pub use`.
  The plan's C-2 constraint says "no semantic edits inside moved
  files beyond import-path adjustments needed to compile after
  relocation." Changing `mod inst;` to `pub(crate) mod inst;` is a
  visibility edit, not a path edit, and the plan does not name this
  as an exception to C-2.
- Why it matters:
  Phase 2 is the linchpin phase ("green bar after move"). If the
  executor literally types the `pub use cpu::{csr, inst, mm, trap}`
  line from the plan, `cargo build` fails. That sinks the phased-PR
  story.
- Recommendation:
  Either (a) adopt R-010 Option B (flat layout), which makes this
  hoist go away entirely — topics live at `arch/riscv/<topic>/` and
  are declared `pub mod` there with appropriate visibility — or (b)
  if Option A is kept, add an explicit Phase 2 substep: "Promote
  `mod inst;` → `pub(crate) mod inst;` in `arch/riscv/cpu/mod.rs`
  (visibility edit; noted exception to C-2, item-preserving)," and
  change the re-export to `pub(crate) use cpu::{csr, inst, mm, trap}`
  to match the most-restrictive child visibility. Under R-010 Option
  B these issues disappear naturally.



### R-012 `Phase 4 mip-bit paste target is ambiguous between two paths`

- Severity: HIGH
- Section: Implementation Plan / Phase 4, Architecture / API Surface
- Type: Correctness
- Problem:
  Phase 4 at `01_PLAN.md:895–897` says:
  > Cut mip bit constants `SSIP/MSIP/STIP/MTIP/SEIP/MEIP/HW_IP_MASK`
  > from `device/mod.rs` (lines 55–72). Paste into the existing
  > `arch/riscv/cpu/trap/interrupt.rs` module (append to the existing
  > `Interrupt` enum module).
  But the Summary (`01_PLAN.md:31–33`), G-3 (`01_PLAN.md:256–261`),
  API Surface (`01_PLAN.md:489–501, 569–572`), and the Response
  Matrix M-002 row all name the target as
  `arch/riscv/trap/interrupt.rs` (no `cpu/` prefix). Which is
  canonical? This is the same inconsistency that drives R-010, but at
  the substep-action level — an executor could pick either.
  Separately, the relocated `arch/riscv/cpu/mod.rs` (formerly
  `cpu/riscv/mod.rs:140`) currently uses both the bare `HW_IP_MASK`
  constant AND a `Mip::STIP.bits()` method call (from the `Mip`
  bitflags type in `csr/mip.rs`). Phase 4 documents the `HW_IP_MASK`
  import rewrite but does not discuss `Mip::STIP`. Since `Mip` is a
  RISC-V CSR type that stays inside `arch/riscv/`, this is
  arch-local and safe; V-F-2 includes `Mip` in its vocabulary, so
  `csr/mip.rs` is an expected allow-listed hit under `arch/**`. Not
  a defect per se, but worth calling out.
- Why it matters:
  A copy-paste-target ambiguity at the substep level is exactly the
  kind of issue that produces an invalid Phase 4 PR. Because Phase 4
  is also where V-F-2 first passes, a wrong paste target ripples into
  every subsequent import path.
- Recommendation:
  Choose the canonical path once and use it everywhere. Given R-010's
  recommendation to flatten, settle on `arch/riscv/trap/interrupt.rs`
  and delete the "existing `arch/riscv/cpu/trap/interrupt.rs`" phrasing
  from Phase 4. Add a single sentence: "After R-010's Phase 2b hoist,
  `arch/riscv/trap/interrupt.rs` is the physical location; do not
  paste into `arch/riscv/cpu/trap/interrupt.rs`." If Option A is
  retained under a MASTER waiver, do the fix the other way — both
  places cannot be left referenced.



### R-013 `V-UT-2 behavioural test duplicates existing coverage and is heavy-weight`

- Severity: MEDIUM
- Section: Validation / Unit Tests / V-UT-2
- Type: Validation
- Problem:
  V-UT-2 (`01_PLAN.md:1016–1046`) has two parts:
  (1) a `const _: fn() = || { ... let _: fn() -> … ::Core = … ::Core::new; };`
  compile-time seam check. The `#[cfg(riscv)]` sits on a `let`
  statement inside a closure, which is valid Rust but an unusual
  placement; under `X_ARCH=loongarch*` the closure body becomes
  effectively empty and the `const _: fn()` still compiles — fine.
  (2) a runtime `#[test] fn selected_core_boots_at_reset_vector()`
  that constructs `Core::new()`, wraps it in `CPU::new(…)`, calls
  `cpu.reset().unwrap()`, and asserts `cpu.pc() == RESET_VECTOR`.
  `CPU::reset` executes `load_direct(None)` which calls
  `bus_mut().load_ram(RESET_VECTOR, …)` loading the default image —
  i.e. it allocates a full `Bus` with 128 MB RAM. This duplicates
  the existing `cpu_reset_sets_pc_to_reset_vector` test in
  `xemu/xcore/src/cpu/mod.rs:331–341`, which already covers the seam
  behaviour under the default feature set.
- Why it matters:
  Round 00 asked for a non-brittle canary, and this plan supplies
  one, so R-005 is "resolved." But the runtime sibling adds no
  independent signal beyond the compile-time check, while materially
  increasing CI runtime (full 128 MB allocation + image load per
  `cargo test` invocation).
- Recommendation:
  Keep only the compile-time part of V-UT-2:
  ```rust
  const _: fn() = || {
      #[cfg(riscv)]
      let _: fn() -> crate::arch::selected::cpu::Core =
          crate::arch::selected::cpu::Core::new;
  };
  ```
  and delete the runtime `selected_core_boots_at_reset_vector` test
  with a note "seam boot behaviour is covered by the existing
  `cpu::tests::cpu_reset_sets_pc_to_reset_vector`." If a runtime
  check is still wanted, assert a *cheap* seam property (e.g. that
  `Core::new()`'s default `pc()` equals `VirtAddr::from(RESET_VECTOR)`
  without calling `reset()` or loading an image).



### R-014 `V-F-2 NG-5 allow-list is whole-file, not pattern-scoped`

- Severity: MEDIUM
- Section: Validation / V-F-2, Constraints / NG-5
- Type: Validation
- Problem:
  Phase 5 adds `// TODO: archBus follow-up` markers at seven sites in
  `device/bus.rs` (`aclint_idx`, `plic_idx`, `mtime`,
  `set_timer_source`, `set_irq_sink`, `ssip_flag`, `take_ssip`). V-F-2
  (`01_PLAN.md:1068–1086`) says "every hit must come from the allow-
  list" and lists `xemu/xcore/src/device/bus.rs` as a per-file
  allow. But the grep pattern
  (`\b(MSIP|MTIP|MEIP|SEIP|SSIP|STIP|mtime|mtimecmp|aclint|plic|hart|RVCore|Mstatus|Mip|Sv32|Sv39)\b`)
  does not key on the `// TODO` marker. It is a plain file allow-list:
  any future code added to `device/bus.rs` that introduces *new*
  RISC-V vocabulary (a future `Bus::mtime64()` or `hart_context_id()`)
  passes V-F-2 silently because the file is whole-allow-listed.
  I-2 states "new violations are prohibited," but V-F-2 as written
  cannot detect them.
- Why it matters:
  The stated contract is "NG-5 is frozen scope; follow-up plans will
  clean it up," but V-F-2 does not enforce that freezing. Without
  enforcement, a subsequent, unrelated PR can silently grow the
  Bus-side RISC-V leakage under cover of the allow-list.
- Recommendation:
  Tighten V-F-2 to a line-level allow-list rather than file-level.
  Either (a) maintain a small `docs/fix/archModule/vfz-allow.txt`
  with exact `file:line: literal` entries for each known NG-5 hit,
  and have the plan's checklist diff `rg` output against that file,
  or (b) change the grep so that within `device/bus.rs` only a
  specific set of identifiers is exempt (`aclint_idx`, `plic_idx`,
  `ssip_flag`, `take_ssip`, `mtime\b`, `set_timer_source`,
  `set_irq_sink`) and any other occurrence there fails. The Phase 5
  `// TODO: archBus follow-up` markers then become the human-readable
  sibling to that machine-checkable enforcement.



### R-015 `S2 green-bar is regression-avoidance, not seam proof`

- Severity: LOW
- Section: State Transition / S2, Implementation Plan / Phase 2
- Type: Validation
- Problem:
  S2 (after Phase 2) claims `make linux` and `make debian` green.
  Phase 2 moves only `cpu/riscv` and `isa/riscv`; `device/intc/mod.rs`
  is untouched and still contains concrete `pub mod aclint; pub mod
  plic;` plus the in-file mip bits at `device/mod.rs:55–72`. So S2
  *is* buildable, but it does not prove anything about the intc seam
  or the `#[cfg(riscv)]`-gated mip-bit re-export — those land in
  Phase 4. The claim "at each phase boundary the full boot matrix is
  green" is a stronger reading than Phase 2 actually delivers, which
  is "Phase 2 does not regress the pre-Phase-4 bus path."
- Why it matters:
  Low-impact nit. The plan doesn't over-claim outright, but a reader
  might conflate green-bar with seam-proof. Worth one sentence of
  clarification in Phase 2's acceptance text.
- Recommendation:
  In Phase 2's green-bar statement, add: "The intc seam is
  unexercised at this phase because `device/intc/` is untouched until
  Phase 4; Phase 2's bar is regression-avoidance, not seam proof." No
  plan restructuring required.



---

## Trade-off Advice

### TR-1 `Defer trait Arch vs adopt now (closed by M-001)`

- Related Plan Item: `T-1` (closed)
- Topic: Clean Design vs Diff Size
- Reviewer Position: Closed — Option A per M-001
- Advice:
  No open trade-off remains. The plan correctly records T-1 as
  closed by M-001 and carries the "partial MANUAL_REVIEW #3"
  narrowing into the Summary and NG-5. Nothing to add.
- Rationale:
  M-001 is a MUST; `trait Arch` is out of scope by directive. The
  plan acknowledges this without claiming MANUAL_REVIEW #3 is fully
  resolved. Good discipline.
- Required Action:
  None — keep as is.



### TR-2 `Back-compat re-exports vs hard cut`

- Related Plan Item: `T-2`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A
- Advice:
  Confirmed from round 00. Back-compat re-exports at `cpu::*` /
  `isa::*` / `device::*` are the right choice — downstream
  (`xdb`, `xam`, `xemu` binary) stays untouched and the refactor
  stays mechanically small.
- Rationale:
  I-4 explicitly guarantees this. No new information since round 00.
- Required Action:
  Keep Option A. No plan change.



### TR-3 `IrqState location`

- Related Plan Item: `T-3`
- Topic: Premature Generalisation vs Now-ness
- Reviewer Position: Prefer Option A
- Advice:
  `IrqState` stays in `device/mod.rs`; bit positions live in
  `arch/<arch>/trap/interrupt.rs` (or wherever R-010 lands them).
  Round 01 tightens the rationale correctly
  ("arch-neutral storage for arch-specific bit positions").
- Rationale:
  Unchanged from round 00. The MANUAL_REVIEW #5/#6 follow-up will
  reshape `IrqState`; moving it now is wasted motion.
- Required Action:
  Keep Option A. No plan change.



### TR-4 `Single PR vs phased PRs`

- Related Plan Item: `T-4`
- Topic: Reviewability vs Merge Overhead
- Reviewer Position: Prefer Option B (phased) with a 2a/2b caveat
- Advice:
  Five phased PRs is the right call. The PR title list in the
  Implementation Plan is useful. Caveat: if R-010 is accepted and
  Phase 2 gains a "Phase 2b" hoist wave (flat-layout `git mv`),
  consider splitting the Phase 2 PR into 2a (`git mv` relocation +
  `pub(in …)` rewrite) and 2b (topic flatten + cross-topic import
  fixups). Otherwise a single "Phase 2" PR contains two semantically
  distinct moves.
- Rationale:
  Each `git mv` wave is its own reviewable unit. A Phase 2a / 2b
  split makes the history easier to bisect if `make linux` regresses.
- Required Action:
  Accept Option B. If R-010 lands as Option B (flat layout), split
  Phase 2 into 2a / 2b in round 02's Implementation Plan.



### TR-5 `Nested (cpu/topic) vs flat (arch/topic) directory layout`

- Related Plan Item: `T-5`
- Topic: Mechanical Churn vs Directive Compliance
- Reviewer Position: Prefer Option B (flat)
- Advice:
  See R-010. M-002's example paths (`riscv/trap/interrupt`,
  `riscv/csr`) and the "upper dir of cpu/isa/device abstracts
  everything in arch" phrasing point to a flat layout at
  `arch/<arch>/<topic>/`. Option A's `pub use cpu::{csr, inst, mm,
  trap}` re-export hoist makes the *import path* topical but the
  *on-disk tree* still groups topics under `cpu/`. The directive
  asks for the latter to go away, not just to be papered over.
  The mechanical cost is modest — one extra `git mv` wave for each
  of `csr`, `mm`, `trap`, `inst`, plus a cross-topic `use` fixup
  (e.g. `trap/handler.rs` currently
  `use crate::cpu::riscv::RVCore` would become
  `use crate::arch::riscv::cpu::RVCore`). All `git mv` history
  composes.
- Rationale:
  M-002 is a MUST directive. "Reviewer-decidable" is not an escape
  hatch for MUST directives; they are binding unless MASTER waives
  them in the next iteration. The burden of proof for keeping
  Option A is on the executor, and the plan does not mount that
  case beyond "minimal diff."
- Required Action:
  Adopt Option B in round 02 unless MASTER issues an explicit waiver
  of M-002 for round 02. Remove T-5 from the Trade-offs list; the
  decision is settled.



---

## Positive Notes

- The Response Matrix is complete and honest: every prior CRITICAL /
  HIGH finding (R-001, R-002, R-003) and every MASTER directive
  (M-001, M-002) is listed with a specific resolution, not a stub
  claim. This is the shape AGENTS.md §Response Rules requires.
- The "Changes from Previous Round" block (Added / Changed / Removed /
  Unresolved) is unusually clear — it names exactly which round-00
  items are resolved, which are narrowed, and which are deliberately
  deferred, with named follow-up plans (`aclintSplit`, `plicGateway`,
  `directIrq`) rather than vague "out of scope" language.
- The enumeration of all 11 `pub(in …)` sites, cross-checked against
  `rg`, removes the round-00 R-002 execution risk cleanly.
- V-F-2's vocabulary allow-list (subject to the R-014 caveat) is the
  right enforcement shape for I-1 / I-2; a vocabulary grep catches
  the round-00 R-004 class of leak that a `crate::arch::…` path grep
  cannot.
- The `compile_error!(all(riscv, loongarch))` / `compile_error!(not
  any)` gates in `arch/mod.rs` tightly bind the `build.rs` contract
  to the source, so anyone manually injecting cfgs via `RUSTFLAGS`
  gets a useful error.
- NG-5 openly declares which MANUAL_REVIEW #3 residuals are *not*
  addressed and names their follow-up plans. The Summary's
  "partially addressed" framing matches.
- I-6 (per-phase green-bar) + the PR title list is the right shape
  for a chain-of-five landing.



---

## Approval Conditions

### Must Fix
- R-010 (resolve M-002 topic layout ambiguity — adopt flat layout per
  R-010's Option B, or obtain an explicit MASTER waiver in a
  `01_MASTER.md` update)
- R-011 (Phase 2 hoist requires visibility edits that C-2 forbids —
  either adopt flat layout per R-010 or name the visibility edits as
  an explicit C-2 exception and use `pub(crate) use` to match `mod
  mm`'s `pub(crate)` visibility)
- R-012 (pick one canonical path for the mip-bit paste target and
  use it everywhere: `arch/riscv/trap/interrupt.rs`)

### Should Improve
- R-013 (simplify V-UT-2: keep compile-time seam, drop redundant
  runtime CPU construction)
- R-014 (tighten V-F-2's NG-5 handling: line-level allow-list or
  per-pattern exclusion rather than whole-file allow)
- R-015 (clarify S2's green-bar claim: regression-avoidance, not
  seam proof)

### Trade-off Responses Required
- T-5 (adopt Option B flat layout; remove T-5 from open trade-offs
  once settled)

### Ready for Implementation
- No
- Reason: R-010 is CRITICAL because M-002 is a MASTER MUST directive
  and the plan's current Option A (nested-via-`pub use`) is not a
  compliant realisation of M-002's example paths. R-011 and R-012
  are HIGH and would cause Phase 2 / Phase 4 build breaks under the
  Option A wording. All three collapse into "pick flat layout" — a
  single-decision round-02 delta. Once flattened, validation and
  trade-off advice are in good shape and implementation can proceed.
