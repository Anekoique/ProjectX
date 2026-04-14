# `directIrq` REVIEW `02`

> Status: Open
> Feature: `directIrq`
> Iteration: `02`
> Owner: Reviewer
> Target Plan: `02_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved
- Blocking Issues: `0`
- Non-Blocking Issues: `3`



## Summary

Round 02 is a tight delta pass that closes every finding from
01_REVIEW cleanly and satisfies the three MASTER directives
end-to-end. R-011 (the sole HIGH blocker) lands as I-D16 plus a
concrete `Bus::tick` rewrite (`02_PLAN.md:352-387`) that excludes
`plic_idx` from the slow-fold and explicitly invokes
`self.mmio[plic_idx].dev.tick()` afterwards; the chosen option (b)
from 01_REVIEW is implementable, matches the existing `mtimer_idx`
exclusion pattern at `bus.rs:219-221`, and the
`notify`-then-`tick` sequence in Phase 1 correctly lands because
`Plic::notify` sets `needs_reevaluate = true` (`01_PLAN.md:709`),
which disarms the `!needs_reevaluate` fast-path guard in
`Plic::tick`. V-IT-8 (`02_PLAN.md:423-458`) is concrete — a
`RaisingDevice` whose own `Device::tick` calls `line.raise()`,
asserted to produce MEIP in exactly one slow pass — and the
registration-order-reversed variant provides a regression
invariant against future re-introduction of order coupling.

R-012 is closed: `xemu/xcore/src/cpu/mod.rs:357` is verified as the
canonical `bus.add_mmio("plic", …)` call site (confirmed against
source), and the plan also flags `cpu/mod.rs:365` as the UART site
behind the `xemu/src/machine/*.rs` wildcard row, which was an
unstated R-012 sub-concern.

R-013 is closed by splitting V-IT-1 into V-IT-1a (genuine
interleave; no join barrier before the final tick) and V-IT-1b
(demoted smoke). V-IT-1a is honest about its limits — on x86 TSO
a `Relaxed` ordering bug would be invisible and the plan names
aarch64 CI + OQ-5 `loom` as the real escalation. V-IT-1b is
correctly dropped from the Acceptance Mapping's I-D9-revised row
(`02_PLAN.md:742`).

R-014 is resolved by pinning the reset semantics to real silicon
(§Hardware-Semantic Grounding H-1 + H-3): a raise-during-reset is
delivered, not dropped. V-F-1 is rewritten as a positive
delivery assertion (`02_PLAN.md:642-685`) which is strictly
stronger than round 01's "some raise is observed" wording.

The §Hardware-Semantic Grounding section (`02_PLAN.md:139-330`)
is the single biggest round-02 addition and fully satisfies
M-002 and M-003. Five H-blocks, each with an authoritative
URL — RISC-V PLIC v1.0.0 for gateway + claim/complete semantics
(H-1, H-2), Rust Nomicon + `std::sync::atomic::Ordering` for the
Release/Acquire contract (H-4), ARM GIC v3 for cross-ISA
level-sensitive validation (H-3), and QEMU `hw/irq.h` +
airbus-seclab for emulator prior art (H-5). Manual passages are
quoted, not just cited. The ARM GIC comparison is substantive —
it shows that `level` / `edge_latch` match the universal
controller contract rather than a RISC-V idiosyncrasy — and
directly supports the R-014 decision (reset does not suppress
peripheral signalling). M-002's "implementation convenience
reasoning removed" obligation is honoured: the plan explicitly
reverses F-6's 01 posture ("implementation convenience was the
argument *against*", `02_PLAN.md:606-610`) once hardware
semantics are grounded.

Three non-blocking items remain. R-015 (MEDIUM): V-IT-8's
"negative control" at `02_PLAN.md:456-458` is miscategorised —
registration-order-reversed is a *positive regression-invariance*
test for I-D16, not a negative control of the pre-fix bug; a true
negative control would assert the pre-fix code fails the same
one-pass claim. The test is still valuable, the label is wrong.
R-016 (LOW): the Bus::tick sketch in §Tick-Order Resolution is
explicitly "[Phase 1 only]" but the plan does not sketch the
Phase 2 shape, and the Phase 2 gate narrative ("bitmap fold
deleted") could be read as dropping the device-tick loop itself;
01_PLAN:969-972 does say "Keep the per-device tick() loop" but
the Phase-2 form of §Tick-Order Resolution should be made
explicit in 02_PLAN so Phase-2 reviewers have the full picture.
R-017 (LOW): V-F-1 uses `thread::sleep(1ms)` twice to create the
race window; this is mild flakiness risk on loaded CI runners,
and a bounded-iteration approach (spawn raiser, loop for N
`plic.tick`s or until raiser asserts N raises completed) would
be more deterministic.

TR-6 and TR-4 carry over as closed per 01_REVIEW; no new
trade-offs surface in round 02. Async posture (M-001) is correctly
treated as settled — the re-confirmation in the Response Matrix
(`02_PLAN.md:121`) and the tie-back via H-4/H-5 are sufficient;
no ground is re-litigated.

No regressions against I-D1..I-D15. Phase gating, C-2 / C-13
diffs, boot trio gate, and `DEBUG=n` project-memory pin all
preserved. The three non-blocking items are cosmetic or
follow-up hardening; none would materially change the
implementation trajectory. Approved.



---

## Findings

### R-015 V-IT-8 "negative control" is actually a positive regression-invariance test

- Severity: MEDIUM
- Section: Validation / §Tick-Order Resolution
- Type: Validation
- Problem:
  `02_PLAN.md:456-458` describes a second V-IT-8 variant:
  "a test variant that registers the raiser *before* PLIC
  (`add_mmio` order reversed) asserts the same outcome — proves
  the fix is not registration-order-dependent." This is labelled
  a "negative control". In testing vocabulary, a negative control
  is a case expected to *fail* or produce a null outcome so that a
  positive result demonstrates the mechanism is load-bearing. The
  reversed-registration variant asserts the *same* positive
  outcome as the primary V-IT-8, which is a regression-invariance
  check (good) rather than a negative control (what the plan
  claims). A true negative control for I-D16 would be: pin the
  pre-fix code path (inline PLIC into the fold in registration
  order) and assert it fails the "one slow pass" claim, then show
  the restructured Bus::tick passes. Without that, V-IT-8 does
  not independently witness that the I-D16 ordering change is
  load-bearing — it witnesses only that the post-fix code is
  correct in both registration directions.
- Why it matters:
  R-011's concern was not symmetry between registration orders;
  it was that the specific ordering `PLIC before later-registered
  adopter` produced a silent one-slow-tick latency regression. A
  test that passes in both orders post-fix does not prove the
  fix was necessary. A future refactor that "simplifies"
  Bus::tick back into a single fold (perhaps keeping plic_idx
  inside) would silently break the G-2 latency claim again; the
  current V-IT-8 catches *some* forms of that regression
  (specifically: raiser-after-PLIC registration order) but not
  all (raiser-before-PLIC would still pass because PLIC-in-fold
  would evaluate the raiser's bits on the same fold iteration
  via `needs_reevaluate` fallback). This is also a clarity issue
  for future reviewers — the "negative control" label sets an
  expectation the test does not meet.
- Recommendation:
  Either (a) re-label: "V-IT-8 regression-invariance variant:
  same outcome under reversed registration order; guards I-D16
  against registration-order coupling." No test code changes, just
  honest naming. Or (b) add a genuine negative control: a test
  that temporarily pins the pre-fix fold order (either via a
  `#[cfg(test)]` alternate `Bus::tick_legacy` or a feature-gated
  toggle, or most cleanly by asserting the V-IT-8 scenario
  requires exactly *one* slow pass and documenting via a comment
  that a legacy in-fold order would require two passes). Option
  (a) is cheaper and sufficient given C-13's "no new crate"
  posture rules out property-based testing frameworks.



### R-016 Bus::tick Phase-2 shape left implicit

- Severity: LOW
- Section: §Tick-Order Resolution / Gates (Phase 2)
- Type: Spec Alignment / Flow
- Problem:
  The `Bus::tick` code sketch at `02_PLAN.md:352-387` is labelled
  "[Phase 1 only]" inline and collects the legacy bitmap via a
  fold that also ticks each device. Phase 2 gate at
  `02_PLAN.md:761-762` says "Bitmap fold deleted from `Bus::tick`
  but the explicit `plic.tick()` call (per I-D16) remains."
  01_PLAN:971 adds "Keep the per-device `tick()` loop" so the
  underlying intent is clear, but 02_PLAN does not sketch the
  Phase-2 or Phase-3 form of the restructured `Bus::tick`. A
  reviewer reading 02_PLAN in isolation must mentally cross-apply
  01_PLAN:969-972 against 02_PLAN's Phase-1 sketch to reconstruct
  the post-phase-2 shape: a `for r in self.mmio.iter_mut()` loop
  that ticks every device except `mtimer_idx` and `plic_idx`,
  then an explicit `self.mmio[plic_idx].dev.tick()`.
- Why it matters:
  The Phase-1 sketch's fold also carries the bitmap computation;
  the Phase-2 shape should be a plain tick loop. Without the
  Phase-2 sketch, a reader could interpret "delete the fold" as
  "delete the iteration over devices entirely", which would drop
  the device tick calls for non-MTIMER non-PLIC devices — a
  correctness regression. The 01_PLAN language protects against
  this but the 02_PLAN delta should re-pin it to avoid
  cross-document interpretation.
- Recommendation:
  Round 03 (if one is needed) or the implementation note should
  add a small Phase-2 sketch to §Tick-Order Resolution:

  ```rust
  // Phase 2 form:
  pub fn tick(&mut self) {
      if let Some(i) = self.mtimer_idx { self.mmio[i].dev.tick(); }
      self.tick_count += 1;
      if !self.tick_count.is_multiple_of(SLOW_TICK_DIVISOR) { return; }
      for (idx, r) in self.mmio.iter_mut().enumerate() {
          if Some(idx) == self.mtimer_idx || Some(idx) == self.plic_idx {
              continue;
          }
          r.dev.tick();
      }
      if let Some(i) = self.plic_idx { self.mmio[i].dev.tick(); }
  }
  ```

  Non-blocking for approval — the 01_PLAN guard language is
  sufficient — but its absence is a small clarity cost.



### R-017 V-F-1 uses wall-clock sleeps to create the race window

- Severity: LOW
- Section: Validation / §Reset-Race Outcome
- Type: Validation
- Problem:
  V-F-1 at `02_PLAN.md:642-685` uses two
  `std::thread::sleep(Duration::from_millis(1))` calls — once to
  let the raiser spin before `reset`, once to keep raising
  post-reset. On a loaded CI runner (e.g., a container
  oversubscribed during the 374-baseline run), 1ms of wall-clock
  may elapse before the spawned thread schedules or may elapse
  with only a handful of raises; in the degenerate case a
  scheduling hiccup could leave `line` un-raised in the post-reset
  window, failing the "MEIP asserted" assertion even though the
  semantics are correct.
- Why it matters:
  V-F-1 is the positive-delivery witness R-014 asked for; if it
  flakes, a future CI failure will be hard to diagnose (is it a
  timing issue or a regression in the reset-race semantics?).
  This is exactly the class of test that `loom` / deterministic
  scheduling would eliminate, which is why OQ-5 exists — but
  absent `loom`, the test should be deterministic under normal
  CI load.
- Recommendation:
  Replace the sleeps with a bounded-iteration coordinator:
  - Use an `AtomicUsize` "raise count" incremented by the raiser
    every iteration.
  - Main thread: spawn raiser; spin-wait until raise_count >= K
    (e.g., 100); call `reset`; spin-wait until raise_count has
    advanced by at least K more; stop; tick; assert.
  This removes wall-clock dependence and still exercises the
  race. Alternatively, use a `std::sync::Barrier` for the
  pre-reset handshake and a second `AtomicUsize` high-water-mark
  for the post-reset one. Cost: two extra atomics. Non-blocking.



---

## Trade-off Advice

No new trade-offs in round 02. TR-6 (AtomicBool) and TR-4
(pulse semantics) remain closed per 01_REVIEW; no re-opening is
warranted.



---

## Positive Notes

- §Hardware-Semantic Grounding (`02_PLAN.md:139-330`) is an
  exemplary response to M-002 + M-003. Each H-block quotes
  manual passages verbatim, names the specific spec section, and
  maps the passage onto a specific design artifact in 01_PLAN
  (level/edge_latch, `needs_reevaluate`, Release/Acquire
  orderings, the event-driven pull model). Five authoritative
  sources (RISC-V PLIC v1.0.0, Rust Nomicon, `std::sync::atomic`
  docs, ARM GIC v3, QEMU `hw/irq.h`) plus two secondary; every
  URL is in the §Citations appendix so a future auditor can
  re-verify. The tie-back summary at `02_PLAN.md:324-330`
  ("Every design choice in 01_PLAN that touches timing or
  ordering has a manual citation above") is checkable and true.
- H-3 (ARM GIC) is not window-dressing. It supplies the
  cross-ISA grounding M-002 asked for: the level-sensitive
  contract is universal, so the `level` atomic plane is not a
  RISC-V idiosyncrasy. It also directly underpins R-014's
  decision that raise-during-reset is delivered (the controller's
  reset clears controller state, not source state) — without H-3,
  R-014's "matches real hardware" claim would rest solely on
  RISC-V PLIC inference.
- R-014's reasoning reversal ("implementation convenience was
  the argument *against* this posture in 01_PLAN F-6. The
  hardware-grounded argument flips the conclusion",
  `02_PLAN.md:606-610`) is a clean example of M-002 discipline:
  the executor explicitly identifies a round-01 decision that
  rested on convenience, looks it up in hardware manuals, and
  documents the reversal. This is exactly the M-002 process.
- R-011's mechanism choice (option (b), promote PLIC tick out of
  the fold) matches the existing `mtimer_idx` exclusion pattern
  so Bus's internal symmetry is preserved; the plan calls this
  out explicitly at `02_PLAN.md:403-408`. The code sketch is
  complete enough to implement without further architectural
  decisions.
- V-IT-1a is honest about its limits. The plan states that on
  x86 TSO a Relaxed bug is invisible (`02_PLAN.md:558-562`) and
  names aarch64 CI + OQ-5 as the escalation. This is the
  opposite of the V-IT-1 round-01 overclaim — the round-02 test
  is paired with a correctly-scoped justification.
- Response Matrix (`02_PLAN.md:117-132`) is complete: every
  R-011..R-014 finding, every MASTER directive (M-001/M-002/M-003
  + the three inherited), and both carry-over TRs (TR-4, TR-6)
  have a decision + resolution pointer. No rejections, no
  silent drift.
- I-D16 pinned with both narrative statement and code mechanism;
  Risk 8 ("future refactor re-introduces PLIC into the fold")
  is named explicitly with V-IT-8 as the mitigation. This is the
  correct way to encode a "don't silently undo this" invariant.
- Phase gating preserved end-to-end: Phase 1 gate +12 tests
  (one more than round 01 for V-IT-8), Phase 2/3 unchanged, boot
  trio + `DEBUG=n` + C-2 + C-13 diffs all still present.
- OQ-6 closed cleanly per TR-6; OQ-5 deferred with an explicit
  re-open trigger ("if V-E-4 or V-IT-1a flakes under stress on
  weakly-ordered targets").



---

## Approval Conditions

### Must Fix
- None.

### Should Improve
- R-015 (MEDIUM) — re-label V-IT-8's reversed-registration
  variant as a regression-invariance check rather than a
  "negative control"; optionally add a true negative control.
- R-016 (LOW) — add a Phase-2 sketch of `Bus::tick` to
  §Tick-Order Resolution to foreclose misinterpretation of
  "delete the bitmap fold".
- R-017 (LOW) — replace V-F-1's `thread::sleep` coordination
  with deterministic atomic-counter handshakes to avoid CI
  flakiness.

### Trade-off Responses Required
- None. TR-4 and TR-6 closed per 01_REVIEW; no new TRs.

### Ready for Implementation
- Yes
- Reason: No CRITICAL findings; no HIGH findings; the three
  MEDIUM/LOW items are cosmetic or flakiness-hardening and do
  not affect the implementation trajectory. R-011..R-014 are
  resolved with concrete artifacts (I-D16, V-IT-8, cpu/mod.rs:357
  row, V-IT-1a/1b split, V-F-1 positive assertion) and
  M-001/M-002/M-003 are satisfied end-to-end by the
  §Hardware-Semantic Grounding section. R-015..R-017 can be
  absorbed as implementation-time refinements or deferred to a
  round-03 tightening pass if one materialises for unrelated
  reasons; they do not gate the Phase-1 diff.
