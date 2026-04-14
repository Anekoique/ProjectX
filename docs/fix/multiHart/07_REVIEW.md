# `multiHart` REVIEW `07`

> Status: Closed
> Feature: `multiHart`
> Iteration: `07`
> Owner: Reviewer
> Target Plan: `07_PLAN.md`
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
- Blocking Issues: 0
- Non-Blocking Issues: 0

## Summary

Round 07 cleanly absorbs all four carryovers from round 06. The
`Rc<RefCell<Bus>>` → `Arc<Mutex<Bus>>` pivot (R-034) is propagated
end-to-end: `G-1/2/3`, Architecture diagram, `CPU::step` body, every
store path, `CPU::from_config` construction, T-4, CC-9 Send/Sync
chain, and NG-6 poison policy. The `Bus::store` value-type ambiguity
(R-036) is closed by C-13 plus an explicit `size ∈ {1,2,4,8}` with a
`cfg(isa64)` gate on the 8-byte case, and the Response Matrix
clarifies that `sc_w/sc_d` return 0/1 are GPR writes, not bus stores
— removing the last sizing edge. Test-count arithmetic (R-035) is
internally consistent: 354 baseline + 10 PR1 = 364 lib ⇒ 371; +1 at
PR2a ⇒ 372; +3 at PR2b ⇒ 375, reconciled across Summary, Response
Matrix, and Phase gates. Tick/reset ordering (R-037) is promoted to
I-12 with a concrete body and a reset sequence (bus first, then
cores), and V-E-7 tests the cadence.

Architecture, API surface, invariants, PR split, and validation
skeleton remain aligned with prior approved plans; `CoreOps::step`
signature is preserved; C-12 is extended to `mm*.rs` per TR-3. Plan
body is 397 lines, inside the 400-line cap (C-7). MASTER directives
(00-M-001/002, 01-M-001..004) and all prior CRITICAL/HIGH findings
appear in the Response Matrix; no approved architecture, API
semantics, invariants, or constraints are silently changed. CC-4
remains a documented KNOWN-LIMITATION under NG-6, which is
consistent with the single-OS-thread NG-2 scope.

No CRITICAL, HIGH, or MEDIUM issues remain. The plan is ready for
implementation.

---

## Findings

(None.)

All round-06 carryovers are resolved:

- R-034 (CRITICAL, Send violation): resolved — `Arc<Mutex<Bus>>`
  everywhere; `.lock().unwrap()` at every former `.borrow*()` site;
  Send/Sync chain documented at CC-9; NG-7 retired; T-4 reframes the
  choice as mandated rather than optional.
- R-036 (MEDIUM, store sizing): resolved — C-13 fixes the multiHart
  scope to `cfg(isa64)`; `size ∈ {1,2,4,8}` with the 8-byte arm
  gated by `cfg(isa64)`; FSD routes f64 bit-pattern as `Word`;
  `sc_w/sc_d` return values clarified as GPR writes.
- R-035 (LOW, test math): resolved — 354 + 10 = 364 lib at PR1,
  +1 at PR2a, +3 at PR2b; Summary, Response Matrix, and Phase gates
  are mutually consistent.
- R-037 (LOW, tick/reset order): resolved — I-12 pins `bus.tick()`
  to once per `CPU::step` before `cores[current].step()`;
  `CPU::reset` order is bus first, then cores; V-E-7 checks the
  cadence.

---

## Trade-off Advice

### TR-1 `Arc<Mutex<Bus>> as mandated choice (T-4)`

- Related Plan Item: `T-4`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A (accept as framed)
- Advice:
  Keep the plan's framing — `Arc<Mutex<_>>` is compelled by the
  `XCPU: Send` bound, not selected among equivalents. The ~1.5%
  uncontended-lock overhead is acceptable relative to ~100 ns
  dispatch, and `std::sync::Mutex` preserves C-6 (no new deps).
- Rationale:
  The `Rc<RefCell<_>>` alternative fails to compile against
  `OnceLock<Mutex<CPU<Core>>>`, so the comparison is closed by the
  type system. `parking_lot` is correctly rejected under C-6. The
  Send/Sync chain at CC-9 makes the soundness argument explicit.
- Required Action:
  Keep as is; no further justification needed.

### TR-2 `Bus::store as sole chokepoint (T-3)`

- Related Plan Item: `T-3`
- Topic: Flexibility vs Safety
- Reviewer Position: Prefer Option A
- Advice:
  Retain `Bus::store` as the single physical-address chokepoint and
  the C-12 audit extension to `mm*.rs`.
- Rationale:
  Funneling every physical store through one `lock().unwrap()`
  makes the write-plus-peer-invalidate atom trivially provable
  under NG-2 and gives grep-auditable I-8 enforcement. The
  alternative mm-layer hook would scatter responsibility across
  eight methods, which is exactly what round 06 retired.
- Required Action:
  Keep as is; the grep audit in the PR1 gate is sufficient.

---

## Positive Notes

- Response Matrix is complete and specific: every prior finding
  (R-001..R-033, R-034..R-037, TR-3/9/10) and every MASTER
  directive is accounted for, with accepted/rejected decisions and
  concrete plan-section impacts.
- The Send/Sync chain at CC-9 is an unusually clear soundness
  argument — each link (`Device: Send` → `Bus: Send` → `Mutex<Bus>:
  Send+Sync` → `Arc<Mutex<Bus>>: Send+Sync` → `CPU<Core>: Send` →
  `XCPU: Sync`) is explicit.
- C-13 plus the `sc_w/sc_d` GPR-write clarification closes R-036
  cleanly without inventing a new type; the plan resists the
  temptation to over-engineer.
- I-12 framing ("N=1 byte-identical to pre-pivot; at N>1 ticks N
  times per round matching HW hart-cycle clocking") is a nice
  semantic anchor that ties the scheduler, the bus cadence, and
  V-E-1/V-E-7 together.
- PR split (PR1 pivot at N=1 / PR2a PLIC runtime-size at N=1 /
  PR2b activate N>1) keeps every step byte-identical or
  regression-gated, which maximizes reviewability.

---

## Approval Conditions

### Must Fix
- (none)

### Should Improve
- (none)

### Trade-off Responses Required
- (none — TR-1/TR-2 are acceptance notes, not action items)

### Ready for Implementation
- Yes
- Reason: 0 CRITICAL, 0 HIGH, 0 MEDIUM open. All round-06
  carryovers (R-034/R-035/R-036/R-037) are resolved with concrete
  plan-surface changes. Plan body is within the 400-line cap;
  architecture, API, invariants, and validation are internally
  consistent and traceable via the Acceptance Mapping.
