# `multiHart` PLAN `01`

> Status: Draft
> Feature: `multiHart`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `none` (inherited MASTER directives from archModule /
>   archLayout still binding — see Master Compliance)

---

## Summary

Round 01 keeps the round-00 thesis — a `Hart` abstraction inside
`arch/riscv/cpu/` with every per-hart field migrated off `RVCore`, a
round-robin scheduler at `num_harts ≥ 1`, and ACLINT sub-devices
widened to per-hart state arrays — and absorbs the three HIGH
findings from `00_REVIEW.md`. The PR shape moves from 2 PRs to **3
PRs** per TR-3(b): **PR1** (Hart abstraction at `num_harts = 1`,
byte-identical behaviour), **PR2a** (PLIC runtime-size conversion
at `num_harts = 1`, `Plic::new(num_harts, Vec<IrqState>)`, still
byte-identical), **PR2b** (activate `num_harts > 1` via
`--harts N` CLI, `xemu-2hart.dtb`, SMP Linux smoke). Two correctness
fixes land in PR1: (i) cross-hart LR/SC reservation invalidation
(I-8) via a `RVCore::invalidate_reservations_except(src, addr,
size)` hook called from the post-store path — a no-op at
`num_harts = 1`, load-bearing at `num_harts > 1`; (ii) per-hart
`mhartid` seeding at `Hart::new` (per R-010) so PR2b carries zero
mhartid-specific diff. `RVCore::with_bus` retains its current
`(Bus, IrqState)` shape with an internal `vec![irq; num_harts]`
expansion (per R-005(a)). Acceptance Mapping grows rows for
I-1/I-2/I-3/I-8/C-7. Test-count arithmetic (R-003) is reconciled
explicitly: PR1 = 354 + 8 = 362 lib tests; PR2a = 362 + 2 = 364;
PR2b = 364 + 3 = 367. OpenSBI HSM presence is pre-verified (R-008)
— `resource/opensbi/lib/sbi/sbi_hsm.c` exists in-tree, no `.mk`
edit required.

## Log

[**Feature Introduce**]

- **3-PR shape** (TR-3(b) adopted): PR1 refactor, PR2a PLIC runtime-
  size, PR2b multi-hart activation. Each PR independently passes
  all six inherited gates at its own `num_harts` value.
- **I-8 (new invariant)**: cross-hart LR/SC reservation
  invalidation. Lands in PR1 via a `RVCore`-owned broadcast helper;
  trivially satisfied at `num_harts = 1`, correct at `N > 1`.
- **`Plic::new(num_harts: usize, irqs: Vec<IrqState>) -> Self`**:
  PR2a-scope device-API change. `const NUM_CTX`/`const CTX_IP`
  deleted; runtime `num_ctx = 2 * num_harts`, per-hart
  `irqs: Vec<IrqState>`, `evaluate()` indexes
  `irqs[ctx >> 1]` with `ip = if ctx & 1 == 0 { MEIP } else { SEIP }`.
- **mhartid seeding at `Hart::new`** (R-010): PR1 deletes the
  hard-coded `mhartid = 0` from `csr.rs:250` and writes
  `hart.csr.set(CsrAddr::mhartid, id.0 as Word)` inside
  `Hart::new`. `mhartid` remains `[RO]` from the guest view.
- **`RVCore::with_bus(bus: Bus, irq: IrqState)` signature
  preserved** (R-005(a)): the constructor internally builds
  `vec![irq; bus.num_harts()]`. New accessor `Bus::num_harts() ->
  usize` added.
- **Explicit test-count arithmetic** (R-003): gate matrix states
  post-PR counts explicitly; no more "same count" claim.
- **Phase file lists** now enumerate every touched function /
  constant per R-006.

[**Review Adjustments**]

R-001 (HIGH), R-002 (HIGH), R-003 (HIGH) resolved in full — see
Response Matrix. R-004..R-010 resolved per matrix. TR-3(b) adopted.
TR-1/2/4/5 kept as concurred (no change).

[**Master Compliance**]

No `01_MASTER.md`. Inherited binding directives from archModule /
archLayout continue to apply:

- **00-M-001** — no global `trait Arch`. Honoured: `Hart` stays
  arch-internal; no cross-arch hart trait.
- **00-M-002** — topic-organised `arch/<name>/`. Honoured: `Hart`
  lives at `arch/riscv/cpu/hart.rs`.
- **01-M-001** — no `selected` alias word. Honoured.
- **01-M-002** — clean, concise, elegant. Honoured: 3 PRs, each
  tight; `Hart` is plain data; the reservation-invalidation helper
  is a 6-line `for` loop; round-robin is a cursor advance.
- **01-M-003** — no redundant arch-validity checks. Honoured: no
  new cfg scaffolding.
- **01-M-004** — top-level `cpu/`, `device/`, `isa/` stay trait +
  cfg. Honoured: all `Hart` code under `arch/riscv/cpu/`; `CoreOps`
  / `DebugOps` signatures unchanged; no `HartId` in
  `crate::cpu::*`.

### Changes from Previous Round

[**Added**]

- **I-8** — cross-hart LR/SC reservation invalidation invariant and
  its implementation hook (`RVCore::invalidate_reservations_except`).
- **V-UT-9** — `Hart::id ordering preserved` unit test (I-2).
- **V-UT-10** — `Plic::new(num_harts, Vec<IrqState>)` routing test
  (PR2a — MEIP on ctx 2 lands on `irqs[1]`, not `irqs[0]`).
- **V-UT-11** — cross-hart LR/SC invalidation unit test (PR1-scope,
  runs at `num_harts = 2` via a direct `RVCore::with_config`
  harness; does not require PR2b CLI activation).
- **V-IT-6** — PR2a regression: all 13 existing `plic.rs` tests
  pass unchanged with `Plic::new(1, vec![irq])`.
- Phase-2a file list with enumerated PLIC function-level deltas.
- `Bus::num_harts() -> usize` accessor (supports R-005(a)).
- Acceptance-Mapping rows for I-1, I-2, I-3, I-8, C-7.
- TR-1 rationale one-liner about HSM handshake (per TR-1
  `Required Action`).

[**Changed**]

- **PR shape**: 2 PRs → 3 PRs (PR1, PR2a, PR2b) per TR-3(b).
- **PR1 gate matrix**: test count stated explicitly as 362 lib + 1
  `arch_isolation` + 6 `xdb` = 369, not "same count".
- **`RVCore::with_bus` signature**: reverted to
  `(Bus, IrqState)` from round-00's proposed
  `(Bus, Vec<IrqState>)` (R-005(a)).
- **mhartid seeding**: moved from PR2 (round 00) to PR1 (round 01)
  per R-010.
- **NG-1**: reframed — PR2a performs a targeted PLIC runtime-size
  conversion (device-API reshape, not "mechanical"); full PLIC
  gateway redesign stays in `plicGateway`.
- **Non-Goals renumbered**: NG-5 (PLIC deferral caveat) and NG-8
  (OpenSBI HSM) updated; NG-8 now notes HSM pre-verified in-tree.

[**Removed**]

- The word "mechanical" describing PR2's PLIC change (per R-004).
- Round-00 claim that PR1 adds "zero net tests" (per R-003).

[**Unresolved**]

- **SMP Linux boot throughput at `num_harts = 2`**: one-instruction
  round-robin halves guest IPS. Known; acceptable for PR2b gate
  (correctness, not perf). Perf follow-up deferred (T-1 / NG-2).
- **Full difftest coverage at `num_harts > 1`**: NG-3 still holds.
  This leaves a correctness canary gap at exactly the
  configuration I-8 becomes load-bearing; V-UT-11 +
  `smp_linux_smoke` are the only cross-hart correctness gates.
  Documented, not fixed.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | PR2a now carries full PLIC device-API reshape; `Plic::new(num_harts, Vec<IrqState>)` in API Surface; enumerated deltas in Phase-2a file list; V-UT-10 covers per-hart routing. |
| Review | R-002 | Accepted | I-8 added; `RVCore::invalidate_reservations_except(src, addr, size)` hook added to API Surface; called from `store_op`/`sc_w`/`sc_d` post-store paths; V-UT-11 validates at `num_harts = 2`. Option (a) (RVCore-owned walk) chosen over (b) (Bus broadcast) per lighter single-hart cost and NG-2 alignment. |
| Review | R-003 | Accepted | PR1 gate matrix states 362 lib tests post-merge; PR2a 364; PR2b 367. "Same count" claim deleted. |
| Review | R-004 | Accepted | "Mechanical" removed from Summary / NG-1; PR split adopted per TR-3(b). |
| Review | R-005 | Accepted-modified | Option (a) adopted: `with_bus(bus: Bus, irq: IrqState)` signature preserved; expansion to `vec![irq; num_harts]` happens inside the constructor using new `Bus::num_harts()` accessor. (Reviewer's preferred option (b) adds an extra constructor name and rejected to keep surface minimal per 01-M-002.) |
| Review | R-006 | Accepted | Phase-2a (PLIC) and Phase-1 (ACLINT install rewrite) file lists now enumerate exact function-level deltas. |
| Review | R-007 | Accepted | Acceptance Mapping gains I-1, I-2, I-3, I-8, C-7 rows; V-UT-9 added for I-2. |
| Review | R-008 | Accepted | Pre-verified: `resource/opensbi/lib/sbi/sbi_hsm.c` exists in-tree; NG-8 updated to cite verification; no `.mk` edit required. |
| Review | R-009 | Accepted-as-is | `ebreak_as_trap` stays on `RVCore`, passed to `Hart::step_one` as a parameter. If implementation hits >5 call-site plumbing sites, mirror to `Hart` and document in 02 round. No plan-level change needed. |
| Review | R-010 | Accepted | mhartid seeding moved to PR1. Hard-coded `mhartid = 0` at `csr.rs:250` deleted in PR1. |
| Review | TR-1 | Concurred | Round-robin kept; one-line note added to T-1 about naive "skip halted" breaking SBI HSM handshake. |
| Review | TR-2 | Concurred | `Vec<Hart>` kept. |
| Review | TR-3 | Adopted (b) | 3-PR split (PR1 / PR2a / PR2b). |
| Review | TR-4 | Concurred | Scalar `DebugOps` kept. |
| Review | TR-5 | Concurred | Per-hart `take_ssip(HartId) -> bool` kept. |
| Master | 00-M-001 (inherited) | Applied | `Hart` stays arch-internal concrete. |
| Master | 00-M-002 (inherited) | Applied | `hart.rs` under `arch/riscv/cpu/`. |
| Master | 01-M-001 (inherited) | Applied | No `selected` identifier. |
| Master | 01-M-002 (inherited) | Applied | 3 tight PRs; `Hart` is plain data; invalidation helper is a 6-line loop. |
| Master | 01-M-003 (inherited) | Applied | No new cfg; `#[cfg(riscv)]` seam untouched. |
| Master | 01-M-004 (inherited) | Applied | Per-hart state lives exclusively under `arch/riscv/cpu/`; top-level `cpu/` seam unchanged. |

> Every HIGH / MEDIUM / LOW finding is enumerated. Every inherited
> MASTER directive is enumerated. Rejections / modifications carry
> reasoning.

---

## Spec {Core specification}

[**Goals**]

- **G-1** Introduce `HartId(u32)` and `Hart` at
  `arch/riscv/cpu/hart.rs`; migrate every per-hart field off
  `RVCore` onto `Hart`.
- **G-2** Shrink `RVCore` to `{ harts: Vec<Hart>, current: HartId,
  bus: Bus, ebreak_as_trap: bool }`.
- **G-3** Extend `Mswi`, `Mtimer`, `Sswi` to per-hart state arrays
  (`msip[N]`, `mtimecmp[N]`, `ssip[N]`) with spec-mandated strides.
  Single-hart MMIO offsets remain byte-identical.
- **G-4** `MachineConfig::num_harts: usize` (default 1) flows into
  `RVCore::with_config`, `Bus::new`, and every sub-device.
- **G-5** Round-robin scheduler: one instruction per hart per
  `RVCore::step()`. Deterministic declaration order. At
  `num_harts == 1`, byte-identical to today.
- **G-6** Per-hart SSIP plumbing on the bus:
  `Bus::take_ssip(HartId) -> bool`, `Bus::ssip_flag(HartId) ->
  Arc<AtomicBool>`.
- **G-7** PR1 ships a behaviour-preserving refactor at
  `num_harts = 1`: 354 pre-existing + 8 new PR1 = 362 lib tests
  + 1 `arch_isolation` + 6 `xdb` = **369 tests**, all pass;
  `make linux` and `make debian` boot unchanged; difftest corpus
  zero divergence.
- **G-8** PR2a ships a PLIC runtime-size conversion at
  `num_harts = 1`: zero guest-observable change; all 13 existing
  PLIC tests pass unchanged; 2 new PR2a tests (V-UT-10, V-IT-6).
  Post-PR2a: 362 + 2 = 364 lib tests.
- **G-9** PR2b activates `num_harts = 2` end-to-end: `--harts N`
  CLI, `xemu-2hart.dtb`, Linux SMP boot to `buildroot login:`
  with `smp: Brought up 1 node, 2 CPUs` in dmesg. 3 new PR2b
  tests (V-IT-2, V-IT-4, V-IT-5). Post-PR2b: 364 + 3 = 367 lib
  tests.
- **G-10** Cross-hart LR/SC correctness (I-8): a hart's store
  invalidates every other hart's overlapping reservation. Tested
  at `num_harts = 2` via V-UT-11 (no SMP Linux required).

[**Non-Goals**]

- **NG-1** PLIC gateway redesign — deferred to `plicGateway`.
  PR2a performs a runtime-size device-API conversion (see
  R-001 / Phase-2a file list); nothing more.
- **NG-2** Parallel (multi-threaded) hart execution — round-robin
  single-threaded only.
- **NG-3** Cycle-accurate lockstep with Spike/QEMU — difftest runs
  at `num_harts == 1` only. NG-3 is the reason V-UT-11 exists as a
  unit-level cross-hart LR/SC canary.
- **NG-4** Asymmetric hart configurations (mixed ISA, mixed MMU
  type). All harts share the same ISA profile.
- **NG-5** `Bus::mtime` removal — kept. MTIMER still exposes a
  single `mtime` per ACLINT cluster (spec-correct).
- **NG-6** Multi-hart debugger UX (per-hart `info reg`, hart
  selection in xdb REPL). `DebugOps` signatures unchanged; reads
  route through `self.current`.
- **NG-7** DTB mutation tooling. PR2b ships a static
  `resource/xemu-2hart.dts`.
- **NG-8** OpenSBI reconfiguration. HSM pre-verified:
  `resource/opensbi/lib/sbi/sbi_hsm.c` present in-tree; the 2-hart
  DTB drives SMP bring-up through the existing build. No `.mk`
  edit needed.
- **NG-9** Per-hart breakpoints or watchpoints in xdb UX. `Hart`
  owns its own `breakpoints` vector; xdb reads route through
  `self.current`.

[**Architecture**]

Before:

```
RVCore {
    gpr, fpr, pc, npc, csr, privilege, pending_trap, reservation,
    bus, mmu, pmp, irq, halted, ebreak_as_trap,
    breakpoints, next_bp_id, skip_bp_once,
}
Bus { …, ssip_pending: Arc<AtomicBool> }
Mswi   { msip: u32,  irq: IrqState }
Mtimer { mtime, mtimecmp: u64, irq: IrqState, … }
Sswi   { ssip: Arc<AtomicBool> }
Plic   { enable: Vec<u32> (len=2), …, irq: IrqState }
```

After (PR1 — `num_harts = 1`, Vec-of-1):

```
RVCore {
    harts: Vec<Hart>,      // len == config.num_harts
    current: HartId,
    bus: Bus,
    ebreak_as_trap: bool,
}
Hart {
    id: HartId,
    gpr, fpr, pc, npc, csr, privilege, pending_trap, reservation,
    mmu, pmp, irq, halted,
    breakpoints, next_bp_id, skip_bp_once,
}
Bus { …, ssip_pending: Vec<Arc<AtomicBool>> }  // len == num_harts
Mswi   { msip: Vec<u32>,  irq: Vec<IrqState> }
Mtimer { mtime, mtimecmp: Vec<u64>, irq: Vec<IrqState>, … }
Sswi   { ssip: Vec<Arc<AtomicBool>> }
Plic   { enable: Vec<u32> (len=2), …, irq: IrqState }  // unchanged in PR1
```

After (PR2a — PLIC runtime-size):

```
Plic {
    num_ctx: usize,                  // 2 * num_harts
    priority, pending,
    enable: Vec<u32>   (len num_ctx),
    threshold: Vec<u8> (len num_ctx),
    claimed: Vec<u32>  (len num_ctx),
    irqs: Vec<IrqState>,             // len num_harts
}
```

MMIO decode (spec-exact, unchanged in all PRs):

- MSWI:   `offset / 4 = hart_id`, valid for `hart_id < num_harts`.
- MTIMER: `mtimecmp[h]` at `0x0000 + h * 8`, `mtime` at `0x7FF8`.
- SSWI:   `offset / 4 = hart_id`, valid for `hart_id < num_harts`.
- PLIC ctx decode (PR2a): `ctx = offset-decoded`, then
  `hart_id = ctx >> 1`, `ip = if ctx & 1 == 0 { MEIP } else { SEIP }`.

`RVCore::step` body (N harts, deterministic):

```
bus.tick();
for h in &mut harts {
    h.csr.set(time, bus.mtime());
    if bus.take_ssip(h.id) { h.csr.mip |= SSIP; }
    h.sync_interrupts();
}
harts[current].step_one(&mut bus, ebreak_as_trap)?;
current = HartId((current.0 + 1) % num_harts);
```

For `num_harts == 1`, `current` stays `HartId(0)` and the modulo
is a no-op. I-4 is structural.

Cross-hart reservation invalidation (I-8), called from every store
path on the *current* hart:

```
// RVCore
fn invalidate_reservations_except(&mut self, src: HartId,
                                  addr: usize, size: usize) {
    let end = addr + size;
    for h in &mut self.harts {
        if h.id == src { continue; }
        if let Some(r) = h.reservation
            && r < end && r + RESERVATION_GRANULE > addr
        {
            h.reservation = None;
        }
    }
}
```

`RESERVATION_GRANULE = 8` (double-word, covers RV64 LR.D). Called
from `Hart::store_op` and from `sc_w` / `sc_d` success paths. At
`num_harts = 1` the loop body never executes (all harts have
`id == src`); zero overhead.

[**Invariants**]

- **I-1** `RVCore::harts.len() == config.num_harts` for the
  lifetime of the core (no hotplug).
- **I-2** `harts[i].id == HartId(i as u32)` for all `i`. HartId is
  the canonical array index.
- **I-3** Every per-hart sub-device stores `Vec<T>` of length
  `num_harts` and decodes `hart_id = offset / stride` (MSWI 4,
  MTIMER mtimecmp 8, SSWI 4). PLIC uses `hart_id = ctx >> 1`.
- **I-4** At `num_harts = 1`, all guest-visible behaviour is
  byte-identical pre-/post-refactor across PR1 + PR2a: same MMIO
  offsets answer, same IRQ edges fire, same CSR deltas per step,
  same `mhartid` read (== 0).
- **I-5** MSIP / MTIP / MEIP / SEIP assertion targets the correct
  hart's `IrqState`: MSIP[h] → `Mswi.irq[h]`; `mtimecmp[h]` fires
  → `Mtimer.irq[h].MTIP`; PLIC ctx `c` drives
  `Plic.irqs[c >> 1]` with bit `if c & 1 == 0 { MEIP } else { SEIP }`.
- **I-6** `mhartid` CSR reads `hart.id.0 as Word`. Seeded at
  `Hart::new`; hard-coded `mhartid = 0` at `csr.rs:250` deleted
  in PR1.
- **I-7** `arch_isolation` passes unchanged: no new seam file, no
  new `SEAM_FILES` / `SEAM_ALLOWED_SYMBOLS` entries,
  `BUS_DEBUG_STRING_PINS` count unchanged. `Hart` / `HartId` never
  re-exported through the seam.
- **I-8** *(new)* A store from hart `h_src` to physical byte range
  `[addr, addr + size)` clears `harts[h].reservation` for every
  `h != h_src` whose reservation lies within a granule of the
  store (`|r - addr| < max(size, 8)`). Implemented via
  `RVCore::invalidate_reservations_except`, called synchronously
  from every store path.
- **I-9** `Bus::num_harts()` returns the value passed to
  `Bus::new`; used by `RVCore::with_bus` to size the per-hart
  IrqState expansion.

[**Data Structure**]

```rust
// arch/riscv/cpu/hart.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HartId(pub u32);

pub(in crate::arch::riscv) struct Hart {
    pub(in crate::arch::riscv) id: HartId,
    pub(in crate::arch::riscv) gpr: [Word; 32],
    pub(in crate::arch::riscv) fpr: [u64; 32],
    pub(in crate::arch::riscv) pc: VirtAddr,
    pub(in crate::arch::riscv) npc: VirtAddr,
    pub(in crate::arch::riscv) csr: CsrFile,
    pub(in crate::arch::riscv) privilege: PrivilegeMode,
    pub(in crate::arch::riscv) pending_trap: Option<PendingTrap>,
    pub(in crate::arch::riscv) reservation: Option<usize>,
    pub(in crate::arch::riscv) mmu: Mmu,
    pub(in crate::arch::riscv) pmp: Pmp,
    pub(in crate::arch::riscv) irq: IrqState,
    pub(in crate::arch::riscv) halted: bool,
    pub(in crate::arch::riscv) breakpoints: Vec<Breakpoint>,
    pub(in crate::arch::riscv) next_bp_id: u32,
    pub(in crate::arch::riscv) skip_bp_once: bool,
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
    num_ctx: usize,                 // 2 * num_harts
    priority: Vec<u8>,
    pending: u32,
    enable: Vec<u32>,               // len == num_ctx
    threshold: Vec<u8>,             // len == num_ctx
    claimed: Vec<u32>,              // len == num_ctx
    irqs: Vec<IrqState>,            // len == num_harts
}

// config/mod.rs (addition)
pub struct MachineConfig {
    pub ram_size: usize,
    pub disk: Option<Vec<u8>>,
    pub num_harts: usize,           // default 1
}

// Reservation granule constant
pub(in crate::arch::riscv) const RESERVATION_GRANULE: usize = 8;
```

[**API Surface**]

```rust
// arch/riscv/cpu/hart.rs — new module (PR1)
impl Hart {
    pub(in crate::arch::riscv) fn new(id: HartId, irq: IrqState) -> Self;
    pub(in crate::arch::riscv) fn reset(&mut self);
    pub(in crate::arch::riscv) fn sync_interrupts(&mut self);
    pub(in crate::arch::riscv) fn step_one(
        &mut self, bus: &mut Bus, ebreak_as_trap: bool,
    ) -> XResult;
}

// arch/riscv/cpu/mod.rs — RVCore surface (PR1)
impl RVCore {
    pub fn new() -> Self;                                  // unchanged
    pub fn with_config(config: MachineConfig) -> Self;     // unchanged signature
    pub fn with_bus(bus: Bus, irq: IrqState) -> Self;      // signature preserved (R-005(a))
    pub fn raise_trap(&mut self, cause: TrapCause, tval: Word);
    pub(in crate::arch::riscv) fn current(&self) -> &Hart;
    pub(in crate::arch::riscv) fn current_mut(&mut self) -> &mut Hart;
    pub(in crate::arch::riscv) fn invalidate_reservations_except(
        &mut self, src: HartId, addr: usize, size: usize,
    ); // I-8 hook; no-op at num_harts == 1
}

// impl CoreOps for RVCore — signatures unchanged
// impl DebugOps for RVCore — signatures unchanged (route via self.current)

// device/bus.rs (PR1)
impl Bus {
    pub fn new(ram_base: usize, ram_size: usize, num_harts: usize) -> Self;
    pub fn num_harts(&self) -> usize;                      // supports R-005(a)
    pub fn ssip_flag(&self, hart: HartId) -> Arc<AtomicBool>;
    pub fn take_ssip(&self, hart: HartId) -> bool;
    // unchanged: add_mmio, set_timer_source, set_irq_sink, mtime,
    //   tick, read, write, read_ram, load_ram, replace_device,
    //   reset_devices.
}

// arch/riscv/device/intc/aclint/mod.rs (PR1)
impl Aclint {
    pub fn new(num_harts: usize, irqs: Vec<IrqState>,
               ssip: Vec<Arc<AtomicBool>>) -> Self;
    pub fn install(self, bus: &mut Bus, base: usize) -> usize; // mtimer_idx
}

// arch/riscv/device/intc/plic.rs (PR2a — new signature)
impl Plic {
    pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self;
    // read / write / notify / reset — unchanged Device trait shape;
    //   internal logic indexes irqs[ctx >> 1].
}
```

[**Constraints**]

- **C-1** `num_harts ∈ [1, 16]`. 1 is default and the only PR1 /
  PR2a value. PR2b exercises 2. Upper bound 16 is a pragmatic
  guard.
- **C-2** MMIO offset layout invariant: MSWI at `base + 0x0000`
  (size 0x4000); MTIMER at `base + 0x4000` (size 0x8000); SSWI at
  `base + 0xC000` (size 0x4000). Per-hart offsets multiply hart id
  by stride.
- **C-3** `Hart` is never exposed outside `arch/riscv/`. No
  `crate::cpu::Hart`; no new seam-allowed symbol. Only
  `MachineConfig::num_harts` is new public surface.
- **C-4** Round-robin order is declaration order: hart 0, 1, …,
  num_harts-1, wrap.
- **C-5** PR1 and PR2a do not modify `xemu.dts` / `xemu.dtb` /
  `xemu-debian.dtb`. PR2b adds `xemu-2hart.dts` as a sibling.
- **C-6** No new crate dependencies (no `rayon`, `crossbeam`,
  `parking_lot`).
- **C-7** Plan body ≤ 420 lines (inherited archLayout C-7 + margin
  for 3-PR scope and response matrix). This PLAN is ≤ 420 lines
  by construction.
- **C-8** `DebugOps` signatures unchanged across all three PRs.
- **C-9** `CoreOps` signatures unchanged across all three PRs.

---

## Implement {detail design}

### Execution Flow

[**Main Flow**]

PR1 — Hart abstraction at num_harts=1 + I-8 hook (pure refactor):

1. Add `arch/riscv/cpu/hart.rs` with `HartId`, `Hart`,
   `RESERVATION_GRANULE`.
2. Register `pub(in crate::arch::riscv) mod hart;` in
   `arch/riscv/cpu/mod.rs`.
3. Implement `Hart::new(id, irq)` (seeds `csr[mhartid] = id.0`),
   `Hart::reset`, `Hart::sync_interrupts`, `Hart::step_one`. Move
   method bodies currently on `RVCore` (fetch, decode, execute,
   retire, commit_trap, check_pending_interrupts, trap_on_err,
   dispatch) onto `Hart`.
4. Shrink `RVCore` to `{ harts, current, bus, ebreak_as_trap }`.
5. Implement `RVCore::current(&self) -> &Hart`,
   `RVCore::current_mut(&mut self) -> &mut Hart`,
   `RVCore::invalidate_reservations_except`.
6. Reimplement `CoreOps`:
   - `pc()` → `self.current().pc`.
   - `reset()` → `bus.reset_devices(); for h in harts { h.reset() }`.
   - `step()` → bus.tick; per-hart time/ssip/sync; step current;
     advance cursor. Byte-identical at N=1.
   - `halted()` → `self.current().halted`.
   - `halt_ret()` → `self.current().gpr[a0]`.
7. Rewire `DebugOps` to route through `self.current()` /
   `current_mut()`.
8. Bus: widen `Bus::new(ram_base, ram_size, num_harts)`; add
   `Bus::num_harts()`; change `ssip_pending: Arc<AtomicBool>` to
   `Vec<Arc<AtomicBool>>`; `ssip_flag(HartId) -> Arc<AtomicBool>`;
   `take_ssip(HartId) -> bool`.
9. ACLINT sub-devices: `Mswi { msip: Vec<u32>, irq: Vec<IrqState> }`;
   `Mtimer { mtime, mtimecmp: Vec<u64>, irq: Vec<IrqState>, … }`;
   `Sswi { ssip: Vec<Arc<AtomicBool>> }`. Decode
   `hart_id = offset / stride` with bounds check.
10. `RVCore::with_config`: builds `num_harts` `IrqState`s and SSIP
    flags; `Aclint::new(num_harts, irqs.clone(), ssips).install(…)`;
    push `num_harts` `Hart::new(HartId(i), irqs[i].clone())`.
11. `RVCore::with_bus(bus, irq)`: internally builds
    `vec![irq; bus.num_harts()]` and the rest of the wiring
    (R-005(a)).
12. **I-8 wiring**: insert
    `self.invalidate_reservations_except(self.current, addr, size)`
    at the end of `Hart::store_op` (`inst/base.rs:73` context),
    `sc_w` success path (`inst/atomic.rs:64`), and `sc_d` success
    path (`inst/atomic.rs:85`). Because `Hart::store_op` holds
    `&mut Hart` not `&mut RVCore`, the hook is invoked from
    `RVCore::step` after `step_one` returns (route: record the
    current hart's last-store range in a scratch field
    `last_store: Option<(usize, usize)>` on `Hart`, consumed and
    broadcast by `RVCore` immediately after `step_one`). This
    keeps `Hart::step_one` self-contained and avoids a back-
    reference. Single instruction ⇒ at most one store ⇒ at most
    one broadcast per step.
13. Delete hard-coded `mhartid = 0` at
    `xemu/xcore/src/arch/riscv/cpu/csr.rs:250` (R-010). `Hart::new`
    writes `id.0`.

PR2a — PLIC runtime-size conversion (num_harts = 1 still the only
exercised value):

14. Rewrite `xemu/xcore/src/arch/riscv/device/intc/plic.rs`:
    - Delete `const NUM_CTX: usize = 2;` and
      `const CTX_IP: [u64; NUM_CTX] = [MEIP, SEIP];`.
    - Add fields `num_ctx: usize` and replace scalar
      `irq: IrqState` with `irqs: Vec<IrqState>`.
    - Rewrite `pub fn new(num_harts: usize, irqs: Vec<IrqState>) ->
      Self`: `num_ctx = 2 * num_harts`; `enable`, `threshold`,
      `claimed` all `vec![_; num_ctx]`; debug-assert
      `irqs.len() == num_harts`.
    - Rewrite `fn ctx_at(offset, base, stride)` bounds check to
      use `self.num_ctx` (becomes a method, not associated fn).
    - Rewrite `fn evaluate` to iterate `0..self.num_ctx` and
      target `self.irqs[ctx >> 1]` with
      `ip = if ctx & 1 == 0 { MEIP } else { SEIP }`.
    - `claim` and `complete` unchanged in semantics; update
      signature where they call `ctx_at`.
15. Update `Plic::new(irq.clone())` call at
    `xemu/xcore/src/arch/riscv/cpu/mod.rs:68` to
    `Plic::new(config.num_harts, irqs_for_plic.clone())` where
    `irqs_for_plic` is the same `num_harts`-length Vec used for
    ACLINT (each hart's IrqState is cloned once; IrqState is
    `Arc<AtomicU64>`-backed so clones share state).
16. Preserve the 13 existing PLIC unit tests: their `setup()`
    helper becomes
    `Plic::new(1, vec![irq.clone()])` — byte-identical behaviour
    at `num_harts = 1` (V-IT-6).

PR2b — Activate multi-hart (`num_harts > 1`):

17. `MachineConfig::num_harts` is already present from PR1;
    add `MachineConfig::with_harts(n: usize) -> Self` builder.
18. CLI flag `--harts N` on the xdb / xemu binary front-end.
    Parse and pass through to `MachineConfig`.
    Default remains 1. Collision check: `--harts` is new; no
    existing clap attribute uses it (verified via
    `xemu/xdb/src/cli.rs` grep).
19. Add `resource/xemu-2hart.dts` cloning `xemu.dts` and adding a
    `cpu1` node (same ISA, mmu-type as cpu0), plus a
    `cpus/cpu-map` `cluster0/core1` child. Both harts feed the
    same `clint@2000000` / `plic@c000000`.
20. `resource/Makefile`: new `xemu-2hart.dtb` build rule mirroring
    the existing `xemu.dtb` rule. Optional `linux-2hart` phony
    target passing `xemu-2hart.dtb` to the loader.
21. Boot path (firmware mode): `RVCore::setup_boot` seeds every
    hart with `a0 = hart.id.0` (hartid) and `a1 = fdt_addr`;
    hart 0 starts executing at `RESET_VECTOR`; non-zero harts start
    halted at `RESET_VECTOR` until ACLINT MSIP releases them
    (OpenSBI's `_start_warm` / `wait_for_coldboot` loop handles
    this — HSM present in-tree per R-008).
22. Gate: `make linux` with `xemu-2hart.dtb` reaches
    `buildroot login:` within 120s; dmesg contains
    `smp: Brought up 1 node, 2 CPUs`.

[**Failure Flow**]

1. Out-of-range hart_id in MMIO (`hart_id >= num_harts`):
   sub-device returns 0 on read, drops on write (existing
   unmapped-offset behaviour).
2. `num_harts == 0` or `num_harts > 16`: `MachineConfig`
   constructor / `MachineConfig::with_harts` rejects via
   `debug_assert!` at construction (C-1).
3. OpenSBI fails to bring hart 1 online in PR2b: Linux boots
   with 1 CPU; V-IT-5 asserts the dmesg line and the test fails.
4. PLIC routing mis-wired in PR2a (e.g. ctx 2 driving `irqs[0]`):
   caught by V-UT-10 at unit level before PR2a lands.
5. I-8 violation (cross-hart SC incorrectly succeeds after a peer
   store): caught by V-UT-11 at unit level.
6. Difftest divergence at `num_harts > 1`: unsupported per NG-3;
   the difftest driver asserts `num_harts == 1` at setup time.

[**State Transition**]

- **S0 (today)** `RVCore` with ~18 fields, implicit single hart,
  scalar ssip / msip / mtimecmp / irqs.
- **S0 → S1 (PR1)** `RVCore { harts: Vec<Hart> (len 1), current:
  HartId(0), bus, ebreak_as_trap }`. Bus ssip Vec-of-1, ACLINT
  Vec-of-1. I-8 hook live but no-op. mhartid seeded by
  `Hart::new`. Guest-visible behaviour: identical (I-4).
- **S1 → S2 (PR2a)** `Plic { num_ctx: 2, enable: Vec (len 2),
  irqs: Vec (len 1) }`. Guest-visible behaviour: identical (all
  PLIC tests unchanged).
- **S2 → S3 (PR2b)** `num_harts = N` opt-in: harts Vec-of-N,
  ACLINT Vecs-of-N, PLIC `num_ctx = 2N`, `irqs` Vec-of-N,
  `xemu-2hart.dtb` rebuilt. Default stays 1; only users passing
  `--harts 2` see the change.

### Implementation Plan

[**Phase 1 — PR1: Hart abstraction at num_harts=1 + I-8 hook**]

Files touched (enumerated; exact call-site audit at
implementation time):

- **New**: `xemu/xcore/src/arch/riscv/cpu/hart.rs`.
- **Modified**:
  - `arch/riscv/cpu/mod.rs` — shrink RVCore; add `current`,
    `current_mut`, `invalidate_reservations_except`; rewire
    `with_config`, `with_bus`; update `Aclint::new`,
    `Aclint::install`, `Plic::new` call sites (`Plic::new` still
    takes scalar IrqState in PR1; changes in PR2a).
  - `arch/riscv/cpu/debug.rs` — route all 7 read methods through
    `current()` / `current_mut()`.
  - `arch/riscv/cpu/csr.rs` — delete hard-coded `mhartid = 0` at
    line 250; `mhartid` becomes guest-`[RO]` but host-writable
    via `Hart::new`.
  - `arch/riscv/cpu/inst/base.rs` — `store_op` records
    `last_store = Some((addr, size))` on `Hart` after the store.
  - `arch/riscv/cpu/inst/atomic.rs` — `sc_w` / `sc_d` success
    paths likewise record `last_store`.
  - `arch/riscv/cpu/inst/{mul,privileged,compressed,zicsr,float}.rs`
    — signatures stay `&mut RVCore` at the dispatch boundary;
    internally route through `current_mut()`.
  - `arch/riscv/cpu/mm.rs` + `mm/mmu.rs` + `mm/pmp.rs` — move Mmu
    / Pmp onto `Hart`; call sites route through `current_mut()`.
  - `arch/riscv/cpu/trap/handler.rs` — operate on `&mut Hart`
    where appropriate; `RVCore::raise_trap` delegates to
    `current_mut`.
  - `device/bus.rs` — `Bus::new(ram_base, ram_size, num_harts)`;
    `ssip_pending: Vec<Arc<AtomicBool>>`; new
    `num_harts()` / `ssip_flag(HartId)` / `take_ssip(HartId)`.
  - `arch/riscv/device/intc/aclint/mod.rs` —
    `Aclint::new(num_harts, irqs, ssip)`,
    `Aclint::install` unchanged signature but internal widening.
  - `arch/riscv/device/intc/aclint/{mswi,mtimer,sswi}.rs` —
    per-hart Vec state.
  - `config/mod.rs` — `MachineConfig::num_harts: usize` with
    default 1.
  - Test fixtures (`new_bus` helpers): pass `num_harts = 1`.

Constraint: `arch_isolation.rs` untouched. No new seam file. No
new allow-list entry. `Hart` / `HartId` never cross
`arch::riscv::` boundary.

Gate matrix (PR1, must all pass):

- `cargo fmt --check`.
- `make clippy` clean.
- `X_ARCH=riscv64 cargo test --workspace` — **362 lib + 1
  `arch_isolation` + 6 `xdb` = 369 tests pass**. Breakdown:
  354 pre-existing lib + V-UT-1..V-UT-2 + V-UT-3..V-UT-7 (ACLINT
  + Bus + MachineConfig) + V-UT-9 (HartId ordering) +
  V-UT-11 (cross-hart LR/SC) + V-IT-3 (round-robin N=1) = 8 new
  PR1 lib tests. V-UT-8 is a pass-through (counted in the 354).
- `X_ARCH=riscv64 cargo test --test arch_isolation -- --exact arch_isolation`.
- `make linux` → `buildroot login:` within 60s.
- `make debian` → Debian login + Python3 within 120s.
- Difftest corpus (archModule-03 green set) — zero new
  divergences.

[**Phase 2a — PR2a: PLIC runtime-size conversion at num_harts=1**]

Files touched:

- **Modified**:
  - `xemu/xcore/src/arch/riscv/device/intc/plic.rs`:
    1. Delete `const NUM_CTX: usize = 2;`.
    2. Delete `const CTX_IP: [u64; NUM_CTX] = [MEIP, SEIP];`.
    3. Add `num_ctx: usize` field on `Plic`.
    4. Replace `irq: IrqState` field with `irqs: Vec<IrqState>`.
    5. Rewrite `Plic::new(num_harts: usize, irqs: Vec<IrqState>) ->
       Self`:
       `num_ctx = 2 * num_harts`;
       `enable = vec![0; num_ctx]`; same for `threshold`,
       `claimed`; `debug_assert_eq!(irqs.len(), num_harts)`.
    6. Rewrite `Self::ctx_at` from associated fn to method
       `fn ctx_at(&self, offset: usize, base: usize, stride:
       usize) -> Option<usize>` using `self.num_ctx` in the
       bounds check.
    7. Rewrite `fn evaluate(&mut self)` to iterate
       `0..self.num_ctx`, compute
       `let ip = if ctx & 1 == 0 { MEIP } else { SEIP };`,
       target `self.irqs[ctx >> 1]`.
    8. `fn complete` bounds check uses `self.num_ctx`.
    9. Existing test helper `setup()` becomes
       `Plic::new(1, vec![irq.clone()])`.
  - `xemu/xcore/src/arch/riscv/cpu/mod.rs:68`:
    `Plic::new(irq.clone())` → `Plic::new(config.num_harts,
    plic_irqs.clone())` where `plic_irqs` is built alongside
    the ACLINT IrqState Vec.

Gate matrix (PR2a, must all pass):

- All PR1 gates (regression).
- **364 lib + 1 + 6 = 371 tests pass**. Breakdown: 362 PR1 +
  V-UT-10 (PLIC 2-hart routing unit test, instantiated at
  `num_harts = 2` directly without CLI) + V-IT-6 (existing 13
  PLIC tests unchanged, counted as one integration assertion
  block).
- `make linux` / `make debian` — unchanged boot.
- Difftest corpus — zero new divergences (num_harts still 1).

[**Phase 2b — PR2b: Activate num_harts > 1**]

Files touched:

- **New**:
  - `resource/xemu-2hart.dts` (sibling of `xemu.dts`).
  - `resource/xemu-2hart.dtb` (build-tree artifact).
- **Modified**:
  - `resource/Makefile` — `xemu-2hart.dtb` rule; optional
    `linux-2hart` target.
  - `xemu/xcore/src/config/mod.rs` —
    `MachineConfig::with_harts(n) -> Self` builder.
  - `xemu/xdb/src/cli.rs` — add `--harts N` (clap attribute on
    the existing top-level command).
  - `xemu/xdb/src/main.rs:43` `machine_config()` — thread
    `--harts` into `MachineConfig::with_harts`.
  - `xemu/xcore/src/arch/riscv/cpu/mod.rs` — `setup_boot`
    firmware path seeds `a0 = hart.id.0`, `a1 = fdt_addr` for
    every hart; non-zero harts start `halted = true`.

Gate matrix (PR2b, must all pass):

- All PR2a gates at `num_harts = 1` (regression guard).
- **367 lib + 1 + 6 = 374 tests pass**. Breakdown: 364 PR2a +
  V-IT-2 (`plic_2hart_context_map`) + V-IT-4
  (`round_robin_fairness_two_harts`) + V-IT-5
  (`smp_linux_smoke`).
- `make linux-2hart` → `buildroot login:` in ≤ 120s with
  `smp: Brought up 1 node, 2 CPUs`.
- Difftest: unchanged (`num_harts = 1` only, NG-3).

---

## Trade-offs {ask reviewer for advice}

- **T-1 (scheduling model)** Round-robin one-instruction vs.
  N-instruction burst vs. work-stealing.
  - (a) One-instruction round-robin — chosen. Deterministic,
    trivial, degenerates to today at N=1.
  - (b) N-instruction burst — better cache locality, risk of
    starving cross-hart MTIP delivery.
  - (c) Work-stealing / skip-halted — naive skip breaks the SBI
    HSM handshake: non-zero harts start halted and need hart-0
    ticks to deliver MSIP; skipping halted harts then deadlocks.
  - **Proposal**: (a). Perf task revisits; (c) requires
    interrupt-prompt cross-hart wakeup design before it is safe.
- **T-2 (Hart as struct vs. SoA)** One struct per hart vs.
  parallel arrays on `RVCore`.
  - (a) `Vec<Hart>` — chosen. Clean encapsulation, easy split
    borrow.
  - (b) SoA — no measured win; fights existing RVCore pattern.
- **T-3 (PR count)** 2 PRs (round-00 proposal) vs. 3 PRs
  (R-001/TR-3(b) preferred).
  - (a) 2 PRs — round-00 choice; rejected this round.
  - (b) 3 PRs — chosen. PR1 pure refactor; PR2a PLIC device-API
    reshape at N=1 (byte-identical); PR2b SMP activation. Tighter
    bisection if SMP Linux flakes.
- **T-4 (debug UX)** Scalar vs. per-hart `DebugOps`.
  - (a) Scalar via `self.current` — chosen (concurred TR-4).
  - (b) Per-hart parameter — defer to xdb UX task (NG-6).
- **T-5 (SSIP fan-out shape)** Per-hart vs. bitmap.
  - (a) `take_ssip(HartId) -> bool` — chosen (concurred TR-5).
  - (b) Bitmap — caps `num_harts` at 64 for no current win.
- **T-6 (I-8 mechanism)** RVCore-owned walk vs. Bus-broadcast
  shared-state table.
  - (a) `RVCore::invalidate_reservations_except` — chosen. Cheap
    at N=1 (compile-time-ish skip), no new synchronisation, no
    new `Mutex`/`parking_lot` (C-6). Routes through
    `Hart::last_store` scratch to keep `Hart::step_one`
    self-contained.
  - (b) `Bus::invalidate_reservations(addr)` with
    `Vec<Arc<Mutex<Option<usize>>>>` — decouples hart borrowing
    but adds mutex overhead and a new dependency-shaped wiring.
    Rejected: NG-2 + C-6 forbid mutex infrastructure for this
    cost.
- **T-7 (last_store scratch field on Hart)** Option A: scratch
  field on `Hart`, consumed by `RVCore::step` after
  `step_one`. Option B: pass a `&mut Option<(usize,usize)>`
  through every store path. Option A chosen — single assignment
  per step, no signature churn through the dispatch tree.

## Validation {test design}

[**Unit Tests**]

- **V-UT-1** `Hart::new` — id stored (`hart.id == HartId(i)`),
  GPR/FPR zeroed, PC=0, privilege=Machine, `mhartid` CSR
  == `id.0`, IRQ clone shares state.
- **V-UT-2** `Hart::reset` — clears GPRs / FPRs / PC / privilege /
  pending_trap / reservation / breakpoints / skip_bp_once; IRQ
  reset driven by `RVCore::reset`.
- **V-UT-3** `Mswi` at `num_harts = 4` — write to MSIP[2]
  (`offset = 8`) raises only `irq[2]`.
- **V-UT-4** `Mtimer` at `num_harts = 2` — `mtimecmp[0] = 0` fires
  MTIP on `irq[0]` only; `mtimecmp[1] = u64::MAX` keeps
  `irq[1].MTIP = 0`.
- **V-UT-5** `Sswi` at `num_harts = 3` — SETSSIP[1] raises
  `ssip[1]` only.
- **V-UT-6** `Bus::new(_, _, 4)` — `ssip_pending.len() == 4`;
  `num_harts() == 4`; each `ssip_flag(HartId(i))` shares storage
  with `ssip_pending[i]`.
- **V-UT-7** `MachineConfig::default().num_harts == 1`.
- **V-UT-8** Existing `sswi_edge_delivered_once_and_clearable` and
  `stip_delivered_in_s_mode_with_sie` tests pass unchanged (I-4).
  [pass-through, not additive]
- **V-UT-9** *(new)* `HartId ordering preserved` — after
  `RVCore::with_config(MachineConfig { num_harts: 3, … })`,
  `core.harts[i].id == HartId(i as u32)` for `i in 0..3` (I-2).
- **V-UT-10** *(PR2a)* `Plic::new(2, vec![irq0, irq1])`: enable
  source 1 on ctx 2 (hart 1 M-mode) only; priority 1; threshold
  0. `notify(0x02)` asserts MEIP on `irq1`, not `irq0`
  (I-5, PR2a).
- **V-UT-11** *(PR1)* `cross_hart_lr_sc_invalidation`: construct
  `RVCore::with_config(MachineConfig { num_harts: 2, … })`
  directly (no CLI); hart 0 executes LR.W on address A; hart 1
  executes SW to address A (same double-word granule); scheduler
  runs one step on each; hart 0 executes SC.W on A and observes
  failure (rd != 0; `reservation == None`). Validates I-8 at the
  unit level without SMP Linux.
- **V-UT-12** *(PR1)* `same_hart_store_keeps_other_reservation`
  — construct `RVCore` at `num_harts = 2`; hart 0 LRs A, hart 0
  stores to unrelated B; hart 1 LRs C; hart 0 SCs A → succeeds,
  hart 1's reservation on C untouched. Confirms the
  `if h.id == src { continue; }` path.

[**Integration Tests**]

- **V-IT-1** `arch_isolation` integration test passes unchanged
  (I-7). `SEAM_FILES`, `SEAM_ALLOWED_SYMBOLS`,
  `BUS_DEBUG_STRING_PINS` invariant.
- **V-IT-2** *(PR2b)* `plic_2hart_context_map` — PLIC with
  `num_harts = 2` through real bus path; source enabled only on
  ctx 2; `notify(0x02)` raises MEIP on `harts[1].irq`.
- **V-IT-3** *(PR1)* `round_robin_fairness_single_hart` — N=1
  core runs 100 steps; each step advances `current` back to
  `HartId(0)`; `hart[0].csr.cycle` increments by 100. Covers the
  degenerate case structurally.
- **V-IT-4** *(PR2b)* `round_robin_fairness_two_harts` — N=2
  core runs tight NOP loops on both harts; after 1000 steps each
  hart executed 500 ± 1 instructions.
- **V-IT-5** *(PR2b)* `smp_linux_smoke` — `make linux-2hart`
  boots to `buildroot login:` within 120s; dmesg contains
  `smp: Brought up 1 node, 2 CPUs`.
- **V-IT-6** *(PR2a)* `plic_existing_13_tests_unchanged` — all 13
  tests in `plic.rs` (`priority_read_write`,
  `enable_per_context`, …, `reset_clears_state`) pass unchanged
  with `Plic::new(1, vec![irq.clone()])` via the updated
  `setup()` helper. Zero-diff behaviour at `num_harts = 1`.

[**Failure / Robustness Validation**]

- **V-F-1** `num_harts = 0` or `num_harts > 16` — `debug_assert!`
  at `MachineConfig::with_harts` / builder (C-1).
- **V-F-2** MMIO write to `MSIP[num_harts]` returns `Ok(())`
  silently; no `msip[_]` mutated. Same for SSWI out-of-range.
- **V-F-3** MTIMER MMIO read at `mtimecmp[h]` for `h >= num_harts`
  returns 0.
- **V-F-4** `RVCore::reset()` iterates every hart; after reset,
  `harts[i].pc == RESET_VECTOR` for all `i` and
  `harts[i].reservation.is_none()`.
- **V-F-5** *(PR2b)* OpenSBI brings only hart 0 online: dmesg
  shows `Brought up 1 node, 1 CPU`; V-IT-5 fails. Explicit PR2b
  gate failure mode.
- **V-F-6** *(PR2b)* `make linux-2hart` timeout (> 120s): V-IT-5
  fails. No silent pass.

[**Edge Case Validation**]

- **V-E-1** `num_harts = 1` byte-identical to pre-refactor (I-4):
  every existing aclintSplit test passes unchanged; `make debian`
  boot-to-Python3 trace identical (timing excluded).
- **V-E-2** Offset decode boundary at `num_harts = 3`: MSWI
  accepts `offset ∈ {0, 4, 8}`; `offset = 12` reads 0 (in-region
  but unmapped for hart_id).
- **V-E-3** Round-robin wraparound at `num_harts = 2`: after step
  2, `current == HartId(0)`; after step 3, `current == HartId(1)`.
- **V-E-4** *(PR2b)* hartid seeding — after `with_config(num_harts
  = 2)` and `setup_boot(Firmware { fdt_addr })`, `harts[0].gpr[a0]
  == 0`, `harts[1].gpr[a0] == 1`; `mhartid` CSR reads match.
- **V-E-5** *(PR1)* `store_overlapping_granule_invalidates` —
  hart 0 LR.D on `0x80001000`; hart 1 `sw` to `0x80001004`
  (within 8-byte granule). Hart 0's SC.D fails.
- **V-E-6** *(PR1)* `store_outside_granule_preserves` — hart 0
  LR.W on `0x80001000`; hart 1 `sw` to `0x80001010`
  (outside granule). Hart 0's SC.W succeeds.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (HartId + Hart) | V-UT-1, V-UT-2 |
| G-2 (RVCore shape) | V-UT-1, V-IT-3, `arch_isolation` invariance |
| G-3 (ACLINT per-hart) | V-UT-3, V-UT-4, V-UT-5 |
| G-4 (MachineConfig::num_harts) | V-UT-7, V-F-1 |
| G-5 (round-robin) | V-IT-3, V-IT-4, V-E-3 |
| G-6 (per-hart SSIP) | V-UT-5, V-UT-6 |
| G-7 (PR1 behaviour-preservation) | V-E-1, PR1 gate matrix (linux/debian/difftest, 369-test count) |
| G-8 (PR2a PLIC reshape) | V-UT-10, V-IT-6, PR2a gate matrix (371-test count) |
| G-9 (PR2b SMP boot) | V-IT-5, V-E-4, V-F-5, V-F-6 |
| G-10 (cross-hart LR/SC correctness) | V-UT-11, V-UT-12, V-E-5, V-E-6 |
| C-1 (hart count bounds) | V-F-1 |
| C-2 (MMIO layout invariant) | V-UT-3..5, V-E-2, V-IT-6 |
| C-3 (no new seam) | V-IT-1 |
| C-4 (deterministic order) | V-IT-3, V-IT-4, V-E-3 |
| C-5 (PR1/PR2a DTB untouched) | PR1/PR2a gate matrix (`make linux` / `make debian`) |
| C-6 (no new deps) | Cargo.lock diff review per PR |
| C-7 (≤ 420-line budget) | Checked at plan-review time |
| C-8 (DebugOps signatures) | V-IT-1 + xdb 6-test suite unchanged |
| C-9 (CoreOps signatures) | PR1/PR2a/PR2b gate matrices |
| I-1 (harts.len == num_harts) | V-UT-7 + V-UT-6 (explicit 4-hart length) |
| I-2 (harts[i].id == HartId(i)) | V-UT-9 |
| I-3 (per-hart stride decode) | V-UT-3, V-UT-4, V-UT-5, V-E-2, V-UT-10 |
| I-4 (byte-identical single-hart) | V-E-1, V-IT-6 |
| I-5 (per-hart IRQ routing) | V-UT-3, V-UT-4, V-UT-10, V-IT-2 |
| I-6 (mhartid per hart) | V-UT-1, V-E-4 |
| I-7 (arch_isolation invariant) | V-IT-1 |
| I-8 (cross-hart LR/SC invalidation) | V-UT-11, V-UT-12, V-E-5, V-E-6 |
| I-9 (Bus::num_harts accessor) | V-UT-6 |
