# `aclintSplit` REVIEW `01`

> Status: Open
> Feature: `aclintSplit`
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
- Blocking Issues: `0`
- Non-Blocking Issues: `4`



## Summary

Round-01 resolves both blockers from round-00. The `Aclint` façade is
revived as a thin composite struct (`Aclint { mswi, mtimer, sswi }`)
with `new(irq, ssip) -> Self` and `install(self, &mut bus, base) ->
usize`, honouring the inherited "preserve `Aclint` façade" directive
and keeping `SEAM_ALLOWED_SYMBOLS` at `xemu/xcore/tests/arch_isolation.rs:48`
byte-identical (R-001 resolved). The plan enumerates the
`arch_isolation.rs` delta as empty and backs it with a Phase-1 exit
grep over `pub struct Mswi|Mtimer|Sswi`, and the `BUS_DEBUG_STRING_PINS`
pin at `arch_isolation.rs:72-75` remains valid because `bus.rs` has
never carried a quoted `"aclint"` literal — only the unquoted `ACLINT`
doc-comment tokens at lines 1-2 and 131-132 (R-002 resolved, verified
against bus.rs ground truth). The three remaining MEDIUM / LOW review
items (R-003 ssip plumbing, R-004 test count, R-005 PR granularity,
R-006 doc pin, R-007 slow-tick safety) are each accepted in the
Response Matrix with pointers to their resolving sections. Every
inherited MASTER directive (`00-M-001/002`, `01-M-001..004`) is
reconciled. Trade-off TR-1 is adopted (option b), TR-2 / TR-3 are
concurred.

Four non-blocking items remain: the plan does not explicitly state
that `Aclint` is **not** itself a `Device` (i.e., no `impl Device for
Aclint`) — this is the crux of "thin" and deserves a single sentence
under Data Structure so a future reader does not re-invent a composite
dispatcher (R-008, LOW). The three `Bus::add_mmio` calls inside
`install` have no specified `name` literals, leaving debug-log and
`replace_device` lookup semantics unstated (R-009, LOW). The test
inventory line "11 re-homed" at `01_PLAN.md:492` becomes 13 once
`unmapped_offset_returns_zero` is split three-ways per R-004 — a
narrative inconsistency worth one word (R-010, LOW). Finally, the
Review-Adjustments note at `01_PLAN.md:60` says `add_mmio("aclint", …)`
"stays in `arch/riscv/cpu/mod.rs`" but post-split the call site is
replaced by `Aclint::new(...).install(...)` — the `"aclint"` literal
is actually **removed** from `cpu/mod.rs` (and replaced by up to three
new literals inside `install`); this does not affect any pin but the
narrative is wrong (R-011, LOW).

None of these block implementation. Approve with revisions; the plan
is ready for a single merged editorial pass and then execution.



---

## Findings

### R-008 `Plan does not state Aclint is not a Device`

- Severity: LOW
- Section: Data Structure / API Surface
- Type: API
- Problem:
  The API Surface at `01_PLAN.md:314-325` declares `impl Aclint { pub
  fn new; pub fn install }` and lists three separate `impl Device for
  Mswi / Mtimer / Sswi` blocks — but it never states that `Aclint`
  itself has no `Device` impl. Today's monolithic `Aclint` at
  `xemu/xcore/src/arch/riscv/device/intc/aclint.rs:92` is
  `impl Device for Aclint`; removing that impl is the behavioural
  pivot that makes the split "real". A reader who only skims Data
  Structure sees three `Device` impls under sub-devices but might
  reasonably assume `Aclint` also implements `Device` with a
  fan-out-by-offset dispatcher — which is exactly what the split is
  eliminating.
- Why it matters:
  The whole point of the thin-struct façade (TR-1 option b) is that
  `Aclint` is a *registration helper*, not a device. If the next
  reviewer or maintainer re-adds `impl Device for Aclint` "for
  symmetry", the single-region geometry comes back and I-1 /
  independently-constructible sub-devices is silently violated. One
  sentence in the plan prevents that.
- Recommendation:
  Add a single line under Data Structure (after line 293) or API
  Surface: "`Aclint` deliberately does **not** implement `Device` — it
  is a builder-and-installer type. The three `impl Device` blocks
  live on the sub-device structs; `install` moves each sub-device
  into its own `Bus::add_mmio` slot." Optionally note that this
  removes the today's dispatcher in `aclint.rs:93-148` entirely.



### R-009 `install() does not specify add_mmio name literals`

- Severity: LOW
- Section: API Surface / Phase 1
- Type: Maintainability
- Problem:
  `Aclint::install` at `01_PLAN.md:318-324` is described by region
  only ("MSWI at base+0x0000 (size 0x4000) …") but the plan never
  names the `name: &'static str` parameter passed to each
  `Bus::add_mmio` call. The current single call at `cpu/mod.rs:62-68`
  uses `"aclint"`. Post-split, three calls must pick three names —
  candidates include `"aclint.mswi"/"aclint.mtimer"/"aclint.sswi"`,
  `"mswi"/"mtimer"/"sswi"`, or keeping a single `"aclint"` for one
  region (which would break `Bus::replace_device(name, …)` lookups if
  any test later targets a specific sub-region).
- Why it matters:
  (1) `Bus::replace_device` at `device/bus.rs:105` looks up regions
  by name and panics on miss — any future test that tries
  `bus.replace_device("aclint", …)` after the split will panic
  cryptically. (2) Debug logs at `device/bus.rs:95` emit the name,
  which is grep-visible in CI output; renaming shifts boot-log
  signatures. (3) `BUS_DEBUG_STRING_PINS` at
  `arch_isolation.rs:72-75` currently pins `"aclint"` to count 0 in
  `bus.rs` — still true after the split regardless of name choice
  because the new calls live in `aclint/mod.rs`, not `bus.rs`, so
  the pin is unaffected. But the name choice is still load-bearing
  for points (1) and (2).
- Recommendation:
  Add to API Surface one line naming the three literals, e.g.
  "`install` registers regions with names `"aclint.mswi"`,
  `"aclint.mtimer"`, `"aclint.sswi"` (dotted namespacing preserves
  grep affinity with legacy logs while distinguishing regions for
  `replace_device` lookups)." If a simpler scheme (`"mswi"`,
  `"mtimer"`, `"sswi"`) is chosen, note it explicitly and confirm no
  boot-log grep anywhere in the repo depends on the exact token
  `"aclint"`.



### R-010 `Test inventory line "11 re-homed" undercounts after R-004 split`

- Severity: LOW
- Section: Implementation Plan / Phase 1
- Type: Validation
- Problem:
  `01_PLAN.md:492` says "total 11 re-homed from the pre-split file +
  3 new isolation tests (V-UT-4/5/6) + 1 mount integration test
  (V-IT-6)". The pre-split file at `aclint.rs:164-275` has exactly
  11 tests (verified), but after R-004's resolution
  `unmapped_offset_returns_zero` lives three times (one per
  sub-device, per `01_PLAN.md:505`). The Phase-1 per-file count is
  MSWI 3 + MTIMER 6 + SSWI 4 = 13 sub-device tests, plus
  `reset_clears_state` re-homed to `aclint/mod.rs` (1), totalling 14
  re-homed across the split. The summary line still reads "11".
- Why it matters:
  Test-count arithmetic is a precondition for C-4 "re-homed, not
  rewritten" and for V-IT-1's "350 green" claim. If the real count
  is 14 re-homed, C-6 / V-IT-1 numbers should reflect that (343 lib
  + 1 arch_isolation + 6 xdb = 350 at `01_PLAN.md:515-516` becomes
  346 lib in the new arithmetic, unless some of the new assertions
  sit inside existing test bodies).
- Recommendation:
  Reconcile one of two ways. (a) Rephrase to "11 distinct assertions
  from the pre-split file → 13 tests after splitting
  `unmapped_offset_returns_zero` three ways + 1 mod.rs re-home + 3
  new isolation tests + 1 mount integration = 18 `#[test]` items in
  the split tree." (b) Keep "11" and mark the two new
  unmapped-offset assertions as `#[test]` items that inherit the
  original test's name in a different module, preserving the count.
  Either way, the V-IT-1 / V-E-1 test-count arithmetic must match.



### R-011 `Review-Adjustments narrative wrong about "aclint" literal location`

- Severity: LOW
- Section: Response Matrix / Review Adjustments
- Type: Spec Alignment
- Problem:
  `01_PLAN.md:59-61` states: "`add_mmio("aclint", …)` stays in
  `arch/riscv/cpu/mod.rs` (not `device/bus.rs`); the `"aclint"`
  needle in `bus.rs` remains at count 0." Ground truth at
  `xemu/xcore/src/arch/riscv/cpu/mod.rs:62-68` confirms today's
  `add_mmio("aclint", 0x0200_0000, 0x1_0000, …)` lives in
  `cpu/mod.rs`. Post-split, that call is replaced (per the plan's
  own "After" block at lines 346-349) by
  `Aclint::new(...).install(&mut bus, 0x0200_0000)`, which means the
  literal `"aclint"` is **removed** from `cpu/mod.rs` and up to
  three new literals appear inside `install` in
  `aclint/mod.rs`. The Review-Adjustments note describes the
  opposite invariant.
- Why it matters:
  The argument the note wants to make — that `bus.rs` contains no
  quoted `"aclint"` and therefore the `BUS_DEBUG_STRING_PINS` pin
  at `arch_isolation.rs:73` stays at count 0 — is **correct**. But
  the justification (via the `cpu/mod.rs` site staying put) is
  false. The actual reason is that the `BUS_DEBUG_STRING_PINS`
  scan targets `bus.rs` only (per `arch_isolation.rs:72` comment),
  and no `"aclint"` literal is ever inserted into `bus.rs` by
  this plan.
- Recommendation:
  Rewrite lines 59-61 as: "The `BUS_DEBUG_STRING_PINS` scan at
  `arch_isolation.rs:72` targets `xemu/xcore/src/device/bus.rs`
  only. That file has never contained a quoted `"aclint"` literal
  (only the unquoted token `ACLINT` in doc comments at lines 1-2
  and 131-132, which the scan does not match). The split removes
  the `add_mmio("aclint", …)` call from `cpu/mod.rs` and introduces
  up to three new region-named calls inside `aclint/mod.rs::install`;
  neither file is in the pin's scan list, so the pin stays at
  count 0."



---

## Trade-off Advice

### TR-1 `Façade shape — thin struct vs free function`

- Related Plan Item: `T-1`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Concur with chosen option (b)
- Advice:
  Keep the thin-struct adoption. This resolves R-001 cleanly and
  preserves `SEAM_ALLOWED_SYMBOLS` at zero change. No further action.
- Rationale:
  The struct is a 3-field type whose only methods are `new` +
  `install`; it carries no runtime cost beyond the existing field
  ownership, and the `install` method is strictly a convenience over
  three successive `Bus::add_mmio` calls. Option (a)'s "no type"
  cleanness was already rejected in round 00. Option (c)
  (`Bus::add_aclint`) is correctly rejected in the plan for violating
  01-M-004.
- Required Action:
  Keep as is. Addressing R-008 (state that `Aclint` is not a `Device`)
  is the only tightening worth making in this round.



### TR-2 `Region granularity — three regions vs one dispatcher`

- Related Plan Item: `T-2`
- Topic: Spec Fidelity vs Simplicity
- Reviewer Position: Concur with chosen (a)
- Advice:
  Keep three regions. No change.
- Rationale:
  Already validated at round 00. Plan's rationale is sound: option
  (b) reintroduces the coupling the split removes and fails G-1.
- Required Action:
  None.



### TR-3 `NG-5 Bus-residual scope — rename only`

- Related Plan Item: `T-3`
- Topic: Flexibility vs Safety
- Reviewer Position: Concur with chosen (a)
- Advice:
  Keep the rename-only scope. Deferring `Bus::mtime` /
  `Device::mtime` default-method removal to `directIrq` is correct.
- Rationale:
  Narrow PRs + deferred optimization is the right division. The plan
  cites NG-2 for the deferral, which the future `directIrq` task
  should reference when it activates.
- Required Action:
  None.



---

## Positive Notes

- **R-001 resolution is decisive and well-justified.** The Response
  Matrix cites the inherited directive verbatim, the Data Structure
  section adds the 3-field composite, and the API Surface shows the
  call-site delta at `cpu/mod.rs:61-69` collapsing to two lines. The
  `Aclint` seam symbol at `SEAM_ALLOWED_SYMBOLS[4]` is preserved
  unchanged.
- **R-002 resolution is backed by a verifiable exit check.** The
  Phase-1 grep `pub struct Mswi|Mtimer|Sswi` over
  `arch/riscv/device/intc/aclint/*.rs` returns zero on pass — this
  is a concrete, script-able gate, exactly the right shape for
  `arch_isolation` stability.
- **PR compression (R-005) matches archLayout-04 precedent.** One PR
  for the structural split plus `mtimer_idx` rename, followed by a
  pre-merge validation gate (not a commit). Gate matrix C-6 lists
  all six inherited gates (`cargo test --workspace`, `fmt`, `clippy`,
  `make linux`, `make debian`, difftest) plus `make run` — fully
  compliant with archLayout-04's baseline.
- **`Aclint::new(irq, ssip)` signature preserves today's call shape.**
  Passing `ssip` explicitly (R-003 resolution) mirrors the
  `Aclint::new(irq.clone(), bus.ssip_flag())` pattern at
  `cpu/mod.rs:66` byte-for-byte, which is exactly what "zero
  BootConfig churn" requires.
- **Response Matrix is comprehensive.** Every R-00X finding is rowed
  with decision + resolution pointer; every inherited MASTER directive
  is reconciled; TR-1/2/3 each have verdicts. No silent drift.
- **Test inventory per sub-device is concrete.** The Phase-1 per-file
  table at `01_PLAN.md:496-507` names each test, its new home, and
  its local offset — mechanical enough to implement without
  reconstructing intent.



---

## Approval Conditions

### Must Fix
- (none — no unresolved CRITICAL or HIGH)

### Should Improve
- R-008 (state `Aclint` is not a `Device`; prevents future
  re-introduction of composite dispatch)
- R-009 (specify `add_mmio` name literals inside `install`; affects
  `replace_device` lookup and debug-log grep)
- R-010 (reconcile "11 re-homed" arithmetic with R-004's
  three-way split; align V-IT-1 test-count claim)
- R-011 (correct the Review-Adjustments narrative about where the
  `"aclint"` literal lives pre/post split; the conclusion is right
  but the stated reasoning is wrong)

### Trade-off Responses Required
- TR-1 — adopted, no action
- TR-2 — concurred, no action
- TR-3 — concurred, no action

### Ready for Implementation
- Yes (with the four LOW edits folded in; none are blocking)
- Reason: R-001 (CRITICAL) and R-002 (HIGH) from round 00 are both
  resolved in full and backed by ground-truth evidence (`bus.rs` has
  no quoted `"aclint"` literal; `SEAM_ALLOWED_SYMBOLS` at
  `arch_isolation.rs:42-65` stays byte-identical). The four
  residual findings are editorial / narrative in nature and can
  land in the same editorial pass as implementation. No gate
  failure is anticipated from the current plan as written.
