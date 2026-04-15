# `multiHart` PLAN `00`

> Status: Draft
> Feature: `multiHart`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`
> - Master Directive: `none` (inherited MASTER directives from archModule / archLayout still binding — see Master Compliance)

---

## Summary

Introduce a `Hart` abstraction so `RVCore` can own `N ≥ 1` hardware
threads instead of a single implicit hart flattened into its fields.
Every piece of per-hart architectural state — GPRs, FPRs, PC/NPC,
`CsrFile`, `PrivilegeMode`, `PendingTrap`, LR/SC reservation, `Mmu`
(+ TLB), `Pmp`, `IrqState`, `halted` — moves into a new
`pub(in crate::arch::riscv) struct Hart { id: HartId, … }` living at
`arch/riscv/cpu/hart.rs`. `RVCore` keeps its single ownership of `Bus`
plus a `Vec<Hart>` and a `current: HartId` cursor; `step()` advances one
instruction on the current hart, then round-robins. Per-hart ACLINT
state (`msip[N]`, `mtimecmp[N]`, the SSIP fan-out becomes `Vec<Arc<AtomicBool>>`
indexed by hart) slots into the existing `Mswi` / `Mtimer` / `Sswi`
sub-devices at their spec-mandated strides (`base + hartid * 4`,
`base + 0x4000 + hartid * 8`, `base + 0xC000 + hartid * 4`). **PLIC
per-hart context fan-out is explicitly deferred to `plicGateway`**
(NG-3); this plan only raises `NUM_CTX` to `2 * num_harts` via a
mechanical extension that preserves single-hart offsets. `MachineConfig`
gains `num_harts: usize` (default `1`). Ships as **two PRs**: PR1 is a
pure refactor (Hart abstraction at `num_harts=1`, zero
guest-observable change, same DTB, same `make linux` / `make debian`
pass); PR2 activates `num_harts > 1` behind an opt-in CLI flag and a
2-hart DTB variant, with SMP Linux boot as an integration validation.

## Log {None in 00_PLAN}

[**Feature Introduce**]

- `HartId(u32)` newtype in `arch/riscv/cpu/hart.rs`. No broader
  `HartIdx` re-export; it's an arch-internal identifier.
- `struct Hart { id: HartId, gpr, fpr, pc, npc, csr, privilege,
  pending_trap, reservation, mmu, pmp, irq, halted,
  /* debug fields */ }` — every field currently on `RVCore` that is
  logically per-hart moves onto `Hart`.
- `RVCore` reshaped to `{ harts: Vec<Hart>, current: HartId, bus: Bus,
  ebreak_as_trap: bool }`. The `bus` and the ebreak policy are
  machine-scoped; everything else is hart-scoped.
- `Hart` owns its own `IrqState` (already `Arc<AtomicU64>`-backed, so
  `Clone` is cheap). Each ACLINT sub-device receives a
  `&[IrqState]` (or `Vec<IrqState>`) of length `num_harts` and asserts
  MSIP/MTIP on the per-hart `IrqState`. SSIP likewise gains a
  `Vec<Arc<AtomicBool>>` fan-out — one pending flag per hart — exposed
  on `Bus` as `Bus::take_ssip(hart: HartId) -> bool`.
- `Mswi::new(num_harts, irq: Vec<IrqState>)`, `Mtimer::new(num_harts,
  irq: Vec<IrqState>)`, `Sswi::new(ssip: Vec<Arc<AtomicBool>>)`.
  Internal state: `msip: Vec<u32>`, `mtimecmp: Vec<u64>`, one per
  hart. Offsets decode as `(offset - base_reg) / stride = hart_id`.
- `MachineConfig::num_harts: usize` with `Default = 1`. CLI flag wiring
  is PR2 scope.
- Round-robin execution: `step()` ticks the bus once, syncs interrupts
  on every hart, executes one instruction on `self.current`, advances
  `self.current = HartId((current.0 + 1) % num_harts)`. For
  `num_harts == 1` this collapses to today's behaviour exactly.
- Two-PR shape: PR1 (refactor, num_harts=1, unchanged DTB) and PR2
  (activate num_harts>1, DTB variant, SMP boot gate).

[**Review Adjustments**]

None — this is round 00.

[**Master Compliance**]

No `00_MASTER.md`. Inherited binding directives from archModule /
archLayout continue to apply:

- **00-M-001** — no global `trait Arch`. Honoured: `Hart` is an
  arch-internal concrete struct under `arch/riscv/cpu/`; `CoreOps`
  stays unchanged as the cross-arch seam.
- **00-M-002** — topic-organised `arch/<name>/`. Honoured: new file is
  `arch/riscv/cpu/hart.rs`; no top-level `cpu/hart.rs`.
- **01-M-001** — no `selected` alias word. Honoured.
- **01-M-002** — clean, concise, elegant. Honoured: `Hart` is a plain
  data struct with a small method set; `RVCore` shrinks from 18 fields
  to 4; round-robin is a three-line scheduler; two PRs, not five.
- **01-M-003** — no redundant arch-validity checks. Honoured: no new
  cfg scaffolding; the `#[cfg(riscv)]` seam in `cpu/mod.rs` is
  untouched.
- **01-M-004** — `cpu/`, `device/`, `isa/` top-level = trait APIs +
  tiny cfg patches only. Honoured: `Hart` and all per-hart state live
  exclusively under `arch/riscv/cpu/`. `CoreOps` gains **no** hart
  parameter (the CPU stepping loop remains arch-agnostic); hart
  selection happens inside `RVCore::step`. `DebugOps` gains
  **no** hart parameter in PR1 (single-hart today); PR2 adds a
  `current_hart() -> u32` inspection accessor and routes the existing
  read-only methods through `self.current`, deferring full multi-hart
  debugger UX to a future xdb task (NG-6).

### Changes from Previous Round

Not applicable — round 00.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Master | 00-M-001 (inherited) | Applied | `Hart` is concrete; `CoreOps` surface unchanged. |
| Master | 00-M-002 (inherited) | Applied | New file under `arch/riscv/cpu/`. |
| Master | 01-M-001 (inherited) | Applied | No `selected` identifier. |
| Master | 01-M-002 (inherited) | Applied | 2 PRs; `Hart` is plain data; round-robin scheduler is a cursor-advance. |
| Master | 01-M-003 (inherited) | Applied | No new cfg; existing `riscv` seam reused. |
| Master | 01-M-004 (inherited) | Applied | All per-hart state under `arch/riscv/cpu/`; top-level `cpu/` seam unchanged. |

> Rules satisfied: no prior review to reconcile (round 00); every
> inherited Master directive is enumerated with its reconciliation.

---

## Spec {Core specification}

[**Goals**]

- **G-1** Introduce `HartId(u32)` and `Hart` in
  `arch/riscv/cpu/hart.rs`; migrate every per-hart field off `RVCore`
  onto `Hart`.
- **G-2** `RVCore` owns `Vec<Hart>`, `Bus`, `current: HartId`,
  `ebreak_as_trap` — nothing else. Machine-scoped state only.
- **G-3** Extend `Mswi`, `Mtimer`, `Sswi` to per-hart state arrays
  (`msip[N]`, `mtimecmp[N]`, `ssip[N]`) with spec-mandated strides.
  Preserve single-hart MMIO offsets byte-identical.
- **G-4** `MachineConfig::num_harts: usize` (default 1) flows into
  `RVCore::with_config` and each ACLINT sub-device.
- **G-5** Round-robin scheduler: one instruction per hart per
  `RVCore::step()` call, fair and deterministic in declaration order.
  For `num_harts == 1` behaviour is byte-identical to today.
- **G-6** Per-hart SSIP plumbing on the bus:
  `Bus::take_ssip(HartId) -> bool`, backed by
  `Vec<Arc<AtomicBool>>`. `Bus::ssip_flag(HartId) -> Arc<AtomicBool>`
  accessor for sub-device construction.
- **G-7** PR1 is behaviour-preserving at `num_harts == 1`: all 354 lib
  tests + `arch_isolation` + xdb pass; `make linux` and `make debian`
  boot unchanged; difftest corpus zero divergence.
- **G-8** PR2 delivers a working `num_harts = 2` SMP boot: Linux with
  a 2-hart DTB reaches `buildroot login:` on both harts online.

[**Non-Goals**]

- **NG-1** PLIC per-hart context fan-out beyond a mechanical
  `NUM_CTX = 2 * num_harts` extension — full gateway/context redesign
  lives in `plicGateway`.
- **NG-2** Parallel (multi-threaded) hart execution — round-robin
  single-threaded only. Multi-threading is a separate perf task with
  its own risk surface (bus locking, TSO, difftest ordering).
- **NG-3** Cycle-accurate lockstep with Spike/QEMU — difftest runs
  with `num_harts == 1` to avoid ordering divergence (NG-2 follow-on).
- **NG-4** Asymmetric hart configurations (mixed ISA, mixed mmu-type).
  All harts share the same ISA profile and MMU configuration.
- **NG-5** `Bus::mtime` removal — kept. MTIMER still exposes a single
  `mtime` clock source (spec-correct; mtime is per ACLINT cluster,
  not per hart). Only `mtimecmp` and `msip` become per-hart. Residual
  hooks `Bus::take_ssip` / `Bus::ssip_flag` are **reshaped** (gain a
  `HartId` argument), not removed — full removal belongs under
  `directIrq`.
- **NG-6** Multi-hart debugger UX (per-hart `info reg`, hart
  selection in xdb REPL). PR2 exposes a minimal `current_hart()`
  accessor; richer UX is a future xdb task.
- **NG-7** DTB mutation tooling. PR2 ships a second, static
  `xemu-2hart.dts` alongside `xemu.dts`; dynamic DTB generation is
  out of scope.
- **NG-8** OpenSBI reconfiguration beyond what the 2-hart DTB
  naturally drives. OpenSBI builds from `resource/opensbi/` already
  support SMP via `platform-override` if the DTB declares multiple
  harts — no `.mk` edit needed.
- **NG-9** Per-hart breakpoints or watchpoints. PR1 keeps the existing
  `breakpoints: Vec<Breakpoint>` on `Hart[0]` (the debug target); PR2
  routes debug reads through `self.current` but does not add per-hart
  breakpoint sets. `xdb` tests are asserted unchanged.

[**Architecture**]

Before:

```
RVCore {
    gpr, fpr, pc, npc, csr, privilege, pending_trap, reservation,
    bus, mmu, pmp, irq, halted, ebreak_as_trap,
    breakpoints, next_bp_id, skip_bp_once,
}
Bus { …, ssip_pending: Arc<AtomicBool> }
Mswi { msip: u32, irq: IrqState }
Mtimer { mtime: u64, mtimecmp: u64, irq: IrqState, … }
Sswi { ssip: Arc<AtomicBool> }
```

After:

```
RVCore {
    harts: Vec<Hart>,                // len == config.num_harts
    current: HartId,
    bus: Bus,
    ebreak_as_trap: bool,
}
Hart {
    id: HartId,
    gpr, fpr, pc, npc, csr, privilege, pending_trap, reservation,
    mmu, pmp, irq, halted,
    breakpoints, next_bp_id, skip_bp_once,   // NG-9 keeps these per-hart
}
Bus { …, ssip_pending: Vec<Arc<AtomicBool>> }   // len == num_harts
Mswi   { msip: Vec<u32>, irq: Vec<IrqState> }
Mtimer { mtime: u64, mtimecmp: Vec<u64>, irq: Vec<IrqState>, … }
Sswi   { ssip: Vec<Arc<AtomicBool>> }
```

MMIO decode (spec-exact):

- MSWI:  `offset / 4   = hart_id`, valid for `hart_id < num_harts`.
- MTIMER: `mtimecmp[h]` at `0x0000 + h * 8`, `mtime` at `0x7FF8`.
- SSWI:  `offset / 4   = hart_id`, valid for `hart_id < num_harts`.

`RVCore::step` flow (N harts):

```
bus.tick();
for h in &mut harts {
    h.csr.set(time, bus.mtime());
    if bus.take_ssip(h.id) { h.csr.mip |= SSIP; }
    h.sync_interrupts();
}
harts[current].step_one_instruction()?;
current = HartId((current.0 + 1) % num_harts);
```

For `num_harts == 1`, `current` stays at `HartId(0)` and the loop runs
the same body as today.

[**Invariants**]

- **I-1** `RVCore::harts.len() == config.num_harts` for the lifetime
  of the core. Resizing harts is out of scope (no hotplug).
- **I-2** `harts[i].id == HartId(i as u32)` for all `i`. HartId is the
  canonical array index; no indirection table.
- **I-3** Every sub-device that stores per-hart state uses
  `Vec<T>` of length `num_harts` and decodes `hart_id = offset / stride`
  with the spec-mandated stride (MSWI: 4, MTIMER mtimecmp: 8, SSWI: 4).
- **I-4** For `num_harts == 1`, all guest-visible behaviour is
  byte-identical pre-/post-refactor: same MMIO offsets answer, same
  IRQ edges fire, same CSR deltas per step, same `mhartid` reads.
- **I-5** MSIP / MTIP / MEIP / SEIP assertion targets the correct
  hart's `IrqState`: writing `MSIP[h]` only raises hart `h`'s MSIP bit;
  `mtimecmp[h]` only fires hart `h`'s MTIP; PLIC M/S contexts wire to
  hart 0 in PR1 (unchanged), hart `floor(ctx/2)` in PR2 under the
  mechanical `NUM_CTX = 2 * num_harts` extension.
- **I-6** `mhartid` CSR reads `hart.id.0 as Word` for the executing
  hart (today `mhartid` is a hard-coded `0` in
  `xcore/src/arch/riscv/cpu/csr.rs:250`; PR1 keeps that behaviour at
  num_harts=1, PR2 routes it through `Hart::id`).
- **I-7** `arch_isolation` passes unchanged. No new seam files, no
  new entries in `SEAM_FILES` / `SEAM_ALLOWED_SYMBOLS`;
  `BUS_DEBUG_STRING_PINS` counts unchanged. `Hart` is never
  re-exported through the seam.

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
    // Debug state (NG-9): per-hart vectors kept to avoid collapsing on
    // `current` and breaking the one-hart-debugged-at-a-time xdb model.
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

// config/mod.rs (addition)
pub struct MachineConfig {
    pub ram_size: usize,
    pub disk: Option<Vec<u8>>,
    pub num_harts: usize,   // default 1
}
```

[**API Surface**]

```rust
// arch/riscv/cpu/hart.rs
impl Hart {
    pub(in crate::arch::riscv) fn new(id: HartId, irq: IrqState) -> Self;
    pub(in crate::arch::riscv) fn reset(&mut self);
    pub(in crate::arch::riscv) fn sync_interrupts(&mut self);
    pub(in crate::arch::riscv) fn step_one(&mut self, bus: &mut Bus,
        ebreak_as_trap: bool) -> XResult;
}

// arch/riscv/cpu/mod.rs — RVCore surface kept minimal
impl RVCore {
    pub fn new() -> Self;                                    // unchanged
    pub fn with_config(config: MachineConfig) -> Self;       // unchanged signature
    pub fn with_bus(bus: Bus, irqs: Vec<IrqState>) -> Self;  // takes Vec<IrqState>
    pub fn raise_trap(&mut self, cause: TrapCause, tval: Word); // acts on current hart
    pub(in crate::arch::riscv) fn current(&self) -> &Hart;
    pub(in crate::arch::riscv) fn current_mut(&mut self) -> &mut Hart;
}

// CoreOps impl (unchanged signatures — dispatch through `current` internally)
impl CoreOps for RVCore { /* step(), reset(), pc(), halted(), … */ }

// device/bus.rs
impl Bus {
    pub fn new(ram_base: usize, ram_size: usize, num_harts: usize) -> Self;
    pub fn ssip_flag(&self, hart: HartId) -> Arc<AtomicBool>;
    pub fn take_ssip(&self, hart: HartId) -> bool;
    // unchanged: add_mmio, set_timer_source, set_irq_sink, mtime, tick,
    //   read, write, read_ram, load_ram, replace_device, reset_devices.
}

// arch/riscv/device/intc/aclint/mod.rs
impl Aclint {
    pub fn new(num_harts: usize, irqs: Vec<IrqState>,
        ssip: Vec<Arc<AtomicBool>>) -> Self;
    pub fn install(self, bus: &mut Bus, base: usize) -> usize; // mtimer_idx
}
```

[**Constraints**]

- **C-1** `num_harts ∈ [1, 16]`. `1` is default and the only value
  exercised in PR1; PR2 exercises `2`. Upper bound 16 is a pragmatic
  guard (MMIO regions MSWI/SSWI are 0x4000 bytes = 4096 harts
  addressable, but DTB cost and round-robin fairness make 16 a
  reasonable cap for now).
- **C-2** MMIO offset layout is invariant: MSWI at `base + 0x0000`,
  size `0x4000`; MTIMER at `base + 0x4000`, size `0x8000`; SSWI at
  `base + 0xC000`, size `0x4000`. Offsets for hart `h` are
  `h * stride` inside each region.
- **C-3** `Hart` is never exposed outside `arch/riscv/`. No
  `crate::cpu::Hart` alias; no new seam-allowed symbol. The only new
  public surface is `MachineConfig::num_harts`.
- **C-4** Round-robin order is declaration order: hart 0, then 1, …,
  then `num_harts - 1`, then back to 0. Deterministic, so difftest
  can reproduce the same trace against itself (even if NG-3 keeps
  difftest at num_harts=1).
- **C-5** PR1 shall not modify `xemu.dts` / `xemu.dtb`. The
  single-hart DTB continues to describe `cpu0` only; PR2 adds
  `xemu-2hart.dts` as a sibling.
- **C-6** No new crate dependencies. No `rayon`, no `crossbeam`, no
  `parking_lot`. The scheduler is a `for` loop.
- **C-7** Body length ≤ 360 lines (inherited archLayout C-7 + a small
  margin for two-PR scope). This PLAN is ≤ 360 lines by construction.
- **C-8** PR1 does not change `cpu/debug.rs` `DebugOps` signatures.
  All read-only methods internally dispatch through `self.current`
  (or `harts[0]` when `num_harts == 1`). PR2 preserves signatures;
  no xdb-side change required.

---

## Implement {detail design}

### Execution Flow

[**Main Flow**]

PR1 — Hart abstraction at num_harts=1 (pure refactor):

1. Create `arch/riscv/cpu/hart.rs` with `HartId` and `Hart`. Fields
   are the migrated per-hart fields of today's `RVCore` plus `id` and
   `irq`.
2. Add `pub mod hart;` in `arch/riscv/cpu/mod.rs` and import
   `Hart`, `HartId`.
3. Implement `Hart::new(id, irq)`, `Hart::reset`, `Hart::sync_interrupts`,
   `Hart::step_one(bus, ebreak_as_trap)`. Move the bodies of today's
   `RVCore::fetch`/`decode`/`execute`/`retire`/`commit_trap`/
   `check_pending_interrupts`/`trap_on_err`/`dispatch` onto `Hart`
   — they operate on per-hart state + `&mut Bus`. Keep public names;
   adjust `self.gpr` → `self.gpr` (same path, now on `Hart`), etc.
4. Shrink `RVCore` to `{ harts, current, bus, ebreak_as_trap }`.
5. Implement `RVCore::current(&self) -> &Hart` /
   `RVCore::current_mut(&mut self) -> &mut Hart`.
6. Reimplement `CoreOps` for `RVCore`:
   - `pc()` → `self.current().pc`.
   - `reset()` → `self.bus.reset_devices(); for h in &mut self.harts { h.reset() }`.
   - `step()` → `bus.tick()`; sync interrupts on every hart; run one
     instruction on `current`; advance `current`. At `num_harts == 1`
     this is byte-identical to today.
   - `halted()` → `self.current().halted` (PR1: only hart 0 exists).
   - `halt_ret()` → `self.current().gpr[a0]`.
7. Rewire `DebugOps`: route `add_breakpoint`, `read_register`,
   `context`, etc. through `self.current()` / `current_mut()`.
8. Bus: widen `Bus::new` to `(ram_base, ram_size, num_harts)`; change
   `ssip_pending: Arc<AtomicBool>` to `Vec<Arc<AtomicBool>>` of length
   `num_harts`; `ssip_flag(HartId) -> Arc<AtomicBool>`;
   `take_ssip(HartId) -> bool`.
9. ACLINT sub-devices:
   - `Mswi`: `msip: Vec<u32>` of length `num_harts`; `irq: Vec<IrqState>`
     of length `num_harts`. Decode `hart_id = offset / 4` with bounds
     check (unmapped returns 0 / is a no-op).
   - `Mtimer`: `mtimecmp: Vec<u64>` of length `num_harts`; `irq: Vec<IrqState>`;
     mtime stays scalar. `tick()` evaluates
     `mtime >= mtimecmp[h]` for each `h`.
   - `Sswi`: `ssip: Vec<Arc<AtomicBool>>` of length `num_harts`.
10. `RVCore::with_config`: builds `num_harts` `IrqState` instances,
    `num_harts` `Arc<AtomicBool>` SSIP flags (via
    `bus.ssip_flag(HartId(i))` for each `i`), then
    `Aclint::new(num_harts, irqs.clone(), ssips).install(…)`.
    Push `num_harts` `Hart` instances, each with its `IrqState` clone.
11. At `num_harts == 1` PR1 ends here. `arch_isolation` passes
    unchanged (no new seam file or allow-list entry); 354 lib + 1
    integration + 6 xdb tests pass; `make linux` + `make debian`
    boot unchanged; difftest corpus zero divergence.

PR2 — Activate multi-hart (`num_harts > 1`):

12. CLI flag: expose `--harts N` (wire through the
    existing CLI-to-MachineConfig glue). Default remains 1.
13. `mhartid` CSR: replace the hard-coded `0` at `csr.rs:250` with a
    per-hart initial value written by `Hart::new(HartId(i), …)`
    (`hart.csr.set(CsrAddr::mhartid, i as Word)`). `mhartid` remains
    read-only.
14. PLIC: raise `NUM_CTX` from `2` to `2 * num_harts`. Context
    mapping: `ctx = 2 * hart_id + mode` where `mode = 0 → M`,
    `mode = 1 → S`. Wire `CTX_IP[ctx] → harts[hart_id].irq[MEIP|SEIP]`.
    This is a mechanical extension of today's loop; full gateway
    redesign is NG-1 / `plicGateway`.
15. Ship `resource/xemu-2hart.dts` adding `cpu1` (same ISA as cpu0),
    both feeding the same `clint@2000000` / `plic@c000000`.
    Rebuild `xemu-2hart.dtb` as a build-tree artifact.
16. Boot path (firmware mode): OpenSBI reads DTB and starts both
    harts; hart 0 is boot hart, hart 1 enters SBI wait-for-IPI; SBI
    `sbi_hsm_hart_start` resumes hart 1 on Linux request.
    `RVCore::setup_boot` for `BootMode::Firmware`: seed every hart
    with `a0 = hart.id.0` (hartid) and `a1 = fdt_addr`; hart 0 starts
    at `RESET_VECTOR`; non-zero harts start halted at
    `RESET_VECTOR` with their own `halted = true` until ACLINT MSIP
    releases them (today OpenSBI handles this via its
    `_start_warm` / `wait_for_coldboot` loop).
17. Gate: `make linux` with `xemu-2hart.dtb` → `buildroot login:`,
    `dmesg` reports `smp: Brought up 1 node, 2 CPUs`. Difftest still
    runs at `num_harts == 1` (NG-3).

[**Failure Flow**]

1. Out-of-range hart_id in MMIO (`hart_id >= num_harts`): sub-device
   returns 0 on read, silently drops on write (mirrors today's
   unmapped-offset behaviour). No trap — spec says reserved offsets
   are implementation-defined; returning 0 is consistent with the
   current `Reg::decode` None arm.
2. `num_harts == 0` or `num_harts > 16`: `MachineConfig` constructor
   / builder rejects with a debug assertion at construction time
   (C-1). Not a runtime trap.
3. OpenSBI fails to bring hart 1 online in PR2: Linux boots with 1
   CPU, `dmesg` reports `smp: Brought up 1 node, 1 CPU`. This is a
   PR2 validation failure, not a PR1 regression.
4. PLIC MEIP targeting hart 1 before `NUM_CTX` extension (PR2 step
   14): interrupt stays pending on hart 0 (today's behaviour). Caught
   by the mechanical PLIC test (V-IT-2).
5. difftest divergence at `num_harts > 1`: unsupported per NG-3.
   Difftest runner asserts `num_harts == 1`; violating it panics at
   corpus-setup time with a clear error.

[**State Transition**]

- **S0 (today)** `RVCore` with 18 fields, implicit single hart, bus
  ssip_flag scalar, ACLINT scalar per-hart state.
- **S0 → S1 (PR1)** `RVCore { harts: Vec<Hart> (len 1), current:
  HartId(0), bus, ebreak_as_trap }`. Bus ssip Vec of length 1. ACLINT
  per-hart Vecs of length 1. Guest-visible behaviour: **identical**
  (I-4). `arch_isolation` pins unchanged (I-7).
- **S1 → S2 (PR2)** `num_harts > 1` opt-in: harts Vec of length N,
  ACLINT Vecs of length N, PLIC `NUM_CTX = 2 * N`, `xemu-2hart.dts`
  rebuilt, `mhartid` per-hart. Single-hart default behaviour
  (S1) fully preserved because `num_harts = 1` is still the
  `MachineConfig::default()` shape.

### Implementation Plan

[**Phase 1 — PR1: Hart abstraction at num_harts=1**]

Files touched (estimated — exact list determined at implementation
by `rg` audit, shape stable):

- New: `xemu/xcore/src/arch/riscv/cpu/hart.rs`.
- Modified: `arch/riscv/cpu/mod.rs` (shrink RVCore, reroute CoreOps),
  `arch/riscv/cpu/debug.rs` (route through `current()`),
  `arch/riscv/cpu/trap/handler.rs` (take `&mut Hart` instead of
  `&mut RVCore` where appropriate — candidate migration; if the
  refactor churn is too deep, keep on `RVCore` and have it delegate),
  `arch/riscv/cpu/inst/**/*.rs` (the `dispatch` tree — should remain
  `&mut RVCore` signatures but internally go through `current_mut()`
  to minimise churn; exact boundary chosen during implementation),
  `arch/riscv/cpu/mm.rs` + `mm/mmu.rs` + `mm/pmp.rs` (same),
  `arch/riscv/cpu/csr.rs` (hard-coded `mhartid = 0` stays; per-hart
  value deferred to PR2), `device/bus.rs` (ssip Vec, `Bus::new`
  signature), `arch/riscv/device/intc/aclint/{mod,mswi,mtimer,sswi}.rs`
  (per-hart state Vecs), `config/mod.rs` (`num_harts` field, default
  1), test fixtures (`new_bus` helpers pass `num_harts = 1`).

Constraint: `arch_isolation.rs` **untouched**. No new seam file, no
new allow-list entry. `Hart` and `HartId` are visible only inside
`arch::riscv`.

Gate matrix (PR1, must all pass):

- `cargo fmt --check`.
- `make clippy` clean.
- `X_ARCH=riscv64 cargo test --workspace` — 354 lib + 1
  `arch_isolation` + 6 `xdb` = all pass; **same count** as post
  aclintSplit (no net add / remove in PR1).
- `X_ARCH=riscv64 cargo test --test arch_isolation -- --exact arch_isolation` — 1 passed.
- `make linux` → `buildroot login:` within 60s.
- `make debian` → Debian login + Python3 within 120s.
- Difftest corpus (archModule-03 green set) — zero new divergences.

[**Phase 2 — PR2: Activate num_harts > 1**]

Files touched:

- New: `resource/xemu-2hart.dts`, build-tree `xemu-2hart.dtb`
  (Makefile target).
- Modified: `arch/riscv/cpu/csr.rs` (remove hard-coded `mhartid = 0`,
  let `Hart::new` write the per-hart value; `mhartid` stays `[RO]`),
  `arch/riscv/cpu/mod.rs` (`setup_boot` seeds `a0 = hart.id.0` on
  every hart, non-zero harts start in the OpenSBI wait loop),
  `arch/riscv/device/intc/plic.rs` (`NUM_CTX = 2 * num_harts`,
  `CTX_IP` becomes `Vec<u64>`, wire per-hart MEIP/SEIP to
  `harts[ctx / 2].irq`),
  `config/mod.rs` (CLI flag for `num_harts`),
  `xemu/xcore/src/main.rs` or the CLI crate (parse `--harts N`),
  `resource/Makefile` (rule to build `xemu-2hart.dtb`).

Gate matrix (PR2):

- All of Phase 1's gates at `num_harts = 1` (regression guard).
- New: 2-hart SMP boot — `X_ARCH=riscv64 make linux-2hart` (new
  target using `xemu-2hart.dtb`) → `buildroot login:` with
  `smp: Brought up 1 node, 2 CPUs` in dmesg.
- New: `plic_2hart_context_map` integration test proving per-context
  MEIP routing to `harts[ctx / 2].irq`.
- Difftest: unchanged — `num_harts = 1` only (NG-3).

---

## Trade-offs {ask reviewer for advice}

- **T-1 (scheduling model)** Round-robin one-instruction-per-step
  vs. N-instructions-per-step vs. work-stealing.
  (a) One-instruction round-robin — deterministic, trivial, but
      halves guest IPS at `num_harts = 2`.
  (b) N-instruction bursts per hart — better cache locality, risk of
      starvation for time-critical interrupts on the non-current hart.
  (c) Work-stealing (skip halted harts) — best throughput but harder
      to reproduce.
  **Proposal**: (a) for PR2; revisit in a follow-on perf task.
- **T-2 (Hart as struct vs. SoA)** One struct per hart vs. parallel
  arrays on `RVCore`.
  (a) `Vec<Hart>` (chosen) — one cache line per hart, clean
      encapsulation, easy to split borrow via `harts[i]`.
  (b) SoA (`gprs: Vec<[Word; 32]>, pcs: Vec<VirtAddr>, …`) —
      potentially better vectorisation, but fights the existing
      `RVCore` pattern and balloons field count.
  **Proposal**: (a). SoA has no measured win and breaks the 01-M-002
  elegance directive.
- **T-3 (PR count)** 2 PRs (refactor + activate) vs. 3 PRs (split
  activation into PLIC extension and DTS/OpenSBI boot).
  (a) 2 PRs — chosen. PR1 is 100% green-bar refactor; PR2 is the
      smallest landable unit that makes multi-hart real.
  (b) 3 PRs — PR2a PLIC NUM_CTX extension (still num_harts=1 visible,
      behaviour unchanged because second-hart context has no active
      IrqState), PR2b DTS + `--harts 2` + SMP boot. Lets PR2a ship
      without needing an SMP Linux build ready.
  **Proposal**: prefer (a). Ask reviewer whether (b) is preferred for
  bisection clarity if SMP boot turns out flaky.
- **T-4 (debug UX)** `DebugOps` signatures.
  (a) Keep scalar (chosen) — route everything through `self.current`;
      xdb asks "current hart" implicitly. Zero xdb churn.
  (b) Add `hart_id: u32` parameter to every read method — explicit,
      multi-hart-ready, but churns xdb and 6 existing xdb tests for no
      user-visible gain until a xdb hart-select task lands.
  **Proposal**: (a). Defer (b) to the xdb hart-select task (NG-6).
- **T-5 (SSIP fan-out shape)** `Bus::take_ssip(HartId)` vs. batch API.
  (a) Per-hart `take_ssip(HartId) -> bool` (chosen) — mirrors the
      spec (SSIP is per-hart) and the existing single-hart API shape.
  (b) `take_ssip_all() -> u64` returning a bitmap — saves N atomic
      loads per step.
  **Proposal**: (a). The atomic load is `Relaxed` and cheap; bitmap
  adds a size ceiling (64 harts) for no current win.

## Validation {test design}

[**Unit Tests**]

- **V-UT-1** `Hart::new` — id stored, GPR/FPR zeroed, PC=0, privilege
  = Machine, IRQ clone shares state.
- **V-UT-2** `Hart::reset` — clears all per-hart state including
  breakpoints; IRQ reset by owner (`RVCore::reset` drives it).
- **V-UT-3** `Mswi` with `num_harts = 4` — write MSIP[2] raises only
  `irq[2]`; writes to offsets beyond `4 * num_harts` are no-ops (V-E-2).
- **V-UT-4** `Mtimer` with `num_harts = 2` — `mtimecmp[0] = 0` fires
  MTIP on `irq[0]` only; `mtimecmp[1] = u64::MAX` keeps `irq[1].MTIP = 0`.
- **V-UT-5** `Sswi` with `num_harts = 3` — SETSSIP[1] raises
  `ssip[1]` only; `ssip[0]` and `ssip[2]` remain false.
- **V-UT-6** `Bus::new(_, _, 4)` — `ssip_pending.len() == 4`; each
  `Bus::ssip_flag(HartId(i))` returns a distinct `Arc<AtomicBool>`
  sharing storage with `ssip_pending[i]`.
- **V-UT-7** `MachineConfig::default().num_harts == 1`.
- **V-UT-8** Existing `sswi_edge_delivered_once_and_clearable` and
  `stip_delivered_in_s_mode_with_sie` tests pass unchanged after the
  refactor (I-4).

[**Integration Tests**]

- **V-IT-1** `arch_isolation` integration test passes unchanged
  (I-7). `SEAM_FILES`, `SEAM_ALLOWED_SYMBOLS`, `BUS_DEBUG_STRING_PINS`
  arrays and their expected counts are all invariant.
- **V-IT-2** (PR2) `plic_2hart_context_map` — PLIC with `NUM_CTX = 4`
  (2 harts × 2 modes); source enabled only on ctx 2 (hart 1 M-mode);
  `notify(0x02)` raises MEIP on `harts[1].irq`, not `harts[0].irq`.
- **V-IT-3** (PR1) `round_robin_fairness_single_hart` — a
  `num_harts = 1` core runs N steps; each step advances `current` back
  to `HartId(0)`; each step increments `hart[0].csr.cycle` by 1.
  Covers the single-hart degenerate case of the scheduler.
- **V-IT-4** (PR2) `round_robin_fairness_two_harts` — `num_harts = 2`
  core; each hart executes a tight NOP loop; after 1000 steps each
  hart has executed 500 instructions (± 1 for start-up parity).
- **V-IT-5** (PR2) `smp_linux_smoke` — `make linux-2hart` boots to
  `buildroot login:` and dmesg contains `smp: Brought up 1 node, 2
  CPUs`.

[**Failure / Robustness Validation**]

- **V-F-1** `num_harts = 0` / `num_harts > 16` — debug-assert at
  `MachineConfig` build site (C-1).
- **V-F-2** MMIO write to `MSIP[num_harts + 1]` returns `Ok(())`
  silently (no trap), does not mutate any `msip[h]`. Same for SSWI
  out-of-range.
- **V-F-3** MTIMER MMIO read at `mtimecmp` for hart index ≥ num_harts
  returns 0.
- **V-F-4** `RVCore::reset()` iterates every hart; after reset,
  `harts[i].pc == RESET_VECTOR` for all `i`.
- **V-F-5** (PR2) OpenSBI brings only hart 0 online: `dmesg` shows
  `Brought up 1 node, 1 CPU`; test explicitly expects 2, fails — this
  is the PR2 gate, not a PR1 regression.

[**Edge Case Validation**]

- **V-E-1** `num_harts = 1` is byte-identical to pre-refactor (I-4):
  every existing aclintSplit test passes unchanged; `make debian`
  boot-to-Python3 trace is identical (timing excluded).
- **V-E-2** Offset decode boundary — for `num_harts = 3`, MSWI
  accepts `offset ∈ {0, 4, 8}`, `offset = 12` reads 0 (region size
  stays 0x4000, so the read is in-region but unmapped for hart_id).
- **V-E-3** Round-robin wraparound — at `num_harts = 2`, after the
  second step `current` has wrapped back to `HartId(0)`; after the
  third step `current == HartId(1)` again.
- **V-E-4** PR2 hartid seeding — after `RVCore::with_config(num_harts
  = 2)` and `setup_boot(Firmware { fdt_addr })`, `harts[0].gpr[a0]
  == 0` and `harts[1].gpr[a0] == 1`; `mhartid` CSR reads match.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (HartId + Hart) | V-UT-1, V-UT-2 |
| G-2 (RVCore shape) | V-UT-1, V-IT-3, `arch_isolation` invariance |
| G-3 (ACLINT per-hart arrays) | V-UT-3, V-UT-4, V-UT-5 |
| G-4 (MachineConfig::num_harts) | V-UT-7, V-F-1 |
| G-5 (round-robin) | V-IT-3, V-IT-4, V-E-3 |
| G-6 (per-hart SSIP) | V-UT-5, V-UT-6 |
| G-7 (PR1 behaviour-preservation) | V-E-1, PR1 gate matrix (linux/debian/difftest) |
| G-8 (PR2 SMP boot) | V-IT-5, V-E-4, V-F-5 |
| C-1 (hart count bounds) | V-F-1 |
| C-2 (MMIO layout invariant) | V-UT-3..5, V-E-2 |
| C-3 (no new seam) | V-IT-1 |
| C-4 (deterministic order) | V-IT-3, V-IT-4, V-E-3 |
| C-5 (PR1 DTB untouched) | Phase 1 Gate matrix (`make linux` / `make debian`) |
| C-6 (no new deps) | PR1 Cargo.lock diff review |
| C-8 (DebugOps signatures) | V-IT-1 + xdb 6-test suite unchanged |
| I-4 (byte-identical single-hart) | V-E-1 |
| I-5 (per-hart IRQ routing) | V-UT-3, V-UT-4, V-IT-2 |
| I-6 (mhartid per hart) | V-E-4 |
| I-7 (arch_isolation invariant) | V-IT-1 |
