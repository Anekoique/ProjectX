# `perfHotPath` SPEC

> Source: [`/docs/archived/perf/perfHotPath/04_PLAN.md`](/docs/archived/perf/perfHotPath/04_PLAN.md).
> Iteration history, trade-off analysis, and implementation
> plan live under `docs/archived/perf/perfHotPath/`.

---


[**Goals**]

- G-1 (P3): Cached-deadline short-circuit in `Mtimer::tick` so the
  default path is one `u64` compare + return; combined `Mtimer::*`
  self-time < 1 % on all three benchmarks. (DEV.md#phase-9-performance-optimization §3 P3.)
- G-2 (P4): Eliminate the pest tree walk on every cache-hit fetch; the
  fast path becomes one `(pc, raw)` compare + one POD copy of
  `DecodedInst`. Hit rate ≥ 95 % on dhry / cm / mb. (DEV.md#phase-9-performance-optimization
  §3 P4.)
- G-3 (P5): Inline the TLB-hit MMU fast path end-to-end through
  `checked_read` / `checked_write` / `access_bus`; MMU bucket drops ≥
  3 pp (evidence by sampling profile per V-IT-5). (DEV.md#phase-9-performance-optimization §3 P5.)
- G-4 (P6): Bypass `_platform_memmove` for aligned 1/2/4/8-byte RAM
  accesses via typed primitive reads/writes; memmove+memcpy combined
  bucket < 2 %; `Bus::read + Bus::write` combined drops ≥ 3 pp.
  (DEV.md#phase-9-performance-optimization §3 P6.)
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
- NG-2: No paddr-tagged SMC bitmap.
- NG-3: No replacement for `pest`.
- NG-4: No benchmark-specific code paths.
- NG-5: No assembly-file modifications.
- NG-6: No new `Arc<Mutex<_>>`, `RwLock<_>`, `RefCell<_>`, or
  `Box<dyn FnMut>` on `Bus` or `RVCore`.
- NG-7: No multi-thread SMP work.
- NG-8: No invalidation hooks on `satp` / `sfence.vma` / `fence.i` /
  privilege transitions / `mstatus` / RAM stores.
- NG-9: No rework of `retire` / `commit_trap`.
- NG-10: No reduced-bundle re-interpretation of §B thresholds.
- NG-11: No binding dependency on `cargo-asm`.

[**Architecture**]

Top-level shape (unchanged from round 03):

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

### Architecture §P3 — Mtimer deadline gate

Spec citations (M-003):
- **RISC-V Privileged ISA Manual §3.1.10 (`mtime` / `mtimecmp`)** — a
  timer interrupt is pending whenever `mtime ≥ mtimecmp[h]` for hart
  `h`; therefore `min(mtimecmp[*])` is a sound lower bound on the
  next possible firing.
- **RISC-V Privileged ISA Manual §3.1.12 (Sstc)** — Supervisor Sstc
  extends the same relation to `stimecmp`; xemu currently does not
  enable Sstc on the hot path, but the deadline-gate invariant stays
  compatible.

Implementation: `Mtimer` at
`xcore/src/arch/riscv/device/aclint/mtimer.rs` grows one field
`next_fire_mtime: u64`, initialised `u64::MAX`. `tick()` adds
`if self.mtime < self.next_fire_mtime { return; }` before the existing
`check_all`. Recompute `next_fire_mtime` on every `mtimecmp` write, at
the end of `check_timer`, and on `reset`. No change to interrupt
semantics; only the default-path branch count is reduced. Style
follows the existing Mtimer implementation of sibling
`check_timer` / `check_all` (M-004).

### Architecture §P4 — Decoded-instruction cache (SMC gate fully wired)

Spec citations (M-003):
- **RISC-V Unprivileged ISA Manual §5.1 (Zifencei, `fence.i`).** Stores
  to instruction memory become visible to later instruction fetches
  on the same hart only after `FENCE.I`. The SMC test issues
  `fence.i` for architectural hygiene. In xemu, the icache correctness
  invariant is `(pc, raw)`-keyed miss-on-change (I-12), which
  satisfies the architectural observation without the fence, so
  `fence.i` remains a NOP.
- **RISC-V Unprivileged ISA Manual Ch. 2 + Table 24.2.** `addi rd,
  rs1, imm` is encoded as `[imm(11:0) | rs1 | 000 | rd | 0010011]`
  (opcode `OP-IMM`); with `rs1 = x0` and `rd = a0` we get
  `0x00000513 | ((imm & 0xfff) << 20)`. `jalr x0, 0(ra)` (the `ret`
  pseudo-instruction) is `[0 | ra | 000 | 00000 | 1100111]` =
  `0x00008067`. These encodings back the `ENCODE_ADDI_A0_ZERO(imm)`
  and `ENCODE_RET` macros in `smc.c`.
- **RISC-V psABI / RV G-ABI** — `a0` (= `x10`) is the first argument
  / return value register for the standard C calling convention. The
  `smc.c` test observes the function's return through the C return
  channel, which lowers to `a0`.

Design: per-hart direct-mapped 4096-line cache. Key `(pc, raw)`.
Line `{ pc, raw, decoded }`. No `ctx_tag`. No invalidation hooks.
`fence.i` stays a NOP. SMC falls out because the next fetch reads
different `raw`, the key misses, the line is overwritten.

Round-04 change at this section is verification-only and harness-level:

- Phase 2a wires the SMC am-test through **all three** dispatch
  layers (header, main dispatcher, Makefile), so letter `m` executes
  `test_smc()` — not the default-help fallthrough.
- `smc.c` observes the result through the C function-return channel
  (register `a0`), not through `x1`/`ra`.
- Invariant I-18 restated as an ABI-visible observable.
- V-IT-1 remains the binding gate; V-F-2 / V-UT-3b are Rust-level
  mirrors.

### Architecture §P5 — MMU fast-path inline

Spec citations (M-003):
- **RISC-V Privileged ISA Manual Zicsr §3.1.** CSR access semantics
  for `satp` / `mstatus` — no invalidation hook is required for a
  decode cache that keys on `(pc, raw)`; `satp` changes cause
  different `pc → paddr` mappings whose fetches naturally read
  potentially different `raw`, and `mstatus.MPRV` applies to
  loads/stores, not fetch.

Target files: `xcore/src/arch/riscv/cpu/mm.rs` (`access_bus`,
`checked_read`, `checked_write`, `translate`, `fetch`, `load`,
`store`). No algorithmic change. Add `#[inline]` /
`#[inline(always)]` where the current pinned rustc + LTO does not
already fold. Style follows the existing free-function-style helpers
in `cpu/mm.rs` (M-004).

Evidence:

- **Primary (binding).** MMU-entry bucket
  (`access_bus + checked_read + checked_write + load + store`) drops
  ≥ 3 pp between `docs/perf/baselines/2026-04-15/data/*.sample.txt` and
  `docs/perf/<post-hotPath-date>/data/*.sample.txt`.
- **Optional (author-side, not a gate).** `cargo rustc --release -p
  xcore -- --emit=asm` yields `.s` files under `target/release/deps/`.

Trap slim remains DROPPED (round-02 R-003 Option A).

### Architecture §P6 — memmove typed-read bypass

`Ram::read` / `Ram::write` (`xcore/src/device/ram.rs`) gain a size +
alignment pre-check. For aligned 1/2/4/8-byte accesses, use
`u{8,16,32,64}::from_le_bytes(slice[..N].try_into()?)` /
`to_le_bytes`. Fall through to the existing memmove path for MMIO,
unaligned, and odd-size cases. No new `unsafe` unless the safe form
fails to lower; in that case, `ptr::read_unaligned` with an explicit
`// SAFETY:` block (alignment/in-bounds/no-aliasing).

[**Invariants**]

- I-1: `RVCore::step` always calls `fetch` before any icache lookup.
- I-2: `pest` (via `DECODER.decode`) is the sole decode authority.
- I-3: Icache is owned per-hart; no shared state, no atomics.
- I-4: On miss, a line is overwritten in full (`pc`, `raw`,
  `decoded`); partial updates forbidden.
- I-5: Decode failure does not write a line.
- I-6: Compressed instructions: `raw` carries the fetched word as-is.
- I-7: SMC implicit: next fetch reads different `raw`; `(pc, raw)`
  misses; line overwritten. No store hook.
- I-8: Stores to MMIO do not participate in icache logic.
- I-9: `fence.i` remains a NOP (`inst/privileged.rs:71-79`); correct
  under I-7 because the next fetch already reflects any prior store.
- **I-10** (from P1, binding): `CPU::step` destructures `self` into
  disjoint borrows; no `Mutex<Bus>`.
- **I-11:** Icache geometry — per-hart, direct-mapped, 4096 lines,
  index `(pc >> 1) & MASK`.
- **I-12:** Cache miss iff `line.pc != self.pc || line.raw != raw`.
  Sole correctness rule for P4.
- I-16 (P3): `next_fire_mtime = min(mtimecmp[*])`, recomputed on
  every `mtimecmp` write, at the end of `check_timer`, and on
  `reset`. Initialised `u64::MAX`.
- I-17 (P6): Typed-read bypass iff region is RAM AND `size ∈ {1, 2,
  4, 8}` AND `addr % size == 0`; all other cases fall through.
- **I-18 (P4 SMC torture contract, round-04 refined):** For any
  RAM-backed address `pc` mapped RX-and-W, the sequence
  `store_word(pc, raw_a); fetch(pc) == raw_a; call_as_fn_ptr(pc);
  observe a0 == value_a; store_word(pc, raw_b); fence.i;
  fetch(pc) == raw_b; call_as_fn_ptr(pc); observe a0 == value_b`
  must hold end-to-end. This is the ABI-visible observable the
  restored am-test `smc.c` (letter `m`) witnesses. At the ICache
  layer the invariant reduces to I-12 (`(pc, raw)` mismatch on
  `raw_b ≠ raw_a`).

Invariants I-13 / I-14 / I-15 from round 01 remain removed — no
per-hart context tag, no `mstatus` hook, no privilege hook.

[**Data Structure**]

```rust
// xcore/src/arch/riscv/cpu/icache.rs  (new; identical to round 03)
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

**SMC am-test source (new file at
`xkernels/tests/am-tests/src/tests/smc.c`, round-04 R-001/R-002):**

```c
#include "test.h"
#include <stdint.h>

/*
 * SMC torture test for the P4 decoded-instruction cache.
 *
 * RISC-V Unprivileged ISA Manual Section 5.1 (Zifencei): a store to
 * instruction memory becomes visible to subsequent instruction
 * fetches on this hart only after FENCE.I. This test writes a tiny
 * function into RAM, executes it (expecting return value 0),
 * overwrites the immediate field, issues FENCE.I, and re-executes
 * (expecting return value 42).
 *
 * P4 contract (I-12): the icache is keyed on (pc, raw). The
 * overwrite changes the raw word at `pc`, so the cache comparison
 * misses and re-decodes with the new bits. FENCE.I remains a NOP
 * in xemu per the decoded-raw simplification; the cache-miss-on-
 * raw-change path is the actual correctness lever.
 *
 * Encodings (RISC-V Unprivileged ISA v2.2, Chapter 2 + Table 24.2):
 *   addi a0, zero, imm   = 0x00000513 | (imm << 20)
 *   jalr zero, 0(ra)     = 0x00008067        (= ret pseudo-instr)
 *
 * Observable channel: the function's return value flows through
 * a0 per the RV G-ABI (psABI), so the C-level assertion is
 * `check(ret == expected)`.
 */

#define ENCODE_ADDI_A0_ZERO(imm)  (0x00000513u | ((uint32_t)(imm) << 20))
#define ENCODE_RET                 0x00008067u

static uint32_t smc_buf[2] __attribute__((aligned(4)));

typedef int (*smc_fn_t)(void);

void test_smc(void) {
    /* Phase 1 -- write `addi a0, zero, 0; ret`, execute, expect 0. */
    smc_buf[0] = ENCODE_ADDI_A0_ZERO(0);
    smc_buf[1] = ENCODE_RET;
    asm volatile ("fence.i" ::: "memory");
    smc_fn_t fn = (smc_fn_t)smc_buf;
    int r0 = fn();
    check(r0 == 0);

    /* Phase 2 -- overwrite immediate, fence.i, re-execute, expect 42. */
    smc_buf[0] = ENCODE_ADDI_A0_ZERO(42);
    asm volatile ("fence.i" ::: "memory");
    int r1 = fn();
    check(r1 == 42);

    printf("smc: OK\n");
}
```

Style follows the existing am-test files (e.g.
`xkernels/tests/am-tests/src/tests/trap-ecall.c`): one `#include
"test.h"`, a single `void test_*(void)` entry point, `check()`
assertions, a trailing `printf` line for visible confirmation. The
`HIT GOOD TRAP` marker for the outer Makefile grep
(`xkernels/tests/am-tests/Makefile:36`) comes from the am-tests
runtime on normal `main` return (`halt(0)`).

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
- C-2 (round-04, R-003 resolved): The Mandatory Verification block in
  §Validation preamble and §Exit Gate header must pass after every
  implementation phase. The block carries both the AGENTS.md §
  Development Standards line-8 form (`make -C xemu fmt / clippy /
  run / test`) **and** the stronger workspace/doc/perf/am-test/boot
  checks. No top-level shorthand is emitted (no repo-root Makefile
  exists).
- C-3: Benchmarks run with `DEBUG=n`.
- C-4 (round-04, R-003 resolved): Workloads launched via `make -C
  resource linux` / `make -C resource linux-2hart` / `make -C
  resource debian` for OS smoke; per-benchmark runs go through
  `scripts/perf/bench.sh`. Never by hand-calling `target/release/xdb`
  directly.
- C-5: No assembly-file modifications.
- C-6: `bash scripts/ci/verify_no_mutex.sh` remains `ok`.
- C-7: Scope ends at P3+P4+P5+P6.
- C-8: `perf-stats` Cargo feature is off by default; release binaries
  ship without it.
- C-9 (round-04, R-004 resolved): No new `unsafe` in the P6 path if
  `from_le_bytes + try_into` lowers to a single aligned load. Proof
  command: `cd xemu && cargo clippy --all-targets -- -D warnings`.
- C-10: No invalidation hooks on CSR / `fence.i` / `sfence.vma` /
  `satp` / trap / RAM stores.
- C-11: No binding gate may depend on a tool absent from the current
  environment. `cargo-asm` is explicitly optional.
- C-12: §B thresholds apply only to the full P3+P4+P5+P6 bundle.

---
