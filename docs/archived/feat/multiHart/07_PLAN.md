# `multiHart` PLAN `07`

> Status: Draft
> Feature: `multiHart`
> Iteration: `07`
> Owner: Executor
> Depends on:
> - Previous Plan: `06_PLAN.md` (APPROVE with Revisions)
> - Review: `06_REVIEW.md`
> - Master Directive: `04_MASTER.md` (empty; 01-M-002 binding)

---

## Summary

Shares `Bus` via `Arc<Mutex<Bus>>` (one clone per hart) — mandatory
today because `pub static XCPU: OnceLock<Mutex<CPU<Core>>>` at
`xemu/xcore/src/cpu/mod.rs:50` requires `CPU<Core>: Send`, which
`Rc<RefCell<_>>` breaks. Reservations move from `RVCore` to `Bus`.
Every physical store goes through `Bus::store(HartId, addr, size,
val)` — write + peer-reservation invalidation under one
`lock().unwrap()` (atomic under NG-2). Eliminates `split_current_mut`,
`MachineBuilder`, `step` signature change, 8-method `mm.rs` threading,
`last_store`, `invalidate_reservations_except`. Three PRs: PR1 pivot
at `N=1`; PR2a PLIC runtime-size at `N=1`; PR2b activates `N>1`.
Tests: **364→365→368 lib + 1 + 6 = 371/372/375**.

## Log

`Arc<Mutex<Bus>>` shared ownership; reservations on `Bus`;
`Bus::store` chokepoint; `CoreOps::step` unchanged. R-034
(CRITICAL) resolved: `Rc→Arc`, `RefCell→Mutex`. R-035 reconciled:
354 baseline + 10 new = 364 lib at PR1. R-036: multiHart is
`cfg(isa64)`-only; size ∈ {1,2,4,8}, 8-byte only under
`cfg(isa64)`. R-037: `bus.tick()` once per `CPU::step` before
`cores[current].step()` (I-12); `CPU::reset` order = bus first,
then cores. Master 01-M-002 binding; inherited 00-M-001/002 +
01-M-001/003/004. **Unresolved**: CC-4 (`XCPU: Mutex<CPU>` poison)
KNOWN-LIMITATION (NG-6).

**Carry-forward from 06**: per-hart IrqState Vec, ACLINT sub-device
Vec, PLIC runtime-size, X_HARTS, xemu-2hart.dtb, 3-PR split, R-011
AMO/FP store path, R-016 8-byte granule, R-022 14 PLIC tests
unchanged, I-8 grep audit. Removed: `split_current_mut`,
`MachineBuilder`, `CoreBuilder`, `step(&mut self, &mut Bus)`,
8-method `mm.rs` threading, `last_store`,
`invalidate_reservations_except`, `setup_core_and_bus()`,
`CoreOps::{bus, bus_mut}`, NG-7 "future MT promo" framing (retired).

### Response Matrix

| Source | Decision |
|--------|----------|
| R-034 CRITICAL `Rc<RefCell<Bus>>` is `!Send`; breaks `XCPU: OnceLock<Mutex<CPU<Core>>>` | Accept. `Rc<RefCell<Bus>>` → `Arc<Mutex<Bus>>`; `.borrow()/.borrow_mut()` → `.lock().unwrap()`. Updates G-1/2/3, API Surface, Architecture `CPU::step`, store paths, F-8, CC-4/7/10, T-4; NG-7 retired. `std::sync` only — C-6 preserved. `Mutex<Bus>: Send+Sync` since `Bus: Send` (all fields `Send` given `Device: Send` bound). Uncontended lock ~15 ns ⟹ ~1.5% overhead vs ~100 ns dispatch. `.unwrap()` idiomatic (poison = prior panic = fatal, NG-6). |
| R-035 LOW test-count arithmetic | Accept. Baseline **354** lib + 1 arch_isolation + 6 xdb = **361**. PR1 adds **10** net-new (V-UT-1..7, V-UT-10/11/12) → **364+1+6=371**. PR2a +V-UT-8 → **365+1+6=372**. PR2b +V-IT-1/2/3 → **368+1+6=375**. Existing `RVCore::new`/`reset` rebased in-place under `HartId(0)` (zero-delta, not new). |
| R-036 MEDIUM `Bus::store` value type | Accept. **C-13 multiHart PR1 is `cfg(isa64)`-only; `Word = u64` carries all store widths (sb/sh/sw/sd, FSD bit-pattern, AMO.D)**. `size ∈ {1,2,4,8}`; `size == 8` only under `cfg(isa64)`. FSD routes f64-bit-pattern as `Word`. `sc_w/sc_d` return 0/1 are GPR writes, not bus stores. |
| R-037 LOW `bus.tick()` + reset order | Accept. **I-12 `bus.tick()` runs once per `CPU::step` before `cores[current].step()`; at N>1 ticks N times per round, matching HW hart-cycle clocking**. `CPU::reset`: `bus.lock().unwrap().reset_devices(); bus.lock().unwrap().reservations.fill(None); for core in &mut self.cores { core.reset()?; }`. |
| TR-3 C-12 audit scope | Accept. C-12 extended: no `Bus::write` from `inst/{atomic,base,float}.rs` or `arch/*/cpu/mm*.rs`. |
| Inherited from 06_PLAN | All prior findings (R-001..R-033, TR-9/10; Master 00-M-001..01-M-004), carry-forward list, architecture shape, PR split, validation skeleton unchanged except as above. TR-9/10 moot. CC-4 KNOWN-LIMITATION. |

## Spec

[**Goals**]

- **G-1** `CPU<C> = { cores: Vec<C>, bus: Arc<Mutex<Bus>>, current, state, halt_pc, halt_ret, boot_config, boot_layout }`.
- **G-2** `RVCore`: `+id: HartId`; `+bus: Arc<Mutex<Bus>>`; `-reservation`.
- **G-3** `Bus`: `+num_harts`, `+reservations: Vec<Option<usize>>`, `ssip_pending: Vec<Arc<AtomicBool>>` (all len N).
- **G-4** `Bus::store(hart, addr, size, val)` = write + peer-invalidate under one `lock().unwrap()`.
- **G-5** `Bus::{reserve, reservation, clear_reservation}`.
- **G-6** `CoreOps::step(&mut self)` unchanged; `{bus, bus_mut}` → `CPU`.
- **G-7** `CPU::step`: tick bus, step `cores[current]`, handle, advance.
- **G-8** ACLINT sub-devices per-hart `Vec`; byte-identical at `N=1`.
- **G-9** `MachineConfig::num_harts: usize` (default 1).
- **G-10** `X_HARTS` + `xemu-2hart.dtb` + `linux-2hart`/`debian-2hart` (PR2b).
- **G-11** Cross-hart LR/SC correct by construction: every store is `Bus::store`.

[**Non-Goals**]

- **NG-1** PLIC gateway redesign. **NG-2** MT hart execution (single OS thread; shape MT-ready). **NG-3** Difftest at `N>1` (asserts `==1`). **NG-4** Per-hart xdb UX. **NG-5** TLB shootdown. **NG-6** XCPU Mutex poison recovery — `.unwrap()`; poison = prior panic = fatal. **NG-7** *Retired* (shape is `Arc<Mutex<_>>` today). **NG-8** No `Hart` struct.

[**Architecture**]

```
CPU<C>   cores: Vec<C>, bus: Arc<Mutex<Bus>>, current: usize, + today
RVCore   id, bus: Arc<Mutex<Bus>>, + today's fields (- reservation)
Bus      + num_harts, reservations: Vec<Option<usize>>,
         + ssip_pending: Vec<Arc<AtomicBool>> (len N each)
Mswi     msip: Vec<u32>,             irqs: Vec<IrqState>
Mtimer   mtime, mtimecmp: Vec<u64>,  irqs: Vec<IrqState>
Sswi     ssip: Vec<Arc<AtomicBool>>
Plic     num_ctx, enable: Vec<u32>, threshold: Vec<u8>,
         claimed: Vec<u32>, irqs: Vec<IrqState>        // PR2a; len N
```

`CPU::step`:

```rust
fn step(&mut self) -> XResult {
    self.bus.lock().unwrap().tick();
    let result = self.cores[self.current].step();
    if self.cores[self.current].halted() {
        self.set_terminated(State::Halted).log_termination();
    }
    result?;
    self.current = (self.current + 1) % self.cores.len();
    Ok(())
}
```

`RVCore::step` body at `mod.rs:223-260` kept verbatim; `self.bus.xxx()`
→ `.lock().unwrap().xxx()`. `bus.tick()` moves from `mod.rs:225` to
`CPU::step` (per I-12). Reservation flow: `lr_w/lr_d` →
`bus.lock().unwrap().reserve(id, addr)`; `sc_w/sc_d` → check
`reservation(id)`, on hit call `bus.store(...)` + `clear_reservation`
(ret 0) else 1; `store_op`/`fstore_op`/AMO commit →
`bus.lock().unwrap().store(id, addr, size, val)?`. Note: `sc_w/sc_d`
branch return 0/1 are GPR writes, not bus stores.

[**Invariants**]

- **I-1** `cores.len() == config.num_harts`.
- **I-2** `cores[i].id() == HartId(i as u32)`.
- **I-3** ACLINT decode `hart = offset / stride` (MSWI 4, MTIMER
  mtimecmp 8, SSWI 4); out-of-range read 0, write drop.
- **I-4** At `num_harts == 1`, byte-identical pre/post PR1 + PR2a.
- **I-5** IRQ routing: `Mswi.irqs[h].MSIP`, `Mtimer.irqs[h].MTIP`,
  `Plic.irqs[ctx >> 1]` with `ip = if ctx & 1 == 0 { MEIP } else { SEIP }`.
- **I-6** `mhartid` CSR = `self.id().0 as Word`; hard-coded `= 0` at
  `csr.rs:250` deleted.
- **I-7** `arch_isolation` passes after +2-token widening (`SEAM_FILES
  += "src/cpu/core.rs"`, `SEAM_ALLOWED_SYMBOLS += "HartId"`).
- **I-8** Every physical store routes through `Bus::store`; write +
  peer-invalidate under one `lock().unwrap()`.
- **I-9** `Bus::num_harts() == cores.len()` — `debug_assert_eq!` in `CPU::new`.
- **I-10** `Mswi/Mtimer/Plic.irqs[h]` are `.clone()` of same
  `Arc<AtomicU64>`; `Relaxed` under NG-2.
- **I-11** `ssip_pending[h]` = hart `h`'s SSIP edge; cross-hart
  SETSSIP writes `ssip[offset / 4]`.
- **I-12** `bus.tick()` runs once per `CPU::step` before
  `cores[current].step()`; at `N>1` ticks N times per round (one
  per scheduler tick per hart), matching real HW hart-cycle
  clocking. At `N=1` byte-identical to pre-pivot.

[**Data Structure**]

- `cpu/core.rs`: `pub struct HartId(pub u32)` with `Clone,Copy,Debug,
  PartialEq,Eq,PartialOrd,Ord,Hash`.
- `RVCore`: `+id`; `bus: Bus` → `Arc<Mutex<Bus>>`; `-reservation`.
- `CPU<C>`: `core: C` → `cores: Vec<C>`; `+bus`; `+current: usize`.
- `Bus`: `+num_harts`; `+reservations: Vec<Option<usize>>`;
  `ssip_pending: Arc<AtomicBool>` → `Vec<_>`.
- ACLINT sub-devices + `Plic`: per-hart `Vec` per §Architecture.
- `MachineConfig`: `+num_harts: usize` (default 1).
- `pub(in crate::arch::riscv) const RESERVATION_GRANULE: usize = 8;`

[**API Surface**]

```rust
// cpu/core.rs
pub struct HartId(pub u32);
pub trait CoreOps {
    fn id(&self) -> HartId;                     // NEW
    fn pc(&self) -> VirtAddr;
    fn reset(&mut self) -> XResult;
    fn setup_boot(&mut self, mode: BootMode);
    fn step(&mut self) -> XResult;              // unchanged
    fn halted(&self) -> bool;
    fn halt_ret(&self) -> Word;                 // -bus / -bus_mut
}
// cpu/mod.rs (CPU): bus()/bus_mut() -> MutexGuard<'_, Bus> (both
// via self.bus.lock().unwrap()); current()/current_mut();
// new(cores, bus, layout); from_config; step; run; pc;
// replace_device; #[cfg(difftest)] bus_take_mmio_flag.

// device/bus.rs
impl Bus {
    pub fn new(ram_base: usize, ram_size: usize, num_harts: usize) -> Self;
    pub fn num_harts(&self) -> usize;
    pub fn reserve(&mut self, hart: HartId, addr: usize);
    pub fn reservation(&self, hart: HartId) -> Option<usize>;
    pub fn clear_reservation(&mut self, hart: HartId);
    // size ∈ {1,2,4,8}; size == 8 only reachable under cfg(isa64).
    // FSD routes f64 bit-pattern reinterpreted as Word.
    pub fn store(&mut self, hart: HartId, addr: usize,
                 size: usize, val: Word) -> XResult;     // chokepoint
    pub fn ssip_flag(&self, hart: HartId) -> Arc<AtomicBool>;
    pub fn take_ssip(&self, hart: HartId) -> bool;
}

// arch/riscv
impl RVCore {
    pub fn new(bus: Arc<Mutex<Bus>>) -> Self;            // id=0
    pub fn with_id(id: HartId, bus: Arc<Mutex<Bus>>, irq: IrqState) -> Self;
}
impl Plic { pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self; }
```

`Bus::store` body — `self.write(addr, size, val)?` then iterate
`reservations` (skip `i == hart.0`); for each `Some(a)`, compute
`base = a & !(RESERVATION_GRANULE - 1)`; if `base < addr+size &&
base + RESERVATION_GRANULE > addr` set `*r = None`. Return `Ok(())`.

[**Constraints**]

- **C-1** `num_harts ∈ [1, 16]`; `debug_assert!`.
- **C-2** MMIO layout (MSWI/MTIMER/SSWI base+stride) invariant.
- **C-3** `HartId` at `cpu/core.rs`; no RISC-V leakage.
- **C-4** Round-robin = declaration order.
- **C-5** PR1/PR2a do not modify DTBs; PR2b adds `xemu-2hart.dts`.
- **C-6** No new crate deps. `Arc`, `std::sync::Mutex` are in `std`.
- **C-7** Plan body **≤ 400 lines**.
- **C-8** Concurrency per §Concurrency; `Relaxed` under NG-2.
- **C-9** `DebugOps` signatures unchanged.
- **C-10** `CoreOps::step` signature unchanged; trait delta
  `-bus/-bus_mut`, `+id`.
- **C-11** At `N=1` every `.step()`-calling test byte-identical.
- **C-12** `Bus::store` is the sole store chokepoint; PR1 grep
  audit confirms no direct `Bus::write` in
  `inst/{atomic,base,float}.rs` **or `arch/*/cpu/mm*.rs`**.
- **C-13** multiHart PR1 is `cfg(isa64)`-only; `Word = u64` carries
  all store widths. Under `cfg(isa32)` the 8-byte path is
  unreachable (enforced by current `cfg(riscv)` ⇒ `cfg(isa64)` shape).

## Implement

### Execution Flow

[**PR1 — shared-bus pivot at N=1**]

1. `cpu/core.rs`: define `HartId`; widen `arch_isolation.rs:28-65`
   allow-lists (`+"src/cpu/core.rs"`, `+"HartId"`).
2. `CoreOps`: `-bus/-bus_mut`, `+id`.
3. `device/bus.rs`: `Bus::new(ram_base, ram_size, num_harts)` with
   `num_harts`, `reservations` (len N), `ssip_pending` (len N); add
   §API accessors including `store`.
4. `RVCore`: `bus: Bus` → `Arc<Mutex<Bus>>`; `+id`; `-reservation`;
   rewrite `self.bus.xxx()` → `self.bus.lock().unwrap().xxx()`;
   add `with_id`.
5. Rewire LR/SC/AMO/store in
   `arch/riscv/cpu/inst/{atomic,base,float}.rs` per reservation flow.
6. ACLINT sub-devices: per-hart `Vec<_>`; decode `hart = offset/stride`.
7. Delete `mhartid = 0` at `csr.rs:250`; `with_id` seeds
   `csr.set(CsrAddr::mhartid, id.0 as Word)`.
8. Rewrite `CPU<C>`: `core: C` → `cores: Vec<C>`; `+bus`; `+current`.
   Delegates use `bus.lock().unwrap()` + `cores[current]`. `CPU::new`
   asserts `bus.lock().unwrap().num_harts() == cores.len()`.
   `CPU::step` 5 lines (see Architecture).
9. Rewrite `CPU::from_config` at `arch/riscv/cpu/mod.rs:58-90`: N
   IrqStates + SSIP flags, `Arc::new(Mutex::new(bus))`, clone `Arc`
   per `RVCore::with_id(HartId(i), arc.clone(), irqs[i])`; move
   `bus.tick()` from `mod.rs:225` → `CPU::step` (per I-12).
10. Update `setup_core()` once to produce `Arc<Mutex<Bus>>`.

[**PR2a — PLIC runtime-size at N=1**]

11. Rewrite `arch/riscv/device/intc/plic.rs`: drop `NUM_CTX`/`CTX_IP`; `+num_ctx`, `+irqs: Vec<IrqState>`. `Plic::new(num_harts, Vec<IrqState>)`; `num_ctx = 2*num_harts`. `evaluate` iterates `0..num_ctx`; `ip_bit = if ctx & 1 == 0 { MEIP } else { SEIP }`; target `self.irqs[ctx >> 1]`.

[**PR2b — activate N > 1**]

12. `MachineConfig::with_harts(n)`: `debug_assert!((1..=16).contains(&n))`.
13. `RVCore::setup_boot` seeds `a0 = id.0`, `a1 = fdt_addr`; non-zero
    cores start `halted = true`; ACLINT MSIP releases per OpenSBI HSM.
    `CPU::reset` order (R-037): `bus.lock().unwrap().reset_devices();
    bus.lock().unwrap().reservations.fill(None); for core in &mut
    self.cores { core.reset()?; }` — bus first, then each hart.
14. `X_HARTS` env in `xdb/src/main.rs::machine_config` mirroring
    `X_DISK` at `main.rs:43-54`: `env("X_HARTS").map(|s|
    s.parse::<usize>().map_err(|e| anyhow!("X_HARTS must be usize:
    {e}"))).transpose()?.unwrap_or(1)`.
15. `resource/xemu-2hart.dts` (clone of `xemu.dts` + `cpu1` + `cpu-map
    cluster0/core1`, feeding `clint@2000000`/`plic@c000000`); `Makefile`
    `+xemu-2hart.dtb`, `+linux-2hart`, `+debian-2hart`. Difftest
    asserts `N==1`.

[**Failure Flow**] (1) MMIO `hart >= num_harts`: read 0, write drop.
(2) `num_harts ∉ [1,16]`: `debug_assert!`. (3) PLIC mis-wired: V-UT-8.
(4) I-8 violation: V-UT-10..12. (5) Difftest N>1: asserts N=1.
(6) `X_HARTS` parse fail: `anyhow::Error` mirrors `X_DISK`.
(7) Core-1 bringup fail: V-IT-3. (8) `lock().unwrap()` poison = prior
panic = fatal (NG-6).

[**State Transition**]

- **S0** `CPU{core: RVCore}`; `RVCore` owns `Bus`; reservation on RVCore. **S0→S1 (PR1)** `CPU{cores: Vec(1), bus: Arc<Mutex<Bus>>, current: 0}`; reservations on `Bus`. Byte-identical. **S1→S2 (PR2a)** `Plic{num_ctx: 2, enable(2), irqs(1)}`. Identical. **S2→S3 (PR2b)** `X_HARTS=N`: all Vecs len N; `xemu-2hart.dtb`.

### Implementation Plan

- **Phase 1 (PR1, steps 1–10)** Gate: fmt/clippy; **371 tests** (354
  baseline + 10 new = **364 lib** + 1 arch_isolation + 6 xdb);
  arch_isolation widened; `make linux` ≤ 60s; `make debian` ≤ 120s;
  difftest unchanged.
- **Phase 2a (PR2a, step 11)** Gate: PR1 regression; **372 tests**
  (365 lib + 1 + 6; +V-UT-8); 14 existing PLIC tests unchanged.
- **Phase 2b (PR2b, steps 12–15)** Gate: PR2a at N=1 regression;
  **375 tests** (368 lib + 1 + 6; +V-IT-1/2/3); `X_HARTS=2 make
  linux-2hart` → `buildroot login:` ≤ 120s with `smp: Brought up 1
  node, 2 CPUs`; difftest pinned N=1.

### Concurrency

Single OS thread; `CPU::step` round-robin is the sole scheduler
(NG-2). Under NG-2 all rows are safe; shape is MT-ready today (not
future) via `Arc<Mutex<_>>`.

- CC-1 Per-hart `IrqState` Relaxed (MT→AcqRel)
- CC-2 `ssip_pending: Vec<Arc<AtomicBool>>` (MT→same)
- CC-3 `Mtimer::check_timer` tick-sole-writer (MT→CAS)
- CC-4 `XCPU: Mutex<CPU>` poison N-way — **KNOWN-LIMITATION** (NG-6);
  `.unwrap()` idiomatic since poison = prior panic = fatal
- CC-5 UART reader thread `uart.rs:94` untouched
- CC-6 Difftest `mmio_accessed` global → pin N=1 (NG-3)
- CC-7 `Bus::store` write+invalidate under one `lock().unwrap()`;
  `CPU::step` sole caller
- CC-8 PLIC `irqs[ctx >> 1]` CC-1-shaped (MT→AcqRel)
- CC-9 `Device: Send` preserved → `Bus: Send` → `Mutex<Bus>:Send+Sync`
  → `Arc<Mutex<Bus>>: Send+Sync` → `CPU<Core>: Send` → `XCPU: Sync` ✓
- CC-10 *Retired.* Shape is `Arc<Mutex<Bus>>` today; no future promo.

## Trade-offs

- **T-1** `Arc<Mutex<Bus>>` vs bus-threading vs unsafe split-borrows
  — ~1.5% overhead (uncontended lock ~15 ns vs ~100 ns dispatch);
  eliminates 8-method threading, `last_store`, `split_current_mut`.
- **T-2** Reservations on `Bus` vs `RVCore` — cross-hart invalidate
  is one in-struct call; matches L1-D.
- **T-3** `Bus::store` chokepoint vs mm-layer hook — physical-
  address boundary; store paths funnel naturally. C-12 extended to
  `mm*.rs` per TR-3.
- **T-4** `Arc<Mutex<_>>` mandated by `XCPU: Send` bound — not a
  choice. `Rc<RefCell<_>>` fails to compile (R-034).
  `std::sync::Mutex` preserves C-6; `parking_lot` rejected.
- **T-5** `CoreOps::step` unchanged vs `step(&mut self, &mut Bus)` —
  preserves every call site + test body; `.lock().unwrap()` is a
  four-char shift from `self.bus.xxx()`.
- **T-6** XCPU Mutex poison — single-hart limit (NG-6); `.unwrap()`
  idiomatic.

## Validation

[**Unit Tests — PR1 (10 new)**]

ACLINT at `num_harts=2` (`device/intc/aclint/{mswi,mtimer,sswi}.rs`):
- V-UT-1 `mswi_two_harts_msip1_raises_only_irq1`
- V-UT-2 `mtimer_two_harts_mtimecmp0_fires_only_irq0`
- V-UT-3 `sswi_two_harts_setssip1_raises_only_ssip1`

`device/bus.rs` at `num_harts=2`:
- V-UT-4 `bus_new_two_harts_vec_lengths_and_share`
- V-UT-10 `bus_store_invalidates_peer_reservation_in_granule`
- V-UT-11 `bus_store_preserves_peer_reservation_outside_granule`
- V-UT-12 `bus_store_preserves_same_hart_reservation`

`config/mod.rs` / `cpu/mod.rs::tests`:
- V-UT-5 `machine_config_default_num_harts_is_one`
- V-UT-6 `cpu_step_advances_current_single_hart` (modulo smoke; fairness at V-IT-2/V-E-3)
- V-UT-7 `hart_ids_match_index`

Existing `RVCore::new`/`reset` rebased in-place with `id == HartId(0)` (zero-delta; not counted).

[**PR2a (1 new) + regression**]
- V-UT-8 `plic_new_num_harts_two_ctx2_routes_to_irq1`
- V-IT-6 14 existing PLIC tests pass with `Plic::new(1, vec![irq.clone()])` (zero-regression).

[**PR2b Integration (3 new)**]
- V-IT-1 `plic_2hart_context_map`; V-IT-2 `round_robin_fairness_two_harts`; V-IT-3 `smp_linux_smoke` (ignored; `X_HARTS=2`).

[**Failure / Edge Cases**]

- **V-F-1** `num_harts ∉ [1,16]` → `debug_assert!`. **V-F-2** MMIO `MSIP[num_harts]` write drops. **V-F-3** MTIMER `mtimecmp[h >= num_harts]` reads 0. **V-F-4** `CPU::reset`: bus first → `reservations[..] == None` → `cores[i].pc == RESET_VECTOR`. **V-F-5/6** (PR2b) core-1 offline / `linux-2hart > 120s` → V-IT-3. **V-F-7** `lock().unwrap()` poison = prior panic = fatal (NG-6).
- **V-E-1** `N=1` byte-identical; `make debian` trace identical. **V-E-2** `N=3` MSWI accepts {0,4,8}; offset 12 → 0. **V-E-3** Round-robin `0→1→0→1…` at `N=2` (V-IT-2). **V-E-4** (PR2b) `cores[i].gpr[a0]==i`; `mhartid` matches. **V-E-5** hart-0 reserve 0x80001000 + hart-1 store 0x80001004 → hart-0 reservation `None` (V-UT-10). **V-E-6** hart-0 reserve 0x80001000 + hart-1 store 0x80001010 → hart-0 reservation `Some(0x80001000)` (V-UT-11). **V-E-7** `bus.tick()` once per `CPU::step` (I-12); at N=2 tick-count == step-count (2 per round).

[**Acceptance Mapping**]

| Item | Validation |
|------|------------|
| G-1/2/7 | V-UT-4/6/7, V-IT-2, V-E-1/3/7 |
| G-3/4/5/11 | V-UT-4/10/11/12, V-E-5/6, C-12 audit |
| G-6, C-10 | PR1 diff review |
| G-8 | V-UT-1/2/3 |
| G-9 | V-UT-5, V-F-1 |
| G-10 | V-IT-1/3, V-E-4, V-F-5/6 |
| C-1/2 | V-F-1, V-UT-1..3, V-E-2, V-IT-6 |
| C-3/7 | arch_isolation widened; `wc -l` |
| C-4/11 | V-UT-6, V-IT-2, V-E-1/3, V-IT-6 |
| C-5/6/9 | gate matrices; Cargo.lock diff (empty); xdb 6-test |
| C-8, CC-1..10 | §Concurrency (CC-4 KNOWN-LIMITATION; CC-10 retired) |
| C-12 | PR1 grep audit on `inst/*` + `mm*.rs` |
| C-13 | `cfg(isa64)` build matrix; `size == 8` at V-UT-10 |
| I-1..3 | V-UT-1..5, V-E-2 |
| I-4/5 | V-E-1, V-IT-6, V-UT-1/2/8, V-IT-1 |
| I-6 | `with_id` test + V-E-4 |
| I-7 | widened allow-list test |
| I-8 | V-UT-10..12, V-E-5/6, C-12 audit |
| I-9 | V-UT-4 + `debug_assert_eq!` |
| I-10/11 | V-UT-1/2/3/8 |
| I-12 | V-E-7, V-UT-6 |
| NG-1..8 | PLIC untouched; 1 thread; difftest N=1; xdb current-only; no TLB; CC-4 carried; NG-7 retired; no `Hart` |
