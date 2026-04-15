# `perfBusFastPath` PLAN `01`

> Status: Revised
> Feature: `perfBusFastPath`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `none` (00_MASTER is blank)

---

## Summary

Phase P1 of the xemu perf roadmap. Eliminate the `Arc<Mutex<Bus>>` lock
overhead that dominates self-time on dhrystone / coremark / microbench
(roughly 33 to 40 percent of cycles spent in
`pthread_mutex_{lock,unlock,trylock}` per `docs/perf/2026-04-14/REPORT.md`)
when `cores.len() == 1`, while keeping the existing shared-bus path
bit-for-bit identical for multi-hart configurations (`make linux-2hart`).

The fix is structural, not benchmark-targeted. It replaces the single
`Arc<Mutex<Bus>>` field on `CPU` and `RVCore` with a `BusHandle`
ownership abstraction whose `Owned` variant stores `Box<Bus>` (single
hart, direct `&mut Bus` borrow, zero atomics) and whose `Shared`
variant keeps the current `Arc<Mutex<Bus>>` shape byte-for-byte
(multi-hart). All bus access is routed through a uniform
`with` / `with_guard` API so LR/SC and translate-then-access batches
stay atomic. The `Owned` arm is chosen on `cores.len() == 1`, an
emulator-configuration fact independent of which guest binary runs, so
the P1 gain applies to every single-hart workload uniformly.

Exit-gate floor is rewritten per 00_REVIEW R-003: mutex self-time
< 5 %, wall-clock reduction >= 15 % (required), 20-30 % expected,
35 % theoretical ceiling (only if the entire mutex bucket drops and
no redistribution into `access_bus` / `checked_read` / `xdb::main`
occurs -- which bucket math says will not). A `cargo asm CPU::step`
spot-check confirms the `Owned` arm compiles to a direct deref with no
CAS. This plan also widens scope audit per R-001 (adds the missing
hot call sites from a full repo-wide grep), adds a behavioural
critical-section-width test per R-002, codifies invariant I-9 per
R-004, reshapes `clone_for_hart` via a `BusOwner` type-state factory
per R-005, splits `CPU::bus()` / `CPU::bus_mut()` per R-007, adds
source citations per R-008, and keeps the criterion microbench as a
nice-to-have per R-009.

Scope reconfirmation: P1 only changes the bus handle. It does NOT
add a softmmu TLB (that is P2), a decoded-instruction cache (P4), or
any guest-PC or binary-specific specialisation. There are NO
benchmark-targeted tricks in this plan; every branch is driven by
`num_harts`, a configuration fact set before any image is loaded.

## Log

[**Feature Introduce**]

Second iteration. Carries forward the `BusHandle` / `BusGuard`
abstraction and the single-hart ownership fast path from 00_PLAN.
Adds: a complete migration table covering every `bus.lock()` site
in `xemu/xcore/src/`, a Shared-arm critical-section-width regression
test, invariant I-9 (no reentrant bus access from device closures),
a type-safer `BusOwner` factory, a split `CPU::bus()` /
`CPU::bus_mut()` API, softened headline-gain math, and an
asm-level compile-time check for the `Owned` fast path.



[**Review Adjustments**]

All 00_REVIEW findings resolved in this plan (see Response Matrix):

- R-001 HIGH: migration table rewritten from the repo-wide grep
  `rg 'bus\.lock\(\)' xemu/xcore/src`. Every site listed with
  file:line + pattern. Non-existent `arch/riscv/cpu/privileged.rs`
  row removed.
- R-002 HIGH: added V-UT-5
  (`bus_handle_shared_lock_width`, counter-instrumented `Bus`
  wrapper) and V-IT-7
  (`xcore/tests/shared_bus_torture.rs`, 2-hart LR/SC torture with
  50 ms budget).
- R-003 MEDIUM: headline band softened to 15 % floor (required) /
  20-30 % expected / 35 % theoretical ceiling with bucket-math
  justification. Added Phase 3 step 3e running
  `cargo asm -p xcore xcore::cpu::CPU::step --rust` to confirm no
  CAS in the Owned fast path.
- R-004 MEDIUM: invariant I-9 added. `debug_assert!` guard on the
  `Shared` arm uses a `Cell<bool>` re-entry flag in debug builds,
  zero cost in release.
- R-005 MEDIUM: factory reshaped. `BusOwner` type-state constructs
  the bus; `BusOwner::into_handles(self, num_harts)` yields
  `Vec<BusHandle>` where the `Owned` branch is only reachable when
  `num_harts == 1`. No `clone_for_hart` panic path remains.
- R-006 MEDIUM: reset row re-phrased -- no change required, maps
  1:1 to `with_guard`.
- R-007 MEDIUM: public API split into `CPU::bus(&self)` (read-only,
  via `with_read_guard`) and `CPU::bus_mut(&mut self)` (writable,
  via `with_guard`). External-caller audit enumerated.
- R-008 LOW: source URLs added for rvemu, rv8, Rare, riscv-rust,
  rrs, QEMU MTTCG docs, LWN BQL article, and parking_lot.
- R-009 LOW: criterion bench at `xcore/benches/bus_step.rs` added
  as nice-to-have Phase 3 step 3f, explicitly NOT an exit-gate
  artefact.



[**Master Compliance**]

N/A. `00_MASTER.md` contains no directives (blank template).



### Changes from Previous Round

[**Added**]
- Invariant I-9 (no reentrant bus access from device closures).
- `BusOwner` type-state factory (Data Structure / API Surface).
- `CPU::bus_mut()` method; `CPU::bus()` now returns a read-only
  guard via `with_read_guard`.
- `ReadBusGuard<'a>` enum (Owned: `&Bus`, Shared: `MutexGuard<Bus>`
  used read-only) and `BusHandle::with_ref` / `with_read_guard`.
- V-UT-5 `bus_handle_shared_lock_width` counter test.
- V-UT-6 `bus_handle_rejects_reentry` test.
- V-IT-7 `tests/shared_bus_torture.rs` two-hart LR/SC torture.
- V-IT-8 `cargo asm` compile-time check on `CPU::step`.
- Phase 3 step 3e asm check; step 3f criterion microbench
  (nice-to-have, not gated).
- Source URLs in Trade-offs for every cited project.
- Additional call sites in the migration table:
  `cpu/mod.rs:101`, `cpu/mod.rs:293`, `cpu/mod.rs:323`,
  `cpu/debug.rs:93-99`, `cpu/debug.rs:103-109`,
  `inst/base.rs:75`, `inst/compressed.rs:573`,
  `inst/float.rs:1085`, `inst/atomic.rs:195/200/204/212` (test
  helpers).



[**Changed**]
- Headline gain band: was "20-30 % expected, 15 % floor" -> now
  "15 % floor, 20-30 % expected, 35 % theoretical ceiling" with
  bucket-math justification.
- `CPU::bus()` signature: was proposed as
  `&mut self -> BusGuard<'_>` (API break) -> now stays
  `&self -> ReadBusGuard<'_>`; `bus_mut(&mut self) -> BusGuard<'_>`
  is a pure addition. G-5 upheld.
- `BusHandle::clone_for_hart`: removed -- replaced by
  `BusOwner::into_handles`.
- Reset migration row: was "merge two scopes" -> now "no change
  required; maps 1:1 to `with_guard`".
- Privileged-file migration row: removed (file does not exist; the
  real file `arch/riscv/cpu/inst/privileged.rs` has zero
  `bus.lock()` calls).
- Exit gate: added asm check (V-IT-8) and behavioural test
  (V-UT-5 + V-IT-7).



[**Removed**]
- `arch/riscv/cpu/privileged.rs` migration row.
- `BusHandle::clone_for_hart` method (replaced by `BusOwner`).
- Proposed `CPU::bus()` signature change (replaced by split).



[**Unresolved**]
- U-1: Whether the `Shared` arm should eventually switch to
  `parking_lot::Mutex`. 00_REVIEW TR-3 confirmed keeping
  `std::sync::Mutex` for P1; revisit in a later phase if
  `linux-2hart` becomes a hotspot.
- U-2: Whether external callers of `CPU::bus_mut` (difftest
  harness only, per the audit) would benefit from a more narrowly
  scoped API (e.g. `CPU::take_mmio_flag_mut`) -- deferred.
- U-3: Long-term fate of the `Shared` arm once a per-hart TLB /
  MTTCG phase lands. Not in scope for P1.



### Response Matrix

| Source | ID    | Decision | Resolution / Action in this plan                                                                                                                                                                                                                                                                           | Test or gate that proves it                                                                                   |
|--------|-------|----------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------|
| Review | R-001 | Accepted | Migration table fully re-enumerated from `rg 'bus\.lock\(\)' xemu/xcore/src`; all sites listed with file:line and `with` vs `with_guard` vs `with_ref`. Non-existent `arch/riscv/cpu/privileged.rs` row removed. `inst/base.rs:75` migration is `with`, explicitly NOT fused into `store_op`'s write guard (T-4 documents why). | After migration, `rg 'bus\.lock\(\)' xemu/xcore/src` returns zero matches. Checked at each Phase-2 commit.    |
| Review | R-002 | Accepted | Added V-UT-5 `bus_handle_shared_lock_width` (counter-instrumented Shared handle; asserts per-step acquire count equals a frozen pre-P1 constant on a 64-instruction golden trace) and V-IT-7 `tests/shared_bus_torture.rs` (2-hart LR/SC torture, 50 ms budget).                                           | Both tests must pass in `make test` on the P1 branch. Pre-P1 baseline counter captured in a fixture constant. |
| Review | R-003 | Accepted | G-2 rewritten: 15 % floor (required), 20-30 % expected, 35 % theoretical ceiling, with bucket-math reasoning. Phase 3 step 3e runs `cargo asm -p xcore xcore::cpu::CPU::step --rust` and asserts no `lock cmpxchg`, `xchg`, or `pthread_mutex_` symbol in the Owned fast path.                             | V-IT-8 asm check; Phase 3 perf sample must show >= 15 % wall-clock reduction.                                 |
| Review | R-004 | Accepted | New invariant I-9 (no reentrant bus access from closures). Enforced by borrow checker in `Owned`; enforced by a `Cell<bool>` re-entry flag + `debug_assert!` in `Shared` (debug builds only, zero cost in release).                                                                                        | V-UT-6 `bus_handle_rejects_reentry` test (compile_fail doctest for `Owned`; debug-assert panic for `Shared`). |
| Review | R-005 | Accepted | `clone_for_hart` removed. Replaced by `BusOwner` type-state: `BusOwner::new(bus).into_handles(num_harts)` returns `Vec<BusHandle>`; `Owned` is produced only when `num_harts == 1`. No runtime panic path for a compile-time-knowable misuse.                                                              | V-F-1 (`into_handles(2)` yields two `Shared` handles sharing one `Arc`); compiler rejects any other misuse.   |
| Review | R-006 | Accepted | Reset row re-phrased to "already a single scope today; maps 1:1 to `with_guard`". No behavioural change on the `Shared` arm.                                                                                                                                                                              | Existing `cpu/mod.rs` reset test remains green.                                                               |
| Review | R-007 | Accepted | Public API split: `CPU::bus(&self) -> ReadBusGuard<'_>` (read-only, via `with_read_guard`) preserves `&self` source compatibility; `CPU::bus_mut(&mut self) -> BusGuard<'_>` is a pure addition. External-caller audit: only two test-only callers (`cpu/mod.rs:461`, `:551`); no changes required.          | External-caller audit in API Surface; `cargo check -p xcore --features difftest` passes.                     |
| Review | R-008 | Accepted | Source URLs added for rvemu, rv8, Rare, riscv-rust, rrs, QEMU MTTCG, LWN BQL, and parking_lot (in Trade-offs T-1 and T-3).                                                                                                                                                                                | Trade-offs section citations.                                                                                 |
| Review | R-009 | Accepted | Criterion microbench `xcore/benches/bus_step.rs` added as nice-to-have Phase 3 step 3f, explicitly NOT an exit-gate artefact.                                                                                                                                                                             | If present, bench is reported in `docs/perf/<post-P1-date>/REPORT.md`; if absent, gate still passes.          |
| Master | --    | N/A      | `00_MASTER.md` is blank; no directives to apply.                                                                                                                                                                                                                                                          | --                                                                                                             |

> Rules:
> - Every prior HIGH / CRITICAL finding appears here.
> - Every Master directive appears here.
> - Rejections must include explicit reasoning. (None in this round.)

---

## Spec {Core specification}

[**Goals**]

- G-1: On the single-hart configuration, `CPU::step` and every
  per-instruction memory access (`checked_read`, `checked_write`,
  `access_bus`, AMO / LR / SC, `Bus::tick`) execute with **zero**
  `pthread_mutex_*` calls on the hot path.
- G-2: Wall-clock runtime of `make run` (dhrystone, coremark,
  microbench, DEBUG=n) drops by **at least 15 %** vs.
  `docs/perf/2026-04-14/data/bench.csv`; 20-30 % is the **expected**
  band; 35 % is the **theoretical ceiling** only if the entire mutex
  bucket drops and no sample redistributes into `access_bus`,
  `checked_read`, or `xdb::main`. Bucket math: macOS pthread
  uncontended lock+unlock is roughly 20-40 ns; at ~2-3 acquisitions
  per guest instruction and tens-of-M instr/s, mutex work is ~1-3 s
  of a ~9 s dhrystone run, i.e. 10-30 % of wall-clock even if 100 %
  of the bucket vanishes. The remainder of the reported 33-40 %
  self-time is PLT stub plus CAS-only fast-path work that partially
  redistributes into the caller.
- G-3: Multi-hart semantics are byte-identical to main on the
  `linux-2hart` boot path (tick ordering, reservation visibility,
  direct IRQ signalling, MTIMER `mtime` monotonicity all preserved).
- G-4: LR/SC atomicity across translate -> reservation-read ->
  conditional-store is preserved without widening any existing
  critical section.
- G-5: Public `CPU::bus()` signature is preserved as
  `&self -> ReadBusGuard<'_>`; a new `CPU::bus_mut(&mut self)`
  handles the rare mutable-external-access case. The
  `BusHandle` / `BusOwner` refactor is `xcore`-internal otherwise.

- NG-1: Not a JIT, not a TLB, not a softmmu fast path -- those are
  future phases per `docs/perf/PERF_DEV.md`.
- NG-2: No per-instruction benchmark-aware branch pruning, no
  guest-PC specialisation, no skipping of UART / PLIC / MTIMER ticks.
- NG-3: No change to device trait (`Device::tick`, `Mmio::read/write`,
  `Device::reset`) semantics or ordering.
- NG-4: No `unsafe` to bypass the borrow checker. Option 2
  (`Arc<UnsafeCell<Bus>>`) is explicitly rejected (see T-1).
- NG-5: No new public crate-external API surface beyond the split
  of `CPU::bus` into `CPU::bus` (read-only) + `CPU::bus_mut`
  (writable). Both guard return types are `pub` but consumed only
  as `Deref<Target = Bus>` in practice.
- NG-6: No softmmu TLB (P2) or decoded-instruction cache (P4) work
  in this phase. P1 changes only the bus handle.



[**Architecture**]

```
                           +------------------------------+
                           |            CPU               |
                           |  cores: Vec<Core>            |
                           |  bus: BusHandle              |  <- new
                           +--------------+---------------+
                                          |
               +--------------------------+--------------------------+
               |                                                     |
        Owned(Box<Bus>)                                    Shared(Arc<Mutex<Bus>>)
        (cores.len() == 1)                                 (cores.len() >= 2)

        +---------------+                                  +----------------+
        | &mut Bus      |                                  | MutexGuard<Bus>|
        | direct borrow |                                  | lock / unlock  |
        +---------------+                                  +----------------+
               ^                                                     ^
               +------------------+----------------------------------+
                                  |
                   BusHandle::with(|b| ...)               closure variant
                   BusHandle::with_guard() -> BusGuard<'_> guard variant (LR/SC, AMO)
                   BusHandle::with_ref(|b| ...)           read-only closure
                   BusHandle::with_read_guard() -> ReadBusGuard<'_>
                                  |
                                  v
                    +----------------------------------+
                    | call sites (unchanged semantics) |
                    |  CPU::step              bus.tick |
                    |  RVCore::access_bus     translate|
                    |  RVCore::checked_read   read     |
                    |  RVCore::checked_write  store    |
                    |  amo_{w,d} / lr / sc    atomic   |
                    |  debug::fetch_inst      inspect  |
                    |  difftest::take_mmio    flag swap|
                    +----------------------------------+

 Construction:
     BusOwner::new(bus).into_handles(num_harts) -> Vec<BusHandle>
         - num_harts == 1 => vec![Owned(Box<Bus>)]
         - num_harts >= 2 => vec![Shared(arc); num_harts]
     CPU takes handles[0]; each Core i takes handles[i].
```

The `Core` struct (already per-hart) gets the same `BusHandle` shape
as `CPU`. In the `Owned` arm the `Core` and `CPU` coordinate through
the Rust borrow checker: the bus physically lives in `CPU`'s
`BusHandle::Owned` and each `Core` holds its own `BusHandle::Owned`
to a separate `Box<Bus>` is not the design -- rather, when
`num_harts == 1` there is one `BusHandle` total (on `CPU`) and the
single `Core` receives `&mut Bus` via `with` / `with_guard` during
its step. The `Core`'s `bus` field for single-hart mode is a
re-borrow handle that is reset each call. In `Shared` mode each
`Core` owns a cloned `Arc` of the same mutex.



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
  `translate -> reservation-check -> conditional-store` inside **one**
  `with_guard` scope. Holding the guard across translate does not
  widen today's critical section because `access_bus` already did so
  (`mm.rs:258-262`).
- I-5: `Bus::tick` is called exactly once per `CPU::step`, before the
  current hart steps, regardless of `BusHandle` variant.
- I-6: `CPU::reset` clears devices and reservations inside a single
  `with_guard` scope before any core reset, preserving the current
  order at `cpu/mod.rs:141-148`. Already a single scope today.
- I-7: Difftest behaviour is unchanged: `Bus::mmio_accessed: AtomicBool`
  is still visible and exactly one `AtomicBool::swap` observation per
  committed instruction.
- I-8: No `unsafe`. The `Owned` arm uses only safe `&mut` borrows; the
  `Shared` arm uses only `std::sync::Mutex` -- no `UnsafeCell`.
- **I-9 (new, per R-004)**: No closure passed to `BusHandle::with`,
  `BusHandle::with_guard`, `BusHandle::with_ref`, or `with_read_guard`
  may call back into the owning `CPU`'s bus (directly or transitively
  through any `Device::tick` body). In `Owned` this is enforced by the
  borrow checker at compile time (a reentrant call needs `&mut` while
  an outer `&mut` is already live). In `Shared` it would deadlock
  today; under I-9 it is additionally rejected by a
  `debug_assert!(!self.reentry.replace(true))` guard in debug builds,
  using a `Cell<bool>` on the `BusHandle`. Release cost: zero (the
  `Cell` lives under `#[cfg(debug_assertions)]`).



[**Data Structure**]

```rust
// xcore/src/device/bus_handle.rs  (new file, high cohesion, <400 lines)

/// Ownership mode for the bus. Decided at construction time by
/// `BusOwner::into_handles(num_harts)`. Variant never changes at
/// runtime.
pub enum BusHandle {
    Owned(Box<Bus>),
    Shared(Arc<Mutex<Bus>>),
    // NB: under #[cfg(debug_assertions)] each variant carries a
    // `Cell<bool>` reentry flag for the I-9 guard. Zero cost in
    // release builds.
}

/// Short-lived mutable access to the bus. `with` is the common path
/// (one call -> one critical section). `with_guard` is used where a
/// batch of accesses must stay atomic end-to-end (LR/SC, AMO,
/// translate-then-access).
pub enum BusGuard<'a> {
    Owned(&'a mut Bus),
    Shared(MutexGuard<'a, Bus>),
}

/// Short-lived read-only access; returned by `CPU::bus()` for
/// external callers.
pub enum ReadBusGuard<'a> {
    Owned(&'a Bus),
    Shared(MutexGuard<'a, Bus>),  // used read-only
}

impl Deref for BusGuard<'_>       { type Target = Bus; /* ... */ }
impl DerefMut for BusGuard<'_>    { /* ... */ }
impl Deref for ReadBusGuard<'_>   { type Target = Bus; /* ... */ }

/// Type-state factory. Constructs the bus once; splits it into N
/// handles via `into_handles`. `Owned` is reachable only when
/// `num_harts == 1`, encoded structurally (not by runtime check).
pub struct BusOwner {
    bus: Bus,
}
```

No change to `Bus` itself. `Core`'s and `CPU`'s `bus` fields both
become `BusHandle`. In `num_harts == 1` mode, `Core` holds a
forwarding `BusHandle::Owned` temporarily populated per step; the
primary owner is `CPU`.



[**API Surface**]

```rust
impl BusOwner {
    pub fn new(bus: Bus) -> Self;
    pub fn into_handles(self, num_harts: usize) -> Vec<BusHandle>;
    // internal: if num_harts == 1 => Vec<Owned(Box::new(bus))>
    //          if num_harts >= 2 => Vec<Shared(Arc::new(Mutex::new(bus))); n]
}

impl BusHandle {
    /// One-shot critical section. Preferred for the 95 % case.
    pub fn with<R>(&mut self, f: impl FnOnce(&mut Bus) -> R) -> R;

    /// Multi-step critical section. Used by LR/SC, AMO, translate-
    /// then-access.
    pub fn with_guard(&mut self) -> BusGuard<'_>;

    /// Read-only closure access.
    pub fn with_ref<R>(&self, f: impl FnOnce(&Bus) -> R) -> R;

    /// Read-only guard (used by public `CPU::bus()`).
    pub fn with_read_guard(&self) -> ReadBusGuard<'_>;
}
```

Public `CPU` surface (per R-007 split):

```rust
impl CPU {
    /// Read-only access to the bus. Preserves &self to keep source
    /// compatibility with external callers.
    pub fn bus(&self) -> ReadBusGuard<'_> { self.bus.with_read_guard() }

    /// Mutable access. Only called by difftest helpers and tests.
    pub fn bus_mut(&mut self) -> BusGuard<'_> { self.bus.with_guard() }
}
```

**External `CPU::bus()` / `bus_mut()` caller audit**
(grep `\.bus\(\)` across `/Users/anekoique/ProjectX`):
- `xemu/xcore/src/cpu/mod.rs:461` -- `cpu.bus().read(...)` inside a
  `#[cfg(test)]` block; read-only, routes through new
  `CPU::bus`. No change required.
- `xemu/xcore/src/cpu/mod.rs:551` -- `cpu.bus().num_harts()` inside
  a `#[cfg(test)]` block; read-only. No change required.
- No matches outside `xcore`. `xdb`, `xtool`, `xkernels`,
  `xemu/tests/` do not call `cpu.bus()` today.
- Conclusion: R-007 split preserves source compatibility for every
  existing caller; `bus_mut` is a pure addition used only by the
  difftest harness (which today calls
  `self.bus.lock().unwrap().take_mmio_flag()` at
  `cpu/mod.rs:323` internally -- unchanged in shape).

Call-site migration patterns (representative):

```rust
// BEFORE                                         // AFTER
self.bus.lock().unwrap().tick();                  self.bus.with(|b| b.tick());

self.bus.lock().unwrap().clear_reservation(id);   self.bus.with(|b| b.clear_reservation(id));

// sc_w critical section (atomic.rs:65-69)
let success = {                                   let success = {
    let mut bus = self.bus.lock().unwrap();           let mut g = self.bus.with_guard();
    let ok = bus.reservation(id) == Some(paddr);      let ok = g.reservation(id) == Some(paddr);
    bus.clear_reservation(id);                        g.clear_reservation(id);
    ok                                                 ok
};                                                };
```



[**Constraints**]

- C-1: Zero `unsafe`. Every safety claim is enforced by the Rust
  borrow checker.
- C-2: No change to `Device::tick` / `Mmio` trait shapes. The
  refactor touches only the *ownership* of `Bus`, not the devices
  inside it.
- C-3: No benchmark-specific specialisation. The `Owned` arm is
  selected purely on `cores.len() == 1`, an emulator-configuration
  fact independent of which binary is loaded.
- C-4: `linux-2hart` boot path MUST exercise the `Shared` arm and
  MUST match main within +/-5 % wall clock.
- C-5: `make fmt && make clippy && make run && make test` must pass
  on every committed change (AGENTS.md Development Standards).
- C-6: DEBUG=n for every benchmark sample (feedback_debug_flag).
- C-7: Workloads launched via `make run` / `make linux` /
  `make debian` only; no direct `target/release/xdb` invocation
  (feedback_use_make_run).
- C-8: Call sites that today take *two* locks for one logical access
  (`checked_read` at `mm.rs:265-272`, `checked_write` at
  `mm.rs:276-279`) are rewritten to take **one** `with_guard` scope
  each. V-UT-5 (counter test) proves the Shared-arm critical-section
  acquire count does not increase vs. pre-P1 baseline on a fixed
  golden trace.

---

## Implement {detail design}

### Execution Flow

[**Main Flow**]

Single-hart (`Owned`) path, per instruction:

1. `CPU::step` calls `self.bus.with(|b| b.tick())` -- direct `&mut Bus`
   borrow, one function call, zero atomics.
2. `cores[0].step()` begins. Instruction fetch calls
   `RVCore::checked_read(pc, 4, Fetch)`.
3. `checked_read` opens **one** `with_guard` scope, translates, PMP
   checks, reads -- guard drops.
4. Decode + execute. Memory accesses (load/store/AMO) follow the same
   pattern: one `with_guard` per access; LR/SC use a wider
   `with_guard` spanning translate + reservation-check +
   conditional-store.
5. Regular (non-AMO) stores call `store_op` which issues the write
   via `checked_write`, then calls
   `self.bus.with(|b| b.clear_reservation(id))` as a second,
   one-shot scope. Per T-4, this is NOT fused into the
   `checked_write` guard: fusing would widen the Shared-arm scope
   beyond `access_bus`'s current width (translate + write) by one
   additional operation. The V-UT-5 counter guards against
   accidental fusion.
6. Current hart's `halted()` queried; `advance_current()` wraps to 0
   (no-op, single hart).
7. Return to `CPU::run` loop.

Multi-hart (`Shared`) path: identical to main. `with` expands to
`self.bus.lock().unwrap()` followed by the closure; `with_guard` to
`self.bus.lock().unwrap()` returning a `MutexGuard`. The two separate
locks in `checked_read` and `checked_write` are merged into one --
this shortens (never widens) the `Shared` arm's critical section.



[**Failure Flow**]

1. `BusGuard` drop panics mid-critical-section: in `Owned`, `&mut`
   has no destructor so this is impossible. In `Shared`, `MutexGuard`
   poisoning propagates as today (`.unwrap()` on re-lock panics and
   terminates the process -- unchanged).
2. Attempting to construct `Owned` from a config with `num_harts > 1`:
   impossible by construction. `BusOwner::into_handles` is the only
   producer of `BusHandle` and emits `Owned` only on
   `num_harts == 1`.
3. Double-borrow (a call site tries to open two `with_guard`s
   concurrently): in `Owned`, the compiler rejects it. In `Shared`,
   it deadlocks -- same as today. I-9's `debug_assert!` catches this
   in debug builds.
4. Reentrant bus access from a `Device::tick` body (I-9 violation):
   in `Owned`, compile error. In `Shared` with `debug_assertions`,
   debug-assert fires. In `Shared` release, it deadlocks -- same as
   today and documented.
5. Difftest build (`--features difftest`): unchanged. The `AtomicBool`
   field on `Bus` remains, and `with` / `with_guard` / `with_ref`
   give exactly the same access pattern as `lock().unwrap()`.



[**State Transition**]

- Config::`num_harts == 1` -> `BusOwner::into_handles(1)` returns
  `vec![BusHandle::Owned(Box::new(bus))]`. `CPU` takes the handle;
  `Core` receives borrows via `with` / `with_guard`.
- Config::`num_harts >= 2` -> `BusOwner::into_handles(n)` returns
  `vec![BusHandle::Shared(Arc::new(Mutex::new(bus))); n]`. `CPU` and
  each `Core` take one handle.
- Runtime: variant never changes. No dynamic upgrade/downgrade.
- Reset (`CPU::reset`): opens one `with_guard`, calls
  `reset_devices()` + `clear_reservations()`, drops guard, then
  resets cores. Same ordering as `cpu/mod.rs:141-148`; already a
  single scope today (R-006).



### Implementation Plan

[**Phase 1 -- Introduce `BusHandle` / `BusOwner`**]

- 1a. Create `xcore/src/device/bus_handle.rs` with `BusHandle`,
  `BusGuard`, `ReadBusGuard`, `BusOwner`, the `with` / `with_guard`
  / `with_ref` / `with_read_guard` API, and the I-9 reentry guard.
  All methods are safe, no `unsafe`.
- 1b. Unit tests in the same file: round-trip `with` on `Owned` and
  `Shared`, `with_guard` DerefMut, `with_ref` immutability, reentry
  rejection in debug, `BusOwner::into_handles(1)` yields exactly
  one `Owned`, `into_handles(2)` yields two `Shared` handles
  sharing one `Arc` (`Arc::ptr_eq` check).
- 1c. `make fmt && make clippy && cargo test -p xcore bus_handle`.
  No integration yet.

[**Phase 2 -- Migrate call sites**]

**Complete migration table** (every site from `rg 'bus\.lock\(\)'
xemu/xcore/src`):

| File : Line                              | Today                                       | Pattern         | Notes                                                                                 |
|------------------------------------------|---------------------------------------------|-----------------|---------------------------------------------------------------------------------------|
| `cpu/mod.rs:101`                         | `bus.lock().unwrap().num_harts()`           | `with_ref`      | Inside `CPU::with_machine_config`; one read.                                          |
| `cpu/mod.rs:126` `CPU::bus()`            | `self.bus.lock().unwrap()`                  | `with_read_guard` | Public accessor; split per R-007; keeps `&self`.                                    |
| `cpu/mod.rs:142-145` reset               | already one scope                           | `with_guard`    | **No change in shape**; maps 1:1 (R-006).                                             |
| `cpu/mod.rs:168` image load              | `bus.lock().unwrap().load_ram(...)`         | `with`          | Single statement.                                                                     |
| `cpu/mod.rs:199` image load (reset path) | `bus.lock().unwrap().load_ram(...)`         | `with`          | Single statement.                                                                     |
| `cpu/mod.rs:214` step tick               | `bus.lock().unwrap().tick()`                | `with`          | Hottest single-hart path; biggest delta expected.                                     |
| `cpu/mod.rs:293` replace_device          | `bus.lock().unwrap().replace_device(...)`   | `with`          | Wiring path; infrequent.                                                               |
| `cpu/mod.rs:323` take_mmio_flag          | `bus.lock().unwrap().take_mmio_flag()`      | `with`          | Difftest path; one-shot; reached via `CPU::bus_mut`.                                  |
| `cpu/mod.rs:385-387` CPU construction    | `Arc::new(Mutex::new(bus))`                 | new             | Switch to `BusOwner::new(bus).into_handles(num_harts)`.                               |
| `arch/riscv/cpu.rs:207` `mtime()`        | `bus.lock().unwrap().mtime()`               | `with_ref`      | One read.                                                                             |
| `arch/riscv/cpu/debug.rs:93-99` `read_memory` | `bus.lock().unwrap().read_ram(...)`    | `with_ref`      | Debug inspector; read-only.                                                           |
| `arch/riscv/cpu/debug.rs:103-109` `fetch_inst` | two reads under one guard             | `with_ref`      | Debug path; use one `with_ref` closure spanning both reads to keep a single scope.    |
| `arch/riscv/cpu/mm.rs:258` `access_bus`  | `lock().unwrap()` held across translate     | `with_guard`    | Unchanged width; already widest scope.                                                |
| `arch/riscv/cpu/mm.rs:265-272` `checked_read` | two locks (translate, read)            | `with_guard`    | **Merges two locks into one.** Shortens Shared-arm. V-UT-5 covers.                    |
| `arch/riscv/cpu/mm.rs:276-279` `checked_write` | two locks (translate, write)           | `with_guard`    | Same merge.                                                                           |
| `arch/riscv/cpu/inst/base.rs:75`         | `bus.lock().unwrap().clear_reservation(id)` | `with`          | Per-store (non-AMO) hot path. **Not fused** into `checked_write`'s guard -- T-4.      |
| `arch/riscv/cpu/inst/compressed.rs:573`  | `bus.lock().unwrap().read(...)`             | `with_ref`      | Test helper in compressed-form unit tests; read-only.                                 |
| `arch/riscv/cpu/inst/float.rs:1085`      | `bus.lock().unwrap().read(...)`             | `with_ref`      | Test helper in F/D unit tests; read-only.                                             |
| `arch/riscv/cpu/inst/atomic.rs:30`       | AMO clear_reservation                       | `with`          | One-shot.                                                                             |
| `arch/riscv/cpu/inst/atomic.rs:45`       | AMO clear_reservation (64-bit)              | `with`          | One-shot.                                                                             |
| `arch/riscv/cpu/inst/atomic.rs:57`       | LR reserve                                  | `with`          | One-shot.                                                                             |
| `arch/riscv/cpu/inst/atomic.rs:65-69`    | SC triple-lock zone                         | `with_guard`    | Must stay atomic (I-4).                                                               |
| `arch/riscv/cpu/inst/atomic.rs:81`       | LR reserve (64-bit)                         | `with`          | One-shot.                                                                             |
| `arch/riscv/cpu/inst/atomic.rs:89-94`    | SC triple-lock zone (64-bit)                | `with_guard`    | Must stay atomic (I-4).                                                               |
| `arch/riscv/cpu/inst/atomic.rs:195,200,204,212` | AMO test helpers (`#[cfg(test)]`)  | `with` / `with_ref` | Four test-only helpers; mechanical migration.                                   |
| `cpu/mod.rs` + `device/bus.rs` tests     | `Arc::new(Mutex::new(bus))` setup           | update          | Replace with `BusOwner::new(bus).into_handles(1)` and index into the vec.             |

Note (R-001): `arch/riscv/cpu/privileged.rs` does NOT exist.
`arch/riscv/cpu/inst/privileged.rs` DOES exist; `rg bus\.lock\(\)`
returns zero matches in it. Trap handling (`cpu/trap/handler.rs`)
also has no `bus.lock()` calls. The 00_PLAN row is removed entirely.

After each logical group of migrations:
`make fmt && make clippy && make test`.
Checkpoint: after the last group, `rg 'bus\.lock\(\)' xemu/xcore/src`
returns zero matches.

[**Phase 3 -- Verify, measure, document**]

- 3a. Boot gate: `make run` (microbench) completes; `make linux`
  boots to prompt; `make linux-2hart` boots to prompt.
- 3b. Perf sampling: `scripts/perf/sample.sh` with DEBUG=n on
  dhrystone, coremark, microbench; render via
  `scripts/perf/render.py` into
  `docs/perf/<post-P1-date>/data/` + `REPORT.md`.
- 3c. Compare against `docs/perf/2026-04-14/data/bench.csv`:
  report Before / After wall-clock, Before / After
  `pthread_mutex_*` self-time.
- 3d. `linux-2hart` boot-to-prompt time sampled three runs each
  pre / post; confirm +/-5 %.
- 3e. Run `cargo asm -p xcore xcore::cpu::CPU::step --rust`
  (requires `cargo-show-asm` / `cargo-asm`); assert no
  `lock cmpxchg`, `xchg`, or `pthread_mutex_` symbol appears in the
  inlined Owned fast path. Capture the disassembly snippet in the
  perf report as V-IT-8 evidence.
- 3f. (**Nice-to-have, NOT gated per R-009**.) Add
  `xcore/benches/bus_step.rs` criterion bench running a 1 M NOP
  loop through `CPU::step`; capture pre/post numbers in the perf
  report when present.
- 3g. Run V-UT-5, V-UT-6, and V-IT-7 (new tests from R-002 /
  R-004); all must pass.
- 3h. Update `docs/perf/PERF_DEV.md` P1 row to "Done" with the
  measured numbers.



## Trade-offs {ask reviewer for advice}

- T-1: **Ownership model -- `BusHandle` enum (option 1, recommended)
  vs. `Arc<UnsafeCell<Bus>>` behind a single-hart feature flag
  (option 2) vs. trait-object `BusAccess` split (option 3).**

  Option 1 (recommended): safe Rust, one added enum, branch
  predictor trivially collapses the hot path because the variant
  never changes per-run. Cost: one `match` per bus access in both
  arms; LLVM typically hoists the discriminant check out of tight
  loops (verified by Phase 3 step 3e `cargo asm`). No `unsafe`
  invariant cost.

  Option 2: `Arc<UnsafeCell<Bus>>` + `#[cfg(feature = "single_hart")]`
  skips the discriminant entirely. Faster on paper but buys a
  permanent `unsafe` invariant ("no hart ever holds a concurrent
  borrow"), forces a build-flag matrix on every CI config, and
  makes `linux-2hart` a separate binary. Rejected per NG-4.

  Option 3: trait `BusAccess` with `BorrowedBus` / `SharedBus`
  implementers. Vtable indirection re-introduces roughly the cost
  we're removing (branch target buffer miss + indirect call).
  Rejected.

  Precedent (per R-008, sources verified via web):
  - rvemu: https://github.com/d0iasm/rvemu ; book
    https://book.rvemu.app/hardware-components/02-memory.html --
    owned `Bus` by value; single-hart design.
  - rv8 (CARRV 2017):
    https://carrv.github.io/2017/papers/clark-rv8-carrv2017.pdf --
    owned memory through the CPU state.
  - Rare: https://github.com/d0iasm/rare -- owned bus.
  - riscv-rust: https://github.com/takahirox/riscv-rust -- owned
    memory.
  - rrs: https://github.com/GregAC/rrs -- owned memory.
  - QEMU MTTCG design:
    https://www.qemu.org/docs/master/devel/multi-thread-tcg.html --
    started with one BQL, actively pushing down toward IO-memory
    only.
  - LWN BQL push-down: https://lwn.net/Articles/697265/ .
  `BusHandle` is xemu's version of the QEMU BQL push-down, keeping
  a single-BQL compatibility variant (`Shared`) until MTTCG lands.

- T-2: **`CPU::bus()` return type -- uniform guard vs. read/write
  split (reviewer preferred option C in TR-2).**

  Option A (00_PLAN original): uniform `BusGuard<'_>`; requires
  `&mut self`. Breaks external `&self` callers.
  Option B: return `&mut Bus` in `Owned`, `MutexGuard` in `Shared`.
  Bifurcates the API; worse for maintainers.
  Option C (reviewer recommended, adopted): split into
  `CPU::bus(&self) -> ReadBusGuard<'_>` (read-only) and
  `CPU::bus_mut(&mut self) -> BusGuard<'_>` (mutable). Mirrors
  `HashMap::get` vs `get_mut`. Keeps source compatibility for the
  two test-only external callers. Surfaces intent.
  **Adopted: Option C.** External-caller audit above confirms the
  split is a pure addition.

- T-3: **`Shared` arm mutex choice.**
  Option A: keep `std::sync::Mutex`.
  Option B: swap to `parking_lot::Mutex`
  (https://github.com/Amanieu/parking_lot) -- roughly 1.5x
  uncontended speedup per parking_lot's own benchmark, no poison
  semantics.
  **Adopted: A**, per 00_REVIEW TR-3. The `Shared` arm only runs
  under `linux-2hart`, which is not the P1 target; mutex swap is a
  follow-up if `linux-2hart` becomes a hotspot.

- T-4: **Merging `checked_read` / `checked_write`'s two locks into
  one; NOT fusing `store_op`'s `clear_reservation` into
  `checked_write`'s guard.**
  Decision 4a: merge `checked_read` / `checked_write` (adopted).
  Translate-then-access is already one scope on `access_bus`;
  merging the two small scopes matches that width -- no widening.
  Future MTTCG must re-split; documented in P1 commit message.
  Decision 4b: do NOT fuse `store_op`'s post-store
  `clear_reservation(self.id)` (same-hart clear) into
  `checked_write`'s guard. Fusing would widen the Shared-arm scope
  by one additional operation and change per-store semantics from
  "store then clear" to "store-and-clear atomically". That is a
  semantic change, not a perf one, and R-001's incidental
  suggestion to consider fusing is declined on those grounds.
  Cost in the Owned arm: one extra discriminant check per store
  (negligible -- the store itself dominates). V-UT-5 asserts the
  Shared-arm acquire count is not accidentally widened.



## Validation {test design}

[**Unit Tests**]

- V-UT-1: `bus_handle` tests (new file): `Owned` and `Shared`
  round-trip `with(|b| b.store/read)`; `with_guard` allows
  multi-step reservation check; `with_ref` is read-only;
  `BusOwner::into_handles(1)` yields exactly one `Owned`;
  `into_handles(2)` yields two `Shared` handles sharing one `Arc`
  (`Arc::ptr_eq` check).
- V-UT-2: `Bus` existing tests (13 in `device/bus.rs`) unchanged.
- V-UT-3: `CPU` existing tests (13 in `cpu/mod.rs`) run against the
  `Owned` variant (tests are single-hart by default); add one new
  test `cpu_run_with_shared_handle` that constructs a `Shared`
  handle manually and asserts identical step results on a
  5-instruction golden trace.
- V-UT-4: `RVCore` tests (5 in `arch/riscv/cpu.rs`) plus atomic
  tests (20+ in `atomic.rs`): all run green on `Owned`. Add one
  explicit `sc_w_atomicity_under_owned_bus` test that issues
  `lr -> store-that-invalidates -> sc` on a single hart and asserts
  `sc` returns failure -- proves the `with_guard` scope didn't
  collapse the triple-lock zone.
- **V-UT-5 (new, R-002)**: `bus_handle_shared_lock_width` in
  `xcore/src/device/bus_handle.rs` tests module. Wrap a `Shared`
  `BusHandle` with a test-only counter (increment on every
  `lock()` call; `#[cfg(test)]`-only) and run a fixed
  64-instruction golden trace (dhrystone's inner loop, 1
  iteration). Capture the pre-P1 acquire-count as a frozen
  constant (`const PRE_P1_LOCK_ACQUIRES: usize = N;`); assert
  post-P1 count equals or beats `N`. Fails if a reviewer widens a
  critical section.
- **V-UT-6 (new, R-004)**: `bus_handle_rejects_reentry` test uses a
  stub `Device` whose `tick` re-enters the owning `BusHandle`.
  `Owned` path fails to compile (compile_fail doctest); `Shared`
  path panics under `debug_assertions` via the I-9 `Cell<bool>`
  guard.

[**Integration Tests**]

- V-IT-1: `arch_isolation.rs` seam test runs unchanged.
- V-IT-2: `make run` (default microbench / direct image) boots and
  exits 0.
- V-IT-3: `make linux` boots to `/ # ` prompt, runs
  `echo hello; exit`.
- V-IT-4: `make linux-2hart` boots to prompt and both harts appear
  in `cat /proc/cpuinfo`. Exercises the `Shared` arm end-to-end.
- V-IT-5: `make debian` boots to shell prompt.
- V-IT-6: `make xv6` boots (if wired up locally; skipped if not --
  declared so reviewer can decide).
- **V-IT-7 (new, R-002)**: `xemu/xcore/tests/shared_bus_torture.rs`
  -- build a 2-hart `CPU`, run a hand-written LR/SC torture for
  10 000 iterations where hart 0 does `lr`, hart 1 stores in the
  reserved granule, and hart 0's `sc` must fail on every round.
  Must complete within a 50 ms Rust-test budget. Fails fast if the
  `Shared` arm deadlocks or widens.
- **V-IT-8 (new, R-003)**: `cargo asm -p xcore
  xcore::cpu::CPU::step --rust` run as part of Phase 3; asserts no
  `lock cmpxchg`, `xchg`, or `pthread_mutex_*` symbol in the Owned
  inlined body. Output captured in perf report.

[**Failure / Robustness Validation**]

- V-F-1: `BusOwner::into_handles(2)` returns two `Shared` handles
  whose inner `Arc`s are `Arc::ptr_eq` (same mutex). Proves the
  Shared-arm sharing invariant.
- V-F-2: Reset mid-execution: all reservations cleared, devices
  reset, subsequent step succeeds. Existing test passes on `Owned`.
- V-F-3: Difftest build (`cargo test --features difftest -p xcore`)
  green -- confirms the `AtomicBool` side-channel still fires and
  that `CPU::bus_mut` works for the difftest harness.

[**Edge Case Validation**]

- V-E-1: Zero-instruction run (`CPU::run(0)`) -- tick must **not**
  fire. Covers `Owned` arm.
- V-E-2: Single-hart LR/SC on the same hart's own reservation
  succeeds; single-hart LR followed by a peer-less store on a
  different address does **not** invalidate the reservation.
- V-E-3: Two-hart LR/SC: hart 0 `lr`, hart 1 store, hart 0 `sc`
  fails. Covered by V-IT-7 at scale.
- V-E-4: Boot-then-reset loop (load image, reset, re-run) -- catches
  any leaked `BusHandle` ownership across the cycle.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (zero mutex on single-hart hot path)                     | V-IT-2 + V-IT-8 (`cargo asm` no CAS) + Phase 3 perf sample: `pthread_mutex_*` self-time < 5 % on dhrystone/coremark/microbench vs. ~33-40 % baseline. |
| G-2 (>=15 % floor, 20-30 % expected, 35 % ceiling)           | V-IT-2 + Phase 3 perf sample compared line-by-line to `docs/perf/2026-04-14/data/bench.csv`. Floor 15 % required; ceiling bucket-math documented in G-2 text. |
| G-3 (multi-hart unchanged)                                   | V-IT-4, V-E-3, V-IT-7. Delta < 5 % on `linux-2hart` boot time over 3 runs; V-UT-5 asserts acquire count not increased. |
| G-4 (LR/SC atomicity preserved)                              | V-UT-4, V-E-3, V-IT-7. |
| G-5 (public APIs stable: `CPU::bus` still `&self`)           | V-UT-3 uses `CPU::bus()` unchanged; external-caller audit in API Surface. |
| C-1 (no `unsafe`)                                            | `make clippy` + diff grep; 0 new `unsafe`. |
| C-2 (device traits unchanged)                                | V-UT-2 (all `device/bus.rs` tests green). |
| C-3 (no benchmark-specific tricks)                           | Code review: `Owned` selector is `num_harts == 1`. |
| C-4 (linux-2hart +/-5 %)                                     | V-IT-4 sampled 3x, reported in `docs/perf/<post-P1-date>/REPORT.md`. |
| C-5 (fmt/clippy/run/test clean)                              | CI and `make` gate on every Phase 2 commit. |
| C-6 (DEBUG=n benchmarks)                                     | `scripts/perf/sample.sh` env assertion. |
| C-7 (make-based launches)                                    | Perf report records exact `make` targets used. |
| C-8 (no widened Shared critical section)                     | V-UT-5 counter test + V-IT-4 timing +/-5 %. |
| I-9 (no reentrant bus access)                                | V-UT-6 (compile-fail + debug-assert). |

---

## Exit Gate (summary, per R-003)

All five conditions required:
1. `pthread_mutex_*` self-time < 5 % on dhrystone / coremark /
   microbench (from G-1).
2. Wall-clock reduction **>= 15 %** vs. `2026-04-14/bench.csv`
   (20-30 % expected, not required).
3. `linux-2hart` boot-to-prompt within +/-5 % of main (C-4).
4. V-UT-5, V-UT-6, V-IT-7 pass (R-002, R-004).
5. V-IT-8 `cargo asm CPU::step` shows no CAS / `pthread_mutex_*`
   symbol in the Owned fast path (R-003).

Criterion microbench (Phase 3 step 3f) is **not** part of the gate.
