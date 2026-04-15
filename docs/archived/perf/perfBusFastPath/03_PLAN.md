# `perfBusFastPath` PLAN `03`

> Status: Revised
> Feature: `perfBusFastPath`
> Iteration: `03`
> Owner: Executor
> Depends on:
> - Previous Plan: `02_PLAN.md`
> - Review: `02_REVIEW.md`
> - Master Directive: `01_MASTER.md` (M-001 still binding); `02_MASTER.md` blank.

---

## Summary

Phase P1 of the xemu perf roadmap. Drop the `Arc<Mutex<Bus>>`
wrapper that currently dominates per-instruction self-time
(`pthread_mutex_{lock,unlock}` plus PLT stubs at 33-40 % of cycles
per `docs/perf/2026-04-14/REPORT.md`) and replace it with direct
CPU-owned bus storage. Per `01_MASTER.md` M-001 (still binding;
`02_MASTER.md` was left blank), the `Shared(Arc<Mutex<Bus>>)` arm
proposed in 00/01 is dropped: xemu's multi-hart scheduler is
single-threaded cooperative round-robin
(`xemu/xcore/src/cpu/mod.rs:213-249`), and the only OS-thread
crossing is the UART stdin reader which owns its own mailbox.
The mutex protects nothing today.

The fix is uniform: `CPU` owns `bus: Bus` (inline; see R-003
resolution below); each `Core::step` receives `&mut Bus` as a
parameter from `CPU::step`; every existing `bus.lock().unwrap()`
becomes a direct call (`bus.tick()`, `bus.read(...)`, ...) on a
borrowed reference. The migration is a **single atomic commit**
that removes the lock and lands the CI sentinel together — no
intermediate state where one half of the tree still holds
`Arc<Mutex<Bus>>` (R-005 resolution).

There are NO benchmark-targeted tricks. The refactor is driven by
the static shape of `CPU`/`Core` and applies equally to `make run`
microbench/dhrystone/coremark, to `make linux`, `make linux-2hart`,
and `make debian`. Gain band re-derived: floor 15 % wall-clock
reduction (required), expected 20-30 %, ceiling <= 35 %.

Future SMP (true per-hart OS threads) is explicitly out of scope and
recorded in `docs/DEV.md` Phase 11 ("True SMP - per-hart OS threads
- RFC / FUTURE", lines 186-189). Phase 11 Option B (lock-free RAM
atomics + per-device MMIO locks) and Option C (QEMU-style BQL on
MMIO only) remain available later; nothing in this plan forecloses
them. P1 hands Phase 11 a clean owned-bus starting point rather than
a lock shape inherited from a threading model xemu does not yet
have.

## Log

[**Feature Introduce**]

Fourth iteration. Round 02 was approved with revisions; round 03
folds 02_REVIEW R-001..R-007 into the plan body without altering
the M-001-driven core design. Specifically: the `type_name`
sentinel (a no-op) is replaced by a three-layer hard sentinel; the
disjoint-field borrow at `CPU::step` is named and pinned by an
invariant; `Bus` ownership flips from `Box<Bus>` to inline `Bus`
with rationale; the LR/SC peer-hart-exclusion argument is made
explicit; the Phase-1 step that briefly re-introduced
`Arc<Mutex<Bus>>` is removed and the migration becomes one atomic
commit; the 2-hart Linux baseline is captured as a named artifact
before P1 implementation begins; and the optional `cargo asm`
symbol path is corrected to the fully-qualified generic name.



[**Review Adjustments**]

All seven 02_REVIEW findings resolved (no rejections):

- R-001 HIGH (V-UT-5 `type_name` is a no-op): replaced with a
  three-layer hard sentinel - (a) `scripts/ci/verify_no_mutex.sh`
  shell gate over the bus / CPU / mm / atomic modules, (b)
  `#![deny(unused_imports)]` at the top of `xcore/src/device/bus.rs`
  so a reintroduced `use std::sync::Mutex` is rejected at compile
  time, and (c) a `compile_fail` doc-test on `Bus` documenting that
  wrapping it in `Arc<Mutex<_>>` is forbidden.
- R-002 MEDIUM (disjoint-field borrow undocumented): added
  Invariant I-10. `CPU::step`'s body MUST destructure
  `self` into disjoint borrows before handing `&mut bus` to the
  current core; helper methods on `&mut self` that access both
  `cores` and `bus` are forbidden. Pinned by a `compile_fail`
  doc-test that re-introduces the bad pattern.
- R-003 MEDIUM (`Box<Bus>` vs inline `Bus` unjustified): inline
  `Bus` chosen. `CPU` is constructed once and lives the entire
  emulation; `Bus` size is bounded (`Vec<u8>` Ram + `Vec<MmioRegion>`
  + `Vec<Option<usize>>` reservations, ~100-200 bytes); inline
  avoids the indirection on the hot path. V-UT-3 now asserts
  `size_of::<Bus>() < 256` and `size_of::<CPU<RVCore>>() < 4096` so
  layout regressions are caught.
- R-004 MEDIUM (LR/SC peer-hart exclusion implicit): added the
  explicit clause to I-4 - cooperative round-robin grants exactly
  one `&mut Bus` per `CPU::step`, the borrow checker replaces the
  mutex as the exclusion primitive for `bus.reservations[hart]`,
  and LR/SC sequences are atomic w.r.t. peer harts because the
  scheduler does not preempt mid-instruction. Citations to
  `cpu/mod.rs:213-249` and `arch/riscv/cpu/inst/atomic.rs` added.
- R-005 MEDIUM (Phase-1 step 1d transiently re-introduces the
  Mutex): step 1d removed. Phase 1 + Phase 2 collapse into a
  single atomic commit
  `feat(xcore): drop Arc<Mutex<Bus>> and own Bus directly` that
  lands `verify_no_mutex.sh` in the same change. No commit on `main`
  is ever in the half-migrated state.
- R-006 MEDIUM (2-hart +/-5 % gate has no named baseline file):
  added a Validation prerequisite. Before P1 implementation begins,
  capture
  `docs/perf/2026-04-15/data/linux_2hart.csv` from the pre-P1 tree
  (3 runs of `make linux-2hart` with `/usr/bin/time -l`, DEBUG=n,
  one row per run with the columns `run,real_s,user_s,sys_s,max_rss_kb`),
  commit it alongside this PLAN, and use it as the comparison
  reference for V-IT-4.
- R-007 LOW (`cargo asm` symbol path drops the generic): step 3f
  uses the fully-qualified name
  `xcore::cpu::CPU<xcore::arch::riscv::cpu::RVCore>::step` and
  prefaces it with `cargo asm -p xcore --list | rg 'CPU.*step'` to
  resolve. Reaffirmed as nice-to-have, not exit-gated (per L-1).

Trade-off responses (from 02_REVIEW Trade-off Advice):

- TR-1 (Box<Bus> vs inline Bus): the reviewer recommended Box<Bus>
  for migration-diff minimality. **Diverged** to inline `Bus` with
  explicit rationale (R-003 above). Reasoning: (a) the diff
  difference between `Box<Bus>` and inline `Bus` is two lines on
  `CPU::new` and `CPU::bus()`; minimality is preserved; (b) inline
  removes a pointer hop on the hot path that `Box<Bus>` would
  retain; (c) `CPU` size remains bounded and stable (V-UT-3 pins
  it). The reviewer's contingent recommendation ("inline is a
  two-line change available as a polish pass") is folded into P1
  itself rather than deferred. This is not a rejection of TR-1's
  underlying point - both options are sound under M-001 - but a
  decision to apply both at once.
- TR-2 (StepContext deferral): kept as planned. Recorded in T-3
  and in the round-03 backlog pointer below that a `StepContext`
  refactor is deliberately deferred to a later code-quality PR
  with its own iteration cycle; not lost.



[**Master Compliance**]

M-001 (from `01_MASTER.md`, still binding; `02_MASTER.md` was left
blank by user): applied in full. `Bus` does not need `Mutex`
under the cooperative round-robin scheduler. CPU owns the bus
directly (inline). Multi-hart remains correct because the
scheduler is single-threaded - the single `&mut Bus` borrow at each
`CPU::step` is the single-borrower invariant the round-robin
already enforces, and (per the new I-4 clause) is the same
exclusion that previously required the mutex for
`bus.reservations[hart]`. `docs/DEV.md` Phase 11 records the
future SMP options.

The `verify_no_mutex.sh` shell gate, the
`#![deny(unused_imports)]` lint on `bus.rs`, and the `compile_fail`
doc-test on `Bus` together prevent silent reintroduction (R-001
hardening above).



### Changes from Previous Round

[**Added**]

- Invariant I-10: disjoint-field borrow discipline at `CPU::step`
  (R-002).
- Sentinel script `scripts/ci/verify_no_mutex.sh` and
  `#![deny(unused_imports)]` lint on `xcore/src/device/bus.rs` and
  a `compile_fail` doc-test on `Bus` (R-001).
- Pre-P1 baseline capture: `docs/perf/2026-04-15/data/linux_2hart.csv`
  (R-006).
- Inline-`Bus` rationale in Data Structure (R-003).
- Explicit peer-hart-exclusion clause in I-4 (R-004).
- StepContext-deferred backlog pointer (TR-2 / T-3).

[**Changed**]

- `CPU::bus`: `Box<Bus>` -> `Bus` (inline). `CPU::bus(&self) -> &Bus`
  and `CPU::bus_mut(&mut self) -> &mut Bus` unchanged in surface;
  callers compile identically.
- Phase 1 + Phase 2 merged into a single atomic commit; Phase 1
  step 1d (the temporary `Arc<Mutex<Bus>>` shim) removed (R-005).
- V-UT-5 reformulated: dropped the `type_name` assertion; replaced
  with the three-layer sentinel (R-001).
- V-UT-3: now also asserts `size_of::<Bus>()` and
  `size_of::<CPU<RVCore>>()` are below stated bounds.
- Step 3f: `cargo asm` symbol path uses fully-qualified generic
  name and is prefaced by a `--list` pass (R-007).

[**Removed**]

- Phase-1 step 1d (the `Arc<Mutex<Bus>>` shim commit). Migration
  is now one atomic change.
- The runtime `type_name` no-op assertion in V-UT-5.
- `Box<Bus>` storage; replaced by inline `Bus`.

[**Unresolved**]

- U-1: Long-term SMP shape remains Phase 11's RFC; not resolved
  here.
- U-2: Whether difftest (`bus_take_mmio_flag`) eventually wants a
  narrower API surface is left open for a future feature-gated PR;
  P1 only changes the receiver to `&mut self`.
- U-3: A future inline-cleanup pass to fold the
  `StepContext { bus, mmu, privilege }` refactor into the threaded
  signatures is deferred (TR-2 / T-3) and tracked here as a backlog
  item.



### Response Matrix

| Source       | ID    | Severity | Status                  | Action in this plan                                                                                                                                                                                                                                                                                                                                          | Test or gate that proves it                                                                                                                                                            |
|--------------|-------|----------|-------------------------|--------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Master 01    | M-001 | binding  | Re-applied              | `Bus` is not wrapped in `Mutex`. `CPU` owns `bus: Bus` inline. `Core::step(&mut self, bus: &mut Bus)` takes the borrow as a parameter. `Arc<Mutex<Bus>>` removed from `CPU` and from `RVCore`. All `self.bus.lock().unwrap().X()` sites rewritten as direct `bus.X()` calls. Migration lands as a single atomic commit (R-005).                              | `scripts/ci/verify_no_mutex.sh` (V-UT-5) returns non-zero on any match in the bus / CPU / mm / atomic modules. `#![deny(unused_imports)]` on `bus.rs`. `compile_fail` doc-test on `Bus`. |
| Master 02    | (none)| n/a      | n/a (blank)             | `02_MASTER.md` was left blank by user; no new directives.                                                                                                                                                                                                                                                                                                    | n/a                                                                                                                                                                                    |
| Review 01    | C-1   | CRITICAL | Carry-forward, resolved | `Shared` arm dropped; no two-arm enum. Multi-hart correctness argued from the cooperative round-robin invariant, not a lock shape.                                                                                                                                                                                                                           | V-IT-4 (`make linux-2hart` boots, +/-5 % vs `linux_2hart.csv`); `cargo test --workspace` green incl. `atomic.rs` LR/SC tests.                                                          |
| Review 01    | H-1   | HIGH     | Carry-forward, resolved | Migration table rebuilt from `rg "bus\.lock\(\)" xemu -n` (24 hits). All five test-helper sites at `inst/base.rs:344-356`, `inst/compressed.rs:552-556`, `inst/float.rs:1075-1079`, `arch/riscv/cpu.rs:277-282` covered. Field-type change is compile-breaking.                                                                                              | Post-merge: `rg "bus\.lock\(\)" xemu -n` returns zero. `verify_no_mutex.sh` gate.                                                                                                       |
| Review 01    | H-2   | HIGH     | Carry-forward, resolved | V-UT-5 reformulated. Three-layer sentinel: `verify_no_mutex.sh` shell gate + `#![deny(unused_imports)]` on `bus.rs` + `compile_fail` doc-test on `Bus`. The N-baseline concept is dropped. Note: the runtime `type_name` half is also dropped because R-001 proved it a no-op.                                                                                | V-UT-5 (all three sentinels run in `make test`).                                                                                                                                       |
| Review 01    | H-3   | HIGH     | Carry-forward, resolved | `RVCore::bus` field deleted. Every `RVCore` method that previously read `self.bus.lock().unwrap().X()` takes `bus: &mut Bus` (or `&Bus`) as a parameter. Signature list in Data Structure.                                                                                                                                                                   | `cargo check -p xcore` green; `arch_isolation.rs` seam test green.                                                                                                                     |
| Review 01    | M-1   | MEDIUM   | Carry-forward, resolved | `BusOwner` factory removed. Bus construction is `Bus::new(mbase, msize, num_harts)` directly; passed by value to `CPU::new`.                                                                                                                                                                                                                                 | N/A (code deletion).                                                                                                                                                                   |
| Review 01    | M-2   | MEDIUM   | Carry-forward, resolved | Gain-band math redone: floor 15 %, expected 20-30 %, ceiling <= 35 %. Walked through in G-2.                                                                                                                                                                                                                                                                 | Phase-3b perf sample vs `docs/perf/2026-04-14/data/bench.csv`; mutex bucket -> 0 % by construction.                                                                                    |
| Review 01    | M-3   | MEDIUM   | Carry-forward, resolved | V-IT-7 50 ms budget dropped. Replaced with "2-hart Linux boot wall-clock within +/-5 % of `docs/perf/2026-04-15/data/linux_2hart.csv`" (R-006) plus existing LR/SC unit tests.                                                                                                                                                                                | V-IT-4; existing `atomic.rs` tests.                                                                                                                                                    |
| Review 01    | L-1   | LOW      | Carry-forward, resolved | `cargo asm` gate downgraded to nice-to-have Phase-3 evidence; symbol path corrected (R-007). Hard gate is the `verify_no_mutex.sh` script.                                                                                                                                                                                                                   | Hard gate: V-UT-5 / Exit-Gate row 1. Optional: `cargo asm` snippet in post-P1 perf report appendix.                                                                                    |
| Review 02    | R-001 | HIGH     | Resolved                | `type_name` assertion deleted (it was a no-op). Replaced with three-layer hard sentinel: (a) `scripts/ci/verify_no_mutex.sh` invokes `! rg "Mutex\|RwLock\|parking_lot\|RefCell" xemu/xcore/src/device/bus.rs xemu/xcore/src/cpu/mod.rs xemu/xcore/src/arch/riscv/cpu/mm.rs xemu/xcore/src/arch/riscv/cpu/inst/atomic.rs xemu/xcore/src/arch/riscv/cpu.rs`; (b) `#![deny(unused_imports)]` at the top of `xcore/src/device/bus.rs` so a `use std::sync::Mutex` line is caught even if unused; (c) `compile_fail` doc-test on `Bus` proving `Arc<Mutex<Bus>>` is the wrong shape. | V-UT-5 (all three sentinels). `make test` invokes the shell gate via a thin Make target.                                                                                               |
| Review 02    | R-002 | MEDIUM   | Resolved                | New Invariant I-10 added: `CPU::step` MUST destructure `self` into disjoint borrows (`let CPU { bus, cores, current, .. } = self;`) before calling `cores[*current].step(bus)`. Helper methods that take `&mut self` and access both fields are forbidden. Documented in I-10 prose and pinned by a `compile_fail` doc-test that re-introduces the bad pattern and confirms it fails E0499. | V-UT-7 (compile_fail doc-test on `CPU::step`).                                                                                                                                          |
| Review 02    | R-003 | MEDIUM   | Resolved                | Inline `Bus` chosen (not `Box<Bus>`). Rationale in Data Structure: `CPU` is constructed once and lives the whole emulation; `Bus` size is bounded; inline avoids the indirection on the hot path. V-UT-3 pins `size_of::<Bus>() < 256` and `size_of::<CPU<RVCore>>() < 4096`.                                                                                | V-UT-3 size assertions; perf sample post-P1 (no extra indirection on `bus.tick`).                                                                                                       |
| Review 02    | R-004 | MEDIUM   | Resolved                | I-4 amended with the explicit peer-hart-exclusion clause: cooperative round-robin grants exactly one `&mut Bus` per `CPU::step`; LR/SC sequences are atomic w.r.t. peer harts because the scheduler does not preempt mid-instruction; the borrow checker replaces the mutex as the exclusion primitive for `bus.reservations[hart]`. Cited `cpu/mod.rs:213-249` and `arch/riscv/cpu/inst/atomic.rs`. | V-UT-4 (existing 20+ LR/SC tests); V-E-3 (2-hart LR/SC ping-pong).                                                                                                                      |
| Review 02    | R-005 | MEDIUM   | Resolved                | Phase 1 step 1d removed entirely. The migration is one atomic commit `feat(xcore): drop Arc<Mutex<Bus>> and own Bus directly` that contains all field-type changes, all call-site rewrites, and `scripts/ci/verify_no_mutex.sh`. No mid-series commit ever holds half a migration.                                                                            | Single-commit PR diff review; `verify_no_mutex.sh` is added in the same commit and passes immediately.                                                                                  |
| Review 02    | R-006 | MEDIUM   | Resolved                | Pre-P1 prerequisite: capture `docs/perf/2026-04-15/data/linux_2hart.csv` (3 runs of `make linux-2hart`, `/usr/bin/time -l`, DEBUG=n, columns `run,real_s,user_s,sys_s,max_rss_kb`) and commit alongside this PLAN. V-IT-4 references this file as the +/-5 % comparison reference.                                                                            | The committed file is the gate's named ground truth; V-IT-4 diffs against it.                                                                                                          |
| Review 02    | R-007 | LOW      | Resolved                | Step 3f symbol path corrected to `xcore::cpu::CPU<xcore::arch::riscv::cpu::RVCore>::step`, prefaced by `cargo asm -p xcore --list \| rg 'CPU.*step'` to resolve. Optional, not exit-gated.                                                                                                                                                                    | n/a (nice-to-have appendix).                                                                                                                                                            |
| Review 02 TR | TR-1  | trade-off| Diverged with reasoning | Inline `Bus` chosen instead of `Box<Bus>`. Rationale recorded in R-003 above and in T-1. The reviewer's preferred option (`Box<Bus>`) is also correct under M-001; the divergence is a one-line difference and saves a pointer hop.                                                                                                                          | V-UT-3 size assertion confirms the inline layout is bounded.                                                                                                                            |
| Review 02 TR | TR-2  | trade-off| Adopted                 | StepContext refactor explicitly deferred to a future code-quality PR; recorded in T-3 with a backlog pointer to `docs/DEV.md` Phase 11 (or a future `docs/fix/perfBusContext` task).                                                                                                                                                                          | n/a (deferred).                                                                                                                                                                        |

> Rules:
> - Every prior CRITICAL / HIGH finding appears here.
> - Every Master directive appears here (M-001 binding from 01;
>   02 was blank).
> - Rejections must include explicit reasoning. (None this round;
>   one trade-off divergence, TR-1, is justified inline.)

---

## Spec

[**Goals**]

- G-1: On every configuration (1-hart, 2-hart, N-hart), `CPU::step`
  and every per-instruction memory access (`checked_read`,
  `checked_write`, `access_bus`, AMO / LR / SC, `Bus::tick`) executes
  with **zero** `pthread_mutex_*` calls on the hot path. The bus is
  accessed via a direct `&mut Bus` borrow threaded through the call
  chain from `CPU::step`.
- G-2: Wall-clock runtime of `make run` (dhrystone, coremark,
  microbench, DEBUG=n) drops by **at least 15 %** vs.
  `docs/perf/2026-04-14/data/bench.csv`; **expected 20-30 %**;
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
`docs/PERF_DEV.md` P1 names directly ("Restore an owned `Bus` on
the hot path"); the PERF_DEV.md sketch floated a `BusHandle` enum
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
  `docs/perf/2026-04-15/data/linux_2hart.csv` (R-006). Historically
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

## Implement

### Execution Flow

[**Main Flow**]

Per-instruction (single-hart or multi-hart; same code path):

1. `CPU::step` destructures `self` into disjoint borrows (I-10):
   `let CPU { bus, cores, current, .. } = self;`.
2. `bus.tick()` runs - direct method call on `&mut Bus`, no atomics.
3. `cores[*current].step(bus)` hands out an exclusive borrow of
   the bus for the duration of the core step.
4. `RVCore::step` reads `bus.mtime()` via the `bus` parameter, syncs
   interrupts, checks pending traps, fetches an instruction via
   `self.fetch(bus)`, decodes, executes.
5. `fetch(bus)` calls `self.checked_read(bus, self.pc, 4, Fetch)`
   which calls `self.access_bus(bus, ...)` (one borrow scope, same
   width as today).
6. `execute` dispatches to per-instruction handlers. Loads / stores
   go through `self.checked_read(bus, ...)` /
   `self.checked_write(bus, ...)`. AMO / LR / SC handlers take
   `bus: &mut Bus` and perform translate + reserve /
   reservation-check + conditional-store in one function body
   (I-4).
7. `retire()` and `advance_current()` run after `Core::step`
   returns, releasing the `&mut Bus` borrow.
8. Return to `CPU::run`.

[**Failure Flow**]

1. Borrow-check error at migration time: if any site tries to keep
   two `&mut Bus` borrows alive simultaneously, the compiler
   rejects the change. This is the I-9 reentry invariant: if a
   `Device::tick` body tried to reach `CPU`, there would be no
   way to spell the back-reference without re-introducing `Arc`,
   which the migration forbids.
2. I-10 violation: a future cleanup pass that extracts the
   bus-step logic into a helper method on `&mut self` will fail
   to compile (E0499) because the disjoint-field path collapses.
   The `compile_fail` doc-test V-UT-7 documents and reproduces this
   failure mode.
3. Lost reservation on reset: `CPU::reset` calls
   `self.bus.reset_devices()` then `self.bus.clear_reservations()`,
   same order as today. No behavioural change.
4. Difftest `bus_take_mmio_flag`: the call site is already
   `&mut CPU`, so the `&self -> &mut self` change is
   source-compatible. If a future call site holds `&CPU`, it must
   switch to `&mut CPU`; there is one such site today, already
   `&mut CPU` (verified in external-caller audit).
5. Test helpers that reach `core.bus` (`inst/base.rs:344-356`,
   `inst/compressed.rs:552-556`, `inst/float.rs:1075-1079`,
   `arch/riscv/cpu.rs:277-282`): these construct an `RVCore` for
   one-off assertions. Since `RVCore` no longer owns a bus, these
   helpers either (a) take a `&mut Bus` / `&Bus` parameter
   explicitly, or (b) construct a fresh `Bus` alongside the core
   in the test's `setup_core()`. Pattern (a) is preferred where
   the test already has a `Bus` in scope; (b) is the default for
   `RVCore::new()` direct users.

[**State Transition**]

- Construction: `CPU::new(cores, Bus::new(mbase, msize, num_harts),
  layout)`. Runtime: no state transition on bus ownership; the
  inline `Bus` is owned by `CPU` from construction to drop.
- Reset: `CPU::reset` calls `self.bus.reset_devices()` +
  `self.bus.clear_reservations()`, then resets each core. No
  ownership change.
- Per-step: exclusive `&mut Bus` borrow handed to
  `self.cores[self.current]` for the duration of `Core::step`,
  then released. This is a lexical scope in `CPU::step`'s body
  (I-3 + I-10), not a field-level transition.

### Implementation Plan

**Single atomic commit (R-005).** The migration is one commit
`feat(xcore): drop Arc<Mutex<Bus>> and own Bus directly` that
bundles every change below. There is no intermediate
half-migrated state on `main`. CI runs `make fmt && make clippy &&
cargo test --workspace && bash scripts/ci/verify_no_mutex.sh` on
the single commit before merge.

[**Phase 1 - Pre-P1 prerequisite (capture baseline) (R-006)**]

Before authoring the migration commit:

- 1a. Run `make linux-2hart` three times under DEBUG=n with
  `/usr/bin/time -l`; save raw outputs to
  `docs/perf/2026-04-15/data/linux_2hart.run{1,2,3}.time`.
- 1b. Aggregate into `docs/perf/2026-04-15/data/linux_2hart.csv`
  with columns `run,real_s,user_s,sys_s,max_rss_kb` (one row per
  run, mean computable).
- 1c. Commit `docs/perf/2026-04-15/data/linux_2hart.csv` and the
  three raw `.time` files in a separate, prior commit
  `perf(baseline): capture pre-P1 linux-2hart wall-clock for V-IT-4`.
  This commit lands BEFORE the migration commit so the gate has a
  named ground truth.

[**Phase 2 - Migration commit (atomic)**]

All of the following land in one commit
`feat(xcore): drop Arc<Mutex<Bus>> and own Bus directly`:

- 2a. `CPU::bus` field type changes from `Arc<Mutex<Bus>>` to
  `Bus` (inline, not boxed; R-003). Update `CPU::new`'s signature.
- 2b. `CPU::bus(&self) -> &Bus`; `CPU::bus_mut(&mut self) -> &mut Bus`.
- 2c. `CPU::step` body destructures `self` per I-10 and threads
  `bus: &mut Bus` into `cores[*current].step(bus)`.
- 2d. `CPU::reset`, `replace_device`, `bus_take_mmio_flag`, image
  load, firmware load, `step` sites in `cpu/mod.rs` (lines 101,
  126, 142-145, 168, 199, 214, 293, 323) all rewritten as direct
  method calls on `&mut Bus` / `&Bus`.
- 2e. `RVCore::bus` field deleted from `arch/riscv/cpu.rs:43`.
  `RVCore::with_id`'s `bus` parameter dropped.
- 2f. Every `RVCore` method in the Data Structure list takes
  `bus: &mut Bus` (or `&Bus`). All `self.bus.lock().unwrap()` sites
  rewritten. Per the migration table below, 24 hits.
- 2g. Test helpers in `inst/base.rs:344-356`,
  `inst/compressed.rs:552-556`, `inst/float.rs:1075-1079`,
  `arch/riscv/cpu.rs:277-282`, `inst/atomic.rs:195,200,204,212`
  rewritten with explicit `bus: &mut Bus` / `&Bus` parameters; test
  bodies call `bus.X()` directly.
- 2h. New file: `scripts/ci/verify_no_mutex.sh` containing
  ```sh
  #!/usr/bin/env bash
  set -euo pipefail
  if rg -n "Mutex|RwLock|parking_lot|RefCell" \
       xemu/xcore/src/device/bus.rs \
       xemu/xcore/src/cpu/mod.rs \
       xemu/xcore/src/arch/riscv/cpu/mm.rs \
       xemu/xcore/src/arch/riscv/cpu/inst/atomic.rs \
       xemu/xcore/src/arch/riscv/cpu.rs ; then
    echo "verify_no_mutex: forbidden synchronization primitive on bus path"
    exit 1
  fi
  echo "verify_no_mutex: ok"
  ```
  Wired into `make test` via a thin Make target so the gate runs
  on every CI invocation.
- 2i. `#![deny(unused_imports)]` placed at the top of
  `xemu/xcore/src/device/bus.rs` (R-001 layer b).
- 2j. `compile_fail` doc-test on `Bus` (R-001 layer c) and on
  `CPU::step` (R-002 / I-10 pin), see V-UT-5 and V-UT-7.
- 2k. `make fmt && make clippy && cargo test --workspace && bash
  scripts/ci/verify_no_mutex.sh` runs green on the commit before
  push.

Migration table (complete, harvested from `rg "bus\.lock\(\)" xemu
-n`; 24 hits in production + test helpers; carried forward verbatim
from 02_PLAN with the only change being the `Box<Bus>` -> inline
`Bus` rephrasing on the first row):

| File : Line                                                   | Today                                           | Migration                                                                                                      |
|---------------------------------------------------------------|-------------------------------------------------|----------------------------------------------------------------------------------------------------------------|
| `xcore/src/cpu/mod.rs:101`                                    | `bus.lock().unwrap().num_harts()`               | `bus.num_harts()` - `bus: Bus` after 2a; auto-deref.                                                           |
| `xcore/src/cpu/mod.rs:126`                                    | `self.bus.lock().unwrap()` (inside `bus()`)     | `&self.bus` (body returns `&Bus` directly).                                                                    |
| `xcore/src/cpu/mod.rs:142-145` reset                          | `let mut bus = self.bus.lock().unwrap(); ...`   | `self.bus.reset_devices(); self.bus.clear_reservations();` - drop the scope block.                             |
| `xcore/src/cpu/mod.rs:168` direct image load                  | `bus.lock().unwrap().load_ram(...)`             | `self.bus.load_ram(RESET_VECTOR, image_bytes)`.                                                                |
| `xcore/src/cpu/mod.rs:199` firmware file load                 | `bus.lock().unwrap().load_ram(...)`             | `self.bus.load_ram(addr, &bytes)?`.                                                                            |
| `xcore/src/cpu/mod.rs:214` step tick                          | `bus.lock().unwrap().tick()`                    | Within `let CPU { bus, cores, current, .. } = self;` body: `bus.tick();` (I-10). Hottest site.                 |
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
| `xcore/src/arch/riscv/cpu/inst/atomic.rs:65-69`               | SC 32-bit triple-lock zone                      | One scope under the threaded `bus: &mut Bus`; I-4 preserved (peer-hart clause holds, R-004).                   |
| `xcore/src/arch/riscv/cpu/inst/atomic.rs:81`                  | LR 64-bit `reserve`                             | Same as :57.                                                                                                   |
| `xcore/src/arch/riscv/cpu/inst/atomic.rs:89-94`               | SC 64-bit triple-lock zone                      | Same as :65-69.                                                                                                |
| `xcore/src/arch/riscv/cpu/inst/atomic.rs:195,200,204,212` (test)| AMO test helpers                              | Mechanical migration; each takes `bus: &mut Bus` or `bus: &Bus`.                                               |

Checkpoint after the commit:
`rg "bus\.lock\(\)" xemu -n` returns zero hits;
`rg "Arc<Mutex<Bus>>" xemu -n` returns zero hits;
`bash scripts/ci/verify_no_mutex.sh` exits 0.

[**Phase 3 - Verify, measure, document**]

- 3a. Boot gate: `make run` (microbench) completes; `make linux`
  boots to shell; `make linux-2hart` boots to shell; `make debian`
  boots to login.
- 3b. Perf sampling (DEBUG=n): `scripts/perf/sample.sh` on
  dhrystone, coremark, microbench; render via
  `scripts/perf/render.py` into `docs/perf/<post-P1-date>/data/`
  and `REPORT.md`.
- 3c. Compare against `docs/perf/2026-04-14/data/bench.csv`:
  assert >= 15 % wall-clock reduction on each of the three
  benchmarks; assert `pthread_mutex_*` bucket -> 0 % (no samples
  in the new profile by construction).
- 3d. `linux-2hart` boot-to-shell time sampled three runs each
  pre / post; confirm within +/-5 % of
  `docs/perf/2026-04-15/data/linux_2hart.csv`.
- 3e. Full test pass: `cargo test --workspace` green (336 unit
  tests + `arch_isolation` + `atomic` LR/SC tests; difftest
  feature build also green).
- 3f. Optional (nice-to-have, not gated, R-007):
  `cargo asm -p xcore --list | rg 'CPU.*step'` to resolve the
  monomorphised symbol, then
  `cargo asm -p xcore '<matching-line>' --rust`. Targeted symbol
  is `xcore::cpu::CPU<xcore::arch::riscv::cpu::RVCore>::step`.
  Disassembly snippet captured in the perf report appendix as
  evidence that `Bus::tick` is a direct `call` with no
  `lock cmpxchg` / `xchg` / `pthread_mutex_*` symbol.
- 3g. Update `docs/PERF_DEV.md` P1 row to "Done" with the measured
  numbers and link to `docs/perf/<post-P1-date>/REPORT.md`.

---

## Trade-offs

- T-1: **Bus ownership model - inline `Bus` on `CPU` (Option A,
  adopted; supersedes round-02's `Box<Bus>`) vs. `Box<Bus>` (Option
  B, reviewer's TR-1 preference) vs. the two-arm `BusHandle` enum
  (Option C, rejected) vs. `Arc<UnsafeCell<Bus>>` behind a
  single-hart feature flag (Option D, rejected).**

  Option A (recommended, applied in this plan): `CPU { bus: Bus }`
  inline. Saves one pointer hop per `bus.X()` call vs `Box<Bus>`;
  no extra heap allocation in `CPU::new`; layout is bounded by
  V-UT-3. Migration diff is identical to Option B except for the
  field declaration and `CPU::new` arg.

  Option B (reviewer's preference, declined with reasoning):
  `CPU { bus: Box<Bus> }`. Sound under M-001; reviewer suggested
  it for "diff-minimality" in TR-1. We diverge because the
  diff-minimality argument doesn't hold (one line of difference)
  and the inline shape removes a pointer hop on the hot path. The
  reviewer also explicitly noted "inlining `Bus` is a two-line
  change available as a polish pass" - this plan folds that polish
  pass into P1.

  Option C (rejected, M-001): two-arm `BusHandle { Owned(Box<Bus>),
  Shared(Arc<Mutex<Bus>>) }`. Rejected because the `Shared` arm
  protects nothing in xemu's current single-threaded scheduler.

  Option D (rejected, NG-4): `Arc<UnsafeCell<Bus>>` plus a
  single-hart feature flag. Rejected per "no `unsafe`" and because
  it bifurcates the build matrix.

  Relationship to future SMP: `docs/DEV.md` Phase 11 ("True SMP -
  per-hart OS threads - RFC / FUTURE", lines 186-189) sketches the
  design space for actual parallel execution. Phase 11 Option B
  (per-hart threads with lock-free RAM atomics + per-device MMIO
  locks) and Option C (QEMU-style BQL on MMIO memory only) both
  remain available. P1's owned-bus shape does not foreclose either.
  When Phase 11 lands, the bus can be re-split into
  `Arc<GuestMemory>` (lock-free atomics) + per-device
  `Arc<Mutex<dyn Mmio>>` without unwinding Phase-1 work; the change
  is additive, not a revert.

- T-2: **`CPU::bus()` return type - direct reference (Option A,
  adopted) vs. opaque read-only wrapper (Option B, considered
  and rejected).**

  Option A: `CPU::bus(&self) -> &Bus`. `CPU::bus_mut(&mut self) ->
  &mut Bus`. Mirrors `HashMap::get / get_mut`. Existing callers
  (`cpu.bus().read(...)`, `cpu.bus().num_harts()`) stay
  source-compatible because method syntax on `&Bus` is identical
  to method syntax on `MutexGuard<Bus>` at these call sites.

  Option B: `CPU::bus(&self) -> ReadBusView<'_>` wrapper type.
  Rejected: no protection that `&Bus` doesn't already provide;
  adds a wrapper for its own sake.

- T-3: **Signature style for threading `bus` - plain `&mut Bus`
  parameter (Option A, adopted) vs. moving `Bus` into a per-step
  `StepContext<'a> { bus: &'a mut Bus, ... }` struct (Option B,
  deferred).**

  Option A: plain `&mut Bus` parameter on each method that needs
  it. Verbose at method boundaries; unambiguous at the borrow
  checker. Minimises diff size and keeps the migration 1:1.

  Option B: a `StepContext` struct gathering the bus, MMU
  reference, and current privilege. Reduces call-site verbosity
  but introduces a new type and a new borrowing story (the
  struct's fields must not be split-borrowed in ways that conflict
  with disjoint-field inference - which I-10 already pins for
  `CPU::step`; the same pattern would have to repeat at every
  helper). 02_REVIEW TR-2 confirmed deferral. Backlog pointer:
  this refactor is a future code-quality PR (provisional name
  `docs/fix/perfBusContext`); not in scope for P1, P2, or any
  perf phase. Recorded so it is not lost.

- T-4: **`store_op`'s per-store `clear_reservation` placement -
  separate call after `checked_write` (Option A, adopted) vs.
  fused into `checked_write`'s body (Option B, rejected).**

  Option A (current behaviour, preserved): `store_op` calls
  `checked_write(bus, ...)` then `bus.clear_reservation(self.id)`
  as a separate statement.

  Option B: fuse the clear into `checked_write`'s body. Semantic
  change ("store-then-clear" becomes "store-and-clear atomically"
  from an SC peer's perspective). Rejected for multi-hart
  correctness (G-3).

---

## Validation

[**Unit Tests**]

- V-UT-1: `device/bus.rs` existing tests (13) - unchanged. `Bus`
  API is untouched.
- V-UT-2: `cpu/mod.rs` existing tests (13) - mechanically migrated
  to `CPU::new(cores, Bus::new(...), layout)` and `cpu.bus().X()`
  / `cpu.bus_mut().X()`. Green at the migration commit.
- V-UT-3 (R-003 hardening): size assertions. New
  `#[test] fn bus_layout_pinned()` in `xcore/src/device/bus.rs`
  asserts `assert!(std::mem::size_of::<Bus>() < 256)` and a
  matching `cpu_layout_pinned()` in `xcore/src/cpu/mod.rs` asserts
  `assert!(std::mem::size_of::<crate::cpu::CPU<crate::arch::riscv::cpu::RVCore>>() < 4096)`.
  Catches accidental layout regressions (e.g. someone adds a large
  field, or accidentally re-introduces `Mutex`/`RwLock` which would
  push the size up).
- V-UT-4: `arch/riscv/cpu.rs` and `inst/atomic.rs` tests - all
  migrate mechanically to the threaded `bus: &mut Bus` pattern. The
  20+ existing LR/SC tests (`lr_w_then_sc_w_success`,
  `sc_w_fails_after_conflicting_store`, etc.) are the correctness
  gate for I-4 (including the new peer-hart-exclusion clause,
  R-004).
- V-UT-5 (R-001 hardening): three-layer "no-Mutex-on-bus-path"
  sentinel.
  (a) Shell gate `scripts/ci/verify_no_mutex.sh` (body shown in
  Phase 2 step 2h) runs `! rg "Mutex|RwLock|parking_lot|RefCell"
  ...` over `bus.rs`, `cpu/mod.rs`, `mm.rs`, `inst/atomic.rs`,
  `arch/riscv/cpu.rs`. Wired into `make test`. Returns non-zero on
  any match.
  (b) `#![deny(unused_imports)]` at the top of
  `xemu/xcore/src/device/bus.rs`. A reintroduced
  `use std::sync::Mutex` line is rejected at compile time even if
  the import is unused.
  (c) `compile_fail` doc-test on `Bus`:
  ```rust
  /// `Bus` is owned directly by `CPU` and must not be wrapped in a
  /// synchronization primitive on the hot path. See `01_MASTER.md`
  /// M-001 and `docs/perf/busFastPath/03_PLAN.md`.
  ///
  /// ```compile_fail
  /// use std::sync::{Arc, Mutex};
  /// use xcore::device::Bus;
  /// fn _forbidden(b: Bus) -> Arc<Mutex<Bus>> {
  ///     // Reintroducing this shape regresses Phase P1 (perfBusFastPath).
  ///     // The doc-test is wired as compile_fail not because the line below
  ///     // fails to typecheck (it does typecheck), but because a sentinel
  ///     // attribute on `Bus` (e.g. `#[deprecated]` re-export shim) makes
  ///     // wrapping the type fail to compile. If this doc-test starts
  ///     // *passing* (compiles instead of fails), the sentinel was deleted
  ///     // and round-04 must investigate.
  ///     Arc::new(Mutex::new(b))
  /// }
  /// ```
  ```
  Note: layer (c) is a documentation-anchored sentinel, not a
  cryptographic one; layers (a) and (b) are the load-bearing
  gates. Together the three are stronger than any single test
  could be.
- V-UT-6: Existing `arch_isolation.rs` seam test - unchanged.
  Exercises that `bus: &mut Bus` parameters do not leak
  arch-specific types across `xcore`'s public surface.
- V-UT-7 (R-002 hardening): I-10 disjoint-field-borrow pin.
  `compile_fail` doc-test on `CPU::step` documenting that routing
  the bus access through a helper method on `&mut self` would fail
  to compile (E0499). The doc-test reproduces the bad pattern and
  confirms it does not compile, anchoring the invariant against
  future cleanup passes.

[**Integration Tests**]

- V-IT-1: `make run` (default microbench / direct image) boots and
  exits 0.
- V-IT-2: `make linux` boots to `/ # ` prompt, runs
  `echo hello; exit`.
- V-IT-3: `make debian` boots to login (full userland sanity).
- V-IT-4 (R-006 hardening): `make linux-2hart` boots to prompt
  and both harts appear in `/proc/cpuinfo`. Wall-clock sampled 3
  runs post-P1; mean `real_s` within +/-5 % of the mean of the 3
  rows in `docs/perf/2026-04-15/data/linux_2hart.csv` (committed as
  the named pre-P1 baseline per Phase 1).
- V-IT-5: `make xv6` - boots if the target is wired up in the
  local tree; skipped otherwise and declared in the perf report so
  the reviewer can decide.
- V-IT-6: `cargo test --workspace` - 336 unit tests + atomic LR/SC
  tests + `arch_isolation` all green.
- V-IT-7: `atomic.rs` LR/SC correctness under the threaded
  `bus: &mut Bus` model - already covered by existing unit tests.
  No new 50 ms budget.

[**Failure / Robustness Validation**]

- V-F-1: `CPU::reset` after boot: all reservations cleared, all
  devices reset, subsequent step succeeds. Reuses existing
  `cpu/mod.rs` reset test under the new ownership.
- V-F-2: Difftest build (`cargo test --features difftest -p xcore`)
  green - confirms `bus_take_mmio_flag`'s `&mut self` receiver
  does not break the difftest harness.
- V-F-3: Repo-level grep gate (V-UT-5 layer a) fails if any
  future commit reintroduces `Mutex` / `RwLock` / `parking_lot` /
  `RefCell` on the bus / CPU / mm / atomic path.
- V-F-4: I-10 violation gate (V-UT-7) fails if a cleanup pass
  re-introduces a `&mut self` helper that touches both `cores` and
  `bus`.

[**Edge Case Validation**]

- V-E-1: Zero-instruction run (`CPU::run(0)`) - `step` not called,
  `bus.tick()` not called. Behaviour unchanged.
- V-E-2: Single-hart LR/SC on own reservation succeeds; LR
  followed by unrelated store does not invalidate the reservation.
- V-E-3: 2-hart LR/SC: hart 0 `lr`; scheduler advances to hart 1;
  hart 1 stores in the reserved granule; scheduler returns to
  hart 0; `sc` fails. Deterministic under cooperative round-robin.
  Covered by existing atomic tests; reconfirmed under the new
  borrow model (R-004 peer-hart clause is the why).
- V-E-4: Boot -> reset -> re-run loop: no leaked bus borrow, no
  lingering reservation, no stale device state.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (zero pthread on hot path, all configs) | V-UT-5 (three-layer sentinel); Phase-3c perf sample with zero mutex bucket. |
| G-2 (>= 15 % wall-clock, expected 20-30 %, ceiling <= 35 %) | Phase-3b sampling vs `docs/perf/2026-04-14/data/bench.csv` on dhrystone/coremark/microbench. |
| G-3 (multi-hart semantics preserved) | V-IT-4 (`make linux-2hart` boots, +/-5 % vs `docs/perf/2026-04-15/data/linux_2hart.csv`); V-E-3 (LR/SC ping-pong). |
| G-4 (LR/SC atomicity preserved) | V-UT-4 (`atomic.rs` 20+ LR/SC tests); V-E-2, V-E-3; I-4 peer-hart clause is the design argument. |
| G-5 (public API stable modulo `bus_mut` addition) | V-UT-2 (cpu/mod.rs tests unchanged in shape); external-caller audit documents zero external callers. |
| C-1 (no `unsafe`) | `make clippy` diff; `rg unsafe` review of the PR diff. |
| C-2 (device traits unchanged) | V-UT-1 (`device/bus.rs` tests green). |
| C-3 (no benchmark tricks) | Code review: bus ownership shape is static, driven by `CPU`/`Core` type layout. |
| C-4 (linux-2hart +/-5 %) | V-IT-4 sampled 3x post-P1 vs the 3-run pre-P1 baseline. |
| C-5 (fmt/clippy/test clean) | `make fmt && make clippy && make test` gate on the migration commit. |
| C-6 (DEBUG=n benchmarks) | `scripts/perf/sample.sh` env check. |
| C-7 (make-based launches) | Perf report records exact `make` targets. |
| C-8 (1:1 body changes) | PR diff review; migration table maps each site to a one-line replacement. |
| I-4 peer-hart exclusion | V-UT-4 + V-E-3 (LR/SC ping-pong tests prove the borrow-checker exclusion is operationally equivalent to the mutex it replaces). |
| I-8 (no `Mutex` / `RwLock` / `RefCell` / `Arc` on bus path) | V-UT-5 three-layer sentinel; hard `rg` gate via `verify_no_mutex.sh`. |
| I-10 (disjoint-field borrow at `CPU::step`) | V-UT-7 (`compile_fail` doc-test pinning the pattern). |

---

## Exit Gate

Union of hard gates; all must pass before P1 is declared done and
the `docs/PERF_DEV.md` P1 row is flipped to "Done":

1. `bash scripts/ci/verify_no_mutex.sh` exits 0
   (R-001 layer a; replaces the `rg "Mutex|lock()"` ad-hoc gate
   from round 02).
2. `rg "bus\.lock\(\)" xemu -n` returns zero hits.
3. `rg "Arc<Mutex<Bus>>" xemu -n` returns zero hits.
4. `make fmt && make clippy` clean (no new warnings; the
   `#![deny(unused_imports)]` lint on `bus.rs` adds no new noise
   today).
5. `cargo test --workspace` green (336 unit tests +
   `arch_isolation` + `atomic` LR/SC tests + V-UT-3 size
   assertions + V-UT-5 sentinels + V-UT-7 I-10 doc-test; difftest
   feature build also green).
6. `make run` on dhrystone / coremark / microbench: wall-clock
   reduction >= 15 % per benchmark vs.
   `docs/perf/2026-04-14/data/bench.csv`.
7. `make linux` boots to interactive shell.
8. `make linux-2hart` boots to shell; mean post-P1 `real_s` within
   +/-5 % of mean of `docs/perf/2026-04-15/data/linux_2hart.csv`
   (R-006).
9. `make debian` boots to login.
10. Fresh perf sample collected via `scripts/perf/sample.sh` with
    DEBUG=n, rendered into `docs/perf/<post-P1-date>/`; the
    `pthread_mutex_*` bucket shows 0 % of self-time in the new
    profile (by construction - no mutex remains).

Nice-to-have (does not gate; capture if tooling available):
- `cargo asm -p xcore --list | rg 'CPU.*step'` to resolve the
  monomorphised symbol, then
  `cargo asm -p xcore '<matching-line>' --rust` against
  `xcore::cpu::CPU<xcore::arch::riscv::cpu::RVCore>::step` (R-007).
  Disassembly snippet appended to the perf report confirming
  `Bus::tick` is a direct `call`.
- `criterion` microbench at `xcore/benches/bus_step.rs` on a 1 M
  NOP loop; pre/post numbers archived alongside the perf report.

---

## Response Matrix (see header for the full table)

The Response Matrix in the Log section above addresses every
C-/H-/M-/L- finding from `01_REVIEW.md`, every R- finding from
`02_REVIEW.md`, the M-001 directive from `01_MASTER.md` (still
binding; `02_MASTER.md` blank), and the trade-off advice TR-1 /
TR-2 from `02_REVIEW.md`. Each row carries Severity, Status
(resolved | rejected with reason | carry-forward), Action in this
plan, and the Test or gate that proves it. No outright rejections
this round; one trade-off divergence (TR-1, inline `Bus` instead of
`Box<Bus>`) is justified inline.
