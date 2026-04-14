# `multiHart` REVIEW `04`

> Status: Closed
> Feature: `multiHart`
> Iteration: `04`
> Owner: Reviewer
> Target Plan: `04_PLAN.md`
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

Round 04 cleanly executes the user-ordered pivot. `Hart` is gone,
`RVCore` IS the hart, and `CPU<Core>` owns `Vec<Core> + Bus +
current`. The pivot is structurally sound: ground truth at
`arch/riscv/cpu/mod.rs:30-49` confirms `RVCore` already holds
exclusively per-hart state (only `bus: Bus` at line 39 was shared)
so collapsing `Hart` into `RVCore` is a true simplification, not a
relabel. The borrow-checker design (`split_current_mut` returning
`(&mut Core, &mut Bus)` via disjoint field access) is the textbook
Rust pattern and will compile. Bus threading through 8 `mm.rs`
methods is mechanical and type-system-guided.

Every prior-round finding (R-001..R-024, TR-3/6/7/8, R-025(a))
carries cleanly; the Response Matrix maps them with section
pointers. Inherited MASTER directives (00-M-001/002, 01-M-001..004)
are honoured — the new `MachineBuilder` trait at `cpu/core.rs`
specifically preserves 01-M-004 (no RISC-V types leak into
`CPU::from_config`).

Test arithmetic is internally consistent: PR1 354 + 11 = 365 lib
+ 1 + 6 = 372; PR2a 366 lib = 373; PR2b 369 lib = 376. The two
folded V-UT-1/V-UT-2 assertions reasonably re-home into existing
`RVCore::new` / `reset` tests since the additions (`id` seeding,
`last_store` clearing) are property checks on the existing
constructors.

One blocking issue: **R-026 (HIGH) — `HartId` re-export at
`cpu/core.rs` will fail `arch_isolation`**. The plan claims I-7
holds with no seam-test edits, but
`xemu/xcore/tests/arch_isolation.rs:31-37` shows `SEAM_FILES` does
**not** include `src/cpu/core.rs`. If executor adds `pub use
crate::arch::riscv::cpu::HartId;` to `cpu/core.rs` per step 1, the
seam test will detect a non-seam file referencing
`crate::arch::riscv::` and fail. Either the seam allow-list must
be widened in PR1 (with `HartId` added to `SEAM_ALLOWED_SYMBOLS`),
or `HartId` must be defined directly at `cpu/core.rs` as a plain
newtype owned by the trait layer (the C-3 framing already calls
`HartId` "the minimum surface" with "no RISC-V semantics"). The
plan must pick one and update I-7 / Response Matrix accordingly.

Three MEDIUM/LOW residuals: R-027 (`CPU::from_config<B>` API
generic produces awkward turbofish at call sites; the `B`
parameter should bind to a concrete type alias or be replaced by
an associated impl), R-028 (`take_last_store` consumption order in
`CPU::step` reads `self.cores[self.current]` *after*
`self.current` has been used as the source index — careful: the
plan's pseudocode is correct, but rationale should be explicit
that `take_last_store` happens **before** `self.current +=` so
`src` and consumer index match), R-029 (plan body sits at C-7
ceiling exactly — same R-025 condition as round 03, carried
forward).

One NIT: R-030 — V-IT-3 ("round_robin_fairness_single_hart") at
`num_harts = 1` is degenerate and asserts only that `current`
stays at 0 after one step then wraps; consider whether this earns
the integration-test slot or should fold into a unit test.

Approve with R-026 fixed. R-027/R-028 are clarification asks; R-029
is a known-accepted carried trade. The plan is implementation-ready
once R-026 is resolved.



---

## Findings

### R-026 `HartId` re-export at `cpu/core.rs` violates `arch_isolation`

- Severity: HIGH
- Section: Spec / Invariants / I-7 + Implement / step 1
- Type: Spec Alignment
- Problem:
  Step 1 (line 414) and I-7 (line 260-262) both state `HartId` is
  defined in `arch/riscv/cpu/mod.rs` and **re-exported at
  `cpu/core.rs`** as a "plain newtype" so `CoreOps::id` can return
  it without leaking RISC-V semantics. Ground truth at
  `xemu/xcore/tests/arch_isolation.rs:31-37` shows the
  `SEAM_FILES` allow-list contains exactly five entries:
  `src/arch/mod.rs`, `src/cpu/mod.rs`, `src/isa/mod.rs`,
  `src/device/mod.rs`, `src/device/intc/mod.rs`. **`src/cpu/core.rs`
  is not on this list.** Adding `pub use
  crate::arch::riscv::cpu::HartId;` in `cpu/core.rs` will trigger
  the non-seam `crate::arch::riscv::` reference detector and fail
  V-IT-1 (`arch_isolation`).
  
  `SEAM_ALLOWED_SYMBOLS` (lines 42-65) also does not include
  `HartId`. Even widening `SEAM_FILES` to include `src/cpu/core.rs`
  is incomplete without a corresponding symbol entry.
- Why it matters:
  V-IT-1 is the round-04 PR1 baseline gate (`+ 1` in the
  `365 + 1 + 6 = 372` arithmetic at G-7). A failing arch_isolation
  blocks PR1. The plan asserts I-7 "passes unchanged" (line
  260-261); this is incorrect under the current SEAM definition.
  Inherited 00-M-001 (no global `Arch` trait) and 00-M-002 (no new
  top-level seam files) are both adjacent to this question, so
  the resolution must be deliberate, not silent.
- Recommendation:
  Pick one of the following in `05_PLAN.md` (or as an inline 04
  amendment if executor prefers):
  
  **(a) Define `HartId` directly at `cpu/core.rs`** as a
  plain `pub struct HartId(pub u32);` owned by the trait layer.
  C-3 already frames `HartId` as having "no RISC-V semantics";
  defining it where `CoreOps::id` returns it is structurally
  cleaner and requires zero seam-test edits. `arch/riscv/cpu`
  imports `HartId` from `cpu::core` (a downward dependency, not a
  re-export). **Recommended** — minimal churn, no MASTER drift.
  
  **(b) Widen `SEAM_FILES` to include `src/cpu/core.rs`** AND add
  `HartId` to `SEAM_ALLOWED_SYMBOLS`. Update I-7 to enumerate
  both edits and the rationale (cpu/core.rs becomes a trait-seam
  re-export site for arch-owned types). Higher-friction; needs
  MASTER acknowledgement that the seam vocabulary grows.
  
  Whichever option, update step 1, I-7, C-3, and the Response
  Matrix row for I-7. If (b), update the V-IT-1 / arch_isolation
  invariant statement explicitly.



### R-027 `CPU::from_config<B>` generic shape produces awkward call sites

- Severity: MEDIUM
- Section: Spec / API Surface (line 351-352)
- Type: API
- Problem:
  Plan signature:
  
  ```rust
  pub fn from_config<B>(config: MachineConfig, layout: BootLayout) -> Self
      where B: MachineBuilder<Core = Core>;
  ```
  
  The `B` type parameter does not appear in the parameter list or
  return type — only in the where-clause as a phantom selector.
  Rust cannot infer `B`, so every call site requires turbofish:
  `CPU::<Core>::from_config::<RVCore>(...)`. With both the type
  parameter (`Core`) and method generic (`B`) needing explicit
  binding, the call site reads as a structurally-confused double
  generic when (today) there is exactly one `Core: RVCore`
  alias and one `MachineBuilder: RVCore`.
- Why it matters:
  The whole point of the seam pin (T-13) is that `CPU::from_config`
  stays arch-agnostic. But the *callers* (likely `xdb/src/main.rs`
  and any test fixture) will need to know `RVCore` as the
  builder, which is the same RISC-V leak the trait was designed
  to prevent — it just moves the leak from the function body to
  the call-site turbofish. Today's `RVCore::with_config` at
  `arch/riscv/cpu/mod.rs:58` is called only from the `XCPU`
  init path and tests; the new shape multiplies the friction.
- Recommendation:
  Two acceptable resolutions:
  
  **(a)** Drop the `<B>` method generic. Add a cfg-gated `pub type
  CoreBuilder = crate::arch::riscv::cpu::RVCore;` alias at
  `cpu/mod.rs` next to the existing `pub type Core = …`, and
  bind `B = CoreBuilder` inside `from_config`'s body. This makes
  `CPU::from_config(config, layout)` a direct call with no
  turbofish at any seam site, and the alias lives in the existing
  SEAM_FILES set (`src/cpu/mod.rs`). The trait is still arch-
  agnostic; only the *selector* binds at the seam.
  
  **(b)** Document the call-site shape explicitly in the plan
  (showing the turbofish form and where it's used) so executor
  doesn't discover the friction at integration. Lower quality but
  still acceptable if the call site count is genuinely ≤ 2.
  
  Option (a) is preferred. Update §API Surface and add a brief
  rationale paragraph after the trait definition in §Architecture.



### R-028 `CPU::step` ordering: `take_last_store` must consume before cursor advance

- Severity: LOW
- Section: Spec / Architecture (line 167-188) + I-8 (line 263-267)
- Type: Flow
- Problem:
  The pseudocode at lines 170-183 reads:
  
  ```rust
  let (core, bus) = self.split_current_mut();
  let result = core.step(bus);
  result?;
  if let Some((addr, size)) = self.cores[self.current].take_last_store() {
      self.invalidate_reservations_except(self.current, addr, size);
  }
  …
  self.current = (self.current + 1) % self.cores.len();
  ```
  
  This is correct — `take_last_store` is consumed via index
  `self.current` (the source hart) and then `self.current` is
  advanced. But the order is load-bearing for I-8 ("`CPU::step`
  consumes via `take_last_store()`") and the plan does not state
  *why* the consume must precede the advance. A future refactor
  that reorders these two lines would silently break peer-
  invalidation routing (the `src` index passed to
  `invalidate_reservations_except` would be the *next* core, not
  the one that wrote).
- Why it matters:
  I-8 is the load-bearing invariant for cross-hart LR/SC
  correctness (G-10). The pseudocode order is correct *today* but
  there is no anchor preventing accidental swap. V-UT-11 / V-UT-13
  / V-UT-14 will catch incorrect peer-invalidation, but only if
  exercised at `num_harts >= 2`; at `num_harts == 1` the
  invalidation loop is a no-op (line 228) and a wrong order would
  pass.
- Recommendation:
  Add a one-sentence comment in §Architecture pseudocode at line
  175 (e.g., "// MUST happen before `self.current` advances —
  `src` parameter must equal the index of the core that wrote.")
  and add a sentence to I-8 (line 263-267) stating the consume-
  before-advance ordering as part of the post-condition.
  Optionally add a `debug_assert!` in
  `invalidate_reservations_except` that `src < self.cores.len()`.



### R-029 Plan body sits at the C-7 720-line ceiling exactly

- Severity: LOW
- Section: Spec / Constraints / C-7
- Type: Maintainability
- Problem:
  `wc -l 04_PLAN.md` returns 720, matching the C-7 budget exactly
  (R-025(a) relaxed the budget from 700 → 720). Identical
  condition to round 03's R-025: any inline edit during PR1
  implementation pushes the plan over budget and forces either a
  silent C-7 violation or another constraint relaxation.
- Why it matters:
  Carry-forward of R-025. Audit-trail hygiene. With three R-026
  / R-027 / R-028 fixes likely landing as plan amendments, the
  20-line headroom of round 03 → round 04 is already consumed.
- Recommendation:
  Two acceptable resolutions, executor's choice:
  (a) Trim the duplicative bullet in §Trade-offs (T-1..T-13 each
  get 1-2 lines; T-2/T-3/T-5/T-6/T-7 could collapse to one-line
  references to prior-round T-* entries) to reclaim ~10 lines for
  R-026/R-027/R-028 amendments.
  (b) Relax C-7 again in `05_PLAN.md` to `≤ 750` with a one-line
  Log entry. Cheaper but starts a creep pattern.
  Non-blocking; flag in PR1 description if grazed.



### R-030 V-IT-3 at `num_harts = 1` is a degenerate fairness check

- Severity: LOW
- Section: Validation / Unit Tests — PR1 (line 630)
- Type: Validation
- Problem:
  `V-IT-3 round_robin_fairness_single_hart` (line 630) is listed
  as an integration test asserting G-5 ("round-robin in
  `CPU::step`") at `num_harts = 1`. The assertion is degenerate:
  with one core, `current` cycles `0 → 0 → 0 …`, which proves
  only that `(0 + 1) % 1 == 0`. This is a 1-line property test,
  not an integration test, and provides no signal about
  multi-hart fairness.
- Why it matters:
  V-IT-4 (PR2b) covers the actual two-hart round-robin. V-IT-3
  inflates the integration-test count without adding coverage.
  Acceptance Mapping (line 697) maps G-5 to "V-IT-3, V-IT-4,
  V-E-3" — V-IT-4 + V-E-3 already cover the meaningful cases.
- Recommendation:
  Either (a) demote V-IT-3 to a unit test in `cpu/mod.rs::tests`
  (it asserts `cpu.current == 0` after step at `num_harts = 1`),
  or (b) drop V-IT-3 entirely and rely on V-IT-4 / V-E-3 (the
  existing PR1 lib-test bus + I-4 byte-identical gate are
  sufficient at single-hart). Update G-7 test arithmetic if (b)
  reduces the lib count by 1 (365 → 364 lib + 1 + 6 = 371).



---

## Trade-off Advice

### TR-9 `MachineBuilder` trait shape — generic vs associated type

- Related Plan Item: T-13
- Topic: Flexibility vs Simplicity (API ergonomics)
- Reviewer Position: Prefer revision (see R-027(a))
- Advice:
  The trait shape itself is correct — externalizing
  arch-specific MMIO wiring keeps `CPU::from_config`
  arch-agnostic and honours 01-M-004. But the `<B>` method
  generic on `from_config` is the wrong way to bind the impl:
  `B` is purely a phantom selector and forces turbofish at
  every seam call site. A cfg-gated `pub type CoreBuilder = …`
  alias next to the existing `Core` alias eliminates the
  generic without compromising the trait abstraction.
- Rationale:
  The seam vocabulary already binds `Core` to `RVCore` via
  `pub type` at `cpu/mod.rs:41`. A parallel `CoreBuilder`
  alias is consistent, requires zero new SEAM_FILES entries,
  and is one line of code. The trait at `cpu/core.rs` stays
  arch-agnostic; only the *selector* lives at the existing
  seam.
- Required Action:
  Adopt R-027(a): drop `<B>`, add `pub type CoreBuilder` at
  `cpu/mod.rs`, update `from_config` to bind via the alias.
  Note in §API Surface that the generic was reduced to a
  type alias for ergonomics.



### TR-10 Test fixture migration approach for `setup_core_bus`

- Related Plan Item: Step 14 (line 473-481)
- Topic: Compatibility vs Clean Design
- Reviewer Position: Endorse current plan
- Advice:
  Step 14's plan to replace `setup_core() -> RVCore` with
  `setup_core_bus() -> (RVCore, Bus)` plus a thin single-return
  wrapper is the right shape. Estimated ~60 call site edits
  (per the plan's grep tally) is mechanical but unavoidable.
  No alternative (e.g., a `RVCore::with_default_bus()` test-only
  helper that internally builds a Bus and stores it as a field
  for one-step access) survives the architectural pivot — the
  whole point is `RVCore` no longer owns a `Bus`.
- Rationale:
  The clean signature `core.step(&mut bus)` matches the
  production `CoreOps::step` exactly, so test fixtures and
  production code share the same calling convention. A wrapper
  that hides the bus would obscure the very invariant being
  tested in V-UT-11..14 (cross-hart visibility).
- Required Action:
  Keep current step 14 plan. Optionally add an example test
  fixture pattern (5-line snippet) to the §Implement section so
  executor has an anchor for the ~60 mechanical migrations.



---

## Positive Notes

- **The pivot is structurally correct.** Ground truth at
  `arch/riscv/cpu/mod.rs:30-49` confirms `RVCore` already holds
  exclusively per-hart state (only `bus: Bus` at line 39 was
  shared). Collapsing `Hart` into `RVCore` eliminates a
  redundant indirection. The user's framing ("`RVCore` mean a
  hart seems reasonable") matches the actual ownership graph.
- **`split_current_mut` is the textbook disjoint-borrow pattern.**
  Lines 185-188 show the helper as a 3-line method returning
  `(&mut self.cores[self.current], &mut self.bus)`. Rust's
  borrow checker accepts disjoint field access via
  `IndexMut::index_mut` + plain field access; this will
  compile at the first attempt. V-F-7 correctly designates
  this as a compile-time gate.
- **Bus-threading scope is bounded and correct.** Verified the
  8 methods at `mm.rs:253, 264, 271, 278, 283, 294, 306, 318,
  323` all touch `self.bus` (5 unique funnels: `access_bus`,
  `checked_read`, `checked_write`, `translate`, plus the
  fetch/load/store/amo_* wrappers). Plan's enumeration is
  exhaustive. `arch/riscv/cpu/debug.rs:94, 99, 103` also touch
  `self.bus` but these are `DebugOps` paths consumed via
  `CPU::debug_ops()`; threading the bus through them is a
  separate question handled by the `CPU::current()` /
  `CPU::bus()` accessors (correct).
- **R-020 hook placement on `RVCore::checked_write` is sound.**
  Same `mm.rs:271` location confirmed in round 03; only callers
  remain `Hart::store` / `Hart::amo_store` (now `RVCore::store`
  / `RVCore::amo_store` at lines 306, 323). The `op` gate
  (`matches!(op, MemOp::Store | MemOp::Amo)`) defends against
  future Fetch / Load callers as before.
- **`CPU::step` external signature is preserved.** Verified
  `xdb/src/cmd.rs:37` calls `cpu.step()` and
  `xdb/src/difftest/mod.rs:56` calls `self.backend.step()` —
  both at the `CPU` level, not `Core`. Plan's claim "external
  signature unchanged" holds for both call sites; xdb requires
  zero churn.
- **`bus.tick` relocation rationale is correct (T-10).** Moving
  `bus.tick()` from `RVCore::step` (currently at `mod.rs:225`)
  to `CPU::step` once-per-fairness-cycle is the only sound
  policy at `num_harts > 1` — per-core ticking would over-tick
  device timing. At `num_harts = 1` it's byte-identical.
- **ACLINT per-hart shapes match the spec.** The plan correctly
  identifies MSWI stride 4, MTIMER mtimecmp stride 8, SSWI
  stride 4 (line 250) — these match RISC-V ACLINT spec exactly.
  Per-device `Vec<IrqState>` length `num_harts` is the right
  shape.
- **PLIC PR2a reshape is well-scoped.** Dropping `NUM_CTX` /
  `CTX_IP` constants (verified at
  `arch/riscv/device/intc/plic.rs:12, 22`) and recomputing
  `num_ctx = 2 * num_harts` with `ctx & 1` parity selecting
  MEIP / SEIP is exactly the right shape for the
  M-mode-context-then-S-mode-context layout. The 14 existing
  tests (verified by `grep -c '#\[test\]'`) regression-block at
  `Plic::new(1, vec![irq])` is sound.
- **Response Matrix is complete.** Every prior-round finding
  (R-001..R-024, TR-3/6/7/8, R-025) plus the user pivot is
  enumerated with section pointers. Inherited MASTER directives
  (00-M-001/002, 01-M-001..004) all have applied-status notes.
  No silent drift.
- **The pivot net-shrinks the codebase.** Removing the `Hart`
  struct + `CoreOps::{bus, bus_mut}` methods is genuine
  simplification, not refactoring churn. Honours 01-M-002 (clean
  concise).



---

## Approval Conditions

### Must Fix
- **R-026** (HIGH) — `HartId` re-export at `cpu/core.rs` will
  fail `arch_isolation`. Adopt option (a) defining `HartId` at
  `cpu/core.rs` directly, or option (b) widening `SEAM_FILES` +
  `SEAM_ALLOWED_SYMBOLS`. Update step 1, I-7, C-3, Response
  Matrix.

### Should Improve
- **R-027** (MEDIUM) — Drop the `<B>` method generic on
  `CPU::from_config`; bind via a cfg-gated `pub type
  CoreBuilder` alias at `cpu/mod.rs`.
- **R-028** (LOW) — Add a one-line comment + I-8 sentence
  pinning the `take_last_store` → cursor-advance ordering in
  `CPU::step`.
- **R-029** (LOW) — Plan body at C-7 ceiling exactly (720
  lines); trim or relax during R-026 amendment.
- **R-030** (LOW / NIT) — V-IT-3 single-hart fairness is
  degenerate; demote to unit test or drop in favour of V-IT-4
  + V-E-3.

### Trade-off Responses Required
- **TR-9** — Adopt R-027(a) (drop `<B>`, add `CoreBuilder`
  alias), or justify keeping the method generic with a concrete
  call-site analysis.
- **TR-10** — No action; current step 14 endorsed.

### Ready for Implementation
- No
- Reason: R-026 (HIGH) is a blocker — the seam test
  `arch_isolation` will fail at PR1 with the planned
  `pub use crate::arch::riscv::cpu::HartId;` at `cpu/core.rs`.
  Verified by reading `xemu/xcore/tests/arch_isolation.rs:31-37`
  (`SEAM_FILES` does not include `src/cpu/core.rs`) and lines
  42-65 (`SEAM_ALLOWED_SYMBOLS` does not include `HartId`).
  Once R-026 lands (option (a) recommended for minimum churn),
  R-027 / R-028 / R-029 / R-030 are all non-blocking
  clarifications. Auto-flip to Yes after the fix.



---

## Open Questions for MASTER

The user will hand-author MASTER. Two design decisions where
reviewer judgement is non-decisive and user weigh-in is valuable:

- **`HartId` ownership location** (R-026): define at
  `cpu/core.rs` (trait-layer-owned newtype, recommended) vs
  re-export from `arch/riscv/cpu/mod.rs` with seam-test widening.
  Both are defensible. Option (a) keeps the seam vocabulary
  tight; option (b) keeps `HartId` co-located with the rest of
  the RISC-V hart-identity wiring (`mhartid` CSR seeding at I-6).
  User's call on which side of the trait/arch boundary `HartId`
  belongs.
- **`CPU::from_config` generic shape** (R-027 / TR-9): drop the
  `<B>` method generic + add `pub type CoreBuilder` alias
  (recommended) vs keep the trait-bound generic for explicit
  call-site documentation. Option (a) is ergonomically better
  but adds a second seam alias; option (b) is more "trait-pure"
  but uglier at every call site. User may have a preference on
  whether seam aliases or trait generics carry the arch
  selector.
