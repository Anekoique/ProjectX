# `multiHart` REVIEW `01`

> Status: Open
> Feature: `multiHart`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
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
- Blocking Issues: 2
- Non-Blocking Issues: 7



## Summary

Round 01 is a substantial improvement over round 00. The 3-PR split
(PR1 refactor → PR2a PLIC reshape → PR2b SMP activation) is adopted;
the Response Matrix covers every R-001..R-010 and inherited MASTER
directive with pointers; R-001 (PLIC runtime-size) is reframed
correctly as a device-API reshape with enumerated function-level
deltas matching the `plic.rs` source of truth; R-002 introduces I-8
with a clean RVCore-owned broadcast helper and a `Hart::last_store`
scratch field that avoids threading a back-reference through
`step_one`; R-003 resolves the test-count debt with explicit PR-wise
arithmetic.

Two correctness gaps remain blocking. First, the I-8 hook sites
enumerated in Phase-1 step 12 (`store_op`, `sc_w`, `sc_d` success)
omit `amo_store` (all 18 AMO variants) and `fstore_op` (FSW / FSD /
C.FSD / C.FSDSP) — both write physical memory and must invalidate
cross-hart reservations per RISC-V §8.2. Second, the PR1 new-test
count enumerated in the gate breakdown (line 719–721) sums to 10,
not 8, and the arithmetic does not account for V-UT-12 / V-E-5 /
V-E-6 which are all declared PR1-scope elsewhere in the plan.

Three medium issues: the `--harts N` CLI mechanism is mis-specified
against the actual startup-argv path (xdb/main.rs uses env vars, not
clap); the C-7 plan-length budget of 420 lines is violated (file is
987 lines — the Acceptance Mapping row `"Checked at plan-review
time"` is factually wrong); and the `Bus::num_harts` invariant I-9
has no matching assertion gate between `Bus::new(_, _, N)` and the
`RVCore::with_bus` expansion in R-005(a).

With R-011 and R-012 addressed and R-013..R-015 absorbed, the plan
reaches the aclintSplit-01 quality bar. No CRITICAL blockers.



---

## Findings

### R-011 I-8 hook set misses AMO and FP stores

- Severity: HIGH
- Section: Implement / Execution Flow / Phase-1 step 12; Invariants / I-8
- Type: Correctness
- Problem:
  Phase-1 step 12 specifies the I-8 broadcast hook at three sites:
  `Hart::store_op` (base.rs:73), `sc_w` success path
  (atomic.rs:64), and `sc_d` success path (atomic.rs:85). It omits:
  (a) `amo_store` — reached by `amo_w` (atomic.rs:29) and `amo_d`
  (atomic.rs:44), covering all 18 AMO operations
  (amoswap/add/xor/and/or/min/max/minu/maxu × .w/.d); each AMO is a
  physical memory write that per RISC-V spec §8.2 invalidates any
  reservation including peer-hart reservations.
  (b) `fstore_op` (float.rs:271) — reached by FSW, FSD, C.FSD,
  C.FSDSP; these are ordinary stores and per spec also invalidate
  cross-hart reservations.
  I-8 as stated ("A store from hart h_src ... clears
  `harts[h].reservation` for every h != h_src whose reservation lies
  within a granule") is spec-correct, but the enumerated hook sites
  do not cover every store path. A 2-hart Linux guest doing
  `pthread_spinlock` via AMO will silently keep stale reservations
  on peer harts.
- Why it matters:
  V-UT-11 (cross-hart LR/SC invalidation via SW) would pass, but
  the actual Linux workload path — atomics via AMO — would escape
  invalidation, producing a correctness canary gap at exactly the
  configuration NG-3 forbids difftest from catching. The symptom
  would be spurious SC.W success after peer AMO, which surfaces as
  lost updates in lock-free queues and potential kernel deadlock in
  SMP mm.
- Recommendation:
  Move the I-8 hook to a single chokepoint. Options in priority
  order:
  1. Hook inside `Hart::store` (mm.rs:306) and `Hart::amo_store`
     (mm.rs:323). These are the two primitives every store path
     eventually calls; hooking here covers store_op, fstore_op,
     sc_w/sc_d, and all AMOs uniformly.
  2. Alternatively, record `last_store = Some((paddr, size))` in
     both `mm::store` and `mm::amo_store` and keep the
     consume-after-`step_one` flow in `RVCore::step`. This is
     actually what the plan intends with the scratch-field design
     — extend the record sites to the mm layer.
  Update I-8 Invariants prose to say "every physical store,
  including AMO and FP stores," and update step 12 to enumerate
  the mm-layer hook (or the superset list above). Add V-UT-13
  "amo_invalidates_peer_reservation" and V-UT-14
  "fsw_invalidates_peer_reservation" at `num_harts = 2`.



### R-012 PR1 new-test count arithmetic does not add up

- Severity: HIGH
- Section: Implement / Phase 1 Gate Matrix; Spec / G-7
- Type: Validation / Plan Correctness
- Problem:
  PR1 gate matrix line 719–721 claims "354 pre-existing lib +
  V-UT-1..V-UT-2 + V-UT-3..V-UT-7 + V-UT-9 + V-UT-11 + V-IT-3 = 8
  new PR1 lib tests". Literal enumeration: V-UT-1, V-UT-2, V-UT-3,
  V-UT-4, V-UT-5, V-UT-6, V-UT-7, V-UT-9, V-UT-11, V-IT-3 = 10
  distinct tests. Separately, V-UT-12
  ("same_hart_store_keeps_other_reservation"), V-E-5
  ("store_overlapping_granule_invalidates"), and V-E-6
  ("store_outside_granule_preserves") are all explicitly tagged
  `*(PR1)*` in the Validation block (lines 889, 949, 952). Either
  these three are excluded (and the Validation tags are wrong) or
  they are included and PR1 delta is ≥ 13 new lib tests, not 8.
  R-003 directed that test-count arithmetic be reconciled
  explicitly; the reconciliation is still off by at least 2 and
  possibly by 5.
- Why it matters:
  Gate-matrix numerical claims are the audit trail for "zero
  regression" and "N new tests for feature X". If PR1 ships with
  362 lib tests but the reviewer-expected count was 367, the
  reviewer cannot tell whether V-UT-12 silently failed to land or
  whether V-E-5/6 were folded into another test. R-003 is
  specifically the finding that flagged this class of drift.
- Recommendation:
  (a) Decide whether each of V-UT-12, V-E-5, V-E-6 is its own
  `#[test]` function or a sub-case inside V-UT-11. (b) If separate,
  update the PR1 total: 354 + 13 = 367 lib (374 total); if
  sub-cased, say so explicitly in the Validation block. (c) Do the
  same audit for PR2a (V-UT-10 + V-IT-6 = 2 claimed; V-IT-6 is
  "existing 13 tests unchanged" which is NOT a new test count but a
  regression assertion — flag explicitly). (d) Do the same audit
  for PR2b (V-IT-2, V-IT-4, V-IT-5, V-E-4 are all tagged PR2b;
  claim is +3 — which is omitted?). Produce a final table with one
  row per new `#[test]`-or-equivalent entity, scoped to its PR.



### R-013 `--harts N` CLI mechanism mis-specified

- Severity: MEDIUM
- Section: Implement / Phase-2b step 18
- Type: API / Plan Correctness
- Problem:
  Step 18 says "CLI flag `--harts N` on the xdb / xemu binary
  front-end. Parse and pass through to `MachineConfig`." The Files
  Touched section (line 782) names `xemu/xdb/src/cli.rs` as the
  edit site. However, `xemu/xdb/src/cli.rs` is the interactive
  REPL command parser (`#[command(multicall = true)]`, commands
  like `step`, `break`, `print`, …) — not a startup argv parser.
  The actual startup path is `xemu/xdb/src/main.rs:43`
  `machine_config()`, which reads env vars (`X_DISK`) via
  `std::env::var`; there is no clap invocation on startup argv
  today. Adding `--harts` to the REPL `Cli` enum would make it an
  interactive debugger command, not a process-launch flag — the
  opposite of intent.
- Why it matters:
  PR2b's `make linux-2hart` target and the V-IT-5 smoke test both
  presume a startup-time `--harts 2`. If the flag lands as a REPL
  command, SMP can only be enabled by typing it at the xdb prompt
  after boot — far too late. The PR2b gate matrix then passes or
  fails based on whether the reader understands the ambiguity.
- Recommendation:
  Pick one and document it:
  (a) Add `X_HARTS` env var read in
  `xemu/xdb/src/main.rs::machine_config`, consistent with the
  existing `X_DISK` / `X_FW` / `X_FDT` pattern. Minimal surface,
  one new env read, `MachineConfig::with_harts` builder fed from
  it.
  (b) Introduce a new startup-level clap parser in `main.rs`
  (distinct from the REPL `Cli` in cli.rs). Larger surface, more
  churn, but matches the "flag" phrasing.
  Given the existing env-var idiom (NG-style convention at
  `main.rs:44`), (a) is preferred. Update Files Touched to name
  `xemu/xdb/src/main.rs` not `xemu/xdb/src/cli.rs`. Delete the
  "grep-verified no clap collision" note (moot under option a).



### R-014 C-7 plan-length budget violated

- Severity: MEDIUM
- Section: Spec / Constraints / C-7; Acceptance Mapping
- Type: Spec Alignment
- Problem:
  C-7 declares "Plan body ≤ 420 lines (inherited archLayout C-7 +
  margin)". The plan file is 987 lines. The Acceptance Mapping row
  for C-7 says "Checked at plan-review time" — checking now, the
  constraint is violated by 140 %. Separately, inherited 01-M-002
  ("clean, concise, elegant") is arguably not honoured for a PLAN
  that doubles the prior round's length.
- Why it matters:
  Self-declared constraints that the plan violates degrade the
  audit surface: future rounds cannot use C-7 as a quality bar if
  round 01 silently exceeds it. The aclintSplit-01 precedent (the
  stated quality target) ran ~400 lines.
- Recommendation:
  Either (a) raise C-7 to `≤ 1000 lines` with explicit rationale
  (3-PR scope + expanded matrix), or (b) trim by merging the
  Response Matrix pointers into R-xxx row prose, collapsing the
  "Changes from Previous Round" block (which duplicates the
  Response Matrix), and folding the numbered Files Touched list
  items inline with their phase step. Option (a) is cheaper if
  the content is genuinely needed; option (b) is preferred if the
  goal is sustainable round-over-round plan hygiene.



### R-015 I-9 (`Bus::num_harts`) lacks agreement assertion

- Severity: MEDIUM
- Section: Invariants / I-9; API Surface / `RVCore::with_bus`
- Type: Invariant
- Problem:
  R-005(a) resolution preserves `with_bus(bus: Bus, irq: IrqState)`
  and relies on `Bus::num_harts()` to size
  `vec![irq; bus.num_harts()]` internally. I-9 says "returns the
  value passed to Bus::new". There is no invariant or debug_assert
  tying the caller's intended N to the Bus's recorded N — a
  `Bus::new(ram, size, 1)` followed by `RVCore::with_bus(bus, irq)`
  from a test expecting 2-hart behaviour will silently build a
  1-hart core. The test setup surface is the most likely place for
  this mismatch to appear first (V-UT-11 constructs via
  `with_config`, but any future test taking the `with_bus` path
  loses the coupling).
- Why it matters:
  Silent N-mismatch is the class of bug that breaks V-UT-11 in a
  way that looks like a hart-scheduler bug rather than a
  construction bug. Low-cost fix, nontrivial if missed.
- Recommendation:
  Add to `RVCore::with_bus`:
  `debug_assert_eq!(bus.num_harts(), <expected>)` where
  `<expected>` is either a second parameter or is inferred from
  caller context. Simpler: make `with_bus` take a reference to
  MachineConfig and cross-check `config.num_harts ==
  bus.num_harts()`. Document in I-9.



### R-016 Reservation granule constant may undercover spec

- Severity: LOW
- Section: Architecture / RESERVATION_GRANULE; I-8
- Type: Correctness
- Problem:
  Plan fixes `RESERVATION_GRANULE = 8` with comment "double-word,
  covers RV64 LR.D". RISC-V spec leaves the reservation set
  granularity implementation-defined up to a page, with minimum 8
  bytes on RV64 (aligned). The invariant prose at I-8 writes
  `|r - addr| < max(size, 8)`. For an 8-byte `LR.D` at
  `0x80001000` and a 4-byte SW at `0x80001004` (plan's V-E-5
  case), `|r - addr| = 4 < 8` — correct. For `LR.D` at
  `0x80001000` and SW at `0x8000100C`, `|r - addr| = 12 >= 8` —
  correctly does NOT invalidate. Granule == 8 is defensible. But
  the helper implementation at line 331 writes
  `if let Some(r) = h.reservation && r < end && r + RESERVATION_GRANULE > addr`
  which is subtly different: it assumes the reservation covers
  `[r, r + GRANULE)` and the store covers `[addr, end)`, and
  checks overlap. For `r = 0x80001000` (LR.W, 4 bytes wide) and
  a store to `0x80001004` (4 bytes): `end = 0x80001008`,
  `r + 8 = 0x80001008 > 0x80001004` — invalidates. Correct.
- Why it matters:
  The semantics work for the tests as written, but the mismatch
  between I-8 prose (`|r - addr| < max(size, 8)`) and code
  (`r < end && r + GRANULE > addr`) is confusing. They are
  equivalent under the assumption `size <= 8` and store aligned,
  but the prose suggests a metric and the code implements a
  range-overlap. Future readers will wonder whether the intent is
  overlap or distance.
- Recommendation:
  Reconcile: rewrite I-8 prose to read "the store range
  `[addr, addr + size)` overlaps the granule-aligned range
  `[r & !(GRANULE-1), (r & !(GRANULE-1)) + GRANULE)` for the
  peer hart's reservation `r`" — or simplify the code to
  `let granule_base = r & !(RESERVATION_GRANULE - 1); if granule_base < end && granule_base + RESERVATION_GRANULE > addr`.
  Pick one model and use it in both prose and code.



### R-017 `DebugOps` self.current routing unspecified for write paths

- Severity: LOW
- Section: API Surface; Implement / Phase-1 step 7
- Type: API
- Problem:
  Plan says "Rewire `DebugOps` to route through `self.current()` /
  `current_mut()`". DebugOps today has both read paths
  (`read_gpr`, `read_pc`, …) and write paths (set breakpoint,
  step). At `num_harts > 1`, a debugger writing to "hart 0" while
  `current == HartId(1)` would silently target the wrong hart.
  NG-6 defers multi-hart debugger UX, which is reasonable, but the
  plan should say that at PR2b activation, xdb's debug behaviour
  on the non-current hart is explicitly undefined / tracked to a
  follow-up.
- Why it matters:
  Users enabling `--harts 2` in xdb (or its equivalent) will see
  confusing behaviour at breakpoints. Not a correctness failure
  for the PR1-PR2b gate matrix, but a user-visible surprise.
- Recommendation:
  Add one paragraph in NG-6 saying "At num_harts > 1, all
  DebugOps calls reflect the hart identified by `self.current` at
  call time; per-hart selection is a `xdb-smp-ux` follow-up." No
  code change in PR1..PR2b.



### R-018 `Hart::last_store` clear point not specified

- Severity: LOW
- Section: Implement / Phase-1 step 12
- Type: Correctness / Flow
- Problem:
  The scratch field design at step 12 says "record ... consumed
  and broadcast by RVCore immediately after step_one". Not stated:
  when is `last_store` cleared? If consumed means "take" via
  `Option::take()`, fine. If consumed means "read", a second step
  with no store would re-broadcast the previous step's store. The
  code shape needs to be `take()` (moves the Option out, leaving
  None), not `as_ref()`.
- Why it matters:
  A repeat broadcast of the previous store's (addr, size) range
  would over-invalidate peer reservations — technically still
  spec-compliant (implementations may over-invalidate) but
  pessimistic and confusing in a test assertion.
- Recommendation:
  One line in step 12: "RVCore calls
  `self.harts[src].last_store.take()` after `step_one` returns
  and, if Some, calls `invalidate_reservations_except`." Make the
  take-semantics explicit.



### R-019 V-IT-6 "13 tests unchanged" double-counts with V-UT-10

- Severity: LOW
- Section: Validation / Integration Tests / V-IT-6
- Type: Validation
- Problem:
  V-IT-6 is described as "all 13 existing PLIC tests pass
  unchanged" — this is a regression-assertion, not a new test. PR2a
  gate matrix (line 762) writes "+2 = 364 lib tests" counting
  V-UT-10 (new) + V-IT-6 (regression block). But V-IT-6 does not
  add a `#[test]` function — the 13 existing PLIC tests are already
  in the 354 baseline; they are re-exercised under the new
  constructor. The "+2" arithmetic is therefore 362 + 1 (V-UT-10)
  + 0 (V-IT-6 re-exercises existing 13) = 363, not 364.
- Why it matters:
  Same class as R-012. Exact numeric gate claims need to match
  what `cargo test` will actually print.
- Recommendation:
  Either add a new PLIC-side test (e.g.
  `plic_new_preserves_behavior_at_num_harts_1`) to justify +2, or
  restate PR2a as "+1 lib test = 363 lib + 1 + 6 = 370 total".
  Align with the decision from R-012.



---

## Trade-off Advice

### TR-6 I-8 hook-point depth (per-op vs mm-layer)

- Related Plan Item: T-6, Implement Phase-1 step 12
- Topic: Correctness coverage vs touch-site count
- Reviewer Position: Prefer mm-layer (single chokepoint)
- Advice:
  Hook `last_store` recording inside `Hart::store` (mm.rs:306) and
  `Hart::amo_store` (mm.rs:323) rather than per-op in
  `store_op` / `sc_w` / `sc_d`. Single recording site per
  primitive; automatically covers store_op, fstore_op, sc_w, sc_d,
  all AMOs. Matches the chokepoint the plan is already relying on
  for its `self.reservation = None` self-invalidation in base.rs.
- Rationale:
  Under the per-op placement, every new store-emitting
  instruction (a future B-extension byte-store, a Zacas
  compare-and-swap, etc.) risks a missed hook. mm-layer placement
  makes hook coverage a property of memory semantics, not opcode
  taxonomy. Incremental cost: two function-site edits instead of
  three; net lower.
- Required Action:
  Adopt at step 12. Restate I-8 coverage as "every call to
  `Hart::store` or `Hart::amo_store`".



### TR-7 PLIC-side regression test wiring

- Related Plan Item: V-IT-6, PR2a gate matrix
- Topic: Regression-block vs new-test accounting
- Reviewer Position: Prefer one explicit new test over the "13
  unchanged" claim
- Advice:
  The cleanest PR2a arithmetic is: the 13 existing PLIC tests
  remain in the 354 baseline (they use the new setup helper and
  pass), and PR2a adds exactly one new test (V-UT-10,
  `Plic::new(2, vec![irq0, irq1])` routing). Gate breakdown:
  `362 (PR1) + 1 (V-UT-10) = 363 lib + 1 arch_isolation + 6 xdb =
  370 total`.
- Rationale:
  Eliminates the "V-IT-6 counts as 1 but asserts 13 tests" oddity
  surfaced in R-019. Matches how cargo test counts work.
- Required Action:
  Align with R-012 / R-019 outcome.



---

## Positive Notes

- R-001 resolution is thorough: the deleted constants
  (`NUM_CTX`, `CTX_IP`) map 1:1 to the actual usages in
  `plic.rs` (lines 12, 22, 40-42, 49, 94, 103 via `claimed.contains`,
  106 via `CTX_IP.iter`), and the `ctx & 1` trick for
  `MEIP` vs `SEIP` is spec-exact.
- `Plic::new` callsite propagation is specified at
  `arch/riscv/cpu/mod.rs:68` — verified; plan accurately names
  the single callsite.
- The `Hart::last_store` scratch-field design is a genuine
  contribution over the round-00 shape: it keeps `Hart::step_one`
  self-contained and avoids a back-reference, as called out in
  T-7. This is the right trade-off and deserves credit.
- TR-3(b) adoption (3 PRs) is the right call and the rationale
  ("tighter bisection if SMP Linux flakes") matches reality.
- R-008 pre-verification (`sbi_hsm.c` in-tree) is done, not
  deferred — good hygiene.
- The round-robin-at-N=1 degeneracy analysis is correct:
  `(0+1) % 1 = 0` makes I-4 byte-identical at the scheduler
  level, and `RVCore::step` body structure at lines 304–315
  matches the current behaviour shape at
  `xemu/xcore/src/arch/riscv/cpu/mod.rs:224–259`.
- Non-Goals are honestly scoped: NG-3 (no difftest at N > 1) is
  acknowledged as creating a correctness canary gap that V-UT-11
  + smp_linux_smoke are designed to cover, not hidden.
- Response Matrix is complete: every R-001..R-010, every TR-1..5,
  and every inherited MASTER directive (00-M-001/002, 01-M-001..004)
  has its own row with decision + resolution pointer.



---

## Approval Conditions

### Must Fix
- R-011 (I-8 hook coverage — AMO + FP stores)
- R-012 (PR1 test-count arithmetic)

### Should Improve
- R-013 (CLI flag mechanism — env var vs startup clap)
- R-014 (C-7 plan-length budget)
- R-015 (I-9 agreement assertion)

### Trade-off Responses Required
- TR-6 (hook point depth — adopt mm-layer)
- TR-7 (PR2a test arithmetic — align with R-012)

### Ready for Implementation
- No
- Reason: R-011 is a correctness bug in the invariant hook set
  that would ship silent cross-hart LR/SC breakage as soon as
  `num_harts > 1` runs AMO-heavy workloads (all SMP Linux
  locking). R-012 is a repeat of R-003 and must be resolved for
  the gate matrix to be auditable. Neither is structurally hard
  — R-011 is a one-line change of hook site, R-012 is
  arithmetic — so round 02 should converge quickly.
