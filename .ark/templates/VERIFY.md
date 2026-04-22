# `{Feature Name}` VERIFY `{NN}`

> Status: Open | Closed
> Feature: `{feature-name}`
> Owner: Verifier
> Target Task: `{task-slug}`
> Verify Scope:
>
> - Plan Fidelity        — does the code deliver what the final PLAN promised?
> - Functional Correctness — does it work under the Validation matrix?
> - Code Quality         — readability, naming, error handling, test depth
> - Organization         — module boundaries, file placement, cohesion
> - Abstraction          — appropriate abstractions; no premature, no leaky
> - SPEC Drift           — does PLAN's Spec section still match the shipped code?

---

## Verdict

- Decision: Approved | Approved with Follow-ups | Rejected
- Blocking Issues: `{count}`
- Non-Blocking Issues: `{count}`



## Summary

`{A short overall assessment: does the implementation deliver what was promised, at the quality bar the task deserves? Are there structural or quality concerns the task should not ship without addressing?}`



## Findings

### V-001 `{short title}`

- Severity: CRITICAL | HIGH | MEDIUM | LOW
- Scope: Plan Fidelity | Correctness | Quality | Organization | Abstraction | SPEC Drift
- Location: `{file:lines or module path}`
- Problem:
  `{What is wrong, missing, or substandard.}`
- Why it matters:
  `{Impact on correctness, future maintenance, or the quality bar this tier requires.}`
- Expected:
  `{Concrete change required to resolve, or "Follow-up task" if deferred.}`



### V-002 `{short title}`

- Severity: CRITICAL | HIGH | MEDIUM | LOW
- Scope: ...
- Location: ...
- Problem:
  ...
- Why it matters:
  ...
- Expected:
  ...



## Follow-ups

`{If Verdict is "Approved with Follow-ups", list the follow-up tasks created for deferred items.}`

- FU-001 : `{new task slug}` — `{short description}`
- FU-002 : `{new task slug}` — `{short description}`
