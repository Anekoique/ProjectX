# `archModule` REVIEW `00`

> Status: Open
> Feature: `archModule`
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
- Blocking Issues: `3`
- Non-Blocking Issues: `6`



## Summary

This is iteration 00, so there is no prior Response Matrix to reconcile and no
prior MASTER directive to carry forward. The plan is a clean, well-scoped
structural refactor that frames the right goals (consolidate the arch fan-out
under a single `arch/` tree) and correctly identifies the `cfg_if` seam as the
unification point. The phased landing sequence, the `git mv`-only constraint,
and the explicit preservation of the existing `CoreOps` contract are
well-considered.

However, the plan materially under-scopes the MANUAL_REVIEW #3 and #4 problem
as it stands. Three issues block a "ready for implementation" verdict:

1. The device layer has far more RISC-V leakage than the `mip` bit constants
   acknowledged in G-3. `device/intc/aclint.rs` and `device/intc/plic.rs` are
   end-to-end RISC-V artefacts (MSIP/MTIP/MEIP/SEIP, mtimecmp, hart
   contexts) and the plan proposes to leave them in `device/` unchanged.
   That directly contradicts MANUAL_REVIEW #3 ("bus design seems to target
   only RISC-V") and #4 ("move arch-specific behaviour into an `arch`
   directory").
2. The mechanical rewrite steps miss the 11 `pub(in crate::cpu::riscv)` and
   `pub(in crate::isa::riscv)` visibility declarations scattered across the
   RISC-V tree, which become invalid after `git mv`.
3. Validation rules V-F-1 and V-E-1 invoke cargo features (`--features
   loongarch`) that do not exist. `riscv`/`loongarch` are `rustc-cfg`s emitted
   by `xcore/build.rs` from the `X_ARCH` env var; the validation commands as
   written cannot be run.

There are also several narrower validation gaps (the grep assertion in V-F-2
is too narrow to enforce I-1, V-UT-2's canary is brittle, Phase 4's stub
`arch/loongarch/irq_bits.rs` will not compile cleanly against `device/mod.rs`
users of the constants). Trade-off framing is solid overall; agree with
Options A/A/A/B on T-1..T-4 but T-1's "defer `trait Arch`" position should
carry an explicit acknowledgement that it leaves MANUAL_REVIEW #3 only
*partially* resolved.

Implementation can proceed once the three blocking items above are addressed
in the next PLAN iteration (or explicitly waived by MASTER).

---

## Findings

### R-001 `device/intc leak is broader than irq_bits`

- Severity: CRITICAL
- Section: Spec / Goals / Architecture
- Type: Spec Alignment
- Problem:
  G-3 treats the only RISC-V leak in `device/` as the `mip` bit constants.
  In fact `device/intc/aclint.rs` and `device/intc/plic.rs` are entirely
  RISC-V-specific:
  - `aclint.rs` imports `device::{IrqState, MSIP, MTIP}`, implements SETSSIP
    edge delivery per the RISC-V ACLINT spec, and exposes `mtime`/`mtimecmp`
    (RISC-V machine-timer semantics).
    (`xemu/xcore/src/device/intc/aclint.rs:13`, `:60–86`)
  - `plic.rs` imports `device::{IrqState, MEIP, SEIP}` and hardcodes two
    hart contexts named by their RISC-V mip bits:
    `const CTX_IP: [u64; NUM_CTX] = [MEIP, SEIP];`
    (`xemu/xcore/src/device/intc/plic.rs:6`, `:21`)
  - `bus.rs` names `aclint_idx` / `plic_idx` as fields (`bus.rs:43–45`) and
    `mtime()` / `set_timer_source()` / `set_irq_sink()` bake RISC-V vocabulary
    into the arch-neutral bus.
  These files are the very "bus design seems to target only RISC-V"
  behaviour MANUAL_REVIEW #3 calls out, and they are the arch-specific
  dispatching that MANUAL_REVIEW #4 asks to move into an `arch/` tree.
- Why it matters:
  Moving only `cpu/riscv`, `isa/riscv`, and a handful of constants leaves
  the substantive arch leakage in place. After the refactor, a reader
  searching for "which files are RISC-V-specific" still has to read
  `device/intc/**` and `device/bus.rs` and learn by inspection. The plan's
  own Failure Flow #1 anticipates this ("if a top-level file still needs a
  concrete arch import after the move, that's an architecture smell") but
  the Implementation Plan does not act on it; it lands a refactor that
  leaves the smell intact.
- Recommendation:
  Either (a) expand Phase 4 to move `device/intc/aclint.rs` and
  `device/intc/plic.rs` under `arch/riscv/device/intc/` (with
  `device/intc/mod.rs` becoming a `cfg_if` seam analogous to `cpu/mod.rs`),
  or (b) add an explicit NG-5 stating that interrupt-controller relocation
  is a separately tracked follow-up plan (with a file/line pointer to where
  it will be tracked), and narrow the plan's Summary so it does not claim
  to resolve MANUAL_REVIEW #3. Option (a) is preferred because it gives a
  concrete, testable meaning to I-1 on the device side; option (b) is
  acceptable only if the Summary and G-* set are tightened so they do not
  over-claim.

### R-002 `pub(in crate::cpu::riscv) visibility paths will not survive git mv`

- Severity: CRITICAL
- Section: Execution Flow / Implementation Plan (Phase 2)
- Type: Correctness
- Problem:
  The RISC-V tree contains 11 `pub(in crate::cpu::riscv)` / `pub(in
  crate::isa::riscv)` declarations (ripgrep across `xemu/`):
  `cpu/riscv/csr.rs:1`, `cpu/riscv/trap.rs:5 occurrences`,
  `cpu/riscv/mm/tlb.rs:2`, `cpu/riscv/csr/ops.rs:2`,
  `cpu/riscv/mm/mmu.rs:1`. After `git mv xemu/xcore/src/cpu/riscv
  xemu/xcore/src/arch/riscv/cpu`, these `pub(in …)` paths no longer refer
  to a real module path and will produce `E0742` / visibility errors.
  The plan's Phase 2 step 5 only calls out `super::` and `crate::cpu::riscv::`
  paths; it does not mention the `pub(in …)` cohort, which is easy to
  overlook because `rg 'use crate::cpu::riscv'` does not find them.
- Why it matters:
  This is a hard compile break that blocks the Phase 2 "green bar" claim.
  Missing it in the plan makes the phased-landing guarantee (each phase
  green) unverifiable; an executor following the plan literally will land
  Phase 2 broken.
- Recommendation:
  Add an explicit Phase 2 substep: "Rewrite every
  `pub(in crate::cpu::riscv)` to `pub(in crate::arch::riscv::cpu)` and
  every `pub(in crate::isa::riscv)` to `pub(in crate::arch::riscv::isa)`;
  verify with
  `rg 'pub\(in crate::(cpu|isa)::(riscv|loongarch)' xemu/xcore/src` = 0
  hits." Also add the same rewrite rule to Phase 3 for LoongArch (even if
  none exist today, an invariant rule is stronger than a one-shot scan).

### R-003 `V-F-1 and V-E-1 reference cargo features that do not exist`

- Severity: HIGH
- Section: Validation / Failure-Robustness / Edge Cases
- Type: Validation
- Problem:
  V-F-1 says `cargo build --no-default-features --features loongarch`
  must compile; V-E-1 says `--features riscv,loongarch` must fail fast.
  But `xemu/xcore/Cargo.toml` exposes only `default`, `debug`, and
  `difftest` as features. The arch selection is a cfg emitted by
  `xemu/xcore/build.rs` from the `X_ARCH` environment variable
  (`build.rs:19–31`, values `riscv32`/`riscv64`/`loongarch32`/
  `loongarch64`). Running the commands in V-F-1 / V-E-1 as written will
  error with "package 'xcore' does not have feature 'loongarch'".
- Why it matters:
  Two of the four failure-mode validators are unrunnable. V-F-1 is the
  primary mechanism for "flushing hidden RISC-V deps out of top-level
  modules," i.e. the main way the plan proves G-2 on the LoongArch side.
  Without a working LoongArch build invocation, I-1 for LoongArch is
  asserted but not tested.
- Recommendation:
  Replace the feature invocations with the actual env-var form and pin it
  to the `Makefile` if one exists:
  `X_ARCH=loongarch32 cargo check -p xcore` (and `loongarch64`). For
  V-E-1, document how the build is supposed to fail when two arch cfgs
  are set simultaneously — today `build.rs` picks exactly one based on
  `X_ARCH`, so V-E-1's "dual feature flags" scenario is structurally
  impossible; either retarget V-E-1 to "manually injecting both cfgs via
  RUSTFLAGS produces a clear compile error via a `compile_error!` in
  `arch/mod.rs`" or drop V-E-1. Also add a concrete LoongArch-side
  acceptance bar since the current one ("clean `cargo check`") admits a
  trivially passing stub.

### R-004 `V-F-2 grep is insufficient to enforce I-1`

- Severity: HIGH
- Section: Validation / Failure-Robustness / Invariants
- Type: Invariant / Validation
- Problem:
  I-1 says no file outside `arch/` may reference arch-specific concepts
  except through the `cfg_if` seam. V-F-2's grep is
  `grep -R "crate::arch::riscv\|crate::arch::loongarch" xemu/xcore/src`
  minus `arch/`, `cpu/mod.rs`, `isa/mod.rs`, `device/mod.rs`. That pattern
  only catches files that explicitly import via `crate::arch::…`. It does
  *not* catch:
  - `device/intc/aclint.rs` (imports `MSIP, MTIP` from `device::` — after
    refactor these are re-exported from `device/mod.rs`, so the file still
    compiles without ever saying `crate::arch::riscv::…`),
  - `device/intc/plic.rs` (same pattern with `MEIP, SEIP`),
  - `device/bus.rs` (RISC-V-named `aclint_idx`/`plic_idx` fields and
    `mtime()` method),
  - any future file that reaches through `crate::cpu::…` or `crate::isa::…`
    to a transitively re-exported RISC-V type.
  So a file can violate I-1 in spirit while V-F-2 is green.
- Why it matters:
  V-F-2 is the only mechanical enforcement of I-1. If it is known to be
  under-specified, I-1 is aspirational rather than enforced, and the
  refactor's core promise ("every arch-specific file lives under exactly
  one folder per arch") is not testable.
- Recommendation:
  Tighten V-F-2 to an allow-list instead of a deny-list: pick a small set
  of RISC-V vocabulary strings (`MSIP`, `MTIP`, `MEIP`, `SEIP`, `mtimecmp`,
  `aclint`, `plic`, `hart`, `RVCore`, `Mstatus`, `Mip`) and grep for them
  outside `arch/`, the seam files, and an explicit allow-list. Expect
  the first run to surface the `device/intc/**` and `device/bus.rs`
  violations, which is exactly the signal the plan claims to provide
  — and is the evidence that drives R-001.

### R-005 `V-UT-2 canary is brittle`

- Severity: MEDIUM
- Section: Validation / Unit Tests
- Type: Validation
- Problem:
  V-UT-2 asserts that
  `core::any::type_name::<arch::selected::cpu::Core>()`
  contains `"riscv"`. `Core` is a `pub use RVCore as Core` alias; on
  stable Rust `type_name` is guaranteed to be "best-effort" and the
  *spelling* of path segments is explicitly allowed to change across
  compiler versions. Furthermore, after the refactor the path will be
  `xcore::arch::riscv::cpu::RVCore` — which contains `"riscv"`, but an
  equally valid refactor that renames the selector to `rv` or `rv64gc`
  would silently break the canary without any real bug.
- Why it matters:
  A canary test that keys on a spelling convention rather than a
  behavioural property produces noise rather than a regression signal.
- Recommendation:
  Either drop V-UT-2, or replace it with a real behavioural canary —
  e.g. assert that `arch::selected::cpu::Core::new().pc()` equals
  `RESET_VECTOR` on the default feature set, which binds the seam to an
  observable arch-specific default rather than a type-name string. A
  compile-time assertion like
  `const _: fn() = || { let _: arch::selected::cpu::Core; };` would
  also suffice as a "seam wired" gate without the brittleness.

### R-006 `arch/loongarch/irq_bits.rs stub is under-specified`

- Severity: MEDIUM
- Section: Implementation Plan (Phase 4)
- Type: Correctness
- Problem:
  Phase 4 says "add an empty (or feature-gated) `arch/loongarch/irq_bits.rs`
  stub so the `selected` alias resolves in LoongArch builds," and
  `device/mod.rs` will do
  `pub use crate::arch::selected::irq_bits::{SSIP, MSIP, STIP, MTIP, SEIP,
  MEIP, HW_IP_MASK}`. An empty stub will not resolve those names; a
  non-empty stub has to invent LoongArch values for symbols that are
  RISC-V vocabulary. This is the same spec mismatch as R-001 in
  miniature — the plan re-exports RISC-V-named constants from the
  arch-neutral `device/` even on non-RISC-V targets.
- Why it matters:
  Phase 4 cannot be "green in both arch configurations" as claimed
  without either (a) defining fake LoongArch `MSIP`/`SEIP` symbols
  (wrong: those are RISC-V mip bit positions), or (b) gating the
  `device/mod.rs` re-exports behind `cfg(riscv)`. Neither option is in
  the plan.
- Recommendation:
  Gate the `device/mod.rs` re-export on `cfg(riscv)` (or move it into a
  `cfg_if` seam that picks a per-arch bit-constants module, and only
  `arch/riscv/irq_bits.rs` exists). Drop the placeholder
  `arch/loongarch/irq_bits.rs` entirely — consumers on LoongArch should
  fail to find RISC-V-named constants, which is the correct enforcement.
  Document this in Phase 4 so the Phase 3 LoongArch `cargo check` is
  honest.

### R-007 `RVCore construction wiring not accounted for`

- Severity: MEDIUM
- Section: Implementation Plan (Phase 2)
- Type: Correctness
- Problem:
  `cpu/riscv/mod.rs:58–104` constructs Aclint/Plic/Uart/TestFinisher/
  VirtioBlk at fixed MMIO addresses and pulls their types via
  `crate::device::{bus::Bus, intc::{aclint::Aclint, plic::Plic},
  test_finisher::TestFinisher, uart::Uart, virtio_blk::VirtioBlk}`. After
  `git mv cpu/riscv → arch/riscv/cpu`, these imports still resolve
  (they go through `crate::device::…`, which does not move), so the
  file compiles — but only because R-001 is unresolved. If R-001 is
  taken (move intc into arch/), the `use` list here must flip to
  `crate::arch::riscv::device::intc::…`, which is an intra-arch path
  and does not violate I-1. The current plan doesn't discuss this
  bridge.
- Why it matters:
  This is the one file where "arch code happens to construct device
  code" collides with the refactor. The plan needs a clear statement
  of which side owns the wiring. If devices stay in `device/`, this is
  a cross-module coupling point worth documenting; if devices move,
  this is the one arch-local place that is allowed to import them.
- Recommendation:
  Decide R-001 first; if device/intc moves, add an explicit "update
  `arch/riscv/cpu/mod.rs` device imports from `crate::device::intc::…`
  to `crate::arch::riscv::device::intc::…`" substep to Phase 4 and
  confirm Phase 2 still builds without this step (it does, because the
  old paths are still valid during Phase 2). If device/intc stays,
  add an explicit note in Architecture: "RISC-V core construction
  depends on `device::intc::*` by design; these are the MANUAL_REVIEW
  #3 residuals not addressed by this plan."

### R-008 `Phase-boundary green-bar claim for Phase 3 is weak`

- Severity: LOW
- Section: Execution Flow / State Transitions
- Type: Validation
- Problem:
  S3 claims "build green in both `riscv` and `loongarch` feature
  configurations" after Phase 3, but the current LoongArch tree is two
  ~5-line stubs (`cpu/loongarch/mod.rs`, `isa/loongarch/mod.rs`).
  `cargo check` in the loongarch configuration today does not exercise
  any meaningful code path; after relocation it exercises the same
  amount. "Green" here is a near-tautology.
- Why it matters:
  The phased-landing narrative sells Phase 3 as meaningful coverage;
  it is mostly moving two near-empty files. That is fine, but the plan
  should not claim more than it delivers.
- Recommendation:
  Downgrade the Phase 3 acceptance bar to "`X_ARCH=loongarch32 cargo
  check -p xcore` succeeds (LoongArch remains a stub; meaningful
  LoongArch coverage is out of scope)" in the Implementation Plan and
  in V-F-1.

### R-009 `lib.rs mod arch registration ordering not specified`

- Severity: LOW
- Section: Implementation Plan (Phase 1)
- Type: Maintainability
- Problem:
  Phase 1 step 2 says "Register `mod arch;` in `lib.rs` (before `mod
  cpu;` and `mod isa;` so they can refer to it)". Rust's module order
  in the crate root does not affect whether `crate::arch::…` paths
  resolve inside `cpu/mod.rs` — module declarations are visible
  crate-wide regardless of textual order. The comment is incorrect and
  may cause confusion or a trivial style bike-shed.
- Why it matters:
  Minor, but gives a wrong mental model to future readers.
- Recommendation:
  Drop the parenthetical; just state "add `mod arch;` alongside the
  existing `mod cpu;` / `mod isa;` declarations in `xcore/src/lib.rs`."

---

## Trade-off Advice

### TR-1 `Defer trait Arch vs adopt now`

- Related Plan Item: `T-1`
- Topic: Clean Design vs Diff Size
- Reviewer Position: Prefer Option A (defer)
- Advice:
  Agree with the plan's recommendation to keep the `cfg_if` seam and
  defer a `trait Arch { type Word; ... }` refactor. But the plan's
  Summary should *not* claim this closes MANUAL_REVIEW #3. Relocation
  + `cfg_if` addresses MANUAL_REVIEW #4 (the "redundant dispatching
  across riscv/loongarch directories" smell) directly; it only
  partially addresses MANUAL_REVIEW #3 (the "bus design seems to
  target only RISC-V" smell), because the bus is still full of
  RISC-V vocabulary (see R-001). A follow-up `archTrait` or
  `archBus` plan should be explicitly forward-referenced in NG-2.
- Rationale:
  A `trait Arch` refactor is legitimately large and easy to regress.
  But without one, enforcement of "bus is arch-neutral" falls to
  ACLINT/PLIC relocation (R-001), and that relocation must happen
  either in this plan or in a named follow-up. Deferring the trait
  is fine; deferring the relocation without naming a follow-up is
  not.
- Required Action:
  Keep Option A. Update the Summary to narrow the MANUAL_REVIEW #3
  claim to "partially addressed"; add a forward reference in NG-2 to
  a planned `archTrait` or `archBus` iteration.

### TR-2 `Back-compat re-exports vs hard cut`

- Related Plan Item: `T-2`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A
- Advice:
  Agree with Option A (keep `pub use` re-exports at old paths). The
  downstream (`xdb`, `xam`, `xemu` binary) is not the problem this
  plan is solving, and dragging those crates into a rename churn
  would obscure the signal.
- Rationale:
  I-3 explicitly guarantees downstream compiles unchanged, and the
  review template is scoped to this plan. A follow-up cleanup PR
  can do the hard cut once MANUAL_REVIEW #3 is fully resolved.
- Required Action:
  Keep as is. No plan change required.

### TR-3 `IrqState location`

- Related Plan Item: `T-3`
- Topic: Premature Generalisation vs Now-ness
- Reviewer Position: Prefer Option A — but flag conditional
- Advice:
  Agree with keeping `IrqState` in `device/` for this plan. However,
  the rationale ("storage is arch-neutral; only the bit-layout
  semantics are arch-specific") is slightly off: `IrqState::set/clear`
  takes a `u64` bit mask that only has meaning against a specific
  arch's interrupt-pending layout. If R-001 is taken and ACLINT/PLIC
  move into `arch/riscv/`, then `IrqState` as the shared vocabulary
  between them still makes sense in `device/`; but if they don't
  move, the argument is weaker.
- Rationale:
  The MANUAL_REVIEW #5/#6 follow-up (external devices talking to
  PLIC directly, async IRQ) will touch `IrqState`'s shape; moving it
  now would collide with that design.
- Required Action:
  Keep Option A. In the plan, clarify that `IrqState` is arch-neutral
  *storage* for arch-specific *bits*, and that the bit-semantics live
  in `arch/<name>/irq_bits.rs` (per G-3). Once R-001 is resolved the
  wording can be tightened further.

### TR-4 `Single PR vs phased PRs`

- Related Plan Item: `T-4`
- Topic: Reviewability vs Merge Overhead
- Reviewer Position: Prefer Option B (phased)
- Advice:
  Agree with phased PRs. Concretely: Phase 1 (skeleton), Phase 2
  (RISC-V move + `pub(in …)` rewrite per R-002), Phase 3+4 (LoongArch
  + irq_bits relocation), Phase 5 (docs). Each PR gives a distinct
  test matrix to check.
- Rationale:
  Four `git mv` walls in one PR produce unreviewable diffs. Per-phase
  landing also lets each phase be reverted independently if regression
  appears in `make linux` / `make debian`.
- Required Action:
  Keep Option B. In the Implementation Plan, add per-PR title
  proposals so the executor's PR chain is predictable.

---

## Positive Notes

- The plan correctly identifies that the unification point is the
  existing `cfg_if` seam, not a new trait layer; this is the right
  call for ProjectX's current single-live-arch state.
- G-4 and C-3 (preserve git history via `git mv`) are well-chosen
  mechanical constraints that are rare to see spelled out explicitly
  and worth keeping.
- The "State Transition" list (S0..S5) gives a good sequential mental
  model; once the blocking items are fixed, this should be the
  reviewer's checklist for landing each phase.
- T-4 framing (phased vs single PR) and the recommendation of per-phase
  green-bar are well-motivated.
- The MANUAL_REVIEW references in the Summary point the reader at
  concrete problems rather than inventing new goals — good scope
  discipline for iteration 00.

---

## Approval Conditions

### Must Fix
- R-001 (device/intc RISC-V leakage — either move into `arch/` or
  narrow the Summary + add NG forward-reference)
- R-002 (rewrite the 11 `pub(in crate::…)` visibility paths as an
  explicit Phase 2/3 substep)
- R-003 (replace `--features loongarch` with `X_ARCH=loongarch32` in
  V-F-1 / V-E-1, or drop V-E-1)

### Should Improve
- R-004 (tighten V-F-2 to a vocabulary allow-list rather than a
  `crate::arch::…` deny-list)
- R-005 (replace V-UT-2 type-name canary with a behavioural or
  compile-time check)
- R-006 (clarify the LoongArch `irq_bits` stub story; gate
  `device/mod.rs` re-exports on `cfg(riscv)`)
- R-007 (document/plan the RVCore construction wiring once R-001 is
  resolved)

### Trade-off Responses Required
- T-1 (accept Option A with narrowed MANUAL_REVIEW #3 claim per
  TR-1)
- T-3 (keep Option A; tighten the rationale wording per TR-3)

### Ready for Implementation
- No
- Reason: R-001 and R-002 are CRITICAL and would cause the
  implementation to either silently under-deliver MANUAL_REVIEW #3
  (R-001) or break the Phase 2 green-bar outright (R-002). R-003 is
  HIGH and leaves LoongArch-side G-2 enforcement untested. Address
  these in `01_PLAN.md` (or waive via `00_MASTER.md`), then
  implementation can proceed.
