# `perfBusFastPath` REVIEW `03`

> Status: Open
> Feature: `perfBusFastPath`
> Iteration: `03`
> Owner: Reviewer
> Target Plan: `03_PLAN.md`
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
- Blocking Issues: 2 (H-1, H-2)
- Non-Blocking Issues: 3 (M-1, M-2, L-1)

Ready for Implementation: **No** — two HIGH findings must be resolved first. Both are narrow, fixable in one editing pass; no structural rework is required.

## Summary

Round 03 is a substantial step up from round 02. M-001 is fully honoured (owned inline `Bus`, no lock), every 02_REVIEW finding R-001..R-007 is wired into the plan body with a concrete code-level action, the migration is genuinely a single atomic commit (Phase-1 baseline capture is a *separate prior* commit that touches only data files — not part of the migration atom), the 24-hit migration table matches `rg "bus\.lock\(\)" xemu -n` exactly, and Phase 11 is referenced explicitly in T-1. Scope discipline is strong (NG-1, C-8, C-3 all fence off P2/P4/P5).

Two HIGH issues remain. First, V-UT-5(c) is drafted as a `compile_fail` doc-test whose body compiles today — the plan itself admits this in the surrounding prose — meaning the test would fail on day 1 unless the sentinel mechanism the doc-test alludes to actually exists (it does not). Second, the `verify_no_mutex.sh` shell gate scans a hard-coded 5-file allow-list, which leaves a real M-001 loophole: any new file under the bus path (`xcore/src/device/bus/*.rs`, a future `bus_handle.rs`, etc.) is outside the gate's view and can silently re-introduce `Arc<Mutex<Bus>>`. Both are concrete, testable defects that should be fixed before implementation begins.

Everything else is approved. TR-1 divergence (inline vs `Box<Bus>`) is adequately justified; reviewer accepts the inline choice. TR-2 deferral is adopted cleanly. R-003 size bounds (`Bus < 256`, `CPU<RVCore> < 4096`) are realistic given the current field inventory (~136 B for `Bus` including difftest; `CPU` struct proper well under 4096 B since cores live in a `Vec`, not inline).

---

## Findings

### R-001 `V-UT-5(c) compile_fail doc-test is self-contradictory and will fail on day 1`

- Severity: HIGH
- Section: Validation / V-UT-5(c) / R-001 resolution
- Type: Validation / Correctness
- Problem:
  The plan marks V-UT-5(c) as `compile_fail`, but its own prose (03_PLAN.md lines 1020–1033) admits the body compiles today and describes the test as a "documentation-anchored, not cryptographic" sentinel. A `compile_fail` doc-test that actually compiles is a **failing test**: `cargo test` reports "doc test was expected to fail but compiled successfully". The sentinel attribute on `Bus` that would make `Arc<Mutex<Bus>>` stop compiling does not exist in the plan. So either (a) the test fails on day 1 and Exit Gate row 5 (`cargo test --workspace` green) cannot be met, or (b) the `compile_fail` attribute is silently dropped and the test becomes a comment — no longer a sentinel layer.
- Why it matters:
  V-UT-5(c) is one of the three layers round 03 uses to replace the round-02 `type_name` no-op. If it fails on day 1 the migration commit cannot land green. If it is reclassified to pass, the plan misrepresents the strength of the sentinel system (only layers (a) + (b) would be real gates).
- Recommendation:
  Pick one for round 04. Option (1): drop V-UT-5(c) entirely, acknowledge the sentinel system is two layers (shell gate + `#![deny(unused_imports)]`) — acceptable after H-2 is fixed because layer (a) becomes genuinely load-bearing. Update the R-001 / H-2 response matrix rows accordingly. Option (2): add a real compile-time sentinel on `Bus` (e.g., `PhantomData<*const ()>` to make `Bus: !Send`, which would make `Arc<Mutex<Bus>>` fail the `Send` bound at compile time) and rewrite the doc-test body to exercise that sentinel. Option (1) is simpler and expected; option (2) is stronger but requires new code in `bus.rs` not itemised in Phase 2.



### R-002 `verify_no_mutex.sh allow-list leaves a real M-001 loophole`

- Severity: HIGH
- Section: Validation / V-UT-5(a) / R-001 resolution / Exit Gate row 1
- Type: Validation / Spec Alignment / Maintainability
- Problem:
  The sentinel script (03_PLAN.md lines 785–797, mirrored in V-UT-5(a)) hard-codes five file paths: `bus.rs`, `cpu/mod.rs`, `mm.rs`, `inst/atomic.rs`, `arch/riscv/cpu.rs`. Any future file on the bus path — a split-out `xcore/src/device/bus/handle.rs`, a new `arch/riscv/cpu/smp.rs`, a `bus_shared.rs` helper — is outside the gate. A future change could wrap `Bus` in `Arc<Mutex<_>>` in a new file and the shell gate would pass green. M-001 is "no Mutex on Bus"; the gate currently enforces only "no Mutex in these five files".
- Why it matters:
  This is the precise failure mode R-001 (round 02) was raised to prevent. The round-02 `type_name` assertion was rejected as a no-op; the round-03 replacement must actually prevent M-001 regression, not just prevent it at the current site list. An allow-list that grows by editor discipline is identical in kind to the round-02 no-op under any codebase movement.
- Recommendation:
  Change layer (a) to match the *type shape* across the whole tree rather than a fixed path list. Concretely: `! rg -n "(Mutex|RwLock|RefCell)<[^>]*Bus[^>]*>" xemu/` — catches `Arc<Mutex<Bus>>`, `Mutex<BusHandle>`, `RwLock<&mut Bus>`, etc., wherever they appear, and is immune to file relocation. Keep the per-file `#![deny(unused_imports)]` (layer b) as a belt-and-suspenders compile-time signal. Update Exit Gate row 1 to reference the type-shape regex and re-word the Response-Matrix row for R-001 / H-2.



### R-003 `R-003 size bound on Bus is plausible but budgeting is hand-waved`

- Severity: MEDIUM
- Section: Data Structure / R-003 resolution / V-UT-3
- Type: Validation / Maintainability
- Problem:
  03_PLAN.md lines 475–488 justify `size_of::<Bus>() < 256` by rough inspection. Actual field inventory from `xemu/xcore/src/device/bus.rs:56-68`: `Ram ram` (32 B), `Vec<MmioRegion> mmio` (24 B), `Option<usize> mtimer_idx` (16 B), `Option<usize> plic_idx` (16 B), `u64 tick_count` (8 B), `usize num_harts` (8 B), `Vec<Option<usize>> reservations` (24 B), `#[cfg(feature = "difftest")] AtomicBool` (~8 B with padding). Sum with difftest: ~136 B, comfortably under 256. Bound holds. But the plan's justification never enumerates fields and does not mention the `difftest` cfg branch, which materially affects layout; a future maintainer cannot tell whether the 256 budget was computed with or without `mmio_accessed`, and `size_of` differs by feature flag.
- Why it matters:
  V-UT-3 runs under whichever cfg `cargo test` compiles. If a future field change pushes `Bus` past 256 in one cfg and not the other, the test will fire confusingly and the author has no documented reference to reason against.
- Recommendation:
  Add one line to the R-003 rationale enumerating the fields with current byte counts, and make V-UT-3 explicit about cfg: assert the bound in both `#[cfg(not(feature = "difftest"))]` and `#[cfg(feature = "difftest")]` branches, or pick one feature mode and comment-document that `difftest` is permitted to exceed. Three lines of code; clarity gain is significant.



### R-004 `External-caller audit is asserted but not scripted in Exit Gate`

- Severity: MEDIUM
- Section: API Surface / G-5 / Exit Gate
- Type: Validation
- Problem:
  03_PLAN.md lines 589–602 state "zero `CPU::bus()` callers outside `xcore`" based on manual grep at plan time. This is load-bearing for G-5 ("Public `CPU::bus()` signature stable"). The Exit Gate does not include any re-check at merge time. If an `xdb` or `xtool` commit adds a `cpu.bus()` caller between this plan and the migration commit, the migration would break a downstream build, and the audit would not notice until CI.
- Why it matters:
  The audit is a snapshot; the gate should be a check. Cheap to make it one.
- Recommendation:
  Add an Exit Gate row: run `cargo check -p xdb --features difftest && cargo check -p xtool` at the migration commit, or add `rg '\.bus\(\)' xemu/xdb xemu/xtool xemu/tests` with expected non-test match count of zero. Either converts the audit into an enforced check.



### R-005 `TR-1 divergence justification lacks one line making the override explicit`

- Severity: LOW
- Section: Log / Review Adjustments / TR-1 / Trade-offs / T-1
- Type: Maintainability
- Problem:
  Round 03 diverges from 02_REVIEW TR-1 (reviewer preferred `Box<Bus>`; plan chose inline). The rationale (03_PLAN.md lines 124–135, 470–497, 892–906) is sound and I accept the inline choice. But the language ("not a rejection of TR-1's underlying point ... but a decision to apply both at once") is slightly oblique. The Response-Matrix row correctly marks this "Diverged with reasoning". One additional sentence in T-1 Option A that explicitly states "chosen in divergence from 02_REVIEW TR-1 (reviewer preferred Option B)" would close the paper trail cleanly.
- Why it matters:
  Documentation clarity; no correctness impact.
- Recommendation:
  Add one line to T-1 Option A's paragraph naming the override explicitly. Non-blocking.



---

## Trade-off Advice

### TR-1 `Inline Bus vs Box<Bus> — reaffirm inline`

- Related Plan Item: `T-1` (Option A)
- Topic: Performance vs Migration-Diff Minimality
- Reviewer Position: Prefer Option A (inline)
- Advice:
  Keep inline `Bus`. Round 02's reviewer preference for `Box<Bus>` is overruled by round 03's counter-argument: diff difference is one line, inline saves a pointer hop on the hot path, V-UT-3 pins the layout. Do not revert to `Box<Bus>`.
- Rationale:
  `CPU` is constructed once and lives the entire emulation; no heap-placement win. P1's whole point is hot-path cycle count; inlining is coherent with that goal.
- Required Action:
  Adopt round 03's inline decision as the settled shape. Address R-005 (one-line clarification in T-1). No other change.



### TR-2 `StepContext deferral — keep deferred`

- Related Plan Item: `T-3`
- Topic: Call-Site Verbosity vs Migration Simplicity
- Reviewer Position: Prefer Option A (plain `&mut Bus`)
- Advice:
  Keep `StepContext` deferred exactly as round 03 plans. The T-3 backlog pointer is sufficient.
- Rationale:
  P1's scope discipline is its strongest feature. `StepContext` would introduce a new borrowing story that must re-prove I-10 at every helper; future work.
- Required Action:
  Keep Option A. No plan change required.



---

## Positive Notes

- The Response Matrix is exemplary: every 01_REVIEW C/H/M/L finding, every R-001..R-007 from 02_REVIEW, both trade-offs, and M-001 each have a row with Severity, Status, Action, and gate. No rows elided.
- Single-atomic-commit discipline (R-005) is implemented correctly: baseline capture is a separate *prior* commit touching only data files, and the migration commit bundles every field-type change, every call-site rewrite, and the sentinel script in one unit. This is the right answer to 02_REVIEW R-005 and the plan executes it cleanly.
- I-10 (disjoint-field borrow discipline at `CPU::step`) is a genuinely useful invariant — exactly the Rust-level reason the one-borrow-per-step pattern type-checks.
- I-4's new peer-hart-exclusion clause (R-004 from 02_REVIEW) correctly articulates why cooperative round-robin is sufficient exclusion for `bus.reservations[hart]`. The cites to `cpu/mod.rs:213-249` and `atomic.rs` make the argument auditable.
- Migration table matches `rg "bus\.lock\(\)" xemu -n` exactly at 24 hits; every grep output line appears in the table and vice versa.
- NG-1, C-3, C-8 form a tight fence against P2/P4/P5 leak. The plan does not overreach.
- Phase 11 is referenced explicitly in T-1 with the Option B / Option C design-space sketch. Future-SMP path is preserved and the owned-bus shape does not foreclose it.
- G-2's gain band (floor 15 %, expected 20–30 %, ceiling ≤ 35 %) is derived from the 2026-04-14 profile with stated arithmetic, not pulled from thin air.

---

## Approval Conditions

### Must Fix
- R-001 (V-UT-5(c) doc-test is self-contradictory; drop or replace)
- R-002 (sentinel script allow-list leaves M-001 loophole; switch to type-shape regex)

### Should Improve
- R-003 (enumerate Bus fields + byte budget; make V-UT-3 explicit about difftest cfg)
- R-004 (add Exit Gate row that rechecks external callers via `cargo check -p xdb -p xtool`)

### Nice to Have
- R-005 (one-line clarification of TR-1 divergence in T-1)

### Trade-off Responses Required
- TR-1 (Reaffirmed — inline `Bus` accepted; only R-005 cosmetic edit needed)
- TR-2 (Adopted — kept deferred; no plan change needed)

### Ready for Implementation
- No
- Reason: R-001 (HIGH) will cause `cargo test --workspace` to fail on the migration commit (Exit Gate row 5) as drafted, and R-002 (HIGH) leaves M-001 enforceable only on the current five-file allow-list. Both are narrow edits; round 04 should converge in one revision. R-003 / R-004 / R-005 are non-blocking and can be folded into the same revision.
