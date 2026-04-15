# `perfBusFastPath` REVIEW `02`

> Status: Open
> Feature: `perfBusFastPath`
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

- Decision: Approved with Revisions
- Blocking Issues: 0
- Non-Blocking Issues: 7

## Summary

Round 02 is a substantial improvement over 01. The executor applied
M-001 in full: `Arc<Mutex<Bus>>` is replaced by a single `Box<Bus>`
owned by `CPU`, the two-arm `BusHandle` enum / `BusOwner` factory /
`Shared` branch are excised completely, and `Core::step` takes
`bus: &mut Bus` as a threaded parameter. The migration table
faithfully reproduces an independent `rg "bus\.lock\(\)" xemu -n`
(24 hits), including the five multi-line test-helper sites at
`inst/base.rs:344-356`, `inst/compressed.rs:552-556`,
`inst/float.rs:1075-1079`, `arch/riscv/cpu.rs:277-282` that 01 missed.
Every prior C/H/M/L finding and the M-001 directive appear in the
Response Matrix with a concrete gate. Multi-hart semantics, LR/SC
atomicity (I-4), tick ordering (I-5), and reset order (I-6) are all
preserved bit-for-bit.

The plan is implementable as written and would produce a correct,
reviewable 1:1 lock-removal diff. The remaining issues are
non-blocking: V-UT-5's `type_name` assertion is weaker than the plan
claims (cannot see field types of non-generic structs); the
`Box<Bus>` vs inline `Bus` choice is stated without justification;
the 2-hart `+/-5 %` gate does not name its baseline artifact; the
borrow-splitting story at `CPU::step` works but is not documented as
a disjoint-field borrow, which will matter when PR reviewers hit the
`self.cores[i].step(&mut *self.bus)` line and wonder why it compiles;
one LR/SC peer-hart-exclusion sentence is missing from I-4; the
Phase-1 intermediate commit briefly violates the zero-Mutex
invariant without a stated gate-exemption policy. None of these
block implementation; all should be tightened in round 03 or folded
into the PR description.

---

## Findings

### R-001 `V-UT-5 type_name assertion does not actually detect Mutex`

- Severity: HIGH
- Section: Validation Strategy / V-UT-5; Response Matrix (H-2, M-001)
- Type: Validation
- Problem:
  V-UT-5 asserts
  `!std::any::type_name::<CPU<RVCore>>().contains("Mutex")` and the
  same for `RVCore`. `std::any::type_name::<T>()` returns the name
  of `T` itself — for `CPU<RVCore>` it is literally
  `"xcore::cpu::CPU<xcore::arch::riscv::RVCore>"`. The function
  does NOT expand the type's field types. A future contributor
  re-adding `bus: Arc<Mutex<Bus>>` as a private field would leave
  the `type_name` output unchanged and the test would still pass.
- Why it matters:
  The Response Matrix for H-2 and M-001 leans on V-UT-5 as the
  runtime half of the "zero mutex" gate. If the assertion is a
  no-op, the only real gate is the repo-level
  `scripts/verify_no_mutex.sh` grep. The plan must acknowledge
  this rather than present two independent sentinels.
- Recommendation:
  Either (a) drop the `type_name` assertion and rely on the
  `verify_no_mutex.sh` grep gate alone — this is fine, since the
  compile-time change to `CPU::bus: Box<Bus>` already prevents
  silent regression across a full rebuild — or (b) replace it
  with a check that actually works: e.g. a private helper
  `fn _assert_bus_type(c: &CPU<RVCore>) -> &Bus { &c.bus }` whose
  signature would break if the field reverts to
  `Arc<Mutex<Bus>>`, or a `const _: () =
  assert!(size_of::<CPU<RVCore>>() < N)` that pins the struct
  layout. State in round 03 which option is picked and why.

### R-002 `Disjoint-field borrow at CPU::step is unstated`

- Severity: MEDIUM
- Section: Execution Flow / Main Flow step 2; Invariants I-3
- Type: Correctness
- Problem:
  Main Flow step 2 shows
  `self.cores[self.current].step(&mut *self.bus)` inside
  `CPU::step(&mut self)`. This compiles because Rust's
  disjoint-field borrow inference treats `self.cores` and
  `self.bus` as separate places on `self`. The plan does not name
  the idiom, does not explain why NLL analysis accepts it, and
  does not warn Phase-2 implementers that routing the access
  through an accessor method on `&mut self` (e.g. `self.bus_mut()`
  or `self.current_core_mut()`) would collapse the disjoint path
  and hit E0499.
- Why it matters:
  The invariant "one `&mut Bus` per `CPU::step`, handed out by the
  cooperative scheduler" (I-3, G-3) is the core correctness claim
  of this phase. Its Rust-level soundness depends on the
  disjoint-field pattern. A later cleanup pass that extracts
  `self.bus.tick()` and the core step into a helper method on
  `&mut self` would stop compiling. Reviewers of the eventual PR
  will reasonably ask "why does this compile?" and absent an
  answer in the PLAN will debate it in the PR.
- Recommendation:
  Add one sentence to I-3 and one to Execution Flow step 2:
  "The `&mut *self.bus` reborrow coexists with
  `&mut self.cores[self.current]` because the borrow checker sees
  two disjoint fields of `self`; this requires the two accesses to
  appear directly on `self` inside `CPU::step`'s body, not behind
  a helper method on `&mut self`." One paragraph saves a round of
  PR back-and-forth.

### R-003 `Box<Bus> vs inline Bus is unjustified`

- Severity: MEDIUM
- Section: Data Structure; Invariants I-1
- Type: Design
- Problem:
  The plan picks `bus: Box<Bus>` without comparison. Inline `Bus`
  would save one pointer hop per access (the `Box` deref compiles
  to a load off `CPU`); heap placement does not help here since
  `CPU` itself is constructed and owned in a stable location.
  Conversely, inline `Bus` enlarges `CPU` and moves `Bus`'s
  `Vec<MmioRegion>` and `Vec<Option<usize>>` headers adjacent to
  `cores`, which may or may not be cache-friendly.
- Why it matters:
  G-2's gain band (15-30 %) relies on removing mutex work without
  re-introducing a new indirection in its place. `Box<Bus>` adds
  one pointer hop per `self.bus.X()` — cheap but not free. The
  plan should state whether `Box<Bus>` is chosen for migration-diff
  minimality (drop-in for the removed `Arc`) or for some other
  reason, so a future inline-vs-boxed pass has a documented
  starting point.
- Recommendation:
  Add a one-paragraph rationale to Data Structure: "`Box<Bus>` is
  chosen for P1 as a drop-in replacement for the removed
  `Arc<Mutex<Bus>>` — the single pointer hop is negligible next
  to the removed `pthread_mutex_*` bucket, and keeping `CPU`'s
  footprint stable minimises diff churn. Inlining `Bus` is a
  two-line change available as a polish pass once P1 numbers are
  in hand." Or promote to a T-5 trade-off with the same wording.

### R-004 `LR/SC peer-hart exclusion argument is implicit`

- Severity: MEDIUM
- Section: Invariants I-4; Goals G-3 / G-4
- Type: Correctness / Invariant
- Problem:
  I-4 says "one `&mut Bus` borrow covers translate +
  reservation-check + conditional-store"; G-4 says "LR/SC
  atomicity preserved." Neither states the peer-hart argument:
  reservations live on `Bus` (`bus.rs:65 reservations:
  Vec<Option<usize>>`); under cooperative round-robin only one
  hart's `Core::step` runs per `CPU::step`; that hart holds
  `&mut Bus` exclusively; therefore no peer hart can race against
  the reservation-check-then-conditional-store sequence. Today
  the mutex enforces peer exclusion at the OS level; tomorrow the
  `&mut Bus` borrow enforces it at the type-system level. Both
  are equally strong under a single-threaded scheduler.
- Why it matters:
  LR/SC correctness is the most-scrutinised invariant of this
  phase. A reviewer who has not internalised the
  cooperative-scheduler premise will read I-4 as "we consolidated
  three locks into one borrow scope" (correct as far as it goes)
  without seeing that peer-hart exclusion was always the mutex's
  job and is now the borrow checker's job. G-3 gestures at this
  but never names `bus.reservations` explicitly.
- Recommendation:
  Add one sentence to I-4: "Peer-hart exclusion for
  `bus.reservations[hart]` is preserved because the cooperative
  round-robin scheduler grants exactly one `&mut Bus` borrow per
  `CPU::step`; the borrow checker replaces the mutex as the
  exclusion primitive for reservation state. This suffices under
  the current single-threaded scheduler; Phase 11 true-SMP is out
  of scope (see T-1)."

### R-005 `Phase 1 intermediate commit briefly re-introduces the Mutex`

- Severity: MEDIUM
- Section: Implementation Plan / Phase 1 step 1d; Exit Gate
- Type: Maintainability / Gate Policy
- Problem:
  Step 1d states: "Temporarily keep `RVCore::bus` as
  `Arc<Mutex<Bus>>` so arch code still compiles; construct the
  wrapper inside `CPU::new` from the `Box<Bus>` via a
  clone-on-construction shim guarded by `#[deprecated]` comment.
  (This shim exists for exactly one commit between Phase 1 and
  Phase 2 and is removed in Phase 2a.)"
  This means one mid-series commit contains BOTH a `Box<Bus>` on
  `CPU` and `Arc<Mutex<Bus>>` on `RVCore`, wired by a shim. That
  commit violates M-001, C-1 / I-8, and would fail the V-UT-5
  shell gate (`rg -q "Mutex|Arc<Mutex<Bus>>|bus\.lock\(\)"`). The
  plan elsewhere says "after each logical group ... run `make fmt
  && make clippy && cargo test --workspace`" — implying per-group
  CI — but does not exempt step 1d from the zero-Mutex grep.
- Why it matters:
  A single mid-series commit that briefly violates the hard
  invariant is defensible, but only if called out explicitly and
  the CI gate is understood to apply at end-of-Phase-2. As
  written, the plan implies two incompatible gate policies.
- Recommendation:
  Pick one: (a) restructure so Phase 1 + Phase 2 land as a single
  atomic PR with no intermediate Mutex-bearing commit, skipping
  the shim entirely; or (b) state that
  `scripts/verify_no_mutex.sh` runs only at end-of-Phase-2 and
  end-of-Phase-3 (not per-commit), and that the 1d commit is
  exempt with a commit-message marker. Option (a) is cleaner.

### R-006 `2-hart +/-5% gate has no named baseline file`

- Severity: MEDIUM
- Section: Constraints C-4; Validation V-IT-4
- Type: Validation
- Problem:
  C-4 and V-IT-4 both require `make linux-2hart` to boot "within
  +/-5 % of `docs/perf/2026-04-14/data/bench.csv`", but
  `bench.csv` is a per-benchmark table (dhrystone / coremark /
  microbench) and does not contain a 2-hart Linux boot-to-shell
  time. Either the baseline is captured under a different
  filename that the plan does not name, or it has not been
  captured at all.
- Why it matters:
  Without a named baseline, "+/-5 %" is unfalsifiable and the
  gate cannot actually block the PR. This is the single most
  important multi-hart correctness signal in the plan; C-4
  depends on it entirely.
- Recommendation:
  Add a Phase-3 prerequisite: "capture
  `docs/perf/2026-04-14/data/linux_2hart_boot.txt` from the
  pre-P1 tree (3 samples, DEBUG=n, `make linux-2hart`, time to
  `/# ` prompt) and commit alongside the PLAN so V-IT-4 has a
  named ground truth." Or point to the existing file path if it
  already exists in the data directory.

### R-007 `Optional cargo-show-asm symbol path is incomplete`

- Severity: LOW
- Section: Phase 3 step 3f; Exit Gate "Nice-to-have"
- Type: Maintainability
- Problem:
  Step 3f names the `CPU::step` symbol as
  `xcore::cpu::CPU::step` without the generic parameter.
  `cargo-show-asm` typically needs the fully-monomorphised name
  (e.g. `xcore::cpu::CPU<xcore::arch::riscv::RVCore>::step`) or a
  prior `--list` pass to resolve. Verbatim execution will miss.
- Why it matters:
  Minor — the gate is optional and the worst case is a 10-minute
  debugging session for whoever runs it. Still worth fixing since
  it is the one concrete command in the perf-evidence appendix.
- Recommendation:
  Replace with: "`cargo asm -p xcore --list | rg 'CPU.*step'`,
  then run `cargo asm -p xcore '<matching-line>' --rust`." Or
  write the fully-qualified generic symbol.

---

## Trade-off Advice

### TR-1 `Owned Box<Bus> vs inline Bus`

- Related Plan Item: T-1 / Data Structure I-1
- Topic: Performance vs Migration Simplicity
- Reviewer Position: Prefer Option A (`Box<Bus>`) for P1; keep
  inline on the table for a later polish pass
- Advice:
  Land `Box<Bus>` as the plan specifies. Do not block round 03 to
  choose between boxed and inline; the difference is below the
  measurement noise once `pthread_mutex_*` is gone. Revisit
  inline in a dedicated perf PR after the P1 baseline is captured.
- Rationale:
  `Box<Bus>` minimises diff churn (drop-in for the removed `Arc`)
  and keeps `CPU`'s size stable for the downstream reset and
  construction paths. Inline `Bus` saves one pointer hop per
  access, but the hop is one load from a cache line `CPU` already
  owns; the savings will not be distinguishable from run-to-run
  variance at the 15-30 % gain band. A future two-line change can
  inline once P1 is stable and its baseline is captured.
- Required Action:
  Add the R-003 one-paragraph rationale to Data Structure.
  No other change to T-1.

### TR-2 `StepContext struct vs plain &mut Bus parameter`

- Related Plan Item: T-3
- Topic: Call-site Ergonomics vs Migration Scope
- Reviewer Position: Prefer Option A (plain `&mut Bus`) for P1
- Advice:
  Adopt as written. Do not introduce a `StepContext` struct in
  P1. The plan's reasoning (minimise diff, preserve 1:1 migration
  shape, avoid disjoint-field-inference complications with a
  multi-field context) is correct.
- Rationale:
  A `StepContext { bus, mmu, privilege }` would collapse
  `mmu.translate(..., bus)` into `ctx.translate(...)`, reading
  better at call sites but adding a new borrow story that would
  need its own validation pass. P1 is not the right place for
  that. Once the bus-threading pattern is stable and Phase-3 perf
  numbers are in, a context-struct refactor becomes a pure
  code-quality PR with well-defined boundaries.
- Required Action:
  Keep as planned. Record in round 03 that a context-struct
  refactor is deliberately deferred, with a pointer into
  `docs/DEV.md` or a phase backlog so it is not lost.

---

## Positive Notes

- Migration table is exhaustive and matches an independent
  `rg "bus\.lock\(\)" xemu -n`. All 24 hits — including the five
  multi-line test-helper sites at `inst/base.rs:344-356`,
  `inst/compressed.rs:552-556`, `inst/float.rs:1075-1079`, and
  `arch/riscv/cpu.rs:277-282` — are present, correctly categorised
  (production vs test helper), and mapped to one-line
  replacements.
- Response Matrix is complete: every 01_REVIEW finding
  (C-1, H-1, H-2, H-3, M-1, M-2, M-3, L-1) plus the M-001
  directive appears with a Decision, a Resolution/Action, and a
  concrete Test-or-gate column. No rejections this round, as
  expected under M-001.
- Gain-band math (G-2) is honest: the plan writes out the bucket
  removal, names the pthread-per-acquire cost (20-40 ns at 2-3
  acq/inst), derives the 15-30 % floor/expected band from that,
  and labels 35 % explicitly as a ceiling, not a forecast. This
  is the discipline the user asked for.
- Trade-offs T-1 cites `docs/DEV.md` Phase 11 explicitly and
  records Phase 11 Option B / Option C as still available. The
  deferred-SMP story is documented without hand-waving.
- Scope discipline is clean: no P2 bus-access API refactor, no
  P4 icache, no P5 MMU inlining smuggled in. C-8 states "Function
  bodies change 1:1" and the migration table backs that up.
- I-9 (no device -> CPU reentry) is reformulated as a borrow-
  checker invariant rather than a runtime `Cell<bool>` debug
  guard — the right reduction now that the mutex is gone.
- The architecture diagram, the exit-gate union, and the
  acceptance-mapping table all match each other; no silent
  divergence between sections.

---

## Approval Conditions

### Must Fix
- (none)

### Should Improve
- R-001 (V-UT-5 `type_name` assertion is a no-op; drop or replace
  with a real compile-time or grep-level sentinel)
- R-002 (document the disjoint-field borrow pattern at I-3 /
  Execution Flow step 2)
- R-003 (justify `Box<Bus>` vs inline `Bus` in Data Structure)
- R-004 (add peer-hart exclusion sentence to I-4)
- R-005 (resolve Phase-1-step-1d intermediate-state gate policy:
  atomic PR or explicit grep-exemption)
- R-006 (name the `linux-2hart` baseline file or add a
  baseline-capture Phase-3 prerequisite)
- R-007 (fix the `cargo asm` symbol path in step 3f)

### Trade-off Responses Required
- T-1 (adopt R-003 rationale; otherwise keep as planned)
- T-3 (keep as planned; record the StepContext deferral)

### Ready for Implementation
- Yes
- Reason: No CRITICAL findings; no HIGH finding blocks the
  migration itself. R-001 is HIGH but concerns a validation
  no-op, not a correctness gap — the hard `rg` gate and the
  compile-time field-type change together already prevent a
  Mutex regression. R-002 through R-007 are clarifications best
  folded into round 03's PLAN or the PR description, but none
  block beginning Phase 1. The migration table is implementable
  as written, the gain band is honest, and M-001 is fully
  applied.
