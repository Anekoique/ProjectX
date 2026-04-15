# `aclintSplit` REVIEW `00`

> Status: Open
> Feature: `aclintSplit`
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
- Blocking Issues: `2`
- Non-Blocking Issues: `5`



## Summary

The plan is spec-faithful on the register map and geometry: MSIP@+0x0000,
MtimecmpLo@+0x4000, MtimeLo@+0xBFF8, Setssip@+0xC000 match
`xemu/xcore/src/arch/riscv/device/intc/aclint.rs:18-27` exactly, and the
`0x4000 + 0x8000 + 0x4000 = 0x1_0000` triple covers every currently
decoded offset with zero gap or overlap. The three-file split under
`arch/riscv/device/intc/aclint/` honours 00-M-002 / 01-M-003 / 01-M-004,
and the constructor-per-device shape honours 01-M-001 and 00-M-001.
Behaviour-preservation invariants I-1..I-6 are stated precisely, and
test re-homing is mechanical (11 existing tests → 10 re-homed + 1 moved
to `aclint/mod.rs`), with three new isolation tests proving
`I-1`. Difftest and boot gates are named and inherited from
archLayout-04.

Two items block approval.

First, the plan eliminates the `Aclint` type outright in favour of a free
function `mount`, but the saved inherited project memory
(`project_manual_review_progress.md` line 13) records the binding
expectation "**preserve `Aclint` façade for BootConfig ergonomics**".
This is a silent deviation from a directive that has been in force since
archModule landed. Either the plan must justify the departure against
that directive explicitly in the Response Matrix (with reviewer-visible
reasoning) or it must keep a thin `Aclint` type that wraps the three
sub-device constructions. See R-001.

Second, the plan does not account for `SEAM_ALLOWED_SYMBOLS` at
`xemu/xcore/tests/arch_isolation.rs:42-65`, which currently pins
`"Aclint"` as an allowed seam re-export. Removing the `Aclint` type
without updating that pin — and without updating the companion
re-exports in `src/arch/riscv/device/intc/mod.rs` (which today only has
`pub mod aclint; pub mod plic;` but which `device/intc/mod.rs` in the
seam file list must continue to gate) — will fail the
`arch_isolation` integration test on PR1 merge. See R-002.

The remaining five findings are non-blocking: an API Surface ambiguity
about how `mount` obtains the SSIP flag (R-003), a test-count
inconsistency between Phase-1 listing and V-E-1 (R-004), an overly fine
PR2/PR3 split that could collapse (R-005), an unclear handling of the
`"ACLINT every step"` comment versus BUS_DEBUG_STRING_PINS (R-006),
and weak evidence for the slow-tick behaviour-preservation claim on
MSWI/SSWI (R-007).

Trade-offs T-1 / T-2 / T-3 are framed honestly; the recommendation on
T-1 is conditional on R-001's resolution.



---

## Findings

### R-001 `Silent deviation from inherited "preserve Aclint façade" directive`

- Severity: CRITICAL
- Section: Response Matrix / API Surface / T-1
- Type: Spec Alignment
- Problem:
  The saved project-memory note at
  `/Users/anekoique/.claude/projects/-Users-anekoique-ProjectX/memory/project_manual_review_progress.md:13`
  is explicit: "Task 2 `aclintSplit` … Split ACLINT into spec-mandated
  MSWI + MTIMER + SSWI; **preserve `Aclint` façade for BootConfig
  ergonomics**". The plan eliminates the `Aclint` composite type
  entirely (`00_PLAN.md:50-57`: "The façade is a **free function**, not
  a wrapper struct: no composite `Aclint` type survives") and the
  Response Matrix does not record a rejection with explicit reasoning.
  This is a silent architectural change against an inherited binding
  expectation.
- Why it matters:
  AGENTS.md §3 requires the Response Matrix to record every inherited
  directive with a decision + resolution; rejections must carry explicit
  reasoning. Eliminating `Aclint` also has concrete downstream impact:
  `SEAM_ALLOWED_SYMBOLS` in `xemu/xcore/tests/arch_isolation.rs:49`
  pins the string `"Aclint"`, and removing the type forces the pin to
  be dropped or replaced, which is a seam-surface change the plan does
  not mention.
- Recommendation:
  Resolve one of two ways. (a) Add a thin `pub struct Aclint { … }`
  whose sole purpose is `Aclint::new(irq, ssip) -> Self` returning a
  value that knows how to register the three sub-devices
  (`Aclint::install(self, &mut bus) -> usize` returning `mtimer_idx`);
  this preserves the BootConfig-side call pattern and the seam symbol
  `"Aclint"`. Or (b) reject the façade expectation explicitly in the
  Response Matrix, enumerate the downstream impact (seam symbol drop,
  `cpu/mod.rs:20` import rewrite, future BootConfig call-site shape),
  and cross-reference T-1 option (a) as the chosen replacement.
  Either path must also update R-002's scope.

### R-002 `arch_isolation seam pin is not in the migration scope`

- Severity: HIGH
- Section: Implementation Plan / Phase 1
- Type: Validation
- Problem:
  `xemu/xcore/tests/arch_isolation.rs:42-65` defines
  `SEAM_ALLOWED_SYMBOLS` containing `"Aclint"` (line 49). The plan's
  Phase 1 touches `arch/riscv/cpu/mod.rs:61-69` and the per-file split
  but does not list the seam-test update. If the `Aclint` type is
  removed (per R-001 option b), the `use super::device::intc::{aclint,
  plic::Plic};` line does not re-export `Aclint`, so the symbol's
  absence is fine from a re-export standpoint, but any dangling
  `"Aclint"` pin in the allow-list is stale pinning and will trip the
  "seam re-exports X which is not in allow-list" check if any other
  file re-exports one of the new types (`Mswi`/`Mtimer`/`Sswi`). The
  plan's V-IT-2 claim ("new files live under `arch/riscv/device/intc/aclint/`
  which is already covered by the existing seam allowlists") is
  imprecise: seam files are allowlisted by *path*, but *symbols* they
  may re-export are separately enumerated at line 42-65, and
  `Mswi`/`Mtimer`/`Sswi`/`mount` are not in that list.
- Why it matters:
  If `arch/riscv/device/intc/mod.rs` gains `pub use aclint::{Mswi,
  Mtimer, Sswi};` (or even `pub use aclint::mount;`), the
  `arch_isolation` test will fail on PR1 because those identifiers are
  not in `SEAM_ALLOWED_SYMBOLS`. This is a concrete PR1 gate failure
  that the plan currently misses.
- Recommendation:
  Add an explicit Phase-1 action: update
  `xemu/xcore/tests/arch_isolation.rs:42-65` to either (a) remove
  `"Aclint"` and add `"Mswi"`, `"Mtimer"`, `"Sswi"`, `"mount"` for the
  symbols that will actually be re-exported through seams, or (b) if
  R-001 option (a) is chosen and `Aclint` survives, document that no
  new seam symbols appear and leave the pin intact. Either way, the
  plan must list the test-file edit and describe the expected diff.

### R-003 `API Surface signature hides SSIP flag dependency`

- Severity: MEDIUM
- Section: API Surface / Failure Flow
- Type: API
- Problem:
  The API Surface at `00_PLAN.md:296` declares
  `pub fn mount(bus: &mut Bus, base: usize, irq: IrqState) -> usize`,
  but the Failure Flow (step 4) relies on `mount` calling
  `bus.ssip_flag()` internally to obtain the shared `Arc<AtomicBool>`.
  Separately, `Sswi::new(ssip: Arc<AtomicBool>)` (line 301) is shown as
  the public constructor. This means the `mount` helper's internal
  control flow is: fetch `bus.ssip_flag()` → build `Sswi` → add_mmio.
  That is fine, but the signature does not document it, and a future
  reader reasonably expects the ssip flag to be a separate parameter
  (mirroring `irq`). This is the exact pattern the current
  `cpu/mod.rs:66` call uses (`Aclint::new(irq.clone(), bus.ssip_flag())`)
  — a two-dependency constructor.
- Why it matters:
  Obscured dependencies are hard to test and invite inconsistency.
  V-F-2's "PRIMARY gate" (`rg '\bssip_pending\b'`) asserts the
  textual invariant but not the semantic one (same `Arc` identity is
  actually shared). A unit test for `mount` cannot verify the wiring
  without constructing a full `Bus`, which is why V-IT-6 exists — but
  V-IT-6 is a bus-level mount test, not an API documentation aid.
- Recommendation:
  Either (a) change the signature to
  `pub fn mount(bus: &mut Bus, base: usize, irq: IrqState,
  ssip: Arc<AtomicBool>) -> usize` and have the caller pass
  `bus.ssip_flag()` explicitly (mirrors today's `Aclint::new` shape,
  keeps the helper pure in its arguments), or (b) keep the two-arg
  signature and add a one-line doc comment in the API Surface stating
  "`mount` internally obtains the SSIP flag via `bus.ssip_flag()` to
  ensure `Arc` identity with `Bus::take_ssip`." Option (a) is
  preferable — it reads as a direct translation of today's call and is
  trivially unit-testable without a Bus.

### R-004 `Unmapped-offset test count inconsistency between Phase 1 and V-E-1`

- Severity: MEDIUM
- Section: Implementation Plan / Validation
- Type: Validation
- Problem:
  Phase 1 file listing at `00_PLAN.md:410-424` states MSWI has 2 tests
  including `unmapped_offset_returns_zero` (regionalised to MSWI),
  MTIMER has 5 tests (none named unmapped), and SSWI has 3 tests
  (none named unmapped). V-E-1 at line 573-575 says "Unmapped offset
  within each sub-region returns 0 — sub-regional version of the
  existing `unmapped_offset_returns_zero`; one assertion per
  sub-device." That is three assertions across three sub-devices, but
  Phase 1 only lists the MSWI-local version. The MTIMER/SSWI
  equivalents are absent from the Phase-1 file list.
- Why it matters:
  Test inventory completeness is a precondition for the "re-home not
  rewrite" invariant in C-4. The Phase-1 list under-counts by two
  tests; either V-E-1 overstates coverage or Phase-1 understates it.
- Recommendation:
  Reconcile by adding `unmapped_offset_returns_zero` to each of
  `mtimer.rs` (Phase-1 MTIMER test list: 5 → 6) and `sswi.rs`
  (Phase-1 SSWI test list: 3 → 4), matching V-E-1's
  one-per-sub-device claim, and document each new assertion's local
  offset (e.g., MTIMER `0x0100`, SSWI `0x0100`). Alternatively, collapse
  V-E-1 to a single MSWI-only test and drop the "per sub-device"
  language — but per-sub-device is stronger coverage and aligns with
  the I-1 isolation invariant.

### R-005 `PR2 / PR3 granularity is excessive`

- Severity: MEDIUM
- Section: Execution Flow / Main Flow
- Type: Maintainability
- Problem:
  PR2 (`aclint_idx` → `mtimer_idx` rename, plan line 361-367) is a
  mechanical rename of one field and five call sites inside `bus.rs`
  plus one doc comment. PR3 (plan line 368-371) is declared as
  "No code change expected" — purely a gate-matrix sweep. Splitting
  a single-field rename into its own PR yields a commit-history
  granularity below what archLayout-04 delivered (which bundled a
  22-row rewrite + 1 visibility widen + test-file update in one
  Phase-1 PR), and a PR containing zero code is unusual. The
  archModule precedent (5 PRs across 4 rounds) batched
  behaviour-preserving renames with their context.
- Why it matters:
  More PRs means more gate-matrix runs (`make linux` + `make debian` +
  difftest are minutes-long), more review surface, and more
  round-trip overhead for a rename that is provably behaviour-preserving.
  The bisection argument in plan line 371 is weak: PR1's 350-test
  green bar + `make run` HIT GOOD TRAP already proves structural
  correctness; a separate PR for a field rename adds no bisection
  signal because the rename cannot introduce semantic drift.
- Recommendation:
  Collapse PR2 into PR1 (rename `aclint_idx` → `mtimer_idx` as part of
  the structural split, since PR1 already touches `bus.rs` call sites
  via the new `mount` helper registration pattern). Keep PR3 as the
  final boot + difftest gate checkpoint, but reframe it as "post-merge
  validation sweep" rather than a PR — or fold it into PR1's gate
  matrix (PR1 already requires `make run`; extending to `make linux`
  + `make debian` + difftest for the final PR is the gate, not a
  separate commit). Net result: 1 PR (split + rename) + 1 validation
  gate, matching archLayout-04's two-phase shape.

### R-006 `Bus docstring edit is not mapped to BUS_DEBUG_STRING_PINS`

- Severity: LOW
- Section: Implementation Plan / Phase 2
- Type: Validation
- Problem:
  Phase 2 (plan line 366-367) edits `device/bus.rs:1-2` doc from
  "ACLINT every step" to "MTIMER every step". The
  `BUS_DEBUG_STRING_PINS` pin at `arch_isolation.rs:72-75` watches the
  literal `"aclint"` (lowercase, in quotes) with an expected count of
  0 — this matches the current source (bus.rs uses `ACLINT` in a
  comment, not `"aclint"` as a literal). Replacing "ACLINT" with
  "MTIMER" in the comment doesn't affect the pin (both are
  case-sensitive and the needle is `"aclint"` in quotes, not `ACLINT`
  in a comment). However, the plan does not explicitly state this —
  a reader might assume the rename affects the pin count.
- Why it matters:
  Small clarity issue; wrong assumption during implementation could
  lead to spurious pin updates.
- Recommendation:
  Add a one-line note in Phase 2: "`BUS_DEBUG_STRING_PINS`
  (`arch_isolation.rs:72-75`) tracks the literal `"aclint"` with
  expected count 0; the doc-comment rename does not change this pin
  and requires no test update."

### R-007 `Weak evidence for MSWI/SSWI slow-tick behaviour preservation`

- Severity: LOW
- Section: Invariants / Spec
- Type: Correctness
- Problem:
  Invariant I-3 (plan line 237-240) states: "MSWI and SSWI tick on the
  slow path." Today's `Aclint::tick` at `aclint.rs:137-148` runs
  `sync_wallclock()` and `check_timer()` on every bus tick, not
  per-device-concern; post-split, only MTIMER ticks every step, and
  MSWI/SSWI move to the SLOW_TICK_DIVISOR=64 cadence. The plan claims
  this is behaviour-preserving because MSWI/SSWI have no tick-driven
  work — MSIP toggles on MMIO write, SETSSIP edges on MMIO write, and
  neither reads wall-clock state. That is correct against the current
  implementation, but the plan does not spell it out as an explicit
  argument.
- Why it matters:
  A future maintainer reading I-3 without the aclint.rs context might
  conclude that slowing the MSWI/SSWI tick is a behavioural change.
  The plan's "same latency as before" language (line 240) refers to
  `Bus::mtime()` only, not to MSWI/SSWI state updates.
- Recommendation:
  Add one sentence under Invariants: "MSWI and SSWI have no
  tick-driven state updates today — `Aclint::tick` only advances
  `mtime` and evaluates `check_timer`; MSIP and SSIP changes are
  MMIO-write-driven. Slowing their tick to `SLOW_TICK_DIVISOR=64` is
  therefore behaviour-preserving by construction."



---

## Trade-off Advice

### TR-1 `Façade shape — free function vs. thin struct`

- Related Plan Item: `T-1`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option (b) — thin struct with `install`
- Advice:
  Choose option (b) (`struct Aclint` / `AclintHandle` with an
  `install(self, &mut bus) -> usize` method, or equivalently
  `Aclint::new(irq, ssip).install(&mut bus) -> usize`). This preserves
  the `Aclint` type name — the exact façade the saved project memory
  names — while delivering the three-device split. R-001 is
  automatically resolved, R-002 shrinks to "no change needed", and
  the BootConfig call site stays one line.
- Rationale:
  The inherited project-memory directive is explicit about preserving
  the façade. The plan's argument for option (a) ("zero new type,
  minimal surface") is true on its own merits, but the project has
  already paid the complexity cost of `Aclint` in its seam surface
  (SEAM_ALLOWED_SYMBOLS, cpu/mod.rs import, BootConfig). Option (b)
  costs one 1-field wrapper struct; option (a) costs a seam-symbol
  migration, a directive-rejection narrative, and a departure from
  the "preserve façade" expectation. Option (a) is cleaner in
  isolation, but (b) is cleaner in context.
- Required Action:
  Adopt (b) or justify rejection in the Response Matrix.

### TR-2 `Region granularity — three regions vs. one dispatcher`

- Related Plan Item: `T-2`
- Topic: Spec Fidelity vs Simplicity
- Reviewer Position: Agree with chosen (a)
- Advice:
  Keep the chosen three-region layout. Option (b) (one region + internal
  dispatcher) would reintroduce the exact coupling the split removes
  and would fail G-1 (three independently constructible devices).
- Rationale:
  The spec defines three separate controllers; making the split
  "notional" by keeping a single region defeats the purpose. Future
  multi-hart growth (`multiHart` task) will grow MSWI and MTIMER
  independently (per-hart MSIP and mtimecmp arrays); keeping them
  fused makes that growth harder.
- Required Action:
  Keep as is.

### TR-3 `NG-5 Bus-residual scope — rename only vs. bus-mtime removal`

- Related Plan Item: `T-3`
- Topic: Flexibility vs Safety
- Reviewer Position: Agree with chosen (a)
- Advice:
  Keep the rename-only scope. Removing `Bus::mtime` / the
  `Device::mtime` default method is a semantic change with measured
  per-step cost (plan line 499-502 cites ns/step) and is rightly
  deferred to `directIrq`.
- Rationale:
  Narrow PRs are easier to review and bisect; bundling an unrelated
  optimization risks conflating split regressions with dispatch-cost
  regressions. The deferral is explicit (NG-2) and the follow-up task
  is named.
- Required Action:
  Keep as is; ensure NG-2 language in `directIrq` references this
  deferral when that task activates.



---

## Positive Notes

- Register map preservation is exact: offsets in plan line 277-281
  match `aclint.rs:18-27` bit-identically. Region geometry
  (`0x4000 + 0x8000 + 0x4000 = 0x1_0000`) is arithmetically correct
  and covers every currently-decoded offset.
- Four named risks (OpenSBI offsets, difftest divergence, multiHart
  coupling, SSWI Arc ownership) are all addressed: C-1 / V-IT-6 /
  V-F-4 / I-4 / V-F-2 / NG-1.
- Test inventory is mechanical and traceable: 11 current tests → 10
  re-homed + 1 cross-register test moved to `aclint/mod.rs`, plus 3
  new isolation tests (V-UT-4/5/6) that directly prove I-1. This is
  the correct shape for a structural split.
- Acceptance mapping (plan line 585-606) is complete: every G-/C-/I-
  item is mapped to at least one V- validation.
- Gate matrix (C-6) correctly inherits the archLayout-04 baseline
  (350 tests + fmt + clippy + linux + debian + difftest) without
  relaxation.
- Tone discipline on NG items is good: each non-goal is mapped to a
  future task (NG-1→multiHart, NG-2→directIrq, NG-4→directIrq), so
  the deferrals are honest rather than hand-waves.



---

## Approval Conditions

### Must Fix
- R-001 (resolve the `Aclint` façade directive conflict — preferably
  via TR-1 option b)
- R-002 (add explicit `arch_isolation.rs` SEAM_ALLOWED_SYMBOLS update
  to Phase 1)

### Should Improve
- R-003 (make `mount` signature pass ssip flag explicitly)
- R-004 (reconcile unmapped-offset test count between Phase 1 and V-E-1)
- R-005 (collapse PR2 into PR1; reframe PR3 as validation gate not a PR)

### Trade-off Responses Required
- TR-1 — adopt option (b) or record explicit rejection with reasoning
  in the Response Matrix
- TR-2 — none (acknowledge concurrence)
- TR-3 — none (acknowledge concurrence)

### Ready for Implementation
- No
- Reason: R-001 is a silent-deviation CRITICAL against an inherited
  directive and must be either honoured or explicitly rejected in the
  Response Matrix. R-002 is a concrete PR1 gate risk
  (`arch_isolation` test will fail on seam-symbol drift) that the
  plan currently omits. Both resolve in one editorial pass; the
  remaining findings are non-blocking and can land in the same round.
