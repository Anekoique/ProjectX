# `plicGateway` REVIEW `02`

> Status: Closed
> Feature: `plicGateway`
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

- Decision: Approved
- Blocking Issues: `0`
- Non-Blocking Issues: `0`



## Summary

Round 02 is a narrow tightening pass that resolves all four R-012..R-015
items from `01_REVIEW.md` without reopening any settled design axis.
Scope, invariants (I-1..I-11), data structures, API surface, non-goals
(NG-1..NG-6), constraints (C-1..C-11), phase boundaries, and trade-offs
(T-1..T-5) are explicitly cited by reference and remain unchanged. The
Response Matrix records R-012..R-015 as Accepted with pointers to the
specific revised sections, and concurrence on TR-1..TR-5 is preserved.
This is the cleanest possible delta: four targeted edits, zero silent
reshuffling.

R-012 is resolved. The wildcard `sample(false)` Edge FSM row is replaced
with an explicit row `(armed=a, in_flight=f, prev_level=*) →
(armed=a, in_flight=f, prev_level=false)`, and a normative prose
clarifier ("`sample(false)` never clears `armed`") is added. The revised
Edge table is internally consistent with the Level table (which also
preserves `armed` across in-flight sample drops) and with V-UT-7 /
V-UT-12. The transition that was previously ambiguous — a pulse
(`prev=false → sample(true) → sample(false)`) during in-flight — now
unambiguously leaves `armed=true, prev_level=false`, so Edge row 6
(`(armed=true, in_flight=true, prev_level=p) → on_complete →
(armed=true, in_flight=false, prev_level=p), emit Pend`) correctly
re-pends once.

R-013 is resolved. Design (a) from `01_REVIEW.md:137-144` is pinned
explicitly. `Core::claim(&mut self, ctx)` and `Core::complete(&mut self,
ctx, src)` take only integer arguments and do not touch any gateway
field. `Plic::read` and `Plic::write` orchestrate the gateway callbacks.
The re-pend-on-complete path (`on_complete() == Pend →
core.set_pending(src) → core.evaluate()`) runs to completion inside the
same MMIO `write` stack frame. This preserves the single-tick
observability of the existing `source_repended_after_complete` test and
does not leak gateway knowledge into `Core`. Thread-safety under I-9 is
fine: the entire claim/complete orchestration runs on the tick thread
via `Bus::tick → plic.notify(bitmap)`, and MMIO `read`/`write` are also
driven from the same hart-execution context that holds `&mut Bus` — no
cross-thread PLIC mutation exists in the narrowed scope. Rejections of
(b) and (c) are reasoned: (b) couples `Core` to the gateway array
width/type, (c) bloats `core.rs` past the C-11 250-line soft cap.

R-014 is resolved. V-E-7 is a Phase-2 `Plic`-boundary integration test
that drives source 5 configured as `SourceKind::Edge` through
`with_config → notify → read(claim) → write(complete)` for two distinct
claim-complete cycles, interleaved with `notify(0x00)` and
`notify(0x20)` during in-flight to exercise Edge row 2 (level drop),
row 4 (latch-during-in-flight), and row 6 (re-pend on complete). The
assertion set is concrete: claim register returns `5` on both cycles,
no spurious claims on other sources, `in_flight == false` after the
second complete. The test lives in `plic/mod.rs#[cfg(test)]` which is
correct per C-11 (it exercises the integration of three modules). The
phase gate arithmetic is correctly updated (Phase-2 new tests ≥ 6,
total ≥ 20).

R-015 is resolved. V-IT-1 now requires `git diff main --
xemu/xcore/tests/arch_isolation.rs` to produce empty output at both
Phase-1 completion and Phase-2 completion. This closes the
"add-leaking-symbol + add-to-allowlist" loophole with zero test code.
C-3's byte-identical requirement is now operationally enforceable in CI.

No new issues are introduced:

- `Core` does not gain gateway knowledge under design (a); it remains a
  pure arbitrator, and the Data Structure diagram at
  `01_PLAN.md:376-447` was already consistent with this pinning.
- No regressions against T-1..T-5. The T-6 addendum under §Trade-offs is
  properly scoped (one-paragraph resolution, concurrence with `Core`
  gateway-agnostic design), not a re-opening of settled axes.
- Invariants I-1..I-11 are untouched. The explicit Edge row for
  `sample(false)` is consistent with I-3 (coalesce contract) and I-9
  (tick-thread-only).
- No silent changes to approved architecture, API, or constraints.
- Validation traceability is preserved and improved (G-2 now maps to
  V-E-7 additionally).
- Self-containment is preserved. The PLAN cites `01_PLAN.md` by
  line-range for the unchanged sections, which is acceptable under
  AGENTS.md §3 for a tightening pass since no executor needs to
  reconstruct intent — the changed sections are self-contained and
  the unchanged sections are already approved.

Ready for Implementation: Yes.



---

## Findings

(None. All four round-01 items R-012..R-015 are resolved by the
corresponding §Revised sections of `02_PLAN.md`. No new correctness,
spec-alignment, API, invariant, flow, validation, or maintainability
issues surface from the delta.)



---

## Trade-off Advice

(None. TR-1..TR-5 from `01_REVIEW.md` remain applicable and the plan
concurs. The new T-6 under §Trade-offs resolves the Core↔Gateway wiring
axis with design (a), which matches this reviewer's reasoning in R-013;
no further advice needed.)



---

## Positive Notes

- The Edge FSM row rewrite is minimal and surgical. Only row 2 changes;
  the prose clarifier is one sentence; the implementation note
  highlights the Level↔Edge parity so a future maintainer understands
  why both tables preserve `armed` on sample drops during in-flight.
- The wiring paragraph under §Revised Core↔Gateway Wiring is written
  in executable pseudocode (numbered `Plic::read` / `Plic::write`
  steps), which eliminates all three interpretations of the API
  surface. The reasoning block distinguishing (a)/(b)/(c) cites C-11
  and the single-responsibility premise, not vague preference.
- V-E-7's step sequence deliberately exercises Edge rows 1, 2, 4, 5, 6
  in one test — the maximum coverage per unit of test code at the
  `Plic` boundary without exploding into a matrix. The pinned
  interleaving order makes the test deterministic.
- V-IT-1's `git diff` gate is the right mechanism choice: option (a)
  from R-015 was recommended over (b) because it is zero-code and
  CI-checkable; the plan adopted (a).
- The Response Matrix explicitly notes that R-012..R-015 are
  MEDIUM/LOW (not HIGH/CRITICAL) and are listed for traceability,
  matching the AGENTS.md Response Rules expectation without
  over-weighting them.
- Phase gate arithmetic is kept honest: Phase-2 new-test count is
  bumped from ≥5 to ≥6 and total from ≥19 to ≥20 to reflect V-E-7, so
  the executor cannot quietly omit the new test.
- The "Unresolved" block correctly retains Open Q 2 from round 01 with
  the same rationale (directIrq-era decision) rather than silently
  dropping it.

---

## Approval Conditions

### Must Fix
- (none)

### Should Improve
- (none)

### Trade-off Responses Required
- (none — TR-1..TR-5 concurred in round 01; T-6 resolved in this round)

### Ready for Implementation
- Yes
- Reason: All four round-01 non-blocking findings (R-012..R-015) are
  resolved with concrete, minimal edits. No new issues introduced. No
  regressions against prior invariants, trade-offs, or phasing.
  Phase 1 remains a pure refactor with the 375-test baseline holding;
  Phase 2 adds an opt-in edge path plus V-E-7 as its `Plic`-boundary
  witness. Executor may proceed to implementation.
