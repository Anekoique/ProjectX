# `plicGateway` REVIEW `01`

> Status: Open
> Feature: `plicGateway`
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
- Blocking Issues: `0`
- Non-Blocking Issues: `4`



## Summary

Round 01 cleanly resolves all four round-00 blockers by **narrowing scope**
to MANUAL_REVIEW items #6 + #7 and deferring #5 (direct device→PLIC
signaling) to a future `directIrq` feature. The four round-00 findings
collapse neatly under the narrowed scope:

- R-001 (scope merge) is resolved via option (a): `PlicIrqLine`,
  `LineSignal`, `Bus::tick` surgery, and device-side migration all move
  to `directIrq`. The Response Matrix records the T4/T5 boundary
  decision explicitly. Verified against `project_manual_review_progress.md:14-16`.
- R-002 (SiFive-variant deviation) is pinned as **Invariant I-8**
  documenting pre-claim level-low clear as a deliberate departure from
  PLIC v1.0.0 §5, matching the aclintSplit precedent of naming SiFive
  variant choices explicitly. `V-UT-11` asserts it.
- R-003 (atomics over-engineering) is resolved via option (b): **I-9**
  commits to tick-thread-only signaling; atomics are dropped;
  `LineSignal` is removed. Verified against `uart.rs:94-130` — the
  reader thread only pushes into `rx_buf`, it never touches PLIC state,
  so tick-only posture is factually correct for this feature.
- R-004 (Device::tick collision) is resolved by **retaining** the
  existing `Bus::tick` bitmap-pump path and the existing
  `Plic::notify(u32)` override (**I-10**). No `Device::tick` repurposing.
  Tick ordering is a non-issue because every device's `tick()` runs in
  the slow-path fold at `bus.rs:229-240` *before* `plic.notify` at line
  242 within the same bus tick. Verified against `bus.rs:220-243`.

The revised two-phase scope is genuinely implementable: Phase 1 is a
pure internal refactor of the existing `plic.rs` (181 lines of core +
176 lines of 15 tests) into `plic/{mod,core,gateway,source}.rs` with
Gateway wired behind the unchanged `Plic::notify(bitmap)` entry point;
Phase 2 adds the Edge arm of the gateway FSM with no in-tree caller.
The Level FSM table (lines 580-588) is bit-faithful to today's
`Plic::update` at `plic.rs:56-68` modulo the explicit I-8 deviation
disclaimer. Data structures (lines 376-447) are minimal,
immutable-friendly (no `Cell`, no `Arc`, no unsafe), and match the
user's "many-small-files" rule with a ≤250-line soft cap per file
(C-11). Every MASTER directive from archModule (00-M-002) and
archLayout (01-M-004) is acknowledged with a Honored decision.

Non-blocking items: the Edge FSM table has one transition ambiguity on
`sample(false)` interaction with `armed` during in-flight (R-012); the
`Core::complete` → `gateway.on_complete()` callback wiring in Phase 1
step 4 wants a tighter description of how `Pend` re-entrance is
avoided (R-013); Phase 2 has no in-tree exerciser so V-E-6 is the only
witness that the edge path is reachable end-to-end at the `Plic`
boundary (R-014); and V-IT-1's "byte-identical allowlist" assertion
duplicates C-3 without specifying **how** (diff vs pinned constant) —
minor (R-015).

None of R-012..R-015 block implementation. Trade-off TR-1 concurs with
T-1 Option B. TR-2 concurs with T-2 Option B (tick-only).
TR-3/TR-4/TR-5 concur with the in-gateway edge representation, reset
preservation, and tick-boundary evaluation choices respectively.
Approve with revisions; Ready for Implementation = Yes.



---

## Findings

### R-012 `Edge FSM transition ambiguity on sample(false) during in_flight`

- Severity: MEDIUM
- Section: State Transition (Edge table, `01_PLAN.md:592-600`)
- Type: Correctness
- Problem:
  The Edge FSM table row 2 says:
  > `(armed=*, in_flight=*, prev_level=*)` → `sample(false)`:
  > `(prev_level=false)`, emit `NoChange`.
  This is a wildcard row. Combined with row 4:
  > `(armed=false, in_flight=true, prev_level=false)` → `sample(true)`:
  > `(armed=true, prev_level=true, in_flight=true)`, emit `NoChange` (latch).
  the intent is: a pulse (`prev=false → sample(true) → sample(false)`)
  during in-flight leaves `armed=true, prev_level=false`, so
  `on_complete` re-pends once. That matches the coalesce contract
  (I-3). But the wildcard row does **not** say whether `armed` is
  preserved on `sample(false)` — it only pins `prev_level`. A naive
  reader (or naive implementer) might reset the entire struct on a
  falling sample and drop the latched rising edge, breaking the
  `edge_during_claim_latches_and_pends_on_complete` contract
  (V-UT-7).
- Why it matters:
  V-UT-7 and V-UT-12 exercise this exact path, so a regression would
  be caught in unit test — but the plan's FSM table is the
  specification the executor codes against, and an ambiguous spec
  forces the executor to re-derive intent from the test names. Per
  AGENTS.md the PLAN should be specific enough to implement without
  reconstructing intent from multiple artifacts.
- Recommendation:
  Replace the wildcard Edge row 2 with an explicit row that spells
  out `armed` preservation:
  | From | Event | To | Emit |
  |---|---|---|---|
  | `(armed=a, in_flight=f, prev_level=*)` | `sample(false)` | `(armed=a, in_flight=f, prev_level=false)` | `NoChange` |
  Alternatively, add a one-line prose clarifier under the Edge table:
  "`sample(false)` never clears `armed` — only `prev_level` — so a
  rising edge latched during in-flight survives subsequent level
  drops." Either edit is ≤3 lines.



### R-013 `Core ↔ Gateway callback wiring under-specified`

- Severity: MEDIUM
- Section: Implementation Plan (Phase 1 step 4, `01_PLAN.md:626-642`) /
  Main Flow §5-6 (`01_PLAN.md:542-551`)
- Type: API / Flow
- Problem:
  Phase 1 step 4 describes the `Plic::notify` body but does not
  describe how `Core::claim(ctx)` reaches back into `Plic` to call
  `gateways[s].on_claim()`, nor how `Core::complete(ctx, src)`'s
  success path reaches `gateways[s].on_complete() -> GatewayDecision`
  and then re-enters `core.set_pending(s) + core.evaluate()` within
  the same MMIO `write` call. Three designs are compatible with the
  API surface at lines 429-440:
  (a) `Core` has no gateway knowledge; `Plic::read`/`Plic::write`
      call `core.claim()`/`core.complete()`, then call the gateway
      callback themselves and any follow-up `core.set_pending` +
      `core.evaluate`.
  (b) `Core::claim(&mut self, ctx, &mut [Gateway; NUM_SRC])` takes
      the gateway array by mutable ref.
  (c) `Core` owns the gateway array internally (Plic = just MMIO facade).
  Main Flow §5-6 implies (a) ("`Plic::read` delegates to
  `core.claim(ctx)`; … invokes `self.gateways[s].on_claim()`") and
  Data Structure shows `Plic { gateways, core }` with
  `Core::claim(&mut self, ctx) -> u32` taking only ctx, which forces
  (a). That is fine, but the plan never pins it as an explicit
  design decision.
- Why it matters:
  (a) keeps `Core` gateway-agnostic (clean single-responsibility) but
  puts claim/complete orchestration in `Plic::read`/`Plic::write`.
  The re-pend path on complete (`on_complete() -> Pend` →
  `core.set_pending(s)` → `core.evaluate()`) must run while the MMIO
  `write` stack frame is still live, because the existing test
  `source_repended_after_complete` at `plic.rs:269-278` expects the
  subsequent `notify(0x02)` to observe pending already correctly
  routed. Without an explicit pin, designs (b) or (c) are equally
  likely and would silently over-engineer the split.
- Recommendation:
  Add one paragraph under Execution Flow §5-6 or at the top of Phase
  1 step 4:
  > `Core` is gateway-agnostic. `Plic::read` orchestrates claim
  > (call `core.claim(ctx) -> src`; if `src != 0` then
  > `gateways[src].on_claim()`). `Plic::write` orchestrates complete
  > (call `core.complete(ctx, src) -> bool`; if true, match
  > `gateways[src].on_complete()` — on `Pend`, call
  > `core.set_pending(src)` then `core.evaluate()` before returning).
  > The re-pend-on-complete must complete within the MMIO `write`
  > call so the existing `source_repended_after_complete` test at
  > `plic.rs:269-278` is observationally unchanged.



### R-014 `Phase 2 edge path has no Plic-boundary integration exerciser`

- Severity: LOW
- Section: Phase 2 Validation (`01_PLAN.md:680-692`) / Risks
- Type: Validation
- Problem:
  Phase 2 adds `SourceKind::Edge` to `Gateway::sample` and
  `on_complete`, gated by `Plic::with_config`. No in-tree device
  adopts Edge (NG-1, NG-5). Validation for Phase 2 is entirely unit
  tests against the `Gateway` struct (V-UT-6, V-UT-7, V-UT-12,
  V-UT-G2) plus V-E-6 (construction-time mixed Level/Edge sanity).
  Risk 2 at `01_PLAN.md:900-903` acknowledges this. The plan's claim
  is that V-UT-G2 is table-driven over every Edge FSM transition,
  which is strong — but there is zero exercise of `Plic::with_config
  → notify(edge_bit) → read(claim) → write(complete)` end-to-end at
  the `Plic` (not just `Gateway`) boundary.
- Why it matters:
  A bug in the `Plic::with_config` wiring (e.g., the sources array
  gets the wrong default, or `Core::set_pending` is called with the
  wrong source index on the `on_complete -> Pend` path) would pass
  every `Gateway` unit test and the level baseline, but produce
  wrong IRQ routing when an edge source is actually wired in
  `directIrq`. The cost to add coverage at the `Plic` boundary is
  ~30 lines of test code.
- Recommendation:
  Add V-E-7 to Phase 2: construct `Plic::with_config` with source 5
  configured as Edge, enable it for ctx 0 at priority > threshold,
  call `notify(0x20)` then `notify(0x00)` then `notify(0x20)` and
  assert two distinct claim-complete cycles land correctly at the
  MMIO boundary. This closes the Plic-boundary gap without needing
  any new in-tree device.



### R-015 `V-IT-1 "byte-identical allowlist" assertion mechanism unspecified`

- Severity: LOW
- Section: Validation (V-IT-1, `01_PLAN.md:783-788`) / C-3
- Type: Validation
- Problem:
  V-IT-1 says:
  > `cargo test -p xcore --test arch_isolation` — `SEAM_ALLOWED_SYMBOLS`
  > unchanged. Diff-style assertion: the post-plicGateway allowlist is
  > byte-identical to the pre-plicGateway allowlist.
  But `cargo test arch_isolation` runs the isolation test; it does
  not by itself check that the allowlist array **hasn't grown**. The
  existing test at `xemu/xcore/tests/arch_isolation.rs` verifies that
  symbols leaking into top-level `src/device/` appear in
  `SEAM_ALLOWED_SYMBOLS`, but it passes trivially if the executor
  also adds a new allowed symbol alongside a new leaking symbol. The
  aclintSplit round-00 R-002 precedent flagged the same gap class.
- Why it matters:
  Under NG-1/NG-6 the executor has no reason to touch
  `SEAM_ALLOWED_SYMBOLS`, but a junior executor might add
  `"SourceKind"` or `"SourceConfig"` "just to be safe" and the test
  would pass. C-3 demands byte-identical; V-IT-1 should enforce it
  operationally.
- Recommendation:
  Tighten V-IT-1 to one of:
  (a) `git diff main -- xemu/xcore/tests/arch_isolation.rs` **empty**
      at Phase-1 completion and Phase-2 completion.
  (b) A compile-time pin: `const _: () = assert!(SEAM_ALLOWED_SYMBOLS.len() == N);`
      where `N` is captured pre-feature.
  Reviewer prefers (a) — it adds zero code and is easily checked in
  CI.



---

## Trade-off Advice

### TR-1 `Module split granularity`

- Related Plan Item: `T-1`
- Topic: Maintainability vs Implementation Cost
- Reviewer Position: Concur with Option B
- Advice:
  Keep the four-file split (`mod/core/gateway/source`) with the
  ≤250-line soft cap per file (C-11).
- Rationale:
  Current `plic.rs` is 181 lines for the monolith + 176 lines of
  tests = 357 total. Post-split: `source.rs` ~30 lines, `gateway.rs`
  ~100 lines (Level + Edge combined), `core.rs` ~130 lines,
  `mod.rs` ~80 lines + 176 lines of tests. Each file is well under
  the cap, each carries a single concern, and the user's global
  "many-small-files > few-large-files" rule is honored. Option A
  (monolith grown to ~500 lines) contradicts the feature's explicit
  responsibility-separation premise.
- Required Action:
  Adopt as planned.



### TR-2 `Concurrency primitives`

- Related Plan Item: `T-2`
- Topic: Performance vs Simplicity
- Reviewer Position: Concur with Option B (tick-only, no atomics)
- Advice:
  Drop atomics; rely on `&mut self` signaling through
  `Bus::tick → plic.notify(bitmap)`. Defer cross-thread ordering to
  `directIrq`.
- Rationale:
  Under the narrowed scope there is no cross-thread PLIC caller.
  UART's reader thread (`uart.rs:94-130`) only pushes into
  `rx_buf`; it never touches PLIC state. Adding atomics now would
  be speculative and would pre-commit the wrong ordering semantics
  for `directIrq` (which will need `Release/Acquire` on its pulse
  latch, and the `Gateway::sample` entry point will move off the
  purely `&mut self` path anyway). Keeping plain fields keeps
  Phase 1 a pure refactor with zero runtime semantics change.
- Required Action:
  Adopt as planned. `directIrq`'s PLAN must re-examine I-9 explicitly.



### TR-3 `Edge-latch representation under tick-only posture`

- Related Plan Item: `T-3`
- Topic: Simplicity vs Forward Compatibility
- Reviewer Position: Concur with Option A (in-gateway `prev_level`)
- Advice:
  Represent rising-edge detection via `prev_level: bool` in each
  gateway; derive rising edge as `!prev_level && level` inside
  `Gateway::sample`.
- Rationale:
  Under NG-1, the only edge signal source is consecutive bitmap
  level samples. `prev_level` is the exact minimal representation;
  no separate latch is needed. Option B (standalone `edge_latch`)
  would hint at a lossless-edge contract the feature explicitly
  disclaims (NG-1). When `directIrq` adds real `pulse()` semantics,
  it will introduce a separate latch field with proper atomic
  ordering — the `prev_level` approach coexists cleanly with that
  future addition rather than being replaced by it.
- Required Action:
  Adopt as planned. Reinforce Risk 2 with a one-line note that
  "lossless-edge-over-poll is not a goal" so a future reviewer does
  not flag it as a regression.



### TR-4 `SourceConfig lifecycle on reset`

- Related Plan Item: `T-4`
- Topic: Correctness (guest-visible vs platform-config)
- Reviewer Position: Concur with Option A (reset preserves `kind`)
- Advice:
  `Plic::reset` clears runtime state (`armed`, `in_flight`,
  `prev_level`, `pending`, `enable`, `threshold`, `claimed`,
  priorities, `irqs`) but preserves `Gateway::kind` /
  `SourceConfig`.
- Rationale:
  Reset is a guest-triggered runtime operation (OpenSBI system
  reset, VirtIO-blk soft reset). Source-kind is a platform
  construction-time choice — the equivalent of a devicetree-baked
  hardware property. Demoting edge sources back to level on a
  runtime reset would silently corrupt guest IRQ behavior and is
  the exact class of bug R-007 flagged. I-11 + V-F-5 pin it.
- Required Action:
  Adopt as planned. Verify V-F-5 exercises the post-reset
  `kind == Edge` observation directly (not via a subsequent
  `notify` side-effect).



### TR-5 `Evaluation site`

- Related Plan Item: `T-5`
- Topic: Determinism vs Latency
- Reviewer Position: Concur with Option A (evaluate inside `notify`)
- Advice:
  Keep evaluation at tick boundary via `Plic::notify` driven by the
  existing `Bus::tick` pump.
- Rationale:
  Under I-9 there is no `raise` caller thread, so R-006's "raise
  caller" axis collapses. Tick-boundary evaluation is
  determinism-preserving, guest-behavior-identical to today, and
  requires no new synchronization. The deferred latency axis (A2
  in R-006's language — any-thread raise + tick-boundary eval) is
  a directIrq-era decision; this plan correctly does not pre-commit
  it.
- Required Action:
  Adopt as planned.



---

## Positive Notes

- The narrowed scope is genuinely narrower. The plan deletes, not just
  demotes, the directIrq work: no `PlicIrqLine`, no `LineSignal`, no
  `line.rs`, no bus surgery, no device changes, no seam allowlist
  change. C-5 and C-6 pin this with `git diff main` assertions. This
  is the cleanest possible T4/T5 split.
- I-8 (SiFive-variant deviation) follows the aclintSplit precedent of
  naming platform choices as explicit invariants rather than tacitly
  preserving them. V-UT-11 asserts the deviation directly by name, so
  a future executor removing I-8 must consciously delete the test.
- The Response Matrix acknowledges inherited MASTER directives
  (archModule 00-M-002, archLayout 01-M-004) per AGENTS.md §3,
  resolving R-005.
- The Level FSM table at `01_PLAN.md:580-588` is bit-faithful to the
  current `Plic::update` at `plic.rs:56-68` modulo the I-8 disclaimer;
  every existing guest-observable behavior is preserved by construction.
- C-11 (file size cap ≤250 lines) + T-1 Option B honors the user's
  "many-small-files" coding-style rule without contortion; post-split
  file sizes naturally land well under the cap.
- The phase gate arithmetic is now explicit (Phase 1 ≥14 new,
  Phase 2 ≥5 new, total ≥19) resolving R-011.
- Non-goals are enumerated comprehensively (NG-1..NG-6), which makes
  review tractable and blocks accidental scope creep during execution.
- Trade-off T-2's rationale correctly identifies the future-cost risk
  of speculative atomics: a pre-committed `Ordering::Relaxed` would be
  the wrong choice for `directIrq`'s cross-thread pulse path, so
  deferring the decision is strictly better than locking in now.

---

## Approval Conditions

### Must Fix
- (none)

### Should Improve
- R-012 (Edge FSM wildcard disambiguation)
- R-013 (Core ↔ Gateway callback wiring pin)
- R-014 (V-E-7 Plic-boundary edge integration)
- R-015 (V-IT-1 diff-empty enforcement)

### Trade-off Responses Required
- (none — TR-1..TR-5 concurred with plan's positions)

### Ready for Implementation
- Yes
- Reason: All four round-00 blockers are resolved. The remaining
  R-012..R-015 are LOW/MEDIUM clarity items that do not change the
  plan's direction, invariants, or data structures; they can be folded
  into Phase 1 execution as a single editorial pass on the PLAN or
  handled as implementation-time decisions with a post-hoc PLAN
  update. No unresolved CRITICAL issues; no unresolved HIGH issues.
  Phase 1 is a pure refactor with the 375-test baseline holding;
  Phase 2 adds an opt-in edge path that no in-tree caller uses yet.
  Executor may proceed.
