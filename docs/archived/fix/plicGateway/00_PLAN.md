# `plicGateway` PLAN `00`

> Status: Draft
> Feature: `plicGateway`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

Restructure the PLIC implementation along the canonical RISC-V PLIC
`source -> gateway -> core -> context` split, and replace the bus-driven
bitmap pump with a direct IRQ signaling handle (`PlicIrqLine`) that each
external device owns. The current monolithic `Plic` struct at
`xemu/xcore/src/arch/riscv/device/intc/plic.rs` conflates three roles
(gateway, arbitration core, per-context output), supports only
level-triggered sources, and is driven by a bitmap that the `Bus` collects
from every device on every tick (see `xemu/xcore/src/device/bus.rs:227-242`).
This plan addresses MANUAL_REVIEW items #5, #6, #7 by introducing a
gateway/core separation, a direct device->PLIC signaling API, and optional
per-source edge-trigger support — while keeping the existing 375-test
baseline green at every phase boundary.

This is the opening round of an 8-round iteration (00..07 PLAN/REVIEW,
MASTER at round 04). It presents the full feature strategy in phases;
subsequent rounds will narrow scope based on review/master feedback.

## Log

None in 00_PLAN.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| — | — | — | No prior REVIEW or MASTER for this feature. |

---

## Spec

### Problem Statement

Verbatim from `docs/MANUAL_REVIEW.md:16-24`:

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

## Implement

### Execution Flow

#### Main Flow

1. **Setup (boot time).** Board construction calls `Plic::new(num_harts,
   irqs)` (level defaults) or `Plic::with_config(..., sources)` (custom
   per-source edge/level). Board hands a `PlicIrqLine` to each device
   that needs one via `uart.with_irq(plic.irq_line(UART_SRC))`,
   `virtio_blk.with_irq(plic.irq_line(BLK_SRC))`.
2. **Device raises interrupt.** UART completes a read that leaves data
   in the FIFO and IER.rxne is set. `Uart::recompute_irq` calls
   `line.raise()`. This toggles one bit in `LineSignal.level` (atomic
   store-release).
3. **Bus tick reaches PLIC.** `Bus::tick` iterates MMIO devices; when
   it reaches PLIC it calls `plic.on_signal()` via `Device::tick`
   (Phase 2+) — no bitmap is collected from other devices.
4. **Gateway sampling.** `on_signal` loads the level snapshot and
   edge-latch snapshot, and for each source `s` in `1..NUM_SRC` invokes
   the appropriate gateway method:
   - Level source: `gateway[s].sample_level(level_bit)`.
   - Edge source: reads and clears the edge-latch bit; if set, calls
     `gateway[s].on_edge()`.
5. **Core update.** Gateway returns `Pend` / `NoChange` / `Clear`;
   `on_signal` applies to `core.pending` accordingly. `core.evaluate()`
   runs once at the end and drives MEIP/SEIP via `IrqState`.
6. **Hart reads claim.** Guest does an MMIO read at `0x200004 + c*0x1000`.
   `Core::claim(ctx)` selects the highest-priority enabled source above
   threshold, clears pending for that source, records `claimed[ctx]=s`,
   and calls `gateway[s].on_claim()` (which sets `in_flight=true`).
   Returns `s`.
7. **Hart writes complete.** Guest writes source ID to the same address.
   `Core::complete(ctx, s)` validates `claimed[ctx]==s`, clears it,
   calls `gateway[s].on_complete()` (clears `in_flight`, and if the
   gateway is still armed — level stayed high, or an edge latched
   during claim — re-pends into core).
8. **Evaluate + drive.** `core.evaluate()` re-runs; MEIP/SEIP edges are
   pushed to `IrqState`.

#### Failure Flow

1. **MMIO out-of-range read/write.** Same as today: return
   `Ok(0)` / `Ok(())`. The gateway/core split does not change the
   address decode in `Device::read/write`.
2. **Complete with wrong source.** `Core::complete` silently ignores if
   `claimed[ctx] != src` (preserves current behavior, `plic.rs:95-101`).
   Gateway is not touched in this case.
3. **Raise on unconfigured source (src=0 or src>=NUM_SRC).** `irq_line`
   panics in debug (`debug_assert!`) and returns a no-op handle in
   release. Existing PLIC already excludes source 0 from arbitration.
4. **Edge pulse while level source.** `pulse()` on a level-configured
   source is treated as a transient raise/lower that may or may not be
   observed on the next sample boundary. Documented and asserted by
   `debug_assert!(kind == Edge)` in dev builds.
5. **Reset mid-claim.** `Plic::reset` clears gateway, core, and the
   shared `LineSignal`. Any outstanding `PlicIrqLine` clones remain
   valid but their stored bits are cleared; devices that still hold
   raised lines must re-assert after reset (same contract devices
   already have for `IrqState::reset`).

#### State Transition

Per-source gateway FSM (for `SourceKind::Level`):

- `(armed=false, in_flight=false)` → `(armed=true)` when
  `sample_level(true)`; emit `Pend`.
- `(armed=true, in_flight=false)` → `(armed=false)` when
  `sample_level(false)`; emit `Clear`.
- `(armed=*, in_flight=false)` → `(armed=*, in_flight=true)` on
  `on_claim()` (core informs gateway when it successfully claims).
- `(armed, in_flight=true)` → `(armed', in_flight=false)` on
  `on_complete()`, where `armed'` reflects the latest sampled level;
  if `armed'` is true, emit `Pend` into core.

Per-source gateway FSM (for `SourceKind::Edge`):

- `(armed=false, in_flight=false)` → `(armed=true)` when `on_edge()`;
  emit `Pend`.
- `(armed=true, in_flight=false)`: further `on_edge()` coalesces,
  remains `(armed=true)`.
- `(_, in_flight=true)` → on `on_edge()`, set `armed=true` (pending-
  edge latch); emit `NoChange`.
- `on_complete()`: clear `in_flight`; if `armed`, emit `Pend`.

### Implementation Plan

The plan proposes three phases. Phase 1 is in-scope for the first few
rounds of this iteration; Phase 2 is the load-bearing MANUAL_REVIEW
deliverable; Phase 3 is a smaller follow-up that completes item #7.

#### Phase 1 — Internal split, behavior preserved

Scope: refactor `plic.rs` into `plic/{mod,source,gateway,core,line}.rs`
with identical external behavior. The bus bitmap pump still drives PLIC
through a thin adapter.

Steps:

1. Create `plic/` module directory; move the existing `Plic` struct into
   `plic/mod.rs` unchanged; re-export from the parent `intc` module.
   Verify `cargo test -p xcore` is 375/375.
2. Introduce `core.rs`: move `priority`, `pending`, `enable`,
   `threshold`, `claimed`, `num_ctx`, `irqs`, plus `claim`, `complete`,
   `is_claimed`, `evaluate`. `Plic` holds a `Core`. All 15 existing
   PLIC tests keep passing verbatim by delegation.
3. Introduce `gateway.rs`: implement `Gateway` + `GatewayDecision` for
   `SourceKind::Level` only. Plumb through `update()`: for each source
   `s`, `gateway[s].sample_level(bit)` → update `core.pending`.
   Introduce `on_claim`/`on_complete` hooks in `Core::claim` and
   `Core::complete` that call back into the correct gateway slot.
   Existing test `claimed_source_not_repended` now validates gateway
   claim-gating rather than the ad-hoc `is_claimed` filter.
4. Delete the now-redundant `Core::is_claimed` loop (replaced by
   gateway state). Validate again: 375/375.
5. Introduce `line.rs` with `PlicIrqLine`, `LineSignal`. Not yet wired
   to devices; `Plic::notify(bitmap)` remains and internally snapshots
   the bitmap into `LineSignal.level`, then runs the gateway pass. This
   keeps Phase 1 strictly additive.

Gate: 375/375 tests pass; clippy clean; xv6 + linux boot.

#### Phase 2 — Direct device signaling, remove bus bitmap pump

Scope: addresses MANUAL_REVIEW items #5 and #6.

Steps:

1. Add `Uart::with_irq(PlicIrqLine)` and `VirtioBlk::with_irq(PlicIrqLine)`.
   In their existing IRQ-recomputation points, call `raise()/lower()`
   instead of only updating internal flags. The `Device::irq_line`
   method stays on the trait but is no longer relied upon.
2. Update board wiring so that after constructing PLIC, the board calls
   `plic.irq_line(src)` and passes it to the device. Exact module path
   confirmed in round 01 after rebase check.
3. Change `Plic::notify` to a no-op deprecated shim (still there to
   keep the trait signature happy for other devices that haven't been
   migrated; the body does nothing). Add `Plic::on_signal(&mut self)`
   which runs the gateway+core pass against `LineSignal`.
4. Modify `Bus::tick`: remove the `irq_lines` bitmap collection loop
   (`src/device/bus.rs:227-242`). Replace with per-device `Device::tick()`
   (default no-op, overridden by `Plic` to call `on_signal`). This keeps
   the bus arch-neutral.
5. Add `PlicIrqLine` to `SEAM_ALLOWED_SYMBOLS` with comment:
   `// arch-neutral signaling handle; u32 source id, no CSR vocabulary`.
6. Keep a transitional test in `tests/bus_plic_signal.rs` that raises
   via `PlicIrqLine` and validates MEIP is asserted after one tick.

Gate: 375 + N new tests pass (N ≥ 3 for gateway level, gateway edge,
and direct-signaling integration); clippy clean; xv6, linux, linux-2hart
boot; am-tests and cpu-tests green.

#### Phase 3 — Per-source edge-trigger configuration

Scope: completes MANUAL_REVIEW item #7.

Steps:

1. Expose `Plic::with_config(num_harts, irqs, [SourceConfig; NUM_SRC])`.
   Default remains all `Level`.
2. Decide the configuration surface: either (a) a board-level constant
   table baked at construction, or (b) a sibling MMIO register block
   `0x003000 + 4*s` defining per-source kind. Option (a) is simpler and
   does not require spec extension; option (b) mirrors some proprietary
   PLIC variants but leaks a non-standard register into the memory
   map. **Proposed: option (a)** for round 00; revisit if any in-tree
   driver ever needs runtime reconfiguration.
3. Add gateway edge path: `on_edge()` as described in State Transition.
   Wire `PlicIrqLine::pulse()` to the `edge_latch` of `LineSignal`.
4. Add unit tests: edge pulse while not in-flight pends once; two
   pulses coalesce; pulse during claim latches and pends on complete;
   level source ignores (or transiently observes) pulse.

Gate: 375 + N tests pass; no in-tree device yet uses edge (virtio-blk
and UART remain level); clippy clean; full boot matrix green. Marks
item #7 complete.

---

## Trade-offs

- **T-1: Module split granularity.**
  Option A — keep `plic.rs` as one file and add a `Gateway` struct
  plus a `SourceKind` enum in-place (minimal churn; file grows to
  ~500 lines). Option B — split into `plic/{mod,source,gateway,core,
  line}.rs` as proposed (higher initial churn; each file 80-200
  lines, matches the "many small files" coding-style rule). Option B
  chosen because (a) the feature is explicitly about separating
  responsibilities that are currently entangled, and (b) reviewers
  have flagged the monolithic layout. Reviewer: acceptable?

- **T-2: Concurrency primitives in `LineSignal`.**
  Option A — `AtomicU32` (chosen); the only cost is a fence on
  raise/lower that is free on single-threaded x86_64/aarch64 hosts.
  Option B — `Cell<u32>` with `!Sync`; forbids cross-thread signaling
  forever. Option C — `Mutex<u32>`; takes the bus lock implicitly.
  Option A keeps the door open to future threading without adopting
  it now. Reviewer: does this match the project's trajectory?

- **T-3: Fate of `Device::irq_line`.**
  Option A — remove outright in Phase 2 (requires migrating every in-
  tree device in one shot; breaks any out-of-tree devices). Option B —
  mark `#[deprecated]` in Phase 2, remove in a later iteration.
  Option C — keep indefinitely as a fallback. Proposed **Option B**.
  Reviewer: confirm deprecation is preferred over immediate removal.

- **T-4: Where `on_signal` is called from.**
  Option A — new `Device::tick(&mut self)` default-no-op, PLIC
  overrides (arch-neutral; adds one trait method). Option B — Bus
  holds a typed `Arc<Mutex<Plic>>` handle (breaks arch isolation;
  bus learns about PLIC). Option C — devices call back into PLIC
  eagerly on raise/lower (requires a back-pointer in `LineSignal`;
  extra Arc cycles; breaks the "evaluate at tick boundary" invariant
  helpful for determinism). Proposed **Option A**.

- **T-5: Eager vs deferred evaluation.**
  Option A — PLIC evaluates only at tick boundary (chosen; preserves
  bus determinism and guest-visible timing). Option B — PLIC evaluates
  on every `raise`/`lower` call (lower latency; changes observable
  event ordering; risks re-entrancy because raise may happen while Bus
  holds `Bus` lock in multiHart). Option A matches the inherited
  single-threaded round-robin model (NG-2).

## Validation

### Unit Tests

- **V-UT-1** All 15 existing PLIC tests (`plic.rs:183-358`) pass
  verbatim after Phase 1 split. Verified by `cargo test -p xcore
  arch::riscv::device::intc::plic`.
- **V-UT-2** `gateway_level_raise_pends` — gateway level path: sample
  true pends core, sample false clears core.
- **V-UT-3** `gateway_level_claim_gates_repend` — sample true → pend;
  claim; sample true again (line still high); core.pending bit stays
  clear until complete.
- **V-UT-4** `gateway_level_complete_with_line_low` — complete while
  line is low: no re-pend.
- **V-UT-5** `gateway_level_complete_with_line_high` — complete while
  line is high: re-pend.
- **V-UT-6** `gateway_edge_pulse_pends_once` — Phase 3 only; two
  edges while not in-flight result in one pending.
- **V-UT-7** `gateway_edge_pulse_during_claim_latches` — Phase 3 only;
  edge during claim latches and pends on complete.
- **V-UT-8** `line_handle_raise_lower_idempotent` — multiple raise()
  calls on an already-raised level source do not multi-pend.
- **V-UT-9** `line_handle_source_zero_panics_in_debug` — debug_assert
  catches `irq_line(0)`.
- **V-UT-10** `core_priority_unchanged` — regression: all arbitration
  tests produce identical outputs post-split.

### Integration Tests

- **V-IT-1** `tests/bus_plic_signal.rs` (new in Phase 2) — construct
  Bus + Plic + mock device; mock calls `line.raise()`; one bus tick
  later, `IrqState.load() & MEIP != 0`.
- **V-IT-2** `cargo test -p xcore arch_isolation` — confirms only
  `PlicIrqLine` crosses the seam, no CSR bits leak.
- **V-IT-3** am-tests suite — must be 100% green, unchanged set.
- **V-IT-4** cpu-tests suite — unchanged pass set.
- **V-IT-5** xv6 boot: `DEBUG=n` run of xv6 image boots to shell and
  responds to one console input (UART source exercises the new path
  end-to-end).
- **V-IT-6** linux boot: single-hart linux boot completes to userspace
  with virtio-blk (block source through new path).
- **V-IT-7** linux-2hart boot: both harts reach userspace; PLIC per-
  context routing unchanged by the split.

### Failure / Robustness Validation

- **V-F-1** Reset mid-claim clears gateway and core; subsequent
  raise()+tick re-pends correctly.
- **V-F-2** Complete with wrong source ID leaves state intact (current
  test `complete_wrong_source_no_change` extended to also assert that
  the gateway's `in_flight` for the originally claimed source remains
  set).
- **V-F-3** MMIO out-of-range read returns 0; out-of-range write is
  silently dropped (current behavior).
- **V-F-4** Raising a line with a dropped PLIC (handle outlives
  controller) stores into the shared `LineSignal` with no observer
  (benign; no panic, no UB).

### Edge Case Validation

- **V-E-1** Raise twice in a row without tick — only one pending.
- **V-E-2** Raise then lower without tick — tick observes level low,
  no pending.
- **V-E-3** Raise and immediately pulse (if misconfigured) — behavior
  documented in Failure Flow #4 and asserted by test.
- **V-E-4** Two contexts enable the same source; one claims; the other
  sees pending cleared on first claim; complete re-pends only if line
  still high (same contract as today; verify unchanged).
- **V-E-5** `NUM_SRC` boundary: source 31 (the highest) signals and
  completes correctly; source 0 never pends.
- **V-E-6** Per-hart routing (existing test
  `plic_new_num_harts_two_ctx2_routes_to_irq1`) passes unchanged
  post-split.

### Acceptance Mapping

| Goal / Constraint | Validation                                         |
|-------------------|----------------------------------------------------|
| G-1               | V-UT-1, V-UT-2, V-UT-3, V-UT-10                    |
| G-2               | V-IT-1, V-UT-8, V-IT-5, V-IT-6                     |
| G-3               | V-UT-6, V-UT-7 (Phase 3)                           |
| G-4               | V-UT-1, V-IT-3, V-IT-4, V-IT-5, V-IT-6, V-IT-7     |
| G-5               | V-UT-1..10 + V-IT-1..7 all green; 375 baseline held|
| I-1               | V-UT-1, V-UT-10, V-E-6                             |
| I-2               | V-UT-3, V-UT-4, V-UT-5                             |
| I-3               | V-UT-6, V-UT-7                                     |
| I-4               | V-E-5                                              |
| I-5               | V-E-6                                              |
| I-6               | compile-time `assert_impl_all!` in unit tests      |
| I-7               | V-IT-2                                             |
| C-1               | V-UT-1                                             |
| C-2               | every gate: 375/375                                |
| C-3               | V-IT-2                                             |
| C-4               | V-IT-2                                             |
| C-5               | grep assertion in V-IT-1: `Bus::tick` no longer    |
|                   | calls `irq_line()` on devices                      |
| C-6               | `git diff` shows no `.s`/`.S` modifications        |
| C-7               | DTB diff against main is empty                     |
| C-8               | `rg 'unsafe' xemu/xcore/src/arch/riscv/device/intc` |
| C-9               | `cargo clippy --workspace -- -D warnings` in CI    |

---

## Gates

At each phase boundary the following must pass:

1. `cargo check -p xcore`
2. `cargo clippy --workspace -- -D warnings`
3. `cargo test -p xcore` — 375 baseline (Phase 1) / 375 + new (Phase 2+)
4. `cargo test --workspace`
5. xv6 boot (`DEBUG=n`), linux boot, linux-2hart boot — smoke
6. am-tests and cpu-tests unchanged

## Risks and Open Questions

- **Risk 1** `Bus::tick` restructuring touches multiHart code
  (`store(HartId, ...)` chokepoint for LR/SC peer invalidation — see
  prior multiHart feature). Adding `Device::tick` must not collide
  with any hart-local state. Flagged for review.
- **Risk 2** Devices whose IRQ condition depends on internal state
  that changes without an external write (e.g., a timer) would need
  to call `line.raise()` from their own `tick()`. This plan assumes
  no such device exists in-tree beyond UART FIFO drain (which already
  recomputes on each MMIO access). To confirm in round 01.
- **Risk 3** `Device::irq_line` has a default implementation returning
  `false`. Removing it later (Trade-off T-3 Option A) would touch every
  device. Deprecation route (T-3 Option B) is chosen for round 00.
- **Open Q 1** Should `Plic::on_signal` be exposed via `Device::tick`
  (arch-neutral) or via a dedicated `IntcController` trait? Deferred
  to round 01.
- **Open Q 2** Should `SourceConfig` be per-hart or global? Spec says
  global (one kind per source across all contexts); proposed global.
  Confirm.
- **Open Q 3** `Plic::notify(bitmap)` legacy path: remove in Phase 2
  or keep as a deprecated shim through the end of the iteration?
  Proposed: remove in Phase 2; grep confirms no external caller.
- **Open Q 4** Should we grow `NUM_SRC` to the spec-canonical 1023?
  Out of scope here (NG-5), but flagging for a possible follow-up
  feature.
