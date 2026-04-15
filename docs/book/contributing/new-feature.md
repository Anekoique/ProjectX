# Opening a new feature

Step-by-step guide for starting a new feature. Assumes you've read
[Workflow overview](./workflow.md).

## 1. Pick a name

A short `camelCase` identifier — no spaces, no slashes. Examples:
`vgaConsole`, `perfIcacheV2`, `rvv`, `sbiDebug`.

## 2. Create the task workspace

```bash
mkdir -p docs/tasks/<feature>
cp docs/template/PLAN.template   docs/tasks/<feature>/00_PLAN.md
cp docs/template/REVIEW.template docs/tasks/<feature>/00_REVIEW.md
cp docs/template/MASTER.template docs/tasks/<feature>/00_MASTER.md
```

Create all three files at the **start** of the round, even if some
are empty. Reviewer and user fill them in turn.

## 3. Author `00_PLAN.md`

Dispatch `plan-executor` sub-agent from the main session. The
sub-agent produces the plan — the main session never authors it
directly.

The PLAN must include:

- `## Summary` — one paragraph.
- `## Log` — reviewer-facing changelog. Start empty for round 00.
- `## Spec` with `[**Goals**]` / `[**Architecture**]` /
  `[**Invariants**]` / `[**Data Structure**]` / `[**API Surface**]` /
  `[**Constraints**]`.
- `## Implement` — step-by-step engineering plan.
- `## Trade-offs` — what was considered and rejected.
- `## Validation` — test plan with real code sketches.

## 4. Get `NN_REVIEW.md`

The main session **stops** after round 00's PLAN. You invoke an
external reviewer (codex / human) to produce `00_REVIEW.md`.

Classify findings: `CRITICAL` / `HIGH` / `MEDIUM` / `LOW`.

## 5. Optional `NN_MASTER.md`

If you want to override the review or add binding directives, write
`00_MASTER.md` yourself. `MUST` directives are binding on the next
PLAN; `SHOULD` directives need explicit response if rejected.

## 6. Iterate

Signal the main session to dispatch round `01`. The next PLAN must:

- Have a **Response Matrix** mapping every prior CRITICAL / HIGH
  finding + MASTER directive to a resolution.
- Address all MASTER `MUST` directives unconditionally.

## 7. Implement

After the final approved PLAN (up to round 04), the main session
authors the code changes **and** `NN_IMPL.md` directly. Include:

- **What shipped** vs what was planned.
- **Deviations** from the plan, with justification.
- **Validation results** — tests run, exit gates met.

## 8. Land

- **Extract SPEC.** Copy the final PLAN's `## Spec` section into
  `docs/spec/<feature>/SPEC.md`.
- **Archive.** `git mv docs/tasks/<feature>  docs/archived/<category>/<feature>`.
- **Update PROGRESS.md** — add the landed feature to the appropriate
  phase or task table.

## Do-nots

- Don't edit previous iteration documents. Always create the next
  numbered file.
- Don't silently deviate during implementation. If the design
  changes meaningfully, open a new iteration.
- Don't dispatch reviewer sub-agents for the PLAN review — reviews
  are external, out-of-session.
