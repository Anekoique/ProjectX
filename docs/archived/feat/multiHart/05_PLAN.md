# `multiHart` PLAN `05`

> Status: Draft
> Feature: `multiHart`
> Iteration: `05`
> Owner: Executor
> Depends on:
> - Previous Plan: `04_PLAN.md`
> - Review: `04_REVIEW.md` (APPROVE-WITH-CHANGES, R-026 HIGH)
> - Master Directive: `04_MASTER.md` (empty template; user ordered a
>   concurrency audit in lieu of written directives — the audit matrix
>   below is treated as the controlling input for round 05).

---

## Summary

Round 05 closes the pivot: resolves R-026 by defining `HartId` at
`cpu/core.rs` (trait-seam layer) with a declared two-token
`arch_isolation` allow-list widening; folds R-027 (drop `<B>` generic,
add `CoreBuilder` alias), R-028 (consume-before-advance prose +
debug_assert), R-029 (trimmed Trade-offs), R-030 (V-IT-3 → V-UT-15
unit test); and absorbs the user-ordered concurrency audit (CC-1..
CC-10) as a top-level Concurrency Matrix bound to I-10/I-11/I-12 and
C-8. `04_MASTER.md` is empty — the concurrency matrix is the de-facto
round-05 directive set. Structural pivot (RVCore-is-hart, `CPU<Core>
{ cores, bus, current }`, `CoreOps::step(&mut self, bus: &mut Bus)`)
inherited from round 04. Three PRs: PR1 pivot + per-hart state at
`N=1`; PR2a PLIC runtime-size at `N=1`; PR2b activates `N>1`. Test
arithmetic: **365 → 366 → 369 lib** + 1 `arch_isolation` + 6 `xdb`.

## Log

[**Feature Introduce**] (a) Concurrency Matrix CC-1..CC-10 as a
top-level section; (b) `HartId` at `cpu/core.rs` + two-token
allow-list widening; (c) `pub type CoreBuilder` alias at `cpu/mod.rs`
replacing the `<B>` method generic; (d) `debug_assert!(src <
self.cores.len())` + R-028 ordering prose; (e) V-IT-3 demoted to
V-UT-15; (f) I-10/I-11/I-12 (per-hart reservation / mip / SSIP
isolation); (g) C-8 (concurrency baseline); (h) `setup_core_and_bus()`
test-fixture snippet (TR-10).

[**Review Adjustments**] R-026 (HIGH) → option (a) + declared
allow-list extension (trait-seam file listed as deliberate seam).
R-027 adopted (a). R-028 adopted. R-029 adopted — Trade-offs trimmed.
R-030 adopted (a) — V-IT-3 → V-UT-15. TR-9 adopted. TR-10 endorsed.

[**Master Compliance**] `04_MASTER.md` empty template. Inherited
00-M-001/002, 01-M-001/002/003/004 enumerated in Response Matrix.
User directive ("check detailly the Concurrent problem of hart
introduce") absorbed as Concurrency Matrix (CC-1..CC-10), bound to
I-10..I-12, C-8, §Architecture NG-2 rationale.

### Changes from Previous Round

[**Added**] Concurrency Matrix (CC-1..CC-10); `HartId` at `cpu/core.rs`;
allow-list edits (`"src/cpu/core.rs"`, `"HartId"`); `CoreBuilder`
alias; I-10/I-11/I-12; C-8; debug_assert in
`invalidate_reservations_except`; `setup_core_and_bus()` snippet;
V-UT-15 `cpu_step_advances_current_single_hart`.

[**Changed**] `CPU::from_config` drops `<B>` generic; `HartId` moved
from arch to trait-seam layer; I-7 wording; step 1 rewrite; V-IT-3 →
V-UT-15.

[**Removed**] `<B>` method-generic on `CPU::from_config`; round-04
T-12 rationale superseded.

[**Unresolved**] CC-4 (`XCPU: Mutex<CPU<Core>>` poison amplification)
— KNOWN-LIMITATION carried from single-hart; outside round-05 scope.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-026 (HIGH) | Accepted (opt a+) | `HartId` defined at `cpu/core.rs`; `arch_isolation.rs` += `"src/cpu/core.rs"` / `"HartId"`. See I-7, step 1. |
| Review | R-027 (MED) | Accepted (opt a) | `pub type CoreBuilder` at `cpu/mod.rs`; `from_config` drops `<B>`. See §API Surface, T-13. |
| Review | R-028 (LOW) | Accepted | §Architecture prose + I-8 + `debug_assert!(src < cores.len())`. |
| Review | R-029 (LOW) | Accepted | Trade-offs trimmed; body ≤ 720. |
| Review | R-030 (LOW) | Accepted (opt a) | V-IT-3 → V-UT-15 in `cpu/mod.rs::tests`. PR1 lib count 365 unchanged. |
| Review | TR-9 / TR-10 | Adopted / Endorsed | R-027 + `setup_core_and_bus()` helper. |
| Review | R-001..R-025 | Carried | Resolved earlier; no regression. |
| User | CC-1..CC-10 | Absorbed | See §Concurrency Matrix; bound to I-10..I-12, C-8. |
| Master | 00-M-001 | Applied | Only `CoreOps` + `DebugOps` + `MachineBuilder`; no `Arch` trait. |
| Master | 00-M-002 | Applied | No new top-level file; allow-list +2 tokens is declared edit, not drift. |
| Master | 01-M-001 | Applied | `current`/`current_mut`; no `selected`. |
| Master | 01-M-002 | Applied | Pivot net-shrinks code. |
| Master | 01-M-003 | Applied | No new cfg scaffolding. |
| Master | 01-M-004 | Applied | `CoreBuilder` alias binds at seam; trait body arch-agnostic. |

### Concurrency Matrix

| ID | Concern | NG-2 safe? | Future-MT delta | Bound to |
|----|---------|-----------|-----------------|----------|
| CC-1 | Per-hart `IrqState = Arc<AtomicU64>` Relaxed | Yes — one writer at a time (tick XOR step) | Escalate to `AcqRel` so MIP ordering holds | I-11, NG-2 rationale |
| CC-2 | Per-hart `ssip_pending: Vec<Arc<AtomicBool>>` | Yes — swap-to-false atomic; cross-hart SETSSIP normal | Relaxed still sufficient (flag-only edge) | I-12, §API Surface |
| CC-3 | MTIMER `check_timer` load-then-set | Yes — tick thread sole MTIP writer | CAS loop or per-hart mutex | §Architecture, V-UT-4 |
| CC-4 | `XCPU: Mutex<CPU>` poison amplified | Unchanged from today; panic surface wider | `parking_lot::Mutex` (new dep; rejected by C-6) | T-14, Unresolved |
| CC-5 | UART reader thread untouched | Yes — no per-hart impact | No change | §Architecture footnote |
| CC-6 | Difftest `mmio_accessed` global | Bug at N>1 | Pin difftest to N=1 | NG-3, step 21 |
| CC-7 | LR/SC cross-hart reservation invalidation | Yes — CPU-level single-threaded loop | Atomic reservation + broadcast before commit | I-8, I-10 |
| CC-8 | PLIC `irqs[ctx >> 1]` routing | Yes — tick thread sole writer | Same `AcqRel` as CC-1 | I-5, §API Surface |
| CC-9 | `Device: Send` preserved | Yes — new `Vec<IrqState>` / `Vec<Arc<AtomicBool>>` all Send | No change; future-MT-ready | §Architecture |
| CC-10 | `Arc<…>` over `Rc<…>` | Send-ready; single-threaded today | Zero refactor on MT adoption | T-15 |

**NG-2 rationale**: All harts run on one OS thread via `CPU::step`
round-robin. Each hart's `IrqState` / `ssip_pending[h]` /
`reservation` / `mhartid` / GPRs has exactly one writer at a time.
`IrqState` has two temporally-disjoint writers on the same thread —
device (`bus.tick()` runs *before* `core.step(bus)` in `CPU::step`)
and `RVCore::sync_interrupts` — so `Relaxed` suffices. The atomic
type is retained for (a) trait-bound Send-readiness and (b) zero
refactor when a future MT mode lands.

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
  `cores[current].last_store.take()` **before** advancing the cursor,
  invalidate peers, advance cursor.
- **G-6** Per-hart SSIP: `Bus::take_ssip(HartId) -> bool`,
  `Bus::ssip_flag(HartId) -> Arc<AtomicBool>`.
- **G-7** PR1 at `num_harts = 1`: **365 lib + 1 `arch_isolation` + 6
  `xdb` = 372 tests**; `make linux`/`debian` unchanged; difftest
  unchanged; `arch_isolation.rs` has `"src/cpu/core.rs"` and `"HartId"`
  appended to the allow-list.
- **G-8** PR2a PLIC runtime-size at `num_harts = 1`: **366 lib + 1 +
  6 = 373**; 14 existing PLIC tests pass unchanged (V-IT-6).
- **G-9** PR2b `num_harts = 2` via `X_HARTS=2` + `xemu-2hart.dtb`;
  Linux SMP boot to `buildroot login:` with `smp: Brought up 1 node,
  2 CPUs`. **369 lib + 1 + 6 = 376**.
- **G-10** Cross-hart LR/SC correctness via `RVCore::checked_write`
  post-condition; every physical store invalidates peer reservations
  within the granule.
- **G-11** Concurrency posture documented: every shared atomic or
  mutex in the multi-hart plumbing is justified against NG-2 and
  carries a future-MT-escalation note (Concurrency Matrix).

[**Non-Goals**]

- **NG-1** PLIC gateway redesign — deferred.
- **NG-2** Parallel (multi-threaded) core execution — foundation for
  CC-1..CC-10 safety analysis.
- **NG-3** Difftest at `num_harts > 1` — driver asserts == 1 at setup
  (CC-6).
- **NG-4** Asymmetric core configs.
- **NG-5** `Bus::mtime` stays.
- **NG-6** Multi-hart debugger UX — every `DebugOps` call targets
  `cores[self.current]` at call time; per-hart selection deferred to
  `xdb-smp-ux` (TR-9 of round 03; carried).
- **NG-7** DTB mutation tooling — PR2b ships static `xemu-2hart.dts`.
- **NG-8** OpenSBI reconfiguration — HSM pre-verified.
- **NG-9** Per-core breakpoints/watchpoints UX.
- **NG-10** No `Hart` struct; RVCore-is-hart is load-bearing.
- **NG-11** (new) `XCPU` poison-recovery and lock-shape changes (CC-4);
  outside round-05 scope.

[**Architecture**]

```
CPU<Core: CoreOps + DebugOps> { cores: Vec<Core>, bus: Bus,
                                current: usize,
                                state, halt_pc, halt_ret,
                                boot_config, boot_layout }

RVCore { id: HartId, gpr, fpr, pc, npc, csr, privilege, pending_trap,
         reservation: Option<usize>, mmu, pmp, irq: IrqState, halted,
         ebreak_as_trap, breakpoints, next_bp_id, skip_bp_once,
         last_store: Option<(usize, usize)> }

Bus { ram, mmio, mtimer_idx, plic_idx, tick_count,
      ssip_pending: Vec<Arc<AtomicBool>>,     // len == num_harts (CC-2)
      num_harts: usize,
      #[cfg(difftest)] mmio_accessed }        // global, OK under NG-3 (CC-6)

Mswi   { msip: Vec<u32>,            irqs: Vec<IrqState> }  // (CC-1)
Mtimer { mtime, mtimecmp: Vec<u64>, irqs: Vec<IrqState> }  // (CC-1,3)
Sswi   { ssip: Vec<Arc<AtomicBool>> }                       // (CC-2)
Plic   { num_ctx, priority, pending,
         enable: Vec<u32>, threshold: Vec<u8>, claimed: Vec<u32>,
         irqs: Vec<IrqState> }               // PR2a; irqs len == num_harts (CC-8)
```

`CPU::step` body (deterministic, single-threaded under NG-2):

```rust
fn step(&mut self) -> XResult {
    self.bus.tick();                 // device advancement (T-10)
    let (core, bus) = self.split_current_mut();
    let result = core.step(bus);
    // CC-7 / R-028: consume last_store BEFORE `self.current` advances.
    // `src` passed to invalidate_reservations_except MUST equal the
    // index of the core that wrote; reordering would silently route
    // invalidation to the next core and break G-10.
    if let Ok(()) = result
        && let Some((addr, size)) = self.cores[self.current].take_last_store()
    {
        self.invalidate_reservations_except(self.current, addr, size);
    }
    if self.cores[self.current].halted() {
        self.set_terminated(State::Halted).log_termination();
    }
    result?;
    self.current = (self.current + 1) % self.cores.len();
    Ok(())
}

fn split_current_mut(&mut self) -> (&mut Core, &mut Bus) {
    // Destructuring lets the borrow checker see `cores` and `bus`
    // as disjoint fields; indexing through `self` would alias.
    let Self { cores, bus, current, .. } = self;
    (&mut cores[*current], bus)
}

fn invalidate_reservations_except(&mut self, src: usize,
                                  addr: usize, size: usize) {
    debug_assert!(src < self.cores.len(), "src out of range");
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

`RVCore::step(&mut self, bus: &mut Bus)` preserves today's body at
`arch/riscv/cpu/mod.rs:223-260`: every `self.bus` rewritten as the
`bus` parameter. `bus.tick` moves from `RVCore::step` to `CPU::step`
(T-10).

**R-020 hook** in `RVCore::checked_write` (mm.rs:271): after the
successful `bus.write(pa, size, value)?`, before `Ok(())`, add
`if matches!(op, MemOp::Store | MemOp::Amo) { self.last_store =
Some((pa, size)); }`. `pa` is in scope from `access_bus`.

**MachineBuilder seam** — trait at `cpu/core.rs`; `CPU::from_config`
binds via the `CoreBuilder` alias (R-027). Body:
`let (cores, bus) = <CoreBuilder as MachineBuilder>::build(config,
layout); debug_assert_eq!(bus.num_harts(), cores.len()); …`. Call
sites (`xdb/src/main.rs`, tests) use `CPU::from_config(config,
layout)` with no turbofish.

**Test-fixture template** (TR-10) — migrate 8 `arch/riscv/cpu` test
modules to:

```rust
fn setup_core_and_bus() -> (RVCore, Bus) {
    let bus = Bus::new(0x80000000, 0x1000_0000, 1);
    let core = RVCore::with_id(HartId(0), IrqState::new());
    (core, bus)
}
```

Sites: `mm.rs`, `mm/{sv39,sv48,pmp}.rs`, `inst/atomic.rs`,
`inst/base.rs`, `csr.rs`, `trap/handler.rs` (`grep -l 'setup_core'`).

**UART** (CC-5): reader thread at `device/uart.rs:94` untouched; all
harts share the single UART via MMIO.

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
- **I-7** `arch_isolation` passes after a **declared** two-line
  allow-list widening: `SEAM_FILES` gains `"src/cpu/core.rs"`;
  `SEAM_ALLOWED_SYMBOLS` gains `"HartId"`. `BUS_DEBUG_STRING_PINS`
  unchanged. `HartId` is *defined* at `cpu/core.rs` as
  `pub struct HartId(pub u32);` — no `pub use` from arch.
- **I-8** `RVCore::checked_write` post-condition: on `Ok`, if `op ∈
  {Store, Amo}` then `self.last_store == Some((pa, size))`. `CPU::step`
  consumes via `take_last_store()` **before advancing `self.current`**
  (R-028 ordering), and on `Some` invokes
  `invalidate_reservations_except(self.current, addr, size)`. Covers
  store_op, fstore_op, sc_w, sc_d, all 18 AMOs via the funnel at
  `mm.rs:306-326`.
- **I-9** `Bus::num_harts()` returns the `num_harts` passed to
  `Bus::new`; `CPU::from_config` asserts
  `debug_assert_eq!(bus.num_harts(), cores.len())`.
- **I-10** (new, CC-7) Each `RVCore` owns its `reservation:
  Option<usize>` exclusively; only `CPU::invalidate_reservations_
  except` and the owning core itself may mutate it. Under NG-2 this
  is statically enforced by the borrow-checker (`&mut cores[i]` is
  disjoint from `&mut cores[j]` in the invalidation loop).
- **I-11** (new, CC-1 / CC-8) Each hart's `mip` / `sip` state lives in
  exactly one `IrqState` — the one on `RVCore`. `Mswi.irqs[h]`,
  `Mtimer.irqs[h]`, `Plic.irqs[h]` are `.clone()`s of the same `Arc`,
  producing strictly-equal `.load()` results. `Relaxed` ordering is
  sufficient under NG-2 (one writer per hart at any point; tick and
  step are temporally disjoint on the same thread).
- **I-12** (new, CC-2) Each hart's SSIP edge signal lives in
  `ssip_pending[h]` exclusively; `Sswi.ssip[h]` is a clone. Cross-hart
  SETSSIP (normal op) writes to `ssip[target_hart = offset / 4]` — not
  `current_hart` — honouring RISC-V ACLINT spec.
- **I-13** `CoreOps::step` is the *only* method that receives
  `&mut Bus`. Core-internal methods that need the bus thread it from
  `step`; no `Core` stores a `Bus`.

[**Data Structure**]

Shapes in §Architecture. Delta vs today:

- `cpu/core.rs`: `#[derive(Clone, Copy, Debug, PartialEq, Eq,
  PartialOrd, Ord, Hash)] pub struct HartId(pub u32);` +
  `MachineBuilder` trait.
- `cpu/mod.rs`: `#[cfg(riscv)] pub type CoreBuilder =
  crate::arch::riscv::cpu::RVCore;` (+ `loongarch` sibling).
- `RVCore`: `+id: HartId`, `+last_store: Option<(usize, usize)>`,
  `-bus: Bus`.
- `CPU<Core>`: `core: Core` → `cores: Vec<Core> + bus: Bus + current:
  usize`.
- `Bus`: `ssip_pending: Arc<AtomicBool>` → `Vec<Arc<AtomicBool>>`;
  `+num_harts: usize`.
- `Plic` (PR2a): drop `NUM_CTX` / `CTX_IP`; `+num_ctx: usize`,
  `+irqs: Vec<IrqState>`; `enable` / `threshold` / `claimed` become
  `Vec<_>`.
- `MachineConfig`: `+num_harts: usize` (default 1).
- `pub(in crate::arch::riscv) const RESERVATION_GRANULE: usize = 8;`.

[**API Surface**]

```rust
// cpu/core.rs
pub struct HartId(pub u32);  // newtype; derives per §Data Structure

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

// arch/riscv/cpu/mm.rs — bus threaded through 8 methods:
// checked_read, checked_write, fetch, load, store, amo_load,
// amo_store, translate — each gains `bus: &mut Bus` first-param.

// cpu/mod.rs
impl<Core: CoreOps + DebugOps> CPU<Core>
where CoreBuilder: MachineBuilder<Core = Core>
{
    pub fn new(cores: Vec<Core>, bus: Bus, layout: BootLayout) -> Self;
    pub fn from_config(config: MachineConfig, layout: BootLayout) -> Self;
    pub fn step(&mut self) -> XResult;   // external API unchanged
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
}

// device/bus.rs
impl Bus {
    pub fn new(ram_base: usize, ram_size: usize, num_harts: usize) -> Self;
    pub fn num_harts(&self) -> usize;
    pub fn ssip_flag(&self, hart: HartId) -> Arc<AtomicBool>;
    pub fn take_ssip(&self, hart: HartId) -> bool;
}

// arch/riscv
impl RVCore {
    pub fn new() -> Self;                             // id = HartId(0)
    pub fn with_id(id: HartId, irq: IrqState) -> Self;
}
impl Mswi   { pub(super) fn new(irqs: Vec<IrqState>) -> Self; }
impl Mtimer { pub(super) fn new(irqs: Vec<IrqState>) -> Self; }
impl Sswi   { pub(super) fn new(ssips: Vec<Arc<AtomicBool>>) -> Self; }
impl Aclint {
    pub fn new(irqs: Vec<IrqState>, ssips: Vec<Arc<AtomicBool>>) -> Self;
    pub fn install(self, bus: &mut Bus, base: usize) -> usize;
}
impl Plic { pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self; }
```

[**Constraints**]

- **C-1** `num_harts ∈ [1, 16]`.
- **C-2** MMIO layout (MSWI/MTIMER/SSWI base + stride) invariant.
- **C-3** `HartId` is a newtype over `u32` defined at `cpu/core.rs`
  (trait-seam layer); no RISC-V semantics leak.
- **C-4** Round-robin = declaration order (`cores[0..N]` cycle).
- **C-5** PR1 / PR2a do not modify DTBs; PR2b adds `xemu-2hart.dts`.
- **C-6** No new crate dependencies.
- **C-7** Plan body ≤ **720** lines (carried from R-025(a)). Round 05
  target ≤ 700 to preserve headroom (R-029).
- **C-8** (new) Concurrency baseline: every shared-state write path
  is justified per Concurrency Matrix against NG-2; `Relaxed` atomic
  ordering permissible under NG-2; future-MT-escalation note
  recorded per CC-x.
- **C-9** `DebugOps` signatures unchanged across all three PRs.
- **C-10** `CoreOps::step` signature changes in PR1 (bus parameter);
  `CoreOps::{bus, bus_mut}` removed in PR1; unchanged after PR1.
- **C-11** At `num_harts == 1` every `.step()`-calling test remains
  byte-identical in observable outcome.

---

## Implement

### Execution Flow

[**Main Flow — PR1**] Bus-pivot + per-core state at `num_harts = 1`:

1. Define `pub struct HartId(pub u32);` at `xcore/src/cpu/core.rs`
   (trait-seam layer; no re-export). Add `"src/cpu/core.rs"` to
   `SEAM_FILES` and `"HartId"` to `SEAM_ALLOWED_SYMBOLS` in
   `xcore/tests/arch_isolation.rs` (R-026).
2. Extend `RVCore`: `+id: HartId`, `+last_store: Option<(usize,
   usize)>`; `-bus: Bus`. Add `RVCore::with_id(id, irq)`; keep
   `new()` (id = `HartId(0)`).
3. Extend `CoreOps` per §API Surface; `impl CoreOps for RVCore`.
4. Thread `bus: &mut Bus` through 8 `mm.rs` methods; callers in
   `inst/{base,atomic,float,privileged,compressed,zicsr,mul}.rs` and
   `trap/handler.rs` propagate via `execute(inst, bus) →
   dispatch(inst, bus) → op methods`. Mechanical.
5. Apply R-020 hook in `RVCore::checked_write` (see §Architecture).
6. `Bus::new(ram_base, ram_size, num_harts)`; `ssip_pending` becomes
   `Vec<Arc<AtomicBool>>`; `take_ssip(HartId)` / `ssip_flag(HartId)`
   bounds-checked; add `num_harts()`. (CC-2)
7. ACLINT sub-devices widen to per-hart `Vec` state. Decode
   `hart = offset / stride`; out-of-range reads 0, writes drop. (CC-1,
   CC-3)
8. Delete hard-coded `mhartid = 0` at `csr.rs:250`; `RVCore::with_id`
   seeds `csr.set(CsrAddr::mhartid, id.0 as Word)`.
9. Add `MachineBuilder` trait at `cpu/core.rs`; impl for `RVCore` in
   `arch/riscv/cpu/mod.rs` — body mirrors today's `with_config` at
   `mod.rs:58-90`, builds `num_harts` IrqStates + SSIP flags,
   `Aclint::new(irqs, ssips)`, `Plic::new(irqs[0].clone())` for now
   (PR2a converts), pushes cores via `RVCore::with_id(HartId(i),
   irqs[i].clone())`, returns `(cores, bus)`. Stub impl for `LACore`.
10. Add `pub type CoreBuilder` at `cpu/mod.rs`; rewrite
    `CPU::from_config` per §Architecture (no method generic).
11. Rewrite `CPU<Core>`: `core: Core` → `cores: Vec<Core> + bus: Bus
    + current: usize`. Rewrite `load_direct` / `load_firmware` /
    `load_file_at` / `replace_device` / `bus_take_mmio_flag` /
    DebugOps delegates via `self.bus` or `self.cores[self.current]`.
    Add `split_current_mut`, `invalidate_reservations_except`, `bus`,
    `bus_mut`, `current`, `current_mut`. Rewire `CPU::step` per
    §Architecture. External signature unchanged; `xdb/src/cmd.rs:37`
    and `xdb/src/difftest/mod.rs:56` unchanged.
12. Move `bus.tick()` from `RVCore::step` (was `mod.rs:225`) to
    `CPU::step` (before `core.step(bus)`). Per-hart per-instruction
    `csr.set(time, …)` / `take_ssip(id)` / `sync_interrupts` continue
    inside `RVCore::step` against the threaded `bus`.
13. Add `setup_core_and_bus()` helper; migrate 8 test modules
    (~60 call sites: `core.step()` → `core.step(&mut bus)`;
    `core.bus.write` → `bus.write`). Mechanical (TR-10).

[**Main Flow — PR2a**] PLIC runtime-size at `num_harts = 1`:

14. Rewrite `arch/riscv/device/intc/plic.rs`: delete `NUM_CTX`,
    `CTX_IP`; add `num_ctx: usize`, `irqs: Vec<IrqState>`.
    `Plic::new(num_harts, Vec<IrqState>)` sets `num_ctx = 2 *
    num_harts`; `enable`/`threshold`/`claimed` become `Vec<_>`.
    `evaluate` iterates `0..num_ctx`; `ip_bit = if ctx & 1 == 0
    { MEIP } else { SEIP }`; target `self.irqs[ctx >> 1]` (CC-8).
    `ctx_at`/`complete` use `self.num_ctx`. 14 existing PLIC tests
    pass unchanged with `Plic::new(1, vec![irq.clone()])` (V-IT-6).
    +1 new V-UT-10.
15. Update `MachineBuilder::build` site: `Plic::new(irq.clone())` →
    `Plic::new(config.num_harts, plic_irqs.clone())`.

[**Main Flow — PR2b**] Activate `num_harts > 1`:

16. `MachineConfig::with_harts(n)` builder; `debug_assert!((1..=16).
    contains(&n))` (C-1).
17. Firmware boot seeding in `RVCore::setup_boot`: `a0 = self.id.0`,
    `a1 = fdt_addr`; non-zero cores start `halted = true`; ACLINT
    MSIP releases them per OpenSBI HSM (NG-8).
18. `reset` in `CoreOps for RVCore` clears `reservation`,
    `last_store`, `halted`; `CPU::reset` resets all cores + bus
    devices.
19. `X_HARTS` env var in `xemu/xdb/src/main.rs::machine_config`,
    mirroring `X_DISK` shape at `main.rs:45-53` (R-013):
    `match env("X_HARTS") { Some(s) => s.parse::<usize>().
    map_err(|e| anyhow!("X_HARTS must be a usize: {e}"))?, None =>
    1 }`. Threaded via `MachineConfig::with_harts(num_harts)`.
20. Add `resource/xemu-2hart.dts` (clone of `xemu.dts` with `cpu1` +
    `cpu-map cluster0/core1`; both cores feed `clint@2000000` /
    `plic@c000000`). `resource/Makefile` gains `xemu-2hart.dtb` rule
    and `linux-2hart` / `debian-2hart` targets.
21. Difftest driver asserts `num_harts == 1` at setup (NG-3, CC-6).

[**Failure Flow**]

1. `hart >= num_harts` in sub-device MMIO: read 0, write drops.
2. `num_harts = 0` or `> 16`: `debug_assert!` at
   `MachineConfig::with_harts`.
3. OpenSBI fails to bring core 1 online (PR2b): dmesg shows 1 CPU;
   V-IT-5 fails.
4. PLIC routing mis-wired in PR2a: caught by V-UT-10.
5. I-8 violation (cross-hart SC succeeds after peer store via Store,
   Amo, or FP path): caught by V-UT-11/13/14.
6. Difftest divergence at `num_harts > 1`: unsupported; driver asserts
   (CC-6).
7. `X_HARTS` parse failure: `anyhow::Error` mirrors `X_DISK` style.
8. Borrow-checker split in `CPU::step` (`&mut cores[current]` vs
   `&mut self.bus`): resolved by `split_current_mut`'s destructuring
   pattern; compile-time gate (V-F-7).
9. R-028 ordering violation (future refactor swaps `take_last_store`
   and `self.current +=`): caught by V-UT-11/13 at `num_harts ≥ 2`;
   `debug_assert!(src < self.cores.len())` traps wildly-wrong `src`.
10. CC-4 residual: hart-N panic poisons `XCPU` Mutex. Known limitation
    carried from today's single-hart emulator; outside scope.

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
- `cargo test --test arch_isolation` passes (with the widened
  allow-list).
- `make linux` → `buildroot login:` ≤ 60 s; `make debian` → login +
  Python3 ≤ 120 s.
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
- Difftest pinned to `num_harts == 1` (CC-6).

---

## Trade-offs

- **T-1** Round-robin in `CPU::step` over N-burst (starvation) /
  skip-halted (breaks SBI HSM).
- **T-2..T-11** Carried from round 04: AoS `Vec<RVCore>`; 3-PR split;
  debug-UX via `cores[current]`; per-hart `take_ssip(HartId)`;
  mm-layer `checked_write` hook; `last_store` on core / `.take()` on
  CPU; `X_HARTS` env var; op-gated callee-record; `bus.tick` once-
  per-cycle in `CPU::step`; bus threaded by parameter (no unsafe).
- **T-12** (R-026) `HartId` at `cpu/core.rs` over arch re-export;
  declared two-token allow-list widening preferred over silent drift.
- **T-13** (R-027/TR-9) `CoreBuilder` alias over `<B>` method generic
  (turbofish at every call site rejected).
- **T-14** (CC-4) `XCPU` Mutex poison — known single-hart limitation;
  `parking_lot` rejected (new dep violates C-6; orthogonal to pivot).
- **T-15** (CC-10) `Arc` over `Rc` — Send-ready; zero refactor on
  future MT; uncontended `fetch_add` overhead negligible.

---

## Validation

[**Unit Tests — PR1 (11 new `#[test]` functions)**]

| # | Test function | File | Purpose |
|---|---------------|------|---------|
| V-UT-3 | `mswi_four_harts_msip2_raises_only_irq2` | `device/intc/aclint/mswi.rs` | G-3, I-3, I-5 |
| V-UT-4 | `mtimer_two_harts_mtimecmp0_fires_only_irq0` | `device/intc/aclint/mtimer.rs` | G-3, I-5, CC-3 |
| V-UT-5 | `sswi_three_harts_setssip1_raises_only_ssip1` | `device/intc/aclint/sswi.rs` | G-3, I-3, I-12 |
| V-UT-6 | `bus_new_four_harts_ssip_vec_len_and_share` | `device/bus.rs` | G-6, I-1, I-9, I-12 |
| V-UT-7 | `machine_config_default_num_harts_is_one` | `config/mod.rs` | G-4 |
| V-UT-9 | `cores_ids_match_index` | `cpu/mod.rs::tests` | I-2 |
| V-UT-11 | `cross_core_lr_sw_sc_invalidation` | `cpu/mod.rs::tests` | I-8 via store_op, I-10, CC-7 |
| V-UT-12 | `same_core_store_keeps_other_reservation` | `cpu/mod.rs::tests` | I-8 `src` skip |
| V-UT-13 | `amo_invalidates_peer_reservation` | `cpu/mod.rs::tests` | I-8 via amo_store |
| V-UT-14 | `fsw_invalidates_peer_reservation` | `cpu/mod.rs::tests` | I-8 via fstore_op |
| V-UT-15 | `cpu_step_advances_current_single_hart` | `cpu/mod.rs::tests` | G-5 single-hart (R-030) |

V-UT-1/V-UT-2 (round-03 `hart_new_…`/`hart_reset_…`) fold into
existing `RVCore::new`/`reset` tests as added assertions. V-UT-8 is
the pass-through ACLINT test in the 354 baseline.

[**Unit Tests — PR2a (1 new `#[test]`)**]

- **V-UT-10** `plic_new_num_harts_two_ctx2_routes_to_irq1` in
  `device/intc/plic.rs` (G-8, I-5, CC-8).
- **V-IT-6** 14 existing PLIC tests pass unchanged with
  `Plic::new(1, vec![irq.clone()])`. Zero-regression gate.

[**Unit / Integration Tests — PR2b (3 new `#[test]`)**]

- **V-IT-2** `plic_2hart_context_map` in `tests/` (G-9, I-5).
- **V-IT-4** `round_robin_fairness_two_harts` in `tests/` (G-5, C-4).
- **V-IT-5** `smp_linux_smoke` in `tests/` (ignored by default;
  `X_HARTS=2`) — G-9 end-to-end.
- **V-IT-1** (existing) `arch_isolation` — passes with widened
  allow-list (I-7).

[**Failure / Robustness Validation**]

- **V-F-1** `num_harts = 0` or `> 16` → `debug_assert!` (C-1).
- **V-F-2** MMIO write to `MSIP[num_harts]` silently drops.
- **V-F-3** MTIMER read at `mtimecmp[h]` for `h >= num_harts` → 0.
- **V-F-4** `CPU::reset` post-condition: every core's `pc ==
  RESET_VECTOR`, `reservation.is_none()`, `last_store.is_none()`.
- **V-F-5** (PR2b) OpenSBI brings only core 0 online: dmesg shows 1
  CPU; V-IT-5 fails.
- **V-F-6** (PR2b) `make linux-2hart` > 120 s: V-IT-5 fails.
- **V-F-7** (PR1 compile-time) `cargo build --tests` fails iff
  `split_current_mut` doesn't produce disjoint borrows.
- **V-F-8** (PR1) `debug_assert!` in `invalidate_reservations_except`
  traps `src >= self.cores.len()` (R-028).

[**Edge Case Validation**]

- **V-E-1** `num_harts = 1` byte-identical (I-4, C-11): all 354
  existing lib tests pass; `make debian` boot-to-Python3 trace
  identical (timing excluded).
- **V-E-2** Offset decode at `num_harts = 3`: MSWI accepts offsets {0,
  4, 8}; `offset = 12` reads 0.
- **V-E-3** Round-robin wraparound at `num_harts = 2` (`current`
  advances `0 → 1 → 0 → 1 …`).
- **V-E-4** (PR2b) hartid seeding: `cores[0].gpr[a0] == 0`,
  `cores[1].gpr[a0] == 1`; `mhartid` CSR matches.
- **V-E-5** (PR1) `store_overlapping_granule_invalidates`: LR.D
  `0x80001000`; peer `sw 0x80001004`; SC.D fails. In V-UT-11.
- **V-E-6** (PR1) `store_outside_granule_preserves`: LR.W
  `0x80001000`; peer `sw 0x80001010`; SC.W succeeds. In V-UT-11/12.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (HartId + RVCore.id/last_store) | existing `RVCore::new` + `reset` tests (rebased), V-UT-9 |
| G-2 (CPU shape) | V-UT-15, V-IT-1, PR1 gate matrix |
| G-3 (ACLINT per-hart) | V-UT-3, V-UT-4, V-UT-5 |
| G-4 (MachineConfig::num_harts) | V-UT-7, V-F-1 |
| G-5 (round-robin in CPU) | V-UT-15, V-IT-4, V-E-3 |
| G-6 (per-hart SSIP) | V-UT-5, V-UT-6 |
| G-7 (PR1 behaviour-preservation) | V-E-1, PR1 gate (372 tests) |
| G-8 (PR2a PLIC reshape) | V-UT-10, V-IT-6, PR2a gate (373 tests) |
| G-9 (PR2b SMP boot) | V-IT-5, V-E-4, V-F-5, V-F-6 |
| G-10 (cross-hart LR/SC via checked_write post-condition) | V-UT-11..14, V-F-8 |
| G-11 (concurrency posture documented) | Concurrency Matrix + I-10/I-11/I-12 + C-8 |
| C-1 (hart count bounds) | V-F-1 |
| C-2 (MMIO layout) | V-UT-3..5, V-E-2, V-IT-6 |
| C-3 (HartId seam at cpu/core.rs) | V-IT-1 with widened allow-list |
| C-4 (deterministic order) | V-UT-15, V-IT-4, V-E-3 |
| C-5 (DTB untouched PR1/PR2a) | PR1/PR2a gate matrices |
| C-6 (no new deps) | Cargo.lock diff review per PR |
| C-7 (≤ 720 lines) | `wc -l 05_PLAN.md` at plan-review |
| C-8 (concurrency baseline) | Concurrency Matrix row coverage |
| C-9 (DebugOps unchanged) | V-IT-1 + xdb 6-test suite |
| C-10 (CoreOps::step signature PR1 only) | PR1/PR2a/PR2b gates |
| C-11 (byte-identical at num_harts==1) | V-E-1, V-IT-6 |
| I-1..I-3 | V-UT-6, V-UT-7, V-UT-9, V-UT-3..5, V-E-2, V-UT-10 |
| I-4 | V-E-1, V-IT-6 |
| I-5 | V-UT-3, V-UT-4, V-UT-10, V-IT-2 |
| I-6 | `RVCore::with_id` test, V-E-4 |
| I-7 | V-IT-1 (allow-list widened by two tokens) |
| I-8 | V-UT-11..14, V-F-8 |
| I-9 | V-UT-6 + `debug_assert_eq!` in `from_config` |
| I-10 | V-UT-11..14 (peer-reservation mutation isolation) |
| I-11 | V-UT-3, V-UT-4, V-UT-10 (per-hart IrqState cloning) |
| I-12 | V-UT-5, V-UT-6 (per-hart ssip_pending) |
| I-13 | grep `&mut Bus` on `cpu/core.rs` at PR1 review |
| CC-1..CC-10 | Concurrency Matrix + Unresolved (CC-4) |
