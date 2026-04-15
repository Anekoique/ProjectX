# `plicGateway` SPEC

> Source: base spec from [`/docs/archived/fix/plicGateway/00_PLAN.md`](/docs/archived/fix/plicGateway/00_PLAN.md),
> with subsequent delta amendments in rounds up to [`/docs/archived/fix/plicGateway/02_PLAN.md`](/docs/archived/fix/plicGateway/02_PLAN.md).
> Iteration history and trade-off analysis live under `docs/archived/fix/plicGateway/`.

---


### Problem Statement

Verbatim from `docs/archived/review/MANUAL_REVIEW.md:16-24`:

> 5. External devices(uart) should interact with PLIC directly.
>    Currently, External devices(uart) interact with PLIC with bus which is incorrect.
>
> 6. Asynchronous interrupt handle, both of external device and interrupt hanler.
>    External device enable irq will notify PLIC, PLIC will handle async. And PLIC
>    receive irq will set meip and notify Interrupt handler, interrupt handler
>    handle async.
>
> 7. Consider Better design to PLIC, which should both support level-triggled and
>    egde-triggled interrupt.
>    Consider add the semantic of Gateways and PLIC-Core which response for
>    different parts of PLIC.
>    `device/source -> per-source gateway -> PLIC core -> hart context` Consider
>    a good design of gateways to handle the source correctly.

Concrete deficiencies in the current code:

- `Bus::tick` polls every device's `irq_line()`, ORs them into a bitmap,
  and calls `Plic::notify(bitmap)` (`xemu/xcore/src/device/bus.rs:227-242`).
  This couples the bus to interrupt semantics that belong to the PLIC and
  forces every MMIO tick to perform N boolean polls even if no source
  changed.
- `Plic` owns a single `pending: u32` and merges all N lines via
  `update()` (`plic.rs:56-68`). Gateway state (level detect vs edge latch,
  claim-in-flight gating) and core state (priority arbitration, per-context
  filtering) share the same struct.
- Edge-triggered sources cannot be modeled: if a device raises and drops
  its line between two ticks, the PLIC never sees the pulse because
  `update()` only latches the current level. Real PCIe/MSI-like devices
  and future UART FIFO-overflow events require edge semantics.
- UART (`src/device/uart.rs:326`) and virtio-blk (`src/device/virtio_blk.rs:232`)
  expose `irq_line(&self) -> bool` only. They cannot signal a change; they
  can only be polled.

### Spec References

- RISC-V PLIC Specification v1.0.0 (2023), sections 1, 4 (Interrupt
  Sources), 5 (Interrupt Gateways), 6 (Priorities), 7 (Interrupt Flow
  and Core), 9 (Interrupt Targets and Hart Contexts). Key points this
  plan relies on:
  - Gateway converts global interrupt signals into a single interrupt
    request to the PLIC core; it is responsible for level/edge
    conversion and for suppressing further requests until the current
    one is claimed and completed (spec §5).
  - Core performs arbitration: selects the highest-priority pending
    enabled source per target (spec §7).
  - Claim returns the highest-priority pending enabled source above
    threshold and atomically clears pending for that source; completion
    is a write of the source ID to the claim/complete register, which
    re-arms the gateway (spec §7).
- SiFive FU540/FU740 PLIC memory map (register offsets already mirrored
  in `plic.rs:11-19`). No layout changes.

### Goals

- **G-1** Separate the PLIC into three crate-internal components with
  clear responsibilities: `Source` (per-interrupt signal carrier),
  `Gateway` (per-source level/edge conversion and claim-gating), `Core`
  (priority arbitration + per-context enable/threshold/claim/complete),
  exposed externally as one `Plic` `Device` with the same MMIO surface.
- **G-2** Provide an arch-neutral `PlicIrqLine` handle that external
  devices (UART, virtio-blk) own and call to assert/deassert their
  source. Bus no longer polls `irq_line()` nor forwards bitmaps.
- **G-3** Support both level-triggered and edge-triggered sources on a
  per-source basis, configurable at PLIC construction. Default remains
  level-triggered (preserves current UART/virtio-blk behavior).
- **G-4** Preserve the existing MMIO register layout, claim/complete
  semantics, and per-hart MEIP/SEIP routing exactly. No guest-visible
  behavior change for existing workloads (xv6, linux, linux-2hart,
  am-tests, cpu-tests).
- **G-5** Keep the 375-test green baseline at every phase boundary; add
  new tests for gateway level vs edge and for the signaling handle.

### Non-Goals

- **NG-1** No OS-thread concurrency. NG-2 below rules out async runtimes
  or worker threads; "async" in MANUAL_REVIEW item #6 is interpreted as
  *event-driven signaling* (device pushes an event; PLIC evaluates at
  the next tick or on signal), not `tokio` or `std::thread`.
- **NG-2** xemu remains single-threaded with the round-robin hart driver
  in `Bus::tick`. This plan does not change the execution model.
- **NG-3** No change to MSWI/MTIMER/SSWI, nor to the M-mode/S-mode trap
  delivery path beyond what PLIC already does via `IrqState`.
- **NG-4** No change to DTB generation. PLIC `compatible`, reg, and
  `interrupts-extended` stay as they are.
- **NG-5** No change to the number of sources (`NUM_SRC = 32`). A
  separate follow-up may raise this to the PLIC-canonical 1023; out of
  scope here because it touches register sizing.
- **NG-6** No change to `seam` tests' symbol allowlist beyond the
  minimum required for `PlicIrqLine` to cross the `src/device/` boundary
  (see Constraints C-3).

### Architecture

Target module layout under `xemu/xcore/src/arch/riscv/device/intc/plic/`:

```
plic/
├── mod.rs          // Plic struct, Device impl, public API (new, irq_line)
├── source.rs       // SourceKind { Level, Edge }, SourceConfig
├── gateway.rs      // Gateway: per-source level/edge state + claim gate
├── core.rs         // Core: priority[], pending, enable[], threshold[],
│                   // claimed[], arbitration, MEIP/SEIP drive
└── line.rs         // PlicIrqLine: arch-neutral handle exported to devices
```

The public single entry remains `Plic` in `mod.rs`; `Source`, `Gateway`,
`Core` are crate-internal. `PlicIrqLine` is the only new item crossing
into `src/device/`.

Runtime data flow (replaces the bus bitmap path):

```
device (UART / virtio-blk)
   │  line.raise() / line.lower()
   ▼
PlicIrqLine (shared handle, Arc<LineSignal>)
   │
   ▼
Plic::on_signal()  (invoked by Bus::tick once per tick; OR called
                    directly by device for eager signaling)
   │  per source: gateway.sample(level) -> maybe_pend
   ▼
Gateway[s]  (level/edge FSM; suppresses while claim-in-flight)
   │  gateway -> core.set_pending(s) when armed
   ▼
Core        (priority arbitration, per-context selection)
   │  core.evaluate() -> IrqState.set(MEIP|SEIP) per hart
   ▼
IrqState (shared with hart)
```

MMIO layout (unchanged):

| Offset range         | Meaning                                   |
|----------------------|-------------------------------------------|
| `0x000000..0x000080` | priority[0..32], u32 per source           |
| `0x001000`           | pending bitmap (read-only summary)        |
| `0x002000 + c*0x80`  | enable[c], 32-bit bitmap per context      |
| `0x200000 + c*0x1000`| threshold[c], u32                         |
| `0x200004 + c*0x1000`| claim/complete[c], u32                    |

### Invariants

- **I-1** (Arbitration equivalence) For every guest-observable sequence
  of MMIO accesses and device signals, the stream of values returned
  from claim reads and the stream of MEIP/SEIP edges driven onto
  `IrqState` is identical to the current monolithic `Plic`, provided
  all sources are configured as `SourceKind::Level` (the default).
- **I-2** (Claim gating) Once a source `s` is claimed by any context,
  the gateway for `s` must not re-pend until a matching complete
  arrives. Re-assertion of the level line while claimed is recorded by
  the gateway but held back from the core.
- **I-3** (Edge semantics) For `SourceKind::Edge`, exactly one pending
  event is generated per rising edge observed on the line. A rising
  edge while claim is in flight is latched in the gateway and released
  to the core on complete. Spurious multiple rising edges while a
  pending/claim is outstanding coalesce to one (matches PLIC spec §5).
- **I-4** (Source 0 reserved) Source 0 never pends, never claims, never
  completes. Current behavior (`(1..NUM_SRC)`) preserved.
- **I-5** (Context routing) Context `c` targets hart `c>>1`; even `c`
  drives MEIP, odd `c` drives SEIP. Existing behavior from
  `plic.rs:107-130`.
- **I-6** (Handle cloning is cheap) `PlicIrqLine` is `Clone + Send +
  Sync` and does not require locking the PLIC to signal. (Under NG-2
  single-threaded execution `Send+Sync` is not load-bearing at runtime,
  but the trait bounds keep the API forward-compatible with future
  threaded work and match the existing `IrqState` precedent.)
- **I-7** (Arch isolation) No RISC-V CSR vocabulary (`Mip`, `MEIP`,
  `SEIP`) escapes the `arch/riscv/` subtree. `PlicIrqLine` exposes only
  `raise/lower/pulse` + opaque source id.

### Data Structure

```rust
// arch/riscv/device/intc/plic/source.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum SourceKind {
    Level,
    Edge,
}

#[derive(Clone, Copy, Debug)]
pub(super) struct SourceConfig {
    pub kind: SourceKind,
}

// arch/riscv/device/intc/plic/gateway.rs
pub(super) struct Gateway {
    kind: SourceKind,
    // Last sampled level (for edge detection).
    prev_level: bool,
    // Pending in gateway waiting to enter core (one-slot latch).
    armed: bool,
    // True while the source is claimed somewhere (core tells us).
    in_flight: bool,
}

pub(super) enum GatewayDecision {
    Pend,      // pass to core
    NoChange,  // nothing to do
    Clear,     // core should drop pending (level went low, not claimed)
}

impl Gateway {
    pub(super) fn sample_level(&mut self, level: bool) -> GatewayDecision;
    pub(super) fn on_edge(&mut self) -> GatewayDecision;
    pub(super) fn on_claim(&mut self);
    pub(super) fn on_complete(&mut self) -> GatewayDecision;
}

// arch/riscv/device/intc/plic/core.rs
pub(super) struct Core {
    priority: [u8; NUM_SRC],
    pending: u32,
    enable: Vec<u32>,
    threshold: Vec<u8>,
    claimed: Vec<u32>,
    num_ctx: usize,
    irqs: Vec<IrqState>,
}

// arch/riscv/device/intc/plic/line.rs  (arch-neutral export)
#[derive(Clone)]
pub struct PlicIrqLine {
    source: u32,
    signal: Arc<LineSignal>,
}

struct LineSignal {
    // Bit per source, high = asserted.
    level: AtomicU32,
    // Rising-edge latch for edge-configured sources.
    edge_latch: AtomicU32,
}

// arch/riscv/device/intc/plic/mod.rs
pub struct Plic {
    gateways: [Gateway; NUM_SRC],
    core: Core,
    signal: Arc<LineSignal>,
}
```

Notes on atomics: under NG-2 we do not strictly need atomics, but
`IrqState` already uses `Arc<AtomicU32>`; matching that shape avoids a
new lock type and makes the handle cheap to clone into devices that
already hold `Arc`s.

### API Surface

Crate-internal (unchanged to consumers):

```rust
// arch/riscv/device/intc/plic/mod.rs
impl Plic {
    pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self;
    pub fn with_config(
        num_harts: usize,
        irqs: Vec<IrqState>,
        sources: [SourceConfig; NUM_SRC],
    ) -> Self;

    /// Allocate a signaling handle for a source. Devices clone this
    /// and call raise/lower/pulse.
    pub fn irq_line(&self, source: u32) -> PlicIrqLine;

    /// Sample all configured sources and drive MEIP/SEIP.
    /// Called once per bus tick.
    pub fn on_signal(&mut self);
}

impl Device for Plic {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, val: Word) -> XResult;
    fn reset(&mut self);
    fn tick(&mut self) { self.on_signal() } // new default-no-op trait method
}
```

Arch-neutral (new, crosses into `src/device/`):

```rust
// arch/riscv/device/intc/plic/line.rs (re-exported at arch boundary)
impl PlicIrqLine {
    /// Assert a level line. Idempotent.
    pub fn raise(&self);
    /// Deassert a level line. Idempotent.
    pub fn lower(&self);
    /// Fire a single edge pulse (for SourceKind::Edge sources).
    pub fn pulse(&self);
    /// Source ID this line binds to (opaque; for debug/logging).
    pub fn source(&self) -> u32;
}
```

Device wiring change (sketch):

```rust
// src/device/uart.rs
pub struct Uart {
    // ... existing fields ...
    irq: Option<PlicIrqLine>,
}

impl Uart {
    pub fn with_irq(mut self, line: PlicIrqLine) -> Self {
        self.irq = Some(line);
        self
    }

    fn recompute_irq(&self) {
        if let Some(line) = &self.irq {
            if self.line_condition() { line.raise() } else { line.lower() }
        }
    }
}
```

The existing `Device::irq_line(&self) -> bool` default returns `false`
and becomes unused for UART/virtio-blk; we *keep* the trait method in
round 00 and mark it deprecated in round 01 once all in-tree devices
have migrated. Final removal considered in round 02 or later.

### Constraints

- **C-1** (MMIO compatibility) Every existing PLIC test in
  `arch/riscv/device/intc/plic.rs` (15 tests, lines 183-358) must
  continue to pass unchanged in Phase 1; moved under
  `plic/mod.rs#[cfg(test)]` in Phase 1 without modification.
- **C-2** (Baseline) 375-test green baseline must hold at every phase
  boundary. New tests are additive.
- **C-3** (Seam) `PlicIrqLine` is the only new symbol crossing from
  `arch/riscv/device/intc/` into `src/device/`. It must be added to
  `SEAM_ALLOWED_SYMBOLS` in `xemu/xcore/tests/arch_isolation.rs` with a
  justification comment. `SourceKind`, `SourceConfig`, `Gateway`,
  `Core`, `GatewayDecision`, `LineSignal` remain `pub(super)` / crate-
  private and do not cross the seam.
- **C-4** (No CSR leakage) `PlicIrqLine` must not reference `Mip`,
  `MEIP`, `SEIP`, or any bit constant from `arch/riscv/cpu/trap/`.
  Source IDs are opaque `u32`.
- **C-5** (Bus simplification) `Bus::tick` must lose the bitmap
  collection loop at `src/device/bus.rs:227-242` no later than Phase 2.
  Replacement: Bus calls a single `Device::tick()` per MMIO entry;
  `Plic` overrides it to run `on_signal()`.
- **C-6** (No assembly changes) No modifications to `.S` / `.s` files.
- **C-7** (No DTB change) `compatible = "riscv,plic0"` and register
  ranges are preserved.
- **C-8** (No unsafe) No new `unsafe` blocks in this feature. Atomic
  operations use `core::sync::atomic`.
- **C-9** (Rustfmt + clippy clean) `cargo clippy --workspace -- -D
  warnings` must pass at each phase.

---
