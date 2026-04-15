# `directIrq` REVIEW `01`

> Status: Open
> Feature: `directIrq`
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
- Non-Blocking Issues: `4`



## Summary

Round 01 substantively addresses every blocking finding from 00_REVIEW
and the two `MUST` directives in 00_MASTER. R-001 commits unambiguously
to option 1 (Device-trait override): `Plic::tick` is the
`Device::tick` impl reached through the existing `r.dev.tick()` vtable
call at `bus.rs:233`, no inherent method, no downcast
(`01_PLAN.md:667-687`, `I-D10-revised` at `495-499`, §Dispatch at
`800-818`). R-002 lands as the `const _: () = assert!(NUM_SRC <= 32)`
at the top of `signals.rs` plus invariant I-D12 and constraint C-12
(`01_PLAN.md:506-508`, `571-574`, `830-833`). R-003 is resolved by
I-D13 plus the I-D11-revised + Phase-1 step 5 change that forces
adopter devices to return `false` from `Device::irq_line`
(`01_PLAN.md:500-505`, `511-513`, `939-940`); V-UT-12 is retained
as FSM-property documentation per the review's "either/or"
recommendation, which is defensible. R-004 is pinned as I-D8a
co-located with the `Plic.signals` Data Structure paragraph plus
the V-UT-10 Arc-identity reset test (`01_PLAN.md:474-479`, `641`,
`1084-1089`). Non-blocking findings R-005..R-010 each have a named
resolution with concrete artifacts (V-IT-6, V-E-6, R-006 call-site
table, shim retention, C-2 amendment, I-D11 rewording, drain-order
swap + V-E-4 three-case enumeration).

The M-001 response is the most consequential change: §Async Posture
(`01_PLAN.md:346-454`) enumerates four postures (P-A/B/C/D), rejects
P-B/C/D with concrete reasoning tied to NG-2 / NG-8 / C-11 / I-D7 /
G-2, and adopts a refined P-A with a new event-driven fast-path
(`pending_raises: AtomicBool`) encoded as I-D14 and I-D15. This is
not a superficial relabel: `Plic::tick` with no raises and
`!needs_reevaluate` performs exactly one `Acquire` swap and returns
(`01_PLAN.md:669-673`), validated mechanically by V-IT-7 and V-UT-11.
The five cited sources (Rust async-book, phil-opp, without.boats,
QEMU hw/irq.h, airbus-seclab) are real and directly support the
thesis that a `Waker` is the user-space wrapper around the atomic
primitive the plan already uses. The "research Rust async"
obligation from M-001 is satisfied, and the rejection of P-D on
NG-2 + no-executor grounds is correctly argued.

The M-002 response is cautious and concrete: C-13 bans new
async-runtime / channel crates, every atomic op has an explicit
ordering with a justification row in §Concurrency Posture
(`01_PLAN.md:743-757`), `loom` is named as a follow-up (OQ-5) not
smuggled into the active plan, and no `async fn` / `Future` / `Waker`
instantiation appears in any new code (NG-9, NG-10). The rejected
postures (P-B's channel, P-C's mutex, P-D's `async fn`) each name a
specific failure mode rather than a generic "complicated".

One remaining blocking concern is not a structural gap but a
tick-ordering hazard that R-001's chosen solution introduces and
that the plan does not fully resolve: in Phase 1 the
`Plic::tick`-via-`Device::tick` path runs *inside* the `fold` loop
at `bus.rs:227-240`, which visits `self.mmio` in registration
order. PLIC's vtable tick therefore runs *before* later-registered
adopter devices' own `tick()` — an adopter device whose internal
`tick()` produces a raise this cycle (the intended UART path) will
miss the current PLIC drain and defer to the next slow-tick. This
is a latency regression vs 00_PLAN's implicit "PLIC evaluates after
all device ticks" posture and undermines G-2's async-latency claim
for the primary in-tree adopter (UART's rx_buf→rx_fifo drain is
itself in `Uart::tick`). R-011 below names this; resolution is a
one-line Phase-1 step addition (order PLIC last in the slow-tick
loop, or run PLIC's `Device::tick` after the fold). The plan
already has `needs_reevaluate` as a cross-tick fallback so guest
correctness is preserved, but the stated G-7 "event-driven" + G-2
"latency reduction" invariants do not match the executed order.

Four non-blocking items: R-006 call-site table omits the real
machine-wire call site at `xemu/xcore/src/cpu/mod.rs:357` (R-012);
V-IT-1's "cross-thread raise" spawns + joins before calling tick,
which does not actually exercise the concurrent-raise-during-tick
race the happens-before claim is supposed to cover (R-013); the
`reset` ordering pins `pending_raises.store(true, Release)` as the
last step of `reset` but leaves the `level`/`edge_latch` stores
racing with a concurrent in-flight raise from another thread —
the narrative in F-6 admits this ("the next tick observes both
post-reset bits and stale pre-reset ordering artifacts") but does
not spell out whether the result is guest-observable (R-014); OQ-6
leans `AtomicBool` with no firm answer — for this iteration that
is fine, but the reviewer should note that a `u32` counter would
trivially solve the split-observation diagnostic question raised
by V-F-5 / loom deferral (TR-6).

TR-4 (T-4 pulse semantics) is addressed adequately — V-IT-6 + V-E-6
witness end-to-end, I-D4 documents the sticky-level hazard, NG-3
forecloses in-tree edge adopters. No new trade-off rejection
reasoning is needed.

Approve with revisions; Ready for Implementation = No (R-011 is the
single blocker).



---

## Findings

### R-011 Phase-1 tick order makes PLIC miss same-cycle adopter raises

- Severity: HIGH
- Section: Execution Flow / Implementation Plan / Architecture
- Type: Correctness
- Problem:
  `Bus::tick`'s slow-path at `bus.rs:227-240` calls `r.dev.tick()` for
  every non-MTIMER device inside a single `fold` pass that iterates
  `self.mmio` in registration order. Round 01 pins `Plic::tick` as the
  `Device::tick` override (I-D10-revised, §Dispatch at `01_PLAN.md:800-818`).
  The slow-tick ordering is therefore determined by MMIO registration
  order, not by the "drain after all device updates" posture that the
  §Async Posture section implicitly relies on. For a UART registered
  *after* PLIC in `Bus::mmio` (or for any adopter registered after
  PLIC), the ordering within one slow-tick is:
    1. `Plic::tick` runs — `take_epoch` swaps `pending_raises=false`,
       drains, evaluates, returns.
    2. Later in the same fold, `Uart::tick` drains `rx_buf` → `rx_fifo`,
       detects a newly-ready byte, calls `line.raise()` — this sets
       `level` and `pending_raises=true` *after* PLIC already drained.
    3. Next slow-tick (64 bus ticks later, SLOW_TICK_DIVISOR):
       `Plic::tick` observes `pending_raises=true`, drains, pends MEIP.
  The net effect is a one-slow-tick (≥ 64 bus ticks) latency floor for
  adopter-device raises that originate from the adopter's own `tick()`
  method — exactly the code path the feature is optimizing (UART's rx
  interrupt post-Phase-1). G-2 promises "any-thread raise is observed
  on the next bus-tick boundary"; under the adopted order, raises
  originating from the *bus-tick thread itself* (the common UART case,
  where the stdio reader thread hands bytes to rx_buf but the
  rx_buf→rx_fifo pump is still in `Uart::tick` per `uart.rs:94-129`)
  are deferred by a full slow-tick. Cross-thread raises from the UART
  reader thread that happen to arrive *before* the fold starts are
  covered correctly; raises that arrive during the fold (common, since
  the stdio reader is active throughout) are not. G-7's event-driven
  fast-path — the reason I-D14 was added — makes this *worse*: before
  round 01, `Plic::notify(bitmap)` ran unconditionally after the fold;
  round 01's `Plic::tick` runs inside the fold and early-returns when
  `pending_raises=false`, so the same-slow-tick evaluation window is
  now strictly narrower than 00_PLAN's.
- Why it matters:
  This is not a correctness bug — raises do propagate, just on the
  next slow-tick — but the deferred path invalidates the V-IT-7
  "event-driven" proof: V-IT-7 counts `sample_with` calls over 1000
  no-raise ticks, which will read zero regardless of order. It does
  not witness that an adopter-device raise originating from
  `Uart::tick` is observed in the *same* slow-tick. V-IT-3's "UART
  end-to-end" is weakened likewise: the test "bus-tick repeatedly
  until MEIP asserts" will pass even if the raise takes two
  slow-ticks to propagate. The observed regression vs 00_PLAN is
  silent. The failure mode in practice is a ≥ 64-bus-tick latency
  added to every UART rx interrupt, which at interactive typing
  cadence is imperceptible but at high-throughput serial IO (the
  debian-2hart boot gate) is noticeable as console lag.
- Recommendation:
  Round 02 must pin the slow-tick ordering so PLIC's `Device::tick`
  runs *after* every other device's `Device::tick`. Two concrete
  options:
    (a) Split `Bus::tick` so the slow path runs one pass of
        `r.dev.tick()` for every non-PLIC, non-MTIMER device, then
        a final `self.mmio[plic_idx].dev.tick()`. Removes the
        bitmap-fold+notify during Phase 1; replace with `Plic::tick`
        as the single post-pass evaluator. Phase 1 adopter devices
        return `false` from `irq_line` (already specified), so the
        notify path becomes inert for them and the bitmap-fold can
        stay as a no-op skeleton until Phase 2 deletes it.
    (b) Keep the fold but promote the PLIC's `Device::tick` out of
        it: after the fold completes (and after `plic.notify(bitmap)`
        for non-adopters in Phase 1), explicitly call
        `self.mmio[plic_idx].dev.tick()` so `Plic::tick` runs last.
        This costs one indirect call per slow tick.
  In either option, add a new invariant I-D16 "in a single slow-tick,
  `Plic::tick` runs after every other device's `Device::tick`" and a
  new validation V-IT-8: register a test device that raises an
  `IrqLine` from *its* `Device::tick`, then assert one `Bus::tick`
  slow-pass is sufficient for MEIP to assert. The plan already names
  the bus-side order in §Dispatch `01_PLAN.md:804-818` but calls it
  "unchanged" — it must not be unchanged; it must become PLIC-last.



### R-012 Phase-3 call-site table omits the real machine-wire site

- Severity: MEDIUM
- Section: Implementation Plan (Phase 3 step 4)
- Type: Spec Alignment
- Problem:
  §Phase 3 step 4's call-site table at `01_PLAN.md:999-1010` enumerates
  seven test-harness sites inside `xemu/xcore/src/device/bus.rs` plus
  two wildcards (`xemu/src/machine/*.rs`, `xemu/xcore/tests/*.rs`). It
  omits `xemu/xcore/src/cpu/mod.rs:357`, which is the canonical PLIC
  registration call site (`let plic_idx = bus.add_mmio(...)`). This is
  the site `arch_isolation.rs:76` counts as `("plic", 1)` per C-3 and
  the line that any Phase-3 diff must change. The table currently
  lists only the string-literal `"plic"` site at `bus.rs:512` which is
  a unit-test stub. The review's R-006 asked for every site named
  explicitly "so the Phase-3 diff is reviewable without grepping"; the
  present table still requires grepping to find the actual wire-up
  call.
- Why it matters:
  A reviewer of the Phase-3 diff who cross-checks against the table
  would miss the change at `cpu/mod.rs:357`. If the Phase-3 executor
  forgets it, `cargo check` catches the compile error but the review
  process does not flag the omission until implementation. The
  `BUS_DEBUG_STRING_PINS` cross-reference in the plan points at
  `"plic"` at `bus.rs:512` but the `arch_isolation.rs:71` comment
  explicitly calls out the test site — the real site at
  `cpu/mod.rs:357` is not pinned anywhere in either document.
- Recommendation:
  Add a row to the table:
  `| xemu/xcore/src/cpu/mod.rs | 357 | let plic_idx = bus.add_mmio("plic", …, plic, 0) |`
  Also verify `set_irq_sink` at `cpu/mod.rs:364` does not itself carry
  a source-id arg that would need retiring (it does not — it takes
  only the mmio index — but stating this closes the review loop).



### R-013 V-IT-1 does not exercise the cross-thread race it claims to

- Severity: MEDIUM
- Section: Validation (Integration Tests)
- Type: Validation
- Problem:
  V-IT-1 at `01_PLAN.md:1103-1106` reads: "spawn a thread that calls
  `line.raise()`; join; call `<Plic as Device>::tick`; assert the
  raise is observed (MEIP set). Exercises I-D9-revised happens-before."
  Join before tick is a full synchronization barrier — every prior
  `Release` store is guaranteed visible after join regardless of
  atomic ordering. The test would pass even if all `raise`/`lower`/
  `pulse` stores used `Relaxed` and `pending_raises.swap` used
  `Relaxed`. This does not witness the `Release`/`Acquire` HB pair
  pinned in I-D9-revised; it witnesses only "raise + tick serialized
  through a join boundary".
- Why it matters:
  I-D9-revised (`01_PLAN.md:480-494`) and the §Concurrency Posture
  table are the two artifacts the plan leans on to claim "no data
  race". The validation mapping at `01_PLAN.md:1189` says "I-D9-revised
  validated by V-IT-1, V-E-4, §Concurrency Posture". V-E-4 expanded
  covers the concurrent-raise-during-tick race, so it does exercise
  the true HB pair. V-IT-1 as written is redundant with "it compiles"
  — a bug in the orderings (e.g., swapping `Release` for `Relaxed`)
  would not cause V-IT-1 to fail. OQ-5 (`loom` deferred) is the
  real artifact; V-IT-1 should either (a) stop claiming to exercise
  HB, or (b) drop the join so that raise and tick are truly
  interleaved.
- Recommendation:
  Rewrite V-IT-1 as:
    - Spawner thread: `for i in 0..N { line.raise(); line.lower(); }`
    - Main thread concurrently: `for i in 0..M { plic.tick(); }`
    - After both complete, final `tick`, assert `pending_raises` is
      false and `level` is zero and the sum of observed MEIP
      assertions over all main-thread ticks ≥ 1.
  This drives the genuinely concurrent path V-E-4 seq B/C describes.
  Alternatively, demote V-IT-1 to "smoke test for cross-thread
  compile/link/basic wiring" and state explicitly that I-D9-revised
  is proved by §Concurrency Posture + deferred `loom` under OQ-5
  — do not claim the test validates the HB.



### R-014 Reset race: `reset` under a concurrent raise is unspecified

- Severity: MEDIUM
- Section: Invariants / Failure Flow / Validation
- Type: Correctness
- Problem:
  `PlicSignals::reset` at `01_PLAN.md:606-611` does three `Release`
  stores: `level <- 0`, `edge_latch <- 0`, `pending_raises <- true`.
  Reset is always called on the bus-tick thread (I-D8, I-D8a). A
  concurrent `IrqLine::raise` on another thread (UART reader,
  `uart.rs:94-129`) can interleave thus:
    - T1 (reset): `level.store(0, Release)`
    - T2 (raise): `level.fetch_or(bit, Release)` — bit set back to 1
    - T1 (reset): `edge_latch.store(0, Release)`
    - T2 (raise): `pending_raises.store(true, Release)`
    - T1 (reset): `pending_raises.store(true, Release)` — redundant
    - Bus tick `Plic::tick`: drains; observes `level=bit`, `edge_latch=0`;
      pends src.
  This is F-6's case, and the plan's narrative "the only guest-visible
  outcome is 'interrupt asserted soon after reset' — acceptable" is
  correct but glosses over two sub-cases:
    (i) T2's `raise` corresponds to a pre-reset device event (a byte
        that arrived before the guest issued the reset MMIO write); it
        *should* be discarded on reset semantics.
    (ii) T2's `raise` corresponds to a post-reset device event (the
         reader thread already sent a new byte after reset began); it
         *should* be delivered.
  The reset handler cannot distinguish (i) from (ii) and will always
  deliver. For UART + PLIC hard_reset this is harmless (the guest
  re-enables interrupts before ack-ing). For a device with
  side-effectful "raise = consume one queued event" semantics (none
  in-tree, but the pattern is not forbidden by the trait), it would
  drop an event.
- Why it matters:
  F-6 names the race but does not bound it. V-F-1 "raise during reset"
  asserts only "some raise post-reset-completion is observed" — this
  is the weaker property. The plan would benefit from a precise
  statement: "reset serializes with raise only at the `pending_raises`
  Release edge; bits set concurrently with reset may persist". Then
  V-F-1 can assert this precisely rather than the current vague form.
- Recommendation:
  Either:
    (a) Document a new invariant (re-number after R-011 takes I-D16):
        "`PlicSignals::reset` is serialized with the bus-tick thread
        only. Raises from other threads interleaved with reset may set
        post-reset-visible bits. Callers expecting full quiescence
        must coordinate device-side first." And tighten V-F-1 to
        enumerate the outcomes.
    (b) Acquire-Release the reset: `pending_raises.swap(true, AcqRel)`
        as the first step of reset, followed by the level / edge_latch
        stores. Still does not make the reset truly atomic with a
        concurrent raise (nothing can, without a lock or RCU), but
        sharpens the observable race so V-F-1 can assert a narrower
        post-condition.
  Either is acceptable; silence is not.



---

## Trade-off Advice

### TR-6 `pending_raises: AtomicBool` vs `AtomicU32` epoch counter (OQ-6)

- Related Plan Item: `T-6` / OQ-6
- Topic: Performance vs Diagnostic clarity
- Reviewer Position: Prefer Option A (AtomicBool) as adopted
- Advice:
  `AtomicBool` with `Release`/`Acquire` is the right call for this
  iteration. The §Concurrency Posture argument is correct — HB via
  a boolean atomic is sufficient for the "no lost raise" property.
- Rationale:
  An `AtomicU32` counter would add diagnostic value only: it lets a
  future observer verify "N raises occurred between drains" without
  replaying. The cost is one extra cache line of state (actually
  zero — `AtomicBool` is padded to a word anyway) and a handful of
  lines of code. The benefit is not zero but is entirely in the
  "future `loom` debug" lane, which is OQ-5. Pin the decision:
  ship `AtomicBool`; if OQ-5 ever gets implemented and the
  exhaustive check wants raise-count diagnostics, promote then.
- Required Action:
  Adopt as is. State in OQ-6 closure: "`AtomicBool` adopted; promote
  to `AtomicU32` only if OQ-5 `loom` validation is ever taken up
  and needs raise-count diagnostics."



### TR-4 Reviewer position on pulse semantics (from 00_REVIEW)

- Related Plan Item: `T-4`
- Topic: Flexibility vs Safety
- Reviewer Position: Accept
- Advice:
  00_REVIEW's TR-4 asked for either (a) a named future consumer
  justifying Option A's sticky-level semantics or (b) revisit
  Option C. Round 01's answer is "V-IT-6 + V-E-6 witness end-to-end;
  I-D4 + NG-3 document the hazard; future edge adopter can revisit".
  This is route (a-lite): no named consumer, but a documented hazard
  plus a witness path. Given NG-3's explicit "no in-tree edge
  adopter this feature", route (a-lite) is acceptable.
- Rationale:
  The stuck-level hazard is real but only visible to future edge
  adopters; a future plan with a concrete consumer can revisit
  T-4 without paying the cost now. V-IT-6 + V-E-6 make the current
  Option A observable end-to-end so a future regression would be
  caught.
- Required Action:
  Adopt as is. Close TR-4.



---

## Positive Notes

- The §Async Posture section (`01_PLAN.md:346-454`) is the strongest
  single artifact in round 01. Four postures, each with a concrete
  code sketch and rejection rationale tied to existing NGs and Cs.
  The summary criterion matrix at `444-454` is exactly the reviewer-
  digestible form the M-001 directive asked for.
- §Concurrency Posture (`738-757`) makes every atomic op + ordering
  + justification explicit. This is the cleanest response to the
  M-002 "atomics alone is forbidden wording" warning.
- R-001's resolution is unambiguous: `Device::tick` override, no
  inherent method, no downcast, with I-D10-revised making it
  checkable in one grep. The §Dispatch section at `798-818`
  spells out the Phase-1 bus-side order end-to-end.
- R-004's resolution places I-D8a directly in the `Plic.signals`
  data-structure paragraph (`01_PLAN.md:641`) with V-UT-10 paired —
  exactly where the review asked. The `reset` in §Concurrency
  Posture adds a `pending_raises.store(true, Release)` as a
  defensive "force next tick to drain" which is a nice touch beyond
  the literal review request.
- Response Matrix (`206-224`) is complete — every CRITICAL/HIGH
  finding and every MASTER directive is present with a decision
  and a resolution pointer.
- Changes-from-Previous-Round block (`144-201`) correctly
  distinguishes Added/Changed/Removed and names each new artifact
  (invariants, constraints, validations).
- Phase gating preserved: every phase still has boot-trio + 374
  baseline + arch_isolation + C-13 no-new-crate checks. No
  smuggled scope creep.
- Inherited MASTER directives (archModule, archLayout, plicGateway
  I-9) are explicitly honored in the Response Matrix — no silent
  drift.



---

## Approval Conditions

### Must Fix
- R-011 (HIGH) — Phase-1 tick order must ensure `Plic::tick` runs
  last so same-slow-tick adopter raises are observed within one
  slow-tick. Add new invariant + V-IT-8 per recommendation.

### Should Improve
- R-012 — Add `xemu/xcore/src/cpu/mod.rs:357` to the Phase-3
  call-site table.
- R-013 — Rewrite V-IT-1 as genuinely concurrent, or demote it
  and credit I-D9-revised to §Concurrency Posture + OQ-5 only.
- R-014 — Tighten the reset-vs-concurrent-raise contract (new
  invariant or `AcqRel` reset head) and sharpen V-F-1 to match.

### Trade-off Responses Required
- TR-6 (OQ-6) — Close as "AtomicBool adopted; revisit only under
  OQ-5 loom follow-up".
- TR-4 (from 00_REVIEW) — Close as accepted; sticky-level hazard
  documented via I-D4 + NG-3 + V-IT-6/V-E-6.

### Ready for Implementation
- No
- Reason: R-011 is a HIGH that materially undermines the G-2 + G-7
  invariants the plan's §Async Posture was written to guarantee.
  Resolution is a small, local change (PLIC-last ordering + one new
  invariant + one new validation) so round 02 should close cleanly.
  The three MEDIUM findings (R-012/013/014) do not block but
  should be resolved in the same round to avoid a third iteration
  just for them.
