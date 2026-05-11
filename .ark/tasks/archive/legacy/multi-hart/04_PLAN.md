# `multiHart` PLAN `04`

> Status: Draft
> Feature: `multiHart`
> Iteration: `04`
> Owner: Executor
> Depends on:
> - Previous Plan: `03_PLAN.md`
> - Review: `03_REVIEW.md` (APPROVED, pre-pivot)
> - Master Directive: `none` (inherited directives binding; user-issued architecture pivot described below)

---

## Summary

Round 04 is a **user-ordered architecture pivot** after round 03 closed
at APPROVE. Discard the `Hart` struct: `RVCore` already holds
exclusively per-hart state (GPR, FPR, PC, NPC, CSR, MMU, PMP,
reservation, IrqState, pending_trap, halted) — it *is* a hart. Pivoted
shape: `RVCore` gains `id: HartId` + `last_store`, loses `bus: Bus`;
`CPU<Core>` gains `cores: Vec<Core>` + `bus: Bus` + `current: usize`;
round-robin lives in `CPU::step`. `CoreOps::step` becomes `fn step(
&mut self, bus: &mut Bus) -> XResult`; `CoreOps::{bus, bus_mut}` are
removed. Every prior finding (R-001..R-024, TR-3/6/7/8, R-025(a))
carries — only the *owning type* changes from `Hart` to `RVCore` and
the *round-robin site* from `RVCore::step` to `CPU::step`. PR1
byte-identical at `num_harts == 1`; PR2a PLIC runtime-size; PR2b
activates `num_harts > 1` via `X_HARTS`. Tests rebase: PR1 354 + 11 =
365 lib; PR2a 366; PR2b 369 (V-UT-1 / V-UT-2 fold into existing
`RVCore::new` / `reset` tests).

## Log

[**Feature Introduce**] Architecture pivot — `Hart` struct removed;
`RVCore` is the hart. `CPU<Core>` owns `Vec<Core>` + `Bus`; round-robin
in `CPU::step`. `CoreOps::step(&mut self, bus: &mut Bus)`. Bus threaded
into 8 methods in `arch/riscv/cpu/mm.rs`; R-020 hook moves from
`Hart::checked_write` to `RVCore::checked_write` (same file `mm.rs:271`);
invalidation helper moves from `RVCore` to `CPU`.

[**Review Adjustments**] Round 03 had no CRITICAL/HIGH/MEDIUM; sole LOW
R-025 resolved by option (a): C-7 relaxed `≤ 700` → `≤ 720`. All
round-00..03 decisions carry.

[**Master Compliance**] No `04_MASTER.md`. Inherited 00-M-001/002,
01-M-001..004 apply (enumerated in §Response Matrix). Pivot
*reinforces* 01-M-004 — per-hart state stays on `RVCore` under
`arch/riscv/cpu/`.

### Changes from Previous Round

[**Added**]
- `CPU<Core>.cores: Vec<Core>`, `CPU.bus: Bus`, `CPU.current: usize`.
- `CPU::split_current_mut`, `CPU::invalidate_reservations_except`,
  `CPU::bus`/`bus_mut`/`current`/`current_mut`.
- `RVCore::id: HartId`, `RVCore::last_store`; `RVCore::with_id(id, irq)`.
- `MachineBuilder` trait at `cpu/core.rs` (tiny) so `CPU::from_config`
  stays arch-agnostic. See §Architecture.

[**Changed**]
- `CoreOps::step` takes `&mut Bus`.
- `CoreOps::{bus, bus_mut}` removed; `CPU::{bus, bus_mut}` added.
- `arch/riscv/cpu/mm.rs` 8 methods gain `bus: &mut Bus` (§API Surface).
- MMIO construction moves out of `RVCore::with_config` into the
  `MachineBuilder::build` impl for `RVCore`.
- R-020 hook lives on `RVCore::checked_write` (not `Hart`).
- Invalidation helper lives on `CPU` (not `RVCore`).
- PR1 new-lib-test count 13 → 11 (V-UT-1/V-UT-2 fold in).
- C-7: `≤ 700` → `≤ 720` (R-025(a)).

[**Removed**]
- `Hart` struct entirely.
- `CoreOps::bus` / `bus_mut`.
- Round-03 step 3 (`Hart::new`…) and V-UT-1 / V-UT-2 as standalone
  `#[test]`s (their assertions re-home into existing `RVCore` tests).

[**Unresolved**]
- None.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| User | Pivot (04) | Accepted (binding) | `Hart` dropped; `RVCore` IS a hart; `CPU` owns `Vec<Core>` + `Bus`. See §Architecture, §API Surface. |
| Review | R-025 (LOW) | Accepted (opt a) | C-7 relaxed to `≤ 720`; no plan-body trim. |
| Review | R-020 (MED, 02) | Carried | Callee-record inside `RVCore::checked_write`. See I-8, step 7. |
| Review | R-021 (LOW) | Carried | Subsumed by R-023 (`X_HARTS` `match`). |
| Review | R-022 (LOW) | Carried | 14 PLIC tests pass unchanged in PR2a (V-IT-6). |
| Review | R-023 (LOW) | Carried | `X_HARTS` `match` mirrors `main.rs:45-53`. See step 21. |
| Review | R-024 (LOW) | Carried | C-7 row reads `≤ 720 lines`. |
| Review | TR-3/6/7/8/9 | Carried | 3-PR split; mm-layer hook depth; V-IT-6 regression block; callee-record; R-025(a). |
| Review | R-001..R-019 | Carried | Resolved in earlier rounds; pivot substitutes Hart → RVCore where applicable; no regression. |
| Master | 00-M-001 | Applied | No `Arch` trait; per-concern `CoreOps` / `DebugOps` only. |
| Master | 00-M-002 | Applied | No new top-level file; changes inside existing `arch/riscv/cpu/{mod,mm}.rs`. |
| Master | 01-M-001 | Applied | No `selected`; `current` / `current_mut` only. |
| Master | 01-M-002 | Applied | Pivot *net-shrinks* code (removes a struct + a seam method). |
| Master | 01-M-003 | Applied | No new cfg scaffolding. |
| Master | 01-M-004 | Applied | `cpu/` / `device/` / `isa/` stay trait APIs; `CPU` gains `Bus` field via generic wrapper. Zero RISC-V types leak via `MachineBuilder` seam. |

---

## Spec

[**Goals**]

- **G-1** `RVCore` gains `id: HartId` + `last_store`; no new struct.
- **G-2** `CPU<Core> = { cores, bus, current, state, halt_pc, halt_ret,
  boot_config, boot_layout }`.
- **G-3** `Mswi`, `Mtimer`, `Sswi` carry per-hart `Vec` state with
  spec-mandated strides; MMIO byte-identical at `num_harts = 1`.
- **G-4** `MachineConfig::num_harts: usize` (default 1) flows into
  `CPU::from_config`, `Bus::new`, every sub-device.
- **G-5** Round-robin in `CPU::step`: tick bus once, step
  `cores[current]` with `&mut self.bus`, consume
  `cores[current].last_store.take()`, invalidate peers, advance cursor.
- **G-6** Per-hart SSIP: `Bus::take_ssip(HartId) -> bool`,
  `Bus::ssip_flag(HartId) -> Arc<AtomicBool>`.
- **G-7** PR1 at `num_harts = 1`: **354 + 11 = 365 lib** + 1
  `arch_isolation` + 6 `xdb` = **372 tests**; `make linux`/`debian`
  unchanged; difftest unchanged.
- **G-8** PR2a PLIC runtime-size at `num_harts = 1`: **366 lib + 1 +
  6 = 373**; 14 existing PLIC tests pass unchanged (V-IT-6).
- **G-9** PR2b `num_harts = 2` via `X_HARTS=2` + `xemu-2hart.dtb`;
  Linux SMP boot to `buildroot login:` with `smp: Brought up 1 node,
  2 CPUs`. **369 lib + 1 + 6 = 376**.
- **G-10** Cross-hart LR/SC correctness via `RVCore::checked_write`
  post-condition; every physical store invalidates peer reservations
  within the granule.

[**Non-Goals**]

- **NG-1** PLIC gateway redesign — deferred.
- **NG-2** Parallel (multi-threaded) core execution.
- **NG-3** Difftest at `num_harts > 1` — driver asserts == 1 at setup.
- **NG-4** Asymmetric core configs.
- **NG-5** `Bus::mtime` stays.
- **NG-6** Multi-hart debugger UX — `DebugOps` targets `cores[current]`
  at call time; per-hart selection deferred to `xdb-smp-ux`.
- **NG-7** DTB mutation tooling — PR2b ships static `xemu-2hart.dts`.
- **NG-8** OpenSBI reconfiguration — HSM pre-verified.
- **NG-9** Per-core breakpoints/watchpoints UX.
- **NG-10** (new) No `Hart` struct; RVCore-is-hart is load-bearing.

[**Architecture**]

```
CPU<Core: CoreOps> { cores: Vec<Core>, bus: Bus, current: usize,
                    state, halt_pc, halt_ret, boot_config, boot_layout }

RVCore { id, gpr, fpr, pc, npc, csr, privilege, pending_trap,
         reservation, mmu, pmp, irq, halted, ebreak_as_trap,
         breakpoints, next_bp_id, skip_bp_once,
         last_store: Option<(usize, usize)> }   // bus removed

Bus { ram, mmio, mtimer_idx, plic_idx, tick_count,
      ssip_pending: Vec<Arc<AtomicBool>>,       // len == num_harts
      num_harts: usize, [difftest] mmio_accessed }

Mswi   { msip: Vec<u32>,            irqs: Vec<IrqState> }
Mtimer { mtime, mtimecmp: Vec<u64>, irqs: Vec<IrqState> }
Sswi   { ssip: Vec<Arc<AtomicBool>> }
Plic   { num_ctx, priority, pending,
         enable: Vec<u32>, threshold: Vec<u8>, claimed: Vec<u32>,
         irqs: Vec<IrqState> }                  // PR2a shape
```

`CPU::step` body (deterministic):

```rust
fn step(&mut self) -> XResult {
    let (core, bus) = self.split_current_mut();  // disjoint borrows
    let result = core.step(bus);
    // …ProgramExit branch matches today's shape at cpu/mod.rs:170-190…
    result?;
    if let Some((addr, size)) = self.cores[self.current].take_last_store() {
        self.invalidate_reservations_except(self.current, addr, size);
    }
    if self.cores[self.current].halted() {
        self.set_terminated(State::Halted).log_termination();
    }
    self.current = (self.current + 1) % self.cores.len();
    Ok(())
}

fn split_current_mut(&mut self) -> (&mut Core, &mut Bus) {
    (&mut self.cores[self.current], &mut self.bus)
}
```

`RVCore::step(&mut self, bus: &mut Bus)` preserves today's body at
`mod.rs:223-260`: every `self.bus` is rewritten as the `bus` parameter.
`bus.tick` moves to `CPU::step` (T-10: ticking once per
fairness-cycle, not per core, matches SMP device-timing semantics; at
`num_harts = 1` this is byte-identical).

R-020 hook in `RVCore::checked_write`:

```rust
fn checked_write(&mut self, bus: &mut Bus, addr: VirtAddr, size: usize,
                 value: Word, op: MemOp) -> XResult {
    let pa = self.access_bus(bus, addr, op, size)?;
    bus.write(pa, size, value).map_err(|e| Self::to_trap(e, addr, op))?;
    if matches!(op, MemOp::Store | MemOp::Amo) {
        self.last_store = Some((pa, size));
    }
    Ok(())
}
```

`CPU::invalidate_reservations_except` (granule range-overlap, R-016):

```rust
fn invalidate_reservations_except(&mut self, src: usize,
                                  addr: usize, size: usize) {
    let end = addr.wrapping_add(size);
    for (i, c) in self.cores.iter_mut().enumerate() {
        if i == src { continue; }
        if let Some(r) = c.reservation() {
            let base = r & !(RESERVATION_GRANULE - 1);
            if base < end && base.wrapping_add(RESERVATION_GRANULE) > addr {
                c.clear_reservation();
            }
        }
    }
}
```

`RESERVATION_GRANULE = 8`. At `num_harts == 1` the loop never enters.

**MachineBuilder seam** — `CPU::from_config` is generic over `Core`;
moving MMIO wiring from `RVCore::with_config` to `CPU::from_config`
would leak RISC-V device types. Tiny trait at `cpu/core.rs`:

```rust
pub trait MachineBuilder {
    type Core: CoreOps;
    fn build(config: MachineConfig, layout: BootLayout)
        -> (Vec<Self::Core>, Bus);
}
```

Implemented for `RVCore` under `arch/riscv/cpu/mod.rs` (stub for
`LACore`). `CPU::from_config` delegates. Preserves 01-M-004.

[**Invariants**]

- **I-1** `CPU::cores.len() == config.num_harts` for CPU lifetime.
- **I-2** `cores[i].id() == HartId(i as u32)` for all `i`.
- **I-3** Per-core sub-devices use `Vec<T>` of length `num_harts`;
  decode `hart = offset / stride` (MSWI 4, MTIMER mtimecmp 8, SSWI 4);
  PLIC `hart = ctx >> 1`.
- **I-4** At `num_harts == 1`, guest-visible behaviour byte-identical
  pre-/post- PR1 + PR2a.
- **I-5** IRQ routing: MSIP[h] → `Mswi.irqs[h]`; `mtimecmp[h]` fire →
  `Mtimer.irqs[h].MTIP`; PLIC ctx `c` drives `Plic.irqs[c >> 1]` with
  `ip = if c & 1 == 0 { MEIP } else { SEIP }`.
- **I-6** `mhartid` CSR reads `self.id.0 as Word`. Seeded in
  `RVCore::with_id`; hard-coded `mhartid = 0` at `csr.rs:250` deleted
  in PR1.
- **I-7** `arch_isolation` passes unchanged: no new `SEAM_FILES` /
  `SEAM_ALLOWED_SYMBOLS` entries; `BUS_DEBUG_STRING_PINS` unchanged;
  `HartId` re-exported at `cpu/core.rs` as plain newtype.
- **I-8** `RVCore::checked_write` post-condition: on `Ok`, if `op ∈
  {Store, Amo}` then `self.last_store == Some((pa, size))`. `CPU::step`
  consumes via `take_last_store()` and, on `Some`, calls
  `invalidate_reservations_except`. Covers store_op, fstore_op, sc_w,
  sc_d, all 18 AMOs via the funnel at `mm.rs:306-326`.
- **I-9** `Bus::num_harts()` returns the `num_harts` passed to
  `Bus::new`; `CPU::from_config` asserts
  `debug_assert_eq!(bus.num_harts(), config.num_harts)`.
- **I-10** `CoreOps::step` is the *only* method that receives `&mut
  Bus`. Core-internal methods that need the bus thread it as a
  parameter from `step`; no `Core` stores a `Bus`.

[**Data Structure**]

Shapes are shown in §Architecture. Type additions / changes:

```rust
// arch/riscv/cpu/mod.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HartId(pub u32);

// RVCore: +id: HartId, +last_store: Option<(usize, usize)>;
//         -bus: Bus (removed).  All other fields unchanged.
// CPU<Core>: core: Core  →  cores: Vec<Core> + bus: Bus + current: usize.
// Bus: ssip_pending: Arc<AtomicBool>  →  Vec<Arc<AtomicBool>>; +num_harts.
// Plic (PR2a): drop NUM_CTX / CTX_IP constants; +num_ctx, +irqs: Vec<IrqState>;
//              enable / threshold / claimed become Vec<_>.
// MachineConfig: +num_harts: usize (default 1).

pub(in crate::arch::riscv) const RESERVATION_GRANULE: usize = 8;
```

[**API Surface**]

```rust
// cpu/core.rs
pub trait CoreOps {
    fn pc(&self) -> VirtAddr;
    fn reset(&mut self) -> XResult;
    fn setup_boot(&mut self, mode: BootMode);
    fn step(&mut self, bus: &mut Bus) -> XResult;    // CHANGED
    fn halted(&self) -> bool;
    fn halt_ret(&self) -> Word;
    // NEW:
    fn id(&self) -> HartId;
    fn reservation(&self) -> Option<usize>;
    fn clear_reservation(&mut self);
    fn take_last_store(&mut self) -> Option<(usize, usize)>;
    // REMOVED: bus / bus_mut
}

pub trait MachineBuilder {
    type Core: CoreOps;
    fn build(config: MachineConfig, layout: BootLayout)
        -> (Vec<Self::Core>, Bus);
}

// arch/riscv/cpu/mod.rs
impl RVCore {
    pub fn new() -> Self;                            // id = HartId(0)
    pub fn with_id(id: HartId, irq: IrqState) -> Self;
    pub(in crate::arch::riscv) fn reservation(&self) -> Option<usize>;
    pub(in crate::arch::riscv) fn clear_reservation(&mut self);
}
impl MachineBuilder for RVCore { /* see §Architecture */ }

// arch/riscv/cpu/mm.rs — bus threaded through
impl RVCore {
    fn checked_read(&mut self, bus: &mut Bus, addr: VirtAddr,
                    size: usize, op: MemOp) -> XResult<Word>;
    fn checked_write(&mut self, bus: &mut Bus, addr: VirtAddr,
                     size: usize, value: Word, op: MemOp) -> XResult;
    pub(super) fn fetch(&mut self, bus: &mut Bus) -> XResult<u32>;
    pub(super) fn load(&mut self, bus: &mut Bus, addr: VirtAddr,
                       size: usize) -> XResult<Word>;
    pub(super) fn store(&mut self, bus: &mut Bus, addr: VirtAddr,
                        size: usize, value: Word) -> XResult;
    pub(super) fn amo_load(&mut self, bus: &mut Bus, addr: VirtAddr,
                           size: usize) -> XResult<Word>;
    pub(super) fn amo_store(&mut self, bus: &mut Bus, addr: VirtAddr,
                            size: usize, value: Word) -> XResult;
    pub(super) fn translate(&mut self, bus: &mut Bus, addr: VirtAddr,
                            size: usize, op: MemOp) -> XResult<usize>;
}

// cpu/mod.rs
impl<Core: CoreOps + DebugOps> CPU<Core> {
    pub fn new(cores: Vec<Core>, bus: Bus, layout: BootLayout) -> Self;
    pub fn from_config<B>(config: MachineConfig, layout: BootLayout) -> Self
        where B: MachineBuilder<Core = Core>;
    pub fn step(&mut self) -> XResult;               // external API unchanged
    pub fn run(&mut self, count: u64) -> XResult;
    pub fn pc(&self) -> usize;
    pub fn bus(&self) -> &Bus;
    pub fn bus_mut(&mut self) -> &mut Bus;
    pub fn current(&self) -> &Core;
    pub fn current_mut(&mut self) -> &mut Core;
    pub fn replace_device(&mut self, name: &str, dev: Box<dyn Device>);
    #[cfg(feature = "difftest")] pub fn bus_take_mmio_flag(&self) -> bool;
    fn split_current_mut(&mut self) -> (&mut Core, &mut Bus);
    fn invalidate_reservations_except(&mut self, src: usize,
                                      addr: usize, size: usize);
    // DebugOps passthrough (delegated via cores[current]) — signatures unchanged.
}

// device/bus.rs
impl Bus {
    pub fn new(ram_base: usize, ram_size: usize, num_harts: usize) -> Self;
    pub fn num_harts(&self) -> usize;
    pub fn ssip_flag(&self, hart: HartId) -> Arc<AtomicBool>;
    pub fn take_ssip(&self, hart: HartId) -> bool;
}

// arch/riscv/device/intc/aclint/{mswi,mtimer,sswi}.rs
impl Mswi   { pub(super) fn new(irqs: Vec<IrqState>) -> Self; }
impl Mtimer { pub(super) fn new(irqs: Vec<IrqState>) -> Self; }
impl Sswi   { pub(super) fn new(ssips: Vec<Arc<AtomicBool>>) -> Self; }
impl Aclint {
    pub fn new(irqs: Vec<IrqState>, ssips: Vec<Arc<AtomicBool>>) -> Self;
    pub fn install(self, bus: &mut Bus, base: usize) -> usize;
}

// arch/riscv/device/intc/plic.rs (PR2a)
impl Plic { pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self; }
```

[**Constraints**]

- **C-1** `num_harts ∈ [1, 16]`.
- **C-2** MMIO layout (MSWI/MTIMER/SSWI base + stride) invariant.
- **C-3** `HartId` is a newtype over `u32` exposed at `cpu/core.rs` via
  `CoreOps::id` — the minimum surface `CPU::invalidate_reservations_
  except` needs; no RISC-V semantics leak.
- **C-4** Round-robin = declaration order (`cores[0..N]` cycle).
- **C-5** PR1 / PR2a do not modify DTBs; PR2b adds `xemu-2hart.dts`.
- **C-6** No new crate dependencies.
- **C-7** Plan body ≤ **720** lines (R-025(a)).
- **C-8** `DebugOps` signatures unchanged across all three PRs.
- **C-9** `CoreOps::step` signature changes in PR1 (bus parameter);
  `CoreOps::{bus, bus_mut}` removed in PR1; unchanged after PR1.
- **C-10** At `num_harts == 1` every `.step()`-calling test remains
  byte-identical in observable outcome.

---

## Implement

### Execution Flow

[**Main Flow — PR1**] Bus-pivot + per-core state at `num_harts = 1`:

1. Add `pub struct HartId(pub u32)` in `arch/riscv/cpu/mod.rs` and
   re-export at `cpu/core.rs` (trait seam only).
2. Extend `RVCore`: add `id: HartId`, `last_store: Option<(usize,
   usize)>`; remove `bus: Bus`.
3. Add `RVCore::with_id(id, irq)`; retain `new()` with `id = HartId(0)`
   for legacy single-core paths.
4. Extend `CoreOps` (add `id`, `reservation`, `clear_reservation`,
   `take_last_store`; remove `bus`/`bus_mut`; change `step` signature).
5. Update `impl CoreOps for RVCore`: thread `bus` through `step`;
   `id`/`reservation`/`clear_reservation`/`take_last_store` trivial.
6. Thread `bus: &mut Bus` through 8 methods in `mm.rs` (§API Surface);
   callers in `inst/{base,atomic,float,privileged,compressed,zicsr,
   mul}.rs` and `trap/handler.rs` receive it by propagating the arg
   from `step` through `execute(inst, bus) → dispatch(inst, bus) → op
   methods`. Mechanical; type-system-guided.
7. **R-020 hook** inside `RVCore::checked_write` after the successful
   `bus.write` and before `Ok(())`:
   ```rust
   if matches!(op, MemOp::Store | MemOp::Amo) {
       self.last_store = Some((pa, size));
   }
   ```
   `pa` is in scope from `access_bus`. Caller audit (round 02) still
   holds: only `store` / `amo_store` funnel through `checked_write`.
8. `Bus::new` widens to `Bus::new(ram_base, ram_size, num_harts)`;
   `ssip_pending: Vec<Arc<AtomicBool>>` length `num_harts`;
   `take_ssip(HartId)` / `ssip_flag(HartId)` bounds-checked; add
   `num_harts()`.
9. ACLINT sub-devices widen to per-hart `Vec` state. Decode `hart =
   offset / stride` with bounds check; out-of-range reads 0, writes
   drop (V-F-2, V-F-3).
10. Delete hard-coded `mhartid = 0` at `csr.rs:250`; `RVCore::with_id`
    seeds `csr.set(CsrAddr::mhartid, id.0 as Word)`.
11. Add `MachineBuilder` trait at `cpu/core.rs`; implement for
    `RVCore` in `arch/riscv/cpu/mod.rs` (body copies today's
    `RVCore::with_config` at `mod.rs:58-90` but builds `num_harts`
    IrqStates + SSIP flags, installs ACLINT via `Aclint::new(irqs,
    ssips)`, installs `Plic::new(irqs[0].clone())` for now (PR2a
    converts to `Plic::new(num_harts, irqs.clone())`), pushes
    `num_harts` cores via `RVCore::with_id(HartId(i), irqs[i].clone())`,
    returns `(cores, bus)`; asserts `debug_assert_eq!(bus.num_harts(),
    config.num_harts)`). Stub impl for `LACore`.
12. Rewrite `CPU<Core>`: replace `core: Core` with `cores: Vec<Core>`
    + `bus: Bus` + `current: usize`. Rewrite constructors and
    `load_direct` / `load_firmware` / `load_file_at` /
    `replace_device` / `bus_take_mmio_flag` / debug-ops delegates to
    go through `self.bus` or `self.cores[self.current]`. Add
    `split_current_mut`, `invalidate_reservations_except`, `bus`,
    `bus_mut`, `current`, `current_mut`. Rewire `CPU::step` per
    §Architecture. `CPU::step` external signature unchanged, so
    `xdb/src/cmd.rs:37` and `xdb/src/difftest/mod.rs:56` do not change.
13. Tick `bus.tick()` in `CPU::step` before `core.step(bus)`. Remove
    the `bus.tick()` line inside `RVCore::step` (was `mod.rs:225`).
    Per-hart per-instruction `csr.set(time, ...)` / `take_ssip` /
    `sync_interrupts` continue to run inside `RVCore::step` against
    the threaded `bus`.

[**Main Flow — PR1 (test-fixture refactor)**]

14. Replace `setup_core() -> RVCore` at `arch/riscv/cpu/mod.rs:276`
    with `setup_core_bus() -> (RVCore, Bus)` (+ thin single-return
    wrapper for tests that don't need the bus). `write_inst(core,
    bus, inst)` gains the bus parameter. Each `core.step()` → `core.
    step(&mut bus)`; each `core.bus.write` → `bus.write`.
15. Migrate fixtures across `arch/riscv/cpu/{inst/*.rs, csr.rs, mm.rs,
    mm/*.rs, trap/handler.rs, debug.rs}` similarly. Estimated ~60
    call sites (per grep `\.step()` / `\.bus\.` under `arch/riscv`);
    mechanical.

[**Main Flow — PR2a**] PLIC runtime-size at `num_harts = 1`:

16. Rewrite `arch/riscv/device/intc/plic.rs`: delete `NUM_CTX`,
    `CTX_IP`; add `num_ctx: usize`, `irqs: Vec<IrqState>`.
    `Plic::new(num_harts, Vec<IrqState>)` sets `num_ctx = 2 *
    num_harts`; `vec![_; num_ctx]` for `enable`, `threshold`,
    `claimed`. `evaluate` iterates `0..self.num_ctx`; `ip_bit = if
    ctx & 1 == 0 { MEIP } else { SEIP }`; target `self.irqs[ctx >>
    1]`. `ctx_at` becomes a `&self` method using `self.num_ctx`;
    `complete` bounds check uses `self.num_ctx`. All **14 existing
    PLIC tests** pass unchanged with `Plic::new(1, vec![irq.clone()])`
    (V-IT-6 regression block). +1 new V-UT-10.
17. Update `RVCore::build` site: `Plic::new(irq.clone())` →
    `Plic::new(config.num_harts, plic_irqs.clone())`.

[**Main Flow — PR2b**] Activate `num_harts > 1`:

18. `MachineConfig::with_harts(n)` builder; `debug_assert!((1..=16).
    contains(&n))` (C-1).
19. Seed firmware boot in `impl CoreOps for RVCore::setup_boot`: set
    `a0 = self.id.0`, `a1 = fdt_addr`; non-zero cores start `halted =
    true`; ACLINT MSIP releases them per OpenSBI HSM (NG-8).
20. `reset` in `CoreOps for RVCore` iterates to clear `reservation`,
    `last_store`, `halted`; `CPU::reset` resets all cores + bus
    devices.
21. **`X_HARTS` env var** in `xemu/xdb/src/main.rs::machine_config`,
    mirroring `X_DISK` shape at `main.rs:45-53`:
    ```rust
    let num_harts = match env("X_HARTS") {
        Some(s) => s.parse::<usize>()
            .map_err(|e| anyhow!("X_HARTS must be a usize: {e}"))?,
        None => 1,
    };
    ```
    Threaded via `MachineConfig::with_harts(num_harts)`.
22. Add `resource/xemu-2hart.dts` (clone of `xemu.dts` with `cpu1` +
    `cpu-map cluster0/core1`; both cores feed `clint@2000000` /
    `plic@c000000`). `resource/Makefile` gains `xemu-2hart.dtb` rule
    and `linux-2hart` / `debian-2hart` phony targets invoking xdb
    with `X_HARTS=2 X_FDT=…xemu-2hart.dtb`.
23. Difftest driver asserts `num_harts == 1` at setup (NG-3).

[**Failure Flow**]

1. `hart >= num_harts` in sub-device MMIO: read 0, write drops.
2. `num_harts = 0` or `> 16`: `debug_assert!` at
   `MachineConfig::with_harts`.
3. OpenSBI fails to bring core 1 online (PR2b): dmesg shows 1 CPU;
   V-IT-5 fails.
4. PLIC routing mis-wired in PR2a: caught by V-UT-10.
5. I-8 violation (cross-hart SC succeeds after peer store via Store,
   Amo, or FP path): caught by V-UT-11/13/14.
6. Difftest divergence at `num_harts > 1`: unsupported; driver asserts.
7. `X_HARTS` parse failure: `anyhow::Error` matches `X_DISK` style.
8. Borrow-checker split in `CPU::step` (`&mut cores[current]` vs
   `&mut self.bus`): resolved by `split_current_mut`'s disjoint-field
   pattern; compile-time gate (V-F-7).

[**State Transition**]

- **S0 (today)** `CPU { core: RVCore }`; `RVCore` owns `bus`.
- **S0 → S1 (PR1)** `CPU { cores: Vec<RVCore> len 1, bus, current: 0 }`;
  `RVCore` bus-less; mm-layer bus-threaded; I-8 hook live (no-op at
  len 1); `mhartid` seeded by `with_id`. I-4 byte-identical.
- **S1 → S2 (PR2a)** `Plic { num_ctx: 2, enable: Vec (len 2), irqs: Vec
  (len 1) }`. Guest-visible identical.
- **S2 → S3 (PR2b)** `X_HARTS=2`: cores Vec-of-N, ACLINT Vecs-of-N,
  PLIC `num_ctx = 2N`; `xemu-2hart.dtb`.

### Implementation Plan

[**Phase 1 — PR1**] Steps 1–15.

Gate matrix:
- `cargo fmt --check`, `make clippy` clean.
- `X_ARCH=riscv64 cargo test --workspace` → **365 lib + 1 + 6 = 372 tests pass**.
- `cargo test --test arch_isolation` passes.
- `make linux` → `buildroot login:` ≤ 60 s; `make debian` → login + Python3 ≤ 120 s.
- Difftest corpus unchanged (aclintSplit green set).

[**Phase 2a — PR2a**] Steps 16–17.

Gate matrix:
- All PR1 gates (regression).
- **366 lib + 1 + 6 = 373** (PR1 365 + V-UT-10).
- 14 existing PLIC tests pass unchanged (V-IT-6 regression block).
- `make linux` / `make debian` unchanged. Difftest unchanged.

[**Phase 2b — PR2b**] Steps 18–23.

Gate matrix:
- All PR2a gates at `num_harts = 1` (regression).
- **369 lib + 1 + 6 = 376** (PR2a 366 + V-IT-2 + V-IT-4 + V-IT-5).
- `X_HARTS=2 make linux-2hart` → `buildroot login:` ≤ 120 s with
  `smp: Brought up 1 node, 2 CPUs`.
- Difftest pinned to `num_harts == 1`.

---

## Trade-offs

- **T-1** scheduling (a) one-instruction round-robin in `CPU::step` —
  chosen; (b) N-burst — starvation risk; (c) skip-halted — breaks SBI
  HSM handshake.
- **T-2** AoS `Vec<RVCore>` — chosen over SoA.
- **T-3** 3 PRs (PR1 / PR2a / PR2b) — carried.
- **T-4** debug UX scalar via `cores[current]` — chosen (NG-6).
- **T-5** per-hart `take_ssip(HartId)` — chosen.
- **T-6** I-8 hook depth at mm-layer (`checked_write`) — chosen
  (R-011/TR-6); all stores covered by construction.
- **T-7** `last_store` scratch on `RVCore`, `.take()` on `CPU` after
  `core.step(bus)` — chosen; single assignment, no dispatch churn.
- **T-8** `X_HARTS` env var over clap flag — chosen (R-013).
- **T-9** callee-record inside `checked_write` gated on `op` —
  chosen (R-020/TR-8).
- **T-10** bus.tick placement (a) in `CPU::step` once per
  fairness-cycle — chosen; (b) in `RVCore::step` — rejected: at
  `num_harts > 1` every core would re-tick, breaking device timing.
- **T-11** bus threading (a) parameterize every `mm.rs` method —
  chosen; type-system-guided; (b) raw-pointer `*mut Bus` field scoped
  per step — rejected: unsafe, hides lifetime.
- **T-12** `HartId` at `arch/riscv/cpu/mod.rs` re-exported at
  `cpu/core.rs` for `CoreOps::id` — chosen; newtype carries no
  RISC-V semantics; top-level-owned `HartId` rejected as invites a
  trait registry.
- **T-13** `MachineBuilder` trait — chosen; keeps `CPU::from_config`
  arch-agnostic. Inline RISC-V types in `cpu/mod.rs::from_config`
  rejected for 01-M-004.

---

## Validation

[**Unit Tests — PR1 (11 new `#[test]` functions)**]

| # | Test function | File | Purpose |
|---|---------------|------|---------|
| V-UT-3 | `mswi_four_harts_msip2_raises_only_irq2` | `device/intc/aclint/mswi.rs` | G-3, I-3, I-5 |
| V-UT-4 | `mtimer_two_harts_mtimecmp0_fires_only_irq0` | `device/intc/aclint/mtimer.rs` | G-3, I-5 |
| V-UT-5 | `sswi_three_harts_setssip1_raises_only_ssip1` | `device/intc/aclint/sswi.rs` | G-3, I-3 |
| V-UT-6 | `bus_new_four_harts_ssip_vec_len_and_share` | `device/bus.rs` | G-6, I-1, I-9 |
| V-UT-7 | `machine_config_default_num_harts_is_one` | `config/mod.rs` | G-4 |
| V-UT-9 | `cores_ids_match_index` | `cpu/mod.rs` tests | I-2 |
| V-UT-11 | `cross_core_lr_sw_sc_invalidation` | `cpu/mod.rs` tests | I-8 via store_op |
| V-UT-12 | `same_core_store_keeps_other_reservation` | `cpu/mod.rs` tests | I-8 `src` skip |
| V-UT-13 | `amo_invalidates_peer_reservation` | `cpu/mod.rs` tests | I-8 via amo_store |
| V-UT-14 | `fsw_invalidates_peer_reservation` | `cpu/mod.rs` tests | I-8 via fstore_op |
| V-IT-3 | `round_robin_fairness_single_hart` | `tests/` | G-5 degenerate |

V-UT-1 / V-UT-2 (round 03 `hart_new_…` / `hart_reset_…`) fold into
existing `RVCore::new` + `reset` tests as added assertions (id seeding,
`last_store` clearing). V-UT-8 is the pass-through ACLINT test in the
354 baseline.

[**Unit Tests — PR2a (1 new `#[test]`)**]

| # | Test function | File | Purpose |
|---|---------------|------|---------|
| V-UT-10 | `plic_new_num_harts_two_ctx2_routes_to_irq1` | `device/intc/plic.rs` | G-8, I-5 |

V-IT-6: **14 existing PLIC tests** pass unchanged with `Plic::new(1,
vec![irq.clone()])`. Zero-regression gate, not a new test.

[**Unit / Integration Tests — PR2b (3 new `#[test]`)**]

| # | Test function | File | Purpose |
|---|---------------|------|---------|
| V-IT-2 | `plic_2hart_context_map` | `tests/` | G-9, I-5 |
| V-IT-4 | `round_robin_fairness_two_harts` | `tests/` | G-5 |
| V-IT-5 | `smp_linux_smoke` | `tests/` (ignored by default; `X_HARTS=2`) | G-9 end-to-end |

[**Integration Tests (existing)**]

- **V-IT-1** `arch_isolation` — unchanged (I-7).

[**Failure / Robustness Validation**]

- **V-F-1** `num_harts = 0` or `> 16` → `debug_assert!` (C-1).
- **V-F-2** MMIO write to `MSIP[num_harts]` silently drops.
- **V-F-3** MTIMER read at `mtimecmp[h]` for `h >= num_harts` → 0.
- **V-F-4** `CPU::reset` post-condition: every core's `pc ==
  RESET_VECTOR`, `reservation.is_none()`, `last_store.is_none()`.
- **V-F-5** *(PR2b)* OpenSBI brings only core 0 online: dmesg shows 1
  CPU; V-IT-5 fails.
- **V-F-6** *(PR2b)* `make linux-2hart` > 120 s: V-IT-5 fails.
- **V-F-7** *(PR1 compile-time)* `cargo build --tests` fails iff
  `split_current_mut` doesn't produce disjoint borrows.

[**Edge Case Validation**]

- **V-E-1** `num_harts = 1` byte-identical (I-4): all 354 existing lib
  tests pass; `make debian` boot-to-Python3 trace identical (timing
  excluded).
- **V-E-2** Offset decode at `num_harts = 3`: MSWI accepts offsets {0,
  4, 8}; `offset = 12` reads 0.
- **V-E-3** Round-robin wraparound at `num_harts = 2` (`current`
  advances `0 → 1 → 0 → 1 …`).
- **V-E-4** *(PR2b)* hartid seeding: `cores[0].gpr[a0] == 0`,
  `cores[1].gpr[a0] == 1`; `mhartid` CSR matches.
- **V-E-5** *(PR1)* `store_overlapping_granule_invalidates` — LR.D on
  `0x80001000`; peer `sw` to `0x80001004`; SC.D fails. Assertion in
  V-UT-11.
- **V-E-6** *(PR1)* `store_outside_granule_preserves` — LR.W on
  `0x80001000`; peer `sw` to `0x80001010`; SC.W succeeds. Assertion
  in V-UT-11 / V-UT-12.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (HartId + RVCore.id/last_store) | existing `RVCore::new` + `reset` tests (rebased), V-UT-9 |
| G-2 (CPU shape) | V-IT-3, V-IT-1, PR1 gate matrix |
| G-3 (ACLINT per-hart) | V-UT-3, V-UT-4, V-UT-5 |
| G-4 (MachineConfig::num_harts) | V-UT-7, V-F-1 |
| G-5 (round-robin in CPU) | V-IT-3, V-IT-4, V-E-3 |
| G-6 (per-hart SSIP) | V-UT-5, V-UT-6 |
| G-7 (PR1 behaviour-preservation) | V-E-1, PR1 gate (372 tests) |
| G-8 (PR2a PLIC reshape) | V-UT-10, V-IT-6, PR2a gate (373 tests) |
| G-9 (PR2b SMP boot) | V-IT-5, V-E-4, V-F-5, V-F-6 |
| G-10 (cross-hart LR/SC via checked_write post-condition) | V-UT-11..14 |
| C-1 (hart count bounds) | V-F-1 |
| C-2 (MMIO layout) | V-UT-3..5, V-E-2, V-IT-6 |
| C-3 (HartId seam) | V-IT-1 |
| C-4 (deterministic order) | V-IT-3, V-IT-4, V-E-3 |
| C-5 (DTB untouched PR1/PR2a) | PR1/PR2a gate matrices |
| C-6 (no new deps) | Cargo.lock diff review per PR |
| C-7 (≤ 720 lines) | `wc -l 04_PLAN.md` at plan-review |
| C-8 (DebugOps unchanged) | V-IT-1 + xdb 6-test suite |
| C-9 (CoreOps::step signature PR1 only) | PR1/PR2a/PR2b gates |
| C-10 (byte-identical at num_harts==1) | V-E-1, V-IT-6 |
| I-1..I-3 | V-UT-6, V-UT-7, V-UT-9, V-UT-3..5, V-E-2, V-UT-10 |
| I-4 | V-E-1, V-IT-6 |
| I-5 | V-UT-3, V-UT-4, V-UT-10, V-IT-2 |
| I-6 | `RVCore::with_id` test, V-E-4 |
| I-7 | V-IT-1 |
| I-8 | V-UT-11..14 |
| I-9 | V-UT-6 + `debug_assert_eq!` in `from_config` |
| I-10 | grep `&mut Bus` on `cpu/core.rs` at PR1 review |
