# `perfBusFastPath` REVIEW `00`

> Status: Open
> Feature: `perfBusFastPath`
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
- Blocking Issues: 2 (1 HIGH, 1 HIGH — no CRITICALs)
- Non-Blocking Issues: 7 (4 MEDIUM, 3 LOW)

The plan is structurally sound, its single-hart `Owned` path genuinely
eliminates the mutex on the hot path, and the enum-based design is
consistent with the PERF_DEV.md P1 description, surveyed Rust RISC-V
emulators, and QEMU's BQL-push-down history. However, the call-site
migration table is **incomplete** (at least four hot sites in
`inst/base.rs`, `inst/compressed.rs`, `inst/float.rs`, and
`cpu/debug.rs` are missing and one — `inst/base.rs:75` — is on every
regular store, i.e. firmly on the hot path), and one cited file path
(`arch/riscv/cpu/privileged.rs`) does not exist in the repository.
These are HIGH because they directly affect whether the next
implementation round will actually hit exit-gate G-1 ("zero
`pthread_mutex_*` on the hot path"). Nothing here is structurally
unsound — the fixes are mechanical — so the plan is approved once
those gaps land in `01_PLAN`.

## Summary

The plan proposes a `BusHandle { Owned(Box<Bus>), Shared(Arc<Mutex<Bus>>) }`
enum plus a `BusGuard` scope guard, selected at construction time
from `cores.len()`. For `cores.len() == 1` the `Owned` arm is a
direct `&mut Bus` borrow with no mutex operation; for `cores.len() >= 2`
it preserves the existing `Arc<Mutex<Bus>>` shape byte-for-byte. The
spec alignment with `docs/PERF_DEV.md` §3 P1 is exact, the
non-benchmark-targeted structural nature of the fix is well
argued (C-3), the refusal of `UnsafeCell` (NG-4, T-1 option 2) is
the right call, and the trade-off matrix (T-1..T-4) is honest about
the split-lock regression risk (T-4 — calls out that MTTCG will have
to split them back out, which matches reviewer concern in the brief).

The gain math is defensible but stretched at the 30 % ceiling:
mutex self-time is 33–40 % and the `Owned` path removes *all* of it,
but `access_bus` (7.7–8.7 %) and `checked_read` (7.4–7.7 %) still
run per-access; their lock cost folds into the mutex bucket, their
dispatch cost does not. The plan correctly keeps a 15 % floor (G-2)
and leans on the 20–30 % expected band as the *ceiling* of what P1
alone delivers — this is consistent with PERF_DEV.md §6's "count
the larger of P1/P2, not both" rule.

Biggest concrete risks are (a) the migration table misses hot sites
that would leave residual mutex traffic on every regular store and
every `c.sd/c.lwsp`-style compressed access, and (b) the plan does
not commit to a regression test that *asserts* the multi-hart critical
section hasn't widened — it relies on "`linux-2hart` within ±5 %" which
is a correctness-by-timing check, not a behavioral one. Both should
be addressed in `01_PLAN`.

---

## Findings

### R-001 Migration table misses hot-path call sites

- Severity: HIGH
- Section: `Implement / Phase 2 — Migration table`
- Type: Correctness / Validation
- Problem:
  The migration table (lines 393–412) covers `cpu/mod.rs`,
  `arch/riscv/cpu/mm.rs`, `arch/riscv/cpu/inst/atomic.rs`, and
  `arch/riscv/cpu.rs`, but a repo-wide grep for `bus.lock()` in
  `xemu/xcore/src/` turns up **four additional sites the plan does
  not name**:
  - `arch/riscv/cpu/inst/base.rs:75` — `clear_reservation` on every
    regular (non-AMO) store. This is firmly on the hot path; every
    `sw`/`sd`/`sh`/`sb` touches it.
  - `arch/riscv/cpu/inst/compressed.rs:573` — bus read inside
    compressed-form handling (test helper, but same pattern).
  - `arch/riscv/cpu/inst/float.rs:1085` — bus read inside F/D
    handling (test helper; still part of the `self.bus.lock()` blast
    radius).
  - `arch/riscv/cpu/debug.rs:103–109` — `fetch_inst` holds a lock
    across two reads (16-bit half then, if needed, the upper half
    of a 32-bit instruction). This is the debug path but uses the
    same field, so the field type change forces the migration.
  Additionally, the plan references `arch/riscv/cpu/privileged.rs`
  (lines 39, 409); that file does not exist. The real file is
  `arch/riscv/cpu/inst/privileged.rs`, and a grep of it finds **zero**
  `bus.lock()` sites — the plan's claim "WFI / IRQ view → with" has
  no actual call site to migrate. Either the plan means a different
  file (trap handling lives in `cpu/trap/handler.rs`) or the row is
  vacuous; either way it's misleading.
- Why it matters:
  The `bus: BusHandle` field type change is compile-breaking: *every*
  `self.bus.lock().unwrap()` call site must move to `with` /
  `with_guard` or the crate does not compile. So the missing sites
  will be caught by the compiler — but they won't be caught by the
  *plan*, and the reviewer cannot check ahead of time that the
  executor has reasoned about each one. More importantly,
  `inst/base.rs:75` is per-store: leaving it as a naive `with(|b|
  b.clear_reservation(id))` is correct but wastes a discriminant
  check per store; consolidating it into the same `with_guard` that
  `checked_write` already opens would eliminate a redundant
  branch-and-call on the hottest store path. The plan should be
  explicit about whether it merges or not.
- Recommendation:
  In `01_PLAN`, expand the migration table to include every
  `bus.lock()` site in `xemu/xcore/src/` (use `grep -rn 'bus\.lock('
  xemu/xcore/src`). For each, state the migration pattern (`with` vs
  `with_guard`). Remove or correct the `privileged.rs` row. For
  `inst/base.rs:75` specifically, say whether the clear_reservation
  fuses into the write's `with_guard` or not — and if not, why.



### R-002 No behavioral regression test for multi-hart critical-section width

- Severity: HIGH
- Section: `Validation / Acceptance Mapping / C-8`
- Type: Validation
- Problem:
  C-8 states "no widened Shared critical section" and maps that to
  "V-IT-4 timing within ±5 percent proves no observable widening."
  Timing on a boot-to-prompt workload is a very weak signal for
  lock-width regressions: 2-hart Linux spends most of its wall-clock
  on guest kernel code where residual contention is masked by guest
  scheduling jitter; a 2-3× widening of a rarely-contended SC scope
  would not move boot time by 5 %. T-4 acknowledges that merging
  `checked_read`/`checked_write`'s two locks into one is a *shape*
  change that MTTCG must undo, but offers no behavioral check for it
  now.
- Why it matters:
  The non-negotiable constraint from the user brief is "multi-hart
  correctness preserved; realistic gain only." Without a behavioral
  regression test the `Shared` arm could silently widen critical
  sections (e.g. a future reviewer re-orders a translate between two
  accesses and holds the guard longer than `access_bus` does today)
  and the only signal is "linux-2hart boot slower". That is a
  correctness-by-timing check, which the plan's own V-E-3 ("hart 0
  lr, hart 1 store, hart 0 sc fails") doesn't cover either — V-E-3
  verifies SC *semantics*, not critical-section width.
- Recommendation:
  Add one of: (a) a `#[cfg(test)]` instrumentation counter on the
  `Shared` arm that records how many distinct `MutexGuard` lifetimes
  are opened per `CPU::step`, with a test asserting the count
  matches the pre-P1 baseline on a fixed 64-instruction golden
  trace; or (b) a micro-bench in `xcore/benches/` (matches PERF_DEV
  §4.2 scaffolding) that measures 2-hart lock acquisition count and
  fails on increase. Either is a real behavioral gate; the current
  plan has none.



### R-003 Headline gain math relies on "mutex bucket → 0" without accounting for LLVM fallout

- Severity: MEDIUM
- Section: `Spec / Goals (G-2)` and `Trade-offs (T-1)`
- Type: Spec Alignment / Correctness of claim
- Problem:
  G-2 claims "at least 15 %; expected 20 to 30 %". The mutex bucket
  is 33.4–39.9 %, so a naive "remove 35 %" arithmetic gives
  ~35 % wall-clock — which would overshoot the stated 30 % ceiling.
  The real picture is more subtle: once the mutex goes, samples
  redistribute into the functions that were *blocked on it*
  (`access_bus`, `checked_read`, `xdb::main`), so their percentage
  shares rise while total wall-clock drops. The plan does not walk
  this through. T-1's "branch predictor collapses" claim is also
  qualitative: in the `Owned` arm the variant is set once at
  construction and never changes, which is a constant-value load —
  LLVM should fold the match in practice, but only if the match is
  in a place where inlining reaches it. The plan does not commit to
  verifying this with `cargo asm` or a microbench.
- Why it matters:
  If the real number lands at 12–18 % (believable — per-lock cost
  on uncontended macOS pthread is ~15–40 ns, and ~2–3 acquisitions
  per guest instruction at tens of M instrs/s is ~1–3 s of a 9 s
  dhrystone run, i.e. 10–30 % ceiling), the exit gate's 15 % floor
  just barely holds and the "expected 20–30 %" band is marketing.
  The plan should either soften the band or commit to verifying the
  match-fold with a disassembly spot-check (one `cargo asm` line,
  cheap).
- Recommendation:
  In `01_PLAN`, either:
  - (a) keep the 15 % floor, reset the expected band to "15–25 %" and
    note the ceiling hinges on LLVM folding the `Owned` match; or
  - (b) add a Phase 3 step that runs `cargo asm
    xcore::cpu::CPU::step` on the `Owned` build and asserts the
    `match BusHandle` is not present in the emitted code. (This is
    a compile-time check, not a runtime one — low-cost.)



### R-004 `BusHandle::with` takes `&mut self` — blocks reentrant bus access from device code

- Severity: MEDIUM
- Section: `API Surface`
- Type: API / Maintainability
- Problem:
  `pub fn with<R>(&mut self, f: impl FnOnce(&mut Bus) -> R) -> R;`
  takes `&mut self`. Inside `f`, if any device needs to go back up
  to the `BusHandle` (e.g. a DMA device that invokes another
  `bus.with` during its tick), the borrow checker will reject it.
  Today this doesn't happen — devices only touch their own state —
  but the plan should declare this as an invariant (e.g. I-9: "no
  `Device::tick` body reaches back into `BusHandle`") and document
  that violating it turns into a compile error, not a runtime one.
  Otherwise a future contributor adds a bus-callback device and
  gets a cryptic borrow-checker message with no reference to why.
- Why it matters:
  Future-maintenance. Not a correctness issue today.
- Recommendation:
  Add I-9 to `Invariants` in `01_PLAN`: "No closure passed to
  `BusHandle::with` / `with_guard` may itself call back into the
  owning `CPU`'s bus." Note that this is enforced by the borrow
  checker.



### R-005 `clone_for_hart` on `Owned` panics — this is fine but not encoded in the type

- Severity: MEDIUM
- Section: `API Surface`
- Type: API
- Problem:
  `clone_for_hart` panics on `Owned` (line 245). The plan justifies
  this as "caller bug". The safer shape is to make
  `clone_for_hart` a method only on `Shared`, or to return
  `Result<BusHandle, AlreadyOwned>`. A runtime panic for a
  compile-time-knowable misuse is not Rust-idiomatic (coding-style
  rule: never use `.unwrap()` for things that aren't structurally
  impossible).
- Why it matters:
  Small API hygiene; affects maintainability, not correctness.
  Matches rules/rust/coding-style.md §Error Handling.
- Recommendation:
  In `01_PLAN`, either (a) split the factory into
  `BusHandle::new_owned(bus) -> Self` and
  `BusHandle::new_shared(bus) -> SharedBusHandle` with
  `clone_for_hart` only on the shared form, or (b) keep the enum
  but make `clone_for_hart` return `Result<Self, OwnershipError>`
  and `.expect("Owned bus handle cannot be shared across harts —
  this is a construction-order bug")` at the one known call site in
  `CPU::with_machine_config`.



### R-006 Reset path should keep current lock-count shape

- Severity: MEDIUM
- Section: `Execution Flow / State Transition`
- Type: Correctness / Flow
- Problem:
  The plan (lines 372–374, table row `cpu/mod.rs:142–145 reset`)
  says reset uses a single `with_guard` scope covering both
  `reset_devices` and `clear_reservations`. Today (see
  `xemu/xcore/src/cpu/mod.rs:141–145`), this is *already* a single
  lock scope — the `{ let mut bus = self.bus.lock().unwrap(); ...
  }` block covers both calls. So the plan is describing a no-op
  for the `Shared` arm, which is correct, but the migration table
  row is phrased as if it were a change. Not wrong, but easy for
  the executor to misread as "merge these two locks" when they are
  already merged.
- Why it matters:
  Reduces risk of a spurious behavioral change while landing the
  refactor.
- Recommendation:
  Clarify the row: "already a single scope today; maps 1:1 to
  `with_guard`."



### R-007 `CPU::bus()` signature change is a breaking change — audit external callers explicitly

- Severity: MEDIUM
- Section: `API Surface`
- Type: API
- Problem:
  Plan line 279–283 changes `CPU::bus()` from `&self -> MutexGuard<'_,
  Bus>` to `&mut self -> BusGuard<'_>`. The plan says "the one
  difftest caller accordingly" — but `CPU::bus()` is a `pub` method
  and the plan does not enumerate *who* calls it. A `grep` of
  `xdb`, `xtool`, `xkernels`, and integration tests would surface
  the full blast radius.
- Why it matters:
  G-5 says "public APIs stable ... the `BusHandle` refactor is
  internal." Changing `&self` to `&mut self` on a public method is
  an observable API break. Either G-5 is not fully met (and the
  plan should say so) or the change has to be structured
  differently (keep `&self` + `&Bus` readonly via `with_ref`).
- Recommendation:
  In `01_PLAN`, list every `CPU::bus()` caller outside `xcore`.
  Either reconcile with G-5 (soften G-5 to "`xcore`-internal only")
  or split the API into `bus()` (read-only, via `with_ref`) and
  `bus_mut()` (mutable, via `with_guard`), keeping the read shape
  source-compatible.



### R-008 Missing reference to where the "rvemu / riscv-rust / Rare / rrs" claim comes from

- Severity: LOW
- Section: `Trade-offs / T-1` and `Summary`
- Type: Spec Alignment / Evidence
- Problem:
  The plan asserts (lines 457–462) that four Rust RISC-V emulators
  adopt "the owned-bus shape." No repo URLs, tags, or file
  references are given. QEMU's BQL push-down is cited by name
  ("LWN 697031/697265, QEMU multi-thread-tcg docs") which is
  verifiable, but the Rust survey is not.
- Why it matters:
  AGENTS.md §Development Standards: "Always use web search to
  retrieve the latest official documentation." The plan's
  structural-fix justification leans partly on this survey; a
  reviewer cannot double-check it without redoing the search.
- Recommendation:
  In `01_PLAN`, add one footnote per named project: repo URL and
  the specific file where the bus is owned (e.g. `rvemu/src/bus.rs`
  holding `Bus` by value in `Emulator`). One link each; ~3 lines
  added.



### R-009 No microbench in P1 scope, but PERF_DEV.md P1 gate needs one to be credible

- Severity: LOW
- Section: `Validation`
- Type: Validation
- Problem:
  PERF_DEV.md §4.2 plans `xcore/benches/` criterion micro-benches
  as part of P1's measurement infrastructure. Plan 00 defers this
  entirely to the `sample.sh` / `bench.sh` pipeline — which is the
  right thing for the *exit gate*, but a microbench would give a
  cheap, low-variance number for the mutex-removal claim
  specifically.
- Why it matters:
  Not blocking. Just notes that a 10-line `criterion` bench on
  `step_1m_nops` would cost half a day and give a reviewer
  much stronger confidence in the headline number than
  `sample.sh` alone. Missing it is defensible — the plan is about
  shipping P1, not about building the benchmarking infra.
- Recommendation:
  Optional. Either add one `criterion` bench in Phase 3, or
  defer explicitly with a note that the infra lands in a later
  phase per PERF_DEV.md §4.2.



---

## Trade-off Advice

### TR-1 Ownership model (enum vs. UnsafeCell vs. trait-object)

- Related Plan Item: `T-1`
- Topic: Flexibility vs Safety
- Reviewer Position: Prefer Option A (enum) — agrees with plan
- Advice:
  Keep Option 1 (the `BusHandle` enum). The plan's rejection of
  Option 2 (`Arc<UnsafeCell<Bus>>`) on safety/CI-matrix grounds
  is correct; the rejection of Option 3 (trait object) on vtable
  cost is correct. The discriminant cost is trivially hoistable by
  LLVM because the variant is set once at construction — the only
  non-trivial case is `#[inline(never)]` boundaries, which xcore
  avoids via `lto = true`. No further justification needed.
- Rationale:
  Safety (C-1, NG-4, coding-style.md), CI simplicity, and the
  surveyed Rust RISC-V emulator precedent all point the same way.
  The perf cost of the discriminant is at worst one compare-and-
  branch-not-taken per access — cheaper than the mutex it replaces
  by ~two orders of magnitude.
- Required Action:
  Keep Option 1. Add the `cargo asm` spot-check from R-003 to
  confirm the match folds away.



### TR-2 `CPU::bus()` return type — uniform BusGuard vs bifurcated

- Related Plan Item: `T-2`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Need More Justification
- Advice:
  See R-007. The current proposal (uniform `BusGuard<'_>`) changes
  `CPU::bus()` from `&self` to `&mut self`, which is an API break
  that G-5 disclaims. The bifurcated option (Option B: `&mut Bus`
  in `Owned`, `MutexGuard` in `Shared`) is worse for maintainers.
  The right answer is option C (not in the plan): keep `bus()` as
  `&self -> ReadBusGuard` (routed through `with_ref`) and add
  `bus_mut(&mut self) -> BusGuard<'_>` for the rare external
  mutator (difftest).
- Rationale:
  G-5 requires stable public APIs. Splitting into read/mut
  mirrors standard Rust practice (`HashMap::get` vs `get_mut`),
  preserves `&self` on the read path, and keeps difftest's
  `bus_take_mmio_flag` path clean. It also surfaces the invariant
  that external callers should almost never need mutable access.
- Required Action:
  Add Option C to T-2, compare against A and B, and commit to one
  in `01_PLAN`. Reviewer recommends C.



### TR-3 `Shared` arm mutex choice — std vs parking_lot

- Related Plan Item: `T-3`
- Topic: Performance vs Compatibility
- Reviewer Position: Prefer Option A (std) — agrees with plan
- Advice:
  Keep `std::sync::Mutex` for P1. `parking_lot` would add a
  dependency and a CI-matrix variant for a bucket (multi-hart
  contention) that is not the P1 target. Defer to Phase P7
  (multi-hart re-profile) per PERF_DEV.md §P7 — that's where the
  data will tell you whether `parking_lot` or `RwLock<Bus>` is
  the right lever. Running the experiment now is speculative.
- Rationale:
  The baseline report does not profile `linux-2hart`; we don't
  know whether `parking_lot` actually helps on our workload.
  P1's gain comes from *removing* the mutex in the single-hart
  arm, not from making the remaining multi-hart arm faster.
- Required Action:
  Keep T-3 as-is. Reviewer confirms.



### TR-4 Merging `checked_read` / `checked_write`'s two locks into one

- Related Plan Item: `T-4`
- Topic: Performance vs Future Flexibility (MTTCG)
- Reviewer Position: Need More Justification
- Advice:
  The plan acknowledges (line 489–491) that this merge will have
  to be split back out when MTTCG lands. That's honest, but the
  plan then does not provide a behavioral check that the merge
  preserves current semantics (see R-002). The merge *is*
  structurally fine on `Shared` — `access_bus` today already
  holds the lock across translate (`mm.rs:258`), so merging the
  translate-then-read pair does not widen beyond existing
  scope. But the plan should prove this with a counter-based test,
  not with "boot-time within 5 %".
- Rationale:
  Future MTTCG split-back is a known cost. Hidden current-day
  widening is the actual risk.
- Required Action:
  Either (a) keep the merge and add the behavioral test from
  R-002; or (b) don't merge — keep two `with` calls in
  `checked_read` / `checked_write`, which makes the `Shared` arm
  byte-identical to main at the cost of one extra discriminant-
  check on the `Owned` arm (negligible). Reviewer weakly prefers
  (b) because it preserves the constraint "multi-hart semantics
  are byte-identical" (G-3) in the strongest sense, leaving the
  lock-merging as a future optimisation once we have the
  microbench scaffolding.



---

## Positive Notes

- Excellent non-negotiable-constraint compliance: C-3 (owned
  selector is `num_harts == 1`, not any guest-binary fingerprint)
  is exactly the "no benchmark-targeted hack" guarantee the user
  brief demands. Every architectural choice is defended in these
  terms.
- Clean invariant set (I-1..I-8). I-4 specifically addresses the
  SC triple-lock concern raised in the user brief and gives a
  concrete code pointer (`mm.rs:258–262`) for why the merge does
  not widen the critical section. That is review-friendly.
- Honest gain ceiling: the plan does not promise > 30 %, lands the
  floor at 15 %, and maps both to PERF_DEV.md §6's additive-vs-
  multiplicative rules. The 15 % floor is a genuine gate, not a
  rubber-stamp.
- T-1 trade-off is well-framed: three options, each with a measurable
  cost, each with a principled rejection. The QEMU BQL-push-down
  parallel is the right historical lens.
- NG-4 (no `unsafe`) and NG-2 (no benchmark-aware branch pruning)
  are both codified as constraints, not vague preferences. This is
  the standard we want at this stage of the roadmap.
- Exit-gate measurability (Phase 3) correctly commits to the
  `scripts/perf/` pipeline with DEBUG=n and `make run` as the
  launch vector, per project feedback memory
  (`feedback_debug_flag`, `feedback_use_make_run`).

---

## Approval Conditions

### Must Fix
- R-001 (Complete the migration table; correct or remove the
  `privileged.rs` row; enumerate every `bus.lock()` site.)
- R-002 (Add a behavioral regression test — counter-based or
  microbench — for `Shared`-arm critical-section width.)

### Should Improve
- R-003 (Either soften the 20–30 % band to 15–25 % or add a
  `cargo asm` spot-check that the `Owned` match folds away.)
- R-004 (Add I-9 declaring no reentrant bus access from device
  closures.)
- R-005 (Reshape `clone_for_hart` to avoid a runtime panic for a
  compile-time-knowable misuse.)
- R-006 (Clarify the reset-path migration row — it is already a
  single scope today.)
- R-007 (Enumerate `CPU::bus()` external callers; either soften
  G-5 or split into `bus()` / `bus_mut()`.)

### Trade-off Responses Required
- T-1 — adopt as-is. Add `cargo asm` spot-check per R-003.
- T-2 — expand with Option C (`bus()` / `bus_mut()` split) and
  commit.
- T-3 — adopt as-is.
- T-4 — either add the behavioral test (R-002) or revert the
  merge and keep two `with` calls; reviewer weakly prefers the
  latter for G-3 strictness.

### Ready for Implementation
- No
- Reason: Two HIGH findings remain (R-001 migration-table
  completeness; R-002 no behavioral test for multi-hart
  critical-section width). Both are mechanical to resolve in
  `01_PLAN`. Once those are addressed and the four trade-off
  responses are recorded, the plan is ready.
