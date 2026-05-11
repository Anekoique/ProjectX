# `plicGateway` PLAN `01`

> Status: Revised
> Feature: `plicGateway`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: none

---

## Summary

Round 01 **narrows** plicGateway to MANUAL_REVIEW items #6 + #7 only and
defers MANUAL_REVIEW #5 (direct device→PLIC signaling) to a separate
`directIrq` feature, per R-001's preferred resolution and the saved
project-memory task decomposition
(`project_manual_review_progress.md:14-16,20`). The feature is now a
**two-phase, in-place** refactor of
`xemu/xcore/src/arch/riscv/device/intc/plic.rs`:

- Phase 1 — split the monolithic `Plic` into internal `Gateway` + `Core`
  components under a `plic/{mod,core,gateway,source}.rs` module layout,
  preserving today's external behavior (including the Bus bitmap pump and
  SiFive-variant level-trigger semantics).
- Phase 2 — add per-source edge-trigger configuration on top of the new
  Gateway FSM; all existing sources continue to default to level.

The `PlicIrqLine` handle, `Bus::tick` bitmap-pump removal, and UART /
virtio-blk migration to direct signaling — all of which 00_PLAN folded
into a Phase 2 — are **out of scope** for this feature. They are
deferred verbatim to the `directIrq` feature, which will claim MANUAL
#5 and re-use the gateway/core split landed here.

Net behavior change under the narrowed scope: none for guests. The
375-test baseline holds at every boundary; Phase 2 adds edge-trigger
unit tests but no in-tree source adopts edge yet.

## Log

[**Feature Introduce**]

Round 01 re-scopes the feature to fit the inherited 5-round cap and the
T4/T5 task split. The architectural story is now "the monolith is
re-expressed as Gateway + Core, and Gateway gains an Edge variant that
sits unused until a future driver wants it." No cross-module seam
change, no arch_isolation allowlist change, no Bus change.

New invariants introduced to respond to review findings:

- **I-8** — SiFive-variant pre-claim level-low clears (explicit
  deviation from PLIC v1.0.0 §5), per R-002.
- **I-9** — concurrency posture: all PLIC state is touched from the bus
  tick thread only; no cross-thread `raise()` path exists in this
  feature, per R-003 option (b). Atomics are dropped.
- **I-10** — gateway evaluation runs inside `Plic::notify` on the
  existing bus bitmap-pump path; no `Device::tick` repurposing, per
  R-004.
- **I-11** — `Plic::reset` preserves per-source configuration
  (`SourceConfig`), per R-007.

[**Review Adjustments**]

Every blocking finding is resolved in the narrowed-scope direction:
R-001 by deferring directIrq work, R-002 by naming the SiFive deviation
as I-8, R-003 by committing to tick-thread-only signaling (dropping
atomics), R-004 by keeping the existing bitmap-pump path so tick
ordering is a non-issue. Non-blocking findings R-005..R-011 are
addressed below, with several (R-006, R-008, R-010) simplifying or
becoming moot under the narrowed scope.

[**Master Compliance**]

No plicGateway-local MASTER directives exist. Inherited MASTER
directives from prior features (archModule 00-M-001/002, archLayout
01-M-001..004) are acknowledged in the Response Matrix. The narrowed
scope leaves the `src/device/` ↔ `arch/riscv/` seam untouched, which
trivially honors 01-M-004.

### Changes from Previous Round

[**Added**]

- Invariants I-8 (SiFive level pre-claim clear), I-9 (tick-thread-only
  signaling), I-10 (gateway evaluation inside `notify`), I-11 (reset
  preserves `SourceConfig`).
- Explicit scope-revision section enumerating what moved to directIrq.
- Response Matrix rows for inherited MASTER directives and for the T4/T5
  task boundary decision.
- Phase-2 gate arithmetic restated with exact new-test counts.

[**Changed**]

- Scope: #5 (direct signaling) deferred to directIrq feature.
- Module layout: `plic/{mod,core,gateway,source}.rs` — no `line.rs`, no
  `PlicIrqLine`.
- `LineSignal` removed; atomics removed. Gateway sampling still
  consumes the `u32` bitmap passed by the existing `Bus::tick` →
  `Plic::notify(bitmap)` path.
- Phase 2 of 00_PLAN (bus surgery, device migration) is gone; what was
  Phase 3 (edge-trigger config) becomes Phase 2.
- T-4 removed entirely (no `Device::tick` change). T-5 reframed around
  evaluation site only (no cross-thread axis since I-9 is tick-only).
- Validation reorganized: V-IT around xv6/linux booting unchanged;
  new V-UT for the SiFive-variant pre-claim clear (V-UT-11 per R-002).

[**Removed**]

- G-2 (direct signaling) — deferred to directIrq.
- `PlicIrqLine`, `LineSignal`, `line.rs`, and all API surface around
  them.
- Phase 2 bus-surgery steps and `Bus::tick` / `Device::tick`
  restructuring.
- C-3 (seam allowlist change), C-5 (bus simplification) — no seam or
  bus change in this feature.
- Open Qs 1, 3 (resolved by narrowing).

[**Unresolved**]

- Open Q 2 (per-hart vs global SourceConfig): confirmed global per spec,
  now stated as a design fact rather than a question.
- Follow-up `directIrq` feature will revisit atomic primitives and
  cross-thread posture; this plan intentionally does not pre-commit
  that future design.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 (CRITICAL) | Accepted | Narrow scope to #6+#7; defer `PlicIrqLine`, Bus::tick surgery, device migration to a future `directIrq` feature. Summary, Goals, Phases, API Surface, Validation all revised. |
| Review | R-002 (HIGH) | Accepted | New Invariant I-8 documents the SiFive-variant pre-claim level-low clear as a deliberate deviation from PLIC v1.0.0 §5. Added V-UT-11 to assert the deviation explicitly. Spec References updated. |
| Review | R-003 (HIGH) | Accepted (option b) | New Invariant I-9 commits to tick-thread-only signaling. Under the narrowed scope there is no cross-thread `raise()` caller, so atomics are redundant. `LineSignal` is removed; gateway sampling continues to consume the bus-supplied `u32` bitmap. Cross-thread posture is the directIrq feature's problem. |
| Review | R-004 (HIGH) | Accepted (option: keep `notify`) | New Invariant I-10 keeps gateway evaluation inside the existing `Plic::notify(bitmap)` path. Because `Bus::tick` still collects the bitmap and calls `notify` after every device's `tick()` (bus.rs:229-242), devices' `tick`-computed line state is already visible to PLIC within the same tick. No `Device::tick` override, no bus surgery, no ordering concern. |
| Review | R-005 (MEDIUM) | Accepted | Inherited-directive rows added below for archModule 00-M-002 and archLayout 01-M-004, plus the T4/T5 boundary row. |
| Review | R-006 (MEDIUM) | Accepted (becomes moot) | Under I-9 (tick-thread-only) there is no "raise caller" axis. T-5 is reframed in the Trade-offs section around a single axis (evaluation site) and (B1) is selected. |
| Review | R-007 (MEDIUM) | Accepted | New Invariant I-11: `Plic::reset` preserves `SourceConfig`. Reset clears runtime gateway state (`armed`, `in_flight`, `prev_level`) but not `kind`. V-F-5 asserts it. |
| Review | R-008 (MEDIUM) | Accepted | `Plic::notify` is kept (it's the evaluation entry point per I-10). `Device::notify` trait method is untouched. The 00_PLAN contradiction is removed. |
| Review | R-009 (MEDIUM) | Accepted | Under I-9 (tick-thread-only) there is no atomic edge-latch; `pulse()` doesn't exist in this feature. Edge is driven via an in-struct `prev_level` that `notify` updates inside the same call. V-UT-12 asserts single-threaded coalesce. |
| Review | R-010 (LOW) | Accepted (becomes moot) | No seam-allowlist change under the narrowed scope. V-IT-1 asserts the arch_isolation allowlist is **unchanged** (exact list pin). |
| Review | R-011 (LOW) | Accepted | Phase 2 gate restated with exact counts (see Phase 2 gate below). |
| Review | TR-1 | Applied | Option B accepted; file-size soft cap ≤ 250 lines per plic/ file at Phase-1 completion. |
| Review | TR-2 | Superseded by I-9 | Atomics dropped per R-003 option (b). TR-2 Option A rationale no longer applies under tick-only posture. |
| Review | TR-3 | N/A | Deferred with directIrq — no `Device::irq_line` change in this feature. |
| Review | TR-4 | N/A | Deferred with directIrq — no `Device::tick` repurposing. |
| Review | TR-5 | Applied (B1 only) | Evaluate at tick boundary via existing `notify` path. No cross-thread axis under I-9. |
| Project Memory | T4/T5 boundary | Accepted (option a) | Narrowed to plicGateway proper (MANUAL #6+#7). `directIrq` (#5) becomes its own feature, reusing the gateway/core split as its substrate. |
| Master (inherited) | archModule 00-M-002 | Honored | New `plic/` subdirectory sits under `arch/riscv/device/intc/plic/`, topic-organized under `arch/<name>/` per archModule. |
| Master (inherited) | archLayout 01-M-004 | Honored | No new symbols cross from `arch/riscv/` into top-level `src/device/`; `SEAM_ALLOWED_SYMBOLS` is unchanged. |

> Rules:
> - Every prior HIGH / CRITICAL finding appears above.
> - No local MASTER exists; inherited MASTERs acknowledged.

---

## Spec

### Problem Statement (unchanged)

MANUAL_REVIEW items addressed here:

> 6. Asynchronous interrupt handle, both of external device and interrupt
>    hanler. External device enable irq will notify PLIC, PLIC will
>    handle async. And PLIC receive irq will set meip and notify
>    Interrupt handler, interrupt handle async.
>
> 7. Consider Better design to PLIC, which should both support
>    level-triggled and edge-triggled interrupt. Consider add the
>    semantic of Gateways and PLIC-Core which response for different
>    parts of PLIC. `device/source -> per-source gateway -> PLIC core ->
>    hart context` Consider a good design of gateways to handle the
>    source correctly.

Under R-001's narrowing, #6 reduces to its architectural half: the
internal Gateway/Core split makes the claim-gating and (future)
edge/level distinction first-class, so when `directIrq` later wires
external devices directly, the plumbing lands on a correctly-factored
Gateway. The "async" signaling path proper belongs to directIrq.

### Scope Revision

What moved **out** of plicGateway compared to 00_PLAN:

| 00_PLAN Item | New Home |
|---|---|
| G-2 `PlicIrqLine` handle, `raise/lower/pulse` | `directIrq` |
| `line.rs` module, `LineSignal`, atomic ordering | `directIrq` |
| `Bus::tick` bitmap-pump removal | `directIrq` |
| UART / virtio-blk `with_irq(PlicIrqLine)` | `directIrq` |
| `SEAM_ALLOWED_SYMBOLS += "PlicIrqLine"` | `directIrq` |
| `Device::tick` repurposing (never needed; pre-existing) | N/A — dropped entirely |
| `Plic::on_signal` entry point | N/A — `Plic::notify(bitmap)` is retained per I-10 |

What stays **in** plicGateway:

- `plic/{mod,core,gateway,source}.rs` module split.
- `Gateway` FSM for level sources (SiFive variant, I-8) and edge
  sources (coalesce-on-armed).
- `Core` arbitration/claim/complete/MEIP-SEIP drive.
- `Plic::with_config(...)` surface for per-source `SourceConfig` at
  construction.
- Preserving `Plic::notify(bitmap)` as the single evaluation entry
  point driven by the existing `Bus::tick` bitmap collection.

### Spec References

- RISC-V PLIC Specification v1.0.0 (2023), §5 (Interrupt Gateways),
  §7 (Interrupt Flow and Core), §9 (Hart Contexts). Note: this
  feature deliberately diverges from §5's sticky-IP rule for
  `SourceKind::Level` to preserve current SiFive-variant behavior
  (see I-8).
- SiFive FU540/FU740 PLIC memory map (offsets in `plic.rs:11-19`,
  unchanged).

### Goals

- **G-1** Separate the PLIC into two crate-internal components with
  clear responsibilities: `Gateway` (per-source level/edge conversion
  and claim-gating) and `Core` (priority arbitration + per-context
  enable/threshold/claim/complete), exposed externally as one `Plic`
  `Device` with the same MMIO surface and the same `Device::notify`
  override signature.
- **G-2** Support both level-triggered and edge-triggered sources on a
  per-source basis, configurable at PLIC construction via
  `Plic::with_config(num_harts, irqs, sources)`. Default remains all
  level. No in-tree device adopts edge in this feature.
- **G-3** Preserve the existing MMIO register layout, claim/complete
  semantics, and per-hart MEIP/SEIP routing exactly. No guest-visible
  behavior change for existing workloads (xv6, linux, linux-2hart,
  am-tests, cpu-tests).
- **G-4** Keep the 375-test green baseline at every phase boundary;
  add new unit tests for the gateway level/edge FSMs and the
  reset-preserves-config invariant.

Non-Goals:

- **NG-1** No direct device→PLIC signaling. External devices keep their
  existing `Device::irq_line(&self) -> bool` surface; `Bus::tick`
  keeps its bitmap collection at `src/device/bus.rs:227-242`. Deferred
  to `directIrq`.
- **NG-2** No cross-thread signaling path in this feature. All PLIC
  state is touched from the bus tick thread only (I-9). Re-examined
  in `directIrq`.
- **NG-3** xemu remains single-threaded with the round-robin hart
  driver in `Bus::tick`. UART's reader thread is unaffected and does
  not touch PLIC state.
- **NG-4** No change to MSWI/MTIMER/SSWI, nor to the M-mode/S-mode trap
  delivery path beyond what PLIC already does via `IrqState`.
- **NG-5** No change to DTB, number of sources (`NUM_SRC = 32`), or
  `Device::tick` / `Device::notify` / `Device::irq_line` trait
  surface.
- **NG-6** No seam allowlist changes. `SEAM_ALLOWED_SYMBOLS` at
  `xemu/xcore/tests/arch_isolation.rs` is byte-identical before and
  after this feature (asserted by V-IT-1).

### Architecture

Target module layout under `xemu/xcore/src/arch/riscv/device/intc/plic/`:

```
plic/
├── mod.rs      // Plic struct, Device impl, public API (new, with_config, notify)
├── source.rs   // SourceKind { Level, Edge }, SourceConfig
├── gateway.rs  // Gateway: per-source FSM + claim gate
└── core.rs     // Core: priority[], pending, enable[], threshold[],
                // claimed[], arbitration, MEIP/SEIP drive
```

The public single entry remains `Plic` in `mod.rs`. `Gateway`,
`GatewayDecision`, `Core` are `pub(super)` crate-internal. `SourceKind`
and `SourceConfig` are `pub` within `plic/` and re-exported
`pub(crate)` at `intc::plic::{SourceKind, SourceConfig}` so board
construction can call `Plic::with_config`. They are not re-exported
across the `arch/riscv/` boundary (I-7).

Runtime data flow (unchanged bus side):

```
devices                     (UART, VirtioBlk, ...)
   │  Device::irq_line() -> bool          (unchanged trait method)
   ▼
Bus::tick        bitmap collect loop      (bus.rs:227-242, unchanged)
   │  plic.notify(bitmap)
   ▼
Plic::notify(bitmap)     (retained entry point; internally:)
   │  for s in 1..NUM_SRC: gateway[s].sample(bitmap_bit) -> decision
   │  apply decisions to core.pending
   │  core.evaluate() -> IrqState set/clear per hart
   ▼
Core
   │  priority arbitration, per-context MEIP/SEIP drive
   ▼
IrqState (shared with hart, unchanged)
```

For `SourceKind::Edge` sources, the bitmap bit is interpreted as the
current *level* and the Gateway maintains `prev_level` internally to
detect rising edges within `notify`. Under NG-2/I-9 single-thread
tick-only posture, an edge that occurs and clears *between* two bus
ticks is invisible — same contract current PLIC has for level sources.
A device that wants lossless edges will use `directIrq`'s per-source
edge latch once that feature lands; this feature does not try to
model lossless edges over the poll-based bus bitmap.

MMIO layout (unchanged):

| Offset range         | Meaning                                   |
|----------------------|-------------------------------------------|
| `0x000000..0x000080` | priority[0..32], u32 per source           |
| `0x001000`           | pending bitmap (read-only summary)        |
| `0x002000 + c*0x80`  | enable[c], 32-bit bitmap per context      |
| `0x200000 + c*0x1000`| threshold[c], u32                         |
| `0x200004 + c*0x1000`| claim/complete[c], u32                    |

### Invariants

- **I-1** (Arbitration equivalence for level sources) For every
  guest-observable sequence of MMIO accesses and bitmap notifications,
  the stream of values returned from claim reads and the stream of
  MEIP/SEIP edges driven onto `IrqState` is bit-identical to the
  current monolithic `Plic`, provided all sources are configured as
  `SourceKind::Level` (the default).
- **I-2** (Claim gating) Once a source `s` is claimed by any context,
  the gateway for `s` does not re-pend into core until a matching
  complete arrives. Re-assertion of the level line while claimed is
  recorded by the gateway (`armed` reflects latest level at complete
  time) but held back from core during in-flight.
- **I-3** (Edge semantics) For `SourceKind::Edge`, exactly one pending
  event is generated per rising edge observed by `notify`. Rising
  edges seen while `in_flight=true` are latched in the gateway and
  released into core on `on_complete`. Multiple rising edges during
  in-flight coalesce to one.
- **I-4** (Source 0 reserved) Source 0 never pends, never claims, never
  completes. `(1..NUM_SRC)` iteration preserved.
- **I-5** (Context routing) Context `c` targets hart `c>>1`; even `c`
  drives MEIP, odd `c` drives SEIP. Existing behavior from
  `plic.rs:107-130` preserved verbatim.
- **I-6** (No CSR vocabulary leakage) No `Mip` / `MEIP` / `SEIP`
  constants or types escape `arch/riscv/`. `Gateway`, `Core`, and
  `SourceConfig` do not reference any CSR type.
- **I-7** (Arch isolation) `SEAM_ALLOWED_SYMBOLS` at
  `xemu/xcore/tests/arch_isolation.rs` is byte-identical before and
  after this feature.
- **I-8** (SiFive-variant level pre-claim clear — deliberate
  deviation) For `SourceKind::Level`, if the device bitmap bit drops
  while `in_flight=false`, the gateway clears `armed` and the core's
  pending bit is cleared. This diverges from RISC-V PLIC v1.0.0 §5
  (which treats core IP as sticky post-forward), preserves the
  behavior of the current monolithic `Plic::update` at
  `plic.rs:56-68`, and therefore preserves every existing guest
  workload (xv6, linux, linux-2hart, am-tests, cpu-tests). A
  spec-pure variant is a future feature.
- **I-9** (Tick-thread-only signaling) All PLIC state — gateways, core,
  arbitration, IrqState writes — is touched from the bus tick thread
  only. UART's reader thread does not touch any PLIC state or any
  shared interrupt signaling structure in this feature. No atomic
  primitives are used in new `plic/` code. Re-examined in `directIrq`.
- **I-10** (Evaluation site) Gateway sampling + core arbitration runs
  inside `Plic::notify(bitmap: u32)`, called by `Bus::tick` after
  every device's `tick()` and `irq_line()` poll
  (`src/device/bus.rs:227-242`). No new `Device::tick` override, no
  `on_signal` entry point. Bus-tick ordering is unchanged: devices
  update their `irq_line` state during their own `tick()`, Bus
  collects the bitmap, PLIC processes it — all within the same
  `Bus::tick` call.
- **I-11** (Reset preserves configuration) `Plic::reset` clears
  runtime state (`armed`, `in_flight`, `prev_level`, `pending`,
  `enable`, `threshold`, `claimed`) but preserves `SourceConfig` /
  `Gateway::kind`. A guest-triggered reset does not silently flip
  edge sources back to level.

### Data Structure

```rust
// arch/riscv/device/intc/plic/source.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SourceKind {
    Level,
    Edge,
}

#[derive(Clone, Copy, Debug)]
pub struct SourceConfig {
    pub kind: SourceKind,
}

impl Default for SourceConfig {
    fn default() -> Self { Self { kind: SourceKind::Level } }
}

// arch/riscv/device/intc/plic/gateway.rs
pub(super) struct Gateway {
    kind: SourceKind,
    prev_level: bool,   // for edge detection
    armed: bool,        // gateway-pending (one-slot latch)
    in_flight: bool,    // claimed somewhere, awaiting complete
}

pub(super) enum GatewayDecision {
    Pend,     // set core.pending for this source
    NoChange, // do nothing
    Clear,    // clear core.pending for this source
}

impl Gateway {
    pub(super) fn new(cfg: SourceConfig) -> Self;
    /// Called once per `Plic::notify`. `level` is the current bitmap
    /// bit for this source.
    pub(super) fn sample(&mut self, level: bool) -> GatewayDecision;
    pub(super) fn on_claim(&mut self);
    pub(super) fn on_complete(&mut self) -> GatewayDecision;
    /// Reset runtime state; preserves `kind` (I-11).
    pub(super) fn reset_runtime(&mut self);
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

impl Core {
    pub(super) fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self;
    pub(super) fn set_pending(&mut self, src: u32);
    pub(super) fn clear_pending(&mut self, src: u32);
    pub(super) fn claim(&mut self, ctx: usize) -> u32;
    pub(super) fn complete(&mut self, ctx: usize, src: u32) -> bool; // true if matched
    pub(super) fn evaluate(&mut self);
    pub(super) fn reset_runtime(&mut self);
    /// MMIO register accessors used by `Plic::read` / `Plic::write`.
    pub(super) fn read_reg(&self, offset: usize, size: usize) -> XResult<Word>;
    pub(super) fn write_reg(&mut self, offset: usize, size: usize, val: Word) -> XResult;
}

// arch/riscv/device/intc/plic/mod.rs
pub struct Plic {
    gateways: [Gateway; NUM_SRC],
    core: Core,
}
```

No `LineSignal`, no atomics, no `PlicIrqLine`. The `u32` bitmap passed
to `Plic::notify` is the sole input channel.

### API Surface

```rust
// arch/riscv/device/intc/plic/mod.rs
impl Plic {
    /// Level-default construction (back-compat for existing callers).
    pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self;

    /// Per-source configuration construction.
    pub fn with_config(
        num_harts: usize,
        irqs: Vec<IrqState>,
        sources: [SourceConfig; NUM_SRC],
    ) -> Self;
}

impl Device for Plic {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, val: Word) -> XResult;
    fn reset(&mut self);
    fn notify(&mut self, bitmap: u32);  // unchanged override signature
}
```

- No change to `Device` trait. No new trait method.
- `SourceKind` and `SourceConfig` are re-exported `pub(crate)` from
  `intc::plic` so the RISC-V board constructor can call
  `Plic::with_config`. They do **not** cross the `arch/riscv/` seam.

### Constraints

- **C-1** (MMIO compatibility) Every existing PLIC test in
  `arch/riscv/device/intc/plic.rs` (15 tests, lines 183-358)
  continues to pass unchanged in Phase 1. Moved under
  `plic/mod.rs#[cfg(test)]` without semantic modification.
- **C-2** (Baseline) 375-test green baseline must hold at every phase
  boundary. New tests are additive.
- **C-3** (No seam change) `SEAM_ALLOWED_SYMBOLS` in
  `xemu/xcore/tests/arch_isolation.rs` is byte-identical before and
  after this feature. `Gateway`, `Core`, `GatewayDecision`,
  `SourceKind`, `SourceConfig` are all crate-internal. V-IT-1 pins
  this exactly.
- **C-4** (No CSR leakage) No `Mip` / `MEIP` / `SEIP` / `Sip` in
  `plic/` files. Assertion in V-IT-2.
- **C-5** (No bus changes) `xemu/xcore/src/device/bus.rs` is untouched
  by this feature. `Bus::tick` bitmap collection (lines 227-242)
  remains. `git diff main -- xemu/xcore/src/device/bus.rs` is empty
  at feature completion.
- **C-6** (No device changes) `xemu/xcore/src/device/uart.rs`,
  `virtio_blk.rs`, `pl011.rs`, etc. are untouched by this feature.
  Gated by `git diff main -- xemu/xcore/src/device/` empty.
- **C-7** (No assembly changes) No `.s` / `.S` modifications.
- **C-8** (No DTB change) `compatible = "riscv,plic0"` and register
  ranges preserved.
- **C-9** (No unsafe) No new `unsafe` blocks in `plic/`.
- **C-10** (Rustfmt + clippy clean) `make fmt`, `make clippy` pass at
  each phase.
- **C-11** (File size soft cap) Each file in `plic/` ≤ 250 lines at
  Phase-1 completion (TR-1). `plic/mod.rs`, `plic/core.rs`,
  `plic/gateway.rs`, `plic/source.rs`.

---

## Implement

### Execution Flow

#### Main Flow

1. **Board construction.** RISC-V `Cpu` builder calls
   `Plic::new(num_harts, irqs)` (all level, back-compat) or
   `Plic::with_config(num_harts, irqs, sources)` for per-source
   configuration. No board today needs edge, so `Plic::new` remains
   the sole caller; `with_config` is exercised only by new unit
   tests in Phase 2.
2. **Device raises interrupt condition.** Unchanged. UART sets
   `thre_ip` / drains `rx_buf` in its own `tick()`; VirtioBlk sets
   `irq` after queue work. Devices expose state via
   `Device::irq_line(&self) -> bool`.
3. **Bus::tick collects bitmap.** Unchanged from `bus.rs:227-242`:
   slow path runs `Device::tick()` for every MMIO device, then ORs
   `dev.irq_line()` into a `u32` bitmap keyed by `irq_source`, then
   calls `plic.notify(bitmap)`.
4. **Plic::notify runs gateway + core.**
   - For `s in 1..NUM_SRC`:
     - `level = (bitmap >> s) & 1 != 0`
     - `decision = self.gateways[s].sample(level)`
     - apply `decision` to `self.core` (`set_pending` / `clear_pending`).
   - `self.core.evaluate()` — per-context arbitration, drive
     `IrqState` (set/clear MEIP or SEIP per hart).
5. **Hart reads claim.** Guest MMIO read at `0x200004 + c*0x1000`.
   `Plic::read` delegates to `core.claim(ctx)`; `claim` selects the
   highest-priority enabled source above threshold, clears
   `core.pending[s]`, records `claimed[ctx]=s`, invokes
   `self.gateways[s].on_claim()` to flip `in_flight=true`.
6. **Hart writes complete.** Guest MMIO write of source ID to the
   same address. `Plic::write` delegates to `core.complete(ctx, src)`.
   On successful match, invokes `self.gateways[s].on_complete()`; if
   that returns `Pend`, `core.set_pending(s)` and `core.evaluate()`
   re-run within the same `write` call.

#### Failure Flow

1. **MMIO out-of-range read/write.** `Device::read/write` return
   `Ok(0)` / `Ok(())` — unchanged.
2. **Complete with wrong source.** `core.complete` returns false
   without touching gateway state; `in_flight` of the originally
   claimed source stays set (the guest is expected to complete
   correctly later). Matches current `plic.rs:95-101` semantics.
3. **Notify with bitmap bit set for `s=0`.** Ignored. `1..NUM_SRC`
   loop skips source 0 (I-4).
4. **Notify with bitmap bit set for `s >= NUM_SRC`.** Masked off;
   bitmap is `u32`, `NUM_SRC = 32`, all bits map to a valid source
   index (or source 0, handled above).
5. **Reset mid-claim.** `Plic::reset` calls
   `gateway[s].reset_runtime()` for every `s` (clears `armed`,
   `in_flight`, `prev_level`; preserves `kind` per I-11) and
   `core.reset_runtime()` (clears `pending`, `enable`, `threshold`,
   `claimed`, priorities, and IRQ lines). Post-reset, a subsequent
   `notify` with the same bitmap re-pends cleanly.
6. **Guest reconfigures edge source at runtime.** Not supported in
   this feature. `SourceConfig` is construction-time only. A future
   feature may add MMIO configurability.

#### State Transition

Per-source gateway FSM for `SourceKind::Level`:

| From | Event | To | Emit |
|---|---|---|---|
| `(armed=false, in_flight=false)` | `sample(true)` | `(armed=true)` | `Pend` |
| `(armed=true, in_flight=false)` | `sample(false)` | `(armed=false)` | `Clear` (I-8 SiFive variant) |
| `(armed=true, in_flight=false)` | `sample(true)` | unchanged | `NoChange` |
| `(any, in_flight=false)` | `on_claim` | `(in_flight=true)` | — |
| `(armed=a, in_flight=true)` | `sample(level)` | `(armed=level\|a, in_flight=true)` | `NoChange` (held during in-flight) |
| `(armed=true, in_flight=true)` | `on_complete` | `(in_flight=false, armed=armed')` where `armed'` = latest level | `Pend` if `armed'` else `NoChange` |
| `(armed=false, in_flight=true)` | `on_complete` | `(in_flight=false)` | `NoChange` |

Per-source gateway FSM for `SourceKind::Edge`:

| From | Event | To | Emit |
|---|---|---|---|
| `(armed=false, in_flight=false, prev_level=false)` | `sample(true)` | `(armed=true, prev_level=true)` | `Pend` |
| `(armed=*, in_flight=*, prev_level=*)` | `sample(false)` | `(prev_level=false)` | `NoChange` |
| `(armed=true, in_flight=false, prev_level=true)` | `sample(true)` | unchanged | `NoChange` (coalesce) |
| `(armed=false, in_flight=true, prev_level=false)` | `sample(true)` | `(armed=true, prev_level=true, in_flight=true)` | `NoChange` (latch) |
| `(any, in_flight=false)` | `on_claim` | `(in_flight=true, armed=false)` | — |
| `(armed=true, in_flight=true)` | `on_complete` | `(in_flight=false, armed=true)` | `Pend` |
| `(armed=false, in_flight=true)` | `on_complete` | `(in_flight=false)` | `NoChange` |

### Implementation Plan

This plan has **two** phases.

#### Phase 1 — Internal split, behavior preserved

Scope: refactor `plic.rs` into `plic/{mod,core,gateway,source}.rs`
with identical external behavior under `SourceKind::Level` defaults.

Steps:

1. Create `xemu/xcore/src/arch/riscv/device/intc/plic/` directory.
   Move the existing `Plic` struct body into `plic/mod.rs`; re-export
   via `intc::plic::Plic` unchanged. Existing 15 plic tests move
   under `plic/mod.rs#[cfg(test)]` verbatim (no semantic edits).
   Verify `make test` → 375/375.
2. Create `plic/source.rs` with `SourceKind`, `SourceConfig`. Default
   all-level. No test additions; compile-only.
3. Create `plic/core.rs`: move `priority`, `pending`, `enable`,
   `threshold`, `claimed`, `num_ctx`, `irqs`, plus `claim`,
   `complete`, `evaluate`, and MMIO register read/write helpers into
   `Core`. `Plic` delegates. Re-run the 15 existing tests
   (arbitration / claim / complete / routing) — all pass by
   delegation. Verify 375/375.
4. Create `plic/gateway.rs`: implement `Gateway` + `GatewayDecision`
   for `SourceKind::Level` only (I-8 semantics — SiFive variant).
   Plumb through `Plic::notify`:
   ```
   for s in 1..NUM_SRC:
       level = (bitmap >> s) & 1 != 0
       match self.gateways[s].sample(level) {
           Pend => self.core.set_pending(s),
           Clear => self.core.clear_pending(s),
           NoChange => {}
       }
   self.core.evaluate()
   ```
   Wire `Core::claim` / `Core::complete` to call back into the
   gateway (`on_claim` / `on_complete`). The existing test
   `claimed_source_not_repended` now validates gateway claim-gating
   rather than an ad-hoc `is_claimed` filter.
5. Delete now-dead helpers in `Plic` that are subsumed by `Gateway`
   (e.g., any `is_claimed` filter pattern). Clippy clean. Verify
   baseline + new gateway-level UT.
6. Add `Plic::with_config` that accepts `[SourceConfig; NUM_SRC]`.
   In Phase 1 it constructs level-only gateways — same result as
   `Plic::new` — but the surface is available for Phase 2.

Phase 1 Gate (exact):

- `make fmt` clean.
- `make clippy` clean, `-D warnings`.
- `make test`: baseline 375 + new Phase-1 tests ≥ **14** added:
  V-UT-2, V-UT-3, V-UT-4, V-UT-5, V-UT-11, V-UT-G1, V-F-1, V-F-2
  (extension), V-F-3 (existing keeps), V-F-5, V-E-1, V-E-2, V-E-5,
  V-IT-1, V-IT-2 grep gates (≥ 14 net new additive tests; baseline
  tests unchanged).
- `make run` (`DEBUG=n`): xv6, linux single-hart, linux-2hart smoke
  boot.
- File size: each `plic/*.rs` ≤ 250 lines (C-11).

#### Phase 2 — Edge-trigger configuration

Scope: completes MANUAL_REVIEW item #7. Add `SourceKind::Edge` path in
the existing Gateway FSM; no in-tree source adopts edge.

Steps:

1. Extend `Gateway::sample` to handle `SourceKind::Edge` per the
   Edge FSM table above. `prev_level` is maintained in the gateway
   struct; rising edge detected when `prev_level == false && level
   == true`.
2. Extend `Gateway::on_complete` for edge: if `armed` (edge latched
   during in-flight), return `Pend`; else `NoChange`.
3. No `pulse()` API. Under I-9 (tick-thread-only) and NG-1 (no
   direct signaling), edges are derived from bitmap level
   transitions seen by consecutive `notify` calls. A lossless edge
   API is `directIrq` territory.
4. Add V-UT-6, V-UT-7, V-UT-12, V-UT-G2 (edge FSM table-driven
   unit), V-E-6 (mixed level+edge construction).

Phase 2 Gate (exact):

- `make fmt` clean.
- `make clippy` clean, `-D warnings`.
- `make test`: Phase-1 gate set + **new Phase-2 tests ≥ 5**:
  V-UT-6, V-UT-7, V-UT-12, V-UT-G2, V-E-6. Total new tests at end
  of Phase 2 ≥ 19.
- `make run` (`DEBUG=n`) unchanged behavior — no in-tree device
  uses `with_config`.
- File size: `plic/gateway.rs` ≤ 250 lines.

---

## Trade-offs

- **T-1 (retained): Module split granularity.**
  Option A — single `plic.rs` grown to ~500 lines. Option B — split
  into `plic/{mod,core,gateway,source}.rs` (~80–200 lines each).
  **Chosen: Option B** per TR-1. Each file has a single concern.
  File-size soft cap of 250 lines (C-11). Option A rejected because
  the feature's explicit premise is separating responsibilities the
  current code entangles; keeping a monolithic file contradicts the
  premise and violates the user's `many-small-files > few-large-files`
  coding-style rule.
- **T-2 (retained, rescoped): Concurrency primitives.**
  Option A — atomics (00_PLAN's choice). Option B — plain fields,
  tick-thread-only. **Chosen: Option B** per R-003 option (b) and
  I-9. Rationale: this feature does not introduce any cross-thread
  caller. UART's reader thread continues to write to `rx_buf` and
  never touches PLIC; the existing bus bitmap-pump path remains.
  Atomics would be speculative for `directIrq`, and `directIrq` will
  reintroduce them with the correct ordering specification tied to
  its own cross-thread signaling path. Adding them now would both be
  unjustified (no current caller) and pre-commit a design that
  belongs to a different feature.
- **T-3 (new): Edge-latch representation under tick-only posture.**
  Option A — in-gateway `prev_level` plus derived rising-edge on
  `sample`. Option B — a separate `edge_latch` field on the gateway
  set by `sample` and cleared by `notify`. **Chosen: Option A.**
  With NG-1 (no direct signaling), the only source of edge
  information is the bitmap level across two consecutive `notify`
  calls. `prev_level` is the simplest exact representation. Option B
  would re-introduce the lossless-edge contract that NG-1 explicitly
  disclaims. A future `directIrq` feature will add a true edge-latch
  when cross-thread `pulse()` exists.
- **T-4 (new): SourceConfig lifecycle on reset.**
  Option A — reset preserves `kind` (I-11). Option B — reset zeroes
  the entire gateway struct back to default (all level). **Chosen:
  Option A** per R-007. Rationale: a guest-triggered reset (OpenSBI
  system reset, VirtIO-blk soft reset) is not reconfiguration;
  silently demoting edge sources to level after a reset would
  produce bizarre behavior only after a reset.
- **T-5 (reframed): Evaluation site.**
  Option A — evaluate inside `notify` on every bus tick's bitmap
  collection (today's path). Option B — evaluate on every `raise`.
  **Chosen: Option A (B1 per R-006).** Under I-9 (tick-thread-only)
  there is no `raise` caller thread, so the R-006 "raise caller
  axis" collapses — only the evaluation-site axis remains, and
  tick-boundary evaluation is determinism-preserving and matches
  the current bitmap-pump path we are deliberately keeping per I-10.

---

## Validation

### Unit Tests

- **V-UT-1** All 15 existing PLIC tests (`plic.rs:183-358`) pass
  verbatim after the Phase 1 split. Verified by `cargo test -p xcore
  arch::riscv::device::intc::plic`.
- **V-UT-2** `gateway_level_raise_pends` — sample `true` from a
  fresh gateway pends core; sample `false` after pended clears core
  (I-8).
- **V-UT-3** `gateway_level_claim_gates_repend` — sample `true` →
  claim → sample `true` again (level stays high): core pending
  stays cleared until complete.
- **V-UT-4** `gateway_level_complete_with_line_low` — complete while
  level is low: no re-pend.
- **V-UT-5** `gateway_level_complete_with_line_high` — complete
  while level is high: re-pend.
- **V-UT-6** `gateway_edge_pends_once_on_rising` — Phase 2: `prev=0,
  cur=1` pends; `prev=1, cur=1` does not re-pend.
- **V-UT-7** `gateway_edge_during_claim_latches_and_pends_on_complete`
  — Phase 2: rising edge while `in_flight=true` sets `armed`;
  `on_complete` returns `Pend`.
- **V-UT-11** `gateway_level_pre_claim_dropline_clears` — explicit
  deviation from PLIC v1.0.0 §5 per I-8. Sample `true` pends; sample
  `false` before claim clears core. Spec-pure variant would keep it
  pending; this test pins SiFive behavior.
- **V-UT-12** `gateway_edge_coalesce_during_in_flight` — multiple
  rising edges (`prev=0→1→0→1`) while `in_flight=true` result in
  exactly one `Pend` on `on_complete`.
- **V-UT-G1** `gateway_struct_level_fsm` — direct table-driven unit
  test on `Gateway` struct (constructor + every state transition in
  the Level FSM table above).
- **V-UT-G2** `gateway_struct_edge_fsm` — same, for the Edge FSM
  table. Phase 2.

### Integration Tests

- **V-IT-1** `cargo test -p xcore --test arch_isolation` —
  `SEAM_ALLOWED_SYMBOLS` unchanged. Diff-style assertion: the
  post-plicGateway allowlist is byte-identical to the
  pre-plicGateway allowlist (R-010). Specifically, `SourceKind`,
  `SourceConfig`, `Gateway`, `GatewayDecision`, `Core` are absent
  from the allowlist.
- **V-IT-2** Grep / diff gates:
  - `rg '\b(Mip|MEIP|SEIP|Sip)\b' xemu/xcore/src/arch/riscv/device/intc/plic`
    → no matches (C-4).
  - `git diff main -- xemu/xcore/src/device/` empty (C-5, C-6).
  - `rg 'unsafe' xemu/xcore/src/arch/riscv/device/intc/plic` empty
    (C-9).
- **V-IT-3** am-tests suite — unchanged pass set.
- **V-IT-4** cpu-tests suite — unchanged pass set.
- **V-IT-5** xv6 boot: `DEBUG=n` run boots to shell and responds to
  one console input (UART source exercises level-FSM path
  end-to-end through the unchanged bus bitmap pump).
- **V-IT-6** Linux single-hart boot completes to userspace with
  virtio-blk (block source level path).
- **V-IT-7** Linux 2-hart boot: both harts reach userspace; per-hart
  MEIP/SEIP routing unchanged (I-5).

### Failure / Robustness Validation

- **V-F-1** Reset mid-claim clears gateway runtime state
  (`armed=false`, `in_flight=false`, `prev_level=false` for every
  source) and core runtime state; subsequent bitmap notification
  re-pends correctly.
- **V-F-2** Complete with wrong source ID leaves gateway and core
  state intact; the `claimed[ctx]` slot for the originally claimed
  source stays set. Extension of existing
  `complete_wrong_source_no_change` to also check
  `gateway[s].in_flight == true`.
- **V-F-3** MMIO out-of-range read returns 0; out-of-range write
  silently dropped (unchanged).
- **V-F-4** Notify with stray bits (bit 0 set, or multiple bits at
  once): bit 0 ignored (I-4); multi-bit bitmap processed in source
  order; final core state matches independent per-source sampling.
- **V-F-5** **New per R-007.** Construct `Plic::with_config` with
  one source configured as `Edge`, call `reset`, assert the edge
  source's `kind` is still `Edge` afterwards (I-11).

### Edge Case Validation

- **V-E-1** Raise level twice in a row across two notifies without
  clearing: core pending asserted once, not toggled.
- **V-E-2** Level low → low across ticks: no pending, no spurious
  `Clear` emitted.
- **V-E-3** Per-hart routing test
  (`plic_new_num_harts_two_ctx2_routes_to_irq1` existing): passes
  unchanged post-split.
- **V-E-4** Two contexts enable the same source; one claims; the
  other sees pending cleared on first claim; complete re-pends only
  if level still high (existing contract).
- **V-E-5** `NUM_SRC` boundary: source 31 pends and completes
  correctly; source 0 never pends (I-4).
- **V-E-6** `Plic::with_config` with mixed Level+Edge sources: level
  source level-trigger, edge source edge-trigger, no cross-talk.
  Phase 2.

### Acceptance Mapping

| Goal / Constraint | Validation                                         |
|-------------------|----------------------------------------------------|
| G-1 (split)       | V-UT-1, V-UT-2, V-UT-3, V-UT-G1, V-IT-3..7         |
| G-2 (edge config) | V-UT-6, V-UT-7, V-UT-12, V-UT-G2, V-E-6, V-F-5     |
| G-3 (no guest change) | V-IT-3, V-IT-4, V-IT-5, V-IT-6, V-IT-7         |
| G-4 (baseline)    | V-UT-1 + all Phase-1/2 UT green; 375 held          |
| I-1               | V-UT-1, V-E-3, V-E-4                               |
| I-2               | V-UT-3, V-UT-4, V-UT-5                             |
| I-3               | V-UT-6, V-UT-7, V-UT-12                            |
| I-4               | V-E-5, V-F-4                                       |
| I-5               | V-E-3                                              |
| I-6               | V-IT-2 (grep gate)                                 |
| I-7               | V-IT-1                                             |
| I-8 (SiFive dev.) | V-UT-11 (explicit)                                 |
| I-9 (tick-only)   | `rg 'Atomic' xemu/xcore/src/arch/riscv/device/intc/plic` empty; V-IT-2 (no device changes imply no cross-thread caller exists) |
| I-10 (evaluation) | V-IT-2 (no bus changes imply `notify` remains the entry point); V-IT-5 (xv6 end-to-end) |
| I-11 (reset preserves cfg) | V-F-5                                     |
| C-1               | V-UT-1                                             |
| C-2               | every gate: 375/375 baseline                       |
| C-3               | V-IT-1                                             |
| C-4               | V-IT-2                                             |
| C-5               | V-IT-2                                             |
| C-6               | V-IT-2                                             |
| C-7               | `git diff main -- '*.s' '*.S'` empty               |
| C-8               | DTB diff empty                                     |
| C-9               | V-IT-2 (unsafe grep)                               |
| C-10              | `make fmt && make clippy` in CI at every phase     |
| C-11              | `wc -l` on each `plic/*.rs` ≤ 250                  |

---

## Gates

At each phase boundary the following must pass (per AGENTS.md
"Verification" clause):

1. `make fmt` — clean.
2. `make clippy` — clean, `-D warnings`.
3. `make test` — baseline 375 + Phase-specific new tests:
   - **Phase 1 end**: baseline 375 + ≥14 new (V-UT-2/3/4/5/11/G1,
     V-F-1/2/5, V-E-1/2/5, V-IT-1, V-IT-2).
   - **Phase 2 end**: Phase-1 total + ≥5 new (V-UT-6/7/12/G2,
     V-E-6). Total ≥ 19 new tests end-to-end.
4. `make run` (`DEBUG=n`) — xv6, linux, linux-2hart boot to userspace
   and respond to one input.
5. am-tests and cpu-tests suites unchanged.

## Risks and Open Questions

- **Risk 1** A Phase-1 test move (from `plic.rs` to `plic/mod.rs`)
  that accidentally changes a test's module path could break
  `cargo test arch::riscv::device::intc::plic::<name>` invocations
  elsewhere (e.g., in docs or scripts). Mitigation: run
  `rg 'arch::riscv::device::intc::plic'` before Phase 1 to map
  every external test reference; ensure the new module path matches.
- **Risk 2** Phase 2 edge FSM exercises code paths that have no
  in-tree caller. Unit tests are the sole coverage until
  `directIrq` lands. Mitigation: V-UT-G2 is a table-driven FSM test
  covering every state transition in the Edge table.
- **Risk 3** Follow-on `directIrq` feature must re-examine I-9 and
  either reintroduce atomics with proper ordering or adopt a
  different cross-thread posture (e.g., a lock-free ring). Not this
  feature's burden, but it constrains this feature to keep all new
  `plic/` code re-entrance-neutral from a future threaded caller's
  point of view — no hidden `&mut self` assumptions that would need
  unsafe retrofitting. Addressed by keeping the
  `Plic::notify(&mut self, u32)` bus-tick-only contract.
- **Open Q 1** (resolved) Per-hart vs global `SourceConfig`? Global
  per PLIC spec — one `SourceKind` per source across all contexts.
- **Open Q 2** Should `directIrq` land before any in-tree device
  adopts edge, to avoid stranding the edge FSM in unused territory?
  Out of scope for this plan but flagged: the edge FSM here is
  correctness-equivalent to a future `directIrq` edge path, just
  driven by polled level instead of pulse.
