# `aclintSplit` SPEC

> Source: [`/docs/archived/refactor/aclintSplit/01_PLAN.md`](/docs/archived/refactor/aclintSplit/01_PLAN.md).
> Iteration history, trade-off analysis, and implementation
> plan live under `docs/archived/refactor/aclintSplit/`.

---


[**Goals**]

- **G-1** Three independently constructible sub-devices — `Mswi`,
  `Mtimer`, `Sswi` — one per spec-defined controller, each owning only
  its spec-mandated state.
- **G-2** Preserve externally observable MMIO: guest accesses at
  `0x0200_0000 + {0x0000, 0x4000, 0x4004, 0xBFF8, 0xBFFC, 0xC000}`
  return bit-identical values with identical `IrqState` edges.
- **G-3** Preserve the `RVCore::with_config` call-site shape: a single
  `Aclint::new(irq, ssip).install(&mut bus, base)` call registers all
  three sub-devices and returns the MTIMER region index.
- **G-4** Re-home every existing `aclint.rs` test under the owning
  sub-device's `#[cfg(test)]`; add three isolation tests proving each
  sub-device is usable alone.
- **G-5** Narrow NG-5 Bus residuals: rename `aclint_idx` →
  `mtimer_idx` in `device/bus.rs` (naming accuracy only). Other NG-5
  residuals stay.

[**Non-Goals**]

- **NG-1** Per-hart MSIP / MTIMECMP arrays — deferred to `multiHart`.
- **NG-2** Replacing `Device::mtime` with a `TimerSource` trait —
  deferred to `directIrq`.
- **NG-3** Changing the MMIO base (`0x0200_0000`) or footprint
  (`0x1_0000`) — would force firmware/DT churn.
- **NG-4** Removing `Bus::take_ssip` / `Bus::ssip_flag`.
- **NG-5** Re-wiring the slow-tick divisor (`SLOW_TICK_DIVISOR = 64`);
  MTIMER stays on the fast path, MSWI/SSWI move to the slow path (both
  no-op per R-007).
- **NG-6** Renaming tests in other modules.
- **NG-7** Altering `SYNC_INTERVAL = 512` or the 10 MHz wall-clock scale.

[**Architecture**]

Before:

```
arch/riscv/device/intc/
├── aclint.rs   (276 lines; single Aclint; three concerns bundled)
├── plic.rs
└── mod.rs      (pub mod aclint; pub mod plic;)
```

After:

```
arch/riscv/device/intc/
├── aclint/
│   ├── mod.rs    (pub struct Aclint + install; mod mswi/mtimer/sswi;
│   │             mount-integration test; cross-register reset test)
│   ├── mswi.rs   (pub(super) struct Mswi + Device impl + tests)
│   ├── mtimer.rs (pub(super) struct Mtimer + Device impl + tests)
│   └── sswi.rs   (pub(super) struct Sswi + Device impl + tests)
├── plic.rs
└── mod.rs        (pub mod aclint; pub mod plic;)   ← seam file, unchanged
```

Region plan (guest-visible, unchanged):

```
0x0200_0000 ─┬── MSWI   (0x4000 B)  → msip at +0x0000
0x0200_4000 ─┼── MTIMER (0x8000 B)  → mtimecmp +0x0000 (global +0x4000),
             │                        mtime    +0x7FF8 (global +0xBFF8)
0x0200_C000 ─┴── SSWI   (0x4000 B)  → setssip  +0x0000 (global +0xC000)

Total 0x1_0000. Zero gap, zero overlap (checked by Bus::add_mmio).
```

Runtime interaction (shared state):

```
      ┌── IrqState (Arc<AtomicU64>) ──┐         ┌── Arc<AtomicBool> ──┐
      ▼                               ▼         ▼                    ▼
  ┌─────┐                    ┌──────────┐   ┌──────┐             ┌───────┐
  │Mswi │  MSIP edge         │  Mtimer  │   │ Sswi │  SSIP edge  │  Bus  │
  │msip │ ←───────────── MTIP│ mtime,   │   │ssip  │ ──────────► │ ssip_ │
  │     │                    │ mtimecmp │   │      │             │pending│
  └──┬──┘                    └────┬─────┘   └──┬───┘             └───┬───┘
     │                            │            │                     │
     └──────── Bus::add_mmio x3 (via Aclint::install) ────────────────┘
                                  │
                                  ▼
                             Bus::mtime() → mtimer_idx → Mtimer::mtime()
```

[**Invariants**]

- **I-1** Each of `Mswi`/`Mtimer`/`Sswi` compiles and tests pass in
  isolation — no cross-sub-device field access. Shared values
  (`IrqState`, SSIP `Arc<AtomicBool>`) predate the split.
- **I-2** External MMIO semantics are byte-identical: for every
  `(offset, size, value)` triple used by OpenSBI fw_jump, am-tests, or
  Linux boot, post-split `bus.read/write` returns the same value and
  asserts the same `IrqState` bits as pre-split.
- **I-3** Tick cadence: MTIMER ticks every bus step (fast path via
  `mtimer_idx`). MSWI and SSWI have **no tick-driven state** today —
  `Aclint::tick` at `aclint.rs:137-148` only advances `mtime` and
  evaluates `check_timer`; MSIP and SSIP transitions are
  MMIO-write-driven. Moving MSWI/SSWI to the slow path (they inherit
  the empty `Device::tick` default from `device/mod.rs:28`) is
  behaviour-preserving by construction.
- **I-4** `Bus::take_ssip` still drains the shared `Arc<AtomicBool>`:
  the flag moves from `Aclint::ssip` (old) to `Sswi::ssip` (new);
  `Arc` identity is stable (passed from `bus.ssip_flag()` at
  construction, never cloned to a fresh allocation).
- **I-5** `hard_reset` / `reset` resets all three sub-devices to
  power-on state identically to the pre-split `Aclint::reset`.
- **I-6** No new `unsafe`, no new crate dependencies, no new global
  mutable state.

[**Data Structure**]

```rust
// arch/riscv/device/intc/aclint/mswi.rs
pub(super) struct Mswi { msip: u32, irq: IrqState }

// arch/riscv/device/intc/aclint/mtimer.rs
pub(super) struct Mtimer {
    epoch: Instant,
    mtime: u64,
    ticks: u64,
    mtimecmp: u64,
    irq: IrqState,
}

// arch/riscv/device/intc/aclint/sswi.rs
pub(super) struct Sswi { ssip: Arc<AtomicBool> }

// arch/riscv/device/intc/aclint/mod.rs — the thin façade.
pub struct Aclint { mswi: Mswi, mtimer: Mtimer, sswi: Sswi }

// Per-sub-device mmio_regs! (offsets local to each sub-device's region):
// Mswi:    { Msip       = 0x0000 }
// Mtimer:  { MtimecmpLo = 0x0000, MtimecmpHi = 0x0004,
//            MtimeLo    = 0x7FF8, MtimeHi    = 0x7FFC }
// Sswi:    { Setssip    = 0x0000 }
```

`Mswi` / `Mtimer` / `Sswi` are `pub(super)` so they are constructible
within `intc::aclint` for tests and for the façade, but invisible to
`intc/mod.rs` (the seam) and to `cpu/mod.rs`. This keeps
`SEAM_ALLOWED_SYMBOLS` stable.

[**API Surface**]

```rust
// arch/riscv/device/intc/aclint/mod.rs
use std::sync::{Arc, atomic::AtomicBool};
use crate::device::{IrqState, bus::Bus};

impl Aclint {
    /// Build all three sub-devices sharing the given IRQ state and SSIP flag.
    pub fn new(irq: IrqState, ssip: Arc<AtomicBool>) -> Self;

    /// Register MSWI, MTIMER, SSWI on the bus at `base`:
    ///   MSWI   at base+0x0000 (size 0x4000)
    ///   MTIMER at base+0x4000 (size 0x8000)
    ///   SSWI   at base+0xC000 (size 0x4000)
    /// Returns the bus index of the MTIMER region, to be passed to
    /// `Bus::set_timer_source`.
    pub fn install(self, bus: &mut Bus, base: usize) -> usize;
}

// Sub-device impls (crate-internal under arch/riscv/device/intc/aclint/):
impl Mswi   { pub(super) fn new(irq: IrqState) -> Self; }
impl Mtimer { pub(super) fn new(irq: IrqState) -> Self; }
impl Sswi   { pub(super) fn new(ssip: Arc<AtomicBool>) -> Self; }

impl Device for Mswi   { fn read; fn write; fn reset;                 }
impl Device for Mtimer { fn read; fn write; fn tick; fn mtime; fn reset; }
impl Device for Sswi   { fn read; fn write; fn reset;                 }
```

Call-site delta at `arch/riscv/cpu/mod.rs:61-69`:

```rust
// Before:
let aclint_idx = bus.mmio.len();
bus.add_mmio("aclint", 0x0200_0000, 0x1_0000,
             Box::new(Aclint::new(irq.clone(), bus.ssip_flag())), 0);
bus.set_timer_source(aclint_idx);

// After:
let mtimer_idx =
    Aclint::new(irq.clone(), bus.ssip_flag()).install(&mut bus, 0x0200_0000);
bus.set_timer_source(mtimer_idx);
```

The `use super::device::intc::{aclint::Aclint, plic::Plic};` line
stays — seam-symbol `"Aclint"` re-export is preserved.

[**Constraints**]

- **C-1** Guest-visible MMIO base / size / offsets for all six
  documented registers are bit-identical pre/post split.
- **C-2** `IrqState` bit semantics unchanged — `MSIP` / `MTIP`
  set/clear with identical edge timing.
- **C-3** `install` uses only `Bus::add_mmio`; no new `Bus` API added.
  The three regions must not overlap each other or RAM — `add_mmio`
  asserts this.
- **C-4** Tests inherited from `aclint.rs` are re-homed, not rewritten.
  Assertions and local offsets preserved; one new isolation test per
  sub-device (V-UT-4/5/6).
- **C-5** Plan body length ≤ 400 lines.
- **C-6** Gate matrix per PR (inherited from project memory):
  `X_ARCH=riscv64 cargo test --workspace` ≥ 350 green; `cargo fmt
  --check`; `make clippy`; `make linux` → `Welcome to Buildroot`;
  `DEBUG=n make debian` → `debian login:`; difftest corpus
  (archLayout-04 green set) zero new divergences.
- **C-7** No modifications to assembly files (inherited
  `feedback_no_modify_asm.md`).
- **C-8** `arch_isolation` test (`xemu/xcore/tests/arch_isolation.rs`)
  passes unchanged; `SEAM_ALLOWED_SYMBOLS` and `BUS_DEBUG_STRING_PINS`
  remain byte-identical (verified by Phase 1 exit check).

---
