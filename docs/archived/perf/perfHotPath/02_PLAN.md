# `hotPath` PLAN `02`

> Status: Draft
> Feature: `hotPath`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `01_MASTER.md` (blank; `00_MASTER.md` directives M-001/M-002/M-003 remain in force)

---

## Summary

Round 02 keeps the bundled `hotPath` scope mandated by `00_MASTER.md` M-001
(P3 Mtimer deadline gate + P4 decoded-instruction cache + P5 MMU fast-path
inlining + P6 memmove typed-read bypass) but lands two decisive
simplifications in response to `01_REVIEW.md`:

1. **R-001 HIGH.** The P4 cache is re-scoped as a pure
   **decoded-raw cache keyed on `(pc, raw)`**. `RVCore::step` already calls
   `fetch` before `decode`, and `Decoder::decode(raw)` at
   `xemu/xcore/src/isa/riscv/decoder.rs:130-143` is a pure function of the
   raw bits plus static dispatch tables; privilege mode, SUM/MXR, MPRV,
   ASID, and satp do not affect what `DecodedInst` comes out. The entire
   invalidation lattice from 01_PLAN (satp / sfence.vma / fence.i /
   privilege-change / mstatus-bit-isolation / RAM-store flush) therefore
   **collapses to zero hooks**. Self-modifying code falls out for free:
   when guest memory changes, the next fetch reads a different `raw`, the
   `(pc, raw)` comparison misses, and the line is overwritten. The cache
   line shrinks to `{ pc, raw, decoded }` — no `ctx_tag` field, no
   bump points, no debounce logic.
2. **R-002 HIGH.** The Exit Gate is split into **binding per-phase gates**
   plus combined bundle gates. If any P3/P4/P5/P6 sub-threshold misses,
   that phase is not considered landed and must be split into a follow-up
   iteration; the bundled implementation workflow survives for M-001 /
   M-003 reasons, but attribution stays phase-scoped per
   `docs/PERF_DEV.md:186-190`.

Two follow-on corrections land alongside:

- **R-003 MEDIUM.** P5's "trap slim" subgoal is re-scoped. The
  steady-state zero-pending-trap branch lives in `RVCore::retire`
  (`xemu/xcore/src/arch/riscv/cpu.rs:152-159`), already a tight
  `Option::take` match; `commit_trap` (in `trap/handler.rs:58-75`) is
  only entered after `pending_trap.take()` returns `Some`. Round 02
  adopts **R-003 Option A**: drop trap-slim from this bundle. P5 focuses
  solely on MMU fast-path inlining. If later profiling surfaces a real
  trap branch, it goes into a targeted future iteration.
- **R-004 MEDIUM.** Both Validation Strategy and Exit Gate now carry the
  mandated `make fmt && make clippy && make run && make test` block from
  `AGENTS.md §Development Standards`. Existing targeted evidence
  (`cargo test --workspace`, `make linux`, `make debian`, specific unit
  tests, benchmark CSVs) remains as additional evidence, not replacement.

All changes stay structural; no benchmark-targeted constants, no workload
switches. The aggregate combined thresholds (dhry ≥ 20 %, cm ≥ 20 %, mb
≥ 15 %) from 01_PLAN are retained, and per-phase thresholds from
`docs/PERF_DEV.md §3` are now binding inside the bundle.

## Log

[**Feature Introduce**]

- **R-001 collapse.** The invalidation machinery is gone. No `ctx_tag`
  field on `ICacheLine`, no `icache_ctx_tag` field on `RVCore`, no CSR
  or trap-path hooks, no `checked_write` flush. The `(pc, raw)`
  comparison is the sole correctness rule; SMC is handled as a natural
  consequence of reading fresh bytes on re-fetch.
- **Phase 5 collapse.** Round-01 had six invalidation-install substeps
  (5.1 satp, 5.2 sfence.vma, 5.3 fence.i, 5.4 privilege debounce, 5.5
  mstatus bit-isolation, 5.6 RAM-store SMC). Round 02 replaces all of
  them with a single sentence: "no invalidation hooks are installed;
  `fence.i` reverts to its pre-P1 NOP per `inst/privileged.rs:71-79`".
- **Binding per-phase gates.** Exit Gate §A lists P3/P4/P5/P6 gates;
  Exit Gate §B lists combined bundle gates. Any §A miss splits that
  phase out of this round before it is declared landed.
- **P5 trap-slim removed.** Option A from R-003 — no extraction of
  `has_pending_trap()`, no re-work of `retire`. P5 keeps only
  `#[inline]`/`#[inline(always)]` audit pressure on the MMU fast path.
- **Repo-mandated command block.** Added verbatim to §Validation and
  §Exit Gate.
- **SMC am-test demoted.** Not a gate. A Rust unit test still pins the
  behaviour at the cache level; the C am-test is noted only as
  nice-to-have future work.

[**Review Adjustments**]

- **R-001** (HIGH, Option 1 from recommendation): Decoded-raw cache.
  See §Architecture §P4, §Data Structure, §Invariants I-11/I-12.
- **R-002** (HIGH): Per-phase binding gates. See §Exit Gate §A / §B.
- **R-003** (MEDIUM, Option A): P5 trap-slim dropped. See
  §Architecture §P5, §Implementation Steps §3.
- **R-004** (MEDIUM): `make fmt && make clippy && make run && make test`
  verbatim in §Validation Strategy preamble and §Exit Gate header.
- **TR-1** (adopt): Decoded-raw cache adopted; see §Trade-offs T-1.
- **TR-2** (adopt): Bundle + binding per-phase gates; see §Trade-offs T-2.

[**Master Compliance**]

- **M-001 (combined scope).** P3 + P4 + P5 + P6 all land in this one
  iteration. See §Spec Architecture subsections and §Implementation
  Plan phases 1-5.
- **M-002 (path rename).** All references use `docs/perf/hotPath/` and
  `docs/perf/busFastPath/`. No `docs/fix/` references remain. No action
  needed in round 02 beyond path discipline.
- **M-003 (clean layout).** Single Summary, single Log, single
  Response Matrix, single Spec (per-phase subsections only where design
  actually diverges), single Trade-offs, single Validation, single
  Exit Gate. The R-001 collapse makes the layout substantially leaner
  than round 01.

### Changes from Previous Round

[**Added**]

- §Architecture §P4 rewritten around `(pc, raw)` key.
- I-12: cache miss on `line.pc != pc || line.raw != raw`. Sole
  correctness rule for P4.
- Exit Gate §A (per-phase binding) and §B (combined bundle).
- Mandatory command block `make fmt && make clippy && make run &&
  make test` in §Validation Strategy and §Exit Gate.
- V-UT-3 reframed as "SMC Rust unit test at the cache level — write
  new bytes at the same PC through `Bus::store`, step, assert the
  re-decoded instruction executes" (optional belt-and-braces, not a
  gate).
- §Trade-offs T-3 documenting why R-001 collapse is correct in this
  codebase and why the same collapse would be wrong in a QEMU-style
  cache that memoises translated basic blocks.

[**Changed**]

- `ICacheLine`: three fields instead of four (dropped `ctx_tag`).
- `RVCore`: one new field instead of two (dropped `icache_ctx_tag`).
- §Invariants: I-11 becomes "icache geometry: per-hart, direct-mapped,
  4096 lines, index `(pc >> 1) & MASK`". I-12 becomes the raw-mismatch
  rule.
- §Execution Flow Main Flow step 4 compares `(pc, raw)` only; step 6
  writes `{pc, raw, decoded}` only.
- §Implementation Plan phase count: 5 (previously 9). R-001 collapse
  absorbed the invalidation-install phase; R-003 Option A removed the
  trap-slim subphase.
- §Trade-offs T-4/T-5/T-6 from round 01 (mstatus bit-isolation,
  trap-entry debounce, unsafe ptr fallback) are no longer relevant;
  replaced with T-3 (why the decoded-raw collapse is sound) and
  retained T-1/T-2.

[**Removed**]

- `ctx_tag: u64` field on `ICacheLine`.
- `icache_ctx_tag: u64` field on `RVCore`.
- `invalidate_icache()` method.
- All six invalidation-install hooks (satp / sfence.vma / fence.i /
  privilege-change / mstatus-bit-isolation / RAM-store).
- Round-01 invariants I-13 (fence.i local-hart), I-14 (mstatus bit
  isolation), I-15 (privilege debounce). Obsolete under R-001.
- Round-01 V-UT-2 (ctx_tag bumps on events), V-UT-4/5 (mstatus flips),
  V-UT-6 (M→M debounce), V-UT-7 (MPRV trigger). Obsolete under R-001.
- V-IT-1 (am-test `make m` as a gate). Demoted to future work.
- V-F-4 (MPRV flip integration), V-F-6 (PMP ctx_tag non-bump).
  Obsolete.
- Round-01 T-4/T-5/T-6 trade-off rows.
- P5 trap-slim sub-phase (R-003 Option A).

[**Unresolved**]

- If rustc 1.96 + LTO does not already inline `checked_read` →
  `access_bus` → `Bus::read` end-to-end, P5's 3 pp MMU-bucket target
  may be unreachable with pure `#[inline]` pressure. The plan's
  fallback is to capture the `cargo asm` transcript and, if the audit
  shows an unavoidable dynamic dispatch or trait object, raise the
  issue for round 03 rather than silently add `unsafe` shortcuts.
- Linux-boot icache hit rate is measured opportunistically under
  `--features perf-stats` and recorded in the benchmark artefact, but
  not gated. Pinned only on dhry/cm/mb (G-4 / Exit Gate §A P4).
- A C am-test for SMC is nice-to-have future work. With `(pc, raw)`
  keying, SMC is not a distinct code path, so no gate depends on it.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Master `00_MASTER.md` | M-001 | Applied | Round 02 still covers P3 + P4 + P5 + P6 in one iteration. See §Spec Architecture §P3..§P6, §Implementation Plan phases 1-5, §Exit Gate §B. |
| Master `00_MASTER.md` | M-002 | Applied | All paths use `docs/perf/hotPath/` and `docs/perf/busFastPath/`. No action needed this round. |
| Master `00_MASTER.md` | M-003 | Applied | One Summary, one Log, one Response Matrix, one Spec, one Trade-offs, one Validation, one Exit Gate. R-001 collapse makes the layout leaner. |
| Master `01_MASTER.md` | — | N/A | `01_MASTER.md` is blank; no new directives. The three `00_MASTER.md` directives remain in force. |
| Round-00 Review | R-001 | Superseded | Round-01 adopted Option B (`checked_write` SMC flush). Round 02 deletes that hook entirely under the new round-01 R-001 — `(pc, raw)` keying handles SMC without any store-path hook. |
| Round-00 Review | R-002 | Superseded | Round-01 wired `xkernels/tests/am-tests/src/tests/smc.c` + Rust integration test. Round 02 demotes that am-test to future work: `(pc, raw)` keying makes SMC a natural cache miss, so no gate depends on a dedicated SMC harness. A Rust unit test (V-UT-3) stays as belt-and-braces coverage. |
| Round-00 Review | R-003 | Obsolete | Round-01's `mstatus` bit-isolation hook is deleted in round 02 per R-001; `mstatus` writes do not affect decode, so no bump is needed. |
| Round-00 Review | R-004 | Obsolete | Round-01's privilege-debounced trap-entry hook is deleted in round 02 per R-001; privilege transitions do not affect decode of `raw`. |
| Round-00 Review | R-005 | Retained | `(pc >> 1) & MASK` index kept; aliasing documented under I-11 with the full-`pc` tag acting as the disambiguator. XOR-fold deferred contingent on Linux hit-rate telemetry under `perf-stats`. |
| Round-00 Review | R-006 | Obsolete | MPRV/alternate-mapping torture test targeted the (now deleted) `mstatus` bit-isolation hook. MPRV affects load/store (`cpu/mm.rs:228-235, 261-264`), not fetch, so the test is not meaningful for the decoded-raw cache. |
| Round-00 Review | R-007 | Obsolete | `ctx_tag` widening from u32→u64 is moot — the field does not exist in round 02. |
| Round-00 Review | R-008 | Retained | `perf-stats` Cargo feature in `xemu/xcore/Cargo.toml`, off by default, documents hit-rate telemetry. Unchanged. |
| Round-00 Review | R-009 | Retained | Response Matrix populated per template; every prior finding plus every MASTER directive appears below. |
| Round-01 Review | R-001 | Accepted (HIGH) | Decoded-raw cache, key `(pc, raw)`, zero invalidation hooks. See §Architecture §P4, §Invariants I-12, §Data Structure. |
| Round-01 Review | R-002 | Accepted (HIGH) | Exit Gate split into binding per-phase gates (§A) + combined bundle gates (§B). Policy: any §A miss splits that phase from this round. See §Exit Gate. |
| Round-01 Review | R-003 | Accepted (MEDIUM, Option A) | P5 trap-slim dropped. `retire` at `cpu.rs:152-159` is already the steady-state zero-pending branch; `commit_trap` is only entered after `pending_trap.take()` returns `Some`. P5 focuses on MMU fast path only. See §Architecture §P5, §Implementation Plan phase 3. |
| Round-01 Review | R-004 | Accepted (MEDIUM) | `make fmt && make clippy && make run && make test` is verbatim in §Validation Strategy preamble and §Exit Gate header, as a binding gate. |
| Round-01 Review | TR-1 | Adopted | Simpler decoded-raw cache. Code-level proof: `Decoder::decode(raw)` is pure (`isa/riscv/decoder.rs:130-143`), tables are static at program start (`decoder.rs:206-260`), fetch resolves translation before decode (`cpu/mm.rs:306-315`, `cpu.rs:238-245`). See §Trade-offs T-1. |
| Round-01 Review | TR-2 | Adopted | Bundle + binding per-phase gates. See §Trade-offs T-2, §Exit Gate. |

> Rules:
> - Every prior HIGH / CRITICAL finding appears above.
> - Every MASTER directive appears above.
> - Rejections include explicit reasoning (obsoleted items cite the R-001 collapse).

---

## Spec

[**Goals**]

- G-1 (P3): Cached-deadline short-circuit in `Mtimer::tick` so the
  default path is one `u64` compare + return; combined `Mtimer::*`
  self-time < 1 % on all three benchmarks.
- G-2 (P4): Eliminate the pest tree walk on every cache-hit fetch; the
  fast path becomes one `(pc, raw)` compare + one POD copy of
  `DecodedInst`. Hit rate ≥ 95 % on dhry/cm/mb.
- G-3 (P5): Inline the TLB-hit MMU fast path end-to-end through
  `checked_read`/`checked_write`/`access_bus`; MMU bucket drops ≥ 3 pp.
- G-4 (P6): Bypass `_platform_memmove` for aligned 1/2/4/8-byte RAM
  accesses via typed primitive reads/writes;
  `_platform_memmove + memcpy` bucket < 2 %; `Bus::read + Bus::write`
  combined drops ≥ 3 pp.
- G-5 (combined): Wall-clock reduction on the 2026-04-15 baseline —
  dhry ≥ 20 %, cm ≥ 20 %, mb ≥ 15 %.
- G-6 (combined): `xdb::main` self-time share drops by ≥ 10 pp on all
  three workloads.
- G-7 (correctness): All existing `make test` / `make run` outcomes
  unchanged. Linux / Linux-2hart / Debian boot to prompt.
- G-8 (mutex-free): `bash scripts/ci/verify_no_mutex.sh` stays `ok`.

- NG-1: No JIT, no trace chaining, no threaded dispatch.
- NG-2: No paddr-tagged SMC bitmap. Not needed under `(pc, raw)` keying.
- NG-3: No replacement for `pest`. Decode-miss fallback is the same
  `DECODER.decode(raw)` path.
- NG-4: No benchmark-specific code paths of any kind.
- NG-5: No assembly-file modifications.
- NG-6: No new `Arc<Mutex<_>>`, `RwLock<_>`, `RefCell<_>`, or
  `Box<dyn FnMut>` on `Bus` or `RVCore`. `verify_no_mutex.sh` must stay
  `ok`.
- NG-7: No multi-thread SMP work (Phase 11 RFC territory).
- NG-8: No invalidation hooks on satp / sfence.vma / fence.i /
  privilege transitions / mstatus. Explicitly out per R-001.
- NG-9: No extraction of `has_pending_trap` / rework of `retire`.
  Explicitly out per R-003 Option A.

[**Architecture**]

Top-level shape (unchanged from 01 except for the P4 inner detail):

```
 RVCore::step  (xcore/src/arch/riscv/cpu.rs:238-245)
   |
   |  fetch(bus) -> raw:u32                 [P5: ensure fully inlined]
   v
 decode_cached(raw) -> DecodedInst          [P4: (pc, raw) key only]
   |   hit : one tag compare + one POD copy
   |   miss: pest decode, line overwrite
   v
 dispatch(bus, decoded)                     [unchanged]
   |
   v
 Bus::tick()                                [P3: Mtimer deadline-gate]
   \__ Mtimer::tick : if mtime < next_fire_mtime { return; }
   \__ other devices unchanged
```

### Architecture §P3 — Mtimer deadline gate (unchanged from round 01)

`Mtimer` (`xcore/src/arch/riscv/device/aclint/mtimer.rs`) grows one
field `next_fire_mtime: u64`, initialised to `u64::MAX`. `tick()`:

```rust
fn tick(&mut self) {
    // existing epoch-init + SYNC_INTERVAL logic unchanged
    if self.mtime < self.next_fire_mtime { return; }
    self.check_all();
}
```

`next_fire_mtime` is recomputed as
`self.mtimecmp.iter().copied().min().unwrap_or(u64::MAX)`:

- on every `write` that hits a `Mtimecmp` register (`mtimer.rs:95-109`),
- at the end of `check_timer` (re-arm after a just-fired deadline),
- on `reset()` (back to `u64::MAX`, `mtimer.rs:129-139`).

Precedent: QEMU's `timer_next_deadline` pattern in `hw/intc/aclint.c`.

### Architecture §P4 — Decoded-instruction cache (R-001 rewritten)

Per-hart direct-mapped 4096-line cache. One new file
`xcore/src/arch/riscv/cpu/icache.rs` plus **one** new field on `RVCore`:

```
                        ┌───────────────────────────────────┐
                        │ RVCore.step                       │
                        └────────────────┬──────────────────┘
               fetch(bus) -> raw:u32     │
                        ┌────────────────▼──────────────────┐
                        │ idx  = ICache::index(self.pc)     │
                        │ line = &mut self.icache.lines[idx]│
                        │ hit  = line.pc  == self.pc        │
                        │     && line.raw == raw            │
                        └──────────────┬────────────────────┘
                                 hit   │   miss
                          decoded = line.decoded
                                       │    decoded = DECODER.decode(raw)?
                                       │    *line = ICacheLine { pc, raw, decoded }
                                       ▼
                          dispatch(bus, decoded)
```

No `ctx_tag`. No invalidation hooks. No CSR / trap-path edits.

**Why `(pc, raw)` is sufficient** (code-level proof cited from R-001 /
TR-1 review advice):

- `RVCore::step` at `cpu.rs:238-245` calls `fetch(bus)` (which resolves
  translation + PMP + permissions and returns the raw 32/16-bit word),
  **then** calls `decode(raw)`.
- `Decoder::decode(raw)` at `isa/riscv/decoder.rs:130-143` is pure:
  `DecodedInst::from_raw(format, inst, kind)` reads the raw bits plus
  static dispatch tables built at program start
  (`decoder.rs:206-260`).
- Privilege mode, MPRV, SUM, MXR, ASID, and satp do not participate in
  decode. MPRV specifically affects loads/stores only
  (`cpu/mm.rs:228-235, 261-264`).
- When the guest writes new instruction bytes via `RVCore::store`, the
  next fetch reads a different `raw`, so the `(pc, raw)` comparison
  misses and the line is overwritten. Self-modifying code correctness
  emerges from the cache key, not from a flush hook.

Shape mirrors the **decode memoisation** families (NEMU IBuf, rvemu's
absence of such a cache — see §Trade-offs T-3) rather than QEMU's
translation-block cache, which memoises translated basic blocks and
therefore does need translation-context invalidation.

### Architecture §P5 — MMU fast-path inline (R-003 Option A)

Target files: `xcore/src/arch/riscv/cpu/mm.rs` (`access_bus`,
`checked_read`, `checked_write`, `translate`, `fetch`, `load`,
`store`).

No algorithmic change. Audit under
`cargo asm --release --features perf-stats` that the TLB-hit path in
`checked_read` / `checked_write` is inlined through to `Bus::read` /
`Bus::write`. Add `#[inline]` / `#[inline(always)]` where LTO alone
does not fold.

**Trap slim DROPPED** (R-003 Option A). The steady-state zero-pending
branch already lives in `RVCore::retire` at
`xemu/xcore/src/arch/riscv/cpu.rs:152-159`:

```rust
fn retire(&mut self) {
    if let Some(trap) = self.pending_trap.take() {
        self.commit_trap(trap);
    } else {
        self.csr.increment_instret();
    }
    self.pc = self.npc;
    self.csr.increment_cycle();
}
```

This compiles to a single `Option::take` discriminant check. Extracting
`has_pending_trap` out of `commit_trap` (the round-01 plan) would not
change the fast path because `commit_trap` is only entered on the
Some branch. No re-work of `retire` happens in round 02. If a future
profile surfaces a concrete hot branch elsewhere
(`check_pending_interrupts`, `mip`/`stimecmp` sync), it is scoped to a
follow-up iteration.

### Architecture §P6 — memmove typed-read bypass (unchanged from round 01)

`Ram::read` / `Ram::write` (`xemu/xcore/src/device/ram.rs`) gain a
size + alignment pre-check. For aligned 1 / 2 / 4 / 8-byte accesses:

```rust
match size {
    1 => Ok(bytes[off] as Word),
    2 => Ok(u16::from_le_bytes(bytes[off..off + 2].try_into()?) as Word),
    4 => Ok(u32::from_le_bytes(bytes[off..off + 4].try_into()?) as Word),
    8 => Ok(u64::from_le_bytes(bytes[off..off + 8].try_into()?) as Word),
    _ => /* existing generic memmove path */,
}
```

`from_le_bytes` + `try_into` on a fixed-size slice compiles to a single
aligned load on modern rustc; no `unsafe` needed. If the `cargo asm`
audit shows this fails to fold under the currently pinned toolchain,
fall back to `ptr::read_unaligned` with a mandatory `// SAFETY:`
comment covering alignment / in-bounds / no-aliasing. The safe path
remains preferred.

[**Invariants**]

- I-1: `RVCore::step` always calls `fetch` before any icache lookup.
- I-2: `pest` (via `DECODER.decode`) is the sole decode authority; the
  cache only memoises its result, never its inputs.
- I-3: The icache is owned per-hart; no shared state, no atomics, no
  locks (inherited M-001 sentinel from P1).
- I-4: On miss, a line is overwritten in full (`pc`, `raw`, `decoded`)
  — partial updates are forbidden.
- I-5: Decode failure (`XError::InvalidInst`) does not write a line; a
  transient illegal word does not poison subsequent legal re-fetches.
- I-6: Compressed instructions: `raw` carries the fetched word as-is
  (16-bit zero-extended to `u32`), matching `fetch` at
  `mm.rs:306-315`.
- I-7: A store of new instruction bytes by the guest (SMC) is handled
  implicitly: the next fetch reads a different `raw`, `(pc, raw)`
  misses, the line is overwritten. No store-path hook.
- I-8: Stores to MMIO regions do not participate in any icache logic
  (no hook exists).
- I-9: `fence.i` remains a NOP (`inst/privileged.rs:71-79`), as it was
  before the round-01 proposal. Correct under I-7 because the fetched
  bytes on the *next* step already reflect any store from the *previous*
  step; `fence.i` serialises ordering, which the step loop already
  provides.
- I-10 (inherited from P1): `CPU::step` destructures `self` into
  disjoint `bus` + `cores[i]` borrows; no `Mutex<Bus>`.
  `verify_no_mutex.sh` regex-scan passes.
- I-11: Icache geometry — per-hart, direct-mapped, 4096 lines, index
  `(pc >> 1) & MASK` (R-005 carry-forward). Aliasing: RVI at `pc` and
  RVI at `pc + 2 * LINES = 0x2000` land in the same slot; the full-`pc`
  tag catches the alias and forces miss-and-replace. Load factor on
  dhry/cm/mb stays below 40 % in the baseline; V-IT-4 hit rate ≥ 95 %
  is the observable gate. XOR-fold is deferred contingent on Linux
  hit-rate telemetry.
- I-12: Cache miss iff `line.pc != self.pc || line.raw != raw`. This
  is the sole correctness rule for P4 (R-001).
- I-13 (removed in round 02 per R-001). Note retained: no per-hart
  context tag exists; `fence.i` does nothing; no invalidation needed.
- I-14 (removed in round 02 per R-001). Note retained: `mstatus`
  writes do not affect decode; no hook.
- I-15 (removed in round 02 per R-001). Note retained: privilege
  transitions do not affect decode; no hook.
- I-16 (P3): `next_fire_mtime` is the running minimum over
  `self.mtimecmp[*]`. Recomputed on every `mtimecmp` write, at the end
  of `check_timer`, and on `reset`. Initialised `u64::MAX`.
- I-17 (P6): The typed-read bypass path is taken iff the region is RAM
  AND `size ∈ {1, 2, 4, 8}` AND `addr % size == 0`. All other cases
  fall through to the existing memmove path, preserving every MMIO
  semantic (side effects, unaligned behaviour, arbitrary sizes).

[**Data Structure**]

```rust
// xemu/xcore/src/arch/riscv/cpu/icache.rs (new)
use crate::isa::riscv::decoder::DecodedInst;
use memory_addr::VirtAddr;

pub const ICACHE_BITS: usize = 12;
pub const ICACHE_LINES: usize = 1 << ICACHE_BITS;
pub const ICACHE_MASK: usize = ICACHE_LINES - 1;

/// R-001: no `ctx_tag`. Line shape is `(pc, raw, decoded)` only.
#[derive(Clone, Copy)]
pub struct ICacheLine {
    pub pc:      VirtAddr,
    pub raw:     u32,
    pub decoded: DecodedInst,
}

impl ICacheLine {
    /// Sentinel. A freshly initialised line cannot collide with a
    /// legitimate fetch because `raw == 0` is illegal encoding; the
    /// first fetch at any PC misses, re-decodes, and overwrites.
    pub const INVALID: Self = Self {
        pc:      VirtAddr::from_usize(0),
        raw:     0,
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

// RVCore (xemu/xcore/src/arch/riscv/cpu.rs) gains ONE field:
pub struct RVCore {
    // ... existing fields unchanged ...
    pub(in crate::arch::riscv) icache: Box<ICache>,
}

// Mtimer (xemu/xcore/src/arch/riscv/device/aclint/mtimer.rs) gains ONE field:
pub(super) struct Mtimer {
    // ... existing fields unchanged ...
    next_fire_mtime: u64, // P3: min(mtimecmp[*]); starts u64::MAX
}
```

`DecodedInst` (in `xemu/xcore/src/isa/riscv/decoder.rs:161`) gains
`Copy` to its existing `Clone, PartialEq, Eq` derive; V-UT-1 pins that
with `fn _assert_copy<T: Copy>() {}`.

[**API Surface**]

```rust
// In ICache
impl ICache {
    pub fn new() -> Box<Self>;
    #[inline] pub fn index(pc: VirtAddr) -> usize;
}

// In RVCore
impl RVCore {
    /// Cache-aware decode replacing `DECODER.decode(raw)` in step().
    /// R-001: no context tag, no invalidation hooks; the `(pc, raw)`
    /// comparison is the sole correctness rule.
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

// In Ram (xemu/xcore/src/device/ram.rs) — typed fast path inside the
// existing Device::read / Device::write impls; no public API change.
```

No new public API on the `RVCore` / `Bus` / `Mtimer` facades. All
additions are `pub(in crate::arch::riscv)` or module-private.

[**Constraints**]

- C-1: No benchmark-targeted code. Reviewer must reject the plan if any
  constant, branch, or special case keys off workload identity.
- C-2: `make fmt && make clippy && make run && make test` must pass
  after every implementation phase (binding per AGENTS.md
  §Development Standards; R-004).
- C-3: Benchmarks run with `DEBUG=n`.
- C-4: Workloads launched via `make run` / `make linux` / `make debian`
  per AGENTS.md; never by hand-calling `target/release/xdb`.
- C-5: No assembly-file modifications.
- C-6: `bash scripts/ci/verify_no_mutex.sh` must remain `ok` — no
  `Mutex`/`RwLock` regression (inherited from P1).
- C-7: Scope ends at P3 + P4 + P5 + P6. No P1/P2 rework, no Phase-11
  multi-thread work, no VGA/framebuffer, no allocator tuning.
- C-8: `perf-stats` Cargo feature is off by default. Release binaries
  ship without it. Exit-gate measurements cite
  `cargo build --release --features perf-stats` explicitly.
- C-9: No new `unsafe` in the P6 path if `from_le_bytes` + `try_into`
  lowers to a single aligned load (verified under `cargo asm`). If a
  `ptr::read_unaligned` fallback is needed, a `// SAFETY:` comment
  covering alignment / in-bounds / no-aliasing is mandatory.
- C-10: No invalidation hooks on CSR writes, `fence.i`, `sfence.vma`,
  `satp`, trap entries, or RAM stores (NG-8 cross-reference).

---

## Implement

### Execution Flow

[**Main Flow**]

1. `RVCore::step` calls `self.fetch(bus)?` → `raw: u32` (unchanged).
2. `idx = ICache::index(self.pc)` (new).
3. Read `let line = &mut self.icache.lines[idx]`.
4. Check `hit = line.pc == self.pc && line.raw == raw`.
5. Hit: `decoded = line.decoded` (POD copy).
6. Miss: `decoded = DECODER.decode(raw)?`;
   `*line = ICacheLine { pc: self.pc, raw, decoded }`.
7. `self.execute(bus, decoded)?` (unchanged).
8. `Bus::tick()` → `Mtimer::tick()` executes the P3 deadline
   short-circuit before any per-hart work.
9. Guest load/store through `RVCore::checked_read` / `checked_write`
   uses the P5-inlined TLB-hit path; RAM accesses in `Bus::read/write`
   follow the P6 typed-read fast path when size and alignment permit.

[**Failure Flow**]

1. `fetch` traps → return `Err(...)` before any icache access. Cache
   unchanged.
2. `DECODER.decode(raw)?` errors on miss → return before writing the
   line. Cache unchanged. (I-5.)
3. `execute` errors after a cache-hit decode → line kept (decode
   succeeded); next fetch at same PC re-hits and traps identically.
4. SMC: guest writes new bytes via `Bus::store`; on the next fetch,
   `raw` differs; `(pc, raw)` misses; line is overwritten; new
   instruction executes. No hook needed.
5. `checked_write` traps (PMP / alignment / page-fault): error
   propagates; nothing in the icache changes (no hook exists).
6. P3: `mtime < next_fire_mtime` → `tick` returns before `check_all`;
   no irq change (correct because no hart's `mtimecmp` was reached).
7. P6: MMIO / unaligned / odd-size access → pre-check fails → falls
   through to existing memmove path with identical semantics.

[**State Transition**]

- `icache line (pc, raw match) → hit` → POD copy of `decoded`.
- `icache line (pc or raw mismatch) → miss` → line replaced.
- `icache line (decode fails) → unchanged`, error propagates.
- `mtimecmp write → next_fire_mtime recomputed` (P3).
- `mtime ≥ next_fire_mtime → check_all runs`; else early return (P3).
- `RAM read/write size ∈ {1,2,4,8} ∧ aligned → typed path`; else
  memmove (P6).
- `MMU TLB hit → inlined load path`; `TLB miss → existing translate
  slow path` (P5).

### Implementation Plan

Each phase must pass `make fmt && make clippy && make run && make test`
plus `bash scripts/ci/verify_no_mutex.sh` before the next phase begins
(binding per C-2 / R-004).

[**Phase 1 — P3 Mtimer deadline gate**]

1. Add `next_fire_mtime: u64` to `Mtimer` (`mtimer.rs:26-33`), init
   `u64::MAX`.
2. Add `recompute_next_fire` helper (§API Surface).
3. Modify `tick` (`mtimer.rs:112-123`): after the existing
   `SYNC_INTERVAL` sync, short-circuit on
   `self.mtime < self.next_fire_mtime`.
4. Call `recompute_next_fire` in `write` after every `Mtimecmp`
   mutation (`mtimer.rs:95-108`), at the end of `check_timer`, and at
   the end of `reset` (`mtimer.rs:129-139`).
5. Unit test V-UT-8 + V-F-5 land in the same commit.

[**Phase 2 — P4 ICache struct + DecodedInst Copy + wiring**]

1. New file `xemu/xcore/src/arch/riscv/cpu/icache.rs` implementing
   `ICache`, `ICacheLine`, `ICACHE_BITS = 12` (§Data Structure).
2. Derive `Copy` on `DecodedInst` in `isa/riscv/decoder.rs:161`.
3. V-UT-1: `fn _assert_copy<T: Copy>() {}` against `DecodedInst`.
4. Add `icache: Box<ICache>` to `RVCore` (`cpu.rs:36-54`); initialise
   in `RVCore::new` via `ICache::new()`.
5. Add `decode_cached` (§API Surface).
6. Replace `self.decode(raw)?` in `cpu.rs:238-245` with
   `self.decode_cached(raw)?`.
7. Cargo feature `perf-stats` in `xemu/xcore/Cargo.toml`, off by
   default; behind it, add hit/miss counters on `ICache` with `&mut`
   increment on each lookup and a dump-at-exit hook.
8. V-UT-2 (cache miss when raw changes at same pc) + V-UT-3 (SMC unit
   test — write new bytes via `Bus::store`, step, assert re-decoded
   instruction executes) land in the same commit.

[**Phase 3 — P5 MMU fast-path inline**]

1. Audit `checked_read` / `checked_write` / `access_bus` / `Bus::read`
   / `Bus::write` / `Ram::read` / `Ram::write` call chain with
   `cargo asm --release --features perf-stats` to confirm TLB-hit
   inlining end-to-end.
2. Add `#[inline]` / `#[inline(always)]` where LTO does not already
   fold. No algorithmic change.
3. Capture the `cargo asm` transcript for V-IT-5 evidence. Commit it
   alongside the source change under `docs/perf/<post-hotPath-date>/`.
4. **Trap slim DROPPED** (R-003 Option A). No `has_pending_trap`
   extraction; no rework of `retire`.

[**Phase 4 — P6 memmove typed-read bypass**]

1. In `Ram::read` (`xemu/xcore/src/device/ram.rs`), add size-match on
   1/2/4/8 with alignment check; use
   `u{8,16,32,64}::from_le_bytes(slice[..].try_into()?)`.
2. In `Ram::write`, mirror with `to_le_bytes`.
3. MMIO and unaligned/odd-size paths untouched.
4. If `cargo asm` shows `from_le_bytes` does not fold to a native load
   under the pinned rustc, fall back to `ptr::read_unaligned` with
   explicit `// SAFETY:` comment; document the choice in the commit
   message.
5. V-UT-9 + V-E-5 + V-E-6 land in the same commit.

[**Phase 5 — Benchmark capture + final gate**]

1. `bash scripts/perf/bench.sh --out docs/perf/<post-hotPath-date>`
   (3 iters × 3 workloads) with `DEBUG=n`.
2. `bash scripts/perf/sample.sh --out docs/perf/<post-hotPath-date>`.
3. `python3 scripts/perf/render.py
       --dir docs/perf/<post-hotPath-date>`.
4. Capture hit rate via
   `cargo build --release --features perf-stats && make run`
   (stats dumped at exit).
5. Diff `data/bench.csv` vs.
   `docs/perf/2026-04-15/data/bench.csv`; compare self-time bucket
   tables against §Exit Gate §A/§B thresholds.
6. Run the mandated command block:
   ```
   make fmt
   make clippy
   make run
   make test
   ```
   and `bash scripts/ci/verify_no_mutex.sh`.

## Trade-offs

- **T-1: Decoded-raw cache vs. translation-context cache.**
  - *Decoded-raw (Option A, chosen per TR-1 + R-001)*: key `(pc, raw)`;
    zero invalidation hooks; SMC handled implicitly by `raw` mismatch.
    Proof: `Decoder::decode(raw)` is pure
    (`isa/riscv/decoder.rs:130-143`), decode tables are static
    (`decoder.rs:206-260`), fetch resolves translation before decode
    (`cpu/mm.rs:306-315`, `cpu.rs:238-245`), MPRV affects loads/stores
    only (`cpu/mm.rs:228-235, 261-264`).
  - *Translation-context (Option B, rejected)*: key `(pc, ctx_tag, raw)`
    with hooks on satp / sfence.vma / fence.i / privilege-change /
    mstatus / RAM-store. Fights G-2 because `checked_write` hook
    flushes on ordinary stack / data traffic; adds CSR and trap-path
    branches that buy no correctness benefit in this codebase.
  - *Chosen*: A. The simpler design is correct here because the cache
    memoises decode, not translation.
- **T-2: Bundled round vs. four separate rounds.**
  - *Bundled (chosen per M-001, M-003, TR-2)*: one branch, one commit
    target, one benchmark capture, one `verify_no_mutex` gate.
  - *Separate*: finer git history, per-phase rollback, 4× benchmark
    capture, 4× review cycles.
  - *Chosen*: bundled **with binding per-phase gates** (Exit Gate §A).
    Per-phase accountability preserved inside the shared artefact. If
    any §A gate misses, that phase splits to a follow-up round before
    this bundle is declared landed.
- **T-3: Why the R-001 collapse is correct in this codebase.**
  - QEMU's TCG translation-block cache memoises the *translated* basic
    block and therefore must invalidate on translation-context changes
    (satp, ASID). That is the family whose invalidation lattice the
    round-01 plan imitated.
  - xemu's cache memoises the *decode* result only; the translated /
    fetched raw word is produced by `fetch` before `decode` and already
    carries any translation effect into `raw`. Any guest action that
    changes what decoding produces at a given PC — a page remap to
    different bytes, a store of new bytes, an ASID switch that changes
    the underlying physical page — lands as a different `raw` on the
    next fetch. The `(pc, raw)` comparison subsumes the translation
    lattice.
  - NEMU IBuf and rvemu both operate at this layer: NEMU caches decode,
    rvemu does no decode-level caching at all. Neither reports
    correctness bugs stemming from translation changes because neither
    needs translation-context invalidation for a decode cache.
- **T-4: P5 trap slim — keep or drop (R-003).**
  - *Keep (rejected)*: extract `has_pending_trap()` out of
    `commit_trap`. Does not change `retire`'s steady-state branch
    because `commit_trap` is only entered on the Some path.
  - *Drop (chosen, R-003 Option A)*: P5 focuses on MMU fast path only.
    If a future profile surfaces a real hot trap branch, it goes into
    a targeted follow-up iteration.
- **T-5: P6 typed-read — safe vs. unsafe.**
  - Safe-first (chosen). `from_le_bytes` + `try_into` on a fixed-size
    slice is typically zero-cost. If the assembly audit shows
    otherwise, fall back to `unsafe ptr::read_unaligned` with a
    `// SAFETY:` comment and retain the safe path behind
    `#[cfg(not(feature = "typed-read-unsafe"))]`.
- **T-6: P3 `next_fire_mtime` recompute — on-write vs. on-fire.**
  - Chosen both: recompute on every `mtimecmp` write and at the end of
    every `check_timer`. Recomputing lazily inside `tick` before the
    compare defeats the whole point of the gate.
- **T-7: Threaded dispatch (out of scope).**
  - Ertl & Gregg 2003 document 20–50 % speedup from direct-threaded
    over switch-dispatch; this is a future phase per PERF_DEV §7, not
    hotPath. Cited for context only.

Sources:

- QEMU TB cache maintenance:
  https://github.com/qemu/qemu/blob/master/accel/tcg/tb-maint.c
- QEMU per-CPU jump cache:
  https://github.com/qemu/qemu/blob/master/include/exec/tb-jmp-cache.h
- QEMU QHT aliasing analysis: https://lwn.net/Articles/697265/
- NEMU IBuf: https://github.com/OpenXiangShan/NEMU
- rv8 CARRV 2017 (JIT trace cache, different model, cited for
  contrast): https://carrv.github.io/2017/papers/clark-rv8-carrv2017.pdf
- rvemu (baseline, no cache): https://github.com/d0iasm/rvemu
- RISC-V Zifencei §5.1 (fence.i is local-hart only; moot now):
  https://github.com/riscv/riscv-isa-manual/releases
- RISC-V Privileged §3.1.6.3 (MPRV applies to loads/stores, not
  instruction fetch): same release index
- Ertl & Gregg 2003, direct-threaded interpreters:
  https://www.complang.tuwien.ac.at/forth/threaded-code.html
- `docs/PERF_DEV.md:186-190` (per-phase gates authoritative).

## Validation

**Mandatory repo verification per AGENTS.md §Development Standards
(R-004)**, run after every implementation phase and once more at the
final Exit Gate:

```
make fmt      # rustfmt clean
make clippy   # no new warnings
make run      # each benchmark: dhrystone / coremark / microbench
make test     # cargo test --workspace + am-tests
```

Additional targeted evidence below (specific Rust unit tests, boot
smoke tests, benchmark CSVs) supplements — does not replace — the
block above.

[**Unit Tests**]

- V-UT-1: `decoded_inst_is_copy` in `isa/riscv/decoder.rs::tests` —
  static `_assert_copy<DecodedInst>()`. Pins the POD property required
  by the cache.
- V-UT-2: `icache_miss_when_raw_changes_at_same_pc` in
  `cpu/icache.rs::tests` — build a two-hart-capable cache fixture,
  store decoded line A at `(pc, raw_a)`, lookup `(pc, raw_b)` with
  `raw_b != raw_a`; assert miss. Pins I-12.
- V-UT-3: `smc_rust_unit` in `cpu/icache.rs::tests` or
  `inst/privileged.rs::tests` — drive `step → store new bytes → step`
  through `RVCore` and assert the second step executes the newly
  written instruction (not the cached one). Belt-and-braces coverage;
  not a gate under the new model but useful regression.
- V-UT-8: `mtimer_deadline_short_circuits` in
  `device/aclint/mtimer.rs::tests` — set `mtimecmp[0] = u64::MAX`;
  tick 1000 times; assert `check_all` was not called (via a test-only
  call counter behind `#[cfg(test)]`).
- V-UT-9: `ram_typed_read_matches_memmove` in `device/ram.rs::tests` —
  for each of sizes {1, 2, 4, 8} at aligned and unaligned addresses,
  assert the typed-read path (or the memmove fallback on unaligned)
  returns identical bytes to a reference `slice::copy_from_slice`
  implementation.

[**Integration Tests**]

- V-IT-2: Wall-clock on dhry/cm/mb via `make run` with `DEBUG=n`;
  deltas vs. `docs/perf/2026-04-15/data/bench.csv`. Thresholds: dhry
  ≥ 20 %, cm ≥ 20 %, mb ≥ 15 % (G-5).
- V-IT-3: `make linux`, `make linux-2hart`, `make debian` all boot;
  latency within ±5 % of post-P1 baseline.
- V-IT-4: Hit-rate telemetry via `--features perf-stats`, dumped at
  exit. Threshold ≥ 95 % on dhry/cm/mb (G-2). Linux boot hit rate
  captured opportunistically; recorded but not gated.
- V-IT-5: P5 MMU inline asm audit —
  `cargo asm --release --features perf-stats
   xemu::arch::riscv::cpu::mm::RVCore::checked_read` shows the TLB-hit
  path lowered with no `call` to `Bus::read` on the hit branch.
  Transcript committed alongside the implementation.
- V-IT-6: P6 memmove bucket drop — `_platform_memmove` + `memcpy` PLT
  combined < 2 % of self-time on dhry/cm/mb in the round's
  `docs/perf/<post-hotPath-date>/data/*.sample.txt`;
  `Bus::read + Bus::write` combined drops by ≥ 3 pp.

[**Failure / Robustness Validation**]

- V-F-1: `decode_failure_does_not_poison_line` — inject a raw word
  that fails `DECODER.decode`; assert the cache line at that index is
  unchanged (pre-existing valid line survives). Pins I-5.
- V-F-2: `mid_execution_bytes_change_triggers_miss` — write new bytes
  at the current PC; next fetch observes different `raw`; assert line
  replaced and new instruction executes. SMC without any hook. Pins
  I-7.
- V-F-5: `mtimer_reset_restores_deadline_to_max` — after `reset()`,
  assert `next_fire_mtime == u64::MAX` and the fast-path short-circuits
  for every subsequent tick until a guest writes `mtimecmp`.

[**Edge Case Validation**]

- V-E-1: `compressed_and_full_inst_at_adjacent_pc` — execute `c.addi`
  at `pc=P`, `addi` at `pc=P+2`; both should hit on re-execution. Pins
  I-6, I-11.
- V-E-2: `index_aliasing_at_conflict` — craft two PCs separated by
  `2 * ICACHE_LINES`; assert full-`pc` tag mismatch forces
  miss-and-replace; both still execute correctly.
- V-E-4: `mtimecmp_write_to_u64_max_sets_deadline_max` — write
  `u64::MAX` to `mtimecmp[0]` of a 1-hart system; assert
  `next_fire_mtime == u64::MAX` and gate short-circuits.
- V-E-5: `ram_read_size_3_falls_through_to_memmove` — size ∉
  {1, 2, 4, 8} takes the generic path; output matches reference.
- V-E-6: `mmio_read_takes_device_path_not_typed` — read from the
  Mtimer MMIO region; typed bypass must NOT activate (would skip
  device side effects).

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (P3 Mtimer gate) | V-UT-8, V-F-5, V-E-4, Exit Gate §A P3 |
| G-2 (P4 hit ≥ 95 %)  | V-UT-1, V-UT-2, V-IT-4, Exit Gate §A P4 |
| G-3 (P5 MMU inline)  | V-IT-5, Exit Gate §A P5 |
| G-4 (P6 memmove)     | V-UT-9, V-IT-6, V-E-5, V-E-6, Exit Gate §A P6 |
| G-5 (wall-clock)     | V-IT-2, Exit Gate §B |
| G-6 (xdb::main drop) | V-IT-2 (re-run sample.sh) |
| G-7 (correctness)    | V-UT-3, V-F-1, V-F-2, V-IT-3, `make test`, `make run` |
| G-8 (mutex-free)     | `bash scripts/ci/verify_no_mutex.sh` |
| C-1 (no workload switch) | Review grep for workload names |
| C-2 (mandated commands)  | §Validation preamble + §Exit Gate header |
| C-6 (no Mutex regression) | `verify_no_mutex.sh` |
| C-8 (perf-stats off)      | `Cargo.toml` default-features inspection |
| C-9 (no new unsafe)       | `cargo clippy --all-targets` after Phase 4 |
| C-10 (no invalidation hooks) | §Architecture §P4, §Invariants I-13..I-15 "removed" notes |
| I-6 (compressed) | V-E-1 |
| I-7 (SMC implicit) | V-F-2, V-UT-3 |
| I-9 (fence.i NOP) | code audit `inst/privileged.rs:71-79` unchanged |
| I-10 (no Mutex) | `verify_no_mutex.sh` |
| I-11 (geometry + aliasing) | V-E-2 |
| I-12 (miss rule) | V-UT-2 |
| I-16 (mtimer deadline) | V-UT-8, V-F-5, V-E-4 |
| I-17 (RAM + aligned + size ∈ {1,2,4,8}) | V-UT-9, V-E-5, V-E-6 |

---

## Exit Gate

**Mandatory repo command block (R-004):**

```
make fmt      # rustfmt clean
make clippy   # no new warnings
make run      # dhrystone / coremark / microbench all green
make test     # cargo test --workspace + am-tests all green
```

Plus `bash scripts/ci/verify_no_mutex.sh` reports `ok`.

### §A — Per-phase binding gates (R-002)

Each sub-bullet must pass independently. If any single bullet misses,
that phase is not considered landed and must be split into a follow-up
iteration before the round is declared complete. The bundled
implementation branch still exists for workflow reasons (M-001, M-003),
but attribution stays phase-scoped per `docs/PERF_DEV.md:186-190`.

- **P3 (Mtimer).** Combined `Mtimer::check_timer + tick + mtime`
  self-time bucket < 1 % on dhry/cm/mb under
  `docs/perf/<post-hotPath-date>/data/*.sample.txt`. All am-tests
  pass. Linux / Linux-2hart / Debian boot latency within ±5 % of
  post-P1 baseline.
- **P4 (icache).** `xdb::main` self-time share drops by ≥ 10 pp on
  all three workloads (from 40/47/45 % → ≤ 30/37/35 %). Icache hit
  rate ≥ 95 % on dhry/cm/mb under `--features perf-stats`.
- **P5 (MMU inline).** MMU-entry bucket drops by ≥ 3 pp. Trap bucket
  evidence is **not** part of this gate (R-003 Option A dropped the
  trap-slim subgoal). `cargo asm` transcript V-IT-5 committed.
- **P6 (memmove).** `_platform_memmove + memcpy` combined bucket
  drops below 2 % on dhry/cm/mb. `Bus::read + Bus::write` combined
  drops by ≥ 3 pp.

### §B — Combined bundle gates

- Wall-clock reduction ≥ **20 %** on dhrystone, ≥ **20 %** on
  coremark, ≥ **15 %** on microbench (G-5).
- `cargo test --workspace` green; the new unit tests under V-UT-1 /
  V-UT-2 / V-UT-3 / V-UT-8 / V-UT-9 and V-F-1 / V-F-2 / V-F-5 all
  pass.
- `make fmt && make clippy && make run && make test` clean (R-004).
- `bash scripts/ci/verify_no_mutex.sh` reports `ok` (C-6).
- Benchmark artefacts committed under
  `docs/perf/<post-hotPath-date>/` including `data/bench.csv`,
  `data/*.sample.txt`, `graphics/*.svg`, and the asm-audit transcript
  for V-IT-5.

### Policy

- If a §A bullet misses, the corresponding phase is rolled forward to
  round 03 (or split into its own `docs/perf/<phase>/` as M-001 is
  re-negotiated). The remaining phases still land provided §B passes
  *without* their contribution — i.e., §B thresholds are verified
  against the reduced bundle.
- If §B misses outright, the round returns to `03_PLAN` with the
  failing phases re-scoped.
