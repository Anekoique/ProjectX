# `perfIcache` PLAN `00`

> Status: Draft
> Feature: `perfIcache`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`
> - Master Directive: `none`

---

## Summary

Phase **P4** of the xemu perf roadmap introduces a per-hart, direct-mapped, 4096-entry decoded-instruction cache (icache) in front of the pest-based decoder at `xemu/xcore/src/isa/riscv/decoder.rs:132`. The post-P1 baseline at `docs/perf/2026-04-15/REPORT.md` shows `xdb::main` (the LTO-folded fetch + decode + dispatch + execute bucket) at 40.4 / 46.8 / 44.7 % self-time on dhrystone / coremark / microbench — the largest single bucket. Because the working set of these workloads is very small (dhry ≈ 200 static instructions, cm ≈ 1.5 K, mb < 100), a single tag compare can elide the pest tree walk and `DecodedInst::from_raw` allocation on the vast majority of fetches. We expect a wall-clock reduction of **15–25 %** on the 2026-04-15 baseline, mirroring the bucket math.

The change is structural: it is keyed on `(pc, ctx_tag)` where `ctx_tag` is a per-hart monotone counter bumped on every event that can change instruction-stream meaning (satp / sfence.vma / fence.i / privilege transitions / RAM stores). No benchmark-specific PC ranges, no name detection, no hot-loop specialisation — the design must apply equally to dhry, cm, mb, `make linux`, `make linux-2hart`, and `make debian`. `pest` remains the miss fallback; the decoder is not rewritten.

## Log {None in 00_PLAN}

[**Feature Introduce**]

- New per-hart icache: 4096 direct-mapped lines, key `(pc, ctx_tag, raw)`, value `DecodedInst`.
- New `RVCore` fields: `icache: Box<ICache>`, `icache_ctx_tag: u32`.
- New invalidation hooks at six precise points (satp write, sfence.vma, fence.i, mret, sret, trap entry) plus a conservative SMC hook in `Bus::store`.
- New torture test (am-test `smc.c` + Rust unit) that lands **before** the cache, gating the optimisation on observable correctness.
- `DecodedInst` gains `Copy` (currently only `Clone, PartialEq, Eq`).

[**Review Adjustments**]

N/A — first round.

[**Master Compliance**]

N/A — first round.

### Changes from Previous Round

[**Added**] N/A — first round.
[**Changed**] N/A — first round.
[**Removed**] N/A — first round.
[**Unresolved**]

- Exact `mstatus`-bit gating for ctx-tag bump (MPRV / MPP / SUM / MXR isolation vs. bumping on any `mstatus` write). Phase 1 will bump on any `mstatus` write and document the over-conservative choice in code; reviewer feedback may sharpen this in 01.
- Whether 4096 entries is sufficient for full Linux boot. Decision deferred until the V-IT-4 hit-rate telemetry runs against `make linux`. If hit rate < 95 % on Linux boot, we will widen to 16 K in 01 (cite NEMU IBuf precedent).

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| — | — | N/A (first round) | No prior review or master findings exist. |

> Rules:
> - Every prior HIGH / CRITICAL finding must appear here.
> - Every Master directive must appear here.
> - Rejections must include explicit reasoning.

---

## Spec {Core specification}

[**Goals**]

- G-1: Eliminate the pest tree walk on the fetch fast path for every cache hit; the hot path becomes one tag compare and one POD copy of `DecodedInst`.
- G-2: Achieve ≥ 15 % wall-clock reduction on dhrystone, ≥ 15 % on coremark, ≥ 10 % on microbench, vs. `docs/perf/2026-04-15/data/bench.csv`.
- G-3: Drop `xdb::main` self-time share by ≥ 10 percentage points on all three workloads (measurable via `make perf`).
- G-4: Preserve correctness under self-modifying code, page-table changes, fence.i, and privilege transitions — proven by a torture test that lands first.
- G-5: Maintain multi-hart soundness: `fence.i` invalidates only the issuing hart's icache (Zifencei §5.1).

- NG-1: No JIT, no trace chaining, no threaded dispatch (those belong to a future phase; cite Ertl & Gregg 2003 in Trade-offs).
- NG-2: No paddr-tagged SMC bitmap (Phase 2 follow-up; out of scope).
- NG-3: No replacement for `pest` decoder — it remains the miss fallback.
- NG-4: No benchmark-specific code paths of any kind (no PC-range special cases, no `name == "dhrystone"` switches, no hot-loop unrolls).
- NG-5: No changes to assembly files (per `feedback_no_modify_asm`).

[**Architecture**]

```
                ┌──────────────────────────────────────────────────────────┐
                │ RVCore::step  (xemu/xcore/src/arch/riscv/cpu.rs:238-246) │
                └─────────────────────────────┬────────────────────────────┘
                                              │
                       fetch(bus) -> raw:u32  ▼
                ┌──────────────────────────────────────────────────────────┐
                │  idx = (pc >> 1) & MASK                                  │
                │  line = &mut self.icache[idx]                            │
                │  hit  = line.pc == pc                                    │
                │      && line.ctx_tag == self.icache_ctx_tag              │
                │      && line.raw == raw                                  │
                └──────────────────────────────┬───────────────────────────┘
                                hit            │            miss
                                 ▼                            ▼
                       decoded = line.decoded   decoded = decoder.decode(raw)?
                                                *line = ICacheLine { pc, ctx_tag,
                                                                     raw, decoded }
                                 │
                                 ▼
                          dispatch(bus, decoded)?

  Invalidation (per-hart, bump self.icache_ctx_tag):
    csr write satp        — csr/ops.rs:30-43
    sfence.vma            — inst/privileged.rs:97-125
    fence.i               — inst/privileged.rs:71-79  (was NOP)
    mret  / sret          — trap/handler.rs:129-164
    trap entry            — trap/handler.rs:77-115
    mstatus write         — csr/ops.rs:44- (conservative, bump on any write)
    Bus::store on RAM     — device/bus.rs:189-193 (Phase-1 SMC, bump storer's tag)
```

The shape mirrors QEMU's per-CPU jump cache geometry (`include/exec/tb-jmp-cache.h`, `TB_JMP_CACHE_BITS = 12`) without the QHT backing — we do not need TB chaining in an interpreter, only the single direct-mapped layer. NEMU's IBuf provides the second precedent (4-way × 4096, invalidated on the same set of events).

[**Invariants**]

- I-1: `RVCore::step` always calls `fetch` before any icache lookup; the cache is consulted only with the `raw` word that was actually fetched in this step.
- I-2: A line with `ctx_tag != self.icache_ctx_tag` is treated as miss regardless of `pc`/`raw`.
- I-3: The cache is owned per-hart; no shared state, no atomics, no locks.
- I-4: `pest` remains the only authority for decoding; the cache only memoises its result, never its inputs.
- I-5: On miss, the cache line is overwritten in full (`pc`, `ctx_tag`, `raw`, `decoded`) — partial updates are forbidden.
- I-6: The miss path's error behaviour (`XError::InvalidInst`) is identical to today's behaviour; an icache miss does not change which instructions trap.
- I-7: Compressed-instruction support: `raw` carries the fetched word as-is — for compressed instructions this is the 16-bit word zero-extended to `u32`, matching what `fetch` returns at `mm.rs:306-315` and what `decoder.decode` accepts.
- I-8: The cache index uses `(pc >> 1) & MASK` so that compressed (2-byte) and standard (4-byte) instructions both land in distinct slots; the full `pc` is the tag, so aliasing is detected.
- I-9: A `Bus::store` that writes to a RAM region invalidates only the storing hart's icache (Phase-1 conservative). Stores to MMIO regions do not bump the tag.
- I-10: All `DecodedInst` variants are POD with no heap allocation — verified by deriving `Copy` and pinning it with V-UT-1.
- I-11: `icache_ctx_tag` is monotone-non-decreasing for the lifetime of a hart; it is never decremented and never wraps within a reboot. Type is `u32`; at realistic invalidation rates (Linux boot ≈ 10 K satp writes + millions of stores), wrap requires ~4 G events and is unreachable in practice. Documented and asserted in V-UT.
- I-12: An icache line with `ctx_tag != self.icache_ctx_tag` is treated as miss; write-through on decode updates all four fields and sets `ctx_tag = self.icache_ctx_tag`.
- I-13: `fence.i` bumps only the issuing hart's `icache_ctx_tag`, not peers' (Zifencei §5.1: fence.i is local-hart only).

[**Data Structure**]

```rust
// xemu/xcore/src/arch/riscv/cpu/icache.rs (new file)

use crate::isa::riscv::decoder::DecodedInst;
use memory_addr::VirtAddr;

pub const ICACHE_BITS: usize = 12;
pub const ICACHE_LINES: usize = 1 << ICACHE_BITS;
pub const ICACHE_MASK: usize = ICACHE_LINES - 1;

#[derive(Clone, Copy)]
pub struct ICacheLine {
    pub pc: VirtAddr,        // full guest virtual address — tag
    pub ctx_tag: u32,        // monotone per-hart
    pub raw: u32,            // raw fetched word (u16 zero-extended for C ext)
    pub decoded: DecodedInst, // POD; one variant per encoding format
}

impl ICacheLine {
    // Sentinel: ctx_tag == 0 is treated as invalid because RVCore
    // initialises icache_ctx_tag = 1, so no live line can ever match.
    pub const INVALID: Self = Self {
        pc: VirtAddr::from_usize(0),
        ctx_tag: 0,
        raw: 0,
        decoded: DecodedInst::C { kind: InstKind::illegal, inst: 0 },
    };
}

pub struct ICache {
    pub lines: [ICacheLine; ICACHE_LINES],
}

impl ICache {
    pub fn new() -> Box<Self> {
        Box::new(Self { lines: [ICacheLine::INVALID; ICACHE_LINES] })
    }

    #[inline]
    pub fn index(pc: VirtAddr) -> usize {
        (pc.as_usize() >> 1) & ICACHE_MASK
    }
}
```

`DecodedInst` already exists at `decoder.rs:160-173` with all-POD variants (`InstKind` is a C-style enum, `RVReg` is a newtype over `u8`, `SWord` is an integer). The current derive is `#[derive(Clone, PartialEq, Eq)]`; we add `Copy`. V-UT-1 pins this.

`RVCore` (`cpu.rs:36-54`) gains exactly two new fields:

```rust
pub struct RVCore {
    // ... existing fields unchanged ...
    pub(in crate::arch::riscv) icache: Box<ICache>,
    pub(in crate::arch::riscv) icache_ctx_tag: u32, // starts at 1
}
```

`Box` keeps the cache off the stack. One allocation per hart at construction, never resized.

[**API Surface**]

```rust
// In ICache (new file)
impl ICache {
    pub fn new() -> Box<Self>;
    #[inline] pub fn index(pc: VirtAddr) -> usize;
}

// In RVCore (xemu/xcore/src/arch/riscv/cpu.rs)
impl RVCore {
    /// Bump the per-hart icache context tag. Cheap (one wrapping_add).
    /// Called by every invalidation hook listed in Architecture.
    #[inline]
    pub(in crate::arch::riscv) fn invalidate_icache(&mut self) {
        self.icache_ctx_tag = self.icache_ctx_tag.wrapping_add(1);
    }

    /// Replace the existing decode call inside step() with cache-aware decode.
    /// Pure private helper, no semantic change vs. raw decoder.decode(raw).
    #[inline]
    fn decode_cached(&mut self, raw: u32) -> XResult<DecodedInst>;
}
```

No new public API on the RVCore facade; everything is `pub(in crate::arch::riscv)`. The decoder's existing `pub fn decode(&self, inst: u32) -> XResult<DecodedInst>` (`decoder.rs:131`) is unchanged.

[**Constraints**]

- C-1: No benchmark-targeted code. Every line of the icache must apply identically to dhrystone, coremark, microbench, `make linux`, `make linux-2hart`, `make debian`, and any future guest. Reviewer must reject the plan if any constant, branch, or special case keys off workload identity.
- C-2: Realistic gain only — 15–25 % wall-clock. The bucket math (xdb::main ≈ 40–47 % self-time, decode being the majority of the work) supports this range and not more. Plan does not promise > 25 %.
- C-3: `make fmt && make clippy && make run && make test` must pass after every implementation iteration (AGENTS.md §Development Standards).
- C-4: Benchmarks run with `DEBUG=n` (per `feedback_debug_flag`).
- C-5: Workloads launched via `make run` / `make linux` / `make debian` (per `feedback_use_make_run`); never direct `target/release/xdb` invocations.
- C-6: No assembly-file modifications (per `feedback_no_modify_asm`).
- C-7: P1 regression guard (`bash scripts/ci/verify_no_mutex.sh`) must remain `ok` — the icache stays per-hart, no shared mutex.
- C-8: Scope is strictly "decoded-instruction cache + invalidation hooks + torture test." No P3 (Mtimer), no P5 (MMU inlining), no P6 (memmove) work bleeds into this phase.
- C-9: Phase 1 SMC strategy is global flush on RAM store (correct, may cost hit-rate on code-writing guests). Paddr-tagged refinement is explicitly deferred; a comment in `Bus::store` must reference this PLAN.

---

## Implement {detail design}

### Execution Flow

[**Main Flow**]

1. `RVCore::step` calls `self.fetch(bus)?` → `raw: u32`.
2. `idx = ICache::index(self.pc)`.
3. Read `let line = &mut self.icache.lines[idx];`.
4. Check `hit = line.pc == self.pc && line.ctx_tag == self.icache_ctx_tag && line.raw == raw`.
5. If hit: `decoded = line.decoded` (POD copy).
6. If miss: `decoded = self.decoder.decode(raw)?`; then `*line = ICacheLine { pc: self.pc, ctx_tag: self.icache_ctx_tag, raw, decoded }`.
7. `self.dispatch(bus, decoded)?` (the macro-generated match in `inst.rs:58-76`, unchanged).
8. `pc` advancement is governed by existing logic (`mm.rs:306-315` already handles 2-vs-4-byte advance based on the low bits of `raw`).

[**Failure Flow**]

1. `fetch` traps (page fault / access fault / misalignment) → return `Err(...)` before any cache access. Cache state is unchanged.
2. `decoder.decode(raw)?` returns `Err(XError::InvalidInst)` on miss → return error before writing the line. Cache state is unchanged. **Crucial**: we do not write the line on decode failure, so a transient illegal word does not poison the line for a subsequent legal re-fetch at the same PC.
3. `dispatch` returns `Err(...)` (illegal instruction during execute, e.g. CSR access trap) → cache line is *kept* (decoding succeeded), tag stays valid. Correct because the next fetch at the same PC will re-decode to the same `DecodedInst` and trap identically.
4. SMC race: a hart writes to its own code page mid-execution. The store path bumps `ctx_tag` *after* the byte hits memory; the next `fetch` at any PC will re-fetch from the updated memory and miss the cache (old `ctx_tag`). Correct.
5. Tag wrap (I-11): `u32` wrap after ~4 G invalidation events. Unreachable in practice; documented. A future hardening step could flush all lines on wrap; out of scope for round 00.

[**State Transition**]

- `(line valid, ctx_tag matches) -> hit` when `pc` and `raw` also match.
- `(line valid, ctx_tag mismatch) -> miss` when ctx_tag was bumped since this line was filled.
- `miss + decode succeeds -> line replaced` (always overwrite, no eviction policy needed for direct-mapped).
- `miss + decode fails -> line unchanged`, error propagates.
- `Bus::store(RAM) -> ctx_tag bumped` (Phase-1 SMC hook).
- `csr_write satp / sfence.vma / fence.i / mret / sret / trap entry / mstatus write -> ctx_tag bumped`.

### Implementation Plan

The order is non-negotiable: the torture test lands first so we can run it red against the no-icache baseline (it should pass — fence.i is currently a NOP), then green against the icache implementation.

[**Phase 1 — Torture test (lands FIRST, before any optimisation code)**]

1. Add `xkernels/tests/am-tests/src/tests/smc.c`:
   - Allocate a 4-byte aligned buffer in RAM.
   - Encode an `addi x1, x0, 0` (raw `0x00000093`) at the buffer; jump-and-link to it; assert `x1 == 0`.
   - Encode `addi x1, x0, 42` (raw `0x02a00093`) over the same buffer.
   - Issue `fence.i` (`asm volatile("fence.i" ::: "memory")`).
   - Jump to the buffer again; assert `x1 == 42`.
   - Failure path prints PASS/FAIL and exits.
2. Wire it into the am-tests harness next to existing tests so `make run AM=smc` runs it.
3. Add `xemu/xcore/src/arch/riscv/cpu/icache.rs` mod stub (empty file, behind `pub mod icache;` in `cpu.rs`) so the next phase compiles cleanly.
4. Add Rust unit `smc_fence_i_invalidates_icache()` in `xemu/xcore/src/arch/riscv/cpu/inst/privileged.rs::tests`:
   - Use the `setup_core` helper at `privileged.rs:139-145`.
   - Inject a raw word at a RAM PC, step once, mutate the byte through `bus.store`, call `core.fence_i(...)`, step again, assert the new instruction's effect.
   - This test passes today (no icache means no staleness to worry about) and must keep passing after the icache lands.

**Gate**: `make test` and `make run AM=smc` both green before Phase 2 begins.

[**Phase 2 — ICache struct + step integration**]

1. Implement `ICache`, `ICacheLine`, `ICACHE_BITS = 12` in `icache.rs`.
2. Derive `Copy` on `DecodedInst` at `decoder.rs:161` (and verify all variants are POD; V-UT-1 enforces this).
3. Add `icache: Box<ICache>` and `icache_ctx_tag: u32` to `RVCore` (`cpu.rs:36-54`); initialise in `RVCore::new` to `ICache::new()` and `1`.
4. Add `RVCore::invalidate_icache` and `RVCore::decode_cached` per **API Surface**.
5. Replace the `core.decode(raw)?` call in `step` (`cpu.rs:238-246`) with `self.decode_cached(raw)?`.
6. Add V-UT-1 (`DecodedInst: Copy`) and a basic V-UT for hit/miss behaviour in `icache.rs::tests`.

**Gate**: `make fmt && make clippy && make test` green; the SMC torture from Phase 1 still green.

[**Phase 3 — Invalidation hooks**]

Each hook is a single `self.invalidate_icache()` call at the point listed:

1. `csr_write_side_effects` at `csr/ops.rs:30-43` — bump inside the `0x180 /* satp */` arm, after `self.mmu.update_satp(satp)`.
2. `csr_write_side_effects` at `csr/ops.rs:44-…` — bump in the `0x300 /* mstatus */ | 0x100 /* sstatus */` arm. Comment explicitly: `// Conservative: bump on any mstatus write because MPRV/MPP/SUM/MXR all change MMU index. A bit-isolated bump can replace this in 01_PLAN if reviewer asks.`
3. `sfence_vma` at `inst/privileged.rs:97-125` — bump after `self.mmu.tlb.flush(vpn, asid)`.
4. `fence_i` at `inst/privileged.rs:71-79` — replace the NOP body with `self.invalidate_icache()`. Update the line-70 comment to: `/// Instruction fence — invalidates this hart's decoded-instruction cache (Zifencei §5.1, local-hart only).`
5. `do_mret` at `trap/handler.rs:129-145` — bump after `self.privilege = mpp` (line 139).
6. `do_sret` at `trap/handler.rs:147-164` — bump after `self.privilege = spp` (line 157).
7. Trap entry at `handler.rs:77-115` — bump after each `self.privilege = …` mutation (lines 101 and 106).
8. `Bus::store` at `device/bus.rs:189-193` — Phase-1 SMC: when the store target falls in a RAM region, route an `invalidate_icache()` to the storing hart's `RVCore`. Mechanism: `Bus::store` already takes `hart: HartId` and already has the LR/SC peer-reservation hook for cross-hart effects, so we add an analogous "icache invalidate self" callback through the existing per-hart core handle. Exact wiring follows the LR/SC pattern; reviewer please scrutinise — this is the trickiest plumbing in the patch.

Add V-UT-2 (each hook bumps `icache_ctx_tag`) and V-UT-3 (RAM store bumps tag, MMIO store does not).

**Gate**: `make fmt && make clippy && make test && make run AM=smc` green. `cargo test --workspace` shows ≥ 372 + N tests where N ≥ 3 (V-UT-1, V-UT-2, V-UT-3 minimum).

[**Phase 4 — Benchmarks + telemetry**]

1. Temporary instrumentation (feature-flagged behind `#[cfg(feature = "icache_stats")]`): two `u64` counters `hits` / `misses` on `RVCore`, dumped at exit. Used to verify hit rate.
2. Run `make run` against `dhrystone`, `coremark`, `microbench` with `DEBUG=n`. Record wall-clock and hit rate.
3. Compare against `docs/perf/2026-04-15/data/bench.csv`. Confirm exit-gate thresholds (G-2, G-3).
4. Run `make linux`, `make linux-2hart`, `make debian`. Confirm boot, latency within ±5 %, hit rate ≥ 95 % on Linux. If Linux hit rate is below threshold, file the 16 K bump as a 01-round consideration.
5. Remove `icache_stats` feature flag (or leave it gated behind `--features` for future profiling — reviewer's call).

**Gate**: All exit-gate items below pass.

## Trade-offs {ask reviewer for advice}

- **T-1: Direct-mapped 4096 entries vs. set-associative or hash.**
  - *Direct-mapped (proposed)*: One `&mut` and three integer compares per fetch. No replacement policy. Simpler code, predictable host-cache behaviour.
    - Cost: aliasing on conflicts. With < 1.5 K static instructions in our benchmarks, this is negligible (load factor < 40 %).
  - *Set-associative (NEMU IBuf: 4-way × 4096)*: Resilient to aliasing. ~4× the compare work per fetch.
    - Cost: noticeably more arithmetic on the hot path; LRU bookkeeping.
  - *Hash table (QEMU QHT)*: Used by QEMU because TBs are large variable-length objects with chained jumps. We have neither variable-length objects nor chaining.
  - Recommendation: start with direct-mapped (cite QEMU `include/exec/tb-jmp-cache.h`, `TB_JMP_CACHE_BITS = 12`). Re-measure after Linux boot; if hit rate < 95 %, escalate to 4-way (cite NEMU IBuf). **Reviewer: do you want us to skip ahead to 4-way to avoid a potential 01-round revisit?**

- **T-2: Cache size — 4 K vs. 16 K.**
  - 4 K (proposed) → fits per-hart in host L2; sufficient for dhry / cm / mb.
  - 16 K (NEMU IBuf precedent) → 4× memory, better Linux hit rate.
  - Recommendation: 4 K + telemetry; bump in 01 if Linux warrants. **Reviewer: same question as T-1 — pre-emptive bump?**

- **T-3: Per-hart vs. shared icache.**
  - Per-hart (proposed): No locks, no atomics. Matches the P1 mutex-free invariant. Fence.i semantics align naturally (Zifencei §5.1, local-hart only).
  - Shared with reader-writer lock: Less memory if hart count grows large, but introduces lock contention on the hottest path. QEMU MTTCG (https://lwn.net/Articles/697265/) chose per-CPU jump cache + a shared QHT precisely because the per-CPU layer is contention-free.
  - Recommendation: per-hart, definitively.

- **T-4: SMC strategy — global flush vs. paddr-tagged.**
  - Global flush on every RAM store (proposed, Phase 1): one `wrapping_add` per store. Correct. Costs hit rate on code-writing guests.
  - Paddr-tagged bitmap (Phase 2 deferred): only flush when store hits a code-tagged page. More complex; win is real only on guests that store to data while running code (most steady-state workloads do not).
  - Recommendation: Phase 1 conservative, defer Phase 2 unless `make linux` regresses. **Reviewer: confirm the deferral is acceptable.**

- **T-5: When to bump on `mstatus` writes — bit-isolated vs. any write.**
  - Any `mstatus` write (proposed): one extra bump per CSR write to `mstatus` / `sstatus` (rare in steady state). Trivial code.
  - Bit-isolated (MPRV / MPP / SUM / MXR only): correct minimum. Requires reading old vs. new value and masking. Slightly more code; saves a few bumps on workloads that only flip FS/SD.
  - Recommendation: any-write for round 00; bit-isolate in 01 only if reviewer measures it costs hit rate.

- **T-6: When to invalidate on traps — every trap vs. only privilege-changing traps.**
  - Every trap entry (proposed): bump unconditionally at `handler.rs:101` and `handler.rs:106`. Simple.
  - Only when MPP/SPP differs from current privilege: subtler; traps that don't change privilege are vanishingly rare.
  - Recommendation: every-trap bump; cost is ~one `wrapping_add` per trap, irrelevant to wall-clock.

- **T-7: Threaded dispatch (out of scope, mentioned for context).**
  - Ertl & Gregg 2003 (https://www.complang.tuwien.ac.at/forth/threaded-code.html) document a 20–50 % speedup from direct-threaded vs. switch-dispatch on classical interpreters. This is a *separate* phase that complements the icache. Cite in the PERF_DEV roadmap, not in this PLAN's scope.

## Validation {test design}

[**Unit Tests**]

- V-UT-1: `decoded_inst_is_copy` in `xemu/xcore/src/isa/riscv/decoder.rs::tests` — `fn _assert_copy<T: Copy>() {}` against `DecodedInst`. Pins I-10. Static-assert; fails at compile time if any future variant adds a non-`Copy` field.
- V-UT-2: `ctx_tag_bumps_on_invalidation_events` in a new test module under `cpu/icache.rs::tests` — drive `csr_write` for satp, call `sfence_vma`, call `fence_i`, call `do_mret`, call `do_sret`, take a trap; assert `icache_ctx_tag` advances by exactly 1 each time. Pins the Phase 3 hook list.
- V-UT-3: `smc_store_invalidates_only_storing_hart` in `device/bus.rs::tests` — two-hart `Bus`, hart 0 stores to RAM, assert hart 0's `icache_ctx_tag` advanced and hart 1's did not. Pins I-9 and I-13's spirit (per-hart isolation).

[**Integration Tests**]

- V-IT-1: `xkernels/tests/am-tests/src/tests/smc.c` runs via `make run AM=smc`; output line `PASS smc` required. Runs **before** the icache exists (sanity baseline) and again after (regression gate).
- V-IT-2: Wall-clock measurement on dhrystone, coremark, microbench with `DEBUG=n` via `make run`; deltas computed against `docs/perf/2026-04-15/data/bench.csv`. Thresholds: dhry ≥ 15 %, cm ≥ 15 %, mb ≥ 10 %.
- V-IT-3: `make linux` and `make linux-2hart` boot to user-space prompt; `make debian` boots to login. Latency within ±5 % of post-P1 baseline.
- V-IT-4: Hit-rate telemetry (temporary, behind `#[cfg(feature = "icache_stats")]`): emit `hits / (hits + misses)` at exit. Threshold ≥ 95 % on dhry / cm / mb.

[**Failure / Robustness Validation**]

- V-F-1: `decode_failure_does_not_poison_line` — inject a raw word that fails `pest`, observe miss path returns `Err` and the existing valid line at that index (if any) is unchanged.
- V-F-2: `mid-execution_satp_change_triggers_refetch` — set up two distinct page mappings of the same VA, run an instruction, change satp via `csr_write`, run again, assert the *new* mapping's instruction executes.
- V-F-3: `tag_wrap_does_not_corrupt` (lower priority) — force `icache_ctx_tag = u32::MAX`, trigger one invalidation, assert lookups behave correctly. Documents I-11's wrap semantics.

[**Edge Case Validation**]

- V-E-1: `compressed_instruction_caches_correctly` — execute a `c.addi` (16-bit) at PC=p, then a 32-bit `addi` at PC=p+2; both should hit on second execution. Pins I-7, I-8.
- V-E-2: `aliasing_at_index_collision` — manually craft two distinct PCs that map to the same `idx`, assert tag-mismatch causes miss-and-replace, both still execute correctly.
- V-E-3: `interrupt_during_step_does_not_corrupt_line` — fire an interrupt between steps, assert the cache is unaffected (interrupt path runs trap entry, which bumps the tag → next decode is a miss; correct).

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (eliminate pest walk on hit) | V-IT-4 (hit rate ≥ 95 %), inspection of generated assembly via `make perf` |
| G-2 (≥ 15 % wall-clock dhry/cm; ≥ 10 % mb) | V-IT-2 |
| G-3 (≥ 10 pp drop in xdb::main self-time) | V-IT-2 (re-run `make perf`) |
| G-4 (correctness under SMC + ctx changes) | V-IT-1, V-UT-3, V-F-1, V-F-2 |
| G-5 (fence.i is local-hart only) | V-UT-3 |
| C-1 (no benchmark-targeted code) | Code review by plan-reviewer; grep for benchmark names in diff |
| C-7 (no shared mutex regression) | `bash scripts/ci/verify_no_mutex.sh` post-merge |
| I-7 (compressed inst handling) | V-E-1 |
| I-9 (per-hart SMC isolation) | V-UT-3 |
| I-11 (monotone non-decreasing tag) | V-UT-2 (each event +1), V-F-3 (wrap behaviour documented) |
| I-13 (fence.i local) | V-UT-3 |

## Exit Gate

All of the following, measured against `docs/perf/2026-04-15/`:

- Wall-clock reduction ≥ 15 % on dhrystone, ≥ 15 % on coremark, ≥ 10 % on microbench.
- `xdb::main` self-time share drops ≥ 10 pp on all three workloads (re-run `make perf`).
- Icache hit rate ≥ 95 % on dhry / cm / mb (V-IT-4 telemetry).
- `cargo test --workspace` green; test count ≥ 372 + 3 (V-UT-1, V-UT-2, V-UT-3 minimum; more from V-F / V-E).
- `make linux` and `make linux-2hart` boot cleanly, latency within ±5 % of post-P1 baseline.
- `make debian` boots to login.
- `make run AM=smc` (the new SMC torture test) passes.
- `bash scripts/ci/verify_no_mutex.sh` reports `ok` (P1 regression guard).
- `make fmt && make clippy` clean.
