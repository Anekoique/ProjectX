# `directIrq` REVIEW `00`

> Status: Open
> Feature: `directIrq`
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
- Blocking Issues: `4`
- Non-Blocking Issues: `6`



## Summary

Round 00 opens the `directIrq` iteration to close MANUAL_REVIEW #5 + #6 by
introducing an arch-neutral `IrqLine` handle, a PLIC-internal
`PlicSignals` atomic signal plane, and phased retirement of the
`Device::irq_line` / `Device::notify` / `Plic::notify` / bus-bitmap-pump
substrate. The handle-and-plane decomposition is the right shape, the
plicGateway-inherited invariants (Gateway / Core / Source split) are
preserved verbatim, the three-phase rollout is coherent (introduce →
migrate → retire), and the I-9 → I-D9 re-examination is load-bearing and
correctly framed — the UART reader thread at `uart.rs:94-129` really
does need any-thread raise and restructuring UART to keep PLIC
tick-only would be a regression relative to G-2's async-latency goal.
Trade-off framing (T-1..T-5) is substantive and well-scoped; the
Option-A recommendations are defensible. Invariant numbering (I-D1..I-D11)
maps 1:1 onto the validation matrix with only two minor gaps.

Four blocking findings must be resolved in Round 01. **R-001 (CRITICAL)**:
`Plic::tick(&mut self)` is named as a *new inherent method* on `Plic` but
`Bus::tick` reaches every device through `Box<dyn Device>`
(`bus.rs:217-243`). The trait already has `fn tick(&mut self)` at
`device/mod.rs:28` — the plan does not say whether the new drain
overrides `Device::tick` (which changes semantics for every existing PLIC
tick call-site, including reset evaluation) or stays inherent (in which
case `Bus::tick` cannot call it without a downcast that violates the
dyn-Device contract). Phase 1 step 7 says "call `plic.tick()` after
`plic.notify(bitmap)`" without disambiguating which path is live.
**R-002 (HIGH)**: `NUM_SRC = 32` is a `core.rs` internal constant
(`plic/core.rs:14`); `PlicSignals { level: AtomicU32, edge_latch: AtomicU32 }`
is a *silent* hard dependency on `NUM_SRC ≤ 32` that the plan neither
declares as an invariant nor pins with a `const_assert!` — T-2 Option A's
"`NUM_SRC ≤ 32`" aside (line 782) is the only mention and it is not
rendered as a constraint the next PLAN and reviewer can enforce.
**R-003 (HIGH)**: I-D10 claims "`Plic::tick` is only called from the bus
tick thread" but Phase 1 runs *both* `plic.notify(bitmap)` and
`plic.tick()` in the same bus tick (`00_PLAN.md:688-694`). The union is
justified as "tolerant of duplicate `sample(level)` calls", but the
bitmap path and the signal-plane path can disagree on a source's level
during Phase 1 (UART is signalling via `IrqLine` while the bitmap still
samples `Device::irq_line`): the same source gets sampled twice per tick,
once with a stale level and once with the fresh one, and the Gateway
Level FSM sees `sample(true) → sample(false)` or vice versa within one
bus tick. Whether this is observationally equivalent to the pre-feature
path depends on whether the second sample's decision can undo the first
— the plan asserts monotonicity without evidence. **R-004 (HIGH)**: I-D8
says "`Arc<PlicSignals>` pointer identity is preserved across reset so
devices' existing `IrqLine` handles keep working", but the current
`Plic::reset` at `plic/mod.rs:140-148` only zeroes `core` + `gateways` +
`evaluate` — the plan must explicitly state that reset goes through
`self.signals.reset()` (not through reconstructing `PlicSignals`) and
that neither `Plic::new` nor `Plic::with_config` rebuild the Arc on
reset. Without this, `Device::hard_reset` paths in `Bus::reset_devices`
would invalidate outstanding `IrqLine` handles.

Non-blocking items cover: the `pulse()` semantics (T-4 / OQ-2) are
argued but not tied to a concrete producer so V-UT-8 / V-E-3 land as
speculative tests without a real consumer (R-005); Phase 3's
`MmioRegion::irq_source` removal changes `Bus::add_mmio`'s public
signature — machine-construction ripple is acknowledged as "mechanical"
but not listed site-by-site (R-006); `Gateway::sample_with(level, edge)`
widens the Gateway API without a deprecation path for the existing
`sample(level)` callers inside `plic/mod.rs` — V-UT-7 asserts
byte-equivalence but the migration order within Phase 1 is not specified
(R-007); the seam-allowlist claim (OQ-4 / C-2) is correct but glosses
over `IrqSignalPlane` — the trait lives at `src/device/irq.rs`
(arch-neutral), so it is correctly *not* a seam symbol, but the plan
should say so to close the loop (R-008); Phase 1's "both paths coexist"
creates a window where a device is *simultaneously* adopting `IrqLine`
and still returning `Device::irq_line() -> true` via the default — UART
explicitly keeps its override per step 5 (`00_PLAN.md:684-685`), but
I-D11's "no device in both states simultaneously" contradicts this
(R-009); the `drain` snapshot at `signals.rs:403-407` takes
`level.load(Acquire)` followed by `edge_latch.swap(0, AcqRel)` — these
are not atomic as a pair, and a concurrent `pulse()` between the two
reads can be split (edge observed this tick, level deferred, or vice
versa); V-E-4 names this race but does not prove the Gateway FSM is
tolerant of split observations under the pulse path (R-010).

None of R-005..R-010 block implementation. TR-1/TR-2/TR-3/TR-5 concur
with Option A. TR-4 wants tighter pulse producer story.
Approve with revisions; Ready for Implementation = No (R-001 is
CRITICAL).



---

## Findings

### R-001 `Plic::tick` dispatch path is unspecified — collision with `Device::tick`

- Severity: CRITICAL
- Section: API Surface / Execution Flow
- Type: API
- Problem:
  The plan declares `Plic::tick(&mut self)` as a new method on `Plic`
  (`00_PLAN.md:446-460`, `515-516`) and says `Bus::tick` calls it
  (`00_PLAN.md:591-594`, Phase 1 step 7 at `688-694`). However,
  `Bus::tick` reaches PLIC through `self.mmio[i].dev` typed as
  `Box<dyn Device>` (see `bus.rs:217-243` — current `plic.notify(bitmap)`
  goes through `Device::notify`). The `Device` trait already has
  `fn tick(&mut self)` with a no-op default (`device/mod.rs:28`). Three
  mutually incompatible interpretations are open:
    1. `Plic::tick` *overrides* `Device::tick` — then the bitmap-fold
       phase-1 coexistence is wrong (every slow-tick loop would now
       call signal-plane drain, not the old `notify` path), and
       existing per-device `tick()` for PLIC changes meaning.
    2. `Plic::tick` is a *new inherent* method — then `Bus::tick`
       cannot reach it without a downcast from `dyn Device`, which
       violates the trait-object contract and has no precedent in
       this codebase.
    3. A new `Device` trait method is added (e.g. `fn drain_signals`)
       — then the plan must name it, and the Phase-3 retirement is
       wider than it appears.
  The plan does not pick.
- Why it matters:
  This is the feature's central bus-side hook. Without disambiguation,
  the implementer reconstructs intent from the three options — exactly
  what the self-contained-PLAN rule forbids. It also shifts the Phase-1
  coexistence argument (R-003): interpretation (1) forces Phase 1 to
  choose *one* path per tick, interpretation (2) cannot be
  implemented, interpretation (3) broadens Phase 3's trait retirement.
- Recommendation:
  Round 01 must commit to one of the three dispatch models:
    - If Device-trait override: rename to `Device::tick` and state
      that `Plic::notify` remains the bitmap entry during Phase 1,
      ticked *before* the new `Device::tick`-as-drain sequence so the
      bitmap path is inert for sources whose devices hold an
      `IrqLine`. Spell out Phase-1 coexistence as "bitmap-first,
      drain-second" with explicit proof the drain cannot undo the
      bitmap.
    - If new trait method: add the signature to the `API Surface`
      section, classify it as a Phase-1 trait-surface addition, and
      state Phase-3 retirement (along with `Device::notify`).
    - If inherent + downcast: justify why the codebase should adopt
      downcasting for this one seam.
  In all three cases, pin the call site in `bus.rs:217-244` as the
  *only* `Plic::tick` caller in Phase 1, and state the relative
  ordering with the bitmap-fold step.



### R-002 `PlicSignals` atomic width silently hard-codes `NUM_SRC ≤ 32`

- Severity: HIGH
- Section: Data Structure / Invariants / Constraints
- Type: Correctness
- Problem:
  `PlicSignals { level: AtomicU32, edge_latch: AtomicU32 }` at
  `00_PLAN.md:393-396` uses `AtomicU32`. `NUM_SRC` is `32` today
  (`plic/core.rs:14`), so `1u32 << s` for `s in 1..NUM_SRC` covers
  bits 1..32. Bit 31 is the last usable bit; `1u32 << 32` on `u32` is
  undefined behaviour in Rust (shift-overflow panics in debug,
  wraps/poisons in release depending on target). T-2 Option A's line
  `NUM_SRC ≤ 32` is the only mention and is not an invariant, not a
  constraint, and not enforced. `Plic::tick` loops `for s in 1..NUM_SRC`
  so a future bump to `NUM_SRC = 48` (not inconceivable — SiFive U74
  has 127 sources) would silently UB rather than fail to compile.
- Why it matters:
  Silent dependency on a *different* module's constant is exactly the
  hidden-assumption class of defect the AGENTS.md spec-alignment
  review is meant to catch. The implementation would land green, the
  first reader raising `NUM_SRC` would not learn of the coupling, and
  the failure mode (shift UB in `raise`/`lower`/`pulse` on a random
  source id) is non-local.
- Recommendation:
  Round 01 must:
    1. Add invariant **I-D12**: "`PlicSignals` atomic width ≥ `NUM_SRC`;
       `u32` backs sources 0..32 and the type changes in lockstep with
       `NUM_SRC`."
    2. Add a `const _: () = assert!(NUM_SRC <= 32, ...);` (or
       `static_assertions::const_assert!`) co-located with
       `PlicSignals`.
    3. State constraint **C-12**: widening `NUM_SRC` past 32 requires
       widening `PlicSignals` to `AtomicU64` or a `[AtomicU32; N]`
       array — note this explicitly so the coupling survives future
       edits.
    4. `Plic::with_irq_line(src)`'s existing `assert!((1..NUM_SRC as u32).contains(&src))`
       at `00_PLAN.md:442` already guards handle creation, but
       `IrqSignalPlane::{raise,lower,pulse}` at `00_PLAN.md:416-427`
       must also document that `src < 32` so a malformed call does
       not UB.



### R-003 Phase-1 double-path coexistence argument is hand-waved, not proved

- Severity: HIGH
- Section: Implementation Plan / Invariants
- Type: Correctness
- Problem:
  Phase 1 step 7 (`00_PLAN.md:688-694`) keeps the bus bitmap fold +
  `plic.notify(bitmap)` **and** calls the new `plic.tick()` (signal
  plane drain) within the same bus tick. The plan claims "the union
  of decisions is still monotonic under the Gateway FSM, which is
  already tolerant of duplicate `sample(level)` calls within a tick."
  This is true for *identical* duplicate samples (coalesce-on-same-level),
  but the Phase-1 window has devices in two states:
    - Non-adopter devices (VirtioBlk): `Device::irq_line()` reports
      fresh truth, `IrqLine` path is inactive, signal-plane bit is 0.
    - Adopter devices (UART): both paths are live — UART's
      `irq_line()` still returns RX/THRE truth (step 5 retains the
      override at `00_PLAN.md:684-685`), *and* the signal plane carries
      `raise`/`lower` from the same computation.
  For UART specifically, a rising edge that lands in the signal plane
  from the reader thread between the bitmap collection
  (`bus.rs:231-240`) and `plic.notify(bitmap)` produces:
    - bitmap-fold path: `sample(level=false)` (stale — before UART's
      `tick()` drained rx_buf into rx_fifo).
    - signal-plane path: `sample_with(level=true, ...)`.
  Within one bus tick, the Gateway sees `sample(false)` then
  `sample(true)` on the same source. The Level FSM emits `Clear` then
  `Pend`, so `Core::pending` ends at `set`, which is the *correct*
  final state — but I-D6 / plicGateway I-3 are about *same-level*
  coalescing, not about tolerating `true → false → true` within a
  single evaluate frame.
- Why it matters:
  The Phase-1 gate asserts 381 tests green + xv6/linux-2hart/debian-2hart
  boot. Any golden trace asserting "PLIC pending bit asserts on tick N
  not N-1" would regress. The plan should either prove order-independence
  or fix the coexistence by gating which path is live per source.
- Recommendation:
  Round 01 should:
    1. State a **Phase-1 invariant I-D13**: "during Phase 1, for each
       source, only one of `{bitmap-fold, signal-plane}` is the
       source of truth; the other is inert." The simplest way to
       enforce this is to drop the device's `irq_line()` override at
       the moment the device starts using `IrqLine` — i.e. contradict
       step 5's "retain `fn irq_line`" and instead set it to `false`
       on the adopter device from Phase 1 onward. UART then signals
       *only* through `IrqLine`; the bus bitmap collects `0` for
       source 10 and the fold does not spuriously Clear it.
    2. Or, if both paths must coexist for device-by-device migration,
       spell out the exact Gateway FSM claim: for a Level source,
       `sample(a); sample(b)` within one `notify`+`tick` frame is
       equivalent to `sample(b)` alone (no matter `a`). Pin this as
       V-UT-12 or similar before unblocking Phase 1.
    3. Align with I-D11: the current I-D11 (`00_PLAN.md:352-354`) says
       "no device is in both states simultaneously", which already
       prohibits the coexistence this step constructs. Either I-D11
       or Phase-1 step 5 must yield.



### R-004 I-D8 "reset preserves Arc pointer identity" — mechanism undocumented

- Severity: HIGH
- Section: Invariants / Implementation Plan
- Type: Invariant
- Problem:
  I-D8 (`00_PLAN.md:340-343`) asserts that `Plic::reset` and
  `Plic::hard_reset` zero `PlicSignals` *in place* so devices' live
  `IrqLine` handles remain valid. The current `Plic::reset` at
  `plic/mod.rs:140-148` mutates runtime fields (`core.reset_runtime()`,
  `gateway.reset_runtime()`, `core.evaluate()`). The plan does not
  state that `signals.reset()` is called in the same branch; Phase 1
  step 3 says "`Device::reset` / `hard_reset` also call
  `signals.reset()` (I-D8)" which is the mechanism — but this is buried
  under §Implementation Plan and not pinned in the Data Structure
  section where the `Arc<PlicSignals>` field is introduced
  (`00_PLAN.md:434-438`).

  More subtly: `Plic::new` and `Plic::with_config` must not be called
  during reset (they would rebuild `signals: Arc<PlicSignals>` and
  invalidate the old Arc). The current codebase doesn't call
  `Plic::new` during reset (reset paths are all methods, not
  constructors), so today this is safe — but the plan should state it
  as an invariant rather than leaving the absence-of-call-site as
  implicit proof. `Bus::reset_devices` at `bus.rs:247-252` calls
  `dev.hard_reset()` through the trait — Plic's `hard_reset` default
  delegates to `reset` (see `device/mod.rs:39-41`), so as long as
  `reset` is in-place, `hard_reset` is too. State it.
- Why it matters:
  A future reset-by-reconstruction refactor would silently break every
  outstanding `IrqLine` (their `Arc<dyn IrqSignalPlane>` would point to
  the dropped plane; raises would hit the old atomics; drains would
  miss them). The failure is silent — guests would see interrupts
  appear to work initially then mysteriously stop after any reset.
- Recommendation:
  Round 01 must:
    1. Add invariant **I-D8a**: "`Plic::{reset, hard_reset}` mutate
       `*signals` in place via `signals.reset()`. They must not
       replace `self.signals` with a new `Arc`. Reconstruction-by-
       replacement is forbidden for the lifetime of the `Plic`
       instance."
    2. Pin a unit test (**V-UT-10**): construct `Plic`, call
       `with_irq_line(2)` → hold the handle, call `Plic::reset` via
       `Device::reset`, call `line.raise()`, call `Plic::tick`,
       assert the raise is observed. Variant with `hard_reset`.
    3. Document this at the `Plic` Data Structure paragraph (near
       `00_PLAN.md:434-438`), not only in Phase 1 step 3.



### R-005 `pulse()` semantics lack an in-tree consumer

- Severity: MEDIUM
- Section: Trade-offs / Validation / Non-Goals
- Type: Validation
- Problem:
  T-4 Option A pins `pulse()` semantics ("set both `level` and
  `edge_latch`; next tick drains edge, leaves level sticky"). NG-3
  confirms no in-tree device is promoted to edge in this feature. V-UT-8
  and V-E-3 exercise the pulse path at the `PlicSignals` boundary and
  at the `Gateway` boundary, but there is no end-to-end witness —
  `Plic::tick` driving an edge source through `sample_with(_, true)` up
  to a MEIP assertion on a context with configured threshold+enable.
  The plicGateway feature had the same gap (R-014 / V-E-7) and fixed
  it by adding a Plic-boundary edge test in 02_PLAN.
- Why it matters:
  The directIrq feature ships the *input path* for edge (`edge_latch`
  latching from `IrqLine::pulse`) but no concrete caller exercises it.
  A future edge adopter may hit semantic surprises (T-4 Option A vs B
  vs C) that V-UT-8/V-E-3 cannot catch because they don't drive through
  `Plic`. Reuse the plicGateway V-E-7 precedent.
- Recommendation:
  Add **V-E-6** (or V-IT-6): at the Plic boundary, construct a PLIC
  with one edge source, obtain its `IrqLine`, call `line.pulse()` from
  a different thread, `Plic::tick`, assert MEIP asserts on ctx 0 with
  threshold=0 and enable=source. Witness is not redundant with the
  plicGateway V-E-7 because that one drives through `notify(bitmap)`;
  this one drives through the signal plane.



### R-006 Phase-3 `Bus::add_mmio` signature change — ripple sites unlisted

- Severity: MEDIUM
- Section: Implementation Plan / API Surface
- Type: Maintainability
- Problem:
  `00_PLAN.md:524-525` lists "removed: `irq_source: u32` parameter"
  from `Bus::add_mmio`. This is a public signature change. The Risks
  section (Risk 5) acknowledges "the ripple is mechanical and a
  `cargo check` will catch every site" — that is correct but not
  sufficient for review: the plan should enumerate the call sites so
  the Phase-3 diff is reviewable without grepping the repo. Current
  call sites are in machine-construction modules under `machine/` and
  test harnesses inside `xemu/xcore/tests/`.
- Why it matters:
  A reviewer cannot approve "mechanical change" in the abstract. Naming
  the sites also surfaces test-harness call sites that may carry
  string-literal `"plic"` / `"aclint"` pins in `BUS_DEBUG_STRING_PINS`
  (C-3) — relocating those pins silently would flip the allowlist
  count.
- Recommendation:
  Add a list to Phase 3 step 4 naming every `Bus::add_mmio(...)` call
  site in the repo (the grep is `rg 'add_mmio\('`). Cross-reference
  `BUS_DEBUG_STRING_PINS` (`arch_isolation.rs:74-77`) to verify Phase
  3 does not perturb the `("plic", 1)` pin.



### R-007 `Gateway::sample` / `sample_with` migration order unspecified

- Severity: MEDIUM
- Section: Implementation Plan / Data Structure
- Type: Correctness
- Problem:
  Phase 1 step 4 (`00_PLAN.md:672-674`) says "Extend
  `Gateway::sample_with(level: bool, edge: bool)`; retain the
  existing `sample(level)` as an inline call to `sample_with(level,
  false)` for in-place test compatibility." Meanwhile `Plic::notify`
  at `plic/mod.rs:128-138` calls `self.gateways[s].sample(level)`. Two
  options are compatible:
    1. `sample(level)` stays as an inline wrapper (thin shim); both
       `notify` and the new `tick` paths coexist for Phase 1 and Phase
       2 until `notify` is retired.
    2. `sample(level)` is deleted and `Plic::notify` is rewritten to
       `sample_with(level, false)` in Phase 1.
  Option 1 is consistent with "retain" wording; Option 2 is consistent
  with "extend". Phase 2 deletes the bitmap fold but not necessarily
  `Plic::notify` — which stays until Phase 3 per `00_PLAN.md:110-112`.
- Why it matters:
  The V-UT-7 assertion ("`Gateway::sample_with(level, false)` is
  byte-equivalent to `sample(level)`") is only meaningful if `sample`
  still exists as a distinct call path. If step 4 deletes it in
  Phase 1, V-UT-7 degenerates to "test the same method twice".
- Recommendation:
  Phase 1 step 4 must pick one: keep `sample(level)` as an inline
  shim through Phase 2, or rewrite `Plic::notify` to `sample_with(level,
  false)` immediately. V-UT-7 is only meaningful under the "keep shim"
  branch; rewrite it as a `sample_with(level, false)`-only regression
  test otherwise.



### R-008 OQ-4 / C-2 resolution understates the arch-neutral trait question

- Severity: LOW
- Section: Constraints / Unresolved
- Type: Spec Alignment
- Problem:
  OQ-4 (`00_PLAN.md:137-140`) and C-2 (`00_PLAN.md:532-539`) state
  `IrqLine` and `IrqSignalPlane` are arch-neutral so neither needs a
  seam-allowlist entry. Correct — but the plan does not state *why*
  `IrqSignalPlane` does not leak: the trait is declared in
  `src/device/irq.rs` (arch-neutral), so `src/device/uart.rs` can
  import `crate::device::irq::IrqSignalPlane` without touching
  `crate::arch::riscv::*`. The implementation (`PlicSignals`) lives at
  `src/arch/riscv/device/intc/plic/signals.rs` which already sits
  inside the arch tree.
- Why it matters:
  The arch_isolation test at `xemu/xcore/tests/arch_isolation.rs:249-280`
  flags any `pub use crate::arch::*` re-export whose symbol name is
  not in `SEAM_ALLOWED_SYMBOLS`. `IrqSignalPlane` is *not* a seam
  re-export (it is defined arch-neutral), so the test ignores it by
  construction. State this in C-2 so the reviewer does not need to
  re-derive it.
- Recommendation:
  Append to C-2: "`IrqSignalPlane` is declared in `src/device/irq.rs`
  (arch-neutral) and implemented by `PlicSignals` inside
  `src/arch/riscv/device/intc/plic/signals.rs`. The seam test at
  `tests/arch_isolation.rs:249-280` checks only
  `pub use crate::arch::*` names; arch-neutral traits are outside its
  scope."



### R-009 I-D11 contradicts Phase-1 step 5's "retain `fn irq_line`"

- Severity: LOW
- Section: Invariants / Implementation Plan
- Type: Correctness
- Problem:
  I-D11 (`00_PLAN.md:352-354`) reads "A device either holds an
  `IrqLine` and no longer overrides `irq_line()`, or vice versa. No
  device is in both states simultaneously at a phase boundary." Phase 1
  step 5 (`00_PLAN.md:684-685`) explicitly has UART *hold an `IrqLine`*
  **and** *retain `fn irq_line(&self) -> bool`* so the bus bitmap path
  still works for non-adopters. This is exactly the "both states" I-D11
  forbids.
- Why it matters:
  Self-contradiction in the invariant / phase-plan pair forces the
  implementer to pick one. Fixing R-003 (committing to a single
  signaling path per device per tick) likely resolves this as a
  side-effect, but I-D11's wording should be tightened regardless.
- Recommendation:
  Reword I-D11 to "at each phase *boundary*, a device registers as
  either a legacy (`Device::irq_line`-returning) or adopter (`IrqLine`-
  holding) signaller; inside Phase 1, adopter devices may keep their
  `fn irq_line` override returning `false` so the bus bitmap collects
  zero for them." Or: drop the override on Phase-1 adopter devices as
  proposed in R-003.



### R-010 `PlicSignals::drain` is not atomic as a pair — pulse split race

- Severity: LOW
- Section: Data Structure / Edge Cases
- Type: Correctness
- Problem:
  `drain` (`00_PLAN.md:403-407`) does
  ```
  let lvl = self.level.load(Acquire);
  let edg = self.edge_latch.swap(0, AcqRel);
  ```
  These two atomics are not a pair. A concurrent `pulse()` landing
  between the two reads can produce:
    - sequence A: `pulse` → `drain.load(level)` → `drain.swap(edge)`
      — both observed this tick. Correct.
    - sequence B: `drain.load(level)` → `pulse` → `drain.swap(edge)` —
      edge observed this tick, level deferred until next. The Gateway
      sees `sample_with(level=false, edge=true)`; `sample_edge_signal`
      at `00_PLAN.md:482-492` treats `edge_pulse=true` as a forced
      rising edge regardless of `level` — so Pend fires, which is
      acceptable.
    - sequence C: `drain.load(level)` → `drain.swap(edge)` → `pulse` —
      both deferred until next tick. Correct.
  Sequence B is the interesting one: `sample_level` for a Level source
  with `level=false, edge=true` is "ignored" per V-E-3, but the
  `edge_latch` bit has been consumed — the Level-source pulse is
  effectively a raise-then-implicit-lower, which is *not* the T-4
  Option A semantics ("sticky level"). V-E-4 names the race but does
  not specify outcomes.
- Why it matters:
  Low-probability race, but it's the kind of subtle ordering bug that
  only shows up under thread scheduling pressure. The plan should
  either prove the race is benign or swap the order in `drain` to
  `edge_latch` first, `level` second (then sequence-B maps to "edge
  consumed, level observed correctly later").
- Recommendation:
  Swap the `drain` order: load `edge_latch` first via
  `edge_latch.swap(0, AcqRel)`, *then* `level.load(Acquire)`. This
  gives: any `pulse()` that completed before `edge_latch.swap` also
  completed its `level.fetch_or` (pulse writes level first, then
  edge); the later `level.load` observes it. Update V-E-4 to enumerate
  the three sequences and the expected `sample_with` outcomes, so the
  implementer can pin this by test rather than by argument.



---

## Trade-off Advice

### TR-1 Handle type shape (Option A vs B vs C)

- Related Plan Item: `T-1`
- Topic: Flexibility vs Safety — arch isolation vs vtable cost
- Reviewer Position: Prefer Option A
- Advice:
  Option A (`Arc<dyn IrqSignalPlane>`) is the right call.
- Rationale:
  Option B (`Arc<PlicSignals>` direct) breaks C-2 at byte level —
  `src/device/irq.rs` would have to `use crate::arch::riscv::...::PlicSignals`,
  which the arch-isolation test rejects. Option C (closure) loses the
  `src` field's debuggability and doubles the allocation per line.
  Vtable cost is trivially hoistable out of the `IrqLine::raise` hot
  path by the optimizer (it's a single indirect call with a stable
  vtable pointer). The seam-preserving property of Option A is worth
  far more than one indirect call.
- Required Action:
  Adopt as is.



### TR-2 Signal plane representation (Option A vs B vs C)

- Related Plan Item: `T-2`
- Topic: Performance vs Simplicity
- Reviewer Position: Prefer Option A (with R-002 enforcement)
- Advice:
  Two `AtomicU32` is the right shape. It matches the existing
  `IrqState(Arc<AtomicU64>)` pattern at `device/mod.rs:55-79` and keeps
  the cache footprint at 8 bytes.
- Rationale:
  Option B (`AtomicBool`-per-source) would introduce false sharing
  risk and is 32× heavier on drain (32 loads). Option C (`AtomicU64`
  interleaved) buys no atomicity between level and edge — the
  operations are on independent bits.
- Required Action:
  Adopt as is, with R-002's `const_assert!` and I-D12 to pin the
  `NUM_SRC ≤ 32` coupling.



### TR-3 `PlicSignals` ownership (Option A vs B)

- Related Plan Item: `T-3`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A
- Advice:
  `Plic` owns `Arc<PlicSignals>` and hands out handles via
  `with_irq_line`. Matches archLayout: arch-specific concern lives
  under `src/arch/riscv/device/intc/plic/`.
- Rationale:
  Option B (Bus-owned) forces every `raise` to traverse `Bus`'s mutex,
  defeating the whole cross-thread-raise promise of G-2.
- Required Action:
  Adopt as is.



### TR-4 `pulse()` semantics (Option A vs B vs C)

- Related Plan Item: `T-4`
- Topic: Flexibility vs Safety
- Reviewer Position: Need More Justification
- Advice:
  Option A is defensible but not proven. The "sticky level until
  `lower`" model assumes the producer will later call `lower` — for a
  pure-edge device (never in-tree this feature, per NG-3), the level
  stays high forever. That's not wrong for an edge source (the
  Gateway's `sample_edge` ignores level except for the `prev_level`
  bookkeeping) but it is surprising and should be named.
- Rationale:
  An edge-only device calling `pulse()` repeatedly will leave
  `PlicSignals.level & (1<<src) = 1` permanently. The Level-FSM
  ignores this bit (the source is Edge-configured), but a future
  debug inspection of `PlicSignals.level` would show a "stuck raise"
  that is in fact a legitimate edge-only history. Document this.
  Alternative: Option C clears `level` after consuming an edge pulse
  for Edge sources — zero cost, clearer debug state. Worth
  considering if edge adopters are on the roadmap.
- Required Action:
  Round 01 should either (a) justify Option A by naming the future
  consumer and showing that "sticky level" matches the hardware
  behaviour of that consumer, or (b) revisit Option C. Add V-E-6 per
  R-005 to witness the pulse end-to-end before pinning the choice.



### TR-5 Evaluation cadence (Option A vs B vs C)

- Related Plan Item: `T-5`
- Topic: Performance vs Simplicity
- Reviewer Position: Prefer Option A
- Advice:
  Run `Plic::tick` every `Bus::tick` slow-path (every
  `SLOW_TICK_DIVISOR = 64` bus ticks), matching the current
  `plic.notify(bitmap)` cadence.
- Rationale:
  The latency reduction promised by G-2 comes from the raise landing
  in `PlicSignals` immediately (no `Uart::tick` sample delay); PLIC
  evaluation cadence staying at 64 bus ticks is fine because the
  alternative (per-bus-tick evaluation) costs 2 atomic loads ×
  every-bus-tick. Option C (epoch wakeup) needs a third atomic, saves
  nothing at 64-tick cadence, and adds complexity.
- Required Action:
  Adopt as is.



---

## Positive Notes

- Trade-off framing (T-1..T-5) is substantive and option-complete. Each
  trade-off lists three concrete options, a recommendation, and a
  rationale that names the rejected paths rather than hand-waving.
- The I-9 → I-D9 re-examination is correctly framed as a formal
  supersession in the Response Matrix (`00_PLAN.md:149`) rather than
  silently changing the invariant.
- The Phase 1 / 2 / 3 split is clean: introduce coexistence → migrate
  last device + retire bitmap pump → retire trait surface. Each phase
  has a validation gate pinned to `cargo test -p xcore` count + boot
  trio.
- `IrqLine`'s `Clone` + coalesce-by-design (I-D7) matches the prior
  art (QEMU `qemu_irq`, `IrqState(Arc<AtomicU64>)`) and is a clear
  specification rather than an accidental property.
- Arch-neutral trait (`IrqSignalPlane`) in `src/device/` + arch-specific
  impl (`PlicSignals`) in `src/arch/riscv/device/intc/plic/` is the
  right seam; OQ-4 correctly identifies that no new
  `SEAM_ALLOWED_SYMBOLS` entry is needed.
- Non-goals are tight: NG-1..NG-7 name the temptations (lock-free
  core, per-hart threading, edge adopter, MSI, LoongArch, Gateway FSM
  changes, SLOW_TICK_DIVISOR removal) and foreclose them.



---

## Approval Conditions

### Must Fix
- R-001 (CRITICAL) — `Plic::tick` dispatch path must be committed to
  one of: `Device::tick` override, new trait method, or justified
  inherent+downcast. Blocks Phase 1 step 7 scope.
- R-002 (HIGH) — `NUM_SRC ≤ 32` coupling must be named as I-D12 /
  C-12 and enforced by `const_assert!`.
- R-003 (HIGH) — Phase 1 coexistence must prove or enforce
  single-path-per-source. Either reword I-D11 + drop adopters'
  `irq_line` override (preferred), or prove Level FSM tolerates
  mid-frame `sample(false) → sample(true)` (V-UT-12).
- R-004 (HIGH) — I-D8 must be promoted to I-D8a with explicit
  "in-place reset" contract, co-located with the `Plic.signals` Data
  Structure paragraph, and pinned by V-UT-10.

### Should Improve
- R-005 — Add V-E-6: Plic-boundary edge+pulse end-to-end test.
- R-006 — List `Bus::add_mmio` call sites for Phase 3 ripple.
- R-007 — Pick one `Gateway::sample` migration strategy (shim-retain
  vs rewrite in Phase 1).
- R-008 — Tighten C-2 wording on `IrqSignalPlane` non-seam status.
- R-009 — Reword I-D11 to resolve the "both states" contradiction.
- R-010 — Swap `drain` order (edge first, then level) and document
  the three `pulse()` / `drain` interleavings in V-E-4.

### Trade-off Responses Required
- TR-4 (T-4 pulse semantics) — justify Option A with a named future
  consumer, or revisit Option C.

### Ready for Implementation
- No
- Reason: R-001 is CRITICAL — the `Plic::tick` dispatch question must
  be resolved before Phase 1 step 7 can be implemented. The three
  HIGHs (R-002, R-003, R-004) each name a concrete correctness or
  hidden-assumption gap that must close before the feature's core
  invariants (I-D8..I-D11) become implementable as written.
