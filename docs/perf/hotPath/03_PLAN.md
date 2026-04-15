# `hotPath` PLAN `03`

> Status: Draft
> Feature: `hotPath`
> Iteration: `03`
> Owner: Executor
> Depends on:
> - Previous Plan: `02_PLAN.md`
> - Review: `02_REVIEW.md`
> - Master Directive: `02_MASTER.md` (blank; `00_MASTER.md` directives M-001 / M-002 / M-003 remain in force)

---

## Summary

Round 03 keeps the round-02 core unchanged — decoded-raw `(pc, raw)` cache
with zero invalidation hooks (round-01 R-001), binding per-phase Exit
Gate (round-02 R-002), P5 trap-slim dropped (round-02 R-003 Option A),
P3 Mtimer deadline gate, P6 Ram typed-read bypass, bundled scope under
`00_MASTER.md` M-001 — and resolves the four findings in `02_REVIEW.md`:

1. **R-001 HIGH.** Restore the SMC am-test (`xkernels/tests/am-tests/src/tests/smc.c`,
   test-letter `m`) as a binding P4 pre-phase step and §A exit-gate row.
   `docs/PERF_DEV.md:321` still names "the new text-modifying am-test
   passes" as a P4 exit condition. Round 02 silently deleted that gate;
   round 03 reinstates it. The round-02 `(pc, raw)` collapse makes the
   test trivially satisfied (new bytes → different `raw` → miss →
   re-decode), so the restoration is zero-cost at the design level —
   one C file, one `ALL` entry in the am-tests Makefile, one extra row
   in Exit Gate §A. Reviewer advice TR-1 adopted (keep the cache
   simplification, restore the gate).
2. **R-002 HIGH.** Rewrite the mandatory-command block with exact
   runnable invocations keyed to the actual Makefile layout. There is
   no repo-root `Makefile`; round 02's `make fmt && make clippy &&
   make run && make test` was mechanically wrong. Round 03 replaces
   it with `make -C xemu fmt`, `make -C xemu clippy`, `make -C xemu
   test`, `cargo test --workspace`, `cargo test --doc -p xcore`,
   `make -C xkernels/tests/am-tests run`, `make -C resource linux`,
   `make -C resource linux-2hart`, `make -C resource debian`, and the
   benchmark / sample / render scripts under `scripts/perf/`. Each
   command is verified against its source Makefile (see §Validation
   Strategy preamble). Reviewer advice TR-2 adopted (exact commands
   over inherited shorthand).
3. **R-003 MEDIUM.** `cargo-asm` is not present in the environment
   (`command -v cargo-asm` returns non-zero); round 03 demotes it to
   an optional author-side aid and replaces V-IT-5 with profile-bucket
   delta evidence using the already-committed `scripts/perf/sample.sh`
   + `scripts/perf/render.py` pipeline. `cargo rustc --release --
   --emit=asm` remains available as an optional alternate spot-check.
4. **R-004 MEDIUM.** The "reduced-bundle escape hatch" in round-02 §B
   is removed. Any §A per-phase miss triggers a fresh PLAN iteration
   for the failing phase. §B thresholds remain defined only for the
   full P3 + P4 + P5 + P6 bundle; if the bundle splits, §B is
   re-evaluated fresh in the follow-up plan, not reinterpreted here.
   Attribution is binary: each phase is landed or split.

All round-02 design correctness is preserved: three-field `ICacheLine`
(`pc`, `raw`, `decoded`), `fence.i` remains a NOP, no CSR / trap /
store hooks, per-phase §A gates, combined §B gates, M-001 bundled
scope, M-003 clean layout, `perf-stats` Cargo feature, no benchmark-
targeted tricks. No re-introduction of `Mutex<Bus>`, `ctx_tag`, or
invariants I-13 / I-14 / I-15.

## Log

[**Feature Introduce**]

- **SMC am-test restored** at `xkernels/tests/am-tests/src/tests/smc.c`
  with letter `m`. The am-tests `Makefile` `ALL` set becomes
  `u r t s p c e f m` and the `name` substitution chain gains
  `$(patsubst m,smc-exec,...)`. The test writes `addi x1, x0, 0`
  at a known RAM address, executes it, overwrites with `addi x1, x0,
  42`, issues `fence.i`, executes again, and uses the existing
  am-tests `check(cond)` helper to enforce `x1 == 42` before printing
  the `HIT GOOD TRAP` marker. The success regex in
  `xkernels/tests/am-tests/Makefile:36` (`grep -q "GOOD TRAP"`) is
  satisfied by the am-tests runtime on `halt(0)`. The test passes
  trivially under the `(pc, raw)` cache (second fetch reads `raw_b ≠
  raw_a` at the same PC → miss → re-decode of the new instruction).
  `fence.i` stays a NOP; correctness is from the cache key, not the
  hook.
- **Executable command block.** Every mandatory verification line now
  names its working directory via `-C` or is an absolute
  `cargo ... --workspace` call. The block is duplicated verbatim in
  §Validation Strategy preamble and §Exit Gate header so the reviewer
  sees identical instructions in both locations.
- **V-IT-5 re-defined.** Bucket-delta evidence via the existing
  sampling pipeline. The target row is the MMU-entry bucket
  (`access_bus` + `checked_read` + `checked_write` + `load` +
  `store`) in `docs/perf/<post-hotPath-date>/data/*.sample.txt`,
  compared against `docs/perf/2026-04-15/data/*.sample.txt`.
  Threshold: ≥ 3 pp drop, matching the round-01 / round-02 P5 target.
- **Escape hatch removed.** Exit Gate §A policy statement now reads:
  "If any §A sub-gate misses, the failing phase is not landed and a
  fresh PLAN iteration must be opened for that phase. §B is a single
  gate for the full P3 + P4 + P5 + P6 bundle and is not reinterpreted
  against reduced bundles." No "tentative land," no "against the
  reduced bundle" language anywhere in the plan.

[**Review Adjustments**]

- **R-001** (HIGH, restore + TR-1 adopt). See §Spec Goals (G-7
  expanded), §Invariants I-18, §Validation V-IT-1, §Implementation
  Plan phase 2, §Exit Gate §A P4.
- **R-002** (HIGH, TR-2 adopt). See §Validation Strategy preamble,
  §Exit Gate header, §Implementation Plan phase 5.
- **R-003** (MEDIUM). See §Spec Architecture §P5, §Validation V-IT-5,
  §Exit Gate §A P5, §Implementation Plan phase 3.
- **R-004** (MEDIUM, Option 1). See §Exit Gate §A Policy, §Exit Gate
  §B preamble, §Trade-offs T-2.

[**Master Compliance**]

- **M-001 (combined scope).** P3 + P4 + P5 + P6 remain in a single
  branch / commit. §Implementation Plan Phases 1–5 unchanged in count
  and ordering relative to round 02.
- **M-002 (path rename).** All paths remain under
  `docs/perf/hotPath/`. No new path churn.
- **M-003 (clean layout).** Single Summary, single Log, single
  Response Matrix, single Spec block (Architecture subsections by
  phase only where they diverge), single Trade-offs, single
  Validation, single Exit Gate. No restructuring beyond the four
  surgical fixes above.

### Changes from Previous Round

[**Added**]

- §Invariants I-18: SMC am-test torture contract (read / write / exec
  cycle at the same RAM-backed PC observes the new instruction).
- V-UT-3b: optional Rust mirror test
  `smc_raw_mismatch_reindexes_line` in `cpu/icache.rs::tests`
  asserting that writing new bytes at the same `pc` produces a
  different `raw` on re-lookup and overwrites the line.
- §Implementation Plan phase 2a: pre-icache SMC am-test commit (locks
  the behaviour contract; must pass on the baseline).
- §Implementation Plan phase 2b: icache integration commit (am-test
  continues to pass trivially due to `(pc, raw)` mismatch).
- §Validation Strategy preamble: verified command block with working
  directories and per-line `# what passing means` annotations.
- §Exit Gate header: same block, one-to-one with the preamble.
- §Trade-offs T-8: reviewer position on `cargo-asm` as a gate (drop
  vs. bootstrap), with explicit rejection of the bootstrap route.
- §Trade-offs T-9: R-004 options compared (remove escape hatch vs.
  predefine reduced-bundle thresholds), Option 1 chosen.

[**Changed**]

- V-IT-5 redefined from `cargo asm` transcript to profile-bucket
  delta evidence.
- §Exit Gate §A P5 evidence row changed accordingly.
- §Exit Gate §A policy rewritten: any miss = fresh PLAN.
- §Exit Gate §B preamble rewritten: thresholds only valid for the full
  bundle; no reduced-bundle recomputation.
- §Validation Strategy preamble replaced `make fmt && make clippy &&
  make run && make test` with the workdir-qualified block.
- §Constraints C-2 and C-4 rewritten against the workdir-qualified
  commands.

[**Removed**]

- Round-02 string "against the reduced bundle" and any partial-land
  acceptance language.
- Round-02 binding `cargo asm` requirement.
- Round-02 top-level shorthand `make fmt && make clippy && make run
  && make test` (replaced by the verified block).

[**Unresolved**]

- The SMC am-test's `fence.i` stays a NOP in xemu
  (`inst/privileged.rs:71-79`). The test uses `fence.i` for
  architectural hygiene (real hardware requires it); correctness in
  xemu is provided by `(pc, raw)` mismatch, not by the fence. The
  test is still valid regression for hardware-faithful behaviour
  because a conformant implementation *may* rely on `fence.i`, but
  xemu does not need to. Noted, not a gap.
- If Linux / Debian boots show any icache hit-rate anomaly under
  `--features perf-stats`, it is recorded for a future iteration;
  boot is not on the hit-rate gate (gates are dhry / cm / mb only).
- If the bucket-delta evidence for V-IT-5 shows < 3 pp drop on any
  workload, P5 splits out per Exit Gate §A policy; there is no
  reduced-bundle path to hide this.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Master `00_MASTER.md` | M-001 | Applied | Bundled P3 + P4 + P5 + P6 scope preserved. See §Spec Architecture §P3..§P6 and §Implementation Plan phases 1–5. |
| Master `00_MASTER.md` | M-002 | Applied | All references stay under `docs/perf/hotPath/`. No path changes in round 03. |
| Master `00_MASTER.md` | M-003 | Applied | Single top-level sections; per-phase subsections only where design diverges. |
| Master `01_MASTER.md` | — | N/A | Blank; no round-01 directives. |
| Master `02_MASTER.md` | — | N/A | Blank; no round-02 directives. `00_MASTER.md` M-001/M-002/M-003 remain in force. |
| Round-00 Review | R-001 | Obsolete per round-01 R-001 collapse | `checked_write` SMC flush is not installed; `(pc, raw)` keying handles SMC implicitly. |
| Round-00 Review | R-002 | Resolved in round 03 | SMC am-test restored (letter `m`) per round-02 review R-001. `xkernels/tests/am-tests/src/tests/smc.c` added; Makefile `ALL` += `m`; `HIT GOOD TRAP` marker gates §A P4. |
| Round-00 Review | R-003 | Obsolete per round-01 R-001 collapse | `mstatus` bit-isolation hook is not installed; writes do not affect decode. |
| Round-00 Review | R-004 | Obsolete per round-01 R-001 collapse | Privilege-debounced trap hook is not installed; privilege transitions do not affect decode of `raw`. |
| Round-00 Review | R-005 | Retained | Index `(pc >> 1) & MASK`; full-`pc` tag disambiguates aliases. XOR-fold deferred pending Linux hit-rate telemetry. |
| Round-00 Review | R-006 | Obsolete | MPRV torture targeted the deleted `mstatus` hook; MPRV does not participate in decode. |
| Round-00 Review | R-007 | Obsolete | `ctx_tag` does not exist; width discussion moot. |
| Round-00 Review | R-008 | Retained | `perf-stats` Cargo feature in `xemu/xcore/Cargo.toml`, off by default. |
| Round-00 Review | R-009 | Retained | Response Matrix populated in full. |
| Round-01 Review | R-001 | Accepted (round 02) | Decoded-raw cache; carries through unchanged. |
| Round-01 Review | R-002 | Accepted (round 02) | Per-phase §A + combined §B gates; preserved, escape hatch removed in round 03. |
| Round-01 Review | R-003 | Accepted (round 02, Option A) | P5 trap-slim dropped; preserved. |
| Round-01 Review | R-004 | Superseded by round-02 R-002 | Round-02 block was shorthand; round-03 installs the verified workdir-qualified block. |
| Round-02 Review | R-001 | Resolved (HIGH) | SMC am-test restored as binding P4 pre-phase + §A exit-gate row; TR-1 adopted. See §Spec Goals G-7, §Invariants I-18, §Validation V-IT-1, §Exit Gate §A P4, §Implementation Plan phase 2a. |
| Round-02 Review | R-002 | Resolved (HIGH) | Command block rewritten with exact `make -C <dir>` invocations verified against `xemu/Makefile:44-54`, `resource/Makefile:30-33`, `resource/debian.mk:54`, `xkernels/tests/am-tests/Makefile:1-47`; TR-2 adopted. See §Validation Strategy preamble, §Exit Gate header, §Implementation Plan phase 5. |
| Round-02 Review | R-003 | Resolved (MEDIUM) | `cargo-asm` demoted to optional; V-IT-5 replaced with bucket-delta evidence from `scripts/perf/sample.sh` + `render.py`. See §Architecture §P5, §Validation V-IT-5, §Exit Gate §A P5, §Trade-offs T-8. |
| Round-02 Review | R-004 | Resolved (MEDIUM, Option 1) | Escape hatch removed; any §A miss opens a fresh PLAN iteration for that phase; §B not reinterpreted against reduced bundles. See §Exit Gate §A Policy, §Exit Gate §B preamble, §Trade-offs T-9. |
| Round-02 Review | TR-1 | Adopted | Keep decoded-raw simplification; restore SMC am-test. See §Trade-offs T-1 (re-affirmed) + R-001 resolution. |
| Round-02 Review | TR-2 | Adopted | Exact workdir-qualified commands. See §Validation Strategy preamble + R-002 resolution. |

> Rules:
> - Every prior HIGH / CRITICAL finding appears above.
> - Every MASTER directive appears above.
> - Rejections / obsoletions cite the triggering cause (round-01 R-001 collapse or later).

---

## Spec

[**Goals**]

- G-1 (P3): Cached-deadline short-circuit in `Mtimer::tick` so the
  default path is one `u64` compare + return; combined `Mtimer::*`
  self-time < 1 % on all three benchmarks.
- G-2 (P4): Eliminate the pest tree walk on every cache-hit fetch; the
  fast path becomes one `(pc, raw)` compare + one POD copy of
  `DecodedInst`. Hit rate ≥ 95 % on dhry / cm / mb.
- G-3 (P5): Inline the TLB-hit MMU fast path end-to-end through
  `checked_read` / `checked_write` / `access_bus`; MMU bucket drops ≥
  3 pp (evidence by sampling profile per V-IT-5).
- G-4 (P6): Bypass `_platform_memmove` for aligned 1 / 2 / 4 / 8-byte
  RAM accesses via typed primitive reads / writes; memmove + memcpy
  combined bucket < 2 %; `Bus::read + Bus::write` combined drops ≥ 3
  pp.
- G-5 (combined): Wall-clock reduction on the 2026-04-15 baseline —
  dhry ≥ 20 %, cm ≥ 20 %, mb ≥ 15 %.
- G-6 (combined): `xdb::main` self-time share drops by ≥ 10 pp on all
  three workloads.
- G-7 (correctness): All existing `cargo test --workspace` outcomes
  unchanged; `make -C xkernels/tests/am-tests run` passes including
  the new letter `m` (SMC test); Linux / Linux-2hart / Debian boot to
  prompt.
- G-8 (mutex-free): `bash scripts/ci/verify_no_mutex.sh` stays `ok`.

- NG-1: No JIT, no trace chaining, no threaded dispatch.
- NG-2: No paddr-tagged SMC bitmap. Not needed under `(pc, raw)`.
- NG-3: No replacement for `pest`.
- NG-4: No benchmark-specific code paths.
- NG-5: No assembly-file modifications.
- NG-6: No new `Arc<Mutex<_>>`, `RwLock<_>`, `RefCell<_>`, or
  `Box<dyn FnMut>` on `Bus` or `RVCore`.
- NG-7: No multi-thread SMP work.
- NG-8: No invalidation hooks on `satp` / `sfence.vma` / `fence.i` /
  privilege transitions / `mstatus` / RAM stores.
- NG-9: No rework of `retire` / `commit_trap`.
- NG-10: No reduced-bundle re-interpretation of §B thresholds
  (round-03 R-004).
- NG-11: No binding dependency on `cargo-asm` (round-03 R-003).

[**Architecture**]

Top-level shape (unchanged from round 02):

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

### Architecture §P3 — Mtimer deadline gate (unchanged from round 02)

`Mtimer` at `xcore/src/arch/riscv/device/aclint/mtimer.rs` grows one
field `next_fire_mtime: u64`, initialised `u64::MAX`. `tick()` adds
`if self.mtime < self.next_fire_mtime { return; }` before the
existing `check_all`. Recompute `next_fire_mtime` on every
`mtimecmp` write, at the end of `check_timer`, and on `reset`.

### Architecture §P4 — Decoded-instruction cache (round-02 design, SMC gate restored)

Per-hart direct-mapped 4096-line cache. Key `(pc, raw)`. Line
`{ pc, raw, decoded }`. No `ctx_tag`. No invalidation hooks.
`fence.i` remains a NOP. SMC falls out because the next fetch reads
different `raw`, the key misses, the line is overwritten.

The round-03 change at this section is verification-only:

- The SMC am-test (`xkernels/tests/am-tests/src/tests/smc.c`, letter
  `m`) is now a binding P4 pre-phase commit and §A exit-gate row.
- Invariant I-18 captures the torture contract: any sequence
  `store(pc, raw_b); fetch(pc) == raw_b; execute(pc)` must observe
  the semantics of `decode(raw_b)`.
- V-IT-1 restored as a gate (was demoted in round 02).

### Architecture §P5 — MMU fast-path inline (round-03 R-003)

Target files: `xcore/src/arch/riscv/cpu/mm.rs` (`access_bus`,
`checked_read`, `checked_write`, `translate`, `fetch`, `load`,
`store`).

No algorithmic change. Add `#[inline]` / `#[inline(always)]` where
LTO alone does not fold. Evidence:

- **Primary (binding).** Bucket delta from sampling profile.
  `docs/perf/<post-hotPath-date>/data/*.sample.txt` vs
  `docs/perf/2026-04-15/data/*.sample.txt`. The MMU-entry bucket
  (`access_bus` + `checked_read` + `checked_write` + `load` +
  `store`) must drop by ≥ 3 pp on all three workloads.
- **Optional (author-side).** `cargo rustc --release -p xcore --
  --emit=asm` produces `.s` files under `target/release/deps/`; the
  author may grep for remaining `call` edges on the hit path. This
  route does not require the missing `cargo-asm` crate and is not on
  any gate.

Trap slim remains DROPPED (round-02 R-003 Option A). `retire` at
`cpu.rs:152-159` is already the steady-state zero-pending branch;
`commit_trap` is only entered after `pending_trap.take()` returns
`Some`.

### Architecture §P6 — memmove typed-read bypass (unchanged from round 02)

`Ram::read` / `Ram::write` (`xcore/src/device/ram.rs`) gain a size +
alignment pre-check. For aligned 1 / 2 / 4 / 8-byte accesses, use
`u{8,16,32,64}::from_le_bytes(slice[..N].try_into()?)` /
`to_le_bytes`. Fall through to the existing memmove path for MMIO,
unaligned, and odd-size cases.

[**Invariants**]

- I-1: `RVCore::step` always calls `fetch` before any icache lookup.
- I-2: `pest` (via `DECODER.decode`) is the sole decode authority;
  the cache only memoises its result.
- I-3: The icache is owned per-hart; no shared state, no atomics.
- I-4: On miss, a line is overwritten in full (`pc`, `raw`,
  `decoded`); partial updates forbidden.
- I-5: Decode failure does not write a line.
- I-6: Compressed instructions: `raw` carries the fetched word as-is.
- I-7: SMC is implicit: next fetch reads different `raw`; `(pc, raw)`
  misses; line overwritten. No store hook.
- I-8: Stores to MMIO do not participate in icache logic.
- I-9: `fence.i` remains a NOP (`inst/privileged.rs:71-79`); correct
  under I-7 because the next fetch already reflects any prior store.
- I-10 (from P1): `CPU::step` destructures `self` into disjoint
  borrows; no `Mutex<Bus>`.
- I-11: Icache geometry — per-hart, direct-mapped, 4096 lines, index
  `(pc >> 1) & MASK`.
- I-12: Cache miss iff `line.pc != self.pc || line.raw != raw`.
  Sole correctness rule for P4.
- I-16 (P3): `next_fire_mtime = min(mtimecmp[*])`, recomputed on
  every `mtimecmp` write, at the end of `check_timer`, and on
  `reset`. Initialised `u64::MAX`.
- I-17 (P6): Typed-read bypass iff region is RAM AND `size ∈ {1, 2,
  4, 8}` AND `addr % size == 0`; all other cases fall through.
- **I-18 (P4 SMC torture contract, round-03 R-001):** For any RAM
  address `pc`, the sequence `store(pc, raw_b)` followed by
  `fetch(pc)` in the next step returns `raw_b`; the subsequent
  dispatch executes the semantics of `decode(raw_b)`. This is the
  behaviour the restored am-test `smc.c` (letter `m`) and Rust
  mirror `smc_raw_mismatch_reindexes_line` witness.

Invariants I-13 / I-14 / I-15 from round 01 remain removed — no
per-hart context tag, no `mstatus` hook, no privilege hook.

[**Data Structure**]

```rust
// xcore/src/arch/riscv/cpu/icache.rs (new; identical to round 02)
use crate::isa::riscv::decoder::DecodedInst;
use memory_addr::VirtAddr;

pub const ICACHE_BITS:  usize = 12;
pub const ICACHE_LINES: usize = 1 << ICACHE_BITS;
pub const ICACHE_MASK:  usize = ICACHE_LINES - 1;

#[derive(Clone, Copy)]
pub struct ICacheLine {
    pub pc:      VirtAddr,
    pub raw:     u32,
    pub decoded: DecodedInst,
}

impl ICacheLine {
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

// RVCore (xcore/src/arch/riscv/cpu.rs) gains ONE field:
pub struct RVCore {
    // ... existing fields unchanged ...
    pub(in crate::arch::riscv) icache: Box<ICache>,
}

// Mtimer (xcore/src/arch/riscv/device/aclint/mtimer.rs) gains ONE field:
pub(super) struct Mtimer {
    // ... existing fields unchanged ...
    next_fire_mtime: u64, // P3: min(mtimecmp[*]); starts u64::MAX
}
```

`DecodedInst` gains `Copy` in addition to its existing derives.

[**API Surface**]

```rust
impl ICache {
    pub fn new() -> Box<Self>;
    #[inline] pub fn index(pc: VirtAddr) -> usize;
}

impl RVCore {
    #[inline]
    fn decode_cached(&mut self, raw: u32) -> XResult<DecodedInst>;
}

impl Mtimer {
    #[inline]
    fn recompute_next_fire(&mut self) {
        self.next_fire_mtime =
            self.mtimecmp.iter().copied().min().unwrap_or(u64::MAX);
    }
}
```

No new public API on `RVCore` / `Bus` / `Mtimer` facades.

[**Constraints**]

- C-1: No benchmark-targeted code.
- C-2 (round-03 R-002, rewritten): The mandatory verification block
  in §Validation Strategy preamble and §Exit Gate header must pass
  after every implementation phase. Top-level shorthand
  `make fmt` / `make run` / `make test` is forbidden in validation
  because no repo-root `Makefile` exists (verified against
  `/Users/anekoique/ProjectX/xemu/Makefile`,
  `/Users/anekoique/ProjectX/resource/Makefile`,
  `/Users/anekoique/ProjectX/resource/debian.mk`,
  `/Users/anekoique/ProjectX/xkernels/tests/am-tests/Makefile`).
- C-3: Benchmarks run with `DEBUG=n`.
- C-4 (round-03 R-002, rewritten): Workloads launched via
  `make -C resource linux` / `make -C resource linux-2hart` /
  `make -C resource debian` for OS smoke; per-benchmark runs go
  through `scripts/perf/bench.sh`. Never by hand-calling
  `target/release/xdb` directly.
- C-5: No assembly-file modifications.
- C-6: `bash scripts/ci/verify_no_mutex.sh` remains `ok`.
- C-7: Scope ends at P3 + P4 + P5 + P6.
- C-8: `perf-stats` Cargo feature is off by default; release binaries
  ship without it.
- C-9: No new `unsafe` in the P6 path if `from_le_bytes` + `try_into`
  lowers to a single aligned load.
- C-10: No invalidation hooks on CSR / `fence.i` / `sfence.vma` /
  `satp` / trap / RAM stores (NG-8 cross-reference).
- **C-11 (round-03 R-003):** No binding gate may depend on a tool
  absent from the current environment. `cargo-asm` is explicitly
  optional; all binding evidence uses tools already present
  (`cargo`, `make`, `python3`, the committed `scripts/perf/*`
  binaries).
- **C-12 (round-03 R-004):** §B thresholds apply only to the
  **full** P3 + P4 + P5 + P6 bundle. If §A forces a phase split,
  §B is re-evaluated in the follow-up plan for the then-current
  scope; it is not retroactively reinterpreted against a reduced
  bundle in this round.

---

## Implement

### Execution Flow

[**Main Flow**]

1. `RVCore::step` calls `self.fetch(bus)?` → `raw: u32` (unchanged).
2. `idx = ICache::index(self.pc)`.
3. `let line = &mut self.icache.lines[idx]`.
4. `hit = line.pc == self.pc && line.raw == raw`.
5. Hit: `decoded = line.decoded` (POD copy).
6. Miss: `decoded = DECODER.decode(raw)?`; `*line =
   ICacheLine { pc: self.pc, raw, decoded }`.
7. `self.execute(bus, decoded)?`.
8. `Bus::tick()` → `Mtimer::tick()` runs the P3 deadline
   short-circuit.
9. Guest load/store via `checked_read` / `checked_write` uses the
   P5-inlined TLB-hit path; RAM accesses follow P6 typed-read fast
   path when size and alignment permit.

[**Failure Flow**]

1. `fetch` traps → `Err(...)` before icache access; cache unchanged.
2. `DECODER.decode(raw)?` errors on miss → return before writing; I-5.
3. `execute` errors after cache hit → line kept; next fetch re-hits
   and traps identically.
4. SMC (I-7, I-18): guest writes new bytes; next fetch reads new
   `raw`; `(pc, raw)` misses; line overwritten; new instruction
   executes. **This is the path the restored am-test `smc.c`
   witnesses.**
5. `checked_write` traps (PMP / alignment / page-fault): error
   propagates; icache unaffected (no hook).
6. P3: `mtime < next_fire_mtime` → `tick` returns before
   `check_all`; correct because no hart's deadline is reached.
7. P6: MMIO / unaligned / odd-size → falls through to memmove;
   identical semantics.

[**State Transition**]

- `icache line (pc, raw match) → hit` → POD copy of `decoded`.
- `icache line (pc or raw mismatch) → miss` → line replaced.
- `icache line (decode fails) → unchanged`; error propagates.
- `guest store to (pc) → next fetch at pc reads new raw → (pc, raw)
  miss → line replaced with re-decoded instruction` (I-18, SMC path).
- `mtimecmp write → next_fire_mtime recomputed` (P3).
- `mtime ≥ next_fire_mtime → check_all runs`; else early return.
- `RAM read/write size ∈ {1, 2, 4, 8} ∧ aligned → typed path`; else
  memmove (P6).
- `MMU TLB hit → inlined load path`; `TLB miss → existing translate
  slow path` (P5).

### Implementation Plan

Each phase must pass the mandatory command block in §Validation
Strategy preamble and `bash scripts/ci/verify_no_mutex.sh` before the
next phase begins.

[**Phase 1 — P3 Mtimer deadline gate**]

1. Add `next_fire_mtime: u64` to `Mtimer` (`mtimer.rs:26-33`), init
   `u64::MAX`.
2. Add `recompute_next_fire` helper (§API Surface).
3. Modify `tick` (`mtimer.rs:112-123`): after the existing
   `SYNC_INTERVAL` sync, short-circuit on
   `self.mtime < self.next_fire_mtime`.
4. Call `recompute_next_fire` in `write` after every `Mtimecmp`
   mutation (`mtimer.rs:95-108`), at the end of `check_timer`, and
   at the end of `reset` (`mtimer.rs:129-139`).
5. Unit test V-UT-8 + V-F-5 + V-E-4 land in the same commit.

[**Phase 2 — P4 ICache**]

Split into two commits for clean attribution:

*Phase 2a — SMC am-test (pre-icache, round-03 R-001):*

1. Add `xkernels/tests/am-tests/src/tests/smc.c` writing
   `addi x1, x0, 0` to a known RAM location, executing it,
   overwriting with `addi x1, x0, 42`, issuing `fence.i`,
   executing again, asserting `x1 == 42` via the existing
   am-tests `check` helper, then `halt(0)` for the `GOOD TRAP`
   marker.
2. Update `xkernels/tests/am-tests/Makefile`:
   - `ALL` becomes `u r t s p c e f m` (line 17).
   - The `name` substitution chain (line 18) gains
     `$(patsubst m,smc-exec,...)`.
3. Verify with `make -C xkernels/tests/am-tests run TEST=m`. On the
   **baseline** (no icache yet) the am-test passes because fetch
   reads RAM directly — this is the locked-in behaviour contract.
4. Commit includes only the test and Makefile edits; no Rust
   changes.

*Phase 2b — ICache integration:*

1. New file `xcore/src/arch/riscv/cpu/icache.rs` implementing
   `ICache`, `ICacheLine`, `ICACHE_BITS = 12` (§Data Structure).
2. Derive `Copy` on `DecodedInst` in `isa/riscv/decoder.rs:161`.
3. V-UT-1: `fn _assert_copy<T: Copy>() {}` against `DecodedInst`.
4. Add `icache: Box<ICache>` to `RVCore` (`cpu.rs:36-54`);
   initialise in `RVCore::new` via `ICache::new()`.
5. Add `decode_cached` (§API Surface).
6. Replace `self.decode(raw)?` in `cpu.rs:238-245` with
   `self.decode_cached(raw)?`.
7. Cargo feature `perf-stats` in `xemu/xcore/Cargo.toml`, off by
   default; behind it, add hit / miss counters on `ICache` with
   `&mut` increment on each lookup and a dump-at-exit hook.
8. V-UT-2 (cache miss when raw changes at same pc) + V-UT-3b
   (Rust SMC mirror) land in the same commit.
9. Re-run `make -C xkernels/tests/am-tests run TEST=m`; the test
   continues to pass because the second fetch reads a different
   `raw` and the `(pc, raw)` comparison misses → line is
   overwritten with the new decoded instruction.

[**Phase 3 — P5 MMU fast-path inline (round-03 R-003)**]

1. Identify the TLB-hit call chain: `checked_read` /
   `checked_write` / `access_bus` / `Bus::read` / `Bus::write` /
   `Ram::read` / `Ram::write` (all in `xcore/src/`).
2. Add `#[inline]` / `#[inline(always)]` where the current
   pinned rustc + LTO does not already fold. No algorithmic change.
3. Capture primary evidence: run
   `bash scripts/perf/sample.sh --out docs/perf/<post-hotPath-date>`
   and `python3 scripts/perf/render.py --dir
   docs/perf/<post-hotPath-date>`. Compare the MMU-entry bucket
   against `docs/perf/2026-04-15/data/*.sample.txt`. Commit the
   post-phase `sample.txt` files for the gate evidence.
4. Optional author-side spot-check: `cargo rustc --release -p xcore
   -- --emit=asm` (not a gate; not committed).
5. Trap slim DROPPED (round-02 R-003 Option A, preserved).

[**Phase 4 — P6 memmove typed-read bypass**]

1. In `Ram::read` (`xcore/src/device/ram.rs`), size-match on 1 / 2 /
   4 / 8 with alignment check; use
   `u{8,16,32,64}::from_le_bytes(slice[..N].try_into()?)`.
2. In `Ram::write`, mirror with `to_le_bytes`.
3. MMIO and unaligned / odd-size paths untouched.
4. If the bucket delta for memmove fails to improve under the pinned
   rustc, fall back to `ptr::read_unaligned` with explicit
   `// SAFETY:` comment covering alignment / in-bounds / no-aliasing.
5. V-UT-9 + V-E-5 + V-E-6 land in the same commit.

[**Phase 5 — Benchmark capture + final gate**]

1. `bash scripts/perf/bench.sh --out docs/perf/<post-hotPath-date>`
   (3 iters × 3 workloads) with `DEBUG=n`.
2. `bash scripts/perf/sample.sh --out docs/perf/<post-hotPath-date>`.
3. `python3 scripts/perf/render.py --dir
   docs/perf/<post-hotPath-date>`.
4. Capture hit rate via `cargo build --release -p xcore --features
   perf-stats` + workload run (stats dumped at exit).
5. Diff `data/bench.csv` vs.
   `docs/perf/2026-04-15/data/bench.csv`; compare self-time bucket
   tables against §Exit Gate §A / §B.
6. Run the full mandatory command block from §Validation Strategy
   preamble (see that section for the exact commands and per-line
   meanings).

## Trade-offs

- **T-1: Decoded-raw cache vs. translation-context cache.** (Round
  02, TR-1 adopted.) Key `(pc, raw)`; no hooks; SMC via `raw`
  mismatch. Proof: `Decoder::decode(raw)` pure
  (`isa/riscv/decoder.rs:130-143`); static dispatch tables
  (`decoder.rs:206-260`); fetch resolves translation before decode
  (`cpu/mm.rs:306-315`, `cpu.rs:238-245`). Translation-context cache
  (Option B) rejected — fights G-2, adds hooks for zero correctness
  benefit in this codebase.
- **T-2: Bundled round vs. four separate rounds.** (Round 02, TR-2
  adopted for the gate structure.) Bundled with binding §A per-phase
  gates; any §A miss splits the failing phase into a fresh PLAN
  (round-03 R-004 removes the round-02 escape hatch).
- **T-3: Why the round-01 R-001 collapse is correct in this
  codebase.** Preserved from round 02. xemu memoises decode, not
  translation; QEMU TCG memoises translated basic blocks and
  therefore must invalidate on translation-context changes. NEMU
  IBuf caches decode; rvemu caches nothing; both are sound without
  translation-context invalidation.
- **T-4 / T-5 / T-6 / T-7:** Preserved from round 02 (trap-slim drop,
  safe-first typed read, mtimer recompute timing, threaded-dispatch
  out-of-scope). No round-03 changes.
- **T-8: `cargo-asm` gate vs. profile-based evidence (round-03
  R-003).**
  - *Bootstrap `cargo-asm` (rejected):* adds a tool install step to
    every contributor workflow and to CI; increases the build matrix;
    delivers asm dumps that still require hand-interpretation for the
    bucket-drop claim.
  - *Profile-based (chosen):* uses already-committed
    `scripts/perf/sample.sh` + `render.py`; the evidence format
    matches `docs/perf/2026-04-15/REPORT.md` §3; the bucket delta is
    the actual performance claim.
  - Optional author-side `cargo rustc -- --emit=asm` remains
    available as a spot-check; not on any gate.
- **T-9: Escape hatch vs. binary phase attribution (round-03
  R-004).**
  - *Predefine reduced-bundle thresholds (rejected):* multiplies the
    gate surface (combinatorial over which phases drop out), invites
    post-hoc threshold selection, and still does not avoid splitting
    the failing phase.
  - *Binary attribution (chosen, Option 1):* simpler, testable, no
    thresholds to negotiate. If any §A sub-gate misses, that phase
    splits to a fresh PLAN; §B stays defined for the full bundle
    only.

Sources:

- QEMU TB cache maintenance:
  https://github.com/qemu/qemu/blob/master/accel/tcg/tb-maint.c
- QEMU per-CPU jump cache:
  https://github.com/qemu/qemu/blob/master/include/exec/tb-jmp-cache.h
- NEMU IBuf: https://github.com/OpenXiangShan/NEMU
- rvemu (baseline, no cache): https://github.com/d0iasm/rvemu
- RISC-V Zifencei §5.1: https://github.com/riscv/riscv-isa-manual/releases
- RISC-V Privileged §3.1.6.3 (MPRV applies to loads/stores, not
  fetch): same release index.
- `docs/PERF_DEV.md` §3 Phase P4 (authoritative; "the new
  text-modifying am-test passes"):
  /Users/anekoique/ProjectX/docs/PERF_DEV.md:307, :321.
- `docs/PERF_DEV.md` §3 per-phase gates:
  /Users/anekoique/ProjectX/docs/PERF_DEV.md:186-190.
- `AGENTS.md` §Development Standards (authoritative for the
  mandatory command list): /Users/anekoique/ProjectX/AGENTS.md.

## Validation

**Mandatory repo verification (round-03 R-002, TR-2 adopted).** Run
after every implementation phase and once more at the final Exit
Gate. Every command below was verified against a concrete Makefile
target or existing script path; no top-level shorthand.

```sh
# 1. Rust lint + format gates (xemu crate tree):
make -C xemu fmt          # xemu/Makefile:50-51; passes iff `cargo fmt --all` reports no diff.
make -C xemu clippy       # xemu/Makefile:47-48; passes iff no new clippy warnings in production crates.

# 2. Mutex sentinel (M-001 from P1 inherited):
bash scripts/ci/verify_no_mutex.sh
                          # Passes iff no `Arc<Mutex<Bus>>` / `RwLock<Bus>` / `RefCell<Bus>` / `Box<dyn FnMut>` on Bus or RVCore.

# 3. Unit + doc test coverage across the workspace:
cd xemu && cargo test --workspace
                          # Passes iff all workspace unit + integration tests green (supersedes `make -C xemu test`, which is only `cargo test -p xcore`, xemu/Makefile:53-54).
cd xemu && cargo test --doc -p xcore
                          # M-001 compile_fail sentinel green.

# 4. am-test bundle (round-03 R-001 P4 gate):
make -C xkernels/tests/am-tests run
                          # xkernels/tests/am-tests/Makefile:45-47; passes iff every letter in `ALL` reports PASS; the new letter `m` (`smc-exec`) must be present and green.

# 5. Benchmark wall-clock (all three workloads, 3 iters each):
bash scripts/perf/bench.sh --out docs/perf/<post-hotPath-date>
                          # Passes iff dhry ≥ 20 %, cm ≥ 20 %, mb ≥ 15 % vs docs/perf/2026-04-15/data/bench.csv.

# 6. Sampling profile (round-03 R-003: replaces cargo-asm for V-IT-5):
bash scripts/perf/sample.sh --out docs/perf/<post-hotPath-date>
python3 scripts/perf/render.py --dir docs/perf/<post-hotPath-date>
                          # Passes iff MMU bucket drops ≥ 3 pp and memmove + memcpy bucket < 2 % on dhry / cm / mb.

# 7. OS boots (required for §A P5 MMU regression check):
make -C resource linux          # resource/Makefile:32 (linux:run-linux); boot to prompt within ±5 % latency of post-P1.
make -C resource linux-2hart    # resource/Makefile:33; same.
make -C resource debian         # resource/debian.mk:54 (debian:run-debian); same.
```

Note: there is no top-level `Makefile` at `/Users/anekoique/ProjectX/`,
so `make fmt`, `make clippy`, `make run`, `make test` (without a
`-C` subdir) do **not** run; they were a round-02 error and are
forbidden by C-2.

Additional targeted evidence (Rust unit tests, Rust integration
tests, hit-rate telemetry) supplements — does not replace — the block
above.

[**Unit Tests**]

- V-UT-1: `decoded_inst_is_copy` in `isa/riscv/decoder.rs::tests` —
  static `_assert_copy<DecodedInst>()`. Pins POD property.
- V-UT-2: `icache_miss_when_raw_changes_at_same_pc` in
  `cpu/icache.rs::tests` — store line at `(pc, raw_a)`, lookup at
  `(pc, raw_b)`, assert miss. Pins I-12.
- V-UT-3b (round-03 R-001 mirror, optional): `smc_raw_mismatch_reindexes_line`
  in `cpu/icache.rs::tests` — build an `ICache`, insert line at
  `(pc, raw_a)`, re-lookup at `(pc, raw_b)`, assert the subsequent
  re-decode overwrites the line such that `lines[idx].raw == raw_b`.
  Belt-and-braces; the am-test V-IT-1 is the primary gate.
- V-UT-8: `mtimer_deadline_short_circuits` in
  `device/aclint/mtimer.rs::tests` — set `mtimecmp[0] = u64::MAX`;
  tick 1000 times; assert `check_all` was not called.
- V-UT-9: `ram_typed_read_matches_memmove` in
  `device/ram.rs::tests` — each of 1 / 2 / 4 / 8 at aligned +
  unaligned; assert identical output vs. reference.

[**Integration Tests**]

- **V-IT-1 (round-03 R-001 restored, binding):** am-test
  `xkernels/tests/am-tests/src/tests/smc.c` (letter `m`) passes
  under `make -C xkernels/tests/am-tests run` (or `... run TEST=m`
  for isolated invocation). Success is detected by the existing
  `HIT GOOD TRAP` marker grep in
  `xkernels/tests/am-tests/Makefile:36`. The test writes
  `addi x1, x0, 0` at a RAM address, executes, overwrites with
  `addi x1, x0, 42`, `fence.i`, executes again, and asserts
  `x1 == 42` via the am-tests `check` helper before `halt(0)`.
- V-IT-2: Wall-clock on dhry / cm / mb via
  `bash scripts/perf/bench.sh --out docs/perf/<post-hotPath-date>`;
  deltas vs `docs/perf/2026-04-15/data/bench.csv`.
- V-IT-3: `make -C resource linux`, `make -C resource linux-2hart`,
  `make -C resource debian` all boot; latency within ±5 % of post-P1.
- V-IT-4: Hit-rate telemetry via `--features perf-stats`, dumped at
  exit. Threshold ≥ 95 % on dhry / cm / mb.
- **V-IT-5 (round-03 R-003 redefined):** MMU bucket drop by ≥ 3 pp
  between `docs/perf/2026-04-15/data/*.sample.txt` and
  `docs/perf/<post-hotPath-date>/data/*.sample.txt`. Measurement
  follows `docs/perf/2026-04-15/REPORT.md` §3 bucket definition for
  `access_bus + checked_read + checked_write + load + store`. No
  `cargo-asm` dependency.
- V-IT-6: `_platform_memmove + memcpy` combined < 2 % on dhry / cm /
  mb; `Bus::read + Bus::write` combined drops ≥ 3 pp.

[**Failure / Robustness Validation**]

- V-F-1: `decode_failure_does_not_poison_line` — inject a raw word
  that fails decode; assert the pre-existing valid line at that
  index survives. Pins I-5.
- V-F-2: `mid_execution_bytes_change_triggers_miss` — Rust-level
  companion to V-IT-1; drives a `RVCore` through `step` → store new
  bytes via `Bus::store` → step; asserts the second step executes
  the newly written instruction. Pins I-7 / I-18 at the Rust layer.
- V-F-5: `mtimer_reset_restores_deadline_to_max` — after `reset()`,
  assert `next_fire_mtime == u64::MAX` and `tick` short-circuits for
  every subsequent call until a guest writes `mtimecmp`.

[**Edge Case Validation**]

- V-E-1: `compressed_and_full_inst_at_adjacent_pc` — `c.addi` at
  `pc=P`, `addi` at `pc=P+2`; both hit on re-execution. Pins I-6 / I-11.
- V-E-2: `index_aliasing_at_conflict` — PCs separated by
  `2 * ICACHE_LINES`; full-`pc` tag mismatch forces miss-and-replace;
  both still execute correctly.
- V-E-4: `mtimecmp_write_to_u64_max_sets_deadline_max` — write
  `u64::MAX`, assert deadline unchanged and gate short-circuits.
- V-E-5: `ram_read_size_3_falls_through_to_memmove` — size ∉ {1, 2,
  4, 8} takes the generic path.
- V-E-6: `mmio_read_takes_device_path_not_typed` — typed bypass must
  NOT activate on MMIO regions.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (P3 Mtimer gate) | V-UT-8, V-F-5, V-E-4, Exit Gate §A P3 |
| G-2 (P4 hit ≥ 95 %) | V-UT-1, V-UT-2, V-IT-4, Exit Gate §A P4 |
| G-3 (P5 MMU inline) | V-IT-5 (profile bucket delta), Exit Gate §A P5 |
| G-4 (P6 memmove) | V-UT-9, V-IT-6, V-E-5, V-E-6, Exit Gate §A P6 |
| G-5 (wall-clock) | V-IT-2, Exit Gate §B |
| G-6 (xdb::main drop) | V-IT-2 (re-run sample.sh) |
| G-7 (correctness, incl. SMC am-test) | **V-IT-1**, V-F-1, V-F-2, V-UT-3b, V-IT-3, `cargo test --workspace`, `make -C xkernels/tests/am-tests run` |
| G-8 (mutex-free) | `bash scripts/ci/verify_no_mutex.sh` |
| C-1 (no workload switch) | Review grep for workload names |
| C-2 (mandated commands) | §Validation Strategy preamble + §Exit Gate header (verbatim block) |
| C-4 (launch via make -C / scripts) | §Validation Strategy preamble |
| C-6 (no Mutex regression) | `verify_no_mutex.sh` |
| C-8 (perf-stats off) | `Cargo.toml` default-features inspection |
| C-9 (no new unsafe) | `make -C xemu clippy --all-targets` after Phase 4 |
| C-10 (no invalidation hooks) | §Architecture §P4, §Invariants (no I-13/I-14/I-15) |
| C-11 (no cargo-asm gate) | V-IT-5 uses profile delta only |
| C-12 (§B not reinterpreted) | §Exit Gate §B preamble |
| I-6 (compressed) | V-E-1 |
| I-7 (SMC implicit) | V-F-2, V-UT-3b, **V-IT-1** |
| I-9 (fence.i NOP) | code audit `inst/privileged.rs:71-79` unchanged |
| I-10 (no Mutex) | `verify_no_mutex.sh` |
| I-11 (geometry + aliasing) | V-E-2 |
| I-12 (miss rule) | V-UT-2 |
| I-16 (mtimer deadline) | V-UT-8, V-F-5, V-E-4 |
| I-17 (P6 bypass conditions) | V-UT-9, V-E-5, V-E-6 |
| **I-18 (SMC torture contract)** | **V-IT-1** (primary), V-F-2 + V-UT-3b (Rust mirrors) |

---

## Exit Gate

**Mandatory repo command block (round-03 R-002, TR-2 adopted):**

```sh
make -C xemu fmt                              # rustfmt clean
make -C xemu clippy                           # no new warnings
bash scripts/ci/verify_no_mutex.sh            # `ok`
cd xemu && cargo test --workspace             # workspace tests green
cd xemu && cargo test --doc -p xcore          # compile_fail sentinel green
make -C xkernels/tests/am-tests run           # every letter in ALL passes; letter `m` MUST be present
bash scripts/perf/bench.sh --out docs/perf/<post-hotPath-date>
bash scripts/perf/sample.sh --out docs/perf/<post-hotPath-date>
python3 scripts/perf/render.py --dir docs/perf/<post-hotPath-date>
make -C resource linux
make -C resource linux-2hart
make -C resource debian
```

No top-level shorthand. Every line is a runnable invocation.

### §A — Per-phase binding gates (round-02 R-002; round-03 R-004 policy)

Each sub-bullet must pass independently.

**Policy (round-03 R-004, Option 1):** If any §A sub-gate misses,
the failing phase is NOT considered landed and a **fresh PLAN
iteration** must be opened to address that phase. The bundled branch
stays as one commit for workflow reasons, but attribution is binary:
each phase lands or splits. There is no "reduced-bundle"
re-interpretation of §B.

- **P3 (Mtimer).** Combined `Mtimer::check_timer + tick + mtime`
  self-time bucket < 1 % on dhry / cm / mb under
  `docs/perf/<post-hotPath-date>/data/*.sample.txt`.
  `make -C xkernels/tests/am-tests run` passes. Linux / Linux-2hart
  / Debian boot latency within ±5 % of post-P1.
- **P4 (icache).** `xdb::main` self-time share drops by ≥ 10 pp on
  all three workloads (from 40 / 47 / 45 % → ≤ 30 / 37 / 35 %).
  Icache hit rate ≥ 95 % on dhry / cm / mb under `--features
  perf-stats`. **V-IT-1: am-test `smc.c` (letter `m`) passes;
  test-letter wired in `xkernels/tests/am-tests/Makefile`; success
  detected by `HIT GOOD TRAP` marker.** (Round-03 R-001.)
- **P5 (MMU inline).** MMU-entry bucket
  (`access_bus + checked_read + checked_write + load + store`) drops
  by ≥ 3 pp between `docs/perf/2026-04-15/data/*.sample.txt` and
  `docs/perf/<post-hotPath-date>/data/*.sample.txt` — V-IT-5, evidence
  via `scripts/perf/sample.sh` + `render.py` only; no `cargo-asm`
  dependency. Trap bucket evidence is NOT part of this gate (round-02
  R-003 Option A preserved).
- **P6 (memmove).** `_platform_memmove + memcpy` combined bucket
  drops below 2 % on dhry / cm / mb. `Bus::read + Bus::write`
  combined drops by ≥ 3 pp.

### §B — Combined bundle gates

**Preamble (round-03 R-004):** §B thresholds are defined only for
the full P3 + P4 + P5 + P6 bundle. If any §A sub-gate misses and a
phase splits, §B is re-evaluated from scratch in the follow-up
PLAN for the then-current scope. §B is not reinterpreted against a
reduced bundle in this round.

- Wall-clock: dhry ≥ 20 %, cm ≥ 20 %, mb ≥ 15 % vs
  `docs/perf/2026-04-15/data/bench.csv`.
- `xdb::main` self-time share drops by ≥ 10 pp on all three
  workloads.
- `bash scripts/ci/verify_no_mutex.sh` reports `ok`.
- `make -C xemu clippy` clean (no new warnings in production crates).
- `cd xemu && cargo test --workspace` green.
- `make -C xkernels/tests/am-tests run` green (includes letter `m`).
- `make -C resource linux`, `make -C resource linux-2hart`,
  `make -C resource debian` boot to prompt within ±5 % of post-P1.

### Summary

No benchmark-targeted tricks. No `Mutex<Bus>` regression. No scope
leak beyond P3 + P4 + P5 + P6. `fence.i` remains a NOP. SMC am-test
restored as a binding P4 gate; bucket-delta evidence replaces
`cargo-asm`; escape hatch removed; command block is now
executable.
