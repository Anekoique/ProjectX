# `aclintSplit` PLAN `00`

> Status: Draft
> Feature: `aclintSplit`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none (inherits binding directives from `archModule` — see Master Compliance)

---

## Summary

Dissolve the monolithic `Aclint` device at `xemu/xcore/src/arch/riscv/device/intc/aclint.rs`
into the three spec-mandated functional units (riscv-aclint v1.0-rc4): **MSWI**
(machine software interrupts, `msip` at +0x0000), **MTIMER** (`mtimecmp`/`mtime`
at +0x4000 / +0xBFF8), **SSWI** (`setssip` at +0xC000). Each becomes its own
`Device`-implementing struct with a dedicated MMIO region. A thin constructor
helper (`intc::aclint::mount`) registers the triple on the bus in one call,
preserving `RVCore::with_config` ergonomics and keeping the combined
`[0x0200_0000, 0x0201_0000)` footprint bit-identical at every externally
observable offset (OpenSBI, am-tests, Linux DT all keep working). Bus-side
NG-5 residuals (`aclint_idx`, `set_timer_source`, `Bus::mtime`,
`Device::mtime`, `take_ssip`, `ssip_flag`) are narrowed — timer pointer
becomes `mtimer_idx` (rename for accuracy), the rest stay as-is and are
re-examined under `multiHart`/`plicGateway`/`directIrq`. Ships as three PRs,
each independently green-barable against the inherited gate matrix (343
xcore lib + 1 arch_isolation + 6 xdb = 350 tests, `cargo fmt --check`,
`make clippy`, `make linux`, `make debian`, difftest corpus).

## Log {None in 00_PLAN}

[**Feature Introduce**]

Three sibling modules under `arch/riscv/device/intc/aclint/`:

- `mswi.rs` — `Mswi { msip: u32, irq: IrqState }`, region size `0x4000`.
- `mtimer.rs` — `Mtimer { epoch, mtime, ticks, mtimecmp, irq: IrqState }`,
  region size `0x8000` (covers `[0x4000, 0xC000)` so `mtimecmp`@+0 and
  `mtime`@+0x7FF8 stay at the external offsets +0x4000 / +0xBFF8 when
  mounted at guest base +0x4000).
- `sswi.rs` — `Sswi { ssip: Arc<AtomicBool> }`, region size `0x4000`.
- `mod.rs` — re-exports the three types + provides a `mount` helper that
  calls `Bus::add_mmio` three times at the correct sub-offsets and wires
  the shared `IrqState` + `Arc<AtomicBool>` SSIP flag.

The façade is a **free function**, not a wrapper struct: no composite
`Aclint` type survives. Each of MSWI/MTIMER/SSWI is independently
constructible (`Mswi::new(irq)`, `Mtimer::new(irq)`,
`Sswi::new(ssip_flag)`), independently testable, and independently
mountable. This honours the spec — the three controllers are separate
devices that happen to share a vendor-convention MMIO base — while
keeping the call-site a single line. The `mount` helper returns the
`mtimer_idx: usize` so `Bus::set_timer_source` can still mark which
region owns `mtime`.

[**Review Adjustments**]

None — this is round 00.

[**Master Compliance**]

No new MASTER for `aclintSplit`. Inherited binding directives from
`archModule`/`archLayout` (still in force per project memory
`project_manual_review_progress.md`) are carried forward:

- **00-M-001** — no global `trait Arch`; per-concern traits only. Honoured:
  MSWI/MTIMER/SSWI each implement `device::Device`, not a meta-trait.
- **00-M-002** — topic-organised `arch/<name>/` layout. Honoured: the three
  sub-modules nest under `arch/riscv/device/intc/aclint/`.
- **01-M-001** — no `selected` alias word. Honoured: identifiers are
  `Mswi`/`Mtimer`/`Sswi`/`mount`.
- **01-M-002** — clean, concise, elegant. Honoured: `mount` is a 3-call
  helper; each sub-device owns only its spec-mandated state.
- **01-M-003** — no redundant arch-validity checks. Honoured: all three
  types live under `arch/riscv/device/intc/` behind the existing `riscv`
  cfg.
- **01-M-004** — `cpu/`, `device/`, `isa/` top-level = trait APIs + tiny
  cfg patches only. Honoured: `device/mod.rs` only loses the
  `Device::mtime` override usage pattern reference in its doc; no new
  top-level device types. The `Device::mtime` default method itself stays
  (used by `Mtimer`).

### Changes from Previous Round

[**Added**] All content (round 00).

[**Changed**] n/a.

[**Removed**] n/a.

[**Unresolved**]

- Multi-hart fan-out: the spec allows MSWI / MTIMER to hold **N** `msip`
  and **N** `mtimecmp` registers (one per hart). Scope here is hart-count
  = 1 to match the current single-hart core (`RVCore`). Per-hart arrays +
  the `mtimer_idx` → per-hart context mapping land under `multiHart`.
  Marked explicitly as NG-3.
- `Device::mtime` trait method: kept for MTIMER. A cleaner design would
  expose a dedicated `TimerSource` trait and drop the default method, but
  that touches every concrete device and the `Bus::mtime` fast path —
  deferred to `directIrq` when the bus stops brokering timer reads.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Master | 00-M-001 (inherited) | Applied | MSWI/MTIMER/SSWI implement `device::Device`; no `trait Arch`. |
| Master | 00-M-002 (inherited) | Applied | New modules nest under `arch/riscv/device/intc/aclint/`; no flat seams. |
| Master | 01-M-001 (inherited) | Applied | Type names follow spec (`Mswi`/`Mtimer`/`Sswi`); no `selected`. |
| Master | 01-M-002 (inherited) | Applied | `mount` helper is three `add_mmio` calls; each sub-device state is minimal. |
| Master | 01-M-003 (inherited) | Applied | No new cfg scaffolding; existing `riscv` gate covers all three. |
| Master | 01-M-004 (inherited) | Applied | `device/mod.rs` stays trait-only; no concrete arch device surfaces in `device/`. |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here. (None — round 00.)
> - Every Master directive must appear here. (Done via inherited rows.)
> - Rejections must include explicit reasoning. (None — no rejections.)

---

## Spec {Core specification}

[**Goals**]

- **G-1** Replace the single `Aclint` device with three independently
  constructible devices — `Mswi`, `Mtimer`, `Sswi` — one per spec-defined
  controller, each owning only its spec-mandated state.
- **G-2** Preserve the externally observable MMIO map: guest accesses at
  `0x0200_0000 + {0x0000, 0x4000, 0x4004, 0xBFF8, 0xBFFC, 0xC000}` behave
  identically (same semantics, same side effects, same irq-state edges).
  OpenSBI firmware, am-tests (`test.h`), and any Linux DT entry keep
  working.
- **G-3** Preserve `RVCore::with_config` call-site ergonomics: one
  helper call registers all three regions and returns the MTIMER index
  for `Bus::set_timer_source`.
- **G-4** Re-home every existing test in `aclint.rs` under the correct
  sub-device's `#[cfg(test)]` module; add tests proving each sub-device
  is usable in isolation (MSWI alone without MTIMER, and symmetric).
- **G-5** Narrow the NG-5 Bus-side residuals that this task can close
  without conflating with `multiHart`/`directIrq`: rename
  `aclint_idx` → `mtimer_idx` and `set_timer_source` keeps that role
  (naming alignment only); `take_ssip`/`ssip_flag`/`Bus::mtime` stay.

**Non-Goals**

- **NG-1** Per-hart MSIP / MTIMECMP arrays — deferred to `multiHart`.
- **NG-2** Replacing `Device::mtime` with a dedicated `TimerSource`
  trait — deferred to `directIrq`.
- **NG-3** Changing the MMIO base (`0x0200_0000`) or overall footprint
  (`0x1_0000`) — would force firmware/DT churn. Out of scope.
- **NG-4** Removing `Bus::take_ssip` / `Bus::ssip_flag` — SSWI still
  needs a cross-device edge-flag hand-off; the lock-free `Arc<AtomicBool>`
  survives. Cleanup revisits under `directIrq` once devices signal the
  PLIC directly.
- **NG-5** Re-wiring the slow-tick divisor or split-tick policy
  (`SLOW_TICK_DIVISOR = 64`, ACLINT-every-step) — the Bus tick loop
  keeps ticking the MTIMER every step (via `mtimer_idx`) and the other
  two every `SLOW_TICK_DIVISOR` steps, matching today's `aclint_idx`
  behaviour verbatim.
- **NG-6** Renaming / re-numbering existing tests in other modules; we
  only move the ACLINT tests.
- **NG-7** Altering `SYNC_INTERVAL = 512` or the 10 MHz wall-clock scale;
  these are MTIMER-internal constants moved verbatim.

[**Architecture**]

Before (single file, single region):

```
arch/riscv/device/intc/
├── aclint.rs         (276 lines: one struct, one Device impl,
│                      three register-kind concerns bundled)
├── plic.rs
└── mod.rs            (pub mod aclint; pub mod plic;)
```

After (directory, three siblings + helper):

```
arch/riscv/device/intc/
├── aclint/
│   ├── mod.rs        (pub mod mswi; pub mod mtimer; pub mod sswi;
│   │                  pub use …; pub fn mount(bus, irq) -> usize)
│   ├── mswi.rs       (Mswi + Device impl + MSIP tests)
│   ├── mtimer.rs     (Mtimer + Device impl + mtime/mtimecmp tests)
│   └── sswi.rs       (Sswi + Device impl + setssip tests)
├── plic.rs
└── mod.rs            (pub mod aclint; pub mod plic;)
```

Region plan (guest-visible, unchanged):

```
0x0200_0000 ─┬── MSWI   (0x4000 B)  → msip at +0x0000
             │
0x0200_4000 ─┼── MTIMER (0x8000 B)  → mtimecmp at +0x0000 (= global +0x4000),
             │                        mtime    at +0x7FF8 (= global +0xBFF8)
             │
0x0200_C000 ─┴── SSWI   (0x4000 B)  → setssip at +0x0000 (= global +0xC000)

Total: 0x1_0000 (= current size). Zero gap, zero overlap — checked by
Bus::add_mmio.
```

Runtime interaction:

```
          ┌────────── shared IrqState (Arc<AtomicU64>) ──────────┐
          │                                                      │
          ▼                                                      ▼
   ┌──────────┐      ┌──────────┐                         ┌──────────┐
   │   Mswi   │      │  Mtimer  │   sync_wallclock        │   Sswi   │
   │ msip→MSIP│      │ mtime,   │   every SYNC_INTERVAL   │ ssip→    │
   │          │      │ mtimecmp │   ticks; MTIP           │ Arc<Bool>│
   │          │      │ →MTIP    │                         │          │
   └────┬─────┘      └────┬─────┘                         └────┬─────┘
        │                 │                                    │
        └──── Bus ────────┴─────────── Bus::take_ssip ─────────┘
                          │
                          ▼
                    Bus::mtime()  ← mtimer_idx → MTIMER.mtime()
```

[**Invariants**]

- **I-1** Each of `Mswi`/`Mtimer`/`Sswi` compiles and tests pass **in
  isolation** — no sub-device references another sub-device's state. The
  only shared values are `IrqState` (cloned `Arc`) and the SSIP
  `Arc<AtomicBool>`; both predate this split.
- **I-2** External MMIO semantics are byte-identical: for every
  `(offset, size, value)` triple used by OpenSBI fw_jump, am-tests, or
  Linux boot, post-split `bus.read/write` returns the same result the
  pre-split code returned and asserts the same `IrqState` bits.
- **I-3** Tick cadence: `mtime` advances on every bus tick (via
  `mtimer_idx` fast path); MSWI and SSWI tick on the slow path.
  `Bus::mtime()` returns the current MTIMER value with zero MMIO
  dispatch — same latency as before.
- **I-4** `Bus::take_ssip` still drains the shared `Arc<AtomicBool>` —
  the flag pointer moves from `Aclint::ssip` to `Sswi::ssip`; the
  `Arc` identity is stable across the split.
- **I-5** `hard_reset` / `reset` on the bus resets all three sub-devices
  to power-on state identically to the old `Aclint::reset`.
- **I-6** No new `unsafe`, no new dependencies, no new global mutable
  state.

[**Data Structure**]

```rust
// arch/riscv/device/intc/aclint/mswi.rs
pub struct Mswi {
    msip: u32,
    irq: IrqState,
}

// arch/riscv/device/intc/aclint/mtimer.rs
pub struct Mtimer {
    epoch: Instant,
    mtime: u64,
    ticks: u64,
    mtimecmp: u64,
    irq: IrqState,
}

// arch/riscv/device/intc/aclint/sswi.rs
pub struct Sswi {
    ssip: Arc<AtomicBool>,
}

// arch/riscv/device/intc/aclint/mod.rs
pub use {mswi::Mswi, mtimer::Mtimer, sswi::Sswi};

// Per-sub-device register enums (via mmio_regs! macro, offsets are
// local to each sub-device's region base):
// Mswi:    { Msip = 0x0 }
// Mtimer:  { MtimecmpLo = 0x0, MtimecmpHi = 0x4,
//            MtimeLo    = 0x7FF8, MtimeHi    = 0x7FFC }
// Sswi:    { Setssip = 0x0 }
```

[**API Surface**]

```rust
// arch/riscv/device/intc/aclint/mod.rs
use crate::device::{Device, IrqState, bus::Bus};

/// Register the three ACLINT sub-devices on `bus` at the conventional
/// RISC-V layout base `0x0200_0000`:
///   MSWI   at base+0x0000 (size 0x4000)
///   MTIMER at base+0x4000 (size 0x8000)
///   SSWI   at base+0xC000 (size 0x4000)
/// Returns the bus index of the MTIMER region, to be passed to
/// `Bus::set_timer_source`.
pub fn mount(bus: &mut Bus, base: usize, irq: IrqState) -> usize;

// Constructors (all pub for direct test / out-of-bus use):
impl Mswi   { pub fn new(irq: IrqState) -> Self; }
impl Mtimer { pub fn new(irq: IrqState) -> Self; }
impl Sswi   { pub fn new(ssip: Arc<AtomicBool>) -> Self; }

// Device impls — each implements only what the sub-device owns:
impl Device for Mswi   { fn read; fn write; fn reset; }
impl Device for Mtimer { fn read; fn write; fn tick; fn mtime; fn reset; }
impl Device for Sswi   { fn read; fn write; fn reset; }
```

Call-site (`RVCore::with_config`, `arch/riscv/cpu/mod.rs:61-69`) shrinks
to:

```rust
let mtimer_idx = intc::aclint::mount(&mut bus, 0x0200_0000, irq.clone());
bus.set_timer_source(mtimer_idx);
```

[**Constraints**]

- **C-1** Guest-visible MMIO base / size / offsets for all six
  documented registers (MSIP, MtimecmpLo/Hi, MtimeLo/Hi, Setssip) are
  bit-identical pre/post split. Validated by V-F-2.
- **C-2** No behavioural change to `IrqState` bit semantics — `MSIP` and
  `MTIP` still set/clear via `irq.set/clear` with identical edge timing.
- **C-3** `mount` uses only `Bus::add_mmio`; no new bus API added. The
  three regions must not overlap each other or RAM — `add_mmio` asserts
  this automatically.
- **C-4** Tests inherited from `aclint.rs` are re-homed, not rewritten.
  Assertions (`assert_ne!`, `assert_eq!`) and offsets are preserved byte
  for byte. One new test per sub-device asserts isolation (see V-UT-4/5/6).
- **C-5** Plan body length ≤ 300 lines (inherits archLayout C-7).
- **C-6** Gate matrix per PR (inherited from `project_manual_review_progress.md`):
  `X_ARCH=riscv64 cargo test --workspace`
    (≥ 343 xcore lib + 1 arch_isolation + 6 xdb = 350 green);
  `cargo fmt --check`;
  `make clippy`;
  `make linux` → `Welcome to Buildroot`;
  `make debian` (DEBUG=n) → `debian login:`;
  difftest corpus (archLayout-04 green set) zero new divergences.
- **C-7** No modifications to assembly files (project memory
  `feedback_no_modify_asm.md`). Validated by filesystem diff review.

---

## Implement {detail design}

### Execution Flow

[**Main Flow**]

1. **PR1 — Structural split (no behaviour change).**
   `git mv xemu/xcore/src/arch/riscv/device/intc/aclint.rs
            xemu/xcore/src/arch/riscv/device/intc/aclint/mod.rs`
   to preserve history for MTIMER (the largest tenant). Add `mswi.rs` and
   `sswi.rs` as new files. Carve the MSWI / SSWI fields + their
   `Device::read`/`write`/`reset` arms out of the merged `Device for Aclint`
   impl into their own files; what remains in `mod.rs` becomes `Mtimer` +
   `mount`. Re-home tests by register family. `RVCore::with_config` edit:
   replace the three-line ACLINT registration block with
   `let mtimer_idx = intc::aclint::mount(&mut bus, 0x0200_0000, irq.clone());
    bus.set_timer_source(mtimer_idx);`.
2. **PR2 — `mtimer_idx` rename + doc refresh.**
   Rename `Bus::aclint_idx` → `Bus::mtimer_idx` (field + call sites +
   internal doc). `set_timer_source` / `Bus::mtime` / `Bus::tick` now
   read more accurately — the field is named for what it actually is (a
   MTIMER pointer). No behaviour change. `device/bus.rs` doc updated
   (line 1-2) to drop "ACLINT every step" wording in favour of
   "MTIMER every step" for precision.
3. **PR3 — Gate sweep + difftest.**
   Run the full gate matrix (C-6) including `make linux` / `make debian`
   / difftest corpus. No code change expected; if divergence surfaces it
   bisects to PR1 (structural) vs PR2 (rename-only).

[**Failure Flow**]

1. `add_mmio` panic ("overlaps") — `mount` passed wrong sub-offset /
   size; fix in-phase by comparing `{base+0x0000, 0x4000}`,
   `{base+0x4000, 0x8000}`, `{base+0xC000, 0x4000}` to the constants.
2. Test regression for `mtime_write_ignored` (line 252 of old `aclint.rs`)
   — MTIMER's `write` match must have no arm for `MtimeLo`/`MtimeHi` so
   writes at those offsets fall through to `_ => {}` silently.
3. `Bus::mtime` returns 0 post-split — `mtimer_idx` never set; PR1 /
   PR2 must assert `set_timer_source` is still called. Covered by
   V-F-1.
4. `setssip` edge lost — if `Sswi::new` is called with a fresh
   `Arc<AtomicBool>` instead of `bus.ssip_flag()`, `Bus::take_ssip`
   would observe the wrong flag. `mount` must obtain the flag from
   `bus.ssip_flag()` before adding the SSWI region. Covered by V-F-2.
5. `make linux` / `make debian` divergence — bisect PR1 vs PR2.
   Quarantine the PR; PR3 does not land until green.
6. `cargo fmt` / `clippy` failure — fix in-phase; any new `unsafe` or
   `.unwrap()` addition blocks the PR (inherited C-6).

[**State Transition**]

- **S0** (archLayout-04 landed) → **S1** (PR1: three-file split + `mount`
  helper; 350 tests green; `make run` HIT GOOD TRAP) →
  **S2** (PR2: `mtimer_idx` rename; 350 tests green) →
  **S3** (PR3: full boot matrix + difftest green — task complete).

### Implementation Plan

[**Phase 1 — Structural split (PR1)**]

Files touched:

- `arch/riscv/device/intc/aclint.rs` → `arch/riscv/device/intc/aclint/mod.rs`
  (`git mv`). Trimmed to: `mod mswi; mod mtimer; mod sswi; pub use …;
  pub fn mount(…); mmio_regs! for shared `Reg` removed` — the per-sub-device
  macros move into their files.
- `arch/riscv/device/intc/aclint/mswi.rs` — new. Holds MSWI + its
  `mmio_regs! { enum Reg { Msip = 0x0 } }`, `set_msip` helper, 2 tests
  (`msip_set_and_clear`, `unmapped_offset_returns_zero` regionalised to
  MSWI range).
- `arch/riscv/device/intc/aclint/mtimer.rs` — new. Holds MTIMER +
  `mmio_regs! { enum Reg { MtimecmpLo = 0x0, MtimecmpHi = 0x4,
  MtimeLo = 0x7FF8, MtimeHi = 0x7FFC } }`, `sync_wallclock`,
  `check_timer`, `SYNC_INTERVAL`, 5 tests (`mtime_advances_after_sync`,
  `mtime_frozen_between_syncs`, `mtimecmp_sets_mtip`,
  `mtimecmp_max_clears_mtip`, `mtime_write_ignored`) with offsets
  rewritten to MTIMER-local (+0x0000 / +0x0004 / +0x7FF8).
- `arch/riscv/device/intc/aclint/sswi.rs` — new. Holds SSWI + `mmio_regs! {
  enum Reg { Setssip = 0x0 } }`, 3 tests (`setssip_is_edge_triggered`,
  `setssip_read_returns_zero`, `setssip_write_zero_no_effect`) with
  offsets rewritten to +0x0000.
- `arch/riscv/cpu/mod.rs:61-69` — replace ACLINT block with `mount` call;
  `use super::device::intc::{aclint::Aclint, plic::Plic};` becomes
  `use super::device::intc::{aclint, plic::Plic};`.
- `reset_clears_state` test (the cross-register one) — moved to
  `aclint/mod.rs` as an integration test that drives all three sub-devices
  via the bus (`bus.write(base+0, …); bus.write(base+0x4000, …);
  bus.write(base+0xC000, …)`), proving `mount` wires them correctly.

Gate: `X_ARCH=riscv64 cargo test --workspace` (350 green), `make fmt && make
clippy && make run` (HIT GOOD TRAP).

[**Phase 2 — `mtimer_idx` rename (PR2)**]

Files touched:

- `device/bus.rs` — `aclint_idx` → `mtimer_idx` (field line 43;
  initialiser line 57; setter line 114-115; `Bus::mtime` lines 126-128;
  `Bus::tick` lines 135, 148; PR2 doc refresh lines 1-2). Method name
  `set_timer_source` unchanged.
- `arch/riscv/cpu/mod.rs:61` — local `let aclint_idx = …` (already
  renamed to `mtimer_idx` in PR1 via the `mount` return; this phase
  just completes the field rename).

Gate: same as Phase 1 — 350 tests green, boot via `make run`.

[**Phase 3 — Boot + difftest sweep (PR3)**]

No code changes. Runs the full matrix:

- `timeout 60 make linux 2>&1 | tee /tmp/linux.log && grep -q 'Welcome to
  Buildroot' /tmp/linux.log`.
- `DEBUG=n timeout 180 make debian 2>&1 | tee /tmp/debian.log && grep -q
  'debian login:' /tmp/debian.log` (180s escape per archLayout Failure
  Flow step 6).
- Difftest corpus (archLayout-04 green set) — zero new divergences vs
  QEMU/Spike.

Gate: all three green. Task lands.

## Trade-offs {ask reviewer for advice}

- **T-1** _Façade shape_:
  (a) **chosen** — free function `mount(bus, base, irq) -> usize` that
      returns `mtimer_idx`. Pro: zero new type, minimal surface, direct
      test of each sub-device. Con: caller must remember to hand the
      returned idx to `set_timer_source`.
  (b) struct `AclintHandle { mtimer_idx }` with an `install` method. Pro:
      harder to misuse (`handle.wire_timer(&mut bus)`). Con: new type for
      a 1-field wrapper, heavier than the problem warrants.
  (c) collapse into `Bus::add_aclint(base, irq)` — pro: one call, no idx
      juggling. Con: puts arch-specific knowledge in generic `Bus`,
      violates 01-M-004.

  Choosing (a): closest to existing call-site shape (`bus.add_mmio +
  bus.set_timer_source` pattern), no new types, keeps `Bus` arch-agnostic.
  Reviewer may prefer (b) for misuse-resistance.

- **T-2** _Region granularity_:
  (a) **chosen** — three regions `{0x4000, 0x8000, 0x4000}` summing to
      `0x1_0000`. Pro: spec-faithful, each sub-device owns a clean
      offset-0-local register space, easy future per-hart growth (just
      grow the MSWI / MTIMER region).
  (b) keep one `0x1_0000` region with an internal dispatcher struct that
      routes to three sub-devices by offset. Pro: fewer bus entries. Con:
      reintroduces the coupling we're removing; sub-devices no longer
      independently mountable.

  Choosing (a). (b) would make the "split" notional.

- **T-3** _NG-5 Bus-residual scope_:
  (a) rename `aclint_idx` → `mtimer_idx` only. **Chosen.** Aligns the
      name with what the field actually points at; preserves the
      `Bus::mtime` fast path.
  (b) also remove `Bus::mtime` and force CPU to dispatch through MMIO.
      Pro: eliminates the `Device::mtime` default method. Con: per-step
      overhead regression (measured in archLayout baseline ≈ ns/step on
      every `step_once`); unrelated to the split; belongs under
      `directIrq`.

  Choosing (a). Reviewer may argue (b) should at least be prototyped; we
  defer with explicit reasoning per NG-2.

## Validation {test design}

[**Unit Tests**]

- **V-UT-1** (inherited from `aclint.rs`) — MSWI: `msip_set_and_clear`
  asserts `irq.load() & MSIP` toggles via write at MSWI local offset 0;
  `Device::read` returns the set value.
- **V-UT-2** (inherited) — MTIMER: `mtime_advances_after_sync`,
  `mtime_frozen_between_syncs`, `mtimecmp_sets_mtip`,
  `mtimecmp_max_clears_mtip`, `mtime_write_ignored` — re-homed with
  MTIMER-local offsets (0x0 / 0x4 / 0x7FF8).
- **V-UT-3** (inherited) — SSWI: `setssip_is_edge_triggered`,
  `setssip_read_returns_zero`, `setssip_write_zero_no_effect` — re-homed
  with SSWI-local offset 0x0.
- **V-UT-4** (new) — `mswi_independent_of_mtimer`: construct `Mswi::new`
  alone, exercise MSIP, assert no dependency on an MTIMER instance.
- **V-UT-5** (new) — `mtimer_independent_of_sswi`: construct
  `Mtimer::new` alone, exercise mtimecmp/mtime, assert `Bus::take_ssip`
  is not required.
- **V-UT-6** (new) — `sswi_independent_of_mswi`: construct `Sswi::new`
  with a fresh `Arc<AtomicBool>`, drive edge, assert no `IrqState`
  dependency.

[**Integration Tests**]

- **V-IT-1** `cargo test --workspace` — 343 xcore lib + 1 arch_isolation
  + 6 xdb = 350 green at every phase boundary.
- **V-IT-2** `xcore/tests/arch_isolation.rs::arch_isolation` passes
  unchanged — new files live under `arch/riscv/device/intc/aclint/` which
  is already covered by the existing seam allowlists. If a new seam
  symbol (`Mswi`/`Mtimer`/`Sswi`/`mount`) leaks to `device/` top-level,
  the isolation test fails — we assert it doesn't.
- **V-IT-3** `make run` — default direct-boot reaches HIT GOOD TRAP.
- **V-IT-4** `timeout 60 make linux 2>&1 | tee /tmp/linux.log && grep -q
  'Welcome to Buildroot' /tmp/linux.log`.
- **V-IT-5** `DEBUG=n timeout 180 make debian 2>&1 | tee /tmp/debian.log
  && grep -q 'debian login:' /tmp/debian.log`.
- **V-IT-6** New integration test `aclint/mod.rs::mount_wires_all_three`
  — build a `Bus`, call `mount(&mut bus, 0x0200_0000, irq)`, perform
  each of the six guest-visible MMIO register operations (MSIP write,
  MSIP read-back, mtimecmp lo/hi write, mtime lo/hi read, setssip edge)
  at their global offsets and assert the old semantics hold. This is the
  byte-compat gate for C-1 / I-2.

[**Failure / Robustness Validation**]

- **V-F-1** `bus.mtime()` returns a nonzero value after ticking the
  bus past `SYNC_INTERVAL` — asserts the `mtimer_idx` wire-up and the
  `Device::mtime` fall-through on `Mtimer`. If `set_timer_source` were
  skipped, `Bus::mtime()` would return 0 and this fails.
- **V-F-2** `bus.take_ssip()` returns `true` exactly once after a guest
  write of 1 to `base+0xC000` — asserts the `Arc<AtomicBool>` shared
  between `Bus::ssip_pending` and `Sswi::ssip` is the same allocation.
  PRIMARY gate: `rg '\bssip_pending\b' xemu/xcore/src` still shows only
  `bus.rs` (the flag source) and `sswi.rs` (the consumer).
- **V-F-3** Per-phase bisection — each phase commit green under `make
  fmt && make clippy && cargo test --workspace` before the next phase.
- **V-F-4** Difftest corpus (archLayout-04 green set) — zero new
  divergences. If MTIMER wall-clock semantics shift (e.g. tick loop
  no longer calls `Mtimer::tick` every step), difftest catches it via
  `mtime` CSR divergence.
- **V-F-5** `cargo clippy --all-targets -- -D warnings` — no new
  warnings; `cargo fmt --check` — no new diffs.

[**Edge Case Validation**]

- **V-E-1** Unmapped offset within each sub-region returns 0 —
  sub-regional version of the existing `unmapped_offset_returns_zero`;
  one assertion per sub-device.
- **V-E-2** Writing to MTIMER's `mtime` offsets (local +0x7FF8 / +0x7FFC)
  is ignored; field stays at the last `sync_wallclock` value — inherited
  `mtime_write_ignored` shape.
- **V-E-3** `Bus::add_mmio` panics with "overlaps" if `mount` is called
  twice — property asserted by a `#[should_panic(expected = "overlaps")]`
  test in `aclint/mod.rs`.
- **V-E-4** `cargo build --no-default-features --features isa32` — RV32
  build compiles (inherited archLayout V-E-1).

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 three sub-devices | V-UT-4; V-UT-5; V-UT-6 |
| G-2 byte-identical MMIO | V-IT-6; V-F-4; V-IT-3 |
| G-3 call-site ergonomics | V-IT-6 (mount wires triple in one call) |
| G-4 tests re-homed + isolation | V-UT-1; V-UT-2; V-UT-3; V-UT-4; V-UT-5; V-UT-6 |
| G-5 mtimer_idx rename | V-IT-1 after Phase 2; diff review |
| C-1 guest MMIO compat | V-IT-6; V-F-4; V-IT-4; V-IT-5 |
| C-2 IrqState bit semantics | V-UT-1; V-UT-2; V-F-4 |
| C-3 no new Bus API | Diff review (bus.rs) |
| C-4 tests re-homed verbatim | V-UT-1; V-UT-2; V-UT-3; diff review |
| C-5 plan ≤ 300 lines | Body self-review |
| C-6 gate matrix | V-IT-1..V-IT-5; V-F-5 |
| C-7 no asm edits | Phase-1 diff review |
| I-1 independent sub-devices | V-UT-4; V-UT-5; V-UT-6 |
| I-2 external semantics | V-IT-6; V-F-4 |
| I-3 tick cadence | V-F-1; V-F-4 |
| I-4 ssip Arc identity | V-F-2 |
| I-5 hard_reset parity | V-UT-1..V-UT-3 (reset arms); V-IT-6 |
| I-6 no new unsafe / deps | V-F-5; Cargo.toml diff |
