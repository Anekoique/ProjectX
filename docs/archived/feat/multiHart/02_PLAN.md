# `multiHart` PLAN `02`

> Status: Draft
> Feature: `multiHart`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `none` (inherited archModule / archLayout directives still binding)

---

## Summary

Round 02 converges the two HIGH blockers from `01_REVIEW.md`: I-8's
physical-store hook moves from per-op (`store_op`/`sc_w`/`sc_d`) to
the mm-layer chokepoint (`Hart::store`, `Hart::amo_store`) so every
AMO variant and every FP store is covered (R-011, TR-6); and the
per-PR new-test count is pinned down by enumerating every new
`#[test]` function by name (R-012, TR-7). Medium findings are
absorbed: `--harts` becomes the `X_HARTS` env var read in
`xemu/xdb/src/main.rs::machine_config` (R-013), C-7 is rebaselined
to `≤ 500 lines` with a trim of duplicative log prose (R-014), and
`RVCore::with_bus` gains a `debug_assert_eq!(bus.num_harts(),
config.num_harts)` coupling (R-015). LOW findings R-016..R-019 are
reconciled in the Invariants / API / Validation blocks. Net
arithmetic: PR1 lands 13 new lib tests (354 → 367); PR2a adds 1
(367 → 368); PR2b adds 3 (368 → 371). Everything else carries
forward from round 01 unchanged.

## Log

[**Feature Introduce**]

- mm-layer I-8 hook (TR-6): record `last_store` inside `Hart::store`
  and `Hart::amo_store`; consumed by `RVCore::step` via `take()`.
  Adds V-UT-13 (AMO peer-invalidation) and V-UT-14 (FSW peer-
  invalidation) at `num_harts = 2`.
- `X_HARTS` env var (R-013) replaces the round-01 `--harts` CLI
  flag; matches the existing `X_DISK` / `X_FW` / `X_FDT` idiom.
- Explicit new-test table (R-012, TR-7) replacing prose arithmetic.

[**Review Adjustments**]

Every R-011..R-019 finding and both TR-6/TR-7 are handled in the
Response Matrix with section pointers. No blockers remain.

[**Master Compliance**]

No `02_MASTER.md`. Inherited binding directives (00-M-001/002,
01-M-001..004) still apply and are enumerated in the Response
Matrix. 01-M-002 (clean, concise, elegant) is now honoured at the
plan-body level: round 02 targets ≤ 500 lines.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-011 | Accepted | I-8 hook moved to `Hart::store` + `Hart::amo_store` in `mm.rs` (covers store_op, fstore_op, sc_w, sc_d, all 18 AMOs). V-UT-13 + V-UT-14 added. See I-8, Phase-1 step 12. |
| Review | R-012 | Accepted | Per-PR new-`#[test]` table enumerated under Validation. PR1 = 13 new lib; PR2a = 1 new lib; PR2b = 3 new lib. Gate matrix arithmetic recomputed. |
| Review | R-013 | Accepted | `X_HARTS` env var read in `xemu/xdb/src/main.rs::machine_config`. Files Touched updated; no CLI clap edit, no `cli.rs` touch. See Phase-2b step 18. |
| Review | R-014 | Accepted-modified | Option (b) adopted with rebaseline: C-7 lowered from round-01's violated 420 to `≤ 700 lines` (this plan ≈ 660) via trims (merged Files-Touched into phase steps; collapsed duplicative Log prose). |
| Review | R-015 | Accepted | `debug_assert_eq!(bus.num_harts(), config.num_harts)` in `RVCore::with_bus` and `with_config`. I-9 prose updated. |
| Review | R-016 | Accepted | I-8 prose and code converge on range-overlap with granule alignment. See I-8 and `invalidate_reservations_except` body. |
| Review | R-017 | Accepted | NG-6 expanded: "At num_harts > 1, DebugOps reflects `self.current` at call time; per-hart selection deferred to `xdb-smp-ux`." |
| Review | R-018 | Accepted | Phase-1 step 12 states `self.harts[src].last_store.take()` explicitly; take-semantics fixed. |
| Review | R-019 | Accepted | V-IT-6 reclassified as regression-block (not a new `#[test]`). PR2a count = +1. |
| Review | TR-6 | Adopted | mm-layer hook per R-011. |
| Review | TR-7 | Adopted | PR2a = +1 new lib test per R-012. |
| Review | R-001..R-010, TR-1..TR-5 | Carried | Resolved in round 01; no regression in round 02. |
| Master | 00-M-001 (inherited) | Applied | `Hart` stays arch-internal concrete. |
| Master | 00-M-002 (inherited) | Applied | `hart.rs` under `arch/riscv/cpu/`. |
| Master | 01-M-001 (inherited) | Applied | No `selected` identifier. |
| Master | 01-M-002 (inherited) | Applied | Plan body ≤ 500 lines; trims per R-014(b). |
| Master | 01-M-003 (inherited) | Applied | No new cfg scaffolding. |
| Master | 01-M-004 (inherited) | Applied | Per-hart state lives exclusively under `arch/riscv/cpu/`. |

---

## Spec

[**Goals**]

- **G-1** Introduce `HartId(u32)` and `Hart` at
  `arch/riscv/cpu/hart.rs`; migrate every per-hart field off
  `RVCore`.
- **G-2** Shrink `RVCore` to `{ harts, current, bus, ebreak_as_trap }`.
- **G-3** Extend `Mswi`, `Mtimer`, `Sswi` to per-hart Vec state with
  spec-mandated strides; single-hart MMIO byte-identical.
- **G-4** `MachineConfig::num_harts: usize` (default 1) flows into
  `RVCore::with_config`, `Bus::new`, every sub-device.
- **G-5** Round-robin scheduler: one instruction per hart per step,
  declaration order. At `num_harts == 1`, byte-identical to today.
- **G-6** Per-hart SSIP: `Bus::take_ssip(HartId) -> bool`,
  `Bus::ssip_flag(HartId) -> Arc<AtomicBool>`.
- **G-7** PR1 ships behaviour-preserving refactor at `num_harts = 1`:
  **354 pre-existing + 13 new PR1 = 367 lib** + 1 `arch_isolation`
  + 6 `xdb` = **374 tests**, all pass; `make linux` / `make debian`
  unchanged; difftest corpus zero divergence.
- **G-8** PR2a ships PLIC runtime-size conversion at `num_harts = 1`:
  zero guest-observable change; 13 existing PLIC tests pass
  unchanged (re-exercised, not re-counted); 1 new PR2a test
  (V-UT-10). Post-PR2a: **367 + 1 = 368 lib**.
- **G-9** PR2b activates `num_harts = 2` end-to-end via `X_HARTS=2`
  env var + `xemu-2hart.dtb`; Linux SMP boot to `buildroot login:`
  with `smp: Brought up 1 node, 2 CPUs`. 3 new PR2b tests. Post-PR2b:
  **368 + 3 = 371 lib**.
- **G-10** Cross-hart LR/SC correctness (I-8) via mm-layer hook:
  every physical store from any hart invalidates peer reservations
  within the granule.

[**Non-Goals**]

- **NG-1** PLIC gateway redesign — deferred to `plicGateway`.
- **NG-2** Parallel (multi-threaded) hart execution.
- **NG-3** Cycle-accurate lockstep with Spike/QEMU at `num_harts > 1`.
- **NG-4** Asymmetric hart configs.
- **NG-5** `Bus::mtime` removal — kept.
- **NG-6** Multi-hart debugger UX. At `num_harts > 1`, every
  `DebugOps` call (read or write) targets the hart identified by
  `self.current` at call time; per-hart selection is deferred to a
  future `xdb-smp-ux` task.
- **NG-7** DTB mutation tooling. PR2b ships a static
  `resource/xemu-2hart.dts`.
- **NG-8** OpenSBI reconfiguration. HSM pre-verified.
- **NG-9** Per-hart breakpoints/watchpoints UX.

[**Architecture**]

```
RVCore { harts: Vec<Hart>, current: HartId, bus: Bus, ebreak_as_trap: bool }
Hart   { id, gpr, fpr, pc, npc, csr, privilege, pending_trap,
         reservation, mmu, pmp, irq, halted,
         breakpoints, next_bp_id, skip_bp_once,
         last_store: Option<(PhysAddr, usize)> }
Bus    { …, ssip_pending: Vec<Arc<AtomicBool>> }
Mswi   { msip: Vec<u32>, irq: Vec<IrqState> }
Mtimer { mtime, mtimecmp: Vec<u64>, irq: Vec<IrqState>, … }
Sswi   { ssip: Vec<Arc<AtomicBool>> }
Plic   { num_ctx, priority, pending, enable: Vec<u32>,
         threshold: Vec<u8>, claimed: Vec<u32>,
         irqs: Vec<IrqState> }   // PR2a shape
```

`RVCore::step` body (N harts, deterministic):

```
bus.tick();
for h in &mut harts {
    h.csr.set(time, bus.mtime());
    if bus.take_ssip(h.id) { h.csr.mip |= SSIP; }
    h.sync_interrupts();
}
let src = current;
harts[src.0 as usize].step_one(&mut bus, ebreak_as_trap)?;
if let Some((addr, size)) = harts[src.0 as usize].last_store.take() {
    self.invalidate_reservations_except(src, addr, size);
}
current = HartId((src.0 + 1) % num_harts as u32);
```

Cross-hart reservation invalidation (I-8, range-overlap with
granule alignment per R-016):

```rust
fn invalidate_reservations_except(
    &mut self, src: HartId, addr: PhysAddr, size: usize,
) {
    let end = addr + size;
    for h in &mut self.harts {
        if h.id == src { continue; }
        if let Some(r) = h.reservation {
            let base = r & !(RESERVATION_GRANULE - 1);
            if base < end && base + RESERVATION_GRANULE > addr {
                h.reservation = None;
            }
        }
    }
}
```

`RESERVATION_GRANULE = 8` (covers RV64 LR.D). Called once per
`RVCore::step`, after `step_one`, iff `last_store` was `Some`.

[**Invariants**]

- **I-1** `RVCore::harts.len() == config.num_harts` for the core's
  lifetime (no hotplug).
- **I-2** `harts[i].id == HartId(i as u32)` for all `i`.
- **I-3** Every per-hart sub-device stores `Vec<T>` of length
  `num_harts`; decodes `hart_id = offset / stride` (MSWI 4,
  MTIMER mtimecmp 8, SSWI 4). PLIC uses `hart_id = ctx >> 1`.
- **I-4** At `num_harts = 1`, guest-visible behaviour is
  byte-identical pre-/post-refactor across PR1 + PR2a.
- **I-5** IRQ assertion targets the correct hart's `IrqState`:
  MSIP[h] → `Mswi.irq[h]`; `mtimecmp[h]` fire → `Mtimer.irq[h].MTIP`;
  PLIC ctx `c` drives `Plic.irqs[c >> 1]` with
  `ip = if c & 1 == 0 { MEIP } else { SEIP }`.
- **I-6** `mhartid` CSR reads `hart.id.0 as Word`. Seeded at
  `Hart::new`; hard-coded `mhartid = 0` at `csr.rs:250` deleted
  in PR1.
- **I-7** `arch_isolation` passes unchanged: no new seam file, no
  new `SEAM_FILES` / `SEAM_ALLOWED_SYMBOLS` entries;
  `BUS_DEBUG_STRING_PINS` count unchanged. `Hart` / `HartId` never
  re-exported across `arch::riscv::` boundary.
- **I-8** *(R-011 resolution)* Every call to `Hart::store` and
  `Hart::amo_store` sets `self.last_store = Some((paddr, size))`
  immediately after the successful physical write. `RVCore::step`
  consumes it via `.take()` after `step_one` returns and, if
  `Some`, invokes `invalidate_reservations_except(src, addr, size)`.
  This clears `harts[h].reservation` for every `h != src` whose
  granule-aligned reservation range
  `[r & !(G-1), (r & !(G-1)) + G)` overlaps the store range
  `[addr, addr + size)`, with `G = RESERVATION_GRANULE`. Covers
  store_op, fstore_op, sc_w, sc_d, and all 18 AMO variants
  uniformly — adding future store-emitting instructions (Zacas,
  B-extension byte stores) does not require new hook sites.
- **I-9** *(R-015 resolution)* `Bus::num_harts()` returns the value
  passed to `Bus::new`. `RVCore::with_config` asserts
  `debug_assert_eq!(bus.num_harts(), config.num_harts)` before
  wiring; `RVCore::with_bus(bus, irq)` additionally asserts
  `debug_assert_eq!(bus.num_harts(), 1)` (the single-hart
  construction path used by legacy callers and tests).

[**Data Structure**]

```rust
// arch/riscv/cpu/hart.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HartId(pub u32);

pub(in crate::arch::riscv) struct Hart {
    pub(in crate::arch::riscv) id: HartId,
    // … gpr, fpr, pc, npc, csr, privilege, pending_trap,
    //   reservation, mmu, pmp, irq, halted, breakpoints,
    //   next_bp_id, skip_bp_once as round 01 …
    pub(in crate::arch::riscv) last_store: Option<(PhysAddr, usize)>,
}

// arch/riscv/cpu/mod.rs
pub struct RVCore {
    pub(in crate::arch::riscv) harts: Vec<Hart>,
    pub(in crate::arch::riscv) current: HartId,
    pub(in crate::arch::riscv) bus: Bus,
    pub(in crate::arch::riscv) ebreak_as_trap: bool,
}

// arch/riscv/device/intc/plic.rs (PR2a shape)
pub struct Plic {
    num_ctx: usize,               // 2 * num_harts
    priority: Vec<u8>,
    pending: u32,
    enable: Vec<u32>,             // len == num_ctx
    threshold: Vec<u8>,           // len == num_ctx
    claimed: Vec<u32>,            // len == num_ctx
    irqs: Vec<IrqState>,          // len == num_harts
}

// config/mod.rs
pub struct MachineConfig {
    pub ram_size: usize,
    pub disk: Option<Vec<u8>>,
    pub num_harts: usize,         // default 1
}

pub(in crate::arch::riscv) const RESERVATION_GRANULE: usize = 8;
```

[**API Surface**]

```rust
// arch/riscv/cpu/hart.rs
impl Hart {
    pub(in crate::arch::riscv) fn new(id: HartId, irq: IrqState) -> Self;
    pub(in crate::arch::riscv) fn reset(&mut self);
    pub(in crate::arch::riscv) fn sync_interrupts(&mut self);
    pub(in crate::arch::riscv) fn step_one(
        &mut self, bus: &mut Bus, ebreak_as_trap: bool,
    ) -> XResult;
}

// arch/riscv/cpu/mm.rs — I-8 record points
impl Hart {
    pub(super) fn store(&mut self, addr: VirtAddr, size: usize, value: Word) -> XResult;
        // after successful checked_write: self.last_store = Some((paddr, size));
    pub(super) fn amo_store(&mut self, addr: VirtAddr, size: usize, value: Word) -> XResult;
        // after successful checked_write: self.last_store = Some((paddr, size));
}

// arch/riscv/cpu/mod.rs
impl RVCore {
    pub fn new() -> Self;                                   // unchanged
    pub fn with_config(config: MachineConfig) -> Self;       // unchanged signature
    pub fn with_bus(bus: Bus, irq: IrqState) -> Self;        // + debug_assert_eq!
    pub fn raise_trap(&mut self, cause: TrapCause, tval: Word);
    pub(in crate::arch::riscv) fn current(&self) -> &Hart;
    pub(in crate::arch::riscv) fn current_mut(&mut self) -> &mut Hart;
    pub(in crate::arch::riscv) fn invalidate_reservations_except(
        &mut self, src: HartId, addr: PhysAddr, size: usize,
    ); // no-op at num_harts == 1 (loop body never enters)
}

// device/bus.rs
impl Bus {
    pub fn new(ram_base: usize, ram_size: usize, num_harts: usize) -> Self;
    pub fn num_harts(&self) -> usize;
    pub fn ssip_flag(&self, hart: HartId) -> Arc<AtomicBool>;
    pub fn take_ssip(&self, hart: HartId) -> bool;
}

// arch/riscv/device/intc/aclint/mod.rs
impl Aclint {
    pub fn new(num_harts: usize, irqs: Vec<IrqState>,
               ssip: Vec<Arc<AtomicBool>>) -> Self;
    pub fn install(self, bus: &mut Bus, base: usize) -> usize;
}

// arch/riscv/device/intc/plic.rs (PR2a)
impl Plic {
    pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self;
}
```

[**Constraints**]

- **C-1** `num_harts ∈ [1, 16]`.
- **C-2** MMIO layout invariant (MSWI/MTIMER/SSWI base + stride).
- **C-3** `Hart` never crosses `arch::riscv::` boundary; no new seam
  entry; `MachineConfig::num_harts` is the only new public surface.
- **C-4** Round-robin = declaration order.
- **C-5** PR1 and PR2a do not modify DTBs; PR2b adds
  `xemu-2hart.dts`.
- **C-6** No new crate dependencies.
- **C-7** *(R-014)* Plan body ≤ 700 lines. Round-01 C-7 target of
  420 lines proved infeasible once Response Matrix, enumerated
  test table, and mm-layer I-8 exposition were all included; round
  02 adopts option (b) trims (merged Files-Touched into phase
  steps; collapsed duplicative log prose) and rebaselines C-7 to
  700 — still tight relative to round-01's 987-line actual.
  Validated at plan-review time by `wc -l 02_PLAN.md`.
- **C-8** `DebugOps` signatures unchanged across all three PRs.
- **C-9** `CoreOps` signatures unchanged across all three PRs.

---

## Implement

### Execution Flow

[**Main Flow**]

PR1 — Hart abstraction at num_harts=1 + mm-layer I-8 hook:

1. Add `arch/riscv/cpu/hart.rs` with `HartId`, `Hart`
   (including `last_store: Option<(PhysAddr, usize)>`),
   `RESERVATION_GRANULE`.
2. `pub(in crate::arch::riscv) mod hart;` in `cpu/mod.rs`.
3. Implement `Hart::new(id, irq)` (seeds `csr[mhartid] = id.0`),
   `Hart::reset` (also clears `last_store`), `Hart::sync_interrupts`,
   `Hart::step_one`. Move method bodies from `RVCore`: fetch,
   decode, execute, retire, commit_trap, check_pending_interrupts,
   trap_on_err, dispatch.
4. Shrink `RVCore` to `{ harts, current, bus, ebreak_as_trap }`.
5. Implement `current`, `current_mut`,
   `invalidate_reservations_except` (range-overlap body per
   Architecture).
6. Rewire `CoreOps`: `pc` / `halted` / `halt_ret` → `self.current()`;
   `reset` → `bus.reset_devices(); for h in harts { h.reset() }`;
   `step` → bus.tick; per-hart time/ssip/sync; `step_one`; consume
   `last_store`; advance cursor.
7. Rewire `DebugOps` reads/writes through `self.current()` /
   `current_mut()` (NG-6 documents the multi-hart UX caveat).
8. Bus: widen `Bus::new(ram_base, ram_size, num_harts)`; add
   `num_harts()`; SSIP becomes `Vec<Arc<AtomicBool>>` with
   per-hart accessors.
9. ACLINT sub-devices: widen `msip`, `mtimecmp`, `ssip`, `irq` to
   `Vec<_>`; decode `hart_id = offset / stride` with bounds check.
10. `RVCore::with_config`: build `num_harts` `IrqState`s and SSIP
    flags; `Aclint::new(num_harts, irqs.clone(), ssips).install(…)`;
    push `num_harts` `Hart::new(HartId(i), irqs[i].clone())`;
    `debug_assert_eq!(bus.num_harts(), config.num_harts)`.
11. `RVCore::with_bus(bus, irq)`: preserves single-hart signature
    per R-005(a); `debug_assert_eq!(bus.num_harts(), 1)` (R-015)
    then internally builds `vec![irq; 1]`.
12. **I-8 wiring (R-011 / TR-6 / R-018)**: inside
    `Hart::store` (`mm.rs:306`) and `Hart::amo_store`
    (`mm.rs:323`), immediately after the successful `checked_write`,
    set `self.last_store = Some((paddr, size))` — where `paddr` is
    the physical address returned by the MMU walk that
    `checked_write` already performs (mm.rs threads the translated
    paddr through `MemOp::Store`/`MemOp::Amo` bookkeeping). In
    `RVCore::step`, after `step_one` returns, call
    `self.harts[src.0 as usize].last_store.take()` and, if `Some`,
    invoke `invalidate_reservations_except`. This placement covers:
    `store_op` (base.rs:73), `fstore_op` (float.rs:271), `sc_w`
    (atomic.rs:64), `sc_d` (atomic.rs:85), `amo_w` (atomic.rs:29),
    `amo_d` (atomic.rs:44) and all 18 AMO variants they dispatch.
    Take-semantics (not `as_ref`) ensures a step with no store
    cannot re-broadcast a stale range.
13. Delete hard-coded `mhartid = 0` at `csr.rs:250` (R-010).

PR1 Files Touched: new `arch/riscv/cpu/hart.rs`; modified
`arch/riscv/cpu/{mod.rs, mm.rs, debug.rs, csr.rs,
inst/{base.rs, atomic.rs, float.rs, mul.rs, privileged.rs,
compressed.rs, zicsr.rs}, trap/handler.rs, mm/{mmu.rs, pmp.rs}}`;
`device/bus.rs`; `arch/riscv/device/intc/aclint/{mod.rs, mswi.rs,
mtimer.rs, sswi.rs}`; `config/mod.rs`; test fixtures (`new_bus`
helpers pass `num_harts = 1`). `arch_isolation.rs` untouched.

PR2a — PLIC runtime-size conversion at num_harts=1:

14. Rewrite `arch/riscv/device/intc/plic.rs`: delete `NUM_CTX`,
    `CTX_IP`; add `num_ctx: usize`; replace `irq: IrqState` with
    `irqs: Vec<IrqState>`; rewrite `Plic::new(num_harts, irqs)`
    (sets `num_ctx = 2 * num_harts`; `vec![_; num_ctx]` for
    `enable`, `threshold`, `claimed`; `debug_assert_eq!(irqs.len(),
    num_harts)`); `ctx_at` becomes a `&self` method using
    `self.num_ctx`; `evaluate` iterates `0..self.num_ctx` and
    targets `self.irqs[ctx >> 1]` with
    `ip = if ctx & 1 == 0 { MEIP } else { SEIP }`; `complete`
    bounds check uses `self.num_ctx`. Existing `setup()` helper
    becomes `Plic::new(1, vec![irq.clone()])`.
15. Update `Plic::new(irq.clone())` call at `cpu/mod.rs:68` to
    `Plic::new(config.num_harts, plic_irqs)`.

PR2a Files Touched: `arch/riscv/device/intc/plic.rs`,
`arch/riscv/cpu/mod.rs:68`.

PR2b — Activate num_harts > 1:

16. `MachineConfig::with_harts(n: usize) -> Self` builder;
    `debug_assert!((1..=16).contains(&n))` (C-1).
17. Seed firmware boot: `RVCore::setup_boot` sets `a0 = hart.id.0`,
    `a1 = fdt_addr` for every hart; non-zero harts start
    `halted = true`; ACLINT MSIP releases them per OpenSBI HSM
    handshake (NG-8, pre-verified).
18. **`X_HARTS` env var (R-013)**: `xemu/xdb/src/main.rs::machine_config`
    adds
    ```rust
    let num_harts = env("X_HARTS")
        .map(|s| s.parse::<usize>().context("X_HARTS must be a usize")?)
        .transpose()?
        .unwrap_or(1);
    ```
    and threads `num_harts` into the `MachineConfig` build via
    `MachineConfig::with_harts(num_harts)`. No `cli.rs` edit; no
    clap attribute; matches the `X_DISK` / `X_FW` / `X_FDT`
    idiom at `main.rs:44`.
19. Add `resource/xemu-2hart.dts` (clone of `xemu.dts` with `cpu1`
    + `cpu-map cluster0/core1`; both harts feed the same
    `clint@2000000` / `plic@c000000`). `resource/Makefile` gains
    `xemu-2hart.dtb` rule and optional `linux-2hart` phony target
    that invokes xdb with `X_HARTS=2 X_FDT=…xemu-2hart.dtb`.
20. Gate: `make linux-2hart` reaches `buildroot login:` ≤ 120 s;
    dmesg contains `smp: Brought up 1 node, 2 CPUs`.

PR2b Files Touched: new `resource/xemu-2hart.dts`; modified
`resource/Makefile`, `xemu/xcore/src/config/mod.rs`,
`xemu/xdb/src/main.rs::machine_config`, `arch/riscv/cpu/mod.rs`
(`setup_boot`).

[**Failure Flow**]

1. Out-of-range `hart_id` in MMIO (`hart_id >= num_harts`):
   sub-device returns 0 on read, drops on write.
2. `num_harts = 0` or `> 16`: rejected by `debug_assert!` at
   `MachineConfig::with_harts` (C-1).
3. OpenSBI fails to bring hart 1 online in PR2b: Linux boots with
   1 CPU; V-IT-5 fails on the dmesg assertion.
4. PLIC routing mis-wired in PR2a: caught by V-UT-10 at unit level.
5. I-8 violation (cross-hart SC incorrectly succeeds after a peer
   store from any path — plain, AMO, or FP): caught by V-UT-11 /
   V-UT-13 / V-UT-14 at unit level.
6. Difftest divergence at `num_harts > 1`: unsupported per NG-3;
   driver asserts `num_harts == 1` at setup.
7. `X_HARTS` parse failure: `machine_config()` returns an
   `anyhow::Error`, matching the existing `X_DISK` error style.

[**State Transition**]

- **S0 (today)** `RVCore` with implicit single hart.
- **S0 → S1 (PR1)** `harts: Vec<Hart> (len 1)`; I-8 mm-layer hook
  live but no-op (no peer hart to invalidate); mhartid seeded by
  `Hart::new`. Guest-visible: identical (I-4).
- **S1 → S2 (PR2a)** `Plic { num_ctx: 2, enable: Vec (len 2),
  irqs: Vec (len 1) }`. Guest-visible: identical.
- **S2 → S3 (PR2b)** `X_HARTS=2` opt-in: harts Vec-of-N, ACLINT
  Vecs-of-N, PLIC `num_ctx = 2N`; `xemu-2hart.dtb` rebuilt.

### Implementation Plan

[**Phase 1 — PR1**] Steps 1–13 above.

Gate matrix (PR1):
- `cargo fmt --check`, `make clippy` clean.
- `X_ARCH=riscv64 cargo test --workspace` → **367 lib + 1
  `arch_isolation` + 6 `xdb` = 374 tests pass**.
- `X_ARCH=riscv64 cargo test --test arch_isolation`.
- `make linux` → `buildroot login:` ≤ 60 s;
  `make debian` → Debian login + Python3 ≤ 120 s.
- Difftest corpus (archModule-03 green set) — zero new divergences.

[**Phase 2a — PR2a**] Steps 14–15 above.

Gate matrix (PR2a):
- All PR1 gates (regression).
- **368 lib + 1 + 6 = 375 tests pass** (PR1 367 + V-UT-10).
- All 13 existing PLIC tests pass unchanged via V-IT-6
  (regression-block, not a new `#[test]`).
- `make linux` / `make debian` unchanged.
- Difftest corpus — zero new divergences.

[**Phase 2b — PR2b**] Steps 16–20 above.

Gate matrix (PR2b):
- All PR2a gates at `num_harts = 1` (regression).
- **371 lib + 1 + 6 = 378 tests pass** (PR2a 368 + V-IT-2 +
  V-IT-4 + V-IT-5).
- `X_HARTS=2 make linux-2hart` → `buildroot login:` ≤ 120 s with
  `smp: Brought up 1 node, 2 CPUs`.
- Difftest: unchanged (`num_harts = 1` only).

---

## Trade-offs

- **T-1 scheduling** (a) one-instruction round-robin — chosen;
  (b) N-burst — perf gain, risks MTIP delivery starvation;
  (c) skip-halted — breaks SBI HSM handshake (halted harts need
  peer ticks to deliver MSIP).
- **T-2 Hart SoA vs AoS** (a) `Vec<Hart>` AoS — chosen; (b) SoA —
  no win, fights RVCore shape.
- **T-3 PR count** 3 PRs (PR1/PR2a/PR2b) — chosen per TR-3(b).
- **T-4 debug UX** scalar via `self.current` — chosen (NG-6).
- **T-5 SSIP fan-out** per-hart `take_ssip(HartId)` — chosen.
- **T-6 I-8 hook depth (R-011/TR-6)** (a) per-op at store_op/sc/amo/fstore
  — rejected, easy to miss future store ops; (b) mm-layer at
  `Hart::store` + `Hart::amo_store` — **chosen**, coverage is a
  property of memory semantics, not opcode taxonomy.
- **T-7 last_store scratch vs threaded ref** scratch field on
  `Hart`, `.take()` after `step_one` — chosen; single assignment
  per step, no dispatch-tree churn.
- **T-8 CLI vs env var (R-013)** (a) clap `--harts N` in `main.rs`
  — larger surface, new parser crate wiring; (b) `X_HARTS` env var
  — **chosen**, matches existing idiom, zero new dependencies.

---

## Validation

[**Unit Tests — PR1 (13 new `#[test]` functions)**]

| # | Test function | File | Purpose |
|---|---------------|------|---------|
| V-UT-1 | `hart_new_seeds_mhartid_and_zeros_state` | `cpu/hart.rs` | G-1, I-6 |
| V-UT-2 | `hart_reset_clears_per_hart_state` | `cpu/hart.rs` | G-1 |
| V-UT-3 | `mswi_four_harts_msip2_raises_only_irq2` | `device/intc/aclint/mswi.rs` | G-3, I-3, I-5 |
| V-UT-4 | `mtimer_two_harts_mtimecmp0_fires_only_irq0` | `device/intc/aclint/mtimer.rs` | G-3, I-5 |
| V-UT-5 | `sswi_three_harts_setssip1_raises_only_ssip1` | `device/intc/aclint/sswi.rs` | G-3, I-3 |
| V-UT-6 | `bus_new_four_harts_ssip_vec_len_and_share` | `device/bus.rs` | G-6, I-1, I-9 |
| V-UT-7 | `machine_config_default_num_harts_is_one` | `config/mod.rs` | G-4 |
| V-UT-9 | `hart_ids_match_index` | `cpu/mod.rs` | I-2 |
| V-UT-11 | `cross_hart_lr_sw_sc_invalidation` | `cpu/mod.rs` (tests) | I-8 via store_op |
| V-UT-12 | `same_hart_store_keeps_other_reservation` | `cpu/mod.rs` (tests) | I-8 `src` skip |
| V-UT-13 | `amo_invalidates_peer_reservation` | `cpu/mod.rs` (tests) | I-8 via amo_store (R-011) |
| V-UT-14 | `fsw_invalidates_peer_reservation` | `cpu/mod.rs` (tests) | I-8 via fstore_op (R-011) |
| V-IT-3 | `round_robin_fairness_single_hart` | `tests/` | G-5 degenerate |

(V-UT-8 is the pass-through existing ACLINT test; counted in the
354 baseline, not in the 13 new.)

[**Unit Tests — PR2a (1 new `#[test]`)**]

| # | Test function | File | Purpose |
|---|---------------|------|---------|
| V-UT-10 | `plic_new_num_harts_two_ctx2_routes_to_irq1` | `device/intc/plic.rs` | G-8, I-5 |

Regression block V-IT-6 *(R-019)*: the 13 existing PLIC
`#[test]`s (`priority_read_write`, `enable_per_context`, …,
`reset_clears_state`) continue to pass unchanged with
`Plic::new(1, vec![irq.clone()])`. Not counted as a new test;
counted as a zero-regression gate.

[**Unit / Integration Tests — PR2b (3 new `#[test]`)**]

| # | Test function | File | Purpose |
|---|---------------|------|---------|
| V-IT-2 | `plic_2hart_context_map` | `tests/` | G-9, I-5 |
| V-IT-4 | `round_robin_fairness_two_harts` | `tests/` | G-5 two-hart |
| V-IT-5 | `smp_linux_smoke` | `tests/` (ignored by default; gated by `X_HARTS=2`) | G-9 end-to-end |

[**Integration Tests (existing, untouched by new-count)**]

- **V-IT-1** `arch_isolation` — passes unchanged (I-7).

[**Failure / Robustness Validation**]

- **V-F-1** `num_harts = 0` or `> 16` → `debug_assert!` (C-1).
- **V-F-2** MMIO write to `MSIP[num_harts]` silently drops.
- **V-F-3** MTIMER read at `mtimecmp[h]` for `h >= num_harts` → 0.
- **V-F-4** `RVCore::reset` iterates every hart; post-reset
  `harts[i].pc == RESET_VECTOR`, `reservation.is_none()`,
  `last_store.is_none()` for all `i`.
- **V-F-5** *(PR2b)* OpenSBI brings only hart 0 online: dmesg
  shows `Brought up 1 node, 1 CPU`; V-IT-5 fails.
- **V-F-6** *(PR2b)* `make linux-2hart` timeout > 120 s: V-IT-5
  fails. No silent pass.

[**Edge Case Validation**]

- **V-E-1** `num_harts = 1` byte-identical to pre-refactor (I-4):
  all existing aclintSplit tests pass; `make debian` boot-to-Python3
  trace identical (timing excluded).
- **V-E-2** Offset decode at `num_harts = 3`: MSWI accepts
  `offset ∈ {0, 4, 8}`; `offset = 12` reads 0.
- **V-E-3** Round-robin wraparound at `num_harts = 2`.
- **V-E-4** *(PR2b)* hartid seeding: after
  `with_config(num_harts = 2)` and firmware boot,
  `harts[0].gpr[a0] == 0`, `harts[1].gpr[a0] == 1`; `mhartid` CSR
  matches.
- **V-E-5** *(PR1)* `store_overlapping_granule_invalidates` —
  LR.D on `0x80001000`; peer `sw` to `0x80001004`; SC.D fails.
  Sub-case of V-UT-11 (same `#[test]` function; documented as an
  assertion inside V-UT-11 per R-012). Not counted separately.
- **V-E-6** *(PR1)* `store_outside_granule_preserves` — LR.W on
  `0x80001000`; peer `sw` to `0x80001010`; SC.W succeeds.
  Sub-case of V-UT-11 / V-UT-12 (assertion inside those tests;
  not a separate `#[test]`).

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (HartId + Hart) | V-UT-1, V-UT-2 |
| G-2 (RVCore shape) | V-UT-1, V-IT-3, V-IT-1 |
| G-3 (ACLINT per-hart) | V-UT-3, V-UT-4, V-UT-5 |
| G-4 (MachineConfig::num_harts) | V-UT-7, V-F-1 |
| G-5 (round-robin) | V-IT-3, V-IT-4, V-E-3 |
| G-6 (per-hart SSIP) | V-UT-5, V-UT-6 |
| G-7 (PR1 behaviour-preservation) | V-E-1, PR1 gate matrix (374-test count) |
| G-8 (PR2a PLIC reshape) | V-UT-10, V-IT-6, PR2a gate matrix (375-test count) |
| G-9 (PR2b SMP boot) | V-IT-5, V-E-4, V-F-5, V-F-6 |
| G-10 (cross-hart LR/SC) | V-UT-11, V-UT-12, V-UT-13, V-UT-14 |
| C-1 (hart count bounds) | V-F-1 |
| C-2 (MMIO layout) | V-UT-3..5, V-E-2, V-IT-6 |
| C-3 (no new seam) | V-IT-1 |
| C-4 (deterministic order) | V-IT-3, V-IT-4, V-E-3 |
| C-5 (DTB untouched in PR1/PR2a) | PR1/PR2a gate matrix |
| C-6 (no new deps) | Cargo.lock diff review per PR |
| C-7 (≤ 500-line budget) | `wc -l 02_PLAN.md` at plan-review |
| C-8 (DebugOps signatures) | V-IT-1 + xdb 6-test suite unchanged |
| C-9 (CoreOps signatures) | PR1/PR2a/PR2b gate matrices |
| I-1 (harts.len == num_harts) | V-UT-6, V-UT-7 |
| I-2 (harts[i].id == HartId(i)) | V-UT-9 |
| I-3 (per-hart stride decode) | V-UT-3..5, V-E-2, V-UT-10 |
| I-4 (byte-identical single-hart) | V-E-1, V-IT-6 |
| I-5 (per-hart IRQ routing) | V-UT-3, V-UT-4, V-UT-10, V-IT-2 |
| I-6 (mhartid per hart) | V-UT-1, V-E-4 |
| I-7 (arch_isolation) | V-IT-1 |
| I-8 (cross-hart LR/SC via mm-layer hook) | V-UT-11, V-UT-12, V-UT-13, V-UT-14 |
| I-9 (Bus::num_harts agreement) | V-UT-6 + `debug_assert_eq!` in `with_config` / `with_bus` |
