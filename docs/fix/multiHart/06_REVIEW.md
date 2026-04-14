# `multiHart` REVIEW `06`

> Status: Open
> Feature: `multiHart`
> Iteration: `06`
> Owner: Reviewer
> Target Plan: `06_PLAN.md`
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
- Non-Blocking Issues: 3



## Summary

Round 06 is a clean, well-scoped redesign. The `Rc<RefCell<Bus>>`
pivot eliminates `split_current_mut`, `MachineBuilder`, `CoreBuilder`,
`last_store`, `invalidate_reservations_except`, the `step(&mut self,
&mut Bus)` signature change, and the 8-method `mm.rs` threading that
made round 05 feel heavy. The `Bus::store(HartId, addr, size, val)`
chokepoint is the right shape: physical-address boundary, write +
peer-invalidate under one `borrow_mut`, trivially auditable via the
C-12 grep hook. Reservations living on `Bus` as `Vec<Option<usize>>`
matches L1-D semantics and removes cross-core aliasing hazards.
`CoreOps::step` staying `&mut self` preserves every caller and test
body. The 3-PR split (pivot at N=1 / PLIC runtime-size at N=1 / N>1
activation) keeps each gate byte-identical on the prior mode.

However, the plan has **one compile-time correctness blocker**:
the existing `pub static XCPU: OnceLock<Mutex<CPU<Core>>>` at
`xemu/xcore/src/cpu/mod.rs:50` requires `CPU<Core>: Send` (because
`static` items must be `Sync`, and `OnceLock<Mutex<T>>: Sync`
reduces to `T: Send`). Embedding `Rc<RefCell<Bus>>` in both `CPU`
and `RVCore` makes `CPU<Core>: !Send`, so the crate will not
compile. The plan acknowledges `XCPU: Mutex<CPU>` only as a
**poison** concern in CC-4 and treats `Rc→Arc<Mutex>` as a
future-MT promotion in NG-7, but does not recognize that the
current single-threaded build is already blocked at compile time.
This is R-034 below and is the sole blocker.

The remaining findings are non-blocking: a test-count arithmetic
drift (R-035), a `Bus::store` value-type ambiguity between
`Word`/`u64`/`usize` that matters for SD/SC.D/AMO-D (R-036), and a
minor specification gap around `bus.tick()` being moved into
`CPU::step` while `Bus::reset_devices` / `CPU::reset` semantics are
not re-stated for `N>1` (R-037).

Trade-offs are well-framed. T-4 (Rc over Arc) is the trade-off that
breaks the build — see R-034. The plan should either drop Rc in
favor of `Arc<RefCell<Bus>>` + making `Bus: Sync` (impossible
without interior-mutex everywhere) or, more realistically, adopt
`Arc<Mutex<Bus>>` now rather than "one-line MT promotion later".

Once R-034 is resolved, round 07 is ready to implement.



---

## Findings

### R-034 `CPU<Core> with Rc<RefCell<Bus>> is !Send; static XCPU fails to compile`

- Severity: CRITICAL
- Section: API Surface / Architecture / Trade-off T-4
- Type: Correctness
- Problem:
  The plan embeds `Rc<RefCell<Bus>>` into both `RVCore` (G-2) and
  `CPU<C>` (G-1). Rust's `Rc<T>` is `!Send` and `!Sync` by
  construction, so any struct transitively containing an `Rc` is
  also `!Send`. The existing global CPU handle at
  `xemu/xcore/src/cpu/mod.rs:50`
  ```rust
  pub static XCPU: OnceLock<Mutex<CPU<Core>>> = OnceLock::new();
  ```
  is a `static` item and therefore requires `OnceLock<Mutex<CPU<Core>>>: Sync`.
  By the standard library bounds:
  - `OnceLock<T>: Sync` iff `T: Send + Sync`.
  - `Mutex<T>: Send + Sync` iff `T: Send`.
  - Therefore `XCPU: Sync` reduces to `CPU<Core>: Send`.
  Post-pivot, `CPU<Core>` contains `Rc<RefCell<Bus>>`, so
  `CPU<Core>: !Send` and the crate fails to compile. This is a
  hard compile-time error, not a runtime concern.

  NG-7 / T-4 treat `Rc → Arc<Mutex>` as a future multi-threading
  migration, and CC-4 lists `XCPU: Mutex<CPU>` only as a "poison"
  issue. Neither captures that the `Send` bound on `XCPU` is
  mandatory today.

- Why it matters:
  PR1 step 4/8 cannot land as written. The build breaks before
  any test runs. Every downstream gate (make linux, make debian,
  difftest) is unreachable until this is fixed.

- Recommendation:
  In PR1, replace `Rc<RefCell<Bus>>` with `Arc<Mutex<Bus>>` (or
  `Arc<parking_lot::Mutex<Bus>>` if the no-new-deps rule C-6 is
  relaxed; otherwise `std::sync::Mutex` is sufficient). Atomic
  refcounts under NG-2 are ~2 ns per `.clone()` on modern x86/ARM
  and only happen at core construction; borrow-site cost is one
  uncontended mutex acquisition per hart step. Update:
  - G-1, G-2, G-3: `Rc<RefCell<Bus>>` → `Arc<Mutex<Bus>>`.
  - API Surface: `Ref<'_, Bus>` / `RefMut<'_, Bus>` → `MutexGuard<'_, Bus>`.
    `CPU::bus()` / `CPU::bus_mut()` return `MutexGuard<'_, Bus>`
    (single flavor — `Mutex` has no read/write split).
  - Architecture `CPU::step`: `self.bus.borrow_mut().tick()` →
    `self.bus.lock().unwrap().tick()` (or `.expect("bus lock")`).
  - Store paths: `.borrow_mut().store(...)` → `.lock().unwrap().store(...)`.
  - CC-7 / NG-7 / T-4 rewritten: the shape is now MT-ready today;
    NG-7 "one-line promotion" claim is obsolete.
  - F-8 in Failure Flow: `borrow_mut panic` → `lock poison` (same
    diagnosis: a previous panic while holding the guard).
  - C-6 revisited: `std::sync::Mutex` and `Arc` are both in `std`,
    so no new crate deps.

  Alternatives considered and rejected:
  - `Arc<RefCell<Bus>>`: `RefCell: !Sync`, so `Arc<RefCell<Bus>>: !Send`
    (because `Arc<T>: Send` requires `T: Send + Sync`, and while
    `RefCell<Bus>` is `Send` it is not `Sync`). Does not compile either.
  - Removing `Mutex` from `XCPU` (use `RefCell` or `UnsafeCell`):
    violates `static: Sync` fundamentally and introduces
    unsoundness; rejected.
  - Adding `unsafe impl Send for CPU<Core>`: unsound — it would let
    `Rc` cross threads via future code paths. Rejected.

  The `Arc<Mutex<Bus>>` choice also dissolves CC-4's poison
  concern surface (no change — poisoning was already a Mutex
  property of XCPU), and cleanly supersedes NG-7.



### R-035 `PR1 test-count arithmetic inconsistent with baseline`

- Severity: LOW
- Section: Implementation Plan / Validation
- Type: Validation
- Problem:
  The task briefing declares the current baseline as 354 lib + 1
  arch_isolation + 6 xdb = 361. The plan declares PR1 gate as
  "369 tests (362 lib + 1 + 6)" with "(10 new)" unit tests listed
  (V-UT-1..V-UT-7 + V-UT-10/11/12). 354 + 10 = 364 lib, not 362.
  PR2a (+1 V-UT-8) correctly yields 363; PR2b (+3 V-IT) correctly
  yields 366. But the PR1 anchor is off by 2.

  Possible reconciliations: (a) 2 existing `RVCore` tests are
  retired when rebased under `HartId(0)` (plan step "Existing
  `RVCore::new` / `reset` rebased" could be interpreted as
  merging/removing duplicates); (b) the briefing's 354 baseline
  is stale; (c) some V-UT-X's are net-zero (e.g. reshaping an
  existing test). The plan does not say.

- Why it matters:
  Test-count gates are the gating signal for each PR's
  byte-identical claim. A 2-test drift between "new tests" list
  and "gate total" makes the PR1 regression gate non-auditable —
  the executor cannot tell whether a failure is a real regression
  or a bookkeeping artifact.

- Recommendation:
  Add a one-line reconciliation row in Phase 1 gate: either
  (i) name the 2 existing tests that are removed/replaced by the
  new ones (e.g. "V-UT-6 supersedes `cpu_step_baseline`"), or
  (ii) restate the baseline ("354 → 352 after deleting X, Y in
  step N"), or (iii) correct the target to 364 lib.



### R-036 `Bus::store value type is unspecified; SD / SC.D / AMO-D / FSD need 64-bit path`

- Severity: MEDIUM
- Section: API Surface / Execution Flow
- Type: API
- Problem:
  The plan declares:
  ```rust
  pub fn store(&mut self, hart: HartId, addr: usize,
               size: usize, val: Word) -> XResult;
  ```
  `Word` is `u32` under `cfg(isa32)` and `u64` under `cfg(isa64)`
  (per `src/config.rs`). Under `isa64`, `Word = u64` and this
  signature covers all RISC-V integer store widths (sb/sh/sw/sd).
  Under `isa32`, `Word = u32` and `size == 8` would truncate.
  Additionally, FSD (f64 store) in `fstore_op` today routes a
  64-bit payload; the plan's Execution Flow says "`fstore_op` ...
  commit → `bus.borrow_mut().store(id, addr, size, val)?`" but
  does not specify the value type. Finally, 18 AMO double-word
  variants (`amoadd.d`, `amoswap.d`, etc.) move 64-bit payloads.

  Current `Bus::write` signature is `fn write(&mut self, addr,
  size, value: Word)`, so on isa64 this is fine — but the plan
  needs to explicitly state that (i) multiHart PR1 is isa64-only
  under RISC-V, or (ii) the value type is broadened. Looking at
  the codebase, the whole multiHart scope already assumes
  `cfg(riscv)` ⇒ `cfg(isa64)` (per boot and ACLINT mtime width),
  so option (i) is the natural statement.

- Why it matters:
  Without a stated width assumption, a reader cannot verify that
  `store_op`, `fstore_op`, `sc_d`, and all 9 `amo*.d` callers
  produce a `Word`-typed `val` that fits the physical store.
  Silent narrowing or type-mismatch compile errors could surface
  during PR1 implementation and slow execution.

- Recommendation:
  Add an explicit constraint (e.g. **C-13 multiHart PR1 is
  `cfg(isa64)`-only; `Word = u64` carries all store widths
  including FSD/AMO.D**) and a one-liner in API Surface clarifying
  that FSD stores go through `store` with the f64 bit-pattern
  reinterpreted as `Word` (matching the current `fstore_op`
  behavior). Also note that `sc_w` / `sc_d` branch return values
  (0 on success, 1 on failure) are CPU-GPR writes, not bus stores,
  so they do not flow through `Bus::store`.



### R-037 `bus.tick() move + N>1 CPU::reset semantics are under-specified`

- Severity: LOW
- Section: Execution Flow / State Transition
- Type: Flow
- Problem:
  Two small gaps in the CPU lifecycle under `N>1`:

  (a) **`bus.tick()` migration**: The plan says "`bus.tick()`
  moves from `mod.rs:225` to `CPU::step`". Today, `bus.tick()`
  runs once per `RVCore::step` call, so a 2-hart round-robin
  would today tick the bus twice per wall-clock step. After the
  move, `CPU::step` ticks the bus once, then runs
  `cores[current].step()`. Over N harts that means bus ticks at
  the single-hart rate even with N>1. For MTIMER accuracy, this
  is what we want (the shared timer ticks once per scheduler
  tick, not per hart); but it is not stated as an invariant, and
  I-3/I-5 don't capture it. Under `SLOW_TICK_DIVISOR = 64`, this
  is byte-identical to today at `N=1` but changes timer progress
  per cycle at `N>1`. Make it explicit.

  (b) **`CPU::reset` for N>1**: The plan mentions `CPU::reset`
  clears `bus.reservations[..]` (PR2b step 13), but does not
  restate the order of operations — specifically, whether
  `self.bus.borrow_mut().reset_devices()` runs once (shared bus)
  and whether `cores[i].reset()` iterates all harts. A reader
  would have to reconstruct this from S2→S3 alone. At `N=1` it's
  trivial; at `N>1` a wrong order (e.g. resetting cores before
  clearing reservations) could leave stale per-hart state.

- Why it matters:
  Both gaps are non-blocking, but the executor has to reconstruct
  intent from context. With R-034 fixed, PR1 will touch
  `CPU::step` and `CPU::reset` together; an explicit statement
  prevents a revision in PR2b.

- Recommendation:
  - Add **I-12 `Bus::tick` runs once per `CPU::step`, not once
    per core step; under N>1 timer progress is decoupled from
    hart count.** (or fold into I-3.)
  - In Execution Flow PR2b step 13, spell out the reset order:
    `bus.reset_devices(); bus.reservations.fill(None);
    for core in &mut self.cores { core.reset()?; }`.



---

## Trade-off Advice

### TR-1 `Rc vs Arc — the Send bound forces the choice today`

- Related Plan Item: T-4
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option B (Arc + Mutex)
- Advice:
  Drop the `Rc<RefCell<Bus>>` shape. Adopt `Arc<Mutex<Bus>>`
  directly in PR1. See R-034 for the compile-time reasoning.
- Rationale:
  T-4's stated benefit ("skips atomic bumps; one-line MT promo")
  is marginal in absolute terms — `Arc::clone` is one `fetch_add`
  per core construction (N≤16 times at startup) and
  `Mutex::lock` is one `compare_exchange` per hart step.
  Relative to the ~100ns cost of one emulated RISC-V instruction
  dispatch, this is below noise. The "one-line MT promo" framing
  is also misleading: the shape must be `Arc<Mutex<_>>` today to
  satisfy `static Sync`, so there is no future promotion — the
  shape is already whatever it's going to be. The only real
  decision is `std::sync::Mutex` (in tree, aligns with C-6) vs
  `parking_lot::Mutex` (faster, new dep). Pick `std::sync::Mutex`.
- Required Action:
  Executor adopts `Arc<Mutex<Bus>>` in round 07 PLAN. Update T-4
  framing and CC-4 / CC-7 / CC-10 / NG-7 accordingly. Drop the
  "future-MT one-line promo" from the architecture narrative —
  the shape is already MT-ready.



### TR-2 `Reservations on Bus vs RVCore`

- Related Plan Item: T-2
- Topic: Cohesion vs Locality
- Reviewer Position: Prefer Option A (Bus) — as plan
- Advice:
  Keep reservations on `Bus`. Do not revert to `RVCore`-owned
  reservations even under pressure to minimize PR1 churn.
- Rationale:
  Cross-hart invalidate is the defining correctness property of
  LR/SC across harts. Hosting reservations on `Bus` puts the
  write-side peer-invalidate on the same object that owns the
  physical-store primitive, so the atomicity of "write + peer
  invalidate" is a single `lock().unwrap()` span. On `RVCore`,
  cross-hart invalidate would require either the eliminated
  `invalidate_reservations_except` helper or another shared
  handle — either way reintroduces complexity the redesign is
  explicitly removing.
- Required Action:
  Keep as is.



### TR-3 `Bus::store chokepoint vs mm-layer hook`

- Related Plan Item: T-3
- Topic: Layering
- Reviewer Position: Prefer Option A (Bus::store) — as plan
- Advice:
  Keep `Bus::store` at the physical-address boundary.
- Rationale:
  An `mm.rs` hook would sit above page-walk and PMP, making it
  the wrong layer to attach a physical-address-keyed reservation
  invalidation. `Bus::store`'s invariant — "every physical store
  invalidates overlapping peer reservations before returning" —
  is exactly the HW cache-coherent store semantics LR/SC models.
  The C-12 grep audit (no raw `Bus::write` from
  `inst/{atomic,base,float}.rs`) gives mechanical coverage.
- Required Action:
  Keep as is. Strengthen C-12 to also forbid `Bus::write` calls
  from any `arch/*/cpu/mm*.rs` file after the pivot — mm-layer
  callers must go through `Bus::store` with a `HartId`.



### TR-4 `CoreOps::step unchanged vs step(&mut self, &mut Bus)`

- Related Plan Item: T-5
- Topic: Compatibility vs Explicitness
- Reviewer Position: Prefer Option A (unchanged) — as plan
- Advice:
  Keep `fn step(&mut self) -> XResult`. Do not reopen this.
- Rationale:
  The round 05 signature change rippled into 8 `mm.rs` methods,
  `CoreOps::step`, every difftest call site, and 14 xdb test
  harness references. The `Arc<Mutex<Bus>>` pivot dissolves the
  borrow-checker argument for the signature change: each step
  body acquires the lock on demand, which is exactly the
  current `self.bus.xxx()` idiom shifted by four characters.
- Required Action:
  Keep as is. In R-034's rewrite, `self.bus.xxx()` becomes
  `self.bus.lock().unwrap().xxx()` — same pattern, still in-method.



---

## Positive Notes

- The shape-level elimination list (`split_current_mut`,
  `MachineBuilder`, `CoreBuilder`, `last_store`,
  `invalidate_reservations_except`, `step(&mut self, &mut Bus)`,
  8-method `mm.rs` threading, `setup_core_and_bus`,
  `CoreOps::{bus, bus_mut}`) is exactly the cleanup the user asked
  for. Post-fix of R-034, the plan will be materially cleaner than
  round 05.
- `Bus::store` as the physical-store chokepoint with write +
  peer-invalidate under one guard is the correct shape for
  LR/SC semantics. I-8 + C-12 grep audit is mechanical and
  reviewable.
- Reservations as `Vec<Option<usize>>` indexed by `HartId.0`
  (with 8-byte granule) correctly matches the RVWMO LR/SC model
  and the RISC-V unprivileged spec's reservation-set wording.
- PLIC runtime-sizing (`num_ctx = 2*num_harts`, `evaluate`
  iterating `0..num_ctx`) preserves the 14 existing tests as
  zero-regression gates under `Plic::new(1, vec![irq.clone()])`.
- 3-PR split with byte-identical gates at `N=1` on the prior
  mode is exactly the right cadence for an emulator refactor of
  this size.
- Plan body at 399 lines is within C-7 ≤400 and 45% shorter than
  round 05's 719 lines.
- Concurrency matrix (CC-1..CC-10) is compact but preserves the
  per-row reasoning from round 05. Good compression.
- HartId defined at `cpu/core.rs` as a plain newtype (no arch
  imports) keeps the arch-isolation seam with a 2-token allow-list
  widening — mechanically verifiable.



---

## Approval Conditions

### Must Fix
- R-034 (CRITICAL — compile-time blocker; `Rc<RefCell<Bus>>` →
  `Arc<Mutex<Bus>>`)

### Should Improve
- R-035 (LOW — reconcile PR1 test-count arithmetic)
- R-036 (MEDIUM — state `Bus::store` value-type / isa64-only
  constraint; clarify FSD / AMO.D payload routing)
- R-037 (LOW — explicit `bus.tick()` cadence invariant + N>1
  reset ordering)

### Trade-off Responses Required
- T-4 (per TR-1 — adopt Arc<Mutex<Bus>>; retire "future one-line
  MT promo" framing)
- T-3 (per TR-3 — extend C-12 grep audit to `mm*.rs` files)

### Ready for Implementation
- No
- Reason: R-034 is a compile-time correctness blocker. The crate
  will not build as written because `pub static XCPU:
  OnceLock<Mutex<CPU<Core>>>` requires `CPU<Core>: Send`, and
  `Rc<RefCell<Bus>>` embedded in `CPU` makes it `!Send`. Round
  07 must pivot to `Arc<Mutex<Bus>>` (still no new deps under
  C-6). Once R-034 is addressed and R-036/R-037 are clarified,
  the plan is ready.
