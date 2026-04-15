# `aclintSplit` PLAN `01`

> Status: Revised
> Feature: `aclintSplit`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `none` (no `00_MASTER.md`; inherited directives still binding — see Master Compliance)

---

## Summary

Split the monolithic `Aclint` at `xemu/xcore/src/arch/riscv/device/intc/aclint.rs`
into three spec-mandated sub-devices — **MSWI**, **MTIMER**, **SSWI** —
each implementing `device::Device` in its own file under
`arch/riscv/device/intc/aclint/`. A **thin façade struct `Aclint`**
survives: it composes the three sub-devices and installs them on the
bus in a single call (`Aclint::new(irq, ssip).install(&mut bus, base)
-> usize` returning `mtimer_idx`). This honours the inherited
project-memory directive "preserve `Aclint` façade for BootConfig
ergonomics", keeps `SEAM_ALLOWED_SYMBOLS` stable (the only seam symbol
is still `"Aclint"`), and yields three independently constructible,
independently testable devices. Guest-visible MMIO map (base
`0x0200_0000`, size `0x1_0000`, offsets `{0x0000, 0x4000, 0x4004,
0xBFF8, 0xBFFC, 0xC000}`) is byte-identical pre/post. Ships as
**1 PR (structural split + `mtimer_idx` rename) + 1 pre-merge validation
gate**, matching the archLayout-04 two-phase shape.

## Log

[**Feature Introduce**]

Retained from `00_PLAN`: three sibling modules `mswi.rs` / `mtimer.rs` /
`sswi.rs` under `arch/riscv/device/intc/aclint/`, each owning only its
spec-mandated state; region plan `0x4000 + 0x8000 + 0x4000 = 0x1_0000`
unchanged; per-sub-device `mmio_regs!` with local offsets.

New in round 01: the façade is a **struct**, not a free function.
`pub struct Aclint { mswi: Mswi, mtimer: Mtimer, sswi: Sswi }` with
`Aclint::new(irq, ssip) -> Self` and `Aclint::install(self, bus, base)
-> usize` that does the three `Bus::add_mmio` calls and returns
`mtimer_idx`. The three sub-device types `Mswi` / `Mtimer` / `Sswi`
stay module-private to `intc::aclint` (only the `Aclint` façade is
re-exported through `intc::mod.rs`), so no new seam symbols leak.

[**Review Adjustments**]

- **R-001 (CRITICAL, accepted)**: revived `Aclint` as a thin composite
  struct per TR-1 option (b). Façade surface preserved; `cpu/mod.rs:20`
  import and `SEAM_ALLOWED_SYMBOLS` entry `"Aclint"` remain untouched.
- **R-002 (HIGH, accepted)**: enumerated the exact `arch_isolation.rs`
  delta under Phase 1 — with the revived `Aclint` façade the allow-list
  requires **no change**. `Mswi`/`Mtimer`/`Sswi` are pub under
  `arch/riscv/device/intc/aclint/` but never re-exported through a seam
  file; `intc/mod.rs` (seam) still only `pub mod aclint;`. Debug-string
  pins `BUS_DEBUG_STRING_PINS` unchanged: `add_mmio("aclint", …)` stays
  in `arch/riscv/cpu/mod.rs` (not `device/bus.rs`); the `"aclint"`
  needle in `bus.rs` remains at count 0.
- **R-003 (MEDIUM, accepted)**: `Aclint::new(irq, ssip)` takes the SSIP
  `Arc<AtomicBool>` as an explicit parameter, mirroring today's
  `Aclint::new(irq.clone(), bus.ssip_flag())` at `cpu/mod.rs:66`. No
  implicit `bus.ssip_flag()` lookup inside `install`.
- **R-004 (MEDIUM, accepted)**: reconciled test inventory — the
  unmapped-offset assertion is one per sub-device (total 3), explicitly
  listed under Phase 1 and matching V-E-1.
- **R-005 (MEDIUM, accepted)**: collapsed `mtimer_idx` rename into PR1.
  PR3 dropped; pre-merge validation is a gate matrix sweep against PR1
  before merge, not its own commit. Net: 1 PR + 1 gate.
- **R-006 (LOW, accepted)**: added Phase 1 note that the `bus.rs`
  doc-comment rewrite does not change `BUS_DEBUG_STRING_PINS` (needle is
  `"aclint"` in quotes, expected count 0; the comment says `ACLINT`
  uppercase without quotes).
- **R-007 (LOW, accepted)**: added explicit sentence under Invariants
  I-3 documenting that MSWI and SSWI have no tick-driven state today
  (MSIP/SSIP toggle on MMIO write only), so moving them off the
  every-step path is behaviour-preserving by construction.

[**Master Compliance**]

No `00_MASTER.md` this round (user skipped MASTER). Inherited binding
directives from `archModule` / `archLayout` continue to apply:

- **00-M-001** — no global `trait Arch`. Honoured: sub-devices implement
  `device::Device`.
- **00-M-002** — topic-organised `arch/<name>/`. Honoured: split stays
  under `arch/riscv/device/intc/aclint/`.
- **01-M-001** — no `selected` alias word. Honoured.
- **01-M-002** — clean, concise, elegant. Honoured: façade is a 3-field
  struct; `install` is three `add_mmio` calls.
- **01-M-003** — no redundant arch-validity checks. Honoured: inherits
  existing `riscv` cfg gating on `arch/riscv/`.
- **01-M-004** — `cpu/`/`device/`/`isa/` top-level = trait APIs + tiny
  cfg patches only. Honoured: no new concrete devices in `device/`;
  `device/intc/mod.rs` does not exist (the seam is
  `arch/riscv/device/intc/mod.rs`, under arch/).

### Changes from Previous Round

[**Added**]

- Thin `Aclint` façade struct (`Aclint::new` + `Aclint::install`).
- Explicit `arch_isolation.rs` analysis: no allow-list / pin changes
  required.
- Explicit per-sub-device `unmapped_offset_returns_zero` entries (3).
- Explicit MSWI/SSWI no-tick behaviour-preservation argument under I-3.

[**Changed**]

- Façade shape: free `mount(...)` function → struct method `install`.
  Reason: honour inherited "preserve `Aclint` façade" directive; avoid
  seam-symbol churn in `SEAM_ALLOWED_SYMBOLS`.
- PR count: 3 → 1 (+ a pre-merge gate). Reason: reviewer R-005 — PR2
  was one rename, PR3 was zero code; bisection signal from splitting is
  weak and gate-matrix cost is multiplied.
- `Aclint::new` signature takes `ssip` explicitly. Reason: reviewer
  R-003 — mirrors today's constructor, trivially unit-testable without
  a `Bus`.

[**Removed**]

- Free-function `mount(...)` API from `00_PLAN:296`.
- PR2 and PR3 as independent PRs (merged into PR1 / pre-merge gate).

[**Unresolved**]

- **Multi-hart fan-out** (per-hart `msip` / `mtimecmp` arrays) —
  deferred to `multiHart` task; stated as NG-1.
- **`Device::mtime` default method** — kept; removal belongs under
  `directIrq` per NG-2.
- **`Bus::take_ssip` / `Bus::ssip_flag`** — kept; the lock-free
  `Arc<AtomicBool>` hand-off pattern survives until `directIrq` lets
  devices signal the PLIC directly.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 (CRITICAL) | Accepted | Revived `Aclint` as thin composite struct; Data Structure, API Surface, Phase 1 updated. |
| Review | R-002 (HIGH) | Accepted | Phase 1 documents that `SEAM_ALLOWED_SYMBOLS` and `BUS_DEBUG_STRING_PINS` need no edits; sub-devices are never re-exported through a seam file. |
| Review | R-003 (MEDIUM) | Accepted | `Aclint::new(irq: IrqState, ssip: Arc<AtomicBool>)` takes ssip explicitly; `install` contains no implicit `bus.ssip_flag()` lookup. |
| Review | R-004 (MEDIUM) | Accepted | Phase 1 test list updated to MSWI=3 / MTIMER=6 / SSWI=4; `unmapped_offset_returns_zero` appears in all three sub-device files. |
| Review | R-005 (MEDIUM) | Accepted | PR1 absorbs the `mtimer_idx` rename; PR2 eliminated; PR3 becomes a pre-merge validation gate (not a commit). |
| Review | R-006 (LOW) | Accepted | Phase 2 note: `BUS_DEBUG_STRING_PINS` needle is the quoted literal `"aclint"` (count 0); the `ACLINT`-uppercase doc-comment rewrite does not affect the pin. |
| Review | R-007 (LOW) | Accepted | I-3 now states MSWI/SSWI have no tick-driven state; the default empty `Device::tick` covers them. |
| Trade-off | TR-1 | Adopted | Façade is a thin struct (option b). |
| Trade-off | TR-2 | Concurred | Three regions kept. |
| Trade-off | TR-3 | Concurred | Rename-only NG-5 scope; bus-mtime removal deferred to `directIrq`. |
| Master | 00-M-001 (inherited) | Applied | Sub-devices implement `device::Device`; no `trait Arch`. |
| Master | 00-M-002 (inherited) | Applied | Split nests under `arch/riscv/device/intc/aclint/`. |
| Master | 01-M-001 (inherited) | Applied | No `selected` identifier anywhere. |
| Master | 01-M-002 (inherited) | Applied | Façade is a 3-field struct; `install` is three calls. |
| Master | 01-M-003 (inherited) | Applied | No new cfg scaffolding. |
| Master | 01-M-004 (inherited) | Applied | `device/` top-level untouched; concrete types live under `arch/riscv/`. |

> Rules satisfied: every CRITICAL / HIGH review finding appears above with explicit reasoning; every inherited master directive is reconciled.

---

## Spec {Core specification}

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

## Implement {detail design}

### Execution Flow

[**Main Flow**]

1. **PR1 — structural split + `mtimer_idx` rename (single commit).**
   `git mv xemu/xcore/src/arch/riscv/device/intc/aclint.rs
           xemu/xcore/src/arch/riscv/device/intc/aclint/mod.rs`
   (history follows MTIMER, the largest tenant). Carve `Mswi` + `Sswi`
   state + `Device` arms into new `mswi.rs` / `sswi.rs`; trim `mod.rs`
   to the `Aclint` façade + sub-module declarations + cross-register
   mount test. Re-home 11 existing tests per the Phase 1 table. Update
   `cpu/mod.rs:61-69` to the two-line `Aclint::new(...).install(...)`
   pattern. Rename `Bus::aclint_idx` → `Bus::mtimer_idx` in
   `device/bus.rs` (field + 5 call sites + doc-comment lines 1-2 and
   131-134).
2. **Pre-merge validation gate** (no commit). Run the full C-6 matrix
   before merge: 350 tests + fmt + clippy + `make linux` + `DEBUG=n make
   debian` + difftest corpus. Any failure reverts PR1 or issues a
   fix-up commit on the same PR branch.

[**Failure Flow**]

1. `add_mmio` panic (`"overlaps"`) — `install` passed a wrong
   sub-offset; fix in-phase by rechecking `{base+0x0000, 0x4000}`,
   `{base+0x4000, 0x8000}`, `{base+0xC000, 0x4000}` against the
   constants.
2. `mtime_write_ignored` regression — MTIMER's `write` match must have
   no arm for `MtimeLo`/`MtimeHi` so writes fall through to `_ => {}`
   silently. Covered by V-UT-2.
3. `Bus::mtime` returns 0 post-split — `mtimer_idx` never set;
   `set_timer_source` must be called with `install`'s return value.
   Covered by V-F-1.
4. SSIP edge lost — caused by `Sswi::new` receiving a fresh
   `Arc<AtomicBool>` instead of `bus.ssip_flag()`. `Aclint::new`
   signature forces the caller to pass the shared `Arc`; no implicit
   `bus.ssip_flag()` lookup. Covered by V-F-2.
5. `arch_isolation` test fails — caused by accidentally widening a
   sub-device to `pub` at `intc/mod.rs` or `intc/aclint/mod.rs`. Fix:
   keep sub-device structs `pub(super)`; only `Aclint` is `pub`.
   Covered by V-IT-2.
6. `make linux` / `make debian` divergence — bisect against pre-PR1
   HEAD; quarantine PR1 until green.
7. `cargo fmt` / `clippy` failure — fix in-phase; any new `unsafe` or
   `.unwrap()` blocks the PR.

[**State Transition**]

- **S0** (archLayout-04 landed) → **S1** (PR1 structural split +
  `mtimer_idx` rename; 350 tests green, `make run` HIT GOOD TRAP) →
  **S2** (pre-merge gate green on full C-6 matrix — task lands).

### Implementation Plan

[**Phase 1 — PR1: structural split + `mtimer_idx` rename**]

Files touched:

- `arch/riscv/device/intc/aclint.rs` → `arch/riscv/device/intc/aclint/mod.rs`
  (`git mv`). `mod.rs` retains: `mod mswi; mod mtimer; mod sswi;`
  (private); `pub struct Aclint`; `impl Aclint { pub fn new; pub fn
  install; }`; cross-register mount test; the old
  `reset_clears_state` test relocated here (drives all three
  sub-devices through the bus, proving `install` wires them).
- `arch/riscv/device/intc/aclint/mswi.rs` — new. Holds
  `pub(super) struct Mswi`; `mmio_regs! { Msip = 0x0000 }`; `set_msip`
  helper; `Device::{read, write, reset}` impls. **3 tests**:
  `msip_set_and_clear`, `unmapped_offset_returns_zero` (MSWI-local
  offset 0x0100), `reset_clears_state` (MSWI slice: MSIP set → reset →
  MSIP clear).
- `arch/riscv/device/intc/aclint/mtimer.rs` — new. Holds
  `pub(super) struct Mtimer`; `mmio_regs!` with
  `MtimecmpLo/Hi/MtimeLo/Hi` at local offsets `0x0000 / 0x0004 /
  0x7FF8 / 0x7FFC`; `SYNC_INTERVAL = 512`; `sync_wallclock`;
  `check_timer`; `Device::{read, write, tick, mtime, reset}` impls.
  **6 tests**: `mtime_advances_after_sync`,
  `mtime_frozen_between_syncs`, `mtimecmp_sets_mtip`,
  `mtimecmp_max_clears_mtip`, `mtime_write_ignored`,
  `unmapped_offset_returns_zero` (MTIMER-local offset 0x0100).
- `arch/riscv/device/intc/aclint/sswi.rs` — new. Holds
  `pub(super) struct Sswi`; `mmio_regs! { Setssip = 0x0000 }`;
  `Device::{read, write, reset}` impls. **4 tests**:
  `setssip_is_edge_triggered`, `setssip_read_returns_zero`,
  `setssip_write_zero_no_effect`, `unmapped_offset_returns_zero`
  (SSWI-local offset 0x0100).
- `arch/riscv/cpu/mod.rs:61-69` — replace the three-line `add_mmio` +
  `set_timer_source` block with
  ```
  let mtimer_idx =
      Aclint::new(irq.clone(), bus.ssip_flag()).install(&mut bus, 0x0200_0000);
  bus.set_timer_source(mtimer_idx);
  ```
  The `use super::device::intc::{aclint::Aclint, plic::Plic};` line
  stays byte-identical.
- `device/bus.rs` — rename `aclint_idx` → `mtimer_idx`: field at line
  43; initialiser at line 57; setter body at line 115; reader at line
  126; fast-path branch at line 135; slow-path skip condition at line
  148. Update doc comments at lines 1-2 and 131-134 from "ACLINT" →
  "MTIMER" where the text refers to the fast-path tenant. Method name
  `set_timer_source` **unchanged** (it describes a role, not a
  register).
- `arch/riscv/device/intc/mod.rs` — **unchanged** (still `pub mod
  aclint; pub mod plic;`). Seam file at this path.
- `xemu/xcore/tests/arch_isolation.rs` — **unchanged**.
  `SEAM_ALLOWED_SYMBOLS` still pins `"Aclint"`; sub-devices are
  `pub(super)` inside `intc::aclint` and never re-exported. Phase-1
  exit check: `grep -n 'pub struct Mswi\|pub struct Mtimer\|pub struct
  Sswi' xemu/xcore/src/arch/riscv/device/intc/aclint/*.rs` returns
  **zero** matches; all three must be `pub(super)`.

Test inventory — total 11 re-homed from the pre-split file + 3 new
isolation tests (V-UT-4/5/6) + 1 mount integration test (V-IT-6):

| Pre-split (`aclint.rs`) | Post-split home | Local offset |
|-------------------------|-----------------|--------------|
| `mtime_advances_after_sync` | `mtimer.rs` | n/a (tick) |
| `mtime_frozen_between_syncs` | `mtimer.rs` | n/a (tick) |
| `mtimecmp_sets_mtip` | `mtimer.rs` | 0x0000 / 0x0004 |
| `mtimecmp_max_clears_mtip` | `mtimer.rs` | 0x0000 / 0x0004 |
| `msip_set_and_clear` | `mswi.rs` | 0x0000 |
| `setssip_is_edge_triggered` | `sswi.rs` | 0x0000 |
| `setssip_read_returns_zero` | `sswi.rs` | 0x0000 |
| `setssip_write_zero_no_effect` | `sswi.rs` | 0x0000 |
| `unmapped_offset_returns_zero` | split into `mswi.rs` + `mtimer.rs` + `sswi.rs` | 0x0100 each |
| `mtime_write_ignored` | `mtimer.rs` | 0x7FF8 |
| `reset_clears_state` | `aclint/mod.rs` (cross-register integration) | global 0x0000 / 0x4000 / 0xC000 |

Gate before merge: see pre-merge validation gate below.

[**Phase 2 — pre-merge validation gate (no commit)**]

Run on the PR1 branch before merging to main:

- `X_ARCH=riscv64 cargo test --workspace` → 343 xcore lib + 1
  arch_isolation + 6 xdb = 350 pass.
- `X_ARCH=riscv64 cargo test --test arch_isolation -- --exact
  arch_isolation` → 1 pass.
- `cargo fmt --check` → clean.
- `make clippy` → no new warnings.
- `make run` → `HIT GOOD TRAP`.
- `timeout 60 make linux 2>&1 | tee /tmp/linux.log && grep -q 'Welcome
  to Buildroot' /tmp/linux.log`.
- `DEBUG=n timeout 180 make debian 2>&1 | tee /tmp/debian.log && grep
  -q 'debian login:' /tmp/debian.log`.
- Difftest corpus (archLayout-04 green set) — zero new divergences vs
  QEMU and Spike.

Any failure is remedied on the PR1 branch (fix-up commit); the gate is
the task-complete signal, not a separate PR.

---

## Trade-offs

- **T-1** Façade shape. **Chosen: thin struct (TR-1 option b).**
  `pub struct Aclint { mswi, mtimer, sswi }` with `new` + `install`.
  Preserves inherited "preserve `Aclint` façade" directive, keeps
  `SEAM_ALLOWED_SYMBOLS` stable, makes the call site one expression.
  Rejected: (a) free `mount(...)` function — silently deviates from
  the inherited directive and forces a seam-symbol migration.
  Rejected: (c) `Bus::add_aclint(...)` — puts arch-specific knowledge
  in generic `Bus`, violates 01-M-004.
- **T-2** Region granularity. **Chosen: three regions `{0x4000,
  0x8000, 0x4000}`.** Spec-faithful; each sub-device owns a clean
  offset-0-local register space; future per-hart growth in `multiHart`
  only grows the owning region. Rejected: one `0x1_0000` region with
  internal dispatcher — reintroduces the coupling the split removes
  and fails G-1.
- **T-3** NG-5 Bus-residual scope. **Chosen: rename `aclint_idx` →
  `mtimer_idx` only.** Aligns the field name with what it points at;
  keeps the `Bus::mtime` fast path intact. Rejected: also drop
  `Bus::mtime` / the `Device::mtime` default method — per-step
  dispatch cost regression measured in archLayout baseline; unrelated
  to the split; belongs under `directIrq` (NG-2).

---

## Validation

[**Unit Tests**]

- **V-UT-1** MSWI — `msip_set_and_clear`, `reset_clears_state` (MSWI
  slice). Assert `irq.load() & MSIP` toggles via write at local offset
  `0x0000`; read-back matches written value; reset clears field.
- **V-UT-2** MTIMER — `mtime_advances_after_sync`,
  `mtime_frozen_between_syncs`, `mtimecmp_sets_mtip`,
  `mtimecmp_max_clears_mtip`, `mtime_write_ignored`. Offsets local
  (0x0000 / 0x0004 / 0x7FF8).
- **V-UT-3** SSWI — `setssip_is_edge_triggered`,
  `setssip_read_returns_zero`, `setssip_write_zero_no_effect`.
  Offsets local (0x0000).
- **V-UT-4** `mswi_independent_of_mtimer` (new): construct `Mswi::new`
  alone, exercise MSIP, assert no MTIMER required. Proves I-1.
- **V-UT-5** `mtimer_independent_of_sswi` (new): construct
  `Mtimer::new` alone, exercise mtimecmp/mtime, assert `Bus::take_ssip`
  not required. Proves I-1.
- **V-UT-6** `sswi_independent_of_mswi` (new): construct `Sswi::new`
  with a fresh `Arc<AtomicBool>`, drive edge, assert no `IrqState`
  required. Proves I-1.

[**Integration Tests**]

- **V-IT-1** `X_ARCH=riscv64 cargo test --workspace` — 350 green at
  PR1 HEAD.
- **V-IT-2** `arch_isolation` integration test passes unchanged.
  PRIMARY gate: `diff` of `xemu/xcore/tests/arch_isolation.rs` between
  pre-PR1 and post-PR1 HEAD is empty.
- **V-IT-3** `make run` reaches `HIT GOOD TRAP`.
- **V-IT-4** `timeout 60 make linux 2>&1 | tee /tmp/linux.log && grep
  -q 'Welcome to Buildroot' /tmp/linux.log`.
- **V-IT-5** `DEBUG=n timeout 180 make debian 2>&1 | tee
  /tmp/debian.log && grep -q 'debian login:' /tmp/debian.log`.
- **V-IT-6** New integration test
  `aclint::mod::tests::install_wires_all_three`: build a `Bus`, call
  `Aclint::new(irq, ssip_flag).install(&mut bus, 0x0200_0000)`,
  perform all six guest-visible MMIO operations at their **global**
  offsets (MSIP write/read, mtimecmp lo/hi write, mtime lo/hi read,
  setssip edge), assert pre-split semantics hold bit-for-bit. This is
  the byte-compat gate for C-1 / I-2.

[**Failure / Robustness Validation**]

- **V-F-1** `bus.mtime()` returns a nonzero value after enough bus
  ticks to cross `SYNC_INTERVAL` — asserts `mtimer_idx` wire-up and
  the `Device::mtime` override on `Mtimer`. Absent `set_timer_source`,
  `Bus::mtime()` returns 0 and this fails.
- **V-F-2** `bus.take_ssip()` returns `true` exactly once after a
  guest write of 1 to `base + 0xC000` — asserts the `Arc<AtomicBool>`
  identity between `Bus::ssip_pending` and `Sswi::ssip`. Secondary
  textual gate: `rg '\bssip_pending\b' xemu/xcore/src` shows only
  `bus.rs` (source) and `arch/riscv/device/intc/aclint/sswi.rs`
  (consumer).
- **V-F-3** Per-phase bisection: PR1 must be green under `make fmt &&
  make clippy && cargo test --workspace` before the pre-merge gate
  runs.
- **V-F-4** Difftest corpus (archLayout-04 green set) — zero new
  divergences vs QEMU and Spike. Catches MTIMER wall-clock drift
  (`mtime` CSR divergence) if the tick loop stops calling
  `Mtimer::tick` every step.
- **V-F-5** `cargo clippy --all-targets -- -D warnings` — no new
  warnings; `cargo fmt --check` — no new diffs; no new `unsafe`; no
  new `.unwrap()` in non-test code.

[**Edge Case Validation**]

- **V-E-1** `unmapped_offset_returns_zero` — present in **each** of
  `mswi.rs`, `mtimer.rs`, `sswi.rs` (total 3 assertions, all at
  sub-device-local offset `0x0100`). Per-sub-device coverage matches
  I-1 (isolation).
- **V-E-2** `mtime_write_ignored` — writes at MTIMER-local offsets
  `0x7FF8` / `0x7FFC` do not mutate `Mtimer.mtime`; the field retains
  its last `sync_wallclock` value. In `mtimer.rs`.
- **V-E-3** `Bus::add_mmio` panics with `"overlaps"` if `install` is
  called twice on the same bus — asserted by
  `#[should_panic(expected = "overlaps")]` test in `aclint/mod.rs`.
- **V-E-4** `cargo build --no-default-features --features isa32` —
  RV32 build compiles (inherited archLayout V-E).

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 three sub-devices | V-UT-4; V-UT-5; V-UT-6 |
| G-2 byte-identical MMIO | V-IT-6; V-F-4; V-IT-3; V-IT-4; V-IT-5 |
| G-3 call-site ergonomics | V-IT-6 (`install` wires triple in one call); diff review of `cpu/mod.rs:61-69` |
| G-4 tests re-homed + isolation | V-UT-1; V-UT-2; V-UT-3; V-UT-4; V-UT-5; V-UT-6 |
| G-5 `mtimer_idx` rename | V-IT-1 post-rename; diff review of `device/bus.rs` |
| C-1 guest MMIO compat | V-IT-6; V-F-4; V-IT-4; V-IT-5 |
| C-2 `IrqState` bit semantics | V-UT-1; V-UT-2; V-F-4 |
| C-3 no new Bus API | Diff review of `device/bus.rs` public API |
| C-4 tests re-homed verbatim | V-UT-1; V-UT-2; V-UT-3; diff review |
| C-5 plan ≤ 400 lines | Body self-review |
| C-6 gate matrix | V-IT-1..V-IT-5; V-F-5 |
| C-7 no asm edits | Phase-1 diff review |
| C-8 `arch_isolation` pins stable | V-IT-2 (diff empty); Phase-1 grep exit check |
| I-1 independent sub-devices | V-UT-4; V-UT-5; V-UT-6 |
| I-2 external semantics | V-IT-6; V-F-4 |
| I-3 tick cadence | V-F-1; V-F-4 (MTIMER fast-path); V-UT-1 + V-UT-3 prove MSWI/SSWI state is MMIO-driven |
| I-4 ssip Arc identity | V-F-2 |
| I-5 hard_reset parity | V-UT-1..V-UT-3 (reset arms); V-IT-6 |
| I-6 no new unsafe / deps | V-F-5; Cargo.toml diff |
