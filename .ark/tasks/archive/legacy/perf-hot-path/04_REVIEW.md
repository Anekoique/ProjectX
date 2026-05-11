# `{Feature Name}` REVIEW `{NN}`

> Status: Open | Closed
> Feature: `{feature-name}`
> Iteration: `{NN}`
> Owner: Reviewer
> Target Plan: `{NN_PLAN.md}`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved | Approved with Revisions | Rejected
- Blocking Issues: `{count}`
- Non-Blocking Issues: `{count}`



## Summary

{A short overall assessment of the plan in this round:
whether it is coherent, implementable, aligned with goals/constraints,
and whether the trade-offs are acceptable.}



---

## Findings

### R-001 `{short title}`

- Severity: CRITICAL | HIGH | MEDIUM | LOW
- Section: `{e.g. Architecture / Invariants / Execution Flow / Validation}`
- Type: Correctness | Spec Alignment | API | Invariant | Flow | Validation | Maintainability
- Problem:
  {What is wrong / missing / unclear.}
- Why it matters:
  {Impact on correctness, constraints, implementation, or future maintenance.}
- Recommendation:
  {Concrete change expected in next PLAN.}



### R-002 `{short title}`

- Severity: CRITICAL | HIGH | MEDIUM | LOW
- Section: `...`
- Type: `...`
- Problem:
  ...
- Why it matters:
  ...
- Recommendation:
  ...



---

## Trade-off Advice

### TR-1 `{trade-off title}`

- Related Plan Item: `T-1`
- Topic: Performance vs Simplicity | Flexibility vs Safety | Compatibility vs Clean Design | ...
- Reviewer Position: Prefer Option A | Prefer Option B | Need More Justification
- Advice:
  {What choice is recommended.}
- Rationale:
  {Why this trade-off direction is better under the current goals / constraints / risks.}
- Required Action:
  {Executor should adopt it / justify rejection / expand comparison / keep as is with clarification.}



### TR-2 `{trade-off title}`

- Related Plan Item: `T-2`
- Topic: ...
- Reviewer Position: ...
- Advice:
  ...
- Rationale:
  ...
- Required Action:
  ...



---

## Positive Notes

- {What is strong / well-designed / well-validated in this plan}
- {Optional: especially good trade-off framing or clear invariant design}

---

## Approval Conditions

### Must Fix
- R-001
- R-002

### Should Improve
- R-003

### Trade-off Responses Required
- T-001
- T-002

### Ready for Implementation
- Yes | No
- Reason: {short justification}
