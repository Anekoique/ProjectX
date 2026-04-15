# `directIrq` PLAN `00`

> Status: Draft
> Feature: `directIrq`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

Opening plan of the `directIrq` iteration loop. This feature closes
MANUAL_REVIEW items **#5** (external devices must interact with PLIC
directly, not through the bus bitmap pump) and **#6** (asynchronous
signaling: device → PLIC raise, PLIC → hart MEIP), inheriting the
`Gateway + Core + Source` substrate that just landed under
`docs/fix/plicGateway/` (commit `98da289`).

The deliverable is an arch-neutral `IrqLine` handle that each MMIO
device holds and calls directly (`raise()`, `lower()`, `pulse()`) to
signal its PLIC gateway. The bus-side bitmap pump
(`xemu/xcore/src/device/bus.rs:217-243`) and the
`Device::irq_line() -> bool` / `Device::notify(u32)` trait surface are
retired. `Plic::notify(u32)` goes away. The PLIC re-evaluates either
per-raise (when `IrqLine` is called from the tick thread) or at the
next tick boundary (when called from a non-tick thread such as the
UART reader).

Scope spans **three phases** across the iteration loop; this PLAN
covers all three. Phase 1 introduces the handle type and wires it
into `Plic::with_irq_line()`, with UART as the first adopter (both
paths coexist so the test baseline stays green). Phase 2 migrates
VirtioBlk and removes the bus bitmap pump from `Bus::tick`. Phase 3
deletes the legacy trait methods and retires `Plic::notify`.

Concurrency posture is **explicit** and the core invariant of the
feature: the handle is `Arc<Mutex<Plic>>`-free; a device calls
`IrqLine::raise()` with `Acquire/Release` atomics against a
per-source bit; the PLIC consumes those bits inside `Plic::tick`
(driven by `Bus::tick`) under the existing bus mutex. This picks the
design-A2 point from `plicGateway` R-006 vocabulary: any-thread
raise + tick-boundary eval. It does **not** introduce a true
multi-threaded PLIC — that remains a follow-up.

## Log

[**Feature Introduce**]

- First plan of the `directIrq` feature. Introduces:
  - A new module `xemu/xcore/src/device/irq.rs` (arch-neutral) holding
    `IrqLine` — a handle whose `raise`/`lower`/`pulse` calls deposit
    into a shared per-source latch structure consumed by the PLIC.
  - A PLIC-owned signal plane (`PlicSignals`) backed by two atomic
    u32s (`level`, `edge_latch`) — the only PLIC internal touched
    from non-tick threads.
  - `Plic::with_irq_line(src) -> IrqLine` factory and a new
    `Plic::tick()` that drains the signal plane through `Gateway::sample`.
  - A phased removal of `Device::irq_line`, `Device::notify(u32)`,
    `Bus::tick`'s bitmap fold, and `Plic::notify(u32)` — the
    "bus-as-switchboard" pattern MANUAL_REVIEW #5 flagged.
- Inherits the `Gateway + Core + Source` split from the just-landed
  `plicGateway` feature (see `docs/fix/plicGateway/02_PLAN.md`). No
  changes to `Core` are planned; `Gateway::sample` remains the sole
  write path into `Core::{set_pending, clear_pending}` via
  `GatewayDecision`.

[**Review Adjustments**]

None — this is round 00.

[**Master Compliance**]

No MASTER directives for this feature yet. Inherited-MASTER rows
are carried in the Response Matrix for full traceability, matching
the `plicGateway` 01/02 precedent:

- `archModule` 00-M-002 (topic-nested `arch/<name>/` layout) —
  Honored. New module `src/device/irq.rs` is intentionally
  arch-neutral and lives outside `src/arch/`. No arch-path leak.
- `archLayout` 01-M-004 (device split mirrors arch nesting) —
  Honored. The `IrqLine` handle is the seam between arch-neutral
  devices and the arch-specific PLIC that sits under
  `src/arch/riscv/device/intc/plic/`.
- `plicGateway` 02 inherited posture (I-9 tick-thread-only) —
  **Re-examined and superseded**. This feature is the explicit
  re-examination promised by TR-2 and by `01_PLAN.md:904-906`.
  I-9 is retired; new invariant I-D9 (Acquire/Release on
  `PlicSignals`) replaces it.

### Changes from Previous Round

[**Added**]

- New feature. Everything is new relative to `plicGateway/02_PLAN.md`.
  See §Architecture and §Data Structure for the incremental surface.

[**Changed**]

- Nothing from a prior `directIrq` iteration (none exist). The
  changes from the broader project baseline are enumerated in
  §Architecture as phase deltas.

[**Removed**]

- Planned removals across the three phases, at project level:
  - `Device::irq_line(&self) -> bool` — retired in Phase 3.
  - `Device::notify(&mut self, u32)` — retired in Phase 3.
  - `Plic::notify(&mut self, u32)` — retired in Phase 3.
  - `Bus::tick`'s bitmap fold (`bus.rs:227-243`) — retired in Phase 2.
  - `MmioRegion::irq_source: u32` — retired in Phase 3 (replaced by
    PLIC-side construction-time source-id binding).

[**Unresolved**]

- **OQ-1** Handle cloning semantics: should `IrqLine` be `Clone`?
  Argument for: a device with multiple asynchronous producers (e.g.
  VirtIO blk with future PCI MSI) could hand a clone to each. Argument
  against: source-id aliasing is a hazard — two independent producers
  raising the same source-bit coalesce silently (by design, mirroring
  the Gateway's coalesce contract). Left open for REVIEW advice.
  Initial position: make `IrqLine` `Clone` and document the coalesce.
- **OQ-2** `pulse()` vs `raise() + lower()`: edge-triggered devices
  need a single API that latches without a racing `lower()`. Proposed
  semantics: `pulse()` performs `fetch_or` on `edge_latch` and a
  matching `fetch_or` on `level` that is **not** paired with a later
  `lower` — the next tick drains and clears. Reviewer should confirm
  this matches the Gateway's rising-edge contract (`edge_rising` in
  `gateway.rs:72-87`) under any-thread raise.
- **OQ-3** Should `Plic::tick()` be free-running (every bus tick) or
  gated by "signals changed since last tick"? Initial position:
  free-running — correctness trumps micro-optimization; the signal
  plane is two atomics so the fast-path cost is negligible.
- **OQ-4** Seam allowlist: `IrqLine` is arch-neutral but the PLIC
  factory `Plic::with_irq_line(src)` is on an arch-specific type.
  `SEAM_ALLOWED_SYMBOLS` already contains `"Plic"`; no new symbol is
  needed. Reviewer should confirm this reading.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Master (inherited) | archModule 00-M-002 | Honored | New arch-neutral module `src/device/irq.rs` sits outside `src/arch/`. PLIC factory stays under `src/arch/riscv/device/intc/plic/`. No arch-path leak. |
| Master (inherited) | archLayout 01-M-004 | Honored | `IrqLine` is the arch-neutral seam; the signal-plane implementation (`PlicSignals`) lives inside `src/arch/riscv/device/intc/plic/signals.rs`. |
| Master (inherited) | plicGateway 01 T4/T5 | Applied | This feature is the `T5` (directIrq) half of the boundary explicitly deferred by `plicGateway` 01. See `docs/fix/plicGateway/01_PLAN.md:186-193`. |
| plicGateway | I-9 (tick-only) | **Re-examined** | Superseded by I-D9 (Acquire/Release on `PlicSignals`). Rationale: the entire purpose of this feature is to admit cross-thread raises; retaining I-9 would block MANUAL #6. See `plicGateway/01_PLAN.md:904-906`. |
| plicGateway | OQ-2 (directIrq before edge adopter) | Answered | No in-tree device adopts edge in this feature. Edge stays exercised only by the Plic-boundary V-E-7 test from `plicGateway/02`. |
| Project Memory | `feedback_plan_subagent` | Applied | This PLAN is authored by the `plan-executor` sub-agent. |
| Project Memory | `feedback_create_templates` | Applied | Templates will be created at the start of each subsequent round. |
| Project Memory | `manual_review_progress` | Applied | Task #5+#6 is the next step after `aclintSplit → plicGateway`. |

> Rules:
> - No prior HIGH/CRITICAL findings (round 00). Inherited MASTER
>   directives are rendered verbatim per AGENTS.md §Response Rules.
> - The I-9 re-examination is rendered as a formal row because it is a
>   direction change from the prior approved plan, even though I-9 was
>   an invariant of a *different* feature.

---

## Spec

### Spec References

- **MANUAL_REVIEW.md:16-17** (item #5): "External devices(uart) should
  interact with PLIC directly. Currently, External devices(uart)
  interact with PLIC with bus which is incorrect."
- **MANUAL_REVIEW.md:19-20** (item #6): "Asynchronous interrupt
  handle, both of external device and interrupt hanler. External
  device enable irq will notify PLIC, PLIC will handle async. And
  PLIC receive irq will set meip and notify Interrupt handler,
  interrupt handler handle async."
- **RISC-V PLIC spec v1.0.0 §3 (Interrupt Gateways)** — gateways
  convert edge/level on the source wire into a single pending bit.
  This feature preserves the gateway FSM landed by `plicGateway`;
  only the *input* to the FSM changes (bitmap sample → atomic-backed
  device-driven signal).
- **QEMU reference** `hw/intc/sifive_plic.c` uses `qemu_irq`
  callbacks — each device is handed a per-source callable that, when
  invoked, drives the PLIC's line table. The `IrqLine` handle here
  is the Rust analogue: a cloneable, thread-safe handle carrying a
  source index and a shared reference to the signal plane.
- **Prior art in-tree**: `aclintSplit` feature uses `IrqState` (atomic
  `AtomicU64`) as the cross-device interrupt mailbox for
  MSIP/MTIP/SSIP/MEIP/SEIP. `IrqLine` follows the same pattern scaled
  from "CSR bitmap" to "PLIC source bitmap".

[**Goals**]

- **G-1** Every MMIO device that pends a PLIC source does so by
  calling an `IrqLine` handle it holds, not via a trait callback
  sampled by the bus.
- **G-2** The `IrqLine::raise()` path is safe to call from any
  thread. The UART reader thread (`uart.rs:94-129`) must be able to
  signal `rx-available` directly when bytes arrive, eliminating the
  one-tick latency currently imposed by `Uart::tick`.
- **G-3** PLIC evaluation preserves determinism: the guest-visible
  sequence of MEIP/SEIP assertions is a function of
  `(raise order, tick boundaries)` and is reproducible across runs
  with identical guest programs.
- **G-4** `Device::irq_line`, `Device::notify`, `Plic::notify`, and
  the bus bitmap fold are fully removed by the end of the feature.
- **G-5** The arch-isolation seam holds: `IrqLine` is arch-neutral;
  `SEAM_ALLOWED_SYMBOLS` does not grow. The signal plane
  implementation lives under `src/arch/riscv/device/intc/plic/`.
- **G-6** All prior gates stay green: `cargo test -p xcore` passes
  with ≥375 tests at every phase boundary; xv6, linux-2hart, and
  debian-2hart boot.

[**Non-Goals**]

- **NG-1** Lock-free PLIC core. The PLIC is still reached via the
  existing `Arc<Mutex<Bus>>` on the bus tick thread; only the
  *signal plane* is lock-free. A truly concurrent `Core::evaluate`
  is out of scope.
- **NG-2** Full per-hart threading. `xemu` remains single-threaded
  round-robin. Cross-thread raises come from device background
  threads only (UART reader today; VirtIO back-end workers are
  future work).
- **NG-3** Edge-triggered in-tree device. No device is promoted from
  level to edge by this feature. The Edge FSM from `plicGateway`
  stays exercised only by the Plic-boundary V-E-7 unit test.
- **NG-4** MSI/PCI/IOMMU. Only wire-level IRQ lines are modelled.
- **NG-5** LoongArch backend. LoongArch does not use PLIC; `IrqLine`
  is arch-neutral but its only wired implementation is RISC-V PLIC.
- **NG-6** Changing `Gateway` FSM semantics. The plicGateway I-3 /
  I-8 / I-11 invariants are binding. Every gateway transition
  already tested (`gateway.rs:118-212`, `plic/mod.rs:161-393`)
  keeps observational equivalence.
- **NG-7** Removing the Bus slow-tick divisor. `SLOW_TICK_DIVISOR`
  (bus.rs:218-224) keeps gating slow-device ticks; it just no
  longer drives a bitmap fold.

[**Architecture**]

```
        ┌────────────────────────────────────────────────────────┐
        │                       src/device/                      │
        │                                                        │
        │  uart.rs    virtio_blk.rs    ...                       │
        │    │            │                                      │
        │    │ IrqLine    │ IrqLine    (arch-neutral handle)     │
        │    ▼            ▼                                      │
        │  ┌──────────────────────────────────────────────────┐  │
        │  │  src/device/irq.rs — IrqLine { signals, src }    │  │
        │  │     raise() / lower() / pulse()                  │  │
        │  │     — Acquire/Release atomics on `signals`       │  │
        │  └──────────────────────────────────────────────────┘  │
        │                    │                                   │
        └────────────────────┼───────────────────────────────────┘
                             │ Arc<PlicSignals>
                             ▼
        ┌────────────────────────────────────────────────────────┐
        │           src/arch/riscv/device/intc/plic/             │
        │                                                        │
        │  signals.rs — PlicSignals {                            │
        │                 level: AtomicU32,                      │
        │                 edge_latch: AtomicU32,                 │
        │               }                                        │
        │                             ▲                          │
        │                             │ drain in tick()          │
        │                             │                          │
        │  mod.rs — Plic::tick()      │                          │
        │    for s in 1..NUM_SRC {                               │
        │      let lvl = signals.level.load(Acquire) & (1<<s);   │
        │      let edg = signals.edge_latch.fetch_and(!(1<<s),   │
        │                                              AcqRel);  │
        │      gateway[s].sample_with(lvl, edg != 0) → decision; │
        │      core.apply(decision);                             │
        │    }                                                   │
        │    core.evaluate();                                    │
        │                                                        │
        │  gateway.rs, core.rs, source.rs — unchanged            │
        │    (Gateway::sample signature may gain an `edge` bool) │
        └────────────────────────────────────────────────────────┘
```

Component responsibilities:

- **`IrqLine`** — arch-neutral handle. Holds `Arc<dyn IrqSignalPlane>`
  and a `src: u32`. Three methods: `raise()` (sets level bit),
  `lower()` (clears level bit), `pulse()` (sets level bit + edge
  latch, edge bit cleared by next tick's drain — see OQ-2). All
  three are `&self` and thread-safe.
- **`PlicSignals`** — PLIC-internal atomic signal plane. Two
  `AtomicU32` fields indexed by source id. Lives inside `plic/`
  module to preserve arch isolation.
- **`Plic::with_irq_line(src) -> IrqLine`** — factory, called at
  machine construction time to hand devices their line. Multiple
  calls with the same `src` return handles that alias the same bit
  (by design — coalesce contract).
- **`Plic::tick`** — new method, called from `Bus::tick` slow-path.
  Drains `PlicSignals` through each gateway, then calls
  `core.evaluate()`. Replaces `Plic::notify(u32)`.
- **Gateway / Core / Source** — unchanged semantically.
  `Gateway::sample` may be refactored to `sample_with(level: bool,
  edge_pulse: bool)` to admit both the level sample and a
  simultaneous edge-latch event from the signal plane; the
  currently-tested behavior is the `edge_pulse=false` projection.

Phase deltas:

- **Phase 1**: `IrqLine` added. `PlicSignals` added. `Plic::tick`
  added. `Plic::with_irq_line` factory added. UART migrated to hold
  an `IrqLine`; `Uart::irq_line` retained for the remaining devices
  still signaling via the bus. Both paths coexist. `Plic::notify`
  remains.
- **Phase 2**: VirtioBlk migrated. `Bus::tick`'s bitmap fold
  (`bus.rs:227-243`) removed. `Plic::notify` still callable by tests
  (deprecated), but no in-tree caller remains.
- **Phase 3**: `Device::irq_line`, `Device::notify`, `Plic::notify`,
  `MmioRegion::irq_source` all deleted. Trait surface shrinks.

[**Invariants**]

- **I-D1** An `IrqLine` handle is immutable after construction: its
  `src` field never changes. The only mutation is to the shared
  `Arc<PlicSignals>` via atomics.
- **I-D2** `IrqLine::raise()` is an idempotent set operation. Calling
  it N times is observationally identical to calling it once until
  `lower()` or the next level-drop transition observed by the
  gateway.
- **I-D3** `IrqLine::lower()` is an idempotent clear operation.
- **I-D4** `IrqLine::pulse()` guarantees that *at least one* rising
  edge is observed by the Edge gateway for the source, even if the
  producer thread is preempted between the `level.fetch_or` and the
  `edge_latch.fetch_or` — the `edge_latch` bit alone is sufficient to
  trigger `Gateway::sample_with(_, edge_pulse=true)` → `Pend`.
- **I-D5** `Plic::tick` drains the signal plane into gateways and
  calls `core.evaluate()` exactly once per call. It never yields a
  partial drain.
- **I-D6** Between `Plic::tick` calls, any number of `raise`/`lower`/
  `pulse` events may coalesce. This matches the Gateway's existing
  coalesce contract (plicGateway I-3) — no new semantics.
- **I-D7** `IrqLine` clones alias: two clones of the same line share
  the same bit in `PlicSignals`. Coalesce is by design.
- **I-D8** `Plic::reset` and `Plic::hard_reset` clear `PlicSignals`
  to all zeroes in addition to all the existing runtime state. The
  `Arc<PlicSignals>` pointer identity is preserved across reset so
  devices' existing `IrqLine` handles keep working.
- **I-D9** (supersedes plicGateway I-9) All `PlicSignals` reads and
  writes use `Acquire`/`Release`/`AcqRel`. The PLIC tick observes all
  raises sequenced-before the drain, per the happens-before edge
  established by `level.load(Acquire)` after `level.fetch_or(Release)`.
- **I-D10** `Plic::tick` is only called from the bus tick thread (the
  same thread that holds `Bus`'s mutex). `IrqLine::raise` is callable
  from any thread. No PLIC internal beyond `PlicSignals` is touched
  off-thread.
- **I-D11** Phase removal order is monotonic: a device either holds
  an `IrqLine` and no longer overrides `irq_line()`, or vice versa.
  No device is in both states simultaneously at a phase boundary.

[**Data Structure**]

```rust
// src/device/irq.rs (arch-neutral)

use std::sync::Arc;

/// Opaque handle for a single PLIC source wire. Arch-neutral.
/// Cloneable (I-D7); cheap Arc bump.
#[derive(Clone)]
pub struct IrqLine {
    plane: Arc<dyn IrqSignalPlane>,
    src: u32,
}

impl IrqLine {
    pub fn raise(&self)  { self.plane.raise(self.src); }
    pub fn lower(&self)  { self.plane.lower(self.src); }
    pub fn pulse(&self)  { self.plane.pulse(self.src); }
}

/// Arch-neutral trait implemented by the PLIC signal plane.
/// Only the PLIC implements this; the trait is the dyn seam so
/// `src/device/irq.rs` does not import `crate::arch::riscv::…`.
pub trait IrqSignalPlane: Send + Sync {
    fn raise(&self, src: u32);
    fn lower(&self, src: u32);
    fn pulse(&self, src: u32);
}
```

```rust
// src/arch/riscv/device/intc/plic/signals.rs

use std::sync::atomic::{AtomicU32, Ordering::*};
use crate::device::irq::IrqSignalPlane;

pub(super) struct PlicSignals {
    level: AtomicU32,
    edge_latch: AtomicU32,
}

impl PlicSignals {
    pub(super) fn new() -> Self { ... }

    /// Drain snapshot; returns (level_bits, edge_bits) and clears
    /// `edge_latch`. `level` is not cleared — it tracks the wire.
    pub(super) fn drain(&self) -> (u32, u32) {
        let lvl = self.level.load(Acquire);
        let edg = self.edge_latch.swap(0, AcqRel);
        (lvl, edg)
    }

    pub(super) fn reset(&self) {
        self.level.store(0, Release);
        self.edge_latch.store(0, Release);
    }
}

impl IrqSignalPlane for PlicSignals {
    fn raise(&self, src: u32) {
        self.level.fetch_or(1u32 << src, Release);
    }
    fn lower(&self, src: u32) {
        self.level.fetch_and(!(1u32 << src), Release);
    }
    fn pulse(&self, src: u32) {
        // Order: set level first so a concurrent drain that sees the
        // edge bit also sees the level (I-D4).
        self.level.fetch_or(1u32 << src, Release);
        self.edge_latch.fetch_or(1u32 << src, Release);
    }
}
```

```rust
// src/arch/riscv/device/intc/plic/mod.rs (delta vs current)

pub struct Plic {
    gateways: [Gateway; NUM_SRC],
    core: Core,
    signals: Arc<PlicSignals>,   // NEW
}

impl Plic {
    pub fn with_irq_line(&self, src: u32) -> IrqLine {
        assert!((1..NUM_SRC as u32).contains(&src));
        IrqLine::new(self.signals.clone(), src)
    }

    pub fn tick(&mut self) {
        let (lvl, edg) = self.signals.drain();
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
}
```

```rust
// Gateway delta (src/arch/riscv/device/intc/plic/gateway.rs)
//
// Current `sample(level: bool)` becomes `sample_with(level, edge)`.
// The level-only path is `sample_with(level, false)` and is
// byte-equivalent to the current `sample`. The edge branch treats
// `edge == true` as "force a rising-edge observation" regardless
// of `prev_level` — this is what PlicSignals.edge_latch carries.

impl Gateway {
    pub(super) fn sample_with(&mut self, level: bool, edge: bool)
        -> GatewayDecision
    {
        match self.kind {
            SourceKind::Level => self.sample_level(level),
            SourceKind::Edge  => self.sample_edge_signal(level, edge),
        }
    }

    fn sample_edge_signal(&mut self, level: bool, edge_pulse: bool)
        -> GatewayDecision
    {
        // Existing prev_level-derived rising edge OR explicit pulse.
        let rising = (level && !self.prev_level) || edge_pulse;
        self.prev_level = level;
        if !rising { return GatewayDecision::NoChange; }
        let suppress = self.in_flight || self.armed;
        self.armed = true;
        if suppress { GatewayDecision::NoChange } else { GatewayDecision::Pend }
    }
}
```

[**API Surface**]

```rust
// Public (arch-neutral):
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

// Public (arch-specific, already re-exported via seam):
impl Plic {
    pub fn with_irq_line(&self, src: u32) -> IrqLine;   // NEW
    pub fn tick(&mut self);                              // NEW
    // retired in Phase 3: pub fn notify(&mut self, u32);
}

// Device::Device trait (src/device/mod.rs) — Phase 3 delta:
//   removed: fn irq_line(&self) -> bool { false }
//   removed: fn notify(&mut self, _: u32) {}
//
// Bus::add_mmio — Phase 3 delta:
//   removed: irq_source: u32 parameter.
```

[**Constraints**]

- **C-1** File size cap ≤250 lines per file (inherited from
  plicGateway C-11; matches the user's many-small-files rule).
- **C-2** `SEAM_ALLOWED_SYMBOLS` in
  `xemu/xcore/tests/arch_isolation.rs` is byte-identical to its
  pre-feature state at every phase boundary. `IrqLine` and
  `IrqSignalPlane` are arch-neutral and live under
  `src/device/irq.rs`; they are *not* seam symbols.
  Enforcement: `git diff main -- xemu/xcore/tests/arch_isolation.rs`
  returns empty at each phase gate (inherited from plicGateway R-015
  / V-IT-1).
- **C-3** `BUS_DEBUG_STRING_PINS` in the same file: the `"plic"` and
  `"aclint"` string-literal counts are expected to stay at their
  current values (1 and 0 respectively) because those literals
  appear in test construction calls independent of `irq_source`.
  Verify and adjust the pin count only if a count actually changes
  in Phase 2 or 3.
- **C-4** No CSR vocabulary (`Mip`, `MEIP`, `SEIP`, bitflags) leaks
  into `src/device/irq.rs` or into device implementations.
- **C-5** No assembly edits (inherited project-wide constraint).
- **C-6** `cargo test -p xcore` test count grows monotonically
  across phases: Phase 1 ≥ +6, Phase 2 ≥ +4, Phase 3 neutral (tests
  targeting `Device::irq_line` and `Plic::notify` are deleted; their
  replacements are Phase 1/2 tests already counted).
- **C-7** No removal of `SLOW_TICK_DIVISOR` gating in `Bus::tick`
  (NG-7).
- **C-8** Each phase ships independently green against xv6,
  linux-2hart, and debian-2hart boot gates. A phase is not "done"
  until those boot.
- **C-9** `IrqLine` is not visible to guest code (obviously) and has
  no MMIO surface. `PlicSignals` is not visible to the guest.
- **C-10** `Plic::tick` must not allocate. It drains two atomics
  and loops over `NUM_SRC`.
- **C-11** `IrqLine::raise/lower/pulse` must not allocate. They are
  single `fetch_or` / `fetch_and` instructions (modulo the vtable
  dispatch from T-1 Option A).

---

## Implement

### Execution Flow

[**Main Flow**]

Runtime view (post-feature):

1. Machine construction (`machine/mod.rs` or equivalent):
   a. Create `Plic` with appropriate `num_harts`/`IrqState` arity.
   b. For each device needing an IRQ, call
      `plic.with_irq_line(src)` to obtain an `IrqLine`.
   c. Construct device with `Device::with_irq(line)`; register via
      `Bus::add_mmio(name, base, size, Box::new(dev))` — no
      `irq_source` argument (Phase 3).
2. Device signalling (any thread):
   a. Device computes a condition (e.g. UART reader thread pushes a
      byte into `rx_buf`; after push, calls `self.line.raise()`).
   b. `IrqLine::raise` performs one `fetch_or` on
      `PlicSignals.level` with `Release` ordering.
3. Bus tick (tick thread):
   a. `Bus::tick` ticks every device (fast path: MTIMER; slow path:
      remaining devices).
   b. `Bus::tick` calls `plic.tick()` (no more bitmap fold, post
      Phase 2).
4. PLIC tick (tick thread, under `Bus` mutex):
   a. `Plic::tick` calls `self.signals.drain()` →
      `(level_bits, edge_bits)` with `Acquire` / `AcqRel`.
   b. For each source `s in 1..NUM_SRC`, call
      `gateway[s].sample_with(level_bit, edge_bit)` and apply the
      `GatewayDecision` to `core`.
   c. `core.evaluate()` — unchanged semantics; drives MEIP/SEIP on
      `IrqState` per `plicGateway/02_PLAN` Design (a).
5. Guest claim/complete (tick thread, via MMIO):
   a. Unchanged from `plicGateway/02_PLAN`. `Plic::read(claim)` →
      `core.claim(ctx)` + `gateway.on_claim()`. `Plic::write(complete)`
      → `core.complete(ctx, src)` + `gateway.on_complete()`, re-pend
      via `core.set_pending + core.evaluate` on `GatewayDecision::Pend`.

[**Failure Flow**]

1. `IrqLine` call on a dropped PLIC: impossible by construction —
   `Arc<PlicSignals>` keeps the signal plane alive as long as any
   handle exists. The `Plic` itself may be dropped (unlikely in
   practice, but safe).
2. Raise from a thread racing with `Plic::reset`: `reset` zeroes
   `PlicSignals`; a concurrent `raise` may land its bit
   before or after the reset. This is indistinguishable from a
   raise that happens immediately after reset completes — both are
   acceptable guest-visible outcomes. Documented as an expected
   race, not a bug.
3. Raise with invalid `src` (0 or ≥ NUM_SRC): prevented by
   `Plic::with_irq_line`'s `assert!` at construction time.
   `IrqLine::raise` itself does no bounds check — the invariant is
   enforced at handle creation.
4. Signal plane bit set for a source with no registered device:
   harmless. `Plic::tick` still walks the gateway; if the gateway is
   disabled / threshold-gated, no pend lands. No crash.
5. PLIC held behind a poisoned mutex: this feature does not change
   the lock topology. Poisoning behavior is unchanged.

[**State Transition**]

Per-source signal-plane state (new), between `Plic::tick` calls:

- `level.bit(s) = 0` → `= 1`: by `raise()` or first half of `pulse()`.
- `level.bit(s) = 1` → `= 0`: by `lower()`. Not by `Plic::tick`.
- `edge_latch.bit(s) = 0` → `= 1`: by second half of `pulse()`.
- `edge_latch.bit(s) = 1` → `= 0`: by `Plic::tick`'s
  `edge_latch.swap(0, AcqRel)`.

Per-source gateway state (unchanged from plicGateway; input source
widens from "raw level" to "(level, edge_pulse)"):

| Source Kind | Input              | Effect on Gateway                                          |
|-------------|--------------------|------------------------------------------------------------|
| Level       | `(level, _)`       | Exact behavior of current `sample_level`.                  |
| Edge        | `(level, false)`   | Exact behavior of current `sample_edge` (poll).            |
| Edge        | `(level, true)`    | Force rising-edge observation; same downstream FSM as poll. |

Gateway claim/complete transitions unchanged (plicGateway
`gateway.rs:89-108`).

### Implementation Plan

[**Phase 1 — Handle + signal plane + UART adopter**]

Scope: opt-in migration. Both `IrqLine` and `Device::irq_line` paths
coexist. No in-tree removals.

Steps:

1. Add `src/device/irq.rs` with `IrqLine` + `IrqSignalPlane`
   (arch-neutral). ≤80 lines.
2. Add `src/arch/riscv/device/intc/plic/signals.rs` with
   `PlicSignals`, `drain`, `reset`, and the `IrqSignalPlane` impl.
   ≤80 lines.
3. Extend `Plic`:
   - `signals: Arc<PlicSignals>` field initialized in both
     constructors (`new` and `with_config`).
   - `with_irq_line(&self, src: u32) -> IrqLine` factory.
   - `tick(&mut self)` implementation per §Execution Flow step 4.
   - `Device::reset` / `hard_reset` also call `signals.reset()`
     (I-D8).
4. Extend `Gateway::sample_with(level: bool, edge: bool)`; retain
   the existing `sample(level)` as an inline call to
   `sample_with(level, false)` for in-place test compatibility.
5. Modify `Uart`:
   - New constructor variant: `with_irq(line: IrqLine) -> Self` (or
     similar builder — `Uart::with_stdio().with_irq(line)`).
   - After any state change that could flip `irq_line()` truthy,
     call `self.line.raise()`; after a state change that makes it
     falsy, call `self.line.lower()`. In practice: `raise` in the
     reader thread after pushing to `rx_buf`, in `tick` after
     THRE promotion, in guest reads/writes that flip conditions;
     `lower` in the symmetric branches.
   - Retain `fn irq_line(&self) -> bool` for the interim (so the
     bus bitmap still works for non-adopter devices).
6. Machine construction site: thread the `IrqLine` through. (No new
   seam symbol — `Plic` is already re-exported.)
7. Keep `Bus::tick`'s `irq_source`/bitmap fold unchanged. Also call
   `plic.tick()` after `plic.notify(bitmap)` so the signal-plane
   path is live in parallel. Both `Plic::notify` and `Plic::tick`
   are compatible with respect to each other: `notify` pushes bitmap
   → gateway; `tick` pushes signal plane → gateway. The union of
   decisions is still monotonic under the Gateway FSM, which is
   already tolerant of duplicate `sample(level)` calls within a tick.

Validation gate for Phase 1:

- `cargo test -p xcore` ≥ 375 + 6 new = ≥ 381 green.
- `cargo test -p xcore --test arch_isolation` green with unchanged
  `SEAM_ALLOWED_SYMBOLS`.
- Boot gate: xv6 + linux-2hart + debian-2hart.

[**Phase 2 — VirtioBlk adopter; retire `Bus::tick` bitmap fold**]

Scope: all in-tree signalling devices migrated. Trait surface still
carries `irq_line` / `notify`.

Steps:

1. Migrate VirtioBlk to hold an `IrqLine`. Replace
   `self.interrupt_status != 0` assertions-via-`irq_line` with
   explicit `raise()` / `lower()` calls at the status-update sites
   (`virtio_blk.rs` around line 232 and its callers).
2. Remove the bitmap fold from `Bus::tick`:
   - Drop the `.fold(0u32, …)` closure and `plic.notify(bitmap)` call.
   - Keep the per-device `tick()` calls.
   - Keep the `plic.tick()` call.
3. `MmioRegion::irq_source` is still present but unread. Mark
   `#[allow(dead_code)]` to keep Phase 2 green without a
   field-removal diff; Phase 3 removes it.
4. Keep `Device::irq_line` / `Device::notify` defaults in the trait.
   Their defaults already return `false` / `()`, so removing the
   overrides in individual impls is a no-op behaviorally.

Validation gate for Phase 2:

- `cargo test -p xcore` ≥ 381 + 4 new = ≥ 385 green. (Tests
  targeting `Device::irq_line` for UART/VirtioBlk — e.g.
  `uart.rs:406-413`, `virtio_blk.rs:310-312` — are reworked to
  assert `IrqLine`-driven state at the `Plic::with_irq_line(...)`
  boundary. Equivalents added in Phase 1 are already counted.)
- Boot gate: xv6 + linux-2hart + debian-2hart.

[**Phase 3 — Retire the legacy trait surface**]

Scope: clean-up. No new functionality.

Steps:

1. Delete `Device::irq_line` from the trait (default and all
   overrides).
2. Delete `Device::notify(u32)` from the trait (default and
   `Plic`'s override).
3. Delete `Plic::notify(u32)` impl on `Plic`.
4. Delete `MmioRegion::irq_source` and the `irq_source` parameter
   of `Bus::add_mmio`. Update call sites in machine construction.
5. Verify `BUS_DEBUG_STRING_PINS` in `arch_isolation.rs` still
   holds (expected: unchanged — C-3).
6. Re-run the full test suite and boot-test trio.

Validation gate for Phase 3:

- `cargo test -p xcore` green; net test count same or greater than
  Phase 2 (obsolete tests deleted, no regressions).
- `cargo clippy` clean.
- `cargo fmt --all` clean.
- `make run` (xv6 default), plus `make run` with the linux-2hart
  and debian-2hart configurations, all boot successfully.
- `git diff main -- xemu/xcore/tests/arch_isolation.rs` still
  empty (C-2).

## Trade-offs

- **T-1 Handle type shape.**
  - **Option A**: `IrqLine { plane: Arc<dyn IrqSignalPlane>, src: u32 }`
    — trait-object indirection keeps `src/device/irq.rs` arch-neutral
    (no `Arc<PlicSignals>` import). One vtable hop per `raise`.
  - **Option B**: `IrqLine { plane: Arc<PlicSignals>, src: u32 }`
    directly — no vtable, but `src/device/irq.rs` must import
    `crate::arch::riscv::device::intc::plic::signals::PlicSignals`,
    *breaking arch isolation* (C-2 failure).
  - **Option C**: Callback closure `IrqLine(Arc<dyn Fn() + Send + Sync>)`.
    One closure per line, per level-polarity — heavier allocation,
    harder to clone, harder to debug.
  - **Recommendation**: **Option A**. Vtable cost is negligible at
    bus-tick cadence; the isolation guarantee is worth more than
    one indirect call.

- **T-2 Signal plane representation.**
  - **Option A**: Two `AtomicU32` (`level`, `edge_latch`). Clean,
    cache-aligned, 8 bytes. `NUM_SRC ≤ 32`.
  - **Option B**: Per-source `AtomicBool`s — `NUM_SRC * 1 byte`,
    false sharing possible if placed without alignment.
  - **Option C**: `AtomicU64` carrying both halves interleaved.
    Atomicity between `level` and `edge_latch` matters only for the
    `pulse()` path, and even there the ordering is Release-Release
    on independent bits; a u64 doesn't buy anything.
  - **Recommendation**: **Option A**. Matches `IrqState`'s existing
    pattern (`AtomicU64`) scaled to u32.

- **T-3 Who owns `PlicSignals`.**
  - **Option A**: `Plic` owns it; `with_irq_line` hands out
    `Arc<PlicSignals>` as `Arc<dyn IrqSignalPlane>`.
  - **Option B**: Bus owns it, PLIC borrows. Forces an indirection
    through `Bus` on every tick; introduces a lock across the
    sampling loop.
  - **Recommendation**: **Option A**. The signal plane is a PLIC
    internal; placing it under `plic/` matches `archLayout`.

- **T-4 `pulse()` semantics.**
  - **Option A**: `pulse()` sets both `level` and `edge_latch`; the
    next `Plic::tick` drains edge bits (resetting them) but leaves
    level bits until the producer lowers. (Documented as:
    single-shot pulse with "sticky" level until acked.)
  - **Option B**: `pulse()` sets `edge_latch` only, never touches
    `level`; level sources cannot be pulsed.
  - **Option C**: `pulse()` sets `level`, and `Plic::tick` clears
    `level` for edge sources after consuming the pulse.
  - **Recommendation**: **Option A**. It composes: a device that
    only ever `pulse()`s will observe level=1 until it calls
    `lower()`, matching the "I'm done signaling" gesture. Edge
    gateways consume the rising edge anyway; subsequent
    `sample_with(level=1, edge=false)` is coalesced.

- **T-5 Evaluation cadence.**
  - **Option A**: `Plic::tick` runs every `Bus::tick` slow-path
    (every `SLOW_TICK_DIVISOR` bus ticks). Matches current
    `notify` cadence.
  - **Option B**: `Plic::tick` runs every bus tick. Higher IRQ
    latency floor, but costs two atomic loads per bus tick.
  - **Option C**: "Raise-triggered wake": `raise()` bumps a
    global epoch; tick polls only if the epoch advanced. Needs
    a third atomic; saves nothing at bus-tick cadence.
  - **Recommendation**: **Option A**. Keeps the existing latency
    profile. The real latency win is that the UART reader thread
    no longer waits for its next `Uart::tick` to be *sampled* —
    the raise lands in `PlicSignals` immediately; only PLIC
    evaluation waits for the next tick boundary.

## Validation

[**Unit Tests**]

- **V-UT-1** `IrqLine::raise` sets the corresponding bit in
  `PlicSignals.level`. Construct `PlicSignals`, wrap in
  `Arc<dyn IrqSignalPlane>`, create `IrqLine`, call `raise()`,
  assert `signals.level.load(Acquire) & (1 << src) != 0`.
- **V-UT-2** `IrqLine::lower` clears the corresponding bit.
- **V-UT-3** `IrqLine::pulse` sets both `level` and `edge_latch` bits.
- **V-UT-4** `IrqLine::clone` aliases: two clones of the same line
  both mutate the same bit.
- **V-UT-5** `PlicSignals::drain` returns `(level_snapshot,
  edge_snapshot)` and clears `edge_latch` but not `level`.
- **V-UT-6** `PlicSignals::reset` zeroes both fields.
- **V-UT-7** `Gateway::sample_with(level, false)` is byte-equivalent
  to the current `sample(level)` for every transition exercised by
  `gateway.rs:130-211` (sanity — refactor-preserving).
- **V-UT-8** `Gateway::sample_with(false, true)` on an Edge source
  emits `Pend` (forced rising-edge observation, I-D4). Compare
  against the current poll-derived rising-edge test.
- **V-UT-9** `Plic::tick` drains signals, applies gateway
  decisions, and calls `core.evaluate` — construct PLIC, call
  `with_irq_line(2)`, `raise()`, then `plic.tick()`, assert MEIP
  is asserted for ctx 0 after configuring enable + priority.

[**Integration Tests**]

- **V-IT-1** Cross-thread raise: spawn a thread that calls
  `line.raise()`; join; call `plic.tick()`; assert the raise is
  observed. Uses `Acquire/Release` happens-before (I-D9).
- **V-IT-2** Arch-isolation: `cargo test -p xcore --test arch_isolation`
  green. `git diff main -- xemu/xcore/tests/arch_isolation.rs`
  empty at every phase gate (C-2).
- **V-IT-3** UART end-to-end: construct a machine with UART holding
  an `IrqLine`, write a byte through the PTY, bus-tick repeatedly,
  assert MEIP is asserted before the `Uart::tick` one-tick latch
  would have previously observed it. Demonstrates G-2 (async raise
  latency reduction).
- **V-IT-4** VirtioBlk end-to-end (Phase 2): complete a DMA
  request; assert the `IrqLine::raise` lands in PlicSignals and the
  gateway pends.
- **V-IT-5** (Phase 3) The legacy `Device::irq_line` / `notify`
  trait methods are gone: grep over `src/device/` shows only
  `src/device/irq.rs` carrying `raise` / `lower` / `pulse`; no
  `fn irq_line` or `fn notify` remain.

[**Failure / Robustness Validation**]

- **V-F-1** Raise during reset: spawn a raiser thread that calls
  `line.raise()` in a tight loop; on the main thread, call
  `plic.reset()`. Assert that post-reset, *some* raise
  post-reset-completion is observed (handle is alive across reset,
  I-D8).
- **V-F-2** Raise with no registered gateway (source 31 with no
  enabled context): signal plane bit set, `Plic::tick` runs, no
  MEIP asserted, no panic.
- **V-F-3** Double-lower is idempotent: `line.lower(); line.lower();`
  leaves `level` bit clear with no error.
- **V-F-4** Double-raise is idempotent (I-D2): raise twice, tick
  once, observe a single `Gateway::sample_with(level=true)` →
  single `set_pending`.

[**Edge Case Validation**]

- **V-E-1** `IrqLine` for `src = 0` is rejected at construction
  (`Plic::with_irq_line` asserts). Test uses `std::panic::catch_unwind`
  or `#[should_panic]`.
- **V-E-2** `IrqLine` for `src >= NUM_SRC` rejected at construction.
- **V-E-3** `pulse()` on a Level source: level is set, edge_latch
  is set but ignored by `sample_level`. Observed behavior:
  equivalent to `raise()` followed by whatever the producer does
  next. Documented and tested.
- **V-E-4** Concurrent raise + tick: a raise lands in `PlicSignals`
  in between the `level.load(Acquire)` and the
  `edge_latch.swap(0, AcqRel)`. The raise is *either* observed by
  this tick or deferred to the next — both are acceptable. Asserted
  by repeating the race many times and checking that every raise
  is observed by at most one and at least one tick.
- **V-E-5** Phase-coexistence (Phase 1 only): a device still using
  the `Device::irq_line` path and a device using `IrqLine` coexist;
  the Gateway arbitrates between them correctly (each source gets
  one decision per tick). Uses UART-as-adopter + VirtioBlk-as-legacy.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (device → PLIC direct) | V-IT-3, V-IT-4, V-IT-5 |
| G-2 (any-thread raise)     | V-IT-1, V-IT-3, V-F-1 |
| G-3 (determinism)          | V-E-4, V-UT-7 |
| G-4 (legacy surface removed) | V-IT-5 (Phase 3) |
| G-5 (seam stable)          | V-IT-2 |
| G-6 (boot gates)           | Phase 1/2/3 boot-gate checklist |
| I-D1                       | V-UT-1..V-UT-4 (handle immutability observed) |
| I-D2, I-D3                 | V-F-3, V-F-4 |
| I-D4 (pulse latch)         | V-UT-3, V-UT-8 |
| I-D5 (drain atomicity)     | V-UT-5, V-UT-9 |
| I-D6 (coalesce)            | V-F-4, V-E-4 |
| I-D7 (clones alias)        | V-UT-4 |
| I-D8 (reset preserves Arc) | V-F-1 |
| I-D9 (Acquire/Release)     | V-IT-1, V-E-4 |
| I-D10 (tick-only PLIC)     | Design inspection + V-IT-1 (no panics) |
| I-D11 (monotonic migration)| Phase-gate diff inspection |
| C-1 (file size cap)        | File-size check at every phase gate |
| C-2 (seam stable)          | V-IT-2 |
| C-3 (bus debug pins)       | Re-run `arch_isolation` after Phase 2/3 |
| C-4 (no CSR leak)          | Grep + arch_isolation test |
| C-6 (test count monotone)  | `cargo test -p xcore` output per phase |
| C-8 (boot per phase)       | Boot-gate checklist per phase |
| C-10/C-11 (no alloc)       | Inspection + benchmark if available |

---

## Risks

- **Risk 1**: Double-signal path (Phase 1 keeps both bitmap fold and
  signal plane active) could produce different gateway-decision
  sequences than pure-legacy, breaking any golden-trace test if one
  exists. Mitigation: the gateway's existing coalesce contract
  (plicGateway I-3) makes duplicate `sample(level=true)` a no-op
  after the first, so monotonic union of decisions is identity on
  the pending bit. No golden-trace test is planned to pin bus-tick
  ordering.
- **Risk 2**: `Gateway::sample_with(level, false)` must be
  byte-faithful to the current `sample(level)` or we regress every
  plicGateway test. Mitigation: V-UT-7 is a refactor-preserving
  sanity test enumerating every currently-tested transition.
- **Risk 3**: `pulse()` semantics (T-4) interact with OQ-2 and are
  exercised by no in-tree device in this feature. A future edge
  adopter may find the Option A semantics surprising. Mitigation:
  V-UT-8 and V-E-3 pin the behavior at the FSM boundary; a future
  edge adopter has a test harness.
- **Risk 4**: Cross-thread raise (V-IT-1) depends on correct
  `Acquire/Release` pairing. A miscompile or ordering bug would be
  rare but hard to catch. Mitigation: I-D9 is named and tested;
  `loom`-based exhaustive interleaving is out of scope but reviewer
  may request it as follow-up if confidence is low.
- **Risk 5**: `MmioRegion::irq_source` removal (Phase 3) ripples
  through machine construction files. Mitigation: the ripple is
  mechanical and a `cargo check` will catch every site.

## Open Questions

See OQ-1..OQ-4 under [**Unresolved**] above.
