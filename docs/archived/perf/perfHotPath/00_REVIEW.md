# `perfIcache` REVIEW `00`

> Status: Open
> Feature: `perfIcache`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking Issues: 2 (1 HIGH plumbing gap, 1 HIGH harness gap)
- Non-Blocking Issues: 7 (4 MEDIUM, 3 LOW)

## Summary

The plan is structurally sound, well-grounded in the post-P1 baseline, and the
direction (per-hart direct-mapped cache keyed on `(pc, ctx_tag, raw)` with a
`pest`-fallback miss path) is the right shape for the `xdb::main` bucket
documented in `docs/perf/2026-04-15/REPORT.md`. Invariants I-1..I-13 cover the
correctness obligations. Goals, non-goals, and constraints are concrete; the
benchmark-hack discipline (NG-4 / C-1) is explicit. The torture-test-first
ordering is correct per `docs/PERF_DEV.md` §3 P4.

Two blocking gaps remain:

1. The `Bus::store` → storing-hart `icache_ctx_tag++` mechanism is described as
   "follow the LR/SC pattern" but there is no LR/SC peer-reservation *callback*
   in `device/bus.rs:189-208` to follow — `Bus` mutates its own
   `reservations: Vec<Option<usize>>` directly because that state lives on
   `Bus`. The `RVCore`-side `icache_ctx_tag` does *not* live on `Bus`, so the
   plan's hand-wave does not resolve to a concrete patch. Round 01 must pick
   one mechanism and prove it does not reintroduce a `Mutex<…>` (C-7).
2. The SMC torture test is specified as `make run AM=smc`, but the am-tests
   harness (`xkernels/tests/am-tests/Makefile`) dispatches via single-letter
   targets and `MAINARGS`, not `AM=smc`. The harness wiring step ("next to
   existing tests") is also missing the `name` mapping update and the `ALL`
   shortlist update, so the test would not actually execute under the stated
   command.

The remaining findings are MEDIUM/LOW: the `(pc >> 1) & MASK` index function
is correct on aligned PCs but deserves an explicit note about RVC vs. RVI
collision behaviour; the `mstatus` write hook should bump only when MMU-index
bits actually change (the plan's any-write fallback is acceptable for round 00
but should be tightened); the trap-entry hook should debounce traps that don't
change privilege; tag-wrap (I-11) deserves a defensive flush rather than a
"documented and forget" stance; and the validation matrix has small gaps
around `mstatus.MPRV` flipping the MMU index, which is a load-bearing
icache-context bit not pinned by any V-UT.

The trade-off framing (T-1..T-7) is genuinely useful and the recommendations
are defensible. Trade-off advice is captured below in TR-1..TR-4.

---

## Findings

### R-001 `Bus::store → storing-hart icache invalidation has no concrete plumbing`

- Severity: HIGH
- Section: `Implementation Plan / Phase 3 step 8`, `Architecture`, `Invariants I-9`
- Type: Correctness / API
- Problem:
  Phase 3 step 8 proposes that `Bus::store` (`xemu/xcore/src/device/bus.rs:189`)
  bumps the storing hart's `RVCore::icache_ctx_tag`, and justifies the
  mechanism by analogy: *"`Bus::store` already takes `hart: HartId` and
  already has the LR/SC peer-reservation hook for cross-hart effects, so we
  add an analogous 'icache invalidate self' callback through the existing
  per-hart core handle."* This is incorrect: there is no LR/SC *callback* —
  `Bus::invalidate_peer_reservations` (`bus.rs:195-208`) mutates a `Vec`
  *owned by `Bus`* (`reservations: Vec<Option<usize>>`, `bus.rs:124`). There
  is no per-hart core handle on `Bus` to dispatch to. The icache lives on
  `RVCore`, not `Bus`, so the analogous pattern does not apply. The plan's
  three sketched options (A: thread `&mut RVCore` into `Bus::store`; B:
  return-signal consumed by `mm.rs::checked_write` at `mm.rs:282-293`; C: a
  per-hart `dyn FnMut()` callback stored in `Bus`) are not even enumerated
  in the PLAN — only Option C-shaped language ("callback") is hinted at, and
  C reintroduces a heap-allocated callback that itself risks tripping
  `verify_no_mutex.sh` if any future shared state is added.
- Why it matters:
  This is the load-bearing piece of correctness for SMC. Getting it wrong
  silently turns the SMC hook into a no-op (and the V-UT-3 stub will pass
  trivially because it bumps `icache_ctx_tag` on the storing hart by direct
  call, not through `Bus::store`). It also has the highest blast radius on
  the `Bus` API and on the M-001 sentinel (C-7).
- Recommendation:
  In `01_PLAN`:
  (a) Pick Option B: `Bus::store` already returns `XResult`; have
  `RVCore::checked_write` (`mm.rs:282-293`) call `self.invalidate_icache()`
  on the success branch when the resolved physical address falls in RAM.
  This keeps the `Bus` API agnostic of the icache, leaves all icache state
  on `RVCore`, and avoids any callback / box / lock. Justify rejection if
  another option is preferred.
  (b) Spell out the exact `checked_write` change, including the case where
  the store traps (PMP / alignment) — the icache must NOT be bumped if the
  store did not commit.
  (c) Add an explicit V-UT that drives the *real* path
  (`RVCore::store(...)` → `checked_write` → `bus.store` → `invalidate_icache`)
  and asserts `icache_ctx_tag` advanced, *not* a test that calls
  `core.invalidate_icache()` directly.
  (d) Re-affirm C-7 by adding a sentence: "no `Mutex`, `RwLock`, `RefCell`,
  `Box<dyn FnMut>`, `Arc<…>` is added to `Bus` or `RVCore` for this hook."

### R-002 `SMC torture-test invocation and harness wiring do not exist as specified`

- Severity: HIGH
- Section: `Implementation Plan / Phase 1 step 2 + V-IT-1`
- Type: Validation / Maintainability
- Problem:
  `xkernels/tests/am-tests/Makefile` dispatches via single-letter targets
  mapped through the `name = $(patsubst u,uart-putc, ... )` macro and
  selects via `MAINARGS="\"$*\""` and the `ALL = u r t s p c e f` shortlist
  (no `m` for `smc`). The plan's invocation is `make run AM=smc`, which is
  not a recognised target anywhere in the tree. The harness wiring ("Wire it
  into the am-tests harness next to existing tests so `make run AM=smc`
  runs it") leaves out the three concrete edits required:
  (i) add a letter alias and extend `ALL`;
  (ii) extend the `name` `patsubst` chain;
  (iii) ensure `main.c` (or whatever drives `MAINARGS`) routes to
  `test_smc()`.
  Additionally, the existing tests (`csr-warl.c`, `timer-read.c`) print
  `OK` and rely on the AM `halt(0)` path producing `GOOD TRAP`, which the
  Makefile greps for; the plan's `printf("PASS smc")` will not match the
  harness's pass condition.
- Why it matters:
  Phase 1 explicitly gates Phase 2 on this test going green. If the test is
  wired wrong, the gate is meaningless and round 01 will discover that the
  SMC hook was never exercised by integration. This also breaks the round's
  "torture test lands FIRST" discipline (PERF_DEV §3 P4).
- Recommendation:
  In `01_PLAN`:
  (a) Specify the exact letter (e.g. `m`), the `ALL` line edit, the `name`
  `patsubst` edit, and the `MAINARGS` dispatch edit.
  (b) Use the existing pass marker (`printf("smc: OK\n");` then return
  cleanly so the AM runtime emits `GOOD TRAP`), not a custom `PASS smc`
  string.
  (c) Replace `make run AM=smc` with `cd xkernels/tests/am-tests && make m`
  (or whichever letter is chosen).
  (d) Add a Rust-level integration test that does the same SMC sequence
  through `RVCore::step` and `RVCore::store`, so the regression survives
  even if the am-test harness is reorganised.

### R-003 `mstatus-write hook may bump the icache on every FS/SD write`

- Severity: MEDIUM
- Section: `Architecture / Invariants`, `Implementation Plan / Phase 3 step 2`, `T-5`
- Type: Correctness (over-conservative) / Performance
- Problem:
  Phase 3 step 2 bumps `icache_ctx_tag` on every write to `mstatus`/`sstatus`.
  The actual MMU-index bits are MPRV, MPP (when MPRV=1), SUM, and MXR.
  `csr_write_side_effects` at `csr/ops.rs:44-52` already recomputes SD from
  FS dirtiness on every `mstatus` write — and `RVCore::dirty_fp`
  (`cpu.rs:123`) writes `mstatus` on every FP-CSR write *and* on every FP
  instruction's retire path. With dhrystone (no FP) this is harmless; with
  coremark/microbench it's still rare; with Linux + GLIBC it can be
  hundreds of FP-instruction-per-millisecond busy paths, each costing a
  spurious cache flush. The plan flags this as T-5 with a "tighten in 01 if
  needed" stance, but does not pin a measurement plan.
- Why it matters:
  An over-conservative `mstatus` hook can silently destroy the hit rate on
  Linux without triggering the V-IT-4 telemetry threshold (which only runs
  on dhry/cm/mb). The 95 % hit-rate floor is on the wrong workloads to
  catch this regression.
- Recommendation:
  In `01_PLAN`:
  (a) Bit-isolate the bump: read old vs. new `mstatus`, bump only if
  `(old ^ new) & (MPRV | MPP_when_MPRV | SUM | MXR) != 0`. This is one
  XOR-and-mask, comparable cost to the unconditional bump.
  (b) If the bit-isolated form is rejected for round 00 simplicity, add a
  V-IT-4-equivalent for `make linux` (boot with `--features icache_stats`
  and assert hit rate ≥ 90 % across the boot trace) so the regression is
  observable.

### R-004 `Trap-entry hook bumps even when privilege does not change`

- Severity: MEDIUM
- Section: `Implementation Plan / Phase 3 step 7`, `T-6`
- Type: Correctness (over-conservative)
- Problem:
  The plan bumps at both `handler.rs:101` (S-mode trap entry) and
  `handler.rs:106` (M-mode trap entry). These are unconditional. In
  practice, an M-mode trap *from M-mode* (e.g. an `ecall` from an
  M-mode-only firmware) does not change `self.privilege` and does not
  change the MMU index (M-mode has bare translation regardless of
  `satp`). The plan acknowledges this in T-6 but proposes always-bump
  for "simplicity." This is fine on benchmark workloads but pessimises
  OpenSBI's M-mode trap path, which is hot during early boot.
- Why it matters:
  Same risk as R-003 — over-conservative bumps reduce the hit rate
  invisibly because the V-IT-4 measurement is restricted to dhry/cm/mb.
- Recommendation:
  In `01_PLAN`, bump only if `old_priv != new_priv` (one comparison
  before the bump). Update I-13 / V-UT-2 to assert the no-op case as
  well.

### R-005 `Index function (pc >> 1) & MASK undocumented for RVC/RVI aliasing`

- Severity: MEDIUM
- Section: `Architecture` index formula, `Invariants I-8`, `Data Structure`
- Type: Correctness / Maintainability
- Problem:
  The index `(pc >> 1) & MASK` is correct in the sense that it never
  produces an out-of-range index, and the full `pc` tag prevents false
  hits. But the plan does not analyse the alias structure: a 16-bit
  RVC at `pc=0x1000` and a 32-bit RVI at `pc=0x1002` map to *adjacent*
  slots, while a 32-bit RVI at `pc=0x1000` and another at `pc=0x3000`
  (offset by `MASK<<1 = 0x2000`) collide. For dhry/cm/mb the static
  footprint is far below 4096 instructions so collisions are
  irrelevant; for `make linux` boot, a XOR-fold against `ctx_tag`
  (`idx = ((pc >> 1) ^ (ctx_tag as usize >> 4)) & MASK`) would
  decorrelate post-`satp`-change reuse. The plan does not consider
  this and does not measure it.
- Why it matters:
  A direct-mapped 4 K cache with no XOR-fold is fragile under context
  changes — every `satp` write redistributes the working set into the
  same 4 K slots, which is fine in steady state but can churn during
  context-switch storms.
- Recommendation:
  In `01_PLAN`:
  (a) Add an explicit aliasing analysis paragraph under `Invariants
  I-8`: how PC-aligned RVC vs. RVI lands, what the worst-case
  collision pattern is, and why 4 K is sufficient for dhry/cm/mb.
  (b) Defer the XOR-fold proposal to a 02-round only if Linux boot
  hit rate < 95 %, but document that this is the cheapest next
  refinement (one XOR vs. doubling the cache).

### R-006 `Validation does not pin MPRV-driven MMU-index changes`

- Severity: MEDIUM
- Section: `Validation`, `Acceptance Mapping`
- Type: Validation
- Problem:
  V-F-2 covers `satp` change. V-UT-2 covers privilege transitions.
  Neither covers the case where M-mode flips `MPRV` (with a stale
  `MPP` pointing to S/U) and the *next* fetch needs to use the
  S/U MMU view. Since `effective_priv()` (`mm.rs:228-235`) consults
  `MPRV`, an `mstatus.MPRV` write changes which page table the
  fetch sees — which is exactly the case the icache must invalidate
  (an old-`MPRV`-decoded line at the same VA may be a different
  guest instruction in U-mode).
- Why it matters:
  This is the primary justification for bumping on `mstatus`
  writes (T-5). If the validation suite never exercises the
  MPRV-flip path, the bit-isolation refinement (R-003) cannot be
  safely landed and the over-conservative form will be locked in
  by inertia.
- Recommendation:
  Add V-F-4: set up two distinct page mappings (same VA, different
  contents); execute one in M-mode with `MPRV=0`; flip `MPRV=1`
  with `MPP=U` and a different mapping live; assert the *new*
  mapping's instruction executes. Map V-F-4 against the eventual
  bit-isolated `mstatus` hook.

### R-007 `Tag wrap (u32) is documented but not defended`

- Severity: LOW
- Section: `Invariants I-11`, `Failure Flow #5`
- Type: Correctness (defence in depth)
- Problem:
  `icache_ctx_tag: u32` wraps at ~4 G events. The plan argues this
  is "unreachable in practice" and deferred. For a long-running
  Linux guest under heavy I/O, ~4 G `Bus::store` invocations is
  reachable in roughly an hour of wall-clock at modern emulation
  rates. On wrap, a stale line whose `ctx_tag` happens to equal
  the new wrap-around value will silently hit and execute stale
  decoded instructions.
- Why it matters:
  A silent-stale-hit on a long-running emulator session is the
  hardest possible class of bug to debug: it is rare,
  guest-state-dependent, and produces no error.
- Recommendation:
  Either (a) widen to `u64` (no measurable cost on the hot path —
  the comparison is one extra cycle on x86_64 and free on
  AArch64), or (b) on detected wrap (`new == 0` after `wrapping_add`),
  iterate the cache once and reset every line to `ctx_tag = 0`,
  then set `self.icache_ctx_tag = 1`. Pick one in `01_PLAN`.

### R-008 `Phase 4 telemetry feature flag lifecycle is ambiguous`

- Severity: LOW
- Section: `Implementation Plan / Phase 4 step 5`
- Type: Maintainability
- Problem:
  Phase 4 step 5 says "Remove `icache_stats` feature flag (or leave
  it gated behind `--features` for future profiling — reviewer's
  call)." Leaving feature-gated counters in the production code
  path is fine; the ambiguity is whether the exit-gate measurement
  in V-IT-4 is reproducible after the round closes.
- Why it matters:
  Future regressions in hit rate (e.g. R-003 over-conservative
  `mstatus` becoming worse with a new guest) need the same
  telemetry to diagnose. Removing the flag would force a fresh
  patch every time we want to measure.
- Recommendation:
  Keep the `icache_stats` feature gated; document the
  invocation in `docs/PERF_DEV.md` so future rounds can reuse it.
  Update Exit Gate item "≥ 95 % hit rate" to read "≥ 95 % under
  `--features icache_stats`" so the measurement is unambiguous.

### R-009 `Response Matrix N/A correctly noted`

- Severity: LOW
- Section: `Response Matrix`
- Type: Maintainability (positive)
- Problem:
  None. The N/A entry is appropriate because there is no prior
  REVIEW and no prior MASTER. Logged here only to confirm the
  template-compliance check passed.
- Why it matters:
  Confirms `01_PLAN` will need to populate the matrix with R-001
  through R-008 and TR-1 through TR-4.
- Recommendation:
  No change. `01_PLAN` Response Matrix must include every R-*
  finding above and every TR-* below.

---

## Trade-off Advice

### TR-1 `Direct-mapped 4096 vs. set-associative 4-way × 4096`

- Related Plan Item: `T-1`
- Topic: Performance vs Simplicity
- Reviewer Position: Prefer direct-mapped (Option A) for round 00; revisit
  only after `make linux` telemetry says we need it.
- Advice:
  Land the direct-mapped 4 K. Do not pre-emptively go to 4-way.
- Rationale:
  The post-P1 working sets are far below 4 K static instructions. The
  4-way LRU bookkeeping costs measurable arithmetic on the hot path —
  exactly where we are trying to *remove* arithmetic. NEMU IBuf's
  4-way is a counter to *aliasing in larger workloads*; we should buy
  evidence (V-IT-4 on Linux) before paying that cost.
- Required Action:
  Adopt direct-mapped, defer 4-way to a 02-round contingent on Linux
  hit rate.

### TR-2 `4 K vs. 16 K cache size`

- Related Plan Item: `T-2`
- Topic: Memory vs Hit Rate
- Reviewer Position: Prefer 4 K with telemetry; same logic as TR-1.
- Advice:
  Keep 4 K for round 00. Allocate the bump to a 02-round only on
  measured Linux miss-rate evidence.
- Rationale:
  16 K × ~24 B/line × N harts is non-trivial allocation pressure on
  many-hart configurations, and the cache geometry interacts with
  host L1/L2; a blind 4× is unjustified.
- Required Action:
  Confirm 4 K + telemetry; capture the 02-round trigger condition
  ("if Linux boot hit rate < 95 %, escalate to 16 K and re-measure")
  in the Exit Gate.

### TR-3 `SMC strategy: global flush vs. paddr-tagged`

- Related Plan Item: `T-4`
- Topic: Correctness vs Optimality
- Reviewer Position: Prefer global flush (Phase 1 conservative).
- Advice:
  Land global flush. Defer paddr-tagged refinement.
- Rationale:
  Code-writing guests are extremely rare; benchmarks never write
  code; `make linux` writes code only at module load and JIT. The
  paddr-tagged bitmap adds significant state and a per-page
  bookkeeping cost that does not pay off on the workloads in
  scope.
- Required Action:
  Adopt global flush. Add a one-line comment in the eventual
  `checked_write` plumbing pointing at the future paddr-tagged
  refinement issue.

### TR-4 `mstatus bump: any-write vs. bit-isolated`

- Related Plan Item: `T-5`
- Topic: Performance vs Simplicity
- Reviewer Position: Prefer bit-isolated *now*, not as a deferral.
- Advice:
  Implement bit-isolated `(old ^ new) & (MPRV | SUM | MXR) != 0`
  in round 01 alongside the rest of the hooks.
- Rationale:
  See R-003. The cost of an XOR-and-mask is identical to the cost
  of an unconditional bump on the host, but the hit-rate impact on
  FP-heavy workloads (and on guests that flip FS frequently) is
  potentially material. The complexity of bit isolation is one
  line of code.
- Required Action:
  Adopt bit-isolated form in `01_PLAN`; if rejected, justify with
  measured FS-flip rate on a representative workload.

---

## Positive Notes

- The plan correctly orders torture-test → optimisation, matching
  PERF_DEV §3 P4's mandate.
- Invariants I-1..I-13 are concrete and individually checkable;
  I-2 / I-12 in particular pin the "ctx_tag mismatch is a miss"
  rule cleanly.
- Trade-off framing T-1..T-7 is unusually thorough — every
  alternative cites a precedent (QEMU jump-cache, NEMU IBuf,
  Ertl & Gregg) and identifies the cost direction.
- Failure Flow #2 (decode failure does not poison the line) and
  V-F-1 are exactly right and prevent a class of subtle bugs.
- I-10 + V-UT-1 (compile-time `Copy` assertion on `DecodedInst`)
  is a clean invariant: `InstKind` (`isa/riscv/inst.rs:9`),
  `RVReg` (`isa/riscv/reg.rs:10`), `SWord`, and `u8` are all
  already `Copy`, so the derive is non-breaking and the static
  assert pins the property forever.
- C-7 (`verify_no_mutex.sh` regression guard) is explicitly
  honoured in the constraints, and the per-hart icache design
  is consistent with the M-001 sentinel from `device/bus.rs:1-50`.
- The bucket math (`xdb::main` 40-47 % → 15-25 % wall-clock)
  is honest — the plan does not over-promise.

---

## Approval Conditions

### Must Fix
- R-001 (HIGH — pick concrete `Bus::store → invalidate_icache`
  mechanism, prove no `Mutex`/callback regression)
- R-002 (HIGH — fix am-test invocation + harness wiring; add
  Rust-level integration test for SMC)

### Should Improve
- R-003 (MEDIUM — bit-isolated `mstatus` hook now or measure
  Linux boot)
- R-004 (MEDIUM — only bump trap entry on actual privilege change)
- R-005 (MEDIUM — document RVC/RVI aliasing under I-8)
- R-006 (MEDIUM — add V-F-4 for MPRV-driven MMU-index change)
- R-007 (LOW — widen `ctx_tag` to `u64` or add wrap defence)
- R-008 (LOW — keep `icache_stats` feature, document
  invocation, fold into Exit Gate)

### Trade-off Responses Required
- TR-1 (T-1 — direct-mapped acceptable)
- TR-2 (T-2 — 4 K acceptable)
- TR-3 (T-4 — global flush acceptable)
- TR-4 (T-5 — bit-isolated `mstatus` requested)

### Ready for Implementation
- No
- Reason: R-001 is a load-bearing correctness gap that turns the
  SMC hook into a no-op if implemented as currently described, and
  R-002 means the gating torture test cannot run as specified.
  Both must be resolved in `01_PLAN` before implementation begins.
