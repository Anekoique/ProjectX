# `multiHart` REVIEW `02`

> Status: Open
> Feature: `multiHart`
> Iteration: `02`
> Owner: Reviewer
> Target Plan: `02_PLAN.md`
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
- Blocking Issues: 0
- Non-Blocking Issues: 4



## Summary

Round 02 discharges both round-01 HIGH blockers. R-011 is resolved by
moving the I-8 hook to the mm-layer chokepoints (`Hart::store` at
`mm.rs:306` and `Hart::amo_store` at `mm.rs:323`) — verified against
source: `store_op` (base.rs:71), `fstore_op` (float.rs:281), `sc_w`
(atomic.rs:66), `sc_d` (atomic.rs:87), and `amo_w` / `amo_d`
(atomic.rs:29/44) all funnel through these two primitives, so a
single recording site per primitive is a true superset of every store
path the plan enumerates. R-012 is resolved by a literal per-PR
`#[test]`-function table (13 / 1 / 3) with V-E-5 / V-E-6 folded as
assertions inside V-UT-11 / V-UT-12 and V-IT-6 reclassified as a
regression-block. R-013 replaces the CLI-flag mis-spec with an
`X_HARTS` env var read in `xemu/xdb/src/main.rs::machine_config`,
matching the existing `X_DISK` idiom at `main.rs:43-54` — grep
confirms no `cli.rs` edit is required. R-014 trims the plan to 662
lines and rebaselines C-7 to `≤ 700`. R-015 adds the
`debug_assert_eq!(bus.num_harts(), config.num_harts)` coupling in
`with_config` and a `bus.num_harts() == 1` assertion in `with_bus`.
LOW R-016..R-019 + TR-6 / TR-7 are reconciled in Invariants /
Architecture / Validation. The Response Matrix rows R-011..R-019 and
every inherited MASTER directive (00-M-001/002, 01-M-001..004) are
present with section pointers; no silent drift.

One correctness bug remains in the plan's own prose: step 12 claims
`paddr` is "the physical address returned by the MMU walk that
`checked_write` already performs" and "mm.rs threads the translated
paddr through `MemOp::Store` / `MemOp::Amo` bookkeeping". Ground
truth at `mm.rs:271-276` shows `checked_write` returns `XResult`
(type alias for `Result<(), XError>` per `error.rs:43`) — the paddr
obtained by `access_bus` at line 272 is bound to local `pa` and
discarded on the successful path. The hook as specified will not
compile unless `checked_write` is refactored to return
`XResult<usize>` (paddr) or the record is performed inside
`checked_write` instead of in the callers `Hart::store` /
`Hart::amo_store`. This is a MEDIUM API-threading correctness gap:
the invariant (mm-layer hook) is still right, but the plumbing path
is mis-described. Easily fixed with one line of spec in step 12.

Three other non-blocking items remain. The "13 existing PLIC tests"
count cited in G-8 / step 14 / V-IT-6 is off by one: the current
`plic.rs` carries 14 `#[test]` items (verified `grep -c '#[test]'`);
the PR2a regression claim should say "14 existing" and G-8 / V-IT-6
numbers should track (R-022, LOW). The env-var parse snippet at
step 18 uses a closure with an inner `?` that relies on closure-local
`Result` inference — it compiles but is less readable than a plain
`match`; stylistic nit (R-023, LOW). Finally, Acceptance Mapping
row for C-7 reads "`≤ 500-line budget`" while C-7's own prose says
"`≤ 700 lines`" — one narrative inconsistency (R-024, LOW).

None of these block implementation. Approve with revisions; R-020 is
the only item worth folding in before step 12 lands in PR1.



---

## Findings

### R-020 Step 12 mis-states paddr availability from `checked_write`

- Severity: MEDIUM
- Section: Implement / Phase-1 step 12; API Surface
- Type: API / Correctness
- Problem:
  Step 12 at `02_PLAN.md:386-400` directs the executor to write, at
  `Hart::store` (`mm.rs:306`) and `Hart::amo_store` (`mm.rs:323`),
  the body `self.last_store = Some((paddr, size))` "immediately
  after the successful `checked_write` — where `paddr` is the
  physical address returned by the MMU walk that `checked_write`
  already performs (mm.rs threads the translated paddr through
  `MemOp::Store` / `MemOp::Amo` bookkeeping)". Ground truth at
  `mm.rs:271-276`:
  ```
  fn checked_write(&mut self, addr, size, value, op) -> XResult {
      let pa = self.access_bus(addr, op, size)?;
      self.bus.write(pa, size, value)
          .map_err(|e| Self::to_trap(e, addr, op))
  }
  ```
  `XResult` is `Result<(), XError>` (`error.rs:43`, `pub type
  XResult<T = ()> = Result<T, XError>`). The paddr `pa` is bound
  inside `checked_write` and discarded — the caller at line 308
  (`Hart::store`) and line 325 (`Hart::amo_store`) has no `paddr`
  in scope. The plan's narrative is factually wrong about "mm.rs
  threads the translated paddr through … bookkeeping".
- Why it matters:
  An executor following step 12 verbatim will write
  `self.last_store = Some((paddr, size))` in `Hart::store` and
  face a compile error: `paddr` is not in scope. Options for fix:
  (a) change `checked_write` signature to `-> XResult<usize>`
  returning the paddr, and bind it in `store` / `amo_store` before
  recording; (b) perform the `last_store` write inside
  `checked_write` itself, branching on `op == MemOp::Store ||
  op == MemOp::Amo`; (c) call `self.translate(addr, size, op)?`
  again at the end of `store` / `amo_store` to recover paddr (this
  double-walks the MMU — wasteful and risks inconsistency if the
  MMU walker is not idempotent). The plan should pick one
  explicitly. Option (a) is cleanest and is the `checked_read`
  symmetry (`checked_read` already returns `Word`, adding paddr
  to `checked_write`'s return type is a one-line change).
- Recommendation:
  In Phase-1 step 12 and the I-8 invariant prose, replace the
  "paddr is threaded through bookkeeping" narrative with an
  explicit sub-step: "PR1 also widens `Hart::checked_write`'s
  signature from `XResult` to `XResult<usize>` returning the
  translated paddr; `Hart::store` / `Hart::amo_store` bind the
  returned `pa` and set `self.last_store = Some((pa, size))` on
  the successful path before returning `Ok(())`." Audit
  `checked_write`'s two callsites (`store`, `amo_store`) — no
  other callers, so the signature widening is local. Add the
  signature line to the API Surface block for `mm.rs`.



### R-021 Env-var parse snippet at step 18 uses ambiguous closure propagation

- Severity: LOW
- Section: Implement / Phase-2b step 18
- Type: Maintainability
- Problem:
  The code at `02_PLAN.md:441-445`:
  ```rust
  let num_harts = env("X_HARTS")
      .map(|s| s.parse::<usize>().context("X_HARTS must be a usize")?)
      .transpose()?
      .unwrap_or(1);
  ```
  The inner `?` inside the `.map` closure makes the closure's
  return type `Result<usize, anyhow::Error>`, so `.map` produces
  `Option<Result<usize, anyhow::Error>>`, which `.transpose()?`
  then unwraps. It compiles, but the inner `?` is syntactically
  redundant (a `.map` of `.parse::<usize>().context(...)` alone
  has the same type). Worse, a reader encountering a `?` inside
  a closure inside `.map` has to work out that it propagates to
  the closure's Result return, not to the enclosing fn —
  unnecessary cognitive load.
- Why it matters:
  Style / readability only. Functionally correct.
- Recommendation:
  Rewrite as:
  ```rust
  let num_harts = match env("X_HARTS") {
      Some(s) => s.parse::<usize>()
          .context("X_HARTS must be a usize")?,
      None => 1,
  };
  ```
  Matches the shape of the adjacent `X_DISK` match at
  `main.rs:45-53` exactly.



### R-022 "13 existing PLIC tests" count off-by-one

- Severity: LOW
- Section: Spec / G-8; Implement / step 14; Validation / V-IT-6
- Type: Validation / Plan Correctness
- Problem:
  G-8 at `02_PLAN.md:103`, step 14 at line 423, and V-IT-6 at
  line 579-581 each cite "13 existing PLIC tests". Ground truth
  at `xemu/xcore/src/arch/riscv/device/intc/plic.rs`: `grep -c
  '#\[test\]'` returns **14** (test attribute markers at lines
  184, 191, 200, 210, 216, 228, 239, 249, 260, 272, 283, 292,
  310, 321). The number in the 01_REVIEW referenced "13" from
  an earlier count; either a PLIC test was added between rounds
  or the original count was wrong. Either way the plan text
  should match `cargo test` output.
- Why it matters:
  Gate-matrix arithmetic: PR2a claim is "368 lib + 1 + 6 = 375
  tests"; if baseline PLIC count is 14 (not 13), the baseline
  354 already includes it and the `367 + 1 = 368` arithmetic
  still holds — but the V-IT-6 prose "13 existing PLIC tests"
  will be grep-visibly wrong when the reader cross-checks
  against the file. Same class as R-012.
- Recommendation:
  One-word edit: change "13 existing PLIC tests" to "14
  existing PLIC tests" in G-8, step 14, V-IT-6, and any other
  mention. Re-run `grep -c '#\[test\]'` on `plic.rs` at plan
  check-in time to catch future drift.



### R-023 Acceptance Mapping C-7 row contradicts C-7 prose

- Severity: LOW
- Section: Spec / Constraints / C-7; Validation / Acceptance Mapping
- Type: Spec Alignment
- Problem:
  C-7 prose at `02_PLAN.md:335-341` reads "`Plan body ≤ 700
  lines`" with explicit rationale. Acceptance Mapping row at
  line 651 reads "`C-7 (≤ 500-line budget)`". The two disagree.
  The Summary at line 24 also cites "`C-7 is rebaselined to ≤
  500 lines`" — matching the Acceptance Mapping row, so the
  genuine target appears to be 500 but the C-7 constraint itself
  says 700. Plan is 662 lines, so the 500-line target is
  violated today by 32 %; the 700-line target is met.
- Why it matters:
  Same audit-trail class as round-01 R-014. Self-declared
  constraints must match the validation row that checks them.
- Recommendation:
  Pick one. If the genuine target is 700 (consistent with the
  trim + rationale argument in C-7 prose), update the Summary
  at line 24 and the Acceptance Mapping row at line 651 to say
  "`≤ 700 lines`". If the genuine target is 500, trim the plan
  by another ~162 lines (collapse Response Matrix column
  widths, fold Files-Touched into phase steps per R-014(b)).
  Option (a) is cheaper and matches C-7's own stated rationale.



---

## Trade-off Advice

### TR-8 Hook implementation: caller-record vs callee-record

- Related Plan Item: T-6, Phase-1 step 12
- Topic: API Surface vs Minimal Churn
- Reviewer Position: Prefer callee-record inside `checked_write`
- Advice:
  When folding R-020's fix, consider recording `last_store`
  inside `checked_write` itself (option b), keyed on
  `op == MemOp::Store || op == MemOp::Amo`, rather than widening
  `checked_write`'s return type and recording in each caller
  (option a). Single assignment site; no signature change; no
  callsite edits in `store` / `amo_store`.
- Rationale:
  `checked_write` is private to the mm module and already the
  single gate through which every physical store passes. Adding
  the record there keeps the I-8 invariant a property of
  `checked_write`'s post-condition, which is easier to audit
  than "every caller of `checked_write` must remember to record
  after". Matches the TR-6 argument ("coverage is a property of
  memory semantics, not opcode taxonomy") one level deeper:
  coverage is a property of `checked_write`'s contract, not of
  its callers.
- Required Action:
  Executor may adopt either (a) or (b); if (b), update step 12
  and the I-8 invariant to locate the record inside
  `checked_write` and drop the caller-level record prose.



---

## Positive Notes

- **R-011 resolution is decisive and ground-truth-backed.** The
  mm-layer chokepoint claim is verifiable: `store_op` at
  `inst/base.rs:71` calls `self.store(addr, size, …)?`;
  `fstore_op` at `inst/float.rs:281` calls
  `self.store(addr, size, …)?`; `sc_w` / `sc_d` at
  `inst/atomic.rs:66/87` call `self.store(...)?`; `amo_w` /
  `amo_d` at `inst/atomic.rs:29/44` call
  `self.amo_store(addr, …)?`. Hooking `Hart::store` and
  `Hart::amo_store` covers every physical-store opcode in the
  current instruction set without enumeration, and stays correct
  under future extensions (Zacas, B-extension byte stores). V-UT-13
  and V-UT-14 are concretely specified.
- **R-012 resolution is audit-quality.** The Unit Tests tables
  per PR at lines 552-591 are one `#[test]` per row with name,
  file, and Goal/Invariant pointer. V-E-5 and V-E-6 are
  explicitly called out as assertions folded into V-UT-11 /
  V-UT-12 ("Not counted separately"), and V-IT-6 is marked
  "regression-block, not a new `#[test]`" per R-019. Gate-matrix
  arithmetic is now internally consistent (354 → 367 → 368 → 371).
- **R-013 resolution matches the existing idiom exactly.**
  `main.rs:44` reads `X_DISK` via `std::env::var(n).ok().filter
  (|s| !s.is_empty())`; step 18's `X_HARTS` addition uses the
  same `env()` closure shape. No clap dependency, no CLI parser
  introduced, no `cli.rs` edit. The ambiguity in round 01 is
  fully resolved.
- **Seam stability explicit.** I-7 at line 203-206 confirms no
  new `SEAM_FILES` / `SEAM_ALLOWED_SYMBOLS` entries, no
  `BUS_DEBUG_STRING_PINS` count change, and `Hart` / `HartId`
  never cross the `arch::riscv::` boundary. Verified against
  `xemu/xcore/tests/arch_isolation.rs:31/42/72`: the three
  pin-constants are indeed the audit surface, and none of the
  Phase-1 edits (`arch/riscv/cpu/hart.rs`, MMU layer,
  aclint sub-devices) should perturb them.
- **Response Matrix is fully reconciled.** Every R-011..R-019,
  both TR-6 / TR-7, all carried-forward R-001..R-010 / TR-1..5,
  and all inherited MASTER directives have a row with a decision
  and a resolution pointer. No silent drift between rounds.
- **C-7 rebaseline is honest about the trade-off.** The C-7
  prose at 335-341 explicitly acknowledges the round-01 420-line
  target was infeasible and documents the trim methodology
  ("merged Files-Touched into phase steps; collapsed duplicative
  log prose"). 662 actual vs 700 target leaves genuine headroom
  for round-03 edits (R-020 fold-in).
- **`debug_assert_eq!` coupling is tight.** I-9 at line 219-224
  separates `with_config` (asserts `bus.num_harts() ==
  config.num_harts`) from `with_bus` (asserts
  `bus.num_harts() == 1`, the single-hart legacy path). This
  prevents a future test from calling `Bus::new(_, _, 2)` and
  then `RVCore::with_bus(bus, irq)` and silently getting a
  1-hart core.



---

## Approval Conditions

### Must Fix
- (none — no unresolved CRITICAL or HIGH)

### Should Improve
- R-020 (step 12 paddr availability — fix the `checked_write`
  return type OR move the record inside `checked_write`; pick
  one explicitly in the plan before PR1 ships)
- R-021 (env-var parse snippet — rewrite as `match` for symmetry
  with adjacent `X_DISK` block)
- R-022 ("13 existing PLIC tests" → "14"; verified via
  `grep -c '#\[test\]' plic.rs`)
- R-023 (C-7 Acceptance Mapping row ↔ C-7 prose mismatch — pick
  500 or 700 and make both consistent)

### Trade-off Responses Required
- TR-8 (consider callee-record inside `checked_write`; optional
  but cleaner than caller-record + signature widening)

### Ready for Implementation
- Yes (with R-020 folded in before step 12 lands; R-021 / R-022 /
  R-023 are editorial and can land in the same PR1 editorial
  pass)
- Reason: R-011 (HIGH) and R-012 (HIGH) from round 01 are fully
  resolved and ground-truth-backed (mm.rs:306/323 chokepoints
  verified; per-PR `#[test]` tables enumerated; V-E-5 / V-E-6
  folded explicitly; V-IT-6 reclassified). R-013..R-019 + TR-6 /
  TR-7 are absorbed. The four residual findings are one MEDIUM
  paddr-threading clarification (R-020, local to step 12) and
  three LOW editorial items — none structurally hard, none
  blocking. The plan reaches the aclintSplit-01 quality bar.
