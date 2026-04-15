# `directIrq` PLAN `01`

> Status: Revised
> Feature: `directIrq`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `00_MASTER.md`

---

## Summary

Round 01 of `directIrq`. Delta-revises `00_PLAN.md` against `00_REVIEW.md`
(1 CRITICAL + 3 HIGH + 6 non-blocking) and against `00_MASTER.md`
(M-001 + M-002, both `MUST`).

Unchanged from round 00: the goal of closing MANUAL_REVIEW #5 (devices
signal PLIC directly) + #6 (asynchronous delivery); the arch-neutral
`IrqLine` handle; `PlicSignals` as the PLIC-side signal plane; the
Gateway / Core / Source substrate from `plicGateway`; the three-phase
rollout.

Changed under M-001 ("handling still seems synchronous; research Rust
async") — round 00 argued event-driven from the device side only: a
raise immediately sets an atomic bit, but the PLIC still polled that
bit at every slow bus tick. Round 01 reframes the posture with a direct
comparison against real `async fn`/`.await` semantics (Rust has no
built-in executor; a true `async` path needs tokio/embassy-class
infrastructure that NG-2 forbids), commits to the event-primitive
posture that executors themselves are built on (Acquire/Release atomics
+ happens-before), and tightens the PLIC-side so that evaluation is not
a free-running scan but a conditional drain gated on an epoch flag —
the PLIC re-evaluates only when a raise has actually happened since
the last drain, so PLIC ticks with no pending work become a single
`Acquire` swap. This keeps the runtime single-threaded (NG-2) while
giving a defensible "event-driven, not polled" invariant. Cited sources:
`os.phil-opp.com/async-await`, `rust-lang.github.io/async-book`,
`without.boats/blog/wakers-i`,
`airbus-seclab.github.io/qemu_blog/interrupts.html`,
`github.com/qemu/qemu/blob/master/include/hw/irq.h`.

Changed under M-002 ("handle async cautiously") — round 01 explicitly
rejects posture P-D (`async fn raise + .await`) and posture P-B (mpsc
channel per source) and posture P-C (raise-acquires-PLIC-mutex); the
rationale for each rejection is recorded in the new Async Posture
section and in Response Matrix row M-002. Posture P-A-refined is
adopted.

Changed under the REVIEW blockers:

- R-001 CRITICAL (dispatch path): `Plic::tick` is folded into
  `Device::tick` (option 1 from the review). `Bus::tick` calls
  `plic.tick()` via the normal `dyn Device` vtable inside the
  slow-path loop; Phase 1's legacy bitmap fold and
  `plic.notify(bitmap)` are reordered so that adopter devices are
  inert in the bitmap fold (they return `false` from `irq_line`),
  which eliminates the `sample(stale) -> sample(fresh)` within-tick
  reordering flagged by R-003.
- R-002 HIGH (`NUM_SRC` coupling): new invariant I-D12, constraint
  C-12, and a `const _: () = assert!(NUM_SRC <= 32);` co-located
  with `PlicSignals`.
- R-003 HIGH (double-path coexistence): resolved by R-001's
  ordering fix + dropping adopter `irq_line` override; new
  invariant I-D13 makes single-path-per-source binding inside
  Phase 1.
- R-004 HIGH (reset Arc identity): new invariant I-D8a and new
  unit test V-UT-10, co-located in the Data Structure section.

Non-blocking adjustments (R-005..R-010) and the M-001 async posture
drive four new invariants (I-D12..I-D15), two new constraints (C-12,
C-13), three new validations (V-UT-10, V-IT-6, V-IT-7), one tightened
phase-1 invariant (I-D11-revised), and an expanded V-E-4 race
enumeration.

## Log

[**Feature Introduce**]

No new features relative to 00_PLAN's three-phase programme. This plan
revises semantics, invariants, and validations only.

[**Review Adjustments**]

- R-001 (CRITICAL) resolved — committed to Device-trait override
  dispatch. See §API Surface §Dispatch, Response Matrix row R-001,
  I-D10-revised.
- R-002 (HIGH) resolved — I-D12 + C-12 + `const_assert!`. See Data
  Structure.
- R-003 (HIGH) resolved — I-D11-revised + I-D13 + reordered Phase-1
  step 7. Adopter devices' `Device::irq_line` returns `false` from
  Phase 1 onward.
- R-004 (HIGH) resolved — I-D8a + V-UT-10; Data Structure paragraph
  at `Plic.signals` now pins the in-place reset contract.
- R-005 (MEDIUM, Accept) — V-E-6 added: Plic-boundary pulse end-to-end.
- R-006 (MEDIUM, Accept) — Phase 3 step 4 now enumerates every
  `Bus::add_mmio` call site with file:line citations.
- R-007 (MEDIUM, Accept) — Phase 1 step 4 commits to the shim branch:
  `Gateway::sample(level)` stays as `#[inline] sample_with(level, false)`
  through Phase 2, deleted in Phase 3.
- R-008 (LOW, Accept) — C-2 appended with the `IrqSignalPlane`
  arch-neutral residency rationale.
- R-009 (LOW, Accept) — I-D11 reworded; see §Spec §Invariants.
- R-010 (LOW, Accept) — `PlicSignals::drain` order swapped:
  `edge_latch.swap(0, AcqRel)` before `level.load(Acquire)`.
  V-E-4 expanded to enumerate the three interleavings with expected
  outcomes.

[**Master Compliance**]

- M-001 (MUST, Applied) — Async posture audit and redesign. The
  new §Async Posture section (after §Architecture) enumerates four
  candidate interpretations (P-A / P-B / P-C / P-D), adopts a
  refined P-A ("signal plane + epoch-gated drain + hb-via-atomics"),
  rejects the other three with explicit reasoning, and honours the
  "research Rust async" clause by citing the Rust async-book,
  phil-opp's "Async/Await" OS chapter, the "what does a waker do" post
  from without.boats, the QEMU IRQ model from
  airbus-seclab.github.io, and QEMU's `hw/irq.h` header. The
  core argument: a `Waker` is the user-space abstraction for the
  atomic primitive we already use; an `async fn raise()` would
  require an executor we do not have and would forbid (NG-2). New
  invariants I-D14 (event-driven — no per-source poll when nothing
  changed) and I-D15 (epoch-gated drain order) encode the posture so
  the next reviewer can check it mechanically. New validation V-IT-7
  asserts the event-driven property: a run with no raises must do
  zero gateway-sample work per `Plic::tick`.
- M-002 (MUST, Applied) — Caution. Captured as:
  - The rejection reasoning for P-B/P-C/P-D is recorded so future
    iterations inherit the decision trail.
  - No new crate dependencies (`tokio`, `async-std`, `futures`,
    `crossbeam_channel`) are introduced; the existing `std::sync::atomic`
    primitives suffice.
  - `loom`-based interleaving proof is called out as a named
    follow-up (non-blocking, OQ-5), not smuggled into this plan.
  - Every atomic operation has an explicit memory ordering
    (`Acquire`, `Release`, `AcqRel`, or `Relaxed`) justified inline
    in §Concurrency Posture; "atomics" alone is forbidden wording.

### Changes from Previous Round

[**Added**]

- §Async Posture section (post-§Architecture).
- §Concurrency Posture section (post-§Data Structure).
- Invariants I-D8a (in-place reset), I-D12 (`NUM_SRC <= 32` pin),
  I-D13 (Phase-1 single-path-per-source), I-D14 (event-driven),
  I-D15 (epoch-gated drain order).
- Constraint C-12 (`NUM_SRC <= 32`, lockstep widening rule),
  C-13 (no new async-runtime dep).
- Validations V-UT-10 (reset preserves Arc), V-UT-11 (epoch-gate
  no-op), V-UT-12 (Level-FSM `sample(a); sample(b)` equivalence —
  R-003 FSM property documentation), V-IT-6 (Plic-boundary pulse
  end-to-end), V-IT-7 (no-raise -> no sample work), V-E-6 (pulse
  end-to-end via `IrqLine`).
- R-001-chosen dispatch: `Plic` implements `Device::tick`; new field
  `signals.pending_raises: AtomicBool` for I-D14.
- §Phase 3 call-site table for `Bus::add_mmio` (R-006).

[**Changed**]

- I-D9 and I-D10 (round 00) -> I-D9-revised, I-D10-revised: atomic
  memory orderings are spelled out per-op (not just labelled
  "Acquire/Release"), and I-D10 now says "`Plic::tick` is called
  only from `Bus::tick` via `Device::tick`"; no inherent-path,
  no downcast.
- I-D11 (round 00) -> I-D11-revised: forbids a device from holding
  an `IrqLine` and returning `true` from `Device::irq_line`
  simultaneously. Phase-1 adopter devices must override
  `Device::irq_line -> false`.
- `PlicSignals::drain` order swapped (R-010): edge first, level
  second.
- Phase 1 step 7: adopter devices are inert in the bitmap fold. The
  bitmap fold survives Phase 1 only for non-adopter devices.

[**Removed**]

- No code removals relative to 00_PLAN's Phase 3 plan; the Phase-3
  retirements are unchanged.
- Round 00's implicit "both paths live, union is monotonic" claim is
  retracted — replaced by the I-D13 single-path-per-source rule.

[**Unresolved**]

- OQ-1 (round 00) — `IrqLine: Clone`. Position unchanged: yes,
  `Clone`, with I-D7 documenting coalesce. No reviewer advice to the
  contrary in 00_REVIEW.
- OQ-2 (round 00) — resolved by T-4 Option A + R-010 drain-order
  fix + V-IT-6 + V-E-6.
- OQ-3 (round 00) — superseded by I-D14, I-D15.
- OQ-4 (round 00) — resolved by R-008 + C-2 amendment.
- OQ-5 (new) — is `loom` interleaving validation worth the
  build-time cost? Position: no; noted as follow-up.
- OQ-6 (new) — should the raise-epoch be `AtomicBool` or a `u32`
  counter? Position: `AtomicBool` with `Release`/`Acquire` ordering
  suffices at this cadence; a counter would only help diagnose
  "lost wakeups" that `Release`/`Acquire` already prove impossible.
  Kept open for 01_REVIEW advice.

### Response Matrix

| Source | ID | Severity | Decision | Resolution |
|--------|----|----------|----------|------------|
| Master | M-001 | MUST | Applied | New §Async Posture; refined P-A adopted; P-B/C/D rejected inline; I-D14 + I-D15 encode the event-driven invariants; V-IT-7 validates "no-raise -> no-sample" empirically. Web sources cited in §Async Posture. |
| Master | M-002 | MUST | Applied | No async-runtime dep (C-13). Explicit rejection of P-B/C/D with reasoning. Every atomic operation has an explicit memory ordering in §Concurrency Posture. `loom` deferred (OQ-5). |
| Master (inh.) | archModule 00-M-002 | — | Honored | New module `src/device/irq.rs` is arch-neutral; PLIC factory remains under `src/arch/riscv/device/intc/plic/`. |
| Master (inh.) | archLayout 01-M-004 | — | Honored | `IrqLine` is the arch-neutral seam; `PlicSignals` lives under arch tree. |
| Master (inh.) | plicGateway I-9 | — | Re-examined | Superseded by I-D9-revised + I-D14. |
| Review | R-001 | CRITICAL | Accepted | `Plic::tick` is overridden through `Device::tick`. `Bus::tick` path is the only caller. See §API Surface §Dispatch. |
| Review | R-002 | HIGH | Accepted | I-D12 + C-12 + `const _: () = assert!(NUM_SRC <= 32);` in `signals.rs`. |
| Review | R-003 | HIGH | Accepted | I-D11-revised + I-D13. Adopter devices drop their `Device::irq_line -> true` override. V-UT-12 retained as FSM-property documentation. |
| Review | R-004 | HIGH | Accepted | I-D8a co-located with `Plic.signals` Data Structure paragraph; V-UT-10 added. |
| Review | R-005 | MEDIUM | Accepted | V-IT-6 and V-E-6 added (Plic-boundary pulse end-to-end). |
| Review | R-006 | MEDIUM | Accepted | Phase 3 step 4 enumerates every call site. |
| Review | R-007 | MEDIUM | Accepted | Shim branch selected. `Gateway::sample(level)` is `#[inline] sample_with(level, false)` through Phase 2. |
| Review | R-008 | LOW | Accepted | C-2 amendment. |
| Review | R-009 | LOW | Accepted | I-D11 reworded. |
| Review | R-010 | LOW | Accepted | `drain` order swapped. V-E-4 expanded. |

> Rules: Every prior CRITICAL/HIGH finding and every MASTER directive
> appears above. Rejections would carry explicit reasoning; none in
> this round.

---

## Spec

### Spec References

Unchanged from 00_PLAN. Additions in round 01:

- Rust async-book §2 "Executors"
  (`rust-lang.github.io/async-book/02_execution/04_executor.html`):
  "Rust's Futures are lazy; an executor is required to poll them to
  completion." We use this to reject posture P-D.
- phil-opp "Async/Await" §Waker (`os.phil-opp.com/async-await/`):
  a Waker is created by the executor and used by the task to signal
  readiness. We use this to argue that our atomic signal plane is
  the waker primitive minus the scheduler layer.
- without.boats "The Waker API I" (`without.boats/blog/wakers-i/`):
  the executor and event sources coordinate using the Waker API.
  Confirms that a wake notification can be a single atomic write —
  which is exactly what `IrqLine::raise` is.
- QEMU `include/hw/irq.h`
  (`github.com/qemu/qemu/blob/master/include/hw/irq.h`) + airbus-seclab
  QEMU-internals
  (`airbus-seclab.github.io/qemu_blog/interrupts.html`):
  `qemu_set_irq` is a synchronous callback — the device's raise
  directly invokes the interrupt-controller handler on the same
  thread. We use this to ground posture P-A against prior art.

[**Goals**]

Unchanged from 00_PLAN G-1..G-6. Added:

- G-7 The PLIC side of interrupt delivery is event-driven (no
  per-source scan in the common no-raise path). A `Plic::tick` with
  no raises-since-last-drain performs exactly one `Acquire` swap of
  `pending_raises` and returns. Testable as V-UT-11 + V-IT-7.

[**Non-Goals**]

Unchanged from 00_PLAN NG-1..NG-7. Added:

- NG-8 No introduction of any async runtime (`tokio`, `async-std`,
  `embassy`) or channel crate (`crossbeam_channel`, `flume`). The
  std primitives suffice. Enforced by C-13.
- NG-9 No `async fn` in any new code for this feature. The one
  cross-thread signal path (UART reader -> PLIC) is a direct
  `IrqLine::raise` call, not an awaited future.
- NG-10 No `Waker`/`Future` impl. This feature uses the same atomic
  primitive that `Waker` is built on without instantiating a
  `Waker` type.

[**Architecture**]

```
        +------------------------------------------------------+
        |                       src/device/                    |
        |                                                      |
        |  uart.rs    virtio_blk.rs    ...                     |
        |    |            |                                    |
        |    | IrqLine    | IrqLine    (arch-neutral handle)   |
        |    v            v                                    |
        |  +---------------------------------------------+     |
        |  |  src/device/irq.rs — IrqLine { plane, src } |     |
        |  |     raise()  -- fetch_or(level)             |     |
        |  |                 | set(pending_raises)       |     |
        |  |     lower()  -- fetch_and(!level)           |     |
        |  |                 | set(pending_raises)       |     |
        |  |     pulse()  -- fetch_or(level)             |     |
        |  |                 | fetch_or(edge_latch)      |     |
        |  |                 | set(pending_raises)       |     |
        |  +---------------------------------------------+     |
        |                    |                                 |
        +--------------------|---------------------------------+
                             | Arc<dyn IrqSignalPlane>
                             v
        +------------------------------------------------------+
        |           src/arch/riscv/device/intc/plic/           |
        |                                                      |
        |  signals.rs — PlicSignals {                          |
        |                 level: AtomicU32,                    |
        |                 edge_latch: AtomicU32,               |
        |                 pending_raises: AtomicBool,  // NEW  |
        |               }                                      |
        |               const _: () = assert!(NUM_SRC <= 32);  |
        |                                                      |
        |  mod.rs — impl Device for Plic {                     |
        |    fn tick(&mut self) {                              |
        |      let ev = self.signals.take_epoch();             |
        |      if !ev && !self.needs_reevaluate { return; }    |
        |      self.needs_reevaluate = false;                  |
        |      let (edg, lvl) = self.signals.drain();          |
        |      for s in 1..NUM_SRC {                           |
        |        match gateway[s].sample_with(                 |
        |               lvl&bit != 0, edg&bit != 0) {          |
        |          Pend     => core.set_pending(s),            |
        |          Clear    => core.clear_pending(s),          |
        |          NoChange => {},                             |
        |        }                                             |
        |      }                                               |
        |      self.core.evaluate();                           |
        |    }                                                 |
        |  }                                                   |
        +------------------------------------------------------+
```

Dispatch (R-001 resolution):

- `Plic::tick` is not a new inherent method; it is the
  `Device::tick` implementation on `Plic`. `Bus::tick`'s existing
  slow-path `r.dev.tick()` loop at `bus.rs:233` already reaches
  `Plic` through `Box<dyn Device>`, so the new drain rides that
  vtable without any bus-side restructuring.
- Phase 1 keeps the legacy bitmap-fold + `plic.notify(bitmap)` but
  adopter devices return `false` from `Device::irq_line`, so their
  source bits are always zero in the bitmap. No sample clobber
  between paths (I-D13).
- Phase 2 deletes the bitmap fold + `notify` call. `Plic::tick`
  remains the sole entry.
- Phase 3 deletes `Plic::notify` and shrinks the trait surface.

### Async Posture

Mandated by M-001. Four candidate postures are considered.

P-A (refined, adopted) — atomic signal plane + epoch-gated drain +
release/acquire happens-before.

Device thread on raise:
```
// IrqSignalPlane::raise(src)
plane.level.fetch_or(1 << src, Release);
plane.pending_raises.store(true, Release);
```

Bus tick thread on drain (inside `<Plic as Device>::tick`):
```
let event = plane.pending_raises.swap(false, Acquire);
if !event && !self.needs_reevaluate { return; }
let (edg, lvl) = plane.drain();     // edge first, level second (I-D15)
for s in 1..NUM_SRC { gateway[s].sample_with(level_bit, edge_bit); }
core.evaluate();
```

Claim-pend from MMIO (already on bus tick thread):
```
// Plic::read(claim) / Plic::write(complete) — sets
// self.needs_reevaluate = true; next tick's guard passes.
```

Why this satisfies M-001 "event-driven, not polled":

- A `Plic::tick` with no raises-since-last-drain performs exactly
  one `swap` on a single `AtomicBool` and returns. There is no
  per-source scan, no gateway call, no core evaluate (V-UT-11 and
  V-IT-7 assert this mechanically).
- The device side never polls anything: a raise is one `fetch_or`
  plus one `store`, both `Release`. No loop, no retry, no backoff.
- The happens-before edge from raise to drain is established by the
  `Release` on `pending_raises` on the raise side, paired with the
  `Acquire` on the `swap` on the drain side (Rust reference: memory
  model §Atomics). This is the same primitive `Waker::wake` is
  built on, per without.boats.

Why this satisfies M-002 "cautious":

- Zero new dependencies (C-13). `AtomicU32` and `AtomicBool` are
  std.
- The atomic contract is the same primitive that executors are
  built on. We avoid the executor layer because our "executor" is
  `Bus::tick` and already exists.
- All orderings are pinned in §Concurrency Posture with a line of
  justification each.

P-B (rejected) — mpsc event channel per PLIC. `IrqLine::raise` pushes
`IrqEvent { src, kind }` into a `crossbeam_channel::Sender` held by
`PlicSignals`; `Plic::tick` drains the receiver.

Rejected because:
1. Adds a `crossbeam_channel` dependency (NG-8 / C-13).
2. Allocation per raise violates C-11.
3. An unbounded queue admits a raise-storm that overflows between
   ticks; a bounded queue admits dropped raises on overflow — both
   are worse than coalescing, which is the Gateway's documented
   contract (`plicGateway` I-3, `directIrq` I-D6).
4. The coalesce-by-design property (I-D7: two `IrqLine` clones alias
   the same bit) is lost — two clones would push two distinct
   events, so the Gateway sees `sample(true), sample(true)` instead
   of one. Changes observable semantics.

P-C (rejected) — raise acquires PLIC mutex and mutates Gateway
inline. `IrqLine::raise` takes `Arc<Mutex<Plic>>` (or a narrower
`Arc<Mutex<Core>>`), grabs the mutex, calls `gateway.sample(true)`,
updates `core.pending`, optionally signals MEIP.

Rejected because:
1. Couples the UART reader thread to the PLIC mutex across the
   Gateway FSM path. Defeats G-2's async-latency goal.
2. Requires re-deriving the observed-hart set and MEIP from
   inside `raise`, which is bus-tick-only state today
   (`plicGateway` I-9).
3. Deadlock prone if the tick thread holds Bus and waits on Core
   while the raise holds Core and waits on Bus.

P-D (rejected) — `async fn raise(&self) -> .await`.

Rejected because:
1. Rust has no built-in executor (Rust async-book §2). A true
   `async fn` requires a runtime (`tokio`, `async-std`, `embassy`).
2. `xemu` is single-threaded round-robin (NG-2). An executor would
   either need to share the bus tick thread (cooperative scheduling
   with `poll` calls interleaved with CPU stepping — enormous
   surface-area change) or run on a separate thread with all the
   synchronization P-C already failed on.
3. The semantics we actually want are "write a bit; someone will
   read it before the guest observes the next fetch." That is
   exactly `Release` + `Acquire` on an atomic, not a future.
4. M-002 explicitly warns against the cost; P-D is the paradigm case.

Adopted: P-A refined. Reasoning summary:

| Criterion | P-A | P-B | P-C | P-D |
|---|---|---|---|---|
| No new deps (NG-8) | yes | no | yes | no |
| Allocation-free raise (C-11) | yes | no | yes | probably no |
| Coalesce-by-design (I-D7) | yes | no | yes | n/a |
| Cross-thread safe (G-2) | yes | yes | deadlock | yes |
| Event-driven drain (G-7) | yes | yes | n/a | yes |
| Fits NG-2 single-thread model | yes | yes | no | no |
| No executor required | yes | yes | yes | NO |

[**Invariants**]

- I-D1 `IrqLine.src` is immutable after construction.
- I-D2 `raise()` is idempotent.
- I-D3 `lower()` is idempotent.
- I-D4 `pulse()` guarantees at least one rising-edge observation by
  the Edge gateway even if the producer is preempted between
  `level.fetch_or` and `edge_latch.fetch_or`, because the drain
  pulls `edge_latch` first (R-010 / I-D15).
- I-D5 `Plic::tick` either drains fully and calls `core.evaluate()`
  once (when the epoch gate opens) or performs exactly one atomic
  swap and returns (I-D14). No partial drain.
- I-D6 Between drains, any number of `raise`/`lower`/`pulse` events
  coalesce onto the shared bit (per-source coalesce is a feature).
- I-D7 `IrqLine: Clone`; two clones with the same `src` alias the
  same bit. Coalesce is contractual.
- I-D8 `Plic::reset` and `Plic::hard_reset` clear `PlicSignals` in
  place via `signals.reset()`.
- I-D8a (NEW, R-004) `Plic::{new, with_config}` construct
  `self.signals = Arc::new(PlicSignals::new())` exactly once.
  Neither `reset`, `hard_reset`, nor any future method replaces
  `self.signals` with a new `Arc`. In-place reset is contractual
  for the lifetime of the `Plic` instance. Violating this silently
  invalidates every outstanding `IrqLine`. Enforced by V-UT-10.
- I-D9-revised (was I-D9) Atomic orderings for `PlicSignals`:
  - `level.fetch_or(bit, Release)` — `raise`, `pulse` step 1
  - `level.fetch_and(!bit, Release)` — `lower`
  - `edge_latch.fetch_or(bit, Release)` — `pulse` step 2
  - `pending_raises.store(true, Release)` — tail of every `raise`,
    `lower`, `pulse`
  - `pending_raises.swap(false, Acquire)` — head of `Plic::tick`
  - `edge_latch.swap(0, AcqRel)` — drain
  - `level.load(Acquire)` — drain
  - `reset` uses `Release` stores (including a final
    `pending_raises.store(true, Release)` to force the next tick to
    drain).
  The `Release`+`Acquire` pair on `pending_raises` establishes
  happens-before for all preceding `level`/`edge_latch` mutations
  observed by the drain (std::sync::atomic semantics).
- I-D10-revised (was I-D10) `Plic::tick` is called only from
  `Bus::tick` via the `Device::tick` vtable path. No inherent
  method with the same name exists on `Plic`; no downcast from
  `dyn Device`. `IrqLine::raise`/`lower`/`pulse` are callable from
  any thread.
- I-D11-revised (was I-D11) Phase-boundary migration rule: a device
  either (a) holds no `IrqLine` and uses `Device::irq_line -> true`
  as its signalling path, or (b) holds an `IrqLine` and implements
  `Device::irq_line -> false`. No device reports `true` through
  `Device::irq_line` while also holding an `IrqLine`. Checkable
  per-device at every phase commit.
- I-D12 (NEW, R-002) `NUM_SRC <= 32` for the `AtomicU32`-backed
  `PlicSignals`. Co-located `const _: () = assert!(NUM_SRC <= 32);`
  in `signals.rs` fails the build if the constraint breaks.
- I-D13 (NEW, R-003) Within Phase 1, for each PLIC source, at most
  one of `{bitmap-fold path, signal-plane path}` is the source of
  truth. A device adopting `IrqLine` must drop its
  `Device::irq_line -> true` override before Phase 1's validation
  gate. Verifiable by source-id -> path audit at phase commit.
- I-D14 (NEW, M-001) Event-driven PLIC. `Plic::tick` performs no
  per-source work in the absence of a raise-since-last-drain *and*
  `self.needs_reevaluate == false`. Concretely:
  `pending_raises.swap(false, Acquire) == false &&
  !self.needs_reevaluate` -> the function returns after a single
  atomic swap. Testable by V-UT-11 and V-IT-7.
- I-D15 (NEW, M-001 + R-010) Epoch-gated drain ordering. Inside
  `Plic::tick`, if the gate opens, the drain reads `edge_latch`
  first, `level` second, so that any `pulse()` whose `edge_latch`
  bit is visible to the drain also has its `level` bit visible. The
  pulse sequence is `level <- 1`; `edge_latch <- 1`;
  `pending_raises <- true`. The drainer reads in reverse:
  `pending_raises.swap`, then `edge_latch.swap`, then `level.load`.
  The `Release`+`Acquire` on `pending_raises` covers both preceding
  `fetch_or`s; the subsequent reads observe them.

[**Data Structure**]

```rust
// src/device/irq.rs  (arch-neutral, <= 80 lines)

use std::sync::Arc;

/// Opaque handle for a single PLIC source wire.
/// `Clone` (I-D7). Cheap Arc bump.
#[derive(Clone)]
pub struct IrqLine {
    plane: Arc<dyn IrqSignalPlane>,
    src: u32,
}

impl IrqLine {
    pub(crate) fn new(plane: Arc<dyn IrqSignalPlane>, src: u32) -> Self {
        Self { plane, src }
    }
    pub fn raise(&self) { self.plane.raise(self.src); }
    pub fn lower(&self) { self.plane.lower(self.src); }
    pub fn pulse(&self) { self.plane.pulse(self.src); }
}

/// Arch-neutral trait; only `PlicSignals` implements it.
pub trait IrqSignalPlane: Send + Sync {
    fn raise(&self, src: u32);
    fn lower(&self, src: u32);
    fn pulse(&self, src: u32);
}
```

```rust
// src/arch/riscv/device/intc/plic/signals.rs  (<= 80 lines)

use std::sync::atomic::{AtomicBool, AtomicU32,
                        Ordering::{Acquire, AcqRel, Release}};
use crate::device::irq::IrqSignalPlane;
use super::core::NUM_SRC;

// I-D12 / C-12: PlicSignals uses AtomicU32, so NUM_SRC must fit.
const _: () = assert!(
    NUM_SRC <= 32,
    "PlicSignals: widen atomics if NUM_SRC > 32 (I-D12)",
);

pub(super) struct PlicSignals {
    level: AtomicU32,
    edge_latch: AtomicU32,
    pending_raises: AtomicBool,  // I-D14
}

impl PlicSignals {
    pub(super) fn new() -> Self {
        Self {
            level: AtomicU32::new(0),
            edge_latch: AtomicU32::new(0),
            pending_raises: AtomicBool::new(false),
        }
    }

    /// Consume the epoch; return true if there is work to drain.
    pub(super) fn take_epoch(&self) -> bool {
        self.pending_raises.swap(false, Acquire)
    }

    /// Drain snapshot per I-D15: edge first, level second.
    /// Returns (edge_bits, level_bits); clears edge_latch, not level.
    pub(super) fn drain(&self) -> (u32, u32) {
        let edg = self.edge_latch.swap(0, AcqRel);
        let lvl = self.level.load(Acquire);
        (edg, lvl)
    }

    /// In-place reset (I-D8, I-D8a).
    /// Callers mutate through `&self`; they must NOT replace the Arc.
    pub(super) fn reset(&self) {
        self.level.store(0, Release);
        self.edge_latch.store(0, Release);
        // Force the next tick to drain so any racing raise lands.
        self.pending_raises.store(true, Release);
    }
}

impl IrqSignalPlane for PlicSignals {
    fn raise(&self, src: u32) {
        self.level.fetch_or(1u32 << src, Release);
        self.pending_raises.store(true, Release);
    }

    fn lower(&self, src: u32) {
        self.level.fetch_and(!(1u32 << src), Release);
        self.pending_raises.store(true, Release);
    }

    fn pulse(&self, src: u32) {
        let bit = 1u32 << src;
        // Level first, edge second. Drain reverses (I-D15).
        self.level.fetch_or(bit, Release);
        self.edge_latch.fetch_or(bit, Release);
        self.pending_raises.store(true, Release);
    }
}
```

```rust
// src/arch/riscv/device/intc/plic/mod.rs  (delta vs 00_PLAN)

pub struct Plic {
    gateways: [Gateway; NUM_SRC],
    core: Core,
    signals: Arc<PlicSignals>,       // I-D8a: constructed ONCE
    needs_reevaluate: bool,          // set by notify/claim/complete
}

impl Plic {
    pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self {
        Self {
            gateways: Default::default(),
            core: Core::new(num_harts, irqs),
            signals: Arc::new(PlicSignals::new()),   // I-D8a
            needs_reevaluate: false,
        }
    }

    pub fn with_irq_line(&self, src: u32) -> IrqLine {
        assert!(
            (1..NUM_SRC as u32).contains(&src),
            "bad PLIC source {src}",
        );
        IrqLine::new(
            Arc::clone(&self.signals) as Arc<dyn IrqSignalPlane>,
            src,
        )
    }
}

impl Device for Plic {
    // R-001: tick is the Device::tick override. No inherent `tick`.
    fn tick(&mut self) {
        let event = self.signals.take_epoch();        // I-D14
        if !event && !self.needs_reevaluate {
            return;                                   // event-driven fast path
        }
        self.needs_reevaluate = false;
        let (edg, lvl) = self.signals.drain();        // I-D15
        for s in 1..NUM_SRC {
            let bit = 1u32 << s;
            let level = lvl & bit != 0;
            let edge  = edg & bit != 0;
            match self.gateways[s].sample_with(level, edge) {
                GatewayDecision::Pend     => self.core.set_pending(s),
                GatewayDecision::Clear    => self.core.clear_pending(s),
                GatewayDecision::NoChange => {}
            }
        }
        self.core.evaluate();
    }

    fn reset(&mut self) {
        self.core.reset_runtime();
        self.gateways.iter_mut().for_each(|g| g.reset_runtime());
        self.signals.reset();                          // I-D8, I-D8a
        self.needs_reevaluate = true;                  // drain on next tick
    }

    // Phase 1/2 only: shim for the bitmap fold.
    // Deleted in Phase 3 (unchanged from 00_PLAN).
    fn notify(&mut self, bitmap: u32) {
        for s in 1..NUM_SRC {
            let level = bitmap & (1u32 << s) != 0;
            // Adopter devices return false (I-D11-rev + I-D13), so
            // their bits are always 0 here; no clobber.
            match self.gateways[s].sample_with(level, false) {
                GatewayDecision::Pend     => self.core.set_pending(s),
                GatewayDecision::Clear    => self.core.clear_pending(s),
                GatewayDecision::NoChange => {}
            }
        }
        self.needs_reevaluate = true;
    }
}
```

```rust
// Gateway delta (src/arch/riscv/device/intc/plic/gateway.rs)
//
// Current `sample(level)` becomes a shim around `sample_with`:
impl Gateway {
    pub(super) fn sample_with(&mut self, level: bool, edge: bool)
        -> GatewayDecision
    {
        match self.kind {
            SourceKind::Level => self.sample_level(level),
            SourceKind::Edge  => self.sample_edge_signal(level, edge),
        }
    }

    #[inline]
    pub(super) fn sample(&mut self, level: bool) -> GatewayDecision {
        // R-007 shim branch: preserved through Phase 2 so V-UT-7
        // remains a meaningful regression test.
        self.sample_with(level, false)
    }
}
```

### Concurrency Posture

Every atomic operation and its ordering is pinned here. This section
honours M-002's "atomics alone is forbidden wording" by justifying
each ordering.

| Site | Op | Ordering | Justification |
|---|---|---|---|
| `raise` | `level.fetch_or(bit)` | Release | Makes `level` bit visible to any later `Acquire` load of `pending_raises` on the drain side. |
| `raise` tail | `pending_raises.store(true)` | Release | Synchronizes-with the drain's `Acquire` swap; establishes HB for the preceding `level.fetch_or`. |
| `lower` | `level.fetch_and(!bit)` | Release | Same reasoning as `raise`. |
| `lower` tail | `pending_raises.store(true)` | Release | Same reasoning as `raise` tail. |
| `pulse` | `level.fetch_or(bit)` | Release | First of two writes; the subsequent edge write must be visible only if this one was. Drain reads edge first (I-D15). |
| `pulse` | `edge_latch.fetch_or(bit)` | Release | Pairs with drain's `AcqRel` swap. |
| `pulse` tail | `pending_raises.store(true)` | Release | HB edge for both preceding writes. |
| `reset` | 3x `store(0)` / `store(true)` | Release | Reset is a bus-tick-thread action; Release so any post-reset raise on another thread observes zeros. |
| `Plic::tick` head | `pending_raises.swap(false)` | Acquire | The only HB-acquire that sees preceding `level`/`edge_latch` writes. If this returns `false` and `!needs_reevaluate`, no drain is performed (I-D14). |
| `Plic::tick` drain | `edge_latch.swap(0)` | AcqRel | Acquire half pairs with `pulse`'s Release on `edge_latch`; Release half publishes the zero. Comes first (I-D15). |
| `Plic::tick` drain | `level.load` | Acquire | Pairs with `raise`/`lower`/`pulse`'s Release on `level`. Comes after `edge_latch.swap`. |
| `IrqState` (existing) | per `device/mod.rs:55-79` | Relaxed | Unchanged; `IrqState` is the CPU<->PLIC mailbox, documented separately. |

Data-race argument: the only shared mutable data is `PlicSignals`.
Every access uses `Release`/`Acquire`/`AcqRel` with a pending-raise
synchronization point. There are no non-atomic shared writes, no
`UnsafeCell`, no `unsafe`. A formal `loom` exhaustive check is
listed as follow-up (OQ-5).

[**API Surface**]

```rust
// Public (arch-neutral, src/device/irq.rs):
pub struct IrqLine { /* Clone */ }
impl IrqLine {
    pub fn raise(&self);
    pub fn lower(&self);
    pub fn pulse(&self);
}
pub trait IrqSignalPlane: Send + Sync {
    fn raise(&self, src: u32);
    fn lower(&self, src: u32);
    fn pulse(&self, src: u32);
}

// Public (arch-specific, re-exported through seam):
impl Plic {
    pub fn with_irq_line(&self, src: u32) -> IrqLine;          // NEW
}
impl Device for Plic {
    fn tick(&mut self);                                         // NEW override (R-001)
    fn reset(&mut self);                                        // mutated: signals.reset()
    fn notify(&mut self, _: u32);                               // RETAINED Phase 1-2; deleted Phase 3
}

// Device trait (src/device/mod.rs) — Phase 3 delta (unchanged):
//   removed: fn irq_line(&self) -> bool { false }
//   removed: fn notify(&mut self, _: u32) {}
//
// Bus::add_mmio — Phase 3 delta (unchanged):
//   removed: irq_source: u32 parameter.
```

Dispatch (R-001 resolution, normative):

1. `Plic::tick` is the `Device::tick` override. Signature:
   `fn tick(&mut self)`.
2. `Bus::tick` calls it through the existing
   `self.mmio[i].dev.tick()` loop at `bus.rs:233`.
3. The slow-tick order within `Bus::tick` is, from Phase 1 onward:
   - Fast-path MTIMER tick (unchanged, `bus.rs:219-221`).
   - Slow-path loop (`bus.rs:227-240`): each non-MTIMER device's
     `tick()` is called. PLIC's `tick()` now performs the new
     signal drain per §Architecture.
   - Bitmap-fold (`bus.rs:231-240`) collects `irq_line()` bits.
     Adopter devices return `false`, so their source bits are 0.
   - `plic.notify(bitmap)` (`bus.rs:241-243`). Runs after PLIC's
     own `tick()` has drained the signal plane. Because adopter
     bits are 0 in `bitmap`, `notify` cannot clobber an adopter
     source; `notify` sets `needs_reevaluate = true`, so the next
     bus-tick round's `Plic::tick` will apply the gateway decisions
     left by `notify` and then run `evaluate`.
4. Phase 2: the bitmap-fold and `notify` call are deleted. Phase 3:
   `Plic::notify` and the `Device::notify` trait method are deleted.

[**Constraints**]

Unchanged from 00_PLAN (C-1..C-11). Added / amended:

- C-2 (AMENDED, R-008): appended "`IrqSignalPlane` is declared in
  `src/device/irq.rs` (arch-neutral) and implemented by
  `PlicSignals` inside `src/arch/riscv/device/intc/plic/signals.rs`.
  The seam test at `tests/arch_isolation.rs:249-280` checks only
  `pub use crate::arch::*` names; arch-neutral traits are outside
  its scope."
- C-12 (NEW, R-002) `NUM_SRC <= 32`. Widening past 32 requires
  widening `PlicSignals.{level, edge_latch}` from `AtomicU32` to
  `AtomicU64` (or `[AtomicU32; N]`) in the same diff. Enforced by
  the `const _: () = assert!(NUM_SRC <= 32);` in `signals.rs`.
- C-13 (NEW, M-002) No new async-runtime or channel crate is added.
  Specifically forbidden in this feature: `tokio`, `async-std`,
  `embassy`, `smol`, `futures` (beyond what's already in std),
  `crossbeam_channel`, `flume`. Enforced by `Cargo.toml` diff review
  at phase gates.

---

## Implement

### Execution Flow

[**Main Flow**]

Runtime view (post-feature):

1. Machine construction — unchanged from 00_PLAN. Devices obtain an
   `IrqLine` at construction; Phase 3 drops the `irq_source`
   parameter from `Bus::add_mmio`.
2. Device signalling (any thread): `IrqLine::raise()` ->
   `PlicSignals::raise(src)` -> two `Release` atomics (level bit,
   pending-raises flag). Zero allocation, zero syscall.
3. Bus tick (tick thread):
   a. `Bus::tick` -> fast-path MTIMER.
   b. Every `SLOW_TICK_DIVISOR` ticks, slow-path loop:
      - For each non-MTIMER device, `r.dev.tick()`.
      - For `Plic`, `tick()` does: `take_epoch` -> if `false` and
        `!needs_reevaluate`, return (I-D14). Else drain -> gateway
        loop -> `core.evaluate`.
      - [Phase 1/2 only] bitmap fold + `plic.notify(bitmap)`
        (sets `needs_reevaluate = true`).
4. Guest claim/complete: unchanged from `plicGateway/02`. Each
   `Plic::write`/`Plic::read` that mutates pending state sets
   `needs_reevaluate = true` so the next `Plic::tick` will
   `evaluate`, even with no new raise.

[**Failure Flow**]

Unchanged from 00_PLAN F-1..F-5. Additions:

- F-6 Raise from thread X, `reset` from tick thread, raise from X
  again: the second raise sets the bits after `reset`. `reset` has
  set `pending_raises = true` intentionally, so the next `tick`
  drains and observes both the post-reset bits and any stale
  pre-reset ordering artifacts. The only guest-visible outcome is
  "interrupt asserted soon after reset" — acceptable.
- F-7 Bus tick runs while no raises are pending: `take_epoch`
  returns `false`, `needs_reevaluate` is `false`, `tick` returns
  without touching any gateway. This is the I-D14 invariant and the
  G-7 performance property.

[**State Transition**]

Same table as 00_PLAN §State Transition. Additions:

- `pending_raises: false -> true`: on any `raise`/`lower`/`pulse`
  tail (Release store); on `reset` (Release store).
- `pending_raises: true -> false`: at the top of `Plic::tick`
  (Acquire swap).
- `needs_reevaluate: false -> true`: on `Plic::notify(bitmap)`
  [Phase 1-2 only], on any MMIO write that mutates claim/complete
  state, and on `reset`.
- `needs_reevaluate: true -> false`: at the top of the drain branch
  of `Plic::tick` after the epoch gate opens.

### Implementation Plan

[**Phase 1 — Handle + signal plane + UART adopter**]

Scope: opt-in migration. Adopter devices exit the bitmap-fold path.
Both code paths coexist but operate on disjoint source sets
(I-D13).

1. Add `src/device/irq.rs` with `IrqLine` + `IrqSignalPlane`
   (arch-neutral, <= 80 lines).
2. Add `src/arch/riscv/device/intc/plic/signals.rs` with
   `PlicSignals`, `new`, `take_epoch`, `drain` (R-010 order),
   `reset`, and the `IrqSignalPlane` impl; <= 80 lines. Include
   `const _: () = assert!(NUM_SRC <= 32);` at module top (I-D12).
3. Extend `Plic`:
   - `signals: Arc<PlicSignals>` field initialized exactly once in
     both `Plic::new` and `Plic::with_config` (I-D8a).
   - `needs_reevaluate: bool` field, default `false`.
   - `with_irq_line(&self, src: u32) -> IrqLine` factory with
     the `(1..NUM_SRC as u32)` bounds assert.
   - New `Device::tick` override per §API Surface §Dispatch. This
     replaces the current no-op default from `device/mod.rs:28`
     for `Plic` only.
   - Modified `Device::reset` so it calls `self.signals.reset()`
     and leaves `self.signals` Arc unchanged (I-D8, I-D8a), and
     sets `self.needs_reevaluate = true`.
   - Existing `Plic::notify(bitmap)` modified to set
     `self.needs_reevaluate = true` instead of calling
     `core.evaluate` inline.
4. Extend `Gateway`: add `sample_with(level: bool, edge: bool)`.
   Keep the existing `sample(level)` as
   `#[inline] fn sample(&mut self, level: bool) -> GatewayDecision
   { self.sample_with(level, false) }` shim (R-007 shim branch).
5. Modify `Uart`:
   - Constructor `Uart::new_stdio_with_irq(line: IrqLine)` — or a
     builder variant.
   - After a state change that would have made
     `Uart::irq_line()` truthy, call `self.line.raise()`. After a
     state change that would have made it falsy, call
     `self.line.lower()`.
   - Override `Device::irq_line(&self) -> bool { false }` (I-D11-rev
     + I-D13). UART now signals only through `IrqLine`.
6. Machine construction: thread the `IrqLine` into UART's
   constructor in `xemu/src/machine/*.rs` (and in any test
   machines constructed inside `xemu/xcore/tests/`).
7. `Bus::tick` unchanged — adopter devices simply return `false`
   from `Device::irq_line`, so the bitmap fold collects 0 for
   their source. `Plic::tick` runs per normal vtable at step 3.b of
   the slow-path loop; the bitmap fold + `plic.notify(bitmap)`
   continue to run for remaining (non-adopter) devices. No
   coexistence race because I-D13 guarantees disjoint source sets.

Validation gate for Phase 1:

- `cargo test -p xcore` >= baseline 374 + Phase-1 tests (>= +11 per
  Phase-1 validation list below: V-UT-1..V-UT-12, V-IT-1, V-IT-3,
  V-F-1..V-F-4, V-E-1..V-E-4).
- `cargo test -p xcore --test arch_isolation` green; `git diff main
  -- xemu/xcore/tests/arch_isolation.rs` empty (C-2).
- Boot gate: xv6 + linux-2hart + debian-2hart with `DEBUG=n`
  (project memory `feedback_debug_flag`).
- No new crate in `Cargo.toml` (C-13).

[**Phase 2 — VirtioBlk adopter; retire `Bus::tick` bitmap fold**]

Scope: all in-tree signalling devices migrated. `Plic::notify` still
present but no longer called from `Bus::tick`.

1. Migrate VirtioBlk per 00_PLAN Phase 2 step 1. Override
   `Device::irq_line -> false` on VirtioBlk.
2. Delete the bitmap fold from `Bus::tick` (`bus.rs:227-243`) —
   specifically the `.fold(0u32, …)` closure and the
   `self.mmio[i].dev.notify(irq_lines)` call. Keep the per-device
   `tick()` loop.
3. Mark `MmioRegion::irq_source` with `#[allow(dead_code)]` and
   leave the field in place until Phase 3.
4. Keep `Device::irq_line` / `Device::notify` defaults in the
   trait.
5. Keep `Plic::notify` on `Plic` (unused but callable by tests;
   retired in Phase 3).

Validation gate for Phase 2:

- `cargo test -p xcore` >= Phase-1 count + 4 (VirtioBlk adopter
  tests).
- Boot trio green.
- No new crate (C-13).

[**Phase 3 — Retire the legacy trait surface**]

Scope: clean-up. No new functionality.

1. Delete `Device::irq_line` from the trait (default + all
   overrides).
2. Delete `Device::notify` from the trait (default + `Plic`'s
   impl).
3. Delete `Plic::notify` impl.
4. Delete `MmioRegion::irq_source` and the `irq_source: u32`
   parameter of `Bus::add_mmio`. Call sites (R-006) to update:

   | File | Line | Call |
   |---|---|---|
   | `xemu/xcore/src/device/bus.rs` | 159 | `pub fn add_mmio(...)` — signature |
   | `xemu/xcore/src/device/bus.rs` | 473 | `bus.add_mmio("stub", MMIO_BASE, MMIO_SIZE, stub(), 0)` |
   | `xemu/xcore/src/device/bus.rs` | 481 | `bus.add_mmio("stub", MMIO_BASE, MMIO_SIZE, stub(), 0)` |
   | `xemu/xcore/src/device/bus.rs` | 491 | `new_bus().add_mmio("bad", CONFIG_MBASE, 0x100, stub(), 0)` |
   | `xemu/xcore/src/device/bus.rs` | 498-499 | two `add_mmio` calls (`"a"`, `"b"`) |
   | `xemu/xcore/src/device/bus.rs` | 505 | `new_bus().add_mmio("empty", MMIO_BASE, 0, stub(), 0)` |
   | `xemu/xcore/src/device/bus.rs` | 512 | `bus.add_mmio("plic", MMIO_BASE, MMIO_SIZE, stub(), 0)` |
   | `xemu/xcore/src/device/bus.rs` | 520 | `bus.add_mmio("stub", MMIO_BASE, MMIO_SIZE, stub(), 0)` |
   | `xemu/src/machine/*.rs` | per `rg 'add_mmio\('` | machine-construction sites |
   | `xemu/xcore/tests/*.rs` | per `rg 'add_mmio\('` | integration-test sites |

   Every non-header site currently passes `0` or a source-id literal
   as the last positional arg; Phase 3 drops the argument from each.
5. Re-run `cargo test -p xcore --test arch_isolation`. Verify
   `BUS_DEBUG_STRING_PINS` at `arch_isolation.rs:74-77` is
   unchanged (expected: `("plic", 1)` pin stays at 1, per C-3).

Validation gate for Phase 3:

- `cargo test -p xcore` green. Net test count same or greater than
  Phase 2 (Phase-3-retired tests targeting `Device::irq_line` and
  `Plic::notify` have Phase-1/2 replacements already counted).
- `cargo clippy` clean.
- `cargo fmt --all` clean.
- `make run` (xv6 default), `make run` linux-2hart, `make run`
  debian-2hart — all boot successfully with `DEBUG=n`.
- `git diff main -- xemu/xcore/tests/arch_isolation.rs` empty
  (C-2).
- No new crate (C-13).

## Trade-offs

Unchanged T-1..T-5 from 00_PLAN with the following notes:

- T-1 Handle type shape — Option A adopted (TR-1 agreed).
- T-2 Signal plane representation — Option A adopted with I-D12 +
  C-12 enforcement (TR-2 agreed).
- T-3 `PlicSignals` ownership — Option A adopted (TR-3 agreed).
- T-4 `pulse()` semantics — Option A adopted; TR-4 flagged
  stuck-level hazard for future edge adopters. Round 01 response:
  V-IT-6 + V-E-6 witness the end-to-end path; the stuck-level hazard
  is documented in I-D4 and in NG-3 (no edge adopter this feature).
  If a future plan promotes a device to Edge, that plan can revisit
  T-4 with a concrete consumer.
- T-5 Evaluation cadence — changed from 00_PLAN Option A ("run every
  slow bus tick") to a hybrid: gated run every slow bus tick (I-D14,
  I-D15). Same cadence as the 00 recommendation, but with a no-op
  fast path when nothing happened. Cost: one `AtomicBool` per
  `PlicSignals` and one `Release` store per raise.

New trade-off introduced by M-001:

- T-6 Async posture — see §Async Posture. Refined P-A adopted;
  P-B/C/D rejected.

## Validation

[**Unit Tests**]

- V-UT-1 `IrqLine::raise` sets the `level` bit: construct
  `PlicSignals`, wrap in `Arc<dyn IrqSignalPlane>`, create `IrqLine`,
  call `raise`, assert
  `signals.level.load(Acquire) & (1 << src) != 0` and
  `signals.pending_raises.load(Acquire) == true`.
- V-UT-2 `IrqLine::lower` clears the `level` bit; asserts
  `pending_raises` is set.
- V-UT-3 `IrqLine::pulse` sets both bits and `pending_raises`.
- V-UT-4 `IrqLine::clone` aliases: two clones of the same `src`
  both mutate the same bit.
- V-UT-5 `PlicSignals::drain` clears `edge_latch` and returns
  `(edge_snapshot, level_snapshot)`. Order of internal reads: edge
  first (I-D15).
- V-UT-6 `PlicSignals::reset` zeroes `level` and `edge_latch`,
  and sets `pending_raises = true` (forces next-tick drain).
- V-UT-7 `Gateway::sample_with(level, false)` is byte-equivalent to
  `Gateway::sample(level)` for every currently-tested transition.
  (R-007 shim branch — meaningful because shim is retained.)
- V-UT-8 `Gateway::sample_with(false, true)` on an Edge source
  emits `Pend` (forced rising-edge observation, I-D4).
- V-UT-9 `Plic::tick` drains and evaluates: construct PLIC, call
  `with_irq_line(2)`, configure enable/threshold/priority for ctx 0,
  `line.raise()`, invoke `<Plic as Device>::tick(&mut plic)`,
  assert MEIP is asserted on ctx 0's `IrqState`.
- V-UT-10 (NEW, R-004) Arc-identity across reset: construct `Plic`,
  call `with_irq_line(2)`, hold the returned `IrqLine`, call
  `<Plic as Device>::reset(&mut plic)`, call `line.raise()`, call
  `<Plic as Device>::tick(&mut plic)`, assert MEIP asserts. Variant
  with `<Plic as Device>::hard_reset`. Together these enforce
  I-D8a.
- V-UT-11 (NEW, M-001 / I-D14) `Plic::tick` epoch gate: with no
  raises and `needs_reevaluate == false`, `tick` performs exactly
  one atomic swap (`pending_raises.swap`) and zero gateway calls.
  Asserted via an instrumented `Gateway` stub counting sample
  invocations (test-only, feature-gated).
- V-UT-12 (NEW, R-003 FSM property) `sample(a); sample(b)` on a
  Level-source Gateway equals `sample(b)` for all `a, b in {true,
  false}`. Four-case exhaustive test documenting the property that
  makes the Phase-1 ordering benign even if I-D13 were relaxed.

[**Integration Tests**]

- V-IT-1 Cross-thread raise: spawn a thread that calls
  `line.raise()`; join; call `<Plic as Device>::tick`; assert the
  raise is observed (MEIP set). Exercises I-D9-revised
  happens-before.
- V-IT-2 Arch-isolation: `cargo test -p xcore --test arch_isolation`
  green; `git diff main -- xemu/xcore/tests/arch_isolation.rs`
  empty at every phase gate (C-2).
- V-IT-3 UART end-to-end: construct a machine with UART holding an
  `IrqLine`, write a byte through the PTY, bus-tick repeatedly,
  assert MEIP asserts at the next bus-tick boundary. Demonstrates
  G-2 (async raise latency reduction).
- V-IT-4 VirtioBlk end-to-end (Phase 2): complete a DMA request;
  assert `IrqLine::raise` lands in `PlicSignals` and the gateway
  pends.
- V-IT-5 (Phase 3) The legacy surface is gone: no `fn irq_line` or
  `fn notify` remains in `src/device/` (except the new `IrqLine`
  type's methods).
- V-IT-6 (NEW, R-005) Plic-boundary pulse end-to-end: one Edge
  source; obtain `IrqLine`; call `line.pulse()` from a different
  thread; `<Plic as Device>::tick`; assert MEIP asserts on ctx 0
  with threshold=0 and enable=source.
- V-IT-7 (NEW, M-001 / I-D14) Event-driven property: construct PLIC,
  no raises, call `<Plic as Device>::tick` 1000 times; assert via
  instrumented Gateway stub that the total number of
  `Gateway::sample_with` calls is zero. Confirms G-7.

[**Failure / Robustness Validation**]

- V-F-1 Raise during reset: spawn a raiser thread; main thread
  calls `<Plic as Device>::reset`; assert post-reset, some raise
  post-reset-completion is observed. Handle alive (I-D8, I-D8a).
- V-F-2 Raise with no registered gateway: set a signal-plane bit
  for source 31; `Plic::tick` runs; no panic, no MEIP.
- V-F-3 Double-lower idempotent.
- V-F-4 Double-raise idempotent — one `set_pending`.
- V-F-5 (NEW, M-002) `loom` interleaving — deferred, OQ-5.
  Documented here so the next reviewer can weigh in.

[**Edge Case Validation**]

- V-E-1 `IrqLine` for `src = 0` rejected at construction
  (`Plic::with_irq_line` asserts).
- V-E-2 `IrqLine` for `src >= NUM_SRC` rejected at construction.
- V-E-3 `pulse()` on a Level source: `level` is set, `edge_latch`
  is set but ignored by `sample_level`. Behavior equivalent to
  `raise()`. Documented and tested.
- V-E-4 (EXPANDED, R-010) Concurrent raise + tick, three
  interleavings:
  - Seq A — `raise` completes before `Plic::tick`'s
    `pending_raises.swap`: observed this tick.
  - Seq B — `raise` completes after `pending_raises.swap` but
    before `edge_latch.swap`: bits land; because `raise`'s
    `pending_raises.store(true)` re-sets the epoch, the next
    `Plic::tick` observes them. Acceptable; every raise is
    observed within <= 2 ticks.
  - Seq C — `raise` completes after the drain reads: same as Seq
    B — observed next tick.
  Asserted by a racing-raise-then-tick loop that logs per-raise
  observed-tick numbers and verifies the bound.
- V-E-5 Phase-coexistence (Phase 1 only): a device still using the
  `Device::irq_line` path (e.g., VirtioBlk pre-Phase-2) and a
  device using `IrqLine` (UART) coexist; I-D13 holds. Test uses
  UART-as-adopter + VirtioBlk-as-legacy and asserts both fire
  correctly.
- V-E-6 (NEW, R-005) `pulse()` on an Edge source via `IrqLine`
  from a different thread; `Plic::tick`; MEIP asserts.
  End-to-end counterpart to V-UT-8 + V-IT-6.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|---|---|
| G-1 (device -> PLIC direct) | V-IT-3, V-IT-4, V-IT-5 |
| G-2 (any-thread raise) | V-IT-1, V-IT-3, V-F-1, V-IT-6 |
| G-3 (determinism) | V-E-4, V-UT-7, V-UT-12 |
| G-4 (legacy surface removed) | V-IT-5 (Phase 3) |
| G-5 (seam stable) | V-IT-2 |
| G-6 (boot gates) | Phase 1/2/3 boot-gate checklist |
| G-7 (event-driven) | V-UT-11, V-IT-7 |
| I-D1 (handle immutability) | V-UT-1..V-UT-4 |
| I-D2, I-D3 (idempotent raise/lower) | V-F-3, V-F-4 |
| I-D4 (pulse latch, drain-order) | V-UT-3, V-UT-8, V-IT-6, V-E-6 |
| I-D5 (drain atomicity / gate) | V-UT-5, V-UT-9, V-UT-11 |
| I-D6 (coalesce) | V-F-4, V-E-4 |
| I-D7 (clones alias) | V-UT-4 |
| I-D8 (reset clears signals) | V-UT-6, V-F-1 |
| I-D8a (Arc identity across reset) | V-UT-10 |
| I-D9-revised (orderings) | V-IT-1, V-E-4, §Concurrency Posture |
| I-D10-revised (tick via Device vtable) | V-IT-2 + design inspection |
| I-D11-revised (no both-states device) | Phase-gate audit + V-E-5 |
| I-D12 (`NUM_SRC <= 32`) | `const_assert!` in `signals.rs`; build fails on violation |
| I-D13 (single-path per source) | Phase-gate audit + V-E-5 |
| I-D14 (event-driven) | V-UT-11, V-IT-7 |
| I-D15 (drain order) | V-UT-5, V-E-4 |
| C-1 (file size cap) | File-size check at every phase gate |
| C-2 (seam stable) | V-IT-2 |
| C-3 (bus debug pins) | `arch_isolation` after Phase 2/3 |
| C-4 (no CSR leak) | grep + `arch_isolation` |
| C-6 (test count monotone) | `cargo test -p xcore` output per phase |
| C-8 (boot per phase) | Boot-gate checklist per phase |
| C-10/C-11 (no alloc) | Inspection + benchmark if available |
| C-12 (`NUM_SRC <= 32`) | `const_assert!` in `signals.rs` |
| C-13 (no async-runtime dep) | `git diff main -- Cargo.toml xemu/xcore/Cargo.toml` empty at every phase gate |

---

## Risks

- Risk 1 Phase-1 adopter device still has some code path that
  returns `true` from `irq_line()` through a compile-time-hidden
  default. Mitigation: I-D13 audit is a per-device manual check at
  the Phase-1 validation gate; `Uart::irq_line` is explicitly set
  to `false` in step 5.
- Risk 2 `Gateway::sample(level)` shim diverges from
  `sample_with(level, false)` under refactor. Mitigation: V-UT-7
  enumerates every currently-tested transition.
- Risk 3 Cross-thread raise ordering bug (rare, race-dependent).
  Mitigation: I-D9-revised spells each ordering out; V-IT-1 +
  V-E-4 exercise the happens-before; `loom` deferred (OQ-5) is a
  known escalation path.
- Risk 4 Edge adopter (future) finds T-4 "sticky level" surprising.
  Mitigation: V-IT-6 + V-E-6 pin the semantics; future plan can
  revisit with a concrete consumer.
- Risk 5 Phase-3 `add_mmio` signature ripple hits an unenumerated
  site. Mitigation: §Phase 3 call-site table + `cargo check` is
  authoritative.
- Risk 6 (NEW, M-002) A future refactor reinstates an `Arc`
  replacement in `Plic::reset`. Mitigation: V-UT-10 is a permanent
  regression test.
- Risk 7 (NEW, M-001) The epoch gate masks a lost wakeup (a raise
  sets `pending_raises` but the drain thread missed a prior
  `Release` ordering). Mitigation: happens-before established by
  `Release`+`Acquire` on the same atomic; no race possible per
  `std::sync::atomic` semantics. `loom` can confirm (OQ-5).

## Open Questions

- OQ-1 `IrqLine: Clone` — position unchanged: yes.
- OQ-2 — superseded (T-4 Option A + R-010).
- OQ-3 — superseded (I-D14, I-D15).
- OQ-4 — resolved (R-008, C-2 amended).
- OQ-5 (NEW) `loom` interleaving validation — deferred follow-up.
- OQ-6 (NEW) `pending_raises: AtomicBool` vs `AtomicU32` epoch
  counter — position: `AtomicBool`. Revisit if test flakes or if a
  future profile shows raise-storm pathology.

## Gates

- Phase 1 gate: `cargo test -p xcore` >= baseline 374 + 11 new; boot
  trio (xv6, linux-2hart, debian-2hart) green with `DEBUG=n`; C-2
  diff empty; C-13 `Cargo.toml` diff empty.
- Phase 2 gate: Phase-1 gate still passes; `cargo test -p xcore` >=
  Phase-1 + 4; bitmap fold deleted from `Bus::tick`; boot trio green.
- Phase 3 gate: Phase-2 gate still passes; `fn irq_line` and
  `fn notify` gone from `Device` trait; `MmioRegion::irq_source`
  and `Bus::add_mmio`'s `irq_source` parameter gone; `cargo clippy`
  / `cargo fmt --all` clean; boot trio green; `arch_isolation`
  pins unchanged.
- At every gate: no commits authored by the executor (project
  memory `feedback_user_commits`).
