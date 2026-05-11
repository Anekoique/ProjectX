# Opening a new feature

Step-by-step guide for starting a new feature. Assumes you've read
[Workflow overview](./workflow.md).

## 1. Pick a slug

A short `camelCase` identifier — no spaces, no slashes. Examples:
`vgaConsole`, `perfIcacheV2`, `rvv`, `sbiDebug`.

## 2. Create the task

Pick the tier (Quick / Standard / Deep — see [workflow.md](./workflow.md))
and run the matching slash command:

```bash
/ark:quick "<title>"               # trivial, reversible — PRD only
/ark:design "<title>"              # standard — PRD + PLAN + VERIFY
/ark:design --deep "<title>"       # deep — adds REVIEW loop + promoted SPEC
```

For deep tier, Ark creates a git worktree at
`.ark/worktrees/<branch>/` and switches focus to it. `cd` into the
worktree before any further `ark` command.

## 3. Author `PRD.md`

The PRD answers four questions:

- **What** — one-line description of the change.
- **Why** — bug, cleanup, policy, user request, architectural need.
- **Outcome** — observable success criteria. For quick tier this is
  the verification checklist.
- **Related Specs** — feature specs from `.ark/specs/features/`
  this task touches. List paths + one line on how each interacts.

For standard / deep tier, advance to PLAN:

```bash
ark agent task plan
```

## 4. Author `PLAN.md` (or `00_PLAN.md` for deep)

Dispatch the `plan-executor` sub-agent. The PLAN must include:

- `## Summary` — one paragraph.
- `## Log` — reviewer-facing changelog. `None in 00_PLAN` for the
  first deep-tier round; subsequent rounds list Added / Changed /
  Removed / Unresolved + a **Response Matrix** for prior findings.
- `## Spec` with `[**Goals**]` / `[**Non-goals**]` /
  `[**Architecture**]` / `[**Data Structure**]` / `[**API Surface**]` /
  `[**Constraints**]` / `[**CHANGELOG**]`. **Self-contained every
  iteration** — promoted verbatim to
  `.ark/specs/features/<slug>/SPEC.md` on commit.
- `## Runtime` — main / failure flow.
- `## Implementation` — phased steps.
- `## Trade-offs` — options with adv. / disadv.
- `## Validation` — Unit / Integration / Failure / Edge tests +
  Acceptance Mapping.

## 5. Deep-tier REVIEW loop

For deep tier:

```bash
ark agent task review     # transitions phase → review
```

The main session **stops**. Invoke an external reviewer (codex /
human / a different agent) to produce `NN_REVIEW.md`. Classify
findings as `CRITICAL` / `HIGH` / `MEDIUM` / `LOW`.

If verdict is *Rejected* or *Approved with Revisions*:

```bash
cp .ark/tasks/<slug>/NN_PLAN.md   .ark/tasks/<slug>/$(printf '%02d' $((NN+1)))_PLAN.md
cp .ark/tasks/<slug>/NN_REVIEW.md .ark/tasks/<slug>/$(printf '%02d' $((NN+1)))_REVIEW.md
# edit task.toml: bump iteration, set phase = "plan"
ark agent task review     # transitions back after the new PLAN is filled
```

The new PLAN's `## Log` must map every prior CRITICAL / HIGH finding
to Accepted / Rejected / Deferred + reasoning.

Repeat until verdict is **Approved** or `max_iterations` is reached.

## 6. EXECUTE

```bash
ark agent task execute
```

Implementation happens in the main session directly. Follow the
latest PLAN's `## Implementation` phases. If implementation reveals
design gaps, **update the latest PLAN's `## Spec`** — do not silently
diverge.

## 7. VERIFY

```bash
ark agent task verify
```

`VERIFY.md` is seeded with auto-populated sections (Project Spec
Compliance, Related Feature Spec Compliance, PRD Constraints, Plan
Fidelity, SPEC Drift). Resolve every item to PASS / FAIL / N/A with
explanation. Quality bar covers plan fidelity, correctness, code
quality, abstraction, SPEC drift — not just "does it work".

## 8. COMMIT

Stage your work first, then commit atomically:

```bash
git add <files>
/ark:commit -m "<message>"
```

For deep tier, the commit extracts the final PLAN's `## Spec` to
`.ark/specs/features/<slug>/SPEC.md` and appends a row to
`.ark/specs/features/INDEX.md` — all in one commit.

Later, `ark archive` relocates committed tasks to
`.ark/tasks/archive/YYYY-MM/<slug>/`.

## Do-nots

- Don't edit previous iteration documents. Always create the next
  numbered file.
- Don't silently deviate during implementation. If the design changes
  meaningfully, open a new iteration.
- Don't dispatch reviewer sub-agents for the deep-tier PLAN review —
  reviews are external, out-of-session.
