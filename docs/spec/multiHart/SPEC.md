# `multiHart` SPEC

> Source: [`/docs/archived/feat/multiHart/07_PLAN.md`](/docs/archived/feat/multiHart/07_PLAN.md).
> Iteration history, trade-off analysis, and implementation
> plan live under `docs/archived/feat/multiHart/`.

---


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
