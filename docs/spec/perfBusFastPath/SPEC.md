# `perfBusFastPath` SPEC

> Source: [`/docs/archived/perf/perfBusFastPath/03_PLAN.md`](/docs/archived/perf/perfBusFastPath/03_PLAN.md).
> Iteration history, trade-off analysis, and implementation
> plan live under `docs/archived/perf/perfBusFastPath/`.

---


[**Goals**]

- G-1: On every configuration (1-hart, 2-hart, N-hart), `CPU::step`
  and every per-instruction memory access (`checked_read`,
  `checked_write`, `access_bus`, AMO / LR / SC, `Bus::tick`) executes
  with **zero** `pthread_mutex_*` calls on the hot path. The bus is
  accessed via a direct `&mut Bus` borrow threaded through the call
  chain from `CPU::step`.
- G-2: Wall-clock runtime of `make run` (dhrystone, coremark,
  microbench, DEBUG=n) drops by **at least 15 %** vs.
  `docs/perf/baselines/2026-04-14/data/bench.csv`; **expected 20-30 %**;
  ceiling <= 35 %. Bucket math: 2026-04-14 profile shows
  `pthread_mutex_lock` + `pthread_mutex_unlock` + PLT stubs at
  ~33-40 % of self-time. P1 removes all mutex work; the floor 15 %
  comes from the genuine OS `lock()` cost (uncontended pthread is
  ~20-40 ns per acquire; ~2-3 acquires per guest instruction at
  tens of M instr/s); 35 % is the ceiling in which **all** of the
  bucket disappears AND adjacent cache lines get warmer.
- G-3: Multi-hart semantics are preserved bit-for-bit. Guest-visible
  ordering is identical: `CPU::step` still calls `bus.tick()` once
  before advancing the current hart; `advance_current()` still
  round-robins; LR/SC reservations still live on `Bus` and are
  checked+cleared under the same borrow scopes (I-4). The borrow
  changes from `MutexGuard<Bus>` to `&mut Bus`; both give exclusive
  access; the borrow checker replaces the mutex as the exclusion
  primitive (see I-4 peer-hart clause, R-004).
- G-4: LR/SC atomicity across translate -> reservation-read ->
  conditional-store is preserved. Today these sites hold one
  `MutexGuard` across all three operations; P1 holds one `&mut Bus`
  borrow across the same three operations. No widening, no
  narrowing.
- G-5: Public `CPU::bus()` signature is `&self -> &Bus` (was
  `&self -> MutexGuard<'_, Bus>`); method syntax on `&Bus` is
  identical to method syntax on `MutexGuard<Bus>` at the two in-tree
  callers (both `#[cfg(test)]`). `CPU::bus_mut(&mut self) -> &mut Bus`
  added for the difftest path. External-caller audit: zero
  `CPU::bus()` callers outside `xcore`.

- NG-1: No softmmu TLB (P2), no decoded-instruction cache (P4), no
  JIT. P1 only changes the bus ownership.
- NG-2: No per-instruction benchmark-aware branch pruning, no
  guest-PC specialisation, no skipping of UART / PLIC / MTIMER
  ticks.
- NG-3: No change to `Device::tick` / `Mmio::read/write` /
  `Device::reset` semantics or ordering.
- NG-4: No `unsafe` to bypass the borrow checker. The bus exclusion
  invariant is enforced on a plain `&mut Bus`; no `UnsafeCell`, no
  raw pointers, no `transmute::<&_, &mut _>`.
- NG-5: No new public crate-external API surface beyond the
  `CPU::bus_mut` addition. `CPU::bus()` return type changes from
  `MutexGuard<'_, Bus>` to `&Bus`; both deref / method-resolve
  identically for the two in-tree `#[cfg(test)]` callers.
- NG-6: No re-introduction of `Mutex`, `RwLock`, `parking_lot`,
  `RefCell`, or any synchronisation primitive around the bus. Future
  SMP lock shape is Phase 11's RFC decision.



[**Architecture**]

```
                       +--------------------------+
                       |           CPU            |
                       |  cores: Vec<Core>        |
                       |  bus: Bus                |  <- inline, no Mutex
                       +------------+-------------+
                                    |
                                    | CPU::step body MUST destructure
                                    | (I-10):
                                    |   let CPU { bus, cores,
                                    |             current, .. } = self;
                                    |   bus.tick();
                                    |   cores[*current].step(bus);
                                    v
                       +--------------------------+
                       |        Core::step        |
                       |   fn step(                |
                       |     &mut self,            |
                       |     bus: &mut Bus,        |  <- threaded in
                       |   ) -> XResult            |
                       +------------+-------------+
                                    |
                                    v
        +----------------------+    +----------------------+
        | access_bus(bus,...)  |    | checked_read(bus,...)|
        | checked_write(...)   |    | amo / lr / sc        |
        | debug::fetch_inst    |    | translate / mmu walk |
        +----------------------+    +----------------------+
                                    |
                                    v
                       +--------------------------+
                       | direct Bus method calls  |
                       |   bus.read(pa, size)     |
                       |   bus.store(id, ...)     |
                       |   bus.reserve(id, pa)    |
                       |   bus.clear_reservation  |
                       |   bus.tick()             |
                       +--------------------------+
```

Exclusion model: the cooperative round-robin scheduler enforces
that only one `Core::step` runs at a time per `CPU::step` call. That
scheduler is the single-borrower invariant; the `&mut Bus` borrow
the scheduler hands to the chosen core is the Rust-level
materialisation of that invariant. No second OS thread reaches the
bus (UART stdin reader owns its own mailbox; devices do not
back-call `CPU`; per I-9, closures passed into bus methods may not
reach back to `CPU`).

This is the same shape used by rvemu, rv8, Rare, riscv-rust, and
rrs (cited in 01_PLAN T-1 / R-008). It is also the shape
`docs/PROGRESS.md#phase-9-performance-optimization` P1 names directly ("Restore an owned `Bus` on
the hot path"); the DEV.md#phase-9-performance-optimization sketch floated a `BusHandle` enum
as one option, but the phase goal is the owned bus itself, not the
enum - 01_MASTER M-001 confirms the owned-bus reading.



[**Invariants**]

- I-1: `CPU::bus: Bus` (inline). Exactly one owner of the `Bus`
  instance. Constructed once in `CPU::new` / machine-factory code
  and never cloned or shared with any other struct.
- I-2: `RVCore` (and any future `CoreOps` implementor) has no `bus`
  field. Every method that needs bus access takes `bus: &mut Bus`
  or `bus: &Bus` as a parameter.
- I-3: For every `CPU::step` invocation, the call sequence is
  `bus.tick(); cores[*current].step(bus);` after a destructure of
  `self` (see I-10). The `&mut Bus` borrow lives for the duration
  of `Core::step` and is released before `advance_current()` runs.
- I-4: `sc_w` / `sc_d` / `amo_*` perform
  `translate -> reservation-check -> conditional-store` inside the
  **same** `&mut Bus` borrow scope (one function body). Holding
  the borrow across translate matches today's `access_bus` scope
  width; no widening, no narrowing.
  **Peer-hart exclusion (R-004):** Cooperative round-robin
  (`xemu/xcore/src/cpu/mod.rs:213-249`) means at most one hart
  executes per `CPU::step()`. The `&mut Bus` borrow held by
  `Core::step` is therefore the exclusion primitive that previously
  protected `bus.reservations[hart]`. LR/SC sequences in
  `xemu/xcore/src/arch/riscv/cpu/inst/atomic.rs` (`lr_w`, `lr_d`,
  `sc_w`, `sc_d`, AMO) are atomic w.r.t. peer harts because the
  cooperative scheduler does not preempt mid-instruction; the borrow
  checker replaces the mutex as the exclusion primitive for
  `bus.reservations[hart]`. Sufficient under the current
  single-threaded scheduler; Phase 11 true-SMP is out of scope (T-1).
- I-5: `Bus::tick` is called exactly once per `CPU::step`, before
  the current hart steps.
- I-6: `CPU::reset` calls `self.bus.reset_devices()` and
  `self.bus.clear_reservations()` before any core reset, matching
  the current order at `cpu/mod.rs:141-148`. Both are direct method
  calls on `&mut Bus`.
- I-7: Difftest behaviour unchanged: `Bus::mmio_accessed: AtomicBool`
  remains; `CPU::bus_mut().take_mmio_flag()` is the one-line
  replacement for the `self.bus.lock().unwrap().take_mmio_flag()`
  site. Receiver changes from `&self` to `&mut self`; the existing
  caller already holds `&mut CPU`, so this is source-compatible.
- I-8: No `unsafe`. No `Mutex`, `RwLock`, `Arc`, `parking_lot`,
  `UnsafeCell`, `RefCell`, or raw pointer is introduced or remains
  on the bus path after this commit.
- I-9: No method on `Bus` and no `Device::tick` body may call back
  into `CPU` (directly or transitively). Enforced by the borrow
  checker: `CPU::step` holds `&mut Bus` across `Core::step`, and
  `Device::tick`'s receiver (`&mut self` on the device, reached
  through the `&mut Bus`) cannot reach the outer `CPU` because the
  `CPU` is the borrow-exclusive owner of the bus and of the device
  tree by transitivity. A device that wanted to access the bus
  would need an inbound reference, which the type system does not
  provide.
- I-10 (NEW, R-002): Disjoint-field borrow discipline at `CPU::step`.
  `CPU::step`'s body MUST destructure `self` into disjoint borrows
  (e.g. `let CPU { bus, cores, current, .. } = self;`) before
  calling `cores[*current].step(bus)`. Helper methods on `&mut self`
  that access both `cores` and `bus` are FORBIDDEN: routing the
  same access through `self.bus_mut()` and `self.cores[...]` on the
  same `&mut self` would collapse the disjoint-field path and fail
  to compile (E0499). This invariant is the Rust-level reason the
  one-borrow-per-step pattern type-checks; pinned by a
  `compile_fail` doc-test (V-UT-7).



[**Data Structure**]

Changes to existing types (no new types introduced):

```rust
// xemu/xcore/src/cpu/mod.rs
pub struct CPU<Core: CoreOps> {
    cores: Vec<Core>,
    bus: Bus,                       // was: Arc<Mutex<Bus>>; now inline
    current: usize,
    state: State,
    halt_pc: VirtAddr,
    halt_ret: Word,
    boot_config: BootConfig,
    boot_layout: BootLayout,
    uart_line: Option<IrqLine>,
}

// xemu/xcore/src/arch/riscv/cpu.rs
pub struct RVCore {
    pub(in crate::arch::riscv) id: HartId,
    pub(in crate::arch::riscv) gpr: [Word; 32],
    pub(in crate::arch::riscv) fpr: [u64; 32],
    pub(in crate::arch::riscv) pc: VirtAddr,
    pub(in crate::arch::riscv) npc: VirtAddr,
    pub(in crate::arch::riscv) csr: CsrFile,
    pub(in crate::arch::riscv) privilege: PrivilegeMode,
    pub(in crate::arch::riscv) pending_trap: Option<PendingTrap>,
    // bus field DELETED (was: bus: Arc<Mutex<Bus>>)
    pub(in crate::arch::riscv) mmu: Mmu,
    pub(in crate::arch::riscv) pmp: Pmp,
    pub(in crate::arch::riscv) irq: IrqState,
    pub(in crate::arch::riscv) halted: bool,
    pub(in crate::arch::riscv) ebreak_as_trap: bool,
    pub(in crate::arch::riscv) breakpoints: Vec<Breakpoint>,
    pub(in crate::arch::riscv) next_bp_id: u32,
    pub(in crate::arch::riscv) skip_bp_once: bool,
}
```

**Why inline `Bus`, not `Box<Bus>` (R-003 resolution).**
`CPU` is constructed once in machine-factory code and lives the
entire emulation - there is no reuse pattern that benefits from
heap placement. `Bus` size is bounded by inspection:

- `Ram` carries a `Vec<u8>` (3 words: ptr + len + cap = 24 B on
  64-bit) plus a `usize` base.
- `Bus::devices` is a `Vec<MmioRegion>` (3 words header).
- `Bus::reservations` is a `Vec<Option<usize>>` (3 words header,
  contents proportional to `num_harts`; at typical N <= 4 the
  payload is negligible).
- `mmio_accessed: AtomicBool` and small bookkeeping fields.

Total `Bus` is ~100-200 bytes of header plus heap-pointed payload.
Inlining it into `CPU` saves one pointer hop on every
`bus.tick()` / `bus.read()` / `bus.store()` call on the hot path,
and keeps the working set adjacent in cache. `CPU` itself is
already a multi-hundred-byte struct (each `RVCore` is several
hundred bytes), so the size delta is a few percent and bounded by
V-UT-3 (`size_of::<Bus>() < 256`,
`size_of::<CPU<RVCore>>() < 4096`).

The `Box<Bus>` alternative (reviewer's TR-1 preference) would also
satisfy M-001 and is two lines of difference; we choose inline for
perf cleanliness. Nothing in the migration is harder under inline
than under `Box<Bus>`; the call sites are identical because
`Bus` is auto-deref'd in both shapes.

The `CoreOps` trait widens `step` by one parameter:

```rust
// xemu/xcore/src/cpu/core.rs  (existing trait)
pub trait CoreOps {
    fn reset(&mut self) -> XResult;
    fn step(&mut self, bus: &mut Bus) -> XResult;        // NEW param
    fn halted(&self) -> bool;
    fn halt_ret(&self) -> Word;
    fn pc(&self) -> VirtAddr;
    fn setup_boot(&mut self, mode: BootMode);
    // existing debug / inspect methods unchanged except for those
    // that need bus access (see DebugOps migration in Phase 2).
}
```

`RVCore` method signatures that lose `self.bus` and gain a `bus`
parameter (complete list; threaded through mechanically in the
single-commit migration):

```rust
impl RVCore {
    fn step(&mut self, bus: &mut Bus) -> XResult;
    fn fetch(&mut self, bus: &mut Bus) -> XResult<u32>;
    fn execute(&mut self, bus: &mut Bus, inst: DecodedInst) -> XResult;
    fn access_bus(&mut self, bus: &mut Bus, addr: VirtAddr,
                  op: MemOp, size: usize) -> XResult<usize>;
    fn checked_read(&mut self, bus: &mut Bus, addr: VirtAddr,
                    size: usize, op: MemOp) -> XResult<Word>;
    fn checked_write(&mut self, bus: &mut Bus, addr: VirtAddr,
                     size: usize, value: Word, op: MemOp) -> XResult;
    fn translate(&mut self, bus: &mut Bus, addr: VirtAddr,
                 size: usize, op: MemOp) -> XResult<usize>;
    fn store_op(&mut self, bus: &mut Bus, /* existing args */)
                -> XResult;
    // AMO / LR / SC handlers in arch/riscv/cpu/inst/atomic.rs all
    // take bus: &mut Bus.
    // Debug paths in arch/riscv/cpu/debug.rs take bus: &Bus.
}
```

`bus: &mut Bus` is used for methods that may mutate the bus
(`reserve`, `clear_reservation`, `store`, device ticks reachable
from `mmu.translate`'s PTE-access path). `bus: &Bus` is used for
strictly read-only debug paths (`debug::read_memory`,
`debug::fetch_inst`) and for the `mtime()` read in `RVCore::step`.



[**API Surface**]

Public surface on `CPU`:

```rust
impl<Core: CoreOps + DebugOps> CPU<Core> {
    pub fn new(cores: Vec<Core>, bus: Bus,
               layout: BootLayout) -> Self;
    // was: new(cores, bus: Arc<Mutex<Bus>>, layout)

    pub fn bus(&self) -> &Bus;              // was: MutexGuard<'_, Bus>
    pub fn bus_mut(&mut self) -> &mut Bus;  // NEW (difftest + tests)

    pub fn step(&mut self) -> XResult;              // body rewritten
                                                    // per I-10
    pub fn reset(&mut self) -> XResult;             // body rewritten
    pub fn run(&mut self, count: u64) -> XResult;   // unchanged
    pub fn replace_device(&mut self, name: &str,
                          dev: Box<dyn Device>);
        // body: self.bus.replace_device(name, dev);

    #[cfg(feature = "difftest")]
    pub fn bus_take_mmio_flag(&mut self) -> bool;   // &self -> &mut self
        // body: self.bus.take_mmio_flag()
}
```

Concrete shape of the `CPU::step` body (I-10 exemplar):

```rust
pub fn step(&mut self) -> XResult {
    // I-10: destructure self into disjoint borrows so the borrow
    // checker sees `bus` and `cores[current]` as independent
    // places. A helper method on `&mut self` that accessed both
    // would fail to compile (E0499).
    let CPU { bus, cores, current, .. } = self;
    bus.tick();
    let result = cores[*current].step(bus);
    // ... existing post-step bookkeeping
}
```

External-caller audit (grep `\.bus\(\)` and `bus_take_mmio_flag`
across `/Users/anekoique/ProjectX`):

- `xemu/xcore/src/cpu/mod.rs` tests at lines ~461 and ~551:
  `cpu.bus().read(...)` and `cpu.bus().num_harts()` - both
  read-only, both `#[cfg(test)]`, both source-compatible with
  `&Bus`.
- No matches outside `xcore`. `xdb`, `xtool`, `xkernels`,
  `xemu/tests/` do not call `cpu.bus()`.
- `bus_take_mmio_flag` has one call site in `xemu/xdb` difftest
  glue under `#[cfg(feature = "difftest")]`; the caller already
  owns `&mut CPU`, so the `&self -> &mut self` change is
  source-compatible. Verified by
  `cargo check -p xdb --features difftest` in Phase 3.

Call-site migration pattern (representative):

```rust
// BEFORE                                   // AFTER (in CPU::step, per I-10)
self.bus.lock().unwrap().tick();            bus.tick();   // bus is &mut Bus
                                            // from let-destructure

// BEFORE (in RVCore::access_bus, mm.rs:258)
let mut bus = self.bus.lock().unwrap();
self.mmu.translate(self.id, addr, op,
    priv_mode, &self.pmp, &mut bus)
// AFTER (bus: &mut Bus param)
self.mmu.translate(self.id, addr, op,
    priv_mode, &self.pmp, bus)

// BEFORE (sc_w at atomic.rs:65-69)
let success = {
    let mut bus = self.bus.lock().unwrap();
    let ok = bus.reservation(self.id) == Some(paddr);
    bus.clear_reservation(self.id);
    ok
};
// AFTER (bus: &mut Bus param)
let success = {
    let ok = bus.reservation(self.id) == Some(paddr);
    bus.clear_reservation(self.id);
    ok
};
```



[**Constraints**]

- C-1: Zero `unsafe`. Every safety claim is borrow-checker enforced.
- C-2: No change to `Device::tick` / `Mmio` trait shapes.
- C-3: No benchmark-specific specialisation. The owned-bus design
  is driven by the static shape of `CPU`/`Core`; no runtime branch
  on guest binary or `num_harts`.
- C-4: `make linux-2hart` boot wall-clock within +/-5 % of
  `docs/perf/baselines/2026-04-15/data/linux_2hart.csv` (R-006). Historically
  expected to be faster since per-access mutex cost is removed from
  the 2-hart path too; +/-5 % is the tolerance band.
- C-5: `make fmt && make clippy && make test` green at the single
  migration commit; no separate per-group commits exist (R-005).
- C-6: DEBUG=n for every benchmark sample (feedback_debug_flag).
- C-7: Workloads launched via `make run` / `make linux` /
  `make linux-2hart` / `make debian`; no direct
  `target/release/xdb` invocation (feedback_use_make_run).
- C-8: Function bodies change 1:1. Replace
  `self.bus.lock().unwrap()` with the threaded `&mut Bus` / `&Bus`.
  No re-architecture of call chains, no fused scopes, no new
  batching APIs. P2's bus-access refactor, P4's icache, and P5's
  MMU inlining remain out-of-scope.

---
