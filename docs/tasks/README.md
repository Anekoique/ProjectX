# `docs/tasks/` — Active feature workspace

This directory is the scratchpad for **in-flight** features whose
PLAN ↔ REVIEW ↔ MASTER rounds have not yet converged.

## Layout

Each active feature gets its own subdirectory:

```
docs/tasks/<feature>/
├── 00_PLAN.md       ← authored by plan-executor sub-agent
├── 00_REVIEW.md     ← authored by external reviewer (codex / human)
├── 00_MASTER.md     ← optional, authored by user
├── 01_PLAN.md
├── 01_REVIEW.md
├── ...              ← loop capped at 5 rounds (see /AGENTS.md)
└── NN_IMPL.md       ← authored by main session after approval
```

## Lifecycle

1. **Open a task.** Create `docs/tasks/<feature>/` and copy templates
   from [`../template/`](../template/) for round `00`.
2. **Iterate.** PLAN → external REVIEW → optional MASTER → next round,
   up to 5 rounds. See [`/AGENTS.md`](../../AGENTS.md) for loop rules.
3. **Implement.** After the final approved PLAN, the main session
   authors code changes and `NN_IMPL.md` directly.
4. **Land.**
   - **Spec:** extract the final PLAN's `## Spec` (Goals / Architecture /
     Invariants / Data Structure / API Surface / Constraints) into
     `docs/spec/<feature>/SPEC.md`.
   - **Archive:** move the whole task directory into the appropriate
     archive category:
     - `docs/archived/feat/<feature>/` — new capability
     - `docs/archived/fix/<feature>/` — corrects a bug or MANUAL_REVIEW finding
     - `docs/archived/refactor/<feature>/` — reorganisation without new capability
     - `docs/archived/perf/<feature>/` — performance optimisation
     - `docs/archived/review/` — review-only artifacts (audits, retros)
   - **Progress:** append the landed feature to the appropriate phase
     in [`../PROGRESS.md`](../PROGRESS.md).

## Category heuristics

Pick the archive category that matches the feature's **dominant
intent**, not just its side effects:

| Category | Trigger | Example from history |
|----------|---------|----------------------|
| `feat` | Adds new user-visible or API-visible capability | `boot`, `devices`, `float`, `multiHart` |
| `fix` | Corrects a bug, incorrect behavior, or MANUAL_REVIEW finding that isn't a reorg | `directIrq`, `plicGateway` |
| `refactor` | Reshapes code without changing behavior or capability | `archModule`, `archLayout`, `aclintSplit`, `err2trap` |
| `perf` | Primary goal is measurable speedup under a published exit gate | `perfBusFastPath`, `perfHotPath`, `memOpt` |
| `review` | Audit, design-review, or retrospective document (not tied to a single feature) | `MANUAL_REVIEW.md` |

When a task has mixed intent, split it into separate tasks.

## Not for

- Landed features — those live in `spec/` + `archived/`.
- Long-term design musings without a PLAN — those belong in a personal
  note or issue tracker, not in version control under `tasks/`.
