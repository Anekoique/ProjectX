# `hotPath` PLAN `01`

> Status: Draft
> Feature: `hotPath`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `00_MASTER.md`

---

## Summary

This round widens the previous `perfIcache`-scoped proposal into a single
coordinated `hotPath` iteration covering **every remaining PERF_DEV phase**:
P3 (Mtimer deadline gate), P4 (decoded-instruction cache), P5 (MMU fast-path
inlining + trap slimming), and P6 (memmove typed-read bypass). All four
phases attack disjoint slices of the same post-P1 hot loop measured in
`docs/perf/2026-04-15/REPORT.md` (xdb::main 40–47 %, MMU entry 12–15 %,
Mtimer 9–11 %, memmove shim 17–20 % combined) and land together in one
commit — they share the test harness, the benchmark gate, the mutex-free
invariant, and the `verify_no_mutex.sh` regression guard, so splitting them
would duplicate validation infrastructure four times (M-003).

The bucket math (REPORT.md §4) supports an aggregate **≥ 20 %** wall-clock
reduction on dhry/cm, **≥ 15 %** on microbench, and an **≥ 10 pp** drop of
`xdb::main` self-time share on the 2026-04-15 baseline. Every change is
structural: the icache is keyed on `(pc, ctx_tag, raw)` with per-hart
monotone tag bumps on translation-context events; the Mtimer gate is a
cached `next_fire_mtime` deadline; the MMU/trap work is `#[inline]`
pressure with no algorithmic change; memmove typed-read is a 1/2/4/8-byte
aligned-pointer bypass for RAM accesses. No benchmark-specific PC ranges,
no workload-name switches, no hot-loop specialisation — `make linux`,
`make linux-2hart`, `make debian`, dhrystone, coremark, microbench, and
any future RISC-V guest benefit equally (C-1).

## Log

[**Feature Introduce**]

- **Scope expansion (M-001).** Round 00 proposed only P4. Round 01 covers
  P3+P4+P5+P6 as one hotPath iteration, organised under one set of
  top-level sections with per-phase subsections where detail diverges.
- **Directory rename (M-002, already done).** `docs/fix/perfIcache/` →
  `docs/perf/hotPath/` and `docs/fix/perfBusFastPath/` →
  `docs/perf/busFastPath/`. All cross-references in this plan use the
  new paths.
- **Shared infrastructure.** One SMC torture test, one `perf-stats`
  Cargo feature, one `verify_no_mutex.sh` gate, one benchmark re-capture
  under `docs/perf/<today>/` cover all four phases.
- **P3 deadline gate.** Cache `next_fire_mtime = min(mtimecmp[*])` inside
  `Mtimer`; `tick()` fast-returns when `self.mtime < self.next_fire_mtime`;
  recompute on every `mtimecmp` write and on `reset`.
- **P4 decoded-instruction cache.** Per-hart direct-mapped 4096 entries,
  keyed on `(pc, ctx_tag: u64, raw)`, invalidated on satp / sfence.vma /
  fence.i / privilege-mode-change (debounced) / mstatus-bit-change
  (isolated to MPRV|SUM|MXR) / RAM-store (via `checked_write`).
- **P5 MMU/trap inlining.** Audit `#[inline(always)]` annotations on the
  TLB-hit fast path through `checked_read` → `access_bus` → `Bus::read`;
  reduce `commit_trap` zero-pending path to a single field-load branch.
- **P6 memmove bypass.** 1/2/4/8-byte aligned RAM accesses read/write the
  RAM slice as the corresponding primitive via `from_le_bytes` /
  `to_le_bytes` on a fixed-size array slice; MMIO and unaligned/odd-size
  accesses fall through to the existing generic path.

[**Review Adjustments**]

- **R-001** (HIGH): `Bus::store` → storing-hart icache invalidation uses
  **Option B** — `RVCore::checked_write` in `mm.rs:282-293` calls
  `self.invalidate_icache()` on the success branch when the resolved
  physical address falls in a RAM region. No callback on `Bus`, no new
  box/lock, no `Mutex` regression. Detail in §Implementation Steps 5.6.
- **R-002** (HIGH): SMC torture test is fully wired. File
  `xkernels/tests/am-tests/src/tests/smc.c`; letter `m` added to the
  `ALL` list and the `name` patsubst chain in
  `xkernels/tests/am-tests/Makefile` (lines 16 and 18-20). Pass marker
  is `printf("smc: OK\n");` followed by clean return so the AM runtime
  emits `GOOD TRAP` that the Makefile greps for (line 36). Invocation:
  `cd xkernels/tests/am-tests && make m`. A mirror Rust integration
  test in `inst/privileged.rs::tests` drives the full
  `step → checked_write → invalidate_icache` path so regressions
  survive any am-test reorg.
- **R-003** (MEDIUM): `mstatus` write hook bit-isolates — bump only if
  `(old ^ new) & (MPRV | SUM | MXR) != 0`. FS / VS / SD flips do **not**
  bump. Pinned by V-UT-4 (no-bump on FS flip) and V-UT-5 (bump on MPRV
  flip). Aligns with reviewer position TR-4 from round 00.
- **R-004** (MEDIUM): Trap-entry hook debounces on privilege change —
  bump only when `old_priv != self.privilege` after each assignment in
  `trap/handler.rs:77-164`. M→M ecall traps no longer bump. Pinned by
  V-UT-6.
- **R-005** (MEDIUM): Adopt option (a) — keep `(pc >> 1) & MASK` and
  document the aliasing structure in I-8 (see below). RVC vs. RVI
  analysis: compressed instructions advance PC by 2, so `pc >> 1`
  covers adjacent-address slots; full instructions at `pc` and
  `pc + 2*LINES = 0x2000` collide but the full `pc` tag detects the
  alias and causes a clean miss-replace. Deferred XOR-fold to a
  round-02 only if Linux hit rate < 95 %.
- **R-006** (MEDIUM): V-UT-7 + V-F-4 added — two VA→PA mappings of the
  same virtual code address with distinct instruction bytes; execute in
  M-mode with `MPRV=0`; flip `MPRV=1` with `MPP=U` and the alternate
  mapping live; assert the alternate instruction executes. Pins the
  R-003 bit-isolated `mstatus` hook against its primary failure mode.
- **R-007** (LOW): `ctx_tag` widened from `u32` to `u64`. Wrap requires
  ~1.8 × 10^19 events and is unreachable; no cost on the RV64 host.
- **R-008** (LOW): `icache_stats` merged into a single `perf-stats`
  Cargo feature in `xemu/xcore/Cargo.toml`, off by default, never
  compiled into shipping release builds. Exit-gate invocation
  documented in `docs/PERF_DEV.md` for reuse.
- **R-009** (LOW): Response Matrix carried forward; all R-001..R-008
  plus M-001..M-003 have explicit rows below.

[**Master Compliance**]

- **M-001 (scope expansion):** Plan now covers P3+P4+P5+P6. See §Spec
  for the PERF_DEV.md §3 cross-map and per-phase exit-gate thresholds.
  P4 (largest lever) occupies the deepest detail; P3/P5/P6 carry
  proportionally lighter but concrete designs.
- **M-002 (path rename):** Every path reference uses `docs/perf/hotPath/`
  and `docs/perf/busFastPath/`. No `docs/fix/` references remain.
- **M-003 (clean layout):** One Summary, one Log, one Response Matrix,
  one Spec with per-phase subsections where design diverges
  (Architecture §P3..§P6, Invariants shared-then-phase-specific,
  Implementation Steps ordered 1..9, Validation one matrix), one
  Trade-offs, one Exit Gate. No duplicated boilerplate across phases.

### Changes from Previous Round

[**Added**]

- P3 Mtimer deadline-gate design (new; was out-of-scope in round 00).
- P5 MMU/trap inlining design (new).
- P6 memmove typed-read bypass design (new).
- V-UT-4..V-UT-9 and V-IT-5..V-IT-6 for the new phases.
- Bit-isolated `mstatus` hook (R-003, TR-4).
- Privilege-debounced trap-entry hook (R-004).
- `checked_write` SMC invalidation plumbing (R-001 Option B).
- Am-test harness wiring spec (R-002) with exact Makefile edits.
- `perf-stats` Cargo feature replacing ad-hoc `icache_stats` gate (R-008).

[**Changed**]

- Icache `ctx_tag` type: `u32` → `u64` (R-007).
- Feature name: `perfIcache` → `hotPath` (M-002).
- Directory: `docs/fix/perfIcache/` → `docs/perf/hotPath/` (M-002).
- Trap-entry bump: unconditional → privilege-change-only (R-004).
- `mstatus` bump: any-write → bit-isolated MPRV|SUM|MXR (R-003).
- Benchmark thresholds widened for combined phases: dhry ≥ 20 %,
  cm ≥ 20 %, mb ≥ 15 %, xdb::main ≥ 10 pp drop.

[**Removed**]

- Round-00 "deferred to round 01" caveats on `mstatus` bit-isolation
  and `ctx_tag` wrap (both resolved now).
- "Follow the LR/SC pattern" hand-wave for the SMC hook (replaced
  with concrete Option B in `checked_write`).
- `make run AM=smc` invocation (replaced with
  `cd xkernels/tests/am-tests && make m`).

[**Unresolved**]

- Linux-boot icache hit rate: the V-IT-4 threshold (≥ 95 %) is pinned
  only for dhry/cm/mb. Linux boot is opportunistically measured under
  `--features perf-stats` and flagged for a hypothetical round 02 if
  the observed rate drops below 90 %. Not a blocker for this round.
- If the P5 MMU inline audit shows LTO already inlines the TLB-hit
  path completely, that phase's wall-clock contribution may be below
  its 5 % floor. The Exit Gate allows the **combined** floor
  (≥ 20 % dhry) to compensate; if combined misses, phases re-split
  for round 02.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Master | M-001 | Applied | Scope expanded from P4-only to P3+P4+P5+P6; one coordinated iteration, single commit target. |
| Master | M-002 | Applied | All references use `docs/perf/hotPath/` and `docs/perf/busFastPath/`; feature renamed `perfIcache` → `hotPath`. |
| Master | M-003 | Applied | One set of top-level sections; per-phase detail only where design diverges (Architecture, Implementation Steps, Exit Gate sub-rows). |
| Review | R-001 | Accepted (HIGH) | Option B: `RVCore::checked_write` calls `self.invalidate_icache()` on RAM-hit success; `Bus` API unchanged; no new `Mutex`/callback. See §Implementation Steps 5.6. |
| Review | R-002 | Accepted (HIGH) | Concrete Makefile edits specified; `smc` gets letter `m`, `ALL = u r t s p c e f m`, `name` patsubst extended; uses `GOOD TRAP` via `printf("smc: OK\n")` + clean return; Rust integration test mirrors the sequence. |
| Review | R-003 | Accepted (MEDIUM) | Bit-isolated: bump iff `(old ^ new) & (MPRV \| SUM \| MXR) != 0`. V-UT-4 (no-bump) and V-UT-5 (bump) pin it. |
| Review | R-004 | Accepted (MEDIUM) | Trap-entry hook debounces on `old_priv != self.privilege`; V-UT-6 pins it. |
| Review | R-005 | Accepted (MEDIUM) | Keep `(pc >> 1) & MASK`; document aliasing under I-8; XOR-fold deferred to round 02 contingent on Linux hit rate. |
| Review | R-006 | Accepted (MEDIUM) | V-UT-7 + V-F-4 added: MPRV flip with alternate page mapping triggers exactly one icache miss at the next fetch. |
| Review | R-007 | Accepted (LOW) | `ctx_tag: u64`. |
| Review | R-008 | Accepted (LOW) | `perf-stats` Cargo feature, off by default, documented in `docs/PERF_DEV.md`. |
| Review | R-009 | Acknowledged (LOW) | Template compliance confirmed; Matrix populated per R-009's expectation. |
| Review | TR-1 | Accepted | Direct-mapped 4 K retained. |
| Review | TR-2 | Accepted | 4 K retained; 16 K deferred to round 02 contingent on Linux hit rate. |
| Review | TR-3 | Accepted | Global flush on RAM store retained (Phase-1 conservative). |
| Review | TR-4 | Accepted | Bit-isolated `mstatus` hook adopted now (reviewer position). |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here.
> - Every Master directive must appear here.
> - Rejections must include explicit reasoning.

---

## Spec

[**Goals**]

- G-1 (P4): Eliminate the pest tree walk on every cache-hit fetch; the
  fast path becomes one tag compare + one POD copy of `DecodedInst`.
- G-2 (P3): Cached-deadline short-circuit in `Mtimer::tick` so the
  default path is one `u64` compare + return.
- G-3 (P5): LTO-inline the TLB-hit MMU fast path end-to-end; reduce
  `commit_trap` zero-pending path to a single field-load branch.
- G-4 (P6): Bypass `_platform_memmove` for aligned 1/2/4/8-byte RAM
  accesses using typed primitive reads/writes.
- G-5 (combined): Wall-clock reduction on the 2026-04-15 baseline —
  dhry ≥ 20 %, cm ≥ 20 %, mb ≥ 15 %.
- G-6 (combined): `xdb::main` self-time share drops by ≥ 10 pp on all
  three workloads.
- G-7 (combined): Combined icache-addressable + Mtimer + memmove + MMU
  buckets drop by ≥ 20 pp of aggregate self-time share.
- G-8: Preserve correctness under SMC, page-table changes, fence.i,
  privilege transitions, MPRV flips — proven by the torture test +
  V-F/V-UT suite that lands before or with each optimisation.
- G-9: Multi-hart correctness: fence.i invalidates only the issuing
  hart's icache; `Bus::store` from hart N invalidates only hart N's
  icache.

- NG-1: No JIT, no trace chaining, no threaded dispatch.
- NG-2: No paddr-tagged SMC bitmap.
- NG-3: No replacement for `pest`. It remains the decode-miss fallback.
- NG-4: No benchmark-specific code paths of any kind.
- NG-5: No assembly-file modifications.
- NG-6: No new `Arc<Mutex<_>>`, `RwLock<_>`, `RefCell<_>`, or
  `Box<dyn FnMut>` on `Bus` or `RVCore`. `verify_no_mutex.sh` must
  stay `ok`.
- NG-7: No multi-thread SMP work (Phase 11 RFC territory).
- NG-8: No removal or relaxation of existing traps, exceptions, or
  instruction semantics.

[**Architecture**]

Top-level shape:

```
 RVCore::step  (xcore/src/arch/riscv/cpu.rs:238-246)
   │
   │  fetch(bus) -> raw:u32            [P5: ensure fully inlined]
   ▼
 decode_cached(raw) -> DecodedInst     [P4: new]
   │   hit : one tag compare, POD copy
   │   miss: pest decode, line overwrite
   ▼
 dispatch(bus, decoded)                [unchanged]
   │
   ▼
 Bus::tick()                           [P3: Mtimer deadline-gate]
   └─ Mtimer::tick : if mtime < next_fire_mtime { return; }
   └─ other devices unchanged
```

### Architecture §P3 — Mtimer deadline gate

`Mtimer` (`xcore/src/arch/riscv/device/aclint/mtimer.rs`) grows one
field `next_fire_mtime: u64`, initialised to `u64::MAX`. `tick()`
becomes:

```
fn tick(&mut self) {
    // existing epoch-init + SYNC_INTERVAL sync logic unchanged
    if self.mtime < self.next_fire_mtime { return; }
    self.check_all();
}
```

`next_fire_mtime` is recomputed as
`self.mtimecmp.iter().copied().min().unwrap_or(u64::MAX)`:

- on every `write` that hits a `Mtimecmp` register
  (`mtimer.rs:95-109`);
- at the end of `check_timer` (so a just-fired deadline is re-armed);
- on `reset()` (back to `u64::MAX`, `mtimer.rs:129-139`).

Precedent: identical shape to QEMU's `timer_next_deadline` pattern
in `hw/intc/aclint.c`.

### Architecture §P4 — Decoded-instruction cache

Per-hart direct-mapped 4096-line cache. One new file
`xcore/src/arch/riscv/cpu/icache.rs` plus two new fields on `RVCore`:

```
                          ┌──────────────────────────────────────┐
                          │ RVCore.step                          │
                          └────────────────────┬─────────────────┘
                fetch(bus) -> raw:u32          │
                          ┌────────────────────▼─────────────────┐
                          │ idx = (pc >> 1) & MASK  (R-005 I-8)  │
                          │ line = &mut self.icache.lines[idx]   │
                          │ hit = line.pc == pc                  │
                          │     && line.ctx_tag == self.ctx_tag  │
                          │     && line.raw == raw               │
                          └─────────────┬────────────────────────┘
                                   hit  │  miss
                            decoded=line.decoded
                                        │   decoded = pest.decode(raw)?
                                        │   *line = { pc, ctx_tag, raw, decoded }
                                        ▼
                            dispatch(bus, decoded)

  Invalidation (per-hart, bump self.icache_ctx_tag by 1):
    satp write            csr/ops.rs:30-43
    sfence.vma            inst/privileged.rs:97-125
    fence.i               inst/privileged.rs:71-79 (was NOP)
    privilege change      trap/handler.rs:77-164 (debounced, R-004)
    mstatus write         csr/ops.rs mstatus/sstatus arm (bit-isolated, R-003)
    RAM store             cpu/mm.rs:282-293 (R-001 Option B)
```

Shape mirrors QEMU per-CPU jump cache (`include/exec/tb-jmp-cache.h`,
`TB_JMP_CACHE_BITS = 12`) without TB chaining. NEMU IBuf is the
second precedent (4-way × 4096, same invalidation event set).

### Architecture §P5 — MMU + trap inline pressure

Target files: `xcore/src/arch/riscv/cpu/mm.rs` (`access_bus`,
`checked_read`, `checked_write`, `translate`, `fetch`, `load`,
`store`) and `xcore/src/arch/riscv/cpu/trap.rs` /
`trap/handler.rs` (`commit_trap`). No algorithmic change; audit
under `cargo asm` that the TLB-hit path in `checked_read` is
inlined through to `Bus::read`. Add `#[inline]` /
`#[inline(always)]` where LTO alone does not fold. For
`commit_trap`, extract the zero-pending predicate into an
`#[inline(always)] fn has_pending_trap(&self) -> bool` so the
fast path is one field load and one branch.

### Architecture §P6 — memmove typed-read bypass

`Ram::read` / `Ram::write` (`xcore/src/device/ram.rs`) gain a
size + alignment pre-check. For aligned 1/2/4/8-byte accesses:

```
match size {
    1 => Ok(bytes[offset] as Word),
    2 => Ok(u16::from_le_bytes(bytes[offset..offset+2].try_into()?) as Word),
    4 => Ok(u32::from_le_bytes(bytes[offset..offset+4].try_into()?) as Word),
    8 => Ok(u64::from_le_bytes(bytes[offset..offset+8].try_into()?) as Word),
    _ => /* existing generic memmove path */,
}
```

`from_le_bytes` + `try_into` on a fixed-size slice compiles to a
single aligned load on the host; no `unsafe` needed, no clippy
warning. If `cargo asm` shows this does not fold under rustc 1.96,
fall back to `ptr::read_unaligned` with a mandatory `// SAFETY:`
comment covering alignment / in-bounds / no-aliasing; the safe
path is preferred.

[**Invariants**]

Shared (all phases):

- I-1: `RVCore::step` always calls `fetch` before any icache lookup.
- I-2: An icache line with `ctx_tag != self.icache_ctx_tag` is
  treated as miss regardless of `pc`/`raw`.
- I-3: The icache is owned per-hart; no shared state, no atomics,
  no locks (inherited M-001 sentinel from P1).
- I-4: `pest` is the sole decode authority; the cache only memoises
  its result, never its inputs.
- I-5: On miss, an icache line is overwritten in full (`pc`,
  `ctx_tag`, `raw`, `decoded`) — partial updates are forbidden.
- I-6: Decode failure (`XError::InvalidInst`) does not write a
  line; a transient illegal word does not poison subsequent legal
  re-fetches.
- I-7: Compressed instructions: `raw` carries the fetched word
  as-is (16-bit zero-extended to `u32`), matching `fetch` at
  `mm.rs:306-315`.
- I-8: Index `(pc >> 1) & MASK`. Aliasing analysis (R-005): RVC at
  `pc` and RVC/RVI at `pc+2` land in adjacent slots (load factor
  well below 1 on dhry/cm/mb). RVI at `pc` and RVI at
  `pc + 2*LINES = 0x2000` collide but the full-`pc` tag detects
  the alias and triggers miss-replace. Load factor on dhry/cm/mb
  stays below 40 %; V-IT-4 hit rate ≥ 95 % is the observable gate.
- I-9: `Bus::store` to a RAM region invalidates only the storing
  hart's icache (via R-001 Option B in `checked_write`). Stores to
  MMIO do not bump.
- I-10 (inherited from P1): `CPU::step` destructures `self` into
  disjoint `bus` + `cores[i]` borrows; no `Mutex<Bus>`.
  `verify_no_mutex.sh` regex-scan passes.
- I-11: `icache_ctx_tag: u64` monotone non-decreasing for the
  hart's lifetime. Never decremented. Never wraps within a reboot
  (headroom ~1.8 × 10^19 events).
- I-12: A line with `ctx_tag != self.icache_ctx_tag` is a miss;
  write-through on decode fills all four fields and sets
  `ctx_tag = self.icache_ctx_tag`.
- I-13: `fence.i` bumps only the issuing hart's `icache_ctx_tag`
  (Zifencei §5.1: fence.i is local-hart only).
- I-14 (R-003, **new**): `mstatus`/`sstatus` write bumps
  `icache_ctx_tag` iff `(old ^ new) & (MPRV | SUM | MXR) != 0`.
  FS, VS, SD, MIE, SIE, MPIE, SPIE flips do not bump.
- I-15 (R-004, **new**): Privilege-change hook in `trap/handler.rs`
  bumps `icache_ctx_tag` iff `old_priv != self.privilege` after the
  assignment. M→M ecall traps do not bump.
- I-16 (P3, **new**): `next_fire_mtime` is the running minimum
  over `self.mtimecmp[*]`. Recomputed on every `mtimecmp` write,
  at the end of `check_timer`, and on `reset`. Initialised
  `u64::MAX`.
- I-17 (P6, **new**): The typed-read bypass path is taken iff the
  region is RAM AND `size ∈ {1, 2, 4, 8}` AND `addr % size == 0`.
  All other cases fall through to the existing memmove path,
  preserving every MMIO semantic (side effects, unaligned
  behaviour, arbitrary sizes).

[**Data Structure**]

```rust
// xcore/src/arch/riscv/cpu/icache.rs (new)
use crate::isa::riscv::decoder::DecodedInst;
use memory_addr::VirtAddr;

pub const ICACHE_BITS: usize = 12;
pub const ICACHE_LINES: usize = 1 << ICACHE_BITS;
pub const ICACHE_MASK: usize = ICACHE_LINES - 1;

#[derive(Clone, Copy)]
pub struct ICacheLine {
    pub pc: VirtAddr,
    pub ctx_tag: u64,       // R-007: widened from u32
    pub raw: u32,
    pub decoded: DecodedInst,
}

impl ICacheLine {
    // Sentinel ctx_tag=0 is never produced by live code because
    // RVCore initialises icache_ctx_tag = 1.
    pub const INVALID: Self = Self {
        pc: VirtAddr::from_usize(0),
        ctx_tag: 0,
        raw: 0,
        decoded: DecodedInst::C { kind: InstKind::illegal, inst: 0 },
    };
}

pub struct ICache { pub lines: [ICacheLine; ICACHE_LINES] }

impl ICache {
    pub fn new() -> Box<Self> {
        Box::new(Self { lines: [ICacheLine::INVALID; ICACHE_LINES] })
    }
    #[inline]
    pub fn index(pc: VirtAddr) -> usize {
        (pc.as_usize() >> 1) & ICACHE_MASK
    }
}

// RVCore (xcore/src/arch/riscv/cpu.rs) gains two fields:
pub struct RVCore {
    // ... existing fields unchanged ...
    pub(in crate::arch::riscv) icache: Box<ICache>,
    pub(in crate::arch::riscv) icache_ctx_tag: u64,
}

// Mtimer (xcore/src/arch/riscv/device/aclint/mtimer.rs) gains one field:
pub(super) struct Mtimer {
    // ... existing fields unchanged ...
    next_fire_mtime: u64,   // P3: min(mtimecmp[*]); starts u64::MAX
}
```

`DecodedInst` (in `isa/riscv/decoder.rs`) gains `Copy` to its
existing `Clone, PartialEq, Eq` derive; V-UT-1 pins that with
`fn _assert_copy<T: Copy>() {}`.

[**API Surface**]

```rust
// In ICache
impl ICache {
    pub fn new() -> Box<Self>;
    #[inline] pub fn index(pc: VirtAddr) -> usize;
}

// In RVCore
impl RVCore {
    /// Bump per-hart icache context tag. One u64 wrapping_add.
    /// Called by every invalidation hook (satp / sfence.vma /
    /// fence.i / privilege-change-debounced / mstatus-bit-isolated /
    /// RAM-store).
    #[inline]
    pub(in crate::arch::riscv) fn invalidate_icache(&mut self) {
        self.icache_ctx_tag = self.icache_ctx_tag.wrapping_add(1);
    }

    /// Cache-aware decode replacing decoder.decode(raw) in step().
    #[inline]
    fn decode_cached(&mut self, raw: u32) -> XResult<DecodedInst>;
}

// In Mtimer (private)
impl Mtimer {
    #[inline]
    fn recompute_next_fire(&mut self) {
        self.next_fire_mtime =
            self.mtimecmp.iter().copied().min().unwrap_or(u64::MAX);
    }
}

// In Ram (xcore/src/device/ram.rs) — typed fast path added inside
// existing Device::read / Device::write impls; no public API change.
```

No new public API on the RVCore/Bus/Mtimer facades. All additions
are `pub(in crate::arch::riscv)` or module-private.

[**Constraints**]

- C-1: No benchmark-targeted code. Reviewer must reject the plan if
  any constant, branch, or special case keys off workload identity.
- C-2: Honest gain projection — aggregate 20–42 % wall-clock on
  dhry, 20–42 % on cm, 15–35 % on mb (per PERF_DEV §6 floor/ceiling).
- C-3: `make fmt && make clippy && make run && make test` must pass
  after every implementation step.
- C-4: Benchmarks run with `DEBUG=n`.
- C-5: Workloads launched via `make run` / `make linux` / `make debian`.
- C-6: No assembly-file modifications.
- C-7: `bash scripts/ci/verify_no_mutex.sh` must remain `ok` — no
  `Mutex`/`RwLock` regression.
- C-8: Scope ends at P3+P4+P5+P6. No P1/P2 rework, no Phase-11
  multi-thread work, no VGA/framebuffer, no allocator tuning.
- C-9: Phase-1 SMC strategy is global per-hart flush on RAM store.
  Paddr-tagged refinement is a future phase; a comment in
  `checked_write` cites this plan as the deferral anchor.
- C-10: `perf-stats` feature is off by default. Release binaries
  ship without it. Exit-gate measurements cite the invocation
  `cargo build --release --features perf-stats` explicitly.
- C-11: No new `unsafe` in the P6 path if `from_le_bytes` +
  `try_into` lowers to a single aligned load (verified under
  `cargo asm`). If a `ptr::read_unaligned` fallback is needed, a
  `// SAFETY:` comment covering alignment / in-bounds / no-aliasing
  is mandatory.

---

## Implement

### Execution Flow

[**Main Flow**]

1. `RVCore::step` calls `self.fetch(bus)?` → `raw: u32` (unchanged).
2. `idx = ICache::index(self.pc)` (new).
3. Read `let line = &mut self.icache.lines[idx]`.
4. Check `hit = line.pc == self.pc &&
   line.ctx_tag == self.icache_ctx_tag && line.raw == raw`.
5. Hit: `decoded = line.decoded` (POD copy).
6. Miss: `decoded = self.decoder.decode(raw)?`;
   `*line = ICacheLine { pc, ctx_tag: self.icache_ctx_tag, raw,
   decoded }`.
7. `self.dispatch(bus, decoded)?` (unchanged).
8. `Bus::tick()` → `Mtimer::tick()` executes the P3 deadline
   short-circuit before any per-hart work.
9. Guest load/store through `RVCore::checked_read` / `checked_write`
   uses the P5-inlined TLB-hit path; RAM accesses in
   `Bus::read/write` follow the P6 typed-read fast path when size
   and alignment permit.
10. On any RAM-hit write, `checked_write` calls
    `self.invalidate_icache()` (R-001 Option B).

[**Failure Flow**]

1. `fetch` traps → return `Err(...)` before any icache access.
   Cache unchanged.
2. `decoder.decode(raw)?` errors on miss → return before writing
   the line. Cache unchanged. (I-6.)
3. `dispatch` errors after a cache-hit decode → line is kept
   (decode succeeded); next fetch at same PC re-decodes to same
   `DecodedInst` and traps identically.
4. SMC race: byte hits memory → `checked_write` success branch
   bumps ctx_tag → next fetch at any PC misses (old ctx_tag) →
   re-fetches from updated memory.
5. `checked_write` traps (PMP / alignment / page-fault):
   `bus.store` returns `Err` → `checked_write` `?`-propagates
   before calling `invalidate_icache()` → ctx_tag unchanged.
   **Critical.**
6. `mstatus` write with only FS/VS/SD flipping → XOR mask is
   zero → no bump → icache stays warm on FP-heavy workloads
   (R-003).
7. Trap from M-mode to M-mode (no privilege change) → debounced
   hook sees `old_priv == new_priv` → no bump (R-004).
8. P3: `mtime < next_fire_mtime` → `tick` returns before
   `check_all`; no irq change (correct because no hart's
   `mtimecmp` was reached).
9. P6: MMIO / unaligned / odd-size access → pre-check fails →
   falls through to existing memmove path with identical
   semantics.
10. Tag wrap (I-11): practically unreachable at `u64`. If it ever
    did happen, next-fetch comparison would still work because the
    wrapped tag has no live lines with that value (lines were
    overwritten as ctx_tag advanced).

[**State Transition**]

- `icache line (valid, ctx_tag match) → hit` when `pc` and `raw`
  match.
- `icache line (valid, ctx_tag mismatch) → miss` → `line replaced`.
- `icache line (decode fails) → unchanged`, error propagates.
- `Bus::store(RAM) success → ctx_tag bumped` (R-001).
- `satp / sfence.vma / fence.i → ctx_tag bumped`.
- `privilege change (old ≠ new) → ctx_tag bumped`; else unchanged
  (R-004).
- `mstatus write (MPRV|SUM|MXR XOR ≠ 0) → ctx_tag bumped`; else
  unchanged (R-003).
- `mtimecmp write → next_fire_mtime recomputed` (P3).
- `mtime ≥ next_fire_mtime → check_all runs`; else early return
  (P3).
- `RAM read/write size ∈ {1,2,4,8} ∧ aligned → typed path`; else
  memmove (P6).

### Implementation Plan

Ordered; each numbered step must pass `make fmt && make clippy &&
cargo test --workspace && cd xkernels/tests/am-tests && make m &&
bash scripts/ci/verify_no_mutex.sh` before the next step begins.

[**Phase 1 — SMC torture test (lands FIRST, R-002)**]

1. Create `xkernels/tests/am-tests/src/tests/smc.c` with the
   write-execute-rewrite-fence.i-re-execute sequence. Pass marker:
   `printf("smc: OK\n");` then return cleanly so AM emits
   `GOOD TRAP` (matching the Makefile's `grep "GOOD TRAP"` at line
   36). Mirror the structure of `trap-ecall.c`: include `test.h`,
   provide `void test_smc(void)`, end with the `printf(... OK)`
   line.
2. Edit `xkernels/tests/am-tests/Makefile`:
   - Line 16: `ALL = u r t s p c e f m` (append `m`).
   - Lines 18-20: extend the `name` patsubst chain with
     `$(patsubst m,smc,...)`.
3. Invocation gate: `cd xkernels/tests/am-tests && make m`.
4. Rust integration test
   `smc_fence_i_invalidates_icache` in
   `inst/privileged.rs::tests` (uses the existing `setup_core`
   helper at `privileged.rs:139-145`) drives the same sequence
   through `RVCore::step` and `RVCore::store` → survives any
   am-test reorg.
5. **Gate**: test passes *without* the icache (because `fence.i`
   is currently a NOP and there is nothing stale). Must keep
   passing after the icache lands.

[**Phase 2 — ICache struct + DecodedInst: Copy + V-UT-1**]

1. New file `xcore/src/arch/riscv/cpu/icache.rs` implementing
   `ICache`, `ICacheLine`, `ICACHE_BITS = 12` (Data Structure
   above).
2. Derive `Copy` on `DecodedInst` in `isa/riscv/decoder.rs:161`.
3. V-UT-1: `fn _assert_copy<T: Copy>() {}` against `DecodedInst`.

[**Phase 3 — RVCore fields + invalidate_icache + decode_cached**]

1. Add `icache: Box<ICache>` and `icache_ctx_tag: u64` to `RVCore`
   (`cpu.rs:36-54`).
2. Initialise in `RVCore::new` to `ICache::new()` and `1`.
3. Add `invalidate_icache` and `decode_cached` (API Surface).

[**Phase 4 — Wire step to the cache**]

1. Replace `self.decoder.decode(raw)?` in `cpu.rs:238-246` with
   `self.decode_cached(raw)?`.

[**Phase 5 — Install invalidation hooks**]

1. **5.1 satp write** — bump inside the `0x180` arm in
   `csr/ops.rs:30-43`, after `self.mmu.update_satp(satp)`.
2. **5.2 sfence.vma** — bump in `inst/privileged.rs:97-125` after
   `self.mmu.tlb.flush(vpn, asid)`.
3. **5.3 fence.i** — replace the NOP body in
   `inst/privileged.rs:71-79` with `self.invalidate_icache()`
   (I-13 / Zifencei §5.1).
4. **5.4 Privilege-debounced trap-entry (R-004)** — in
   `trap/handler.rs:77-164`, wrap each `self.privilege = X`
   assignment in
   `let old = self.privilege; self.privilege = X; if old != X
   { self.invalidate_icache(); }`.
5. **5.5 mstatus/sstatus bit-isolated (R-003)** — in
   `csr/ops.rs` mstatus/sstatus arm, read `old` and `new`; bump
   iff `(old ^ new) & (MPRV | SUM | MXR) != 0`.
6. **5.6 RAM-store SMC (R-001 Option B)** — in `cpu/mm.rs:282-293`
   `checked_write`, after `bus.store(...)` returns `Ok(())`,
   check `bus.is_ram(pa)` (add helper if absent — pure lookup
   against `Bus::regions`, no new state) and if true call
   `self.invalidate_icache()`. If `bus.store` traps, the early
   `?` exits before `invalidate_icache` — ctx_tag unchanged
   (Failure Flow #5).

[**Phase 6 — P3 Mtimer deadline gate**]

1. Add `next_fire_mtime: u64` to `Mtimer` (`mtimer.rs:26-33`),
   init `u64::MAX`.
2. Add `recompute_next_fire` helper (API Surface).
3. Modify `tick` (`mtimer.rs:112-123`): after the `SYNC_INTERVAL`
   sync, short-circuit on `self.mtime < self.next_fire_mtime`.
4. Call `recompute_next_fire` in `write` after every `Mtimecmp`
   mutation (`mtimer.rs:95-108`), at the end of `check_timer`,
   and at the end of `reset` (`mtimer.rs:129-139`).

[**Phase 7 — P5 MMU / trap inline pressure**]

1. Audit `checked_read` / `access_bus` / `Bus::read` / `Ram::read`
   call chain with `cargo asm --release --features perf-stats` to
   confirm TLB-hit inlining. Add `#[inline]` / `#[inline(always)]`
   where LTO does not fold. No algorithmic change.
2. `commit_trap` — extract the zero-pending predicate into
   `#[inline(always)] fn has_pending_trap(&self) -> bool`.
3. Capture the `cargo asm` transcript for V-IT-5 evidence.

[**Phase 8 — P6 memmove typed-read bypass**]

1. In `Ram::read` (`xcore/src/device/ram.rs`), add size-match on
   1/2/4/8 with alignment check; use
   `u{8,16,32,64}::from_le_bytes(slice[..].try_into()?)`.
2. In `Ram::write`, mirror with `to_le_bytes`.
3. MMIO and unaligned/odd-size paths untouched.
4. If `cargo asm` shows `from_le_bytes` does not fold to a native
   load under rustc 1.96, fall back to `ptr::read_unaligned` with
   explicit `// SAFETY:` comment; document choice in the commit
   message.

[**Phase 9 — Benchmark capture**]

1. `bash scripts/perf/bench.sh --out docs/perf/<today>` (3 iters
   × 3 workloads) with `DEBUG=n`.
2. `bash scripts/perf/sample.sh --out docs/perf/<today>`.
3. `python3 scripts/perf/render.py --dir docs/perf/<today>`.
4. Capture hit rate via
   `cargo build --release --features perf-stats && make run`
   (stats dumped at exit).
5. Diff `data/bench.csv` vs.
   `docs/perf/2026-04-15/data/bench.csv`; compare self-time
   bucket tables (§Exit Gate).

## Trade-offs

- **T-1: Combined iteration vs. four separate phases.**
  - *Combined (chosen, M-001, M-003)*: One test harness, one
    commit, one benchmark capture, one `verify_no_mutex` gate.
    Simpler PR.
  - *Separate*: Finer-grained git history, per-phase rollback.
    Costs 4× benchmark capture and 4× review cycles.
  - *Recommendation*: Combined per M-001/M-003. If any single
    phase regresses in the final benchmark, its step is excluded
    at commit time (phases 6, 7, 8 are independently revertable)
    and re-planned in round 02.
- **T-2: Direct-mapped 4 K vs. set-associative 4-way × 4096.**
  - Reviewer TR-1: direct-mapped. Adopted.
  - Reviewer TR-2: 4 K. Adopted. 16 K deferred contingent on
    Linux hit rate.
- **T-3: SMC strategy — global flush vs. paddr-tagged.**
  - Reviewer TR-3: global flush. Adopted.
- **T-4: mstatus bump — any-write vs. bit-isolated.**
  - Reviewer TR-4: bit-isolated now. Adopted (R-003).
- **T-5: Trap-entry bump — unconditional vs. privilege-debounced.**
  - Chosen debounced (R-004). One extra comparison per trap;
    saves bump on M→M ecall storms in OpenSBI boot.
- **T-6: P6 typed-read — safe `from_le_bytes` vs. unsafe
  `ptr::read_unaligned`.**
  - Safe-first (chosen). `try_into` on a fixed-size slice is
    typically zero-cost on modern rustc; if the assembly audit
    shows otherwise, fall back to `unsafe` with `// SAFETY:` and
    retain the safe path behind
    `#[cfg(not(feature = "typed-read-unsafe"))]`. Reviewer:
    please confirm the preference if the audit forces the
    fallback.
- **T-7: P3 `next_fire_mtime` recompute — on-write vs. on-fire.**
  - Chosen both: recompute on every `mtimecmp` write
    (authoritative) and at the end of every `check_timer` that
    fires (keeps field fresh after IRQ delivery). The
    alternative — recompute lazily inside `tick` before the
    compare — defeats the whole point of the gate.
- **T-8: Ship P5 with or without asm audit evidence.**
  - Chosen with. `cargo asm` output for `checked_read` and
    `commit_trap` is captured into the commit message so a
    future round can verify the inline shape was actually
    achieved.
- **T-9: Threaded dispatch (out of scope).**
  - Ertl & Gregg 2003 document 20–50 % speedup from
    direct-threaded over switch-dispatch; this is a future phase
    per PERF_DEV §7, not hotPath. Cited for context only.

Sources (required by task spec §Notes):

- QEMU TB cache maintenance:
  https://github.com/qemu/qemu/blob/master/accel/tcg/tb-maint.c
- QEMU per-CPU jump cache:
  https://github.com/qemu/qemu/blob/master/include/exec/tb-jmp-cache.h
- QEMU QHT aliasing analysis: https://lwn.net/Articles/697265/
- NEMU IBuf: https://github.com/OpenXiangShan/NEMU
- rv8 CARRV 2017:
  https://carrv.github.io/2017/papers/clark-rv8-carrv2017.pdf
- rvemu (baseline, no cache): https://github.com/d0iasm/rvemu
- RISC-V Zifencei §5.1:
  https://github.com/riscv/riscv-isa-manual/releases
- RISC-V Privileged §10.6 (sfence.vma): same release index
- Ertl & Gregg 2003, direct-threaded interpreters:
  https://www.complang.tuwien.ac.at/forth/threaded-code.html

## Validation

[**Unit Tests**]

- V-UT-1: `decoded_inst_is_copy` in `isa/riscv/decoder.rs::tests`
  — static `_assert_copy<DecodedInst>()`. Pins I-7 and the
  POD-only-variant property.
- V-UT-2: `ctx_tag_bumps_on_standard_events` in
  `cpu/icache.rs::tests` — drive satp write, `sfence.vma`,
  `fence.i`; assert `icache_ctx_tag` advances by exactly 1 each.
  Pins I-13.
- V-UT-3: `smc_store_invalidates_only_storing_hart` in
  `device/bus.rs::tests` — two-hart bus, hart 0 stores to RAM
  via `RVCore::store` → hart 0's `icache_ctx_tag` advances,
  hart 1's does not. Pins I-9.
- V-UT-4: `mstatus_fs_flip_does_not_bump` (R-003) — write
  `mstatus` toggling only FS; assert `icache_ctx_tag` unchanged.
  Pins I-14.
- V-UT-5: `mstatus_mprv_flip_bumps` (R-003) — write `mstatus`
  toggling MPRV (and separately SUM, separately MXR); assert
  `icache_ctx_tag` advances. Pins I-14.
- V-UT-6: `trap_m_to_m_does_not_bump` (R-004) — take an
  ecall-from-M trap with current privilege already M; assert
  `icache_ctx_tag` unchanged. Take an S→M trap; assert one bump.
  Pins I-15.
- V-UT-7: `mprv_flip_triggers_next_fetch_miss` (R-006) — two
  VA→PA mappings at the same VA; in M-mode `MPRV=0` execute
  mapping A; flip `MPRV=1` with `MPP=U` and a different page
  table live; assert the next icache fetch is a miss (via
  `perf-stats` counter) and executes mapping B's instruction.
- V-UT-8: `mtimer_deadline_short_circuits` in
  `device/aclint/mtimer.rs::tests` — set `mtimecmp[0] = u64::MAX`;
  tick 1000 times; assert `check_all` was not called (via a
  test-only call counter behind `#[cfg(test)]`).
- V-UT-9: `ram_typed_read_matches_memmove` in
  `device/ram.rs::tests` — for each of sizes {1, 2, 4, 8} at
  aligned and unaligned addresses, assert the typed-read path
  (or the memmove fallback on unaligned) returns identical bytes
  to a reference `slice::copy_from_slice` implementation.

[**Integration Tests**]

- V-IT-1: `xkernels/tests/am-tests/src/tests/smc.c` via
  `cd xkernels/tests/am-tests && make m`; `GOOD TRAP` required.
  Runs before the icache exists (baseline) and after (regression).
- V-IT-2: Wall-clock on dhry/cm/mb via `make run` with `DEBUG=n`;
  deltas vs. `docs/perf/2026-04-15/data/bench.csv`.
  Thresholds: dhry ≥ 20 %, cm ≥ 20 %, mb ≥ 15 % (G-5).
- V-IT-3: `make linux`, `make linux-2hart`, `make debian` all
  boot; latency within ±5 % of post-P1 baseline.
- V-IT-4: Hit-rate telemetry via `--features perf-stats`, dumped
  at exit. Threshold ≥ 95 % on dhry/cm/mb (G-1). Linux boot hit
  rate captured opportunistically; recorded in the round's
  `docs/perf/<today>/` artefact but not gated (R-005
  carry-forward).
- V-IT-5: P5 MMU inline asm audit — `cargo asm --release
  --features perf-stats xemu::arch::riscv::cpu::mm::RVCore::checked_read`
  shows the TLB-hit path lowered to ≤ 20 lines of host
  instructions with no `call` to `Bus::read` on the hit branch.
  Evidence committed alongside the implementation.
- V-IT-6: P6 memmove bucket drop — `_platform_memmove` +
  `memcpy` PLT combined < 2 % of self-time on dhry/cm/mb in the
  round's `docs/perf/<today>/data/*.sample.txt`.
  `Bus::read/write` combined drops by ≥ 3 pp.

[**Failure / Robustness Validation**]

- V-F-1: `decode_failure_does_not_poison_line` — inject a raw
  word that fails `pest`; assert the cache line at that index is
  unchanged (pre-existing valid line survives).
- V-F-2: `mid_execution_satp_change_triggers_refetch` — two page
  mappings of the same VA; execute, change satp via `csr_write`,
  execute; assert the new mapping's instruction runs.
- V-F-3: `ctx_tag_u64_headroom_documented` — compile-time
  assertion that `icache_ctx_tag` is `u64`. Replaces the round-00
  wrap-defence TODO (R-007).
- V-F-4: `mprv_flip_with_alternate_mapping` — variant of V-UT-7
  exercising the full `step → fetch → decode_cached` path, not
  just the ctx_tag bump hook. Pins R-006 at integration level.
- V-F-5: `mtimer_reset_restores_deadline_to_max` — after
  `reset()`, assert `next_fire_mtime == u64::MAX` and the
  fast-path short-circuits for every subsequent tick until a
  guest writes `mtimecmp`.
- V-F-6: `checked_write_pmp_fault_does_not_bump_ctx_tag` — write
  to a read-only PMP region; assert `bus.store` returns `Err`
  and `icache_ctx_tag` is unchanged (Failure Flow #5).

[**Edge Case Validation**]

- V-E-1: `compressed_and_full_inst_at_adjacent_pc` — execute
  `c.addi` at `pc=P`, `addi` at `pc=P+2`; both should hit on
  re-execution. Pins I-7, I-8.
- V-E-2: `index_aliasing_at_conflict` — craft two PCs separated
  by `2*ICACHE_LINES`; assert tag mismatch forces miss-and-
  replace; both still execute correctly.
- V-E-3: `interrupt_between_steps_does_not_corrupt` — fire an
  interrupt between steps; trap entry bumps ctx_tag (if
  privilege changes) → next decode is a miss → correct.
- V-E-4: `mtimecmp_write_to_u64_max_sets_deadline_max` — write
  `u64::MAX` to `mtimecmp[0]` of a 1-hart system; assert
  `next_fire_mtime == u64::MAX` and gate short-circuits.
- V-E-5: `ram_read_size_3_falls_through_to_memmove` — size ∉
  {1, 2, 4, 8} takes the generic path; output matches reference.
- V-E-6: `mmio_read_takes_device_path_not_typed` — read from the
  Mtimer MMIO region; typed bypass must NOT activate (would skip
  device side effects). Pinned by inspecting the `Bus::read`
  dispatch.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (icache-hit eliminates pest walk) | V-IT-4, V-IT-5 |
| G-2 (Mtimer deadline short-circuits) | V-UT-8, V-IT-2 |
| G-3 (MMU/trap inline) | V-IT-5 |
| G-4 (memmove bypass) | V-UT-9, V-IT-6 |
| G-5 (wall-clock thresholds) | V-IT-2 |
| G-6 (≥ 10 pp xdb::main drop) | V-IT-2 (re-run sample.sh) |
| G-7 (combined bucket drop ≥ 20 pp) | V-IT-2, V-IT-6 |
| G-8 (correctness under SMC/PT/fence.i/priv/MPRV) | V-IT-1, V-UT-3, V-UT-7, V-F-1, V-F-2, V-F-4 |
| G-9 (per-hart fence.i) | V-UT-3 |
| C-1 (no benchmark-targeted code) | Review grep for workload names |
| C-7 (no Mutex regression) | `bash scripts/ci/verify_no_mutex.sh` |
| C-9 (SMC is global flush) | V-F-6 (PMP) + V-UT-3 (scoping) |
| C-10 (perf-stats off by default) | Cargo.toml default features inspection |
| C-11 (no new unsafe warnings) | `cargo clippy --all-targets` post-Phase-8 |
| I-7 (compressed handling) | V-E-1 |
| I-8 (aliasing) | V-E-2 |
| I-9 (per-hart SMC) | V-UT-3, V-F-6 |
| I-10 (no Mutex) | C-7 script |
| I-11 (u64 monotone) | V-F-3 |
| I-13 (fence.i local-hart) | V-UT-2 |
| I-14 (mstatus bit-isolation) | V-UT-4, V-UT-5 |
| I-15 (privilege debounce) | V-UT-6 |
| I-16 (mtimer deadline) | V-UT-8, V-F-5, V-E-4 |
| I-17 (RAM+aligned+size∈{1,2,4,8}) | V-UT-9, V-E-5, V-E-6 |

## Exit Gate

All of the following, measured against `docs/perf/2026-04-15/`:

- Wall-clock reduction ≥ **20 %** on dhrystone, ≥ **20 %** on
  coremark, ≥ **15 %** on microbench (G-5).
- `xdb::main` self-time share drops ≥ **10 pp** on all three
  workloads (from 40/47/45 % → ≤ 30/37/35 %) (G-6).
- Combined icache-addressable + Mtimer + memmove + MMU buckets
  drop by ≥ **20 pp** of aggregate self-time share (G-7).
- Icache hit rate ≥ **95 %** on dhry/cm/mb under
  `--features perf-stats` (G-1).
- Combined `Mtimer::check_timer + tick + mtime` bucket < **1 %**
  on all three workloads (G-2, PERF_DEV §3 P3 exit gate).
- `_platform_memmove + memcpy` PLT < **2 %** on all three
  workloads; `Bus::read + Bus::write` drops ≥ **3 pp** combined
  (G-4).
- MMU-entry bucket ≤ **10 %** (drops ≥ 3 pp from 12–15 %); trap
  bucket drops ≥ **1 pp** (G-3, PERF_DEV §3 P5 exit gate).
- `make linux`, `make linux-2hart`, `make debian` all boot
  cleanly; latency within ±5 % of post-P1 baseline (V-IT-3).
- `cd xkernels/tests/am-tests && make m` shows `GOOD TRAP`
  (V-IT-1).
- `cargo test --workspace` green; test count ≥ 372 + 1 + 6 + 1
  doc-test + **≥ 9 new tests** (V-UT-1..V-UT-9).
- `bash scripts/ci/verify_no_mutex.sh` reports `ok` (C-7, P1
  guard).
- `make fmt && make clippy` clean; no new `unsafe` warnings
  (C-11).
- Benchmark artefacts committed under `docs/perf/<today>/`
  including `data/bench.csv`, `data/*.sample.txt`,
  `graphics/*.svg`, and the asm-audit transcript for V-IT-5.

If any single phase's individual exit sub-threshold misses but
the combined Exit Gate passes, the round lands and the
under-performing phase is revisited in round 02 with a targeted
hypothesis. If the combined Exit Gate misses, the round returns
to `02_PLAN` with the failing phases split back out per T-1.
