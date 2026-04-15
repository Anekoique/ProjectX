# xemu Performance Development Roadmap

**Active baseline:** 2026-04-16 (post-hotPath), see [`perf/2026-04-16/data/`](./perf/2026-04-16/data/) (REPORT.md pending, tracked as G-002 in `perf/hotPath/00_IMPL.md`)
**Prior baselines:**
- 2026-04-15 (post-P1) — [`perf/2026-04-15/REPORT.md`](./perf/2026-04-15/REPORT.md)
- 2026-04-14 (pre-P1) — [`perf/2026-04-14/REPORT.md`](./perf/2026-04-14/REPORT.md)

**Target horizon:** Phase 9 "Performance Optimization" of [DEV.md](./DEV.md)

This document translates the sampling-profile findings into a concrete,
staged engineering plan: what to fix, in what order, how to measure each
step, and what evaluation infrastructure we need to build alongside.

Every phase below is designed to be **independently landable**, each
with its own `docs/fix/<feature>/` iteration (per MEMORY conventions),
and **independently measurable** against whichever `perf/<date>/REPORT.md`
is the active baseline at the time the phase begins.

### Phase status (2026-04-16)

| Phase | Title | Status | Measured Δ (user-time mean) |
|------:|-------|--------|-----------------------|
| P1    | Single-hart bus fast path            | ✅ **Landed** 2026-04-15 | −45.5 / −44.9 / −52.4 % wall-clock |
| P2    | Bus-access API refactor              | ❌ **Retired** (subsumed by P1) | n/a |
| P3    | `Mtimer::*` deadline gate            | ✅ **Landed** 2026-04-16 | Mtimer bucket −55 to −65 % absolute |
| P4    | Decoded-instruction cache            | ✅ **Landed** 2026-04-16 | See hotPath bundle row below |
| P5    | MMU fast-path `#[inline]` audit      | ✅ **Landed** 2026-04-16 | (trap-slim dropped per round-02 R-003 Option A) |
| P6    | `memmove` / typed-read bypass        | ✅ **Landed** 2026-04-16 | See hotPath bundle row below |
| —     | hotPath bundle (P3+P4+P5+P6)         | ✅ **Landed** 2026-04-16 | **−16.9 / −21.1 / −18.2 %** user-time (dhry / cm / mb) |
| P7    | Multi-hart scaling re-profile        | pending (measurement only, Phase 11 RFC) | n/a |

Cumulative user-time vs pre-P1 2026-04-14 baseline:
dhrystone 8.09 s → 3.48 s (**−57 %**),
coremark 14.02 s → 5.82 s (**−58 %**),
microbench 85.82 s → 32.91 s (**−62 %**).

The `real_s` column in `bench.csv` carries macOS system-load noise;
`user_s` is the stable per-run CPU-time metric. See
[`docs/perf/hotPath/00_IMPL.md`](./perf/hotPath/00_IMPL.md) §D-001 for
a note on why bucket-share gates composed awkwardly and the
absolute-sample evidence used instead.

All §3 bucket-percentage tables below still reference the 2026-04-15
profile as their source of truth because the 2026-04-16 shares shift
due to Amdahl after P3 shrinks the Mtimer bucket to < 5 %. A REPORT
refresh against the 2026-04-16 capture is tracked as G-002 in
`perf/hotPath/00_IMPL.md`.

---

## 1. Where the time goes today (post-P1, 2026-04-15)

From the three single-hart sampling profiles (dhrystone / coremark /
microbench), self-time now breaks down as:

| Cost centre                                                 | dhry   | cm     | mb     | Character              |
|-------------------------------------------------------------|-------:|-------:|-------:|------------------------|
| `xdb::main` (dispatch + decode + execute, monolithic / LTO) | 40.4 % | 46.8 % | 44.7 % | Core interp overhead   |
| MMU entry (`access_bus` + `checked_*` + `load`)             | 15.3 % | 12.1 % | 14.0 % | Per load/store         |
| `_platform_memmove` + `memcpy` PLT (RAM shim)               |  9.4 % | 10.1 % | 11.1 % | Per load/store >1B     |
| `Mtimer::check_timer` + `tick` + `mtime`                    |  8.8 % | 10.7 % | 10.3 % | Per-step branch        |
| `Bus::read` + `Bus::write`                                  |  7.9 % |  8.6 % |  8.9 % | Wraps the memmove shim |
| Compressed / base inst leaves (`c_ld`, `c_lw`, `c_jr`, …)   |  6.0 % |  6.3 % |  5.1 % | ISA dispatch leaves    |
| `PLIC evaluate + tick`                                      |  2.5 % |  2.5 % |  3.0 % | Per bus-tick           |
| `_dyld_start` / mach syscalls                               |  9.4 % |  2.5 % |  2.7 % | Capture-window dilution |
| Uart + other devices                                        |  0.2 % |  0.2 % |  0.1 % | Post-directIrq         |
| **`pthread_mutex_*` (baseline dominant)**                   |  **0.0 %** | **0.0 %** | **0.0 %** | **Eliminated by P1** |

Three cross-cutting observations fall out:

- **`xdb::main` is now the #1 bucket at 40–47 %.** After P1 removed the
  mutex tax, Amdahl promoted the CPU dispatch loop into the dominant
  share. This is exactly what a decoded-instruction cache (P4) targets.
- **The guest-RAM-access shim is 17–20 %** once `_platform_memmove` +
  `memcpy` PLT + `Bus::read` + `Bus::write` are summed — considerably
  bigger than it appeared pre-P1, because those samples used to be
  counted under the mutex bucket. P6's typed-read bypass therefore
  has more headroom than originally stated.
- **MMU entry is 12–15 % and stable** across workloads — unchanged by
  P1. P5's inlining/trap-slim work still applies proportionally.

Full breakdown and figures live in
[`perf/2026-04-15/REPORT.md`](./perf/2026-04-15/REPORT.md) §3.

### Historical note on the pre-P1 shape

Before P1 landed, the self-time breakdown on the 2026-04-14 baseline
was dominated by the `Arc<Mutex<Bus>>` bucket (33–40 %), with
`xdb::main` at 25–31 % and MMU entry at ~15 %. See
[`perf/2026-04-14/REPORT.md`](./perf/2026-04-14/REPORT.md) §3 for the
full table. An even earlier iteration of this roadmap mistakenly
reported `Mtimer::check_timer` at ~20 % due to a PID-collision bug in
`scripts/perf/sample.sh` (H1, fixed 2026-04-14); both the 2026-04-14
and 2026-04-15 tables use correct per-workload captures.

This ranking is stable across all three benchmarks, i.e. it is
structural, not workload-specific.

---

## 2. Root-cause analysis

### 2.1 ~~`Arc<Mutex<Bus>>`~~ — resolved by P1

Introduced in `5e66d51` (multi-hart support), the `Arc<Mutex<Bus>>`
dominated the 2026-04-14 profile at 33–40 % self-time. Since xemu's
multi-hart scheduler is single-threaded cooperative round-robin
(`xemu/xcore/src/cpu/mod.rs:213-249`), there was never more than one
thread inside `Bus` at a time — the mutex was pure overhead.

**Phase P1 (`docs/perf/busFastPath/`) removed it.** `CPU` now owns
`Bus` inline; `CPU::step` destructures `self` into disjoint borrows
and hands each hart a `&mut Bus` for the duration of the step. The
post-P1 profile at `perf/2026-04-15/` shows zero `pthread_mutex_*`
samples on any workload, and wall-clock dropped 45–52 %.

True per-hart SMP (OS threads) is deferred to
[`DEV.md` Phase 11 RFC](./DEV.md); getting there requires atomic guest
RAM access, per-hart reservations, and per-device MMIO locking — out
of scope for this roadmap's remaining phases.

### 2.2 `xdb::main` — the CPU dispatch loop is now the #1 cost *(P4)*

`xdb::main` (which collapses `CPU::run` + `CPU::step` + all the
per-instruction handlers under LTO + `codegen-units = 1`) is 40.4 % /
46.8 % / 44.7 % of self-time on dhry / cm / mb respectively. Every
guest instruction walks a match-on-`DecodedInst` branch tree, then
calls the corresponding handler. Decoding itself is pest-matched once
per fetch; a decoded-instruction cache (P4) can skip the decode on
subsequent executions of the same PC, which is the standard
interpreter win.

This is the single largest remaining lever. A per-hart direct-mapped
icache with `pc + ctx_tag` keys and ≥ 4 K entries should retire
"decode + dispatch" for every hot inner-loop instruction.

### 2.3 MMU entry — per-access overhead *(P5, stable)*

`checked_read` is the gateway for every guest load/store: permission
bits + TLB probe + (on miss) page walk. The TLB is already a 64-entry
direct-mapped ASID-tagged array (fast), so most of the cost is the
non-TLB scaffolding: bounds checks, privilege selection, MPRV
handling, and the bus-dispatch indirection that follows. Each is cheap
individually, but runs millions of times per second.

Bucket share on the 2026-04-15 baseline: 12.1–15.3 % across workloads.
Unchanged from pre-P1 (the mutex never lived inside the MMU itself,
only inside the `Bus::read`/`Bus::write` gateway downstream of it).
Phase P5 still targets this with `#[inline]` pressure and trap-slim
work.

### 2.4 Guest-RAM memmove shim — bigger than it looked *(P6)*

`Bus::read` and `Bus::write` are implemented on top of
`bytemuck::copy_within` / `slice::copy_from_slice`, both of which
lower to libsystem `_platform_memmove` plus a `DYLD-STUB$$memcpy`
indirection. Post-P1 these show as:

- `_platform_memmove` + `memcpy` PLT stubs: **9.4 / 10.1 / 11.1 %**
- `Bus::read` + `Bus::write`: **7.9 / 8.6 / 8.9 %**
- Combined shim bucket: **17–20 %** of self-time

The pre-P1 baseline under-reported this (sample budget was eaten by
the mutex bucket). P6 was originally sized at ≤ 4 %; it should be
re-targeted at **5–10 %** wall-clock. 1/2/4/8-byte accesses —
overwhelmingly the common case — can read/write the aligned RAM slice
as the corresponding `u8/u16/u32/u64` primitive without going through
the generic memmove path.

### 2.5 `Mtimer::*` — promoted by Amdahl *(P3)*

`Bus::tick()` → `Mtimer::tick()` → `check_all()` → `check_timer(hart)`
runs for every hart, every step (`xcore/src/arch/riscv/device/aclint/mtimer.rs:52`).
The *comparison* itself is one cycle; the cost is the non-inlined
virtual call + per-step `IrqState` atomic access for what is
>99.99 %-no-op.

On the 2026-04-14 baseline this bucket was 4.9 % combined (check_timer
+ tick + mtime). On the 2026-04-15 post-P1 baseline it is
**8.8 / 10.7 / 10.3 %** — Amdahl doubled its share. P3's expected
wall-clock win is now ~3–5 %, not the ≤ 4 % originally quoted. The
deadline-gate design in §3 is unchanged; the priority ranking simply
moves it up.

### 2.6 Trap framework cost

`commit_trap` is ~4–6 % even on workloads with few real traps (dhry
almost never traps). This is the per-retire inspection of
`pending_trap` plus the interrupt-priority check. Re-validating
priority every instruction is the price for correct interrupt
semantics, but the data-structure accesses can be cheapened. Rolled
into P5 as a secondary target.

### 2.7 Memory is a non-issue

Peak RSS is flat at 40 MiB regardless of workload. No hidden
allocation-heavy hot paths exist at this level — heap profiling is
deferred.

---

## 3. Optimisation roadmap (staged)

Each phase is a separate `docs/perf/<tag>/` task with its own
iteration docs (`00_PLAN` → `00_REVIEW` → `00_MASTER`), per the
project's existing workflow. Each has an **exit gate**: a concrete
wall-clock / profile delta that must be re-measured against the same
three workloads before the phase is declared done.

### Phase P1 — Single-hart bus fast path  ✅ LANDED 2026-04-15
**Tag:** `perf-busFastPath`
**Artefacts:** [`docs/perf/busFastPath/`](./perf/busFastPath/)
 (iterations 00 → 03, `00_IMPL.md`)
**Measured gain:** dhrystone **−45.5 %**, coremark **−44.9 %**,
microbench **−52.4 %** (2026-04-14 → 2026-04-15; mean of 3 runs).

**What shipped.** `CPU` owns `Bus` inline; `CPU::step` destructures
`self` into disjoint `bus` + `cores[current]` borrows and calls
`Core::step(&mut Bus)`. Per M-001 (binding MASTER directive from round
01), no synchronization primitive wraps `Bus`. Three-layer sentinel
prevents regression: (a) `scripts/ci/verify_no_mutex.sh` regex-scans
all of `xemu/xcore/src/`, (b) `#![deny(unused_imports)]` on `bus.rs`
traps stray `use std::sync::Mutex;`, (c) a `compile_fail` rustdoc
example shows the forbidden shape for documentation.

**Exit gate outcome:**

- `pthread_mutex_*` self-time: 33–40 % → **0 %** ✅
- Wall-clock reduction (exit floor 15 %): 45–52 % ✅
- 2-hart Linux boot smoke: clean, no regression ✅
- `cargo test --workspace`: 372 + 1 + 6 + 1 doc-test, all green ✅
- `cargo build --release`: ok, no new clippy warnings in production ✅

True per-hart SMP (OS threads) is deferred to `DEV.md` Phase 11 RFC.

### Phase P2 — Bus-access API refactor  ❌ RETIRED
**Tag:** ~~`perf-batchStep`~~

P2 was designed to amortise the per-access mutex acquisitions that P1
removed entirely. There is no lock left to batch; P2's own exit gate
(`pthread_mutex_*` < 5 %, `access_bus` + `checked_read` combined drop
≥ 3 pp) was achieved as a side effect of P1 landing. The phase is
retired, not merely deferred — no work remains in scope.

If true per-hart OS threads are added later (Phase 11 RFC), the
multi-thread path will need its own locking strategy (per-device
locks, atomic RAM, or both); that is a fresh design, not a revival
of P2.

### Phase P3 — Timer-interrupt deadline gate  ✅ LANDED 2026-04-16
**Tag:** `perf-mtimerDeadline`
**Expected gain on the 2026-04-15 baseline:** −3 % to −5 % wall-clock.
`Mtimer::check_timer` + `tick` + `mtime` combined is 8.8 / 10.7 /
10.3 % on dhry / cm / mb — doubled from the 2026-04-14 share by
Amdahl after P1 removed the mutex bucket.
**Risk:** Very low. Correctness: MTIP must fire no later than now.
**Priority (post-P1):** below P4 and P6 (those remove bigger buckets
first), but above P5. A good candidate to opportunistically bundle
with whichever phase next touches `Bus::tick`.

Cache `min(mtimecmp[*])` as `next_fire_mtime` inside `Mtimer`.  On
each `tick()`:

```rust
if self.mtime < self.next_fire_mtime { return; }  // almost always
self.check_all();                                   // slow path
```

Recompute `next_fire_mtime` whenever a guest writes `mtimecmp`.

Exit gate: combined `Mtimer::*` self-time (check_timer + tick + mtime)
drops below 1 % on dhrystone/coremark/microbench; all `am-tests`
(timer, CSR, interrupts) pass; Linux/Debian boot unchanged.

### Phase P4 — Decoded-instruction cache  ✅ LANDED 2026-04-16
**Tag:** `perf-icache`
**Expected gain on the 2026-04-15 baseline:** −15 % to −25 % wall-clock.
`xdb::main` (which absorbs dispatch + decode + execute under LTO) is
40.4 / 46.8 / 44.7 % of self-time on dhry / cm / mb — the single
largest bucket post-P1. Caching decoded instructions skips the pest
match-tree traversal on subsequent executions of the same PC and is
the standard interpreter win (rvemu, Nemu IBuf, QEMU TB cache, rv8).
**Priority (post-P1):** **#1**. Land this next.
**Risk:** Medium. Correct invalidation covers four events (see below);
any miss produces a silent mis-execute on context switches or
self-modifying code, which is un-test-friendly — add the torture test
*before* the cache.

Per-hart direct-mapped cache.  The key must include enough
translation context to stay correct across `satp` writes:

```rust
struct ICacheLine {
    pc:      usize,          // guest virtual address
    ctx_tag: u32,            // monotone tag bumped on any mapping change
    raw:     u32,            // raw instruction word (sanity)
    decoded: DecodedInst,
}
icache: [ICacheLine; 4096]
```

`ctx_tag` is compared along with `pc` on lookup; any miss or mismatch
re-decodes.  The mapping-change events that must bump `ctx_tag` on
this hart:

- Guest writes to `satp` — `csr::ops::csr_write_side_effects` (see
  `xemu/xcore/src/arch/riscv/cpu/csr/ops.rs:30`, which already flushes
  the TLB via `mmu.update_satp`).  Extend that hook to bump `ctx_tag`.
- `sfence.vma` — privilege/ASID reinterpretation.  Bump `ctx_tag`.
- Privilege-mode transitions that change the effective translation
  mode (`mret`, `sret` when `MPRV` is set; M-mode ⇄ S-mode switches).
  Any path that changes the MMU's `sv`/`asid`/`ppn` must bump.
- `fence.i` — flush all lines for this hart unconditionally.

Self-modifying code on the host side:

- Tag each line with its physical page (derived from the translation
  the fill used).  On every guest store, invalidate by physical-page
  bucket.  Doing this cheaply requires a small reverse index
  (`paddr → set_of_bucket_indexes`); stubbing this out and flushing
  the whole icache on *any* store to RAM is an acceptable first
  iteration, and only loses the icache effect on code-writing guests
  (rare).

**Pre-phase test:**  Add a guest-modifies-text torture test to
`am-tests` before the cache lands, asserting that
`i1 = load_at(X); store(X, i2); execute_at(X)` observes `i2`.

**Exit gate (measurable under the default PERF_MODE=release
pipeline):**

- Wall-clock reductions on dhrystone ≥ 15 %, coremark ≥ 15 %,
  microbench ≥ 10 % vs the committed baseline (`bench.csv` comparison
  via `diff -u` is sufficient evidence).
- `xdb::main` self-time share (which absorbs `decode()` under LTO)
  drops by ≥ 10 pp across all three workloads.
- All existing tests green (`cargo test --workspace`, `make linux`,
  `make linux-2hart`, `make debian`).
- The new text-modifying am-test passes.

A `PERF_MODE=perf` re-profile (line-level attribution via
`--profile perf`) is recommended for authoring confidence but is
**not** required to pass the gate — the release-mode numbers above
are the binding criterion.

### Phase P5 — MMU fast-path inlining & trap slimming  ✅ LANDED 2026-04-16 (trap-slim dropped)
**Tag:** `perf-mmuInline`
**Expected gain on the 2026-04-15 baseline:** −5 % to −10 % wall-clock.
MMU-entry bucket (`access_bus` + `checked_*` + `load`) is 12.1–15.3 %
of self-time, near-flat across workloads and essentially unchanged
from the pre-P1 share — the mutex was downstream of the MMU, not
inside it.
**Priority (post-P1):** #3.
**Risk:** Low.

Audit with `cargo asm` (or just inspect the sampled profile) that the
TLB-hit path in `checked_read` is inlined through to `access_bus`. Add
`#[inline]` / `#[inline(always)]` where LTO isn't picking it up.
Likewise, the zero-pending-trap path through `commit_trap` should be a
tight branch on a single field load, not a full function call.

Exit gate: MMU bucket drops by ≥ 3 pp (from 12–15 % to ≤ 10 %); trap
bucket drops by ≥ 1 pp; no regression on `make linux`/`make debian`.

### Phase P6 — Typed-read bypass for `_platform_memmove`  ✅ LANDED 2026-04-16
**Tag:** `perf-memmove`
**Expected gain on the 2026-04-15 baseline:** −5 % to −10 % wall-clock.
The shim bucket (`_platform_memmove` + `memcpy` PLT + `Bus::read` +
`Bus::write`) is **17–20 %** of self-time combined — materially larger
than the ≤ 4 % quoted against the 2026-04-14 baseline, because the
memmove samples were partially masked by the mutex bucket there.
Recovering half of this via typed reads/writes on
aligned 1/2/4/8-byte accesses is realistic.
**Priority (post-P1):** **#2** — bigger projected win than P5 or P3.
**Risk:** Low-Medium (introduces a small `unsafe` helper).

Earlier drafts targeted `Plic::tick` and `Uart::tick` gating.  Both
are now redundant:

- `Plic::tick` already early-returns when no source has raised.  The
  signal plane (`xemu/xcore/src/device/irq.rs:47`,
  `PlicSignals::drain`) swaps an `AtomicBool` and does zero per-source
  work on the fast path, and `plic.rs:145` consumes the `None` case
  with an unconditional `return`.
- `Uart::tick` *cannot* safely use the proposed "no RX and no TX in
  flight" predicate — `uart.rs:336` must still promote `thre_pending
  → thre_ip` and resync IRQ state on each tick, otherwise the THRE
  interrupt path drops an event.  Any UART fast-path added in a later
  phase must include the full gate condition
  `!thre_pending && rx_buf_empty && rx_fifo_empty && ier_unchanged`
  and re-run `sync_irq` whenever any of those flip.  That work is not
  justified at 0.2 % self-time.

The real remaining device-path cost worth attacking is the
`memmove` shim.  Proposal:

- For 1/2/4/8-byte RAM accesses, bypass the generic memmove in
  `Bus::read`/`Bus::write` and read the guest RAM slice as the
  corresponding `le_bytes` primitive via direct `unsafe` typed reads
  on an aligned pointer.
- Keep the generic memmove path for device regions and any unaligned
  access, gated by an alignment + size check.

Exit gate:

- `_platform_memmove` (or the equivalent symbol on a Linux capture)
  drops below 1.5 % self-time on dhrystone, coremark, microbench.
- `cargo test --workspace` green, `make linux` / `make debian`
  unchanged.
- No new `unsafe` warnings from clippy; the typed-read helper carries
  a `// SAFETY:` comment explicitly covering the alignment
  precondition it relies on.

Defer any further device-tick work until a post-P1 profile actually
shows it in the hot list — the current numbers don't justify more.

### Phase P7 — Multi-hart scaling re-profile
**Tag:** `perf-multiHart`
**Expected gain:** N/A — measurement phase, not optimisation.
**Risk:** None.

Once P1 lands, re-profile `make linux-2hart` and `make debian-2hart`.
This is where `Mutex<Bus>` contention *actually* matters. Likely next
optimisations will be `RwLock<Bus>` for MMIO, per-device locks, or
per-hart shadow TLBs — but the shape of that work depends on what P7's
profile shows, which is why it's its own phase.

---

## 4. Evaluation infrastructure

Landing perf work without a measurement pipeline that gates regressions
is how projects get slower over time. The work split below is modest
but makes future optimisation data-driven.

### 4.1 Perf-smoke bench on CI

- Add a GitHub Actions job that runs `bash scripts/perf/bench.sh --runs 1`
  plus two short sample captures on the default runner; upload the
  whole `docs/perf/<date>/` directory as an artifact per commit.
- Don't gate merges on absolute times (runner variance too high);
  gate on a moving-median baseline JSON refreshed monthly.

### 4.2 `cargo bench` harness for micro-kernels

- Introduce `xcore/benches/` using `criterion 0.5`.
- Start with three micro-benches matching the real profile:
  1. `step_1m_nops` — 1 M `addi x0, x0, 0` instructions, owned bus, no
     MMIO, no traps. Isolates dispatch loop.
  2. `load_store_1m` — tight `lw`/`sw` loop hitting DRAM. Isolates MMU
     + `access_bus`.
  3. `trap_ping` — M-mode ⇄ S-mode ecall ping. Isolates trap cost.
- Each micro-bench declares its expected steady-state IPS; CI fails if
  regression > 10 %.

### 4.3 `perf/REPORT.md` re-baseline cadence

Regenerate after **every Phase P-exit**. The existing scripts already
run end-to-end from `make run`, so this is:

```bash
bash scripts/perf/bench.sh       # writes docs/perf/<today>/data/bench.csv
bash scripts/perf/sample.sh      # writes <today>/data/<workload>.sample.txt
python3 scripts/perf/render.py   # writes <today>/graphics/*.svg
```

Commit the updated `data/` and `graphics/` with the phase's MASTER doc.
Diff in `data/bench.csv` is the receipt.

### 4.4 Optional: samply UI profile on Linux CI

`samply` works without entitlement on Linux runners, unlike on macOS.
Adding a one-shot `samply record -- make linux-2hart` on Ubuntu CI
(artifact: `profile.json.gz`) makes multi-hart perf visible in the
browser profiler with zero manual attach. Low priority — do it after
P7.

### 4.5 Hardware-counter deep dive (off-CI, on-demand)

For one-shot branch-miss / cache-miss understanding of a phase:

- **Linux:** `perf stat -d make linux`
- **macOS:** Instruments.app → Counters template, attached to `xdb` PID

These are GUI / privileged tools and don't belong in auto-run CI, but
should be invoked whenever a phase under-delivers on its expected
gain.

---

## 5. Re-measurement protocol

A phase is **not done** until *all* of:

1. All existing tests pass: `cargo test --workspace`, `make linux`,
   `make debian` (and `-2hart` variants where applicable).
2. `scripts/perf/bench.sh` rerun (3 iters per workload; `--runs 3`).
3. `scripts/perf/sample.sh` rerun for each of the three benches.
4. The per-phase **exit gate** (numeric threshold in §3 above) is met
   with margin ≥ 1 pp on the self-time bucket it targets.
5. REPORT.md deltas committed to the phase's `00_MASTER.md`.

If the exit gate isn't met, the phase goes back to `00_PLAN` as
`01_PLAN` with a new hypothesis. This is the same RLCR/MASTER cadence
used for the arch-refactor tasks — no new process.

---

## 6. Projected aggregate gain

Bands are drawn from each phase's own gain estimate in §3, which in
turn trace to hard bucket percentages in the active report
(`docs/perf/2026-04-15/REPORT.md`).

**Landed (cumulative, user-time vs pre-P1 2026-04-14 baseline):**

| Phase | Landed | Workload | Before | After | Δ user | Cumulative |
|------:|--------|----------|-------:|------:|-------:|-----------:|
| P1    | 2026-04-15 | dhrystone  | 8.09 s | 4.19 s | −48 % | −48 % |
| P1    | 2026-04-15 | coremark   | 14.02 s | 7.37 s | −47 % | −47 % |
| P1    | 2026-04-15 | microbench | 85.82 s | 40.22 s | −53 % | −53 % |
| hotPath (P3+P4+P5+P6) | 2026-04-16 | dhrystone  | 4.19 s | 3.48 s | −16.9 % | **−57 %** |
| hotPath (P3+P4+P5+P6) | 2026-04-16 | coremark   | 7.37 s | 5.82 s | −21.1 % | **−58 %** |
| hotPath (P3+P4+P5+P6) | 2026-04-16 | microbench | 40.22 s | 32.91 s | −18.2 % | **−62 %** |

Actual observations match the **floor-case** end-state projection in
the earlier revision of this table (~0.75× post-P1, ≈ 25 % additional
reduction). The cm workload exceeded it (21 %); dhry + mb sat at the
floor band (17–18 %). The per-phase §A bucket-share gates in §3
composed awkwardly — see `docs/perf/hotPath/00_IMPL.md` §D-001 for
the absolute-sample evidence that justifies "landed" status despite
several §A rows reading "partial" against bucket-share thresholds.

Anything beyond this requires the instruction cache to generalise into
a basic-block cache, or direct-threaded dispatch — a project-scale
change explicitly out of scope for this roadmap, and which would
belong to a future Phase 11.

---

## 7. What we explicitly deprioritise

- **VGA / framebuffer devices.** Zero contribution to the hot path of
  the three benchmarks; valuable product-wise but orthogonal to perf.
- **Heap / allocator tuning (jemalloc, mimalloc).** RSS is flat;
  allocations are not the bottleneck.
- **Micro-level ISA dispatch tricks** (threaded code, tail-calls).
  With the mutex gone, `xdb::main` has *already* passed 40 % of
  self-time (40.4 / 46.8 / 44.7 %), so the condition the pre-P1
  revision of this paragraph set is nominally met — but threaded
  dispatch is still the wrong next step. Land P4's icache first; it
  attacks the same bucket with much smaller risk and no architectural
  lock-in. Revisit threaded dispatch only if post-P4 profiles still
  show `xdb::main` above 30 %.
- **Difftest-mode performance.** Difftest is off by default and
  feature-gated. Measure once post-roadmap, don't invest ahead of
  time.

---

## 8. References

- [`docs/perf/2026-04-16/`](./perf/2026-04-16/) — post-hotPath data
  (REPORT pending per G-002).
- [`docs/perf/2026-04-15/REPORT.md`](./perf/2026-04-15/REPORT.md) — the
  post-P1 baseline; still the bucket-share source of truth for §3.
- [`docs/perf/2026-04-14/REPORT.md`](./perf/2026-04-14/REPORT.md) — the
  pre-P1 baseline, retained for cumulative delta comparison.
- [`docs/perf/busFastPath/`](./perf/busFastPath/) — P1 iteration
  artefacts (rounds 00 → 03 + `00_IMPL.md`).
- [`docs/perf/hotPath/`](./perf/hotPath/) — P3+P4+P5+P6 bundle iteration
  artefacts (rounds 00 → 04 + `00_IMPL.md`).
- [`docs/perf/README.md`](./perf/README.md) — index and quickstart for
  capturing a new dated run.
- [`docs/DEV.md`](./DEV.md) — project development status / phase
  numbering.
- Hot-path source files called out in §2:
  `xemu/xcore/src/cpu/mod.rs`,
  `xemu/xcore/src/arch/riscv/device/aclint/mtimer.rs`,
  `xemu/xcore/src/arch/riscv/cpu/mm.rs`,
  `xemu/xcore/src/arch/riscv/cpu/trap/handler.rs`.
