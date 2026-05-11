# Workflow overview

ProjectX uses the **Ark** CLI to drive a spec-driven iteration
workflow. The canonical rules live in
[`/.ark/workflow.md`](../../../.ark/workflow.md); this page is a quick
orientation.

## Three locations per feature

1. **`.ark/tasks/<slug>/`** — in-flight workspace. Holds `PRD.md`,
   `NN_PLAN.md`, `NN_REVIEW.md` (and optional human-authored
   `NN_MASTER.md`-style overrides via `task.toml`) as the design
   converges. Deep-tier tasks run inside their own git worktree at
   `.ark/worktrees/<branch>/`.
2. **`.ark/specs/features/<slug>/SPEC.md`** — landed canonical spec.
   On deep-tier `ark agent task commit`, the latest PLAN's `## Spec`
   block is extracted verbatim (Goals / Non-goals / Architecture /
   Data Structure / API Surface / Constraints / CHANGELOG).
3. **`.ark/tasks/archive/YYYY-MM/<slug>/`** — iteration history,
   relocated by `ark archive` once the task is `Committed`.

Legacy iteration history from before Ark adoption lives at
`.ark/tasks/archive/legacy/<slug>/` and stays there permanently.

## Tiers

| Tier | When | Artifacts |
|------|------|-----------|
| Quick | Trivial, reversible (typo, version bump) | `PRD.md` only |
| Standard | Feature work, testable scope, no API/architecture break | `PRD.md`, `PLAN.md`, `VERIFY.md` |
| Deep | Architectural, cross-cutting, new subsystem | `PRD.md`, `NN_PLAN.md` ⇄ `NN_REVIEW.md` loop, `VERIFY.md`, promoted `SPEC.md` |

Pick the smallest tier that fits. Promote mid-flight with
`ark agent task promote --to <tier>` if scope grows.

## Iteration loop (deep tier)

```
ark agent task new --slug <s> --title "<t>" --tier deep --worktree
ark agent task plan       → NN_PLAN.md
(main session stops)
external reviewer         → NN_REVIEW.md
(optional) user           → MASTER directive in task.toml
ark agent task plan       → next round, OR
ark agent task execute    → implementation
```

**Loop cap.** `task.toml.max_iterations` (typically 3–5). If
exhausted, halt and surface to the user. Any surviving MEDIUM / LOW
findings are addressed inline during implementation.

## Implementation

- Implementation (code **and** updates to the latest PLAN if the design
  needs adjustment) is authored by the **main session directly** —
  not by a sub-agent.
- VERIFY produces an auto-populated checklist; resolve every item
  before `ark agent task commit`.

## Slash commands

- `/ark:quick "<title>"` — start a quick-tier task.
- `/ark:design "<title>"` — start a standard-tier task.
- `/ark:design --deep "<title>"` — start a deep-tier task with worktree.
- `/ark:commit -m "<msg>"` — atomic VERIFY-gated commit.

## Continuing reading

- [Opening a new feature](./new-feature.md)
- [Writing a SPEC](./writing-spec.md)
- [Adding a benchmark](./adding-benchmark.md)
- [`/.ark/workflow.md`](../../../.ark/workflow.md) — canonical Ark workflow
- [`/AGENTS.md`](../../../AGENTS.md) — project standards
