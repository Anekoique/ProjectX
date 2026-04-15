# Workflow overview

ProjectX uses a **spec- and doc-driven iteration workflow**. The
canonical rules live in [`/AGENTS.md`](../../../AGENTS.md); this page
explains the shape at a glance.

## Three locations per feature

1. **`docs/tasks/<feature>/`** — in-flight workspace. Holds
   `NN_PLAN.md` / `NN_REVIEW.md` / `NN_MASTER.md` rounds as the design
   converges.
2. **`docs/spec/<feature>/SPEC.md`** — landed canonical spec.
   Authored by extracting the final PLAN's `## Spec` section
   (Goals / Architecture / Invariants / Data Structure / API Surface /
   Constraints).
3. **`docs/archived/<category>/<feature>/`** — iteration history,
   moved out of `tasks/` once the feature lands.

## Iteration loop

Per feature, up to 5 rounds (`00` – `04`):

```
plan-executor  → NN_PLAN.md
(main session stops)
external reviewer (codex / human) → NN_REVIEW.md
(optional) user → NN_MASTER.md
→ next plan-executor, or → implementation
```

**Loop cap.** If the reviewer returns APPROVED earlier (no
CRITICAL / HIGH findings) or after round 04, proceed to
implementation. Any surviving MEDIUM / LOW findings are addressed
inline during implementation.

## Implementation

- Implementation (code **and** `NN_IMPL.md`) is authored by the
  **main session directly** — not by a sub-agent.
- There is **no** post-implementation review artifact. Audit findings
  are applied inline in the same session.

## Categories at landing

When a feature lands, choose the archive category that matches the
dominant intent:

| Category | Trigger |
|----------|---------|
| `feat` | New user-visible or API-visible capability |
| `fix` | Bug or MANUAL_REVIEW finding that isn't a reorg |
| `refactor` | Reshape code without changing behavior |
| `perf` | Measurable speedup under a published exit gate |
| `review` | Audit / retrospective not tied to one feature |

When a task has mixed intent, split it.

## Continuing reading

- [Opening a new feature](./new-feature.md)
- [Writing a SPEC](./writing-spec.md)
- [Adding a benchmark](./adding-benchmark.md)
- [`/AGENTS.md`](../../../AGENTS.md) — canonical workflow spec
- [`docs/tasks/README.md`](../../tasks/README.md) — active-feature
  lifecycle and category heuristics
