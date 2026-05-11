# `<feature-name>` PLAN `<NN>`

> Status: Draft | Revised | Approved for Implementation
> Feature: `<feature-name>`
> Iteration: `<NN>`
> Owner: Executor
> Depends on:
> - Previous Plan: `<(NN-1)_PLAN.md | none>`
> - Review: `<NN_REVIEW.md | none>`

---

## Summary

<one paragraph: what this PLAN proposes>

## Log `None in 00_PLAN`

[**Added**]

- <new content / new design / new validation>

[**Changed**]

- <what changed; why>

[**Removed**]

- <what was removed; why>

[**Unresolved**]

- <what remains open; why>

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted / Rejected / Deferred | <what changed; if rejected, the reason> |

> Every prior CRITICAL / HIGH finding from `(NN-1)_REVIEW.md` must appear here. Rejections require explicit reasoning.

---

## Spec

> This section is the durable design record. On deep-tier commit, it is copied **verbatim** into `specs/features/<slug>/SPEC.md`. Keep it tight: the SPEC is what future readers consult to understand what was built, not why each step happened. Why-explanations belong in `## Trade-offs`. Implementation steps belong in `## Implementation`. The Spec is the contract.

[**Goals**]

> One line per bullet, ≤80 chars, verb-led, capability-oriented (the *what*, not the *how*). Soft cap: 5. If you have more goals, you are listing implementation steps — promote them to Constraints or drop them.
>
> Good: `G-1: ark context prints a JSON snapshot of git + tasks + specs.`
> Bad:  `G-1: Two flags control output: --scope {session|phase} and --for {design|...} ...`  ← that's a Constraint.

- G-1:
- G-2:
- G-3:

[**Non-goals**]

> Only list when a reasonable reader would assume the item is in scope. Skip blanket exclusions of features nobody requested. Soft cap: 3.

- NG-1:

[**Architecture**]

> Module / file layout with a one-line note per file. Prefer a tree or diagram; avoid prose narration.

```
<directory tree or component diagram>
```

[**Data Structure**]

> Public types only. Field names + types + a one-line comment when meaning is non-obvious.

```rust
struct ...
enum ...
trait ...
```

[**API Surface**]

> Public function signatures + one-line semantics. No bodies.

```rust
fn ...
```

[**Constraints**]

> Invariants the implementation must hold. **One declarative sentence each, ≤120 chars.** Cite the source of truth (a constant, a test, a path) when one exists. The *why* belongs in Trade-offs, not here.
>
> Good: `C-1: ark context emits exactly one stdout write per invocation.`
> Bad:  `C-1: ark context emits exactly one stdout write per invocation: JSON via a single pre-rendered string + trailing newline, text via a single Display write + trailing newline. No interspersed debug prints.`  ← collapse to the first sentence; the elaboration is implementation.

- C-1:
- C-2:

---

## Runtime

[**Main Flow**]

1.
2.

[**Failure Flow**]

1.
2.

[**State Transitions**]

- State A → State B when …

---

## Implementation

[**Phase 1**]

[**Phase 2**]

[**Phase 3**]

---

## Trade-offs

- T-1: <option A vs option B; adv. / disadv.>
- T-2:

---

## Validation

[**Unit Tests**]

- V-UT-1:

[**Integration Tests**]

- V-IT-1:

[**Failure / Robustness**]

- V-F-1: <failure / retry / rollback / crash / timeout>

[**Edge Cases**]

- V-E-1: <duplicate / empty / max / invalid input / concurrency / boundary>

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | … |
| C-1 | … |
