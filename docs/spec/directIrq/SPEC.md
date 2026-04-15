# `directIrq` SPEC

> Source: base spec from [`/docs/archived/fix/directIrq/00_PLAN.md`](/docs/archived/fix/directIrq/00_PLAN.md),
> with subsequent delta amendments in rounds up to [`/docs/archived/fix/directIrq/02_PLAN.md`](/docs/archived/fix/directIrq/02_PLAN.md).
> Iteration history and trade-off analysis live under `docs/archived/fix/directIrq/`.

---


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
