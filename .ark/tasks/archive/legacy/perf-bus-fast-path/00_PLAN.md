# `perfBusFastPath` PLAN `00`

> Status: Draft
> Feature: `perfBusFastPath`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`
> - Master Directive: `none`

---

## Summary

Phase P1 of the xemu perf roadmap. Eliminate the `Arc<Mutex<Bus>>` lock
overhead that dominates self-time on dhrystone / coremark / microbench
(roughly 33 to 40 percent of cycles spent in
`pthread_mutex_{lock,unlock,trylock}` per `docs/perf/2026-04-14/REPORT.md`)
when `cores.len() == 1`, while keeping the existing shared-bus path
bit-for-bit identical for multi-hart configurations (`make linux-2hart`).
The fix is structural, not benchmark-targeted: replace the
`Arc<Mutex<Bus>>` field on `RVCore` / `CPU` with a `BusHandle` enum
whose `Owned` arm stores `Box<Bus>` and whose `Shared` arm keeps the
current `Arc<Mutex<Bus>>`. All bus access goes through a uniform
`with` / `with_guard` API so LR/SC and translate-then-access batches
stay atomic. Expected wall-clock gain 20 to 30 percent; exit-gate floor
15 percent to reject inflation. This mirrors the owned-bus design used
by every surveyed Rust RISC-V emulator (rvemu, riscv-rust, Rare, rrs)
and QEMU's incremental BQL push-down.

## Log {None in 00_PLAN}

[**Feature Introduce**]

First iteration. Introduces the `BusHandle` / `BusGuard` abstraction,
the single-hart ownership fast path, and the call-site migration audit
for 15+ bus-access sites across `cpu/mm.rs`, `cpu/mod.rs`,
`arch/riscv/cpu/inst/atomic.rs`, `arch/riscv/cpu/privileged.rs`, and
`arch/riscv/cpu.rs`.



[**Review Adjustments**]

N/A — first round.



[**Master Compliance**]

N/A — first round.



### Changes from Previous Round

[**Added**]
N/A — first round.



[**Changed**]
N/A — first round.



[**Removed**]
N/A — first round.



[**Unresolved**]
- U-1: How `BusHandle::Shared` selects its locking primitive
  (`std::sync::Mutex` vs `parking_lot::Mutex`). Kept as an open
  trade-off (T-3); single-hart path bypasses it entirely so this only
  affects `linux-2hart` and future N-hart MTTCG work.
- U-2: Whether `CPU::bus()` should return a concrete `&mut Bus` in the
  `Owned` arm (tighter borrow, zero overhead) or keep exposing a
  guard-like wrapper uniformly (cleaner call sites). See T-2.
- U-3: Long-term fate of the `Shared` arm once a per-hart TLB / MTTCG
  phase lands. Not in scope for P1, flagged here so design keeps it
  replaceable.



### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| — | — | N/A (first round) | No prior findings or directives to respond to. |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here.
> - Every Master directive must appear here.
> - Rejections must include explicit reasoning.

---

## Spec {Core specification}

[**Goals**]

- G-1: On the single-hart configuration, `CPU::step` and every
  per-instruction memory access (`checked_read`, `checked_write`,
  `access_bus`, AMO / LR / SC, `Bus::tick`) execute with **zero**
  `pthread_mutex_*` calls on the hot path.
- G-2: Wall-clock runtime of `make run` (dhrystone, coremark,
  microbench, DEBUG=n) drops by at least 15 percent vs.
  `docs/perf/2026-04-14/data/bench.csv`; expected 20 to 30 percent.
- G-3: Multi-hart semantics are byte-identical to main on the
  `linux-2hart` boot path (tick ordering, reservation visibility,
  direct IRQ signaling, MTIMER `mtime` monotonicity all preserved).
- G-4: LR/SC atomicity across translate → reservation-read →
  conditional-store is preserved without widening any existing
  critical section.
- G-5: Public `cpu::with_xcpu` / difftest / integration-test APIs
  keep their current shapes; the `BusHandle` refactor is internal.

- NG-1: Not a JIT, not a TLB, not a softmmu fast path — those are
  future phases per `docs/perf/PERF_DEV.md`.
- NG-2: No per-instruction benchmark-aware branch pruning, no
  guest-PC specialization, no skipping of UART / PLIC / MTIMER ticks.
- NG-3: No change to device trait (`Device::tick`, `Mmio::read/write`,
  `Device::reset`) semantics or ordering.
- NG-4: No `unsafe` to bypass the borrow checker. Option 2
  (`Arc<UnsafeCell<Bus>>`) is explicitly rejected (see T-1).
- NG-5: No new public crate-external API surface beyond what the
  migration unavoidably touches (e.g. `CPU::bus()` return type).



[**Architecture**]

```
                           ┌──────────────────────────────┐
                           │            CPU               │
                           │  cores: Vec<Core>            │
                           │  bus: BusHandle              │  ← new
                           └──────────────┬───────────────┘
                                          │
               ┌──────────────────────────┴──────────────────────────┐
               │                                                     │
        Owned(Box<Bus>)                                    Shared(Arc<Mutex<Bus>>)
        (cores.len() == 1)                                 (cores.len() >= 2)

        ┌───────────────┐                                  ┌───────────────┐
        │ &mut Bus      │                                  │ MutexGuard<Bus>│
        │ direct borrow │                                  │ lock / unlock  │
        └───────────────┘                                  └───────────────┘
               ▲                                                     ▲
               └──────────────────┬──────────────────────────────────┘
                                  │
                         BusHandle::with(|b| …)
                         BusHandle::with_guard() → BusGuard<'_>
                                  │
                                  ▼
                    ┌──────────────────────────────────┐
                    │ call sites (unchanged semantics) │
                    │  CPU::step              bus.tick │
                    │  RVCore::access_bus     translate│
                    │  RVCore::checked_read   read     │
                    │  RVCore::checked_write  store    │
                    │  amo_{w,d} / lr / sc    atomic   │
                    │  privileged::wfi / sret IRQ view │
                    └──────────────────────────────────┘
```

The `Core` struct (already per-hart) gets the same `BusHandle` shape as
`CPU`. In the `Owned` arm the `Core` and `CPU` share the bus through
the Rust borrow checker: construction passes `&mut Bus` to `Core` only
for the duration of one `step`, and the `Bus` physically lives in
`CPU.bus` (`Owned` variant). In the `Shared` arm every hart clones the
`Arc<Mutex<Bus>>` exactly as today.



[**Invariants**]

- I-1: For `cores.len() == 1`, `BusHandle::Owned(Box<Bus>)` holds the
  bus; no `Mutex` or `Arc` is constructed and no lock operation is
  issued on the hot path.
- I-2: For `cores.len() >= 2`, `BusHandle::Shared(Arc<Mutex<Bus>>)`
  holds the bus; every hart uses the same `Arc` and the semantics
  match main byte-for-byte.
- I-3: `BusHandle::with_guard()` returns a scope guard (`BusGuard<'a>`)
  that derefs to `&mut Bus` and encloses a single critical section.
  In `Owned` it is a thin re-borrow; in `Shared` it holds a
  `MutexGuard`. No call site may hold two `BusGuard`s concurrently.
- I-4: `sc_w` / `sc_d` / `amo_*` perform
  `translate → reservation-check → conditional-store` inside **one**
  `with_guard` scope. Holding the guard across translate does not
  widen today's critical section because `access_bus` already did so
  (`mm.rs:258–262`).
- I-5: `Bus::tick` is called exactly once per `CPU::step`, before the
  current hart steps, regardless of `BusHandle` variant.
- I-6: `CPU::reset` clears devices and reservations before any core
  reset, preserving the current order at `cpu/mod.rs:137–148`.
- I-7: Difftest behavior is unchanged: `Bus::mmio_accessed: AtomicBool`
  is still visible and exactly one `AtomicBool::swap` observation per
  committed instruction.
- I-8: No `unsafe`. The `Owned` arm uses only safe `&mut` borrows; the
  `Shared` arm uses only `std::sync::Mutex` (or, per T-3, a
  well-audited replacement) — no `UnsafeCell`.



[**Data Structure**]

```rust
// xcore/src/device/bus_handle.rs  (new file, high cohesion)

/// Ownership mode for the bus. `Owned` is selected at construction
/// time when `cores.len() == 1`; `Shared` otherwise.
pub enum BusHandle {
    Owned(Box<Bus>),
    Shared(Arc<Mutex<Bus>>),
}

/// Short-lived mutable access to the bus. `with` is the common path
/// (one call → one critical section). `with_guard` is used only
/// where a batch of accesses must stay atomic end-to-end (LR/SC,
/// AMO, translate-then-access in the Shared arm).
pub enum BusGuard<'a> {
    Owned(&'a mut Bus),
    Shared(MutexGuard<'a, Bus>),
}

impl Deref for BusGuard<'_>    { type Target = Bus; fn deref(&self) -> &Bus; }
impl DerefMut for BusGuard<'_> {                    fn deref_mut(&mut self) -> &mut Bus; }
```

No change to `Bus` itself. `Core`'s `bus: Arc<Mutex<Bus>>` field and
`CPU`'s `bus: Arc<Mutex<Bus>>` field both become
`bus: BusHandle`.



[**API Surface**]

```rust
impl BusHandle {
    pub fn new_owned(bus: Bus) -> Self;                    // cores == 1
    pub fn new_shared(bus: Bus) -> Self;                   // cores >= 2
    pub fn clone_for_hart(&self) -> Self;                  // Shared → Shared(arc.clone()); Owned → panic (caller bug)

    /// One-shot critical section. Preferred for the 95 % case.
    pub fn with<R>(&mut self, f: impl FnOnce(&mut Bus) -> R) -> R;

    /// Multi-step critical section. Used by LR/SC, AMO, and by any
    /// call site that must translate + access under one lock.
    pub fn with_guard(&mut self) -> BusGuard<'_>;

    /// Read-only accessor (used by `CPU::bus()` external consumers).
    pub fn with_ref<R>(&self, f: impl FnOnce(&Bus) -> R) -> R;
}
```

Call-site migration patterns:

```rust
// BEFORE                                         // AFTER
self.bus.lock().unwrap().tick();                  self.bus.with(|b| b.tick());

self.bus.lock().unwrap().clear_reservation(id);   self.bus.with(|b| b.clear_reservation(id));

// sc_w critical section (atomic.rs:64–69)
let success = {
    let mut bus = self.bus.lock().unwrap();       let mut g = self.bus.with_guard();
    let ok = bus.reservation(id) == Some(paddr);  let ok = g.reservation(id) == Some(paddr);
    bus.clear_reservation(id);                    g.clear_reservation(id);
    ok                                             ok
};                                                 // g drops at block end
```

External `CPU::bus()` at `cpu/mod.rs:125` becomes:

```rust
pub fn bus(&mut self) -> BusGuard<'_> { self.bus.with_guard() }
```

(signature change: `&self` → `&mut self`; documented in the migration
table below).



[**Constraints**]

- C-1: Zero `unsafe`. Every safety claim is enforced by the Rust
  borrow checker.
- C-2: No change to device `Device::tick` / `Mmio` trait shapes. The
  refactor touches only the *ownership* of `Bus`, not the devices
  inside it.
- C-3: No benchmark-specific specialization. The `Owned` arm is
  selected purely on `cores.len() == 1`, an emulator-configuration
  fact independent of which binary is loaded.
- C-4: `linux-2hart` boot path MUST exercise the `Shared` arm and
  MUST match main within ±5 percent wall clock.
- C-5: `make fmt && make clippy && make run && make test` must pass
  on every committed change (AGENTS.md §Development Standards).
- C-6: DEBUG=n for every benchmark sample (feedback_debug_flag).
- C-7: Workloads launched via `make run` / `make linux` /
  `make debian` only; no direct `target/release/xdb` invocation
  (feedback_use_make_run).
- C-8: Call sites that today take *two* locks for one logical access
  (`checked_read` at `mm.rs:265–272`, `checked_write` at
  `mm.rs:276–279`) are rewritten to take **one** `with_guard` scope
  each. This is a side-benefit, not the primary goal — documented
  so the reviewer can check it doesn't widen critical sections on
  the `Shared` arm.

---

## Implement {detail design}

### Execution Flow

[**Main Flow**]

Single-hart (`Owned`) path, per instruction:

1. `CPU::step` calls `self.bus.with(|b| b.tick())` — direct `&mut Bus`
   borrow, one function call, zero atomics.
2. `cores[0].step()` begins. Instruction fetch calls
   `RVCore::checked_read(pc, 4, Fetch)`.
3. `checked_read` opens **one** `with_guard` scope, translates, PMP
   checks, reads — guard drops.
4. Decode + execute. Memory accesses (load/store/AMO) follow the same
   pattern: one `with_guard` per access; LR/SC use a wider
   `with_guard` spanning translate + reservation-check +
   conditional-store.
5. Current hart's `halted()` queried; `advance_current()` wraps to 0
   (no-op, single hart).
6. Return to `CPU::run` loop.

Multi-hart (`Shared`) path: identical to main. `with` expands to
`self.bus.lock().unwrap()` followed by the closure; `with_guard` to
`self.bus.lock().unwrap()` returning a `MutexGuard`. No new critical
sections are introduced; the two separate locks in `checked_read` and
`checked_write` are *merged* into one, which shortens — never widens
— the `Shared` arm's critical section.



[**Failure Flow**]

1. `BusGuard` drop panics mid-critical-section: in `Owned`, `&mut` has
   no destructor so this is impossible. In `Shared`, `MutexGuard`
   poisoning propagates as today (`.unwrap()` on re-lock panics and
   terminates the process — unchanged).
2. Attempting to construct `Owned` from a config with `num_harts > 1`:
   `CPU::with_machine_config` panics at construction time with a
   clear message. This is a static configuration error, not a
   runtime failure.
3. Double-borrow (a call site tries to open two `with_guard`s
   concurrently): in `Owned`, the compiler rejects it at build time.
   In `Shared`, it deadlocks — same as today. This invariant (I-3)
   is enforced by call-site audit; no new deadlock risk vs. main.
4. Difftest build (`--features difftest`): unchanged. The
   `AtomicBool` field on `Bus` remains, and `with` / `with_guard`
   give exactly the same access pattern as `lock().unwrap()`.



[**State Transition**]

- Config::`num_harts = 1`  →  `BusHandle::Owned(Box<Bus>)` at
  `CPU::with_machine_config` construction.
- Config::`num_harts >= 2` →  `BusHandle::Shared(Arc::new(Mutex::new(bus)))`,
  cloned once per `Core`.
- Runtime: variant never changes. No dynamic upgrade/downgrade.
- Reset (`CPU::reset`): opens one `with_guard`, calls
  `reset_devices()` + `clear_reservations()`, drops guard, then
  resets cores. Same ordering as `cpu/mod.rs:141–148`.



### Implementation Plan

[**Phase 1 — Introduce `BusHandle` alongside existing `Arc<Mutex<Bus>>`**]

- 1a. Create `xcore/src/device/bus_handle.rs` with the enum, the
  `with` / `with_guard` / `with_ref` / `clone_for_hart` API, and the
  `BusGuard` wrapper. All methods are safe, no `unsafe`.
- 1b. Write unit tests for `BusHandle` in the same file: `Owned`
  allows `with`, `Shared` allows cloning, `with_guard` gives
  `DerefMut` access, reservation round-trip works in both arms.
- 1c. `make fmt && make clippy && cargo test -p xcore bus_handle`.
  No integration yet.

[**Phase 2 — Migrate call sites (Core + CPU)**]

Migration table (one `Edit` per site, one commit per logical group):

| Site                                        | Pattern         | Notes |
|---------------------------------------------|-----------------|-------|
| `cpu/mod.rs:125` `CPU::bus()`               | `with_guard`    | Signature becomes `&mut self`. Update the one difftest caller accordingly. |
| `cpu/mod.rs:142–145` reset                  | `with_guard`    | Single scope covers both `reset_devices` + `clear_reservations`. |
| `cpu/mod.rs:168,199` image loads            | `with`          | Single-statement `load_ram`. |
| `cpu/mod.rs:214` step tick                  | `with`          | Hottest path; expect the biggest measured delta here. |
| `cpu/mod.rs:385–387` CPU construction       | new             | Switch on `num_harts`; construct `Owned` or `Shared`. |
| `arch/riscv/cpu/mm.rs:258` `access_bus`     | `with_guard`    | Already holds lock across translate; unchanged width. |
| `arch/riscv/cpu/mm.rs:267–270` `checked_read` | `with_guard`  | **Merges two locks into one**; shortens Shared-arm crit section. |
| `arch/riscv/cpu/mm.rs:276–279` `checked_write`| `with_guard`  | Same merge. |
| `arch/riscv/cpu/inst/atomic.rs:30,45` AMO   | `with`          | `clear_reservation` is one-shot. |
| `arch/riscv/cpu/inst/atomic.rs:57,81` LR    | `with`          | `reserve` is one-shot. |
| `arch/riscv/cpu/inst/atomic.rs:64–69,89–94` SC | `with_guard` | Triple-lock zone; must stay atomic. |
| `arch/riscv/cpu.rs:207` `mtime()`           | `with`          | One read. |
| `arch/riscv/cpu/privileged.rs` WFI / IRQ view | `with`        | All are one-shot queries. |
| `cpu/mod.rs` tests                          | update          | Replace `Arc::new(Mutex::new(bus))` setup with `BusHandle::new_*`. |
| `device/bus.rs` tests                       | none            | `Bus` API unchanged. |

After each group: `make fmt && make clippy && make test`.

[**Phase 3 — Verify, measure, document**]

- 3a. Boot gate: `make run` (microbench) completes; `make linux` boots
  to prompt; `make linux-2hart` boots to prompt.
- 3b. Perf sampling: `scripts/perf/sample.sh` with DEBUG=n on
  dhrystone, coremark, microbench; render via
  `scripts/perf/render.py` into
  `docs/perf/<post-P1-date>/data/` + `REPORT.md`.
- 3c. Compare against `docs/perf/2026-04-14/data/bench.csv`:
  report Before / After wall-clock, Before / After
  `pthread_mutex_*` self-time.
- 3d. `linux-2hart` boot-to-prompt time sampled three runs each
  pre / post; confirm ±5 percent.
- 3e. Update `docs/perf/PERF_DEV.md` P1 row to "Done" with the
  measured numbers.



## Trade-offs {ask reviewer for advice}

- T-1: **Ownership model — `BusHandle` enum (option 1, recommended)
  vs. `Arc<UnsafeCell<Bus>>` behind a single-hart feature flag
  (option 2) vs. trait-object `BusAccess` split (option 3).**

  Option 1 (recommended): safe Rust, one added enum, branch
  predictor trivially collapses the hot path because the variant
  never changes per-run. Cost: one `match` per bus access in both
  arms; LLVM typically hoists the discriminant check out of tight
  loops. No `unsafe` invariant cost.

  Option 2: `Arc<UnsafeCell<Bus>>` + `#[cfg(feature = "single_hart")]`
  skips the discriminant entirely. Faster on paper but buys a
  permanent `unsafe` invariant ("no hart ever holds a concurrent
  borrow"), forces a build-flag matrix on every CI config, and
  makes `linux-2hart` a separate binary. Rejected per NG-4.

  Option 3: trait `BusAccess` with `BorrowedBus` / `SharedBus`
  implementers. Vtable indirection re-introduces roughly the cost
  we're removing (branch target buffer miss + indirect call).
  Rejected.

  Rvemu / riscv-rust / Rare / rrs all adopt the owned-bus shape
  (without even keeping a `Shared` fallback, since they are all
  single-hart). QEMU's MTTCG arc is the mirror lesson in the
  opposite direction: the emulator started with one global lock
  (BQL) and has been *pushing the lock down* toward IO memory only
  (LWN 697031/697265, QEMU multi-thread-tcg docs). `BusHandle` is
  xemu's version of that push.

- T-2: **`CPU::bus()` return type.**
  Option A: `BusGuard<'_>` uniformly. Call sites unchanged in shape.
  Option B: In `Owned`, return `&mut Bus` directly (no wrapper).
  Faster borrow, tighter lifetimes, but bifurcates the API.
  Proposal: A. Consistency is worth one extra enum-discriminant
  check per external call — external callers are cold by
  definition (difftest harness, tests, REPL).

- T-3: **`Shared` arm mutex choice.**
  Option A: keep `std::sync::Mutex`. Option B: swap to
  `parking_lot::Mutex` — roughly 1.5× uncontended speedup per
  parking_lot's own benchmark, no poison semantics to worry about.
  Proposal: A for now. The `Shared` arm only runs under
  `linux-2hart`, which is not the P1 target; mutex swap can be a
  follow-up phase if `linux-2hart` itself becomes a hotspot. Plan
  asks reviewer to confirm or overrule.

- T-4: **Merging `checked_read` / `checked_write`'s two locks into
  one.**
  Today these methods lock twice (translate, then read or store);
  the refactor merges them into a single `with_guard` scope.
  Pro: removes redundant lock/unlock pair, matches `access_bus`'s
  already-broader critical section, shortens the `Shared` arm.
  Con: translate + RAM access hold the guard jointly; in `Shared`
  this equals `access_bus`'s existing width, so no regression, but
  it does mean a future MTTCG phase must split this back out
  explicitly. Proposal: merge, document the future-split constraint
  in the P1 commit message.



## Validation {test design}

[**Unit Tests**]

- V-UT-1: `bus_handle` tests (new file): `Owned` and `Shared` round-trip
  `with(|b| b.store/read)`; `with_guard` allows multi-step
  reservation check; `clone_for_hart` on `Owned` panics.
- V-UT-2: `Bus` existing tests (13 in `device/bus.rs`) unchanged —
  `Bus` API and internals are untouched.
- V-UT-3: `CPU` existing tests (13 in `cpu/mod.rs`) run against the
  `Owned` variant (tests are single-hart by default); add one new
  test `cpu_run_with_shared_handle` that constructs a
  `BusHandle::Shared` manually and asserts identical step results
  on a 5-instruction golden trace.
- V-UT-4: `RVCore` tests (5 in `arch/riscv/cpu.rs`) plus atomic tests
  (20+ in `atomic.rs`): all run green on `Owned`. Add one
  explicit `sc_w_atomicity_under_owned_bus` test that issues
  `lr → store-that-invalidates → sc` on a single hart and asserts
  `sc` returns failure — proves the `with_guard` scope didn't
  collapse the triple-lock zone.

[**Integration Tests**]

- V-IT-1: `arch_isolation.rs` seam test runs unchanged — validates
  the refactor doesn't leak arch-specific types into `xcore`'s
  public surface.
- V-IT-2: `make run` (default microbench / direct image) boots and
  exits 0.
- V-IT-3: `make linux` boots to `/ # ` prompt, runs
  `echo hello; exit`.
- V-IT-4: `make linux-2hart` boots to prompt and both harts appear
  in `cat /proc/cpuinfo`. This exercises the `Shared` arm.
- V-IT-5: `make debian` boots to shell prompt (full userland,
  longest-running sanity check).
- V-IT-6: `make xv6` boots (if the target is wired up locally;
  skipped if not — declared explicitly so reviewer can decide).

[**Failure / Robustness Validation**]

- V-F-1: Force `num_harts = 2` at construction and assert `BusHandle`
  is `Shared` (unit test on the factory).
- V-F-2: Reset mid-execution (`CPU::reset` called after boot): all
  reservations cleared, devices reset, subsequent step succeeds.
  Reuses existing `CPU::reset` test; asserts it still passes in the
  `Owned` arm.
- V-F-3: Difftest build (`cargo test --features difftest -p xcore`)
  green — confirms the `AtomicBool` side-channel still fires.

[**Edge Case Validation**]

- V-E-1: Zero-instruction run (`CPU::run(0)`) — tick must **not**
  fire (matches today's behavior since `step` isn't called). Test
  covers `Owned` arm.
- V-E-2: Single-hart LR/SC on the same hart's own reservation
  succeeds; single-hart LR followed by a peer-less store on a
  different address does **not** invalidate the reservation
  (confirms `invalidate_peer_reservations` path is dead-but-correct
  on single hart).
- V-E-3: Two-hart LR/SC: hart 0 issues `lr`, hart 1 stores to the
  reserved granule, hart 0's `sc` fails. Exercises the `Shared`
  arm's atomicity.
- V-E-4: Boot-then-reset loop (load image, reset, re-run) — catches
  any leaked `BusHandle` ownership across the cycle.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (zero mutex on single-hart hot path) | V-IT-2 + perf-sample: `pthread_mutex_*` self-time < 5 percent on dhrystone/coremark/microbench vs. ~33 to 40 percent baseline. |
| G-2 (>= 15 percent wall-clock reduction, expected 20 to 30 percent) | V-IT-2 + perf-sample compared line-by-line to `docs/perf/2026-04-14/data/bench.csv`. |
| G-3 (multi-hart unchanged) | V-IT-4, V-E-3. Delta < 5 percent on `linux-2hart` boot time over 3 runs. |
| G-4 (LR/SC atomicity preserved) | V-UT-4, V-E-3. |
| G-5 (public APIs stable) | V-UT-3 (re-uses existing test setup), V-IT-1 (`arch_isolation`). |
| C-1 (no `unsafe`) | `make clippy` + grep `-n unsafe` in the diff shows 0 new occurrences. |
| C-2 (device traits unchanged) | V-UT-2 (all `device/bus.rs` tests green). |
| C-3 (no benchmark-specific tricks) | Code review: the `Owned` selector is `num_harts == 1`, not any benchmark fingerprint. |
| C-4 (linux-2hart ±5 percent) | V-IT-4 sampled 3×, reported in the new `docs/perf/<post-P1-date>/REPORT.md`. |
| C-5 (fmt/clippy/run/test clean) | CI and `make` gate on every Phase 2 commit. |
| C-6 (DEBUG=n benchmarks) | `scripts/perf/sample.sh` env assertion. |
| C-7 (make-based launches) | Perf report records the exact `make` targets used. |
| C-8 (no widened Shared critical section) | V-IT-4 timing within ±5 percent proves no observable widening. |
