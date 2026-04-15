# `perfBusFastPath` PLAN `02`

> Status: Revised
> Feature: `perfBusFastPath`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `01_MASTER.md` (M-001)

---

## Summary

Phase P1 of the xemu perf roadmap. Remove the `Arc<Mutex<Bus>>`
wrapping that currently dominates per-instruction self-time
(`pthread_mutex_{lock,unlock}` plus PLT stubs at roughly 33-40 % of
cycles per `docs/perf/2026-04-14/REPORT.md` on dhrystone / coremark /
microbench) and replace it with direct CPU-owned bus storage. Per
01_MASTER M-001, the `Shared(Arc<Mutex<Bus>>)` arm proposed in
00_PLAN / 01_PLAN is dropped entirely: xemu's multi-hart scheduler is
single-threaded cooperative round-robin
(`xemu/xcore/src/cpu/mod.rs:213-249`), the only OS-thread crossing
is the UART stdin reader which owns its own
`Arc<Mutex<VecDeque<u8>>>`, and no second OS thread ever reaches
`Bus`. The mutex protects nothing today. The fix is therefore
structural and uniform: `CPU` owns `bus: Box<Bus>`; each `Core::step`
receives `&mut Bus` as a parameter from `CPU::step`; every existing
`bus.lock().unwrap()` becomes a direct call (`bus.tick()`,
`bus.read(...)`, etc.) on a borrowed reference.

There are NO benchmark-targeted tricks in this plan. The refactor is
driven by the static shape of `CPU`/`Core` and applies equally to
`make run` microbench/dhrystone/coremark, to `make linux`, to
`make debian`, and to hand-written guests. The gain band is
re-derived: floor 15 % wall-clock reduction (required), expected
20-30 %, ceiling <= 35 %. The ceiling is grounded in the 2026-04-14
bucket math (pthread self-time 33-40 % disappears entirely; a
fraction redistributes into `xdb::main` / `access_bus` /
`checked_read`, which still do the arithmetic the lock was guarding).

Future SMP (true per-hart OS threads) is explicitly out of scope and
recorded in `docs/DEV.md` Phase 11 ("True SMP - per-hart OS threads -
RFC / FUTURE"). Phase 11 Option B (lock-free RAM atomics + per-device
MMIO locks) and Option C (QEMU-style BQL on MMIO only) remain
available later; nothing in this plan forecloses them. P1 hands
Phase 11 a clean owned-bus starting point rather than a lock shape
inherited from a threading model xemu does not yet have.

## Log

[**Feature Introduce**]

Third iteration. Removes the two-arm `BusHandle` / `BusGuard` /
`ReadBusGuard` / `BusOwner` abstraction introduced in 00_PLAN and
kept in 01_PLAN. Replaces it with a single owned bus on `CPU` and a
per-step `&mut Bus` parameter on `Core::step` / `RVCore::step`. This
resolves 01_MASTER M-001 ("the bus does not need Mutex") and
01_REVIEW C-1 (same observation, CRITICAL), and collapses derivative
01_REVIEW findings (H-3, M-1) that were artefacts of the two-arm
design.



[**Review Adjustments**]

All 01_REVIEW findings resolved. Highlights:

- C-1 CRITICAL (Shared arm protects nothing): dropped entirely.
  `CPU { bus: Box<Bus>, cores: Vec<Core> }`; `Core::step` takes
  `bus: &mut Bus`. No `Mutex`, no `Arc`, no enum discriminant on
  the hot path.
- H-1 HIGH (migration table missed multi-line-chained test helpers):
  rebuilt from `rg "bus\.lock\(\)" xemu -n` with every hit listed
  by file:line, including the previously-missed five test-helper
  sites at `inst/base.rs:344-356`, `inst/compressed.rs:552-556`,
  `inst/float.rs:1075-1079`, `arch/riscv/cpu.rs:277-282`.
- H-2 HIGH (V-UT-5 baseline unimplementable): reframed. With no
  mutex present, the test becomes a zero-hit grep plus a runtime
  `type_name` assertion that `Mutex` is absent from the CPU / Bus
  types. `N`-baseline concept dropped along with the mutex.
- H-3 HIGH (single-hart `Core::bus` hand-wave): dissolved by the
  owned-bus design. `RVCore` loses its `bus` field entirely; every
  method that needed the bus takes `bus: &mut Bus` (or `&Bus` for
  read-only paths) as a parameter. Signature list enumerated in
  Data Structure.
- M-1 MEDIUM (`BusOwner::into_handles` type-state): dropped (no
  factory, no enum).
- M-2 MEDIUM (35 % ceiling math): redone under the new design.
  Full bucket disappears rather than partially redistributes;
  honest ceiling <= 35 %, expected 20-30 %, floor 15 %.
- M-3 MEDIUM (V-IT-7 50 ms budget): dropped. Replaced by "2-hart
  Linux boot wall-clock unchanged within +/-5 % of baseline" plus
  the existing `atomic.rs` LR/SC unit tests.
- L-1 LOW (`cargo asm` tool dependency): downgraded to
  nice-to-have. Hard gate is `rg "Mutex|lock\(\)"` on the bus /
  CPU / RVCore modules returns zero hits.



[**Master Compliance**]

M-001: Applied in full. `Bus` does not need `Mutex` in P1; removed
entirely. CPU owns the bus directly. Multi-hart remains correct
because the scheduler is single-threaded - the single `&mut Bus`
borrow at each `CPU::step` is the single-borrower invariant that
the cooperative round-robin already enforces. Phase 11 RFC in
`docs/DEV.md` records the future SMP options and explicitly retains
the authority to reintroduce per-device or per-memory locking
primitives if and when per-hart OS threads land.



### Changes from Previous Round

[**Added**]
- Owned-bus design: `CPU { bus: Box<Bus>, cores: Vec<Core> }`;
  `Core::step(&mut self, bus: &mut Bus)`.
- Enumerated signature list for every `RVCore::*` method that
  drops `self.bus` in favour of a `bus: &mut Bus` parameter
  (Data Structure block).
- Hard grep gates for post-P1 state (Exit Gate): zero
  `bus\.lock\(\)`, zero `Arc<Mutex<Bus>>`, zero `Mutex` symbol in
  `xemu/xcore/src/device/bus.rs`, `xemu/xcore/src/cpu/mod.rs`,
  `xemu/xcore/src/arch/riscv/cpu.rs`.
- V-UT-5 reformulation: runtime `type_name` check plus repo-level
  `rg` gate.
- Citation to `docs/DEV.md` Phase 11 (Trade-offs T-1).

[**Changed**]
- `BusHandle` enum + `BusGuard` / `ReadBusGuard` / `BusOwner`
  abstractions: removed (M-001).
- Migration pattern: `bus.lock().unwrap().X()` -> direct `bus.X()`
  on a `&mut Bus` / `&Bus` borrow threaded through the call chain
  (no `with` closure, no `with_guard` scope guard).
- Gain-band math: redone under full-bucket-removal assumption
  rather than partial redistribution (M-2).
- V-IT-7 reframed: boot-time equality, not a 50 ms micro-budget.
- `CPU::bus()` accessor: `&self -> &Bus` (was `MutexGuard<Bus>`).
  `CPU::bus_mut()` added for difftest paths as `&mut self ->
  &mut Bus`. No `MutexGuard` anywhere.

[**Removed**]
- `xcore/src/device/bus_handle.rs` new file (not created).
- Two-arm architecture diagram and the `Shared` branch.
- `BusOwner` type-state factory.
- `Cell<bool>` re-entry debug guard (no closure-based API; I-9
  remains, enforced purely by the borrow checker).
- V-IT-8 `cargo asm` from exit-gate (downgraded to nice-to-have
  evidence per L-1).
- V-UT-5 counter-based lock-width test (no locks to count).

[**Unresolved**]
- U-1: Exact placement of `make linux-2hart` in the sample matrix.
  Today `make run` targets microbench/coremark/dhrystone;
  `make linux-2hart` is covered as a boot gate only, not a
  perf-sample point, because its wall-clock dominance is userland
  and not the target of this phase.
- U-2: Fate of the difftest path (`cpu/mod.rs:323`
  `bus_take_mmio_flag`). Migrated mechanically via
  `CPU::bus_mut()`; whether difftest eventually wants a narrower
  API is left open.
- U-3: Long-term SMP shape is Phase 11's RFC; not resolved here.



### Response Matrix

| Source | ID    | Decision  | Resolution / Action in this plan | Test or gate that proves it |
|--------|-------|-----------|----------------------------------|-----------------------------|
| Master | M-001 | Applied   | `Bus` no longer wrapped in `Mutex`. `CPU` owns `Box<Bus>`; `Core::step(&mut self, bus: &mut Bus)` takes the bus by mutable reference. `Arc<Mutex<Bus>>` removed from `CPU` (was `cpu/mod.rs:84`) and `RVCore` (was `arch/riscv/cpu.rs:43`). All `self.bus.lock().unwrap().X()` sites rewritten as direct `bus.X()` calls. | Hard `rg` gate: `rg "Mutex\|lock\(\)" xemu/xcore/src/device/bus.rs xemu/xcore/src/cpu/mod.rs xemu/xcore/src/arch/riscv/cpu.rs` returns zero hits; `rg "Arc<Mutex<Bus>>" xemu -n` returns zero hits. V-UT-5 runtime `type_name` assertion enforces absence of `Mutex` in `CPU`'s compiled type. |
| Review | C-1   | Accepted  | Same as M-001. Shared arm removed; no two-arm enum. Multi-hart correctness argued from the cooperative round-robin invariant, not from a lock shape. | V-IT-4 (`make linux-2hart` boots to shell; wall-clock within +/-5 % of `docs/perf/2026-04-14/data/bench.csv`). `cargo test --workspace` green including `atomic.rs` LR/SC tests that cover multi-hart reservation semantics. |
| Review | H-1   | Accepted  | Migration table rebuilt from `rg "bus\.lock\(\)" xemu -n` (24 hits total, matching the harvest in the brief). Every hit listed by file:line. Production hits (15) become direct `bus.X()` threading. Test-helper hits at `inst/base.rs:344-356`, `inst/compressed.rs:552-556`, `inst/float.rs:1075-1079`, `arch/riscv/cpu.rs:277-282` (the 5 sites 01_PLAN missed) each become either `bus: &mut Bus` / `bus: &Bus` parameters on the helper or construct a fresh `Bus` alongside the core in the test's setup. Full table in Implementation Plan / Phase 2. | Post-Phase-2: `rg "bus\.lock\(\)" xemu -n` returns zero. The field-type change (`bus: Arc<Mutex<Bus>>` -> gone for `RVCore`; `Box<Bus>` for `CPU`) is compile-breaking so the compiler catches any missed site. Checked at each Phase-2 commit. |
| Review | H-2   | Accepted  | V-UT-5 reframed. Instead of a lock-acquire-count baseline, gate is: (a) `rg` on `xemu/xcore/src` returns zero `Mutex` or `lock()` hits in the bus / CPU / RVCore modules (hard gate at Exit Gate); (b) a `#[test]` asserting `!std::any::type_name::<CPU<RVCore>>().contains("Mutex")` at runtime. `N`-baseline concept dropped along with the mutex. | V-UT-5 runtime assertion; hard `rg` gate. |
| Review | H-3   | Accepted  | `RVCore::bus` field deleted. Every `RVCore` method that previously read `self.bus.lock().unwrap().X()` takes `bus: &mut Bus` (or `&Bus` for pure reads) as a parameter. Signature list in Data Structure: `step`, `fetch`, `execute`, `access_bus`, `checked_read`, `checked_write`, `translate`, `store_op`, AMO / LR / SC handlers, `debug::fetch_inst`, `debug::read_memory`. One-call-site refactor, no type-state. | `cargo check -p xcore` green after the signature migration. `arch_isolation` seam test green (bus references do not leak arch-specific types across `xcore`'s public surface). |
| Review | M-1   | Accepted  | `BusOwner` factory removed entirely. Bus construction becomes `Box::new(Bus::new(mbase, msize, num_harts))`. | N/A (code deletion). |
| Review | M-2   | Accepted  | Gain-band math redone. With `Mutex` fully removed, `pthread_mutex_{lock,unlock}` + PLT stubs + the stub-caller redirect all disappear from the profile. Some cycles the lock guarded (arithmetic in `access_bus`/`checked_read`/`xdb::main`) remain and re-attribute to those functions. Honest band: floor 15 %, expected 20-30 %, ceiling <= 35 %. Bucket math walked through in Goals G-2. | Phase 3 perf sample must show >= 15 % wall-clock reduction on dhrystone, coremark, microbench vs. `docs/perf/2026-04-14/data/bench.csv` (DEBUG=n, `make run`); mutex bucket -> 0 % by construction. |
| Review | M-3   | Accepted  | V-IT-7 50 ms budget dropped. Replaced with: "2-hart Linux boot wall-clock within +/-5 % of `docs/perf/2026-04-14/data/bench.csv`" plus reliance on the existing `arch/riscv/cpu/inst/atomic.rs` LR/SC unit tests (which already cover reservation correctness; no new micro-budget test). | V-IT-4 (2-hart Linux boot); existing atomic unit tests; V-IT-7 as restated. |
| Review | L-1   | Accepted  | `cargo asm` gate downgraded to nice-to-have Phase-3 evidence. Hard gate is the `rg` on `Mutex\|lock\(\)`. If `cargo-show-asm` is present, the disassembly snippet is captured as evidence in the perf report; if not, the gate still passes. | Hard gate: `rg` as above. Optional: `cargo asm` output in `docs/perf/<post-P1-date>/REPORT.md` appendix. |

> Rules:
> - Every prior HIGH / CRITICAL finding appears here.
> - Every Master directive appears here.
> - Rejections must include explicit reasoning. (None in this round.)

---

## Spec Alignment

[**Goals**]

- G-1: On every configuration (1-hart, 2-hart, N-hart), `CPU::step`
  and every per-instruction memory access (`checked_read`,
  `checked_write`, `access_bus`, AMO / LR / SC, `Bus::tick`)
  executes with **zero** `pthread_mutex_*` calls on the hot path.
  The bus is accessed via a direct `&mut Bus` borrow threaded
  through the call chain from `CPU::step`.
- G-2: Wall-clock runtime of `make run` (dhrystone, coremark,
  microbench, DEBUG=n) drops by **at least 15 %** vs.
  `docs/perf/2026-04-14/data/bench.csv`; **expected 20-30 %**;
  ceiling <= 35 %. Bucket math: the 2026-04-14 profile shows
  `pthread_mutex_lock` + `pthread_mutex_unlock` + PLT resolution
  stubs at ~33-40 % of self-time across the three benchmarks. The
  P1 design removes *all* mutex work: no `lock()`, no `unlock()`,
  no PLT stub. Some arithmetic the mutex covered (dispatch into
  `Bus::read` / `Bus::store` / `mmu.translate`) remains and
  re-attributes into the caller functions (`access_bus` at 7-9 %,
  `checked_read` at 7-8 %, `xdb::main`). The wall-clock reduction
  equals the portion of the 33-40 % bucket that is genuinely the
  OS `lock()` path itself (uncontended pthread is 20-40 ns per
  acquire; at ~2-3 acquisitions per guest instruction and tens of
  M instr/s this is 15-30 % of wall-clock on a ~9 s dhrystone
  run). Cache-locality gains from the smaller hot path (no mutex
  state adjacent to `Bus::devices`) explain the upper end. 35 %
  is a ceiling, not a forecast.
- G-3: Multi-hart semantics are preserved bit-for-bit. Guest-visible
  ordering is identical: `CPU::step` still calls `bus.tick()` once
  before advancing the current hart; `advance_current()` still
  round-robins; LR/SC reservations still live on `Bus` and are
  checked+cleared under the same borrow scopes (I-4). The only
  change is that the borrow is a plain `&mut Bus` instead of a
  `MutexGuard<Bus>` - both give exclusive access; the borrow
  checker replaces the mutex as the exclusion primitive.
- G-4: LR/SC atomicity across translate -> reservation-read ->
  conditional-store is preserved. Today these sites hold one
  `MutexGuard` across all three operations; P1 holds one
  `&mut Bus` borrow across the same three operations. No widening,
  no narrowing.
- G-5: Public `CPU::bus()` signature is `&self -> &Bus` (was
  `&self -> MutexGuard<'_, Bus>`); method syntax on `&Bus` is
  identical to method syntax on `MutexGuard<Bus>` at the two
  in-tree callers (both `#[cfg(test)]`). Added
  `CPU::bus_mut(&mut self) -> &mut Bus` for the difftest path
  (`bus_take_mmio_flag` at `cpu/mod.rs:321-324`). External-caller
  audit: zero `CPU::bus()` callers outside `xcore`.

- NG-1: No softmmu TLB (P2), no decoded-instruction cache (P4), no
  JIT. P1 only changes the bus ownership.
- NG-2: No per-instruction benchmark-aware branch pruning, no
  guest-PC specialisation, no skipping of UART / PLIC / MTIMER
  ticks.
- NG-3: No change to `Device::tick` / `Mmio::read/write` /
  `Device::reset` semantics or ordering.
- NG-4: No `unsafe` to bypass the borrow checker. The bus
  exclusion invariant is enforced on a plain `&mut Bus`; no
  `UnsafeCell`, no raw pointers, no
  `transmute::<&_, &mut _>`.
- NG-5: No new public crate-external API surface beyond the
  `CPU::bus_mut` addition. `CPU::bus()` return type changes from
  `MutexGuard<'_, Bus>` to `&Bus`; both deref / method-resolve
  identically for the two in-tree `#[cfg(test)]` callers, which
  use only `.read()` / `.num_harts()`.
- NG-6: No re-introduction of `Mutex`, `RwLock`, `parking_lot`, or
  any synchronisation primitive around the bus. Future SMP lock
  shape is Phase 11's RFC decision.

[**Architecture**]

```
                       +--------------------------+
                       |           CPU            |
                       |  cores: Vec<Core>        |
                       |  bus: Box<Bus>           |  <- owned, no Mutex
                       +------------+-------------+
                                    |
                                    | CPU::step:
                                    |   self.bus.tick();
                                    |   self.cores[cur].step(
                                    |       &mut *self.bus);
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

Exclusion model: the cooperative round-robin scheduler enforces that
only one `Core::step` runs at a time per `CPU::step` call. That
scheduler is the single-borrower invariant; the `&mut Bus` borrow
the scheduler hands to the chosen core is the Rust-level
materialisation of that invariant. No second OS thread reaches the
bus (UART stdin reader owns its own mailbox; devices do not
back-call `CPU`; per I-9, closures passed into bus methods may not
reach back to `CPU`).

This is the same shape used by rvemu, rv8, Rare, riscv-rust, and rrs
(all cited in 01_PLAN T-1 / R-008). It is also the shape
`docs/PERF_DEV.md` P1 names directly ("Restore an owned `Bus` on
the hot path"); the PERF_DEV.md sketch floated a `BusHandle` enum
as one option, but the phase goal is the owned bus itself, not the
enum - 01_MASTER M-001 confirms the owned-bus reading is correct.



[**Invariants**]

- I-1: `CPU::bus: Box<Bus>`. Exactly one owner of the `Bus`
  instance. Constructed once in `CPU::new` / machine-factory code
  and never cloned or shared with any other struct.
- I-2: `RVCore` (and any future `CoreOps` implementor) has no
  `bus` field. Every method that needs bus access takes
  `bus: &mut Bus` or `bus: &Bus` as a parameter.
- I-3: For every `CPU::step` invocation, the call sequence is
  `self.bus.tick(); self.cores[self.current].step(&mut *self.bus)`.
  The `&mut Bus` borrow lives for the duration of `Core::step` and
  is released before `advance_current()` runs.
- I-4: `sc_w` / `sc_d` / `amo_*` perform
  `translate -> reservation-check -> conditional-store` inside the
  **same** `&mut Bus` borrow scope (one function body). Holding
  the borrow across translate matches today's `access_bus` scope
  width; no widening, no narrowing.
- I-5: `Bus::tick` is called exactly once per `CPU::step`, before
  the current hart steps.
- I-6: `CPU::reset` calls `self.bus.reset_devices()` and
  `self.bus.clear_reservations()` before any core reset, matching
  the current order at `cpu/mod.rs:141-148`. Both are direct
  method calls on `&mut Bus`.
- I-7: Difftest behaviour unchanged: `Bus::mmio_accessed:
  AtomicBool` remains; `CPU::bus_mut().take_mmio_flag()` is the
  one-line replacement for the `self.bus.lock().unwrap()
  .take_mmio_flag()` site.
- I-8: No `unsafe`. No `Mutex`, `RwLock`, `Arc`, `UnsafeCell`,
  `RefCell`, or raw pointer is introduced or remains on the bus
  path after Phase 2.
- I-9: No method on `Bus` and no `Device::tick` body may call back
  into `CPU` (directly or transitively). Enforced by the borrow
  checker: `CPU::step` holds `&mut Bus` across `Core::step`, and
  `Device::tick`'s receiver (`&mut self` on the device, reached
  through the `&mut Bus`) cannot reach the outer `CPU` because
  the `CPU` is the borrow-exclusive owner of the bus and of the
  device tree by transitivity. A device that wanted to access the
  bus would need an inbound reference, which the type system does
  not provide.



[**Data Structure**]

Changes to existing types (no new types introduced):

```rust
// xemu/xcore/src/cpu/mod.rs
pub struct CPU<Core: CoreOps> {
    cores: Vec<Core>,
    bus: Box<Bus>,                  // was: Arc<Mutex<Bus>>
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
    // existing debug / inspect methods are unchanged except for
    // those that need bus access (see `DebugOps` migration below).
}
```

`RVCore` method signatures that lose `self.bus` and gain a `bus`
parameter (complete list; threaded through mechanically in Phase 2):

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
    // AMO / LR / SC handlers in arch/riscv/cpu/inst/atomic.rs
    // all take `bus: &mut Bus`.
    // Debug paths in arch/riscv/cpu/debug.rs take `bus: &Bus`.
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
    pub fn new(cores: Vec<Core>, bus: Box<Bus>,
               layout: BootLayout) -> Self;
    // was: new(cores, bus: Arc<Mutex<Bus>>, layout)

    pub fn bus(&self) -> &Bus;              // was: MutexGuard<'_, Bus>
    pub fn bus_mut(&mut self) -> &mut Bus;  // NEW (difftest + tests)

    pub fn step(&mut self) -> XResult;              // body rewritten
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

External-caller audit (grep `\.bus\(\)` and `bus_take_mmio_flag`
across `/Users/anekoique/ProjectX`):

- `xemu/xcore/src/cpu/mod.rs` tests at lines ~461 and ~551:
  `cpu.bus().read(...)` and `cpu.bus().num_harts()` - both
  read-only, both `#[cfg(test)]`, both source-compatible with
  `&Bus`.
- No matches outside `xcore`. `xdb`, `xtool`, `xkernels`,
  `xemu/tests/` do not call `cpu.bus()`.
- `bus_take_mmio_flag` has one call site, in `xemu/xdb` difftest
  glue under `#[cfg(feature = "difftest")]`; the caller already
  owns `&mut CPU`, so the `&self -> &mut self` change is
  source-compatible. Verified by
  `cargo check -p xdb --features difftest` in Phase 3.

Call-site migration pattern (representative):

```rust
// BEFORE                                   // AFTER (in CPU::step)
self.bus.lock().unwrap().tick();            self.bus.tick();

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

- C-1: Zero `unsafe`. Every safety claim is borrow-checker
  enforced.
- C-2: No change to `Device::tick` / `Mmio` trait shapes.
- C-3: No benchmark-specific specialisation. The owned-bus design
  is driven by the static shape of `CPU`/`Core`; no runtime branch
  on guest binary or `num_harts`.
- C-4: `make linux-2hart` boot wall-clock within +/-5 % of
  `docs/perf/2026-04-14/data/bench.csv`. Historically it may be
  faster since per-access mutex cost is removed from the 2-hart
  path too; +/-5 % is the tolerance band.
- C-5: `make fmt && make clippy && make test` green on every
  committed group of the Phase 2 migration.
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

## Implementation Steps

### Execution Flow

[**Main Flow**]

Per-instruction (single-hart or multi-hart; same code path):

1. `CPU::step` calls `self.bus.tick()` - direct method call on
   `&mut Box<Bus>`, no atomics.
2. `CPU::step` calls
   `self.cores[self.current].step(&mut *self.bus)`, handing out
   an exclusive borrow of the bus for the duration of the core
   step.
3. `RVCore::step` reads `bus.mtime()` via the `bus` parameter, syncs
   interrupts, checks pending traps, fetches an instruction via
   `self.fetch(bus)`, decodes and executes.
4. `fetch(bus)` calls `self.checked_read(bus, self.pc, 4, Fetch)`
   which calls `self.access_bus(bus, ...)` (one borrow scope, same
   width as today).
5. `execute` dispatches to per-instruction handlers. Loads /
   stores go through `self.checked_read(bus, ...)` /
   `self.checked_write(bus, ...)`. AMO / LR / SC handlers take
   `bus: &mut Bus` and perform translate + reserve /
   reservation-check + conditional-store in one function body
   (I-4).
6. `retire()` and `advance_current()` run after `Core::step`
   returns, releasing the `&mut Bus` borrow.
7. Return to `CPU::run`.

[**Failure Flow**]

1. Borrow-check error at migration time: if any site tries to keep
   two `&mut Bus` borrows alive simultaneously, the compiler
   rejects the change. This is the I-9 reentry invariant: if a
   `Device::tick` body tried to reach `CPU`, there would be no
   way to spell the back-reference without re-introducing `Arc`,
   which Phase 2 forbids.
2. Lost reservation on reset: `CPU::reset` calls
   `self.bus.reset_devices()` then
   `self.bus.clear_reservations()`, same order as today. No
   behavioural change.
3. Difftest `bus_take_mmio_flag`: the call site is already
   `&mut CPU`, so the `&self -> &mut self` change is
   source-compatible. If a future call site holds `&CPU`, it must
   switch to `&mut CPU`; there is one such site today, already
   `&mut CPU` (verified in external-caller audit).
4. Test helpers that reach `core.bus` (`inst/base.rs:344-356`,
   `inst/compressed.rs:552-556`, `inst/float.rs:1075-1079`,
   `arch/riscv/cpu.rs:277-282`): these construct an `RVCore` for
   one-off assertions. Since `RVCore` no longer owns a bus, these
   helpers either (a) take a `&mut Bus` / `&Bus` parameter
   explicitly, or (b) construct a fresh `Box<Bus>` alongside the
   core in the test's `setup_core()`. Pattern (a) is preferred
   where the test already has a `Bus` in scope; (b) is the
   default for `RVCore::new()` direct users.

[**State Transition**]

- Construction: `CPU::new(cores, Box::new(Bus::new(mbase, msize,
  num_harts)), layout)`.
  Runtime: no state transition on bus ownership; the `Box<Bus>`
  is owned by `CPU` from construction to drop.
- Reset: `CPU::reset` calls `self.bus.reset_devices()` +
  `self.bus.clear_reservations()`, then resets each core. No
  ownership change.
- Per-step: exclusive `&mut Bus` borrow handed to
  `self.cores[self.current]` for the duration of `Core::step`,
  then released. This is a lexical scope in `CPU::step`'s body,
  not a field-level transition.

### Implementation Plan

[**Phase 1 - Rewrite `CPU`'s bus ownership**]

- 1a. Change `CPU::bus` field type from `Arc<Mutex<Bus>>` to
  `Box<Bus>`. Update `CPU::new`'s signature accordingly.
- 1b. Add `CPU::bus_mut(&mut self) -> &mut Bus`; rewrite
  `CPU::bus(&self) -> &Bus`.
- 1c. Rewrite the 7 production `self.bus.lock().unwrap().X()`
  sites in `cpu/mod.rs` (lines 101, 126, 142-145, 168, 199, 214,
  293, 323) to direct method calls on `&mut Bus` / `&Bus`.
- 1d. Temporarily keep `RVCore::bus` as `Arc<Mutex<Bus>>` so arch
  code still compiles; construct the wrapper inside `CPU::new`
  from the `Box<Bus>` via a clone-on-construction shim guarded
  by `#[deprecated]` comment. (This shim exists for exactly one
  commit between Phase 1 and Phase 2 and is removed in Phase 2a.)
- 1e. `make fmt && make clippy && cargo test -p xcore cpu::` -
  CPU-only tests must pass at end of Phase 1.

[**Phase 2 - Remove `RVCore::bus` and thread `&mut Bus`**]

Migration table (complete, harvested from
`rg "bus\.lock\(\)" xemu -n`; 24 hits in production + test
helpers):

| File : Line                                                   | Today                                           | Migration                                                                                                      |
|---------------------------------------------------------------|-------------------------------------------------|----------------------------------------------------------------------------------------------------------------|
| `xcore/src/cpu/mod.rs:101`                                    | `bus.lock().unwrap().num_harts()`               | `bus.num_harts()` - `bus` is already `Box<Bus>` after Phase 1.                                                 |
| `xcore/src/cpu/mod.rs:126`                                    | `self.bus.lock().unwrap()` (inside `bus()`)     | `&self.bus` (body returns `&Bus` directly).                                                                    |
| `xcore/src/cpu/mod.rs:142-145` reset                          | `let mut bus = self.bus.lock().unwrap(); ...`   | `self.bus.reset_devices(); self.bus.clear_reservations();` - drop the scope block.                             |
| `xcore/src/cpu/mod.rs:168` direct image load                  | `bus.lock().unwrap().load_ram(...)`             | `self.bus.load_ram(RESET_VECTOR, image_bytes)`.                                                                |
| `xcore/src/cpu/mod.rs:199` firmware file load                 | `bus.lock().unwrap().load_ram(...)`             | `self.bus.load_ram(addr, &bytes)?`.                                                                            |
| `xcore/src/cpu/mod.rs:214` step tick                          | `bus.lock().unwrap().tick()`                    | `self.bus.tick()`. Hottest site; largest measured delta.                                                       |
| `xcore/src/cpu/mod.rs:293` replace_device                     | `bus.lock().unwrap().replace_device(...)`       | `self.bus.replace_device(name, dev)`.                                                                          |
| `xcore/src/cpu/mod.rs:323` take_mmio_flag (difftest)          | `bus.lock().unwrap().take_mmio_flag()`          | `self.bus.take_mmio_flag()`; receiver changes `&self -> &mut self`.                                            |
| `xcore/src/arch/riscv/cpu.rs:43`                              | `bus: Arc<Mutex<Bus>>` field                    | Delete field.                                                                                                  |
| `xcore/src/arch/riscv/cpu.rs:66`                              | `with_id(id, bus, irq)` ctor                    | `with_id(id, irq)` - drop `bus` param; core no longer owns bus.                                                |
| `xcore/src/arch/riscv/cpu.rs:207` `mtime()` in `step`         | `self.bus.lock().unwrap().mtime()`              | `bus.mtime()` - parameter `bus: &mut Bus` in new `step` signature.                                             |
| `xcore/src/arch/riscv/cpu.rs:277-282` (test) `write_inst`     | `core.bus.lock().unwrap().write(...)`           | Test helper: signature becomes `fn write_inst(core: &mut RVCore, bus: &mut Bus, inst: u32)`; body `bus.write(...)`. |
| `xcore/src/arch/riscv/cpu/debug.rs:93-99` `read_memory`       | `let bus = self.bus.lock().unwrap();` + read    | Signature `fn read_memory(&self, bus: &Bus, ...)`; body `bus.read_ram(...)`.                                   |
| `xcore/src/arch/riscv/cpu/debug.rs:103-109` `fetch_inst`      | one guard spanning two reads                    | Signature `fn fetch_inst(&self, bus: &Bus, ...)`; body `bus.read(...)` twice on the same ref.                  |
| `xcore/src/arch/riscv/cpu/mm.rs:258` `access_bus`             | `let mut bus = self.bus.lock().unwrap();`       | `bus: &mut Bus` param threaded through; `&mut *bus` passed to `mmu.translate`.                                 |
| `xcore/src/arch/riscv/cpu/mm.rs:267-272` `checked_read`       | second `lock()` after translate                 | One `bus: &mut Bus` param through `access_bus` and the subsequent `bus.read(pa, size)` in one function body.   |
| `xcore/src/arch/riscv/cpu/mm.rs:276-279` `checked_write`      | second `lock()` after translate                 | Same - one `bus: &mut Bus` param covers translate + `bus.store(...)`.                                          |
| `xcore/src/arch/riscv/cpu/inst/base.rs:75`                    | per-store `clear_reservation`                   | `bus.clear_reservation(self.id)` on the `&mut Bus` param (threaded through `store_op`).                        |
| `xcore/src/arch/riscv/cpu/inst/base.rs:344-348` (test)        | `core.bus.lock().unwrap().load_ram(...)`        | Test helper; signature takes `bus: &mut Bus`; body `bus.load_ram(...)`.                                        |
| `xcore/src/arch/riscv/cpu/inst/base.rs:351-356` (test)        | `core.bus.lock().unwrap().read(...)`            | Test helper; signature takes `bus: &Bus`; body `bus.read(...)`.                                                |
| `xcore/src/arch/riscv/cpu/inst/compressed.rs:552-556` (test)  | `core.bus.lock().unwrap().write(...)`           | Test helper; `bus: &mut Bus`.                                                                                  |
| `xcore/src/arch/riscv/cpu/inst/compressed.rs:573` (test)      | `core.bus.lock().unwrap().read(...)`            | Test helper; `bus: &Bus`.                                                                                      |
| `xcore/src/arch/riscv/cpu/inst/float.rs:1075-1079` (test)     | `core.bus.lock().unwrap().write(...)`           | Test helper; `bus: &mut Bus`.                                                                                  |
| `xcore/src/arch/riscv/cpu/inst/float.rs:1085` (test)          | `core.bus.lock().unwrap().read(...)`            | Test helper; `bus: &Bus`.                                                                                      |
| `xcore/src/arch/riscv/cpu/inst/atomic.rs:30`                  | AMO 32-bit `clear_reservation`                  | `bus.clear_reservation(self.id)` on param.                                                                     |
| `xcore/src/arch/riscv/cpu/inst/atomic.rs:45`                  | AMO 64-bit `clear_reservation`                  | Same.                                                                                                          |
| `xcore/src/arch/riscv/cpu/inst/atomic.rs:57`                  | LR 32-bit `reserve`                             | `bus.reserve(self.id, paddr)` on param.                                                                        |
| `xcore/src/arch/riscv/cpu/inst/atomic.rs:65-69`               | SC 32-bit triple-lock zone                      | One scope under the threaded `bus: &mut Bus`; I-4 preserved.                                                   |
| `xcore/src/arch/riscv/cpu/inst/atomic.rs:81`                  | LR 64-bit `reserve`                             | Same as :57.                                                                                                   |
| `xcore/src/arch/riscv/cpu/inst/atomic.rs:89-94`               | SC 64-bit triple-lock zone                      | Same as :65-69.                                                                                                |
| `xcore/src/arch/riscv/cpu/inst/atomic.rs:195,200,204,212` (test)| AMO test helpers                              | Mechanical migration; each takes `bus: &mut Bus` or `bus: &Bus`.                                               |

After each logical group (CPU surface; mm/access path; atomic path;
test helpers by file), run `make fmt && make clippy && cargo test
--workspace`. Checkpoint at end of Phase 2:
`rg "bus\.lock\(\)" xemu -n` returns zero hits;
`rg "Arc<Mutex<Bus>>" xemu -n` returns zero hits.

[**Phase 3 - Verify, measure, document**]

- 3a. Boot gate: `make run` (microbench) completes; `make linux`
  boots to shell; `make linux-2hart` boots to shell; `make debian`
  boots to login.
- 3b. Perf sampling (DEBUG=n): `scripts/perf/sample.sh` on
  dhrystone, coremark, microbench; render via
  `scripts/perf/render.py` into
  `docs/perf/<post-P1-date>/data/` and `REPORT.md`.
- 3c. Compare against `docs/perf/2026-04-14/data/bench.csv`:
  assert >= 15 % wall-clock reduction on each of the three
  benchmarks; assert `pthread_mutex_*` bucket -> 0 % (no samples
  in the new profile by construction).
- 3d. `linux-2hart` boot-to-shell time sampled three runs each
  pre / post; confirm within +/-5 %.
- 3e. Full test pass: `cargo test --workspace` green (336 unit
  tests + `arch_isolation` + `atomic` LR/SC tests; difftest
  feature build also green).
- 3f. Optional (nice-to-have, not gated): `cargo asm -p xcore
  xcore::cpu::CPU::step --rust` disassembly snippet captured in
  the perf report appendix as evidence that the `Bus::tick` call
  is a direct `call` with no `lock cmpxchg` / `xchg` /
  `pthread_mutex_*` symbol.
- 3g. Update `docs/PERF_DEV.md` P1 row to "Done" with the
  measured numbers and link to
  `docs/perf/<post-P1-date>/REPORT.md`.

---

## Validation Strategy

[**Unit Tests**]

- V-UT-1: `device/bus.rs` existing tests (13) - unchanged. `Bus`
  API is untouched.
- V-UT-2: `cpu/mod.rs` existing tests (13) - mechanically migrated
  to `CPU::new(cores, Box::new(bus), layout)` and `cpu.bus().X()`
  or `cpu.bus_mut().X()`. Green after Phase 1.
- V-UT-3: `arch/riscv/cpu.rs` tests - `write_inst` helper migrated
  to take `bus: &mut Bus`; setup constructs `Box<Bus>` locally.
  Green after Phase 2.
- V-UT-4: `arch/riscv/cpu/inst/atomic.rs` tests (20+ including
  LR/SC semantic tests) - all migrate mechanically to the threaded
  `bus: &mut Bus` pattern. A `sc_w` after a conflicting store
  must still return failure; a `sc_w` inside its own reservation
  window must still succeed. These tests already exist and are
  the correctness gate for I-4.
- V-UT-5: `no_mutex_on_bus_path` - a `#[test]` in
  `xemu/xcore/src/cpu/mod.rs` that asserts
  `!std::any::type_name::<crate::cpu::CPU<crate::arch::riscv::RVCore>>()
  .contains("Mutex")` and
  `!std::any::type_name::<crate::arch::riscv::RVCore>().contains("Mutex")`.
  Complemented by a repo-level `scripts/verify_no_mutex.sh` invoking
  `rg -q "Mutex|Arc<Mutex<Bus>>|bus\.lock\(\)"
  xemu/xcore/src/device/bus.rs xemu/xcore/src/cpu/mod.rs
  xemu/xcore/src/arch/riscv/cpu.rs
  xemu/xcore/src/arch/riscv/cpu/mm.rs
  xemu/xcore/src/arch/riscv/cpu/inst/atomic.rs`
  and exiting non-zero on any match. The shell gate runs as part
  of `make test` (via a thin wrapper target); the `type_name`
  gate runs inside `cargo test`.
- V-UT-6: Existing `arch_isolation.rs` seam test - unchanged.
  Exercises that `bus: &mut Bus` parameters do not leak
  arch-specific types across `xcore`'s public surface.

[**Integration Tests**]

- V-IT-1: `make run` (default microbench / direct image) boots
  and exits 0.
- V-IT-2: `make linux` boots to `/ # ` prompt, runs
  `echo hello; exit`.
- V-IT-3: `make debian` boots to login (full userland sanity).
- V-IT-4: `make linux-2hart` boots to prompt and both harts
  appear in `/proc/cpuinfo`. Wall-clock within +/-5 % of
  `docs/perf/2026-04-14/data/bench.csv`'s 2-hart Linux boot
  timing.
- V-IT-5: `make xv6` - boots if the target is wired up in the
  local tree; skipped otherwise and declared in the perf report
  so the reviewer can decide.
- V-IT-6: `cargo test --workspace` - 336 unit tests + atomic
  LR/SC tests + `arch_isolation` all green.
- V-IT-7: `atomic.rs` LR/SC correctness under the threaded
  `bus: &mut Bus` model - already covered by existing unit tests
  (`lr_w_then_sc_w_success`,
  `sc_w_fails_after_conflicting_store`, etc., 20+ tests). No new
  50 ms budget.

[**Failure / Robustness Validation**]

- V-F-1: `CPU::reset` after boot: all reservations cleared, all
  devices reset, subsequent step succeeds. Reuses existing
  `cpu/mod.rs` reset test under the new ownership.
- V-F-2: Difftest build (`cargo test --features difftest -p xcore`)
  green - confirms `bus_take_mmio_flag`'s `&mut self` receiver
  does not break the difftest harness.
- V-F-3: Repo-level grep gate (V-UT-5 shell script) fails if any
  future commit reintroduces `Mutex` or `bus.lock()` on the bus /
  CPU / RVCore path.

[**Edge Case Validation**]

- V-E-1: Zero-instruction run (`CPU::run(0)`) - `step` not called,
  `bus.tick()` not called. Behaviour unchanged.
- V-E-2: Single-hart LR/SC on own reservation succeeds; LR
  followed by unrelated store does not invalidate the
  reservation.
- V-E-3: 2-hart LR/SC: hart 0 `lr`; scheduler advances to hart 1;
  hart 1 stores in the reserved granule; scheduler returns to
  hart 0; `sc` fails. Deterministic under cooperative
  round-robin. Covered by existing atomic tests; reconfirmed
  under the new borrow model.
- V-E-4: Boot -> reset -> re-run loop: no leaked bus borrow, no
  lingering reservation, no stale device state.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (zero pthread on hot path, all configs) | V-UT-5 (grep + `type_name` assertion); Phase-3c perf sample with zero mutex bucket. |
| G-2 (>= 15 % wall-clock, expected 20-30 %, ceiling <= 35 %) | Phase-3b sampling vs `docs/perf/2026-04-14/data/bench.csv` on dhrystone/coremark/microbench. |
| G-3 (multi-hart semantics preserved) | V-IT-4 (`make linux-2hart` boots, wall-clock +/-5 %); V-E-3 (LR/SC ping-pong). |
| G-4 (LR/SC atomicity preserved) | V-UT-4 (`atomic.rs` 20+ LR/SC tests); V-E-2, V-E-3. |
| G-5 (public API stable modulo `bus_mut` addition) | V-UT-2 (cpu/mod.rs tests unchanged in shape); external-caller audit documents zero external callers. |
| C-1 (no `unsafe`) | `make clippy` diff; `rg unsafe` review of the PR diff. |
| C-2 (device traits unchanged) | V-UT-1 (`device/bus.rs` tests green). |
| C-3 (no benchmark tricks) | Code review: bus ownership shape is static, driven by `CPU`/`Core` type layout. |
| C-4 (linux-2hart +/-5 %) | V-IT-4 sampled 3x pre/post. |
| C-5 (fmt/clippy/test clean) | `make fmt && make clippy && make test` gate on every Phase 2 commit. |
| C-6 (DEBUG=n benchmarks) | `scripts/perf/sample.sh` env check. |
| C-7 (make-based launches) | Perf report records exact `make` targets. |
| C-8 (1:1 body changes) | PR diff review; migration table maps each site to a one-line replacement. |
| I-8 (no `Mutex` / `Arc` / `UnsafeCell` on bus path) | V-UT-5; hard `rg` gate. |

---

## Trade-offs

- T-1: **Bus ownership model - owned `Box<Bus>` on `CPU` (Option
  A, recommended) vs. the two-arm `BusHandle` enum from 01_PLAN
  (Option B, rejected) vs. `Arc<UnsafeCell<Bus>>` behind a
  single-hart feature flag (Option C, rejected).**

  Option A (recommended, applied in this plan):
  `CPU { bus: Box<Bus> }`; `Core::step(&mut self, bus: &mut Bus)`.
  Safe Rust; borrow checker is the exclusion primitive. No
  discriminant, no enum, no factory. Matches the single-threaded
  cooperative round-robin scheduler exactly: one `&mut Bus`
  borrow per `CPU::step`, exactly matching the one hart that runs
  per step. G-2 applies uniformly to 1-hart and N-hart configs;
  C-4 is easier to meet because the 2-hart path also loses its
  mutex. This is what 01_MASTER M-001 directs.

  Option B (rejected): two-arm
  `BusHandle { Owned(Box<Bus>), Shared(Arc<Mutex<Bus>>) }` from
  00_PLAN / 01_PLAN. Rejected because the `Shared` arm protects
  nothing (01_REVIEW C-1, 01_MASTER M-001): no second OS thread
  reaches `Bus` in the current tree. Keeping the arm for a
  threading model xemu does not have is cargo-culting.

  Option C (rejected): `Arc<UnsafeCell<Bus>>` plus a single-hart
  feature flag. Rejected per NG-4 (no `unsafe`) and because it
  bifurcates the build matrix.

  Relationship to future SMP: `docs/DEV.md` Phase 11 ("True SMP -
  per-hart OS threads - RFC / FUTURE") sketches the design space
  for actual parallel execution. Option B there (per-hart threads
  with lock-free RAM atomics + per-device MMIO locks) and Option
  C there (QEMU-style BQL on MMIO memory only) both remain
  available. P1's owned-bus shape does not foreclose either. When
  Phase 11 lands, the bus can be re-split into
  `Arc<GuestMemory>` (lock-free atomics) + per-device
  `Arc<Mutex<dyn Mmio>>` without unwinding Phase-1 work; the
  change is additive, not a revert.

- T-2: **`CPU::bus()` return type - direct reference (Option A,
  adopted) vs. opaque read-only wrapper (Option B, considered
  and rejected).**

  Option A: `CPU::bus(&self) -> &Bus`.
  `CPU::bus_mut(&mut self) -> &mut Bus`. Mirrors `HashMap::get /
  get_mut`. Existing callers (`cpu.bus().read(...)`,
  `cpu.bus().num_harts()`) stay source-compatible because method
  syntax on `&Bus` is identical to method syntax on
  `MutexGuard<Bus>` at these call sites.

  Option B: `CPU::bus(&self) -> ReadBusView<'_>` wrapper type.
  Rejected: no protection that `&Bus` doesn't already provide;
  adding a wrapper for its own sake violates the "clean, concise,
  elegant" project constraint.

- T-3: **Signature style for threading `bus` - plain `&mut Bus`
  parameter (Option A, adopted) vs. moving `Bus` into a per-step
  `StepContext<'a> { bus: &'a mut Bus, ... }` struct (Option B).**

  Option A: plain `&mut Bus` parameter on each method that needs
  it. Verbose at method boundaries; unambiguous at the borrow
  checker. Minimises diff size and keeps the migration 1:1.

  Option B: a `StepContext` struct gathering the bus, MMU
  reference, and current privilege. Reduces call-site verbosity
  but introduces a new type and a new borrowing story (the
  struct's fields must not be split-borrowed in ways that
  conflict with disjoint-field inference). Considered; rejected
  for P1 as scope creep. Can be introduced later as a pure
  refactor if signatures become unwieldy.

- T-4: **`store_op`'s per-store `clear_reservation` placement -
  separate call after `checked_write` (Option A, adopted) vs.
  fused into `checked_write`'s body (Option B, rejected).**

  Option A (current behaviour, preserved): `store_op` calls
  `checked_write(bus, ...)` then `bus.clear_reservation(self.id)`
  as a separate statement. Two sequenced borrows of `&mut Bus`
  on the same parameter.

  Option B: fuse the clear into `checked_write`'s body. Semantic
  change ("store-then-clear" becomes "store-and-clear
  atomically" from an SC peer's perspective). Rejected for the
  same reason 01_PLAN rejected it: multi-hart correctness (G-3)
  requires the clear to be a separately observable event. The
  clear's cost is one direct method call; not worth losing the
  semantic distinction.

---

## Exit Gate

Union of hard gates; all must pass before P1 is declared done and
the `docs/PERF_DEV.md` P1 row is flipped to "Done":

1. `rg "bus\.lock\(\)" xemu -n` returns zero hits.
2. `rg "Arc<Mutex<Bus>>" xemu -n` returns zero hits.
3. `rg "Mutex" xemu/xcore/src/device/bus.rs
   xemu/xcore/src/cpu/mod.rs xemu/xcore/src/arch/riscv/cpu.rs`
   returns zero hits.
4. `make fmt && make clippy` clean (no new warnings).
5. `cargo test --workspace` green (336 unit tests +
   `arch_isolation` + `atomic` LR/SC tests; difftest feature
   build also green).
6. `make run` on dhrystone / coremark / microbench: wall-clock
   reduction >= 15 % per benchmark vs.
   `docs/perf/2026-04-14/data/bench.csv`.
7. `make linux` boots to interactive shell.
8. `make linux-2hart` boots to shell; wall-clock within +/-5 %
   of baseline (expected faster, not slower).
9. `make debian` boots to login.
10. Fresh perf sample collected via `scripts/perf/sample.sh` with
    DEBUG=n, rendered into `docs/perf/<post-P1-date>/`; the
    `pthread_mutex_*` bucket shows 0 % of self-time in the new
    profile (by construction - no mutex remains).

Nice-to-have (does not gate; capture if tooling available):
- `cargo asm -p xcore xcore::cpu::CPU::step --rust` disassembly
  appendix confirming `Bus::tick` is a direct `call`.
- `criterion` microbench at `xcore/benches/bus_step.rs` on a 1 M
  NOP loop; pre/post numbers archived alongside the perf report.

---

## Response Matrix (see header for the full table)

The Response Matrix in the Log section above addresses every
C-/H-/M-/L- finding from `01_REVIEW.md` and the M-001 directive
from `01_MASTER.md`, each with a Resolution / Action and a
concrete Test or gate. No rejections.
