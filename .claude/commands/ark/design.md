---
description: Start a standard or deep-tier task. Produces PRD → PLAN → (REVIEW loop if --deep) → EXECUTE → VERIFY.
argument-hint: "[--deep] <title>"
---

# `/ark:design $ARGUMENTS`

Create a **standard**-tier task (default) or **deep**-tier task (if `--deep` is in `$ARGUMENTS`).

- **Standard** — feature work with testable scope. Single PLAN, no REVIEW loop, single VERIFY gate.
- **Deep** — architectural / cross-cutting work. Iterated PLAN ⇄ REVIEW loop, VERIFY gate, SPEC extracted on commit.

Parse `$ARGUMENTS`: if it contains `--deep`, tier = deep, title = remainder; else tier = standard, title = `$ARGUMENTS`.

Structural ops (task dirs, phase transitions, SPEC extraction, INDEX upserts) are owned by `ark agent`. Artifact bodies (PRD prose, PLAN sections, REVIEW findings) are yours to write.

## Preconditions

- `.ark/` is initialized.
- **Standard:** scope is feature-shaped, testable, doesn't break APIs/architecture. If it does, use `--deep`.
- **Deep:** scope is architectural, cross-cutting, or introduces a new subsystem.

## Phase 1 — DESIGN

### Step 1.1: Pull design-phase context `[AI]`

```bash
ark context --scope phase --for design --format json
```

See `workflow.md` §4 for the projection contract. Read every SPEC under `specs.project` and any `specs.features` rows that look related.

### Step 1.2: Brainstorm `[AI]` `[USER]`

- **Standard:** ≤3 clarifying questions on what's ambiguous in the title (observable outcome? constraints? existing patterns to follow?).
- **Deep:** thorough brainstorm — problem framing, non-goals, performance/security/compat boundaries, alternatives + why rejected, risks/assumptions, interaction with existing feature SPECs.

Do not proceed until the user confirms direction.

### Step 1.3: Create the task `[AI]`

Slugify the title (lowercase, hyphen-separated, ASCII, ≤40 chars).

```bash
# standard tier — opt in to --worktree only if work would collide with in-flight changes
ark agent task new --slug <slug> --title "<title>" --tier standard

# deep tier — --worktree is REQUIRED
ark agent task new --slug <slug> --title "<title>" --tier deep --worktree
```

Scaffolds `.ark/tasks/<slug>/{PRD.md, task.toml}`, registers the slug, sets this session's focus. **Deep tier:** `cd .ark/worktrees/<branch>/` and run all subsequent steps from there.

### Step 1.4: Fill the PRD `[AI]` `[USER]`

Edit `.ark/tasks/<slug>/PRD.md`: **What**, **Why**, **Outcome**, **Related Specs** (one bullet per touched feature SPEC + how it interacts).

**Deep — dispatch `ark-researcher`** when PRD authoring hits a third-party library, prior-art comparison, or cross-cutting pattern map. Findings land at `<task>/research/<topic>.md`. After dispatch, run `git status`; `git restore` any out-of-scope edits and `git clean -fd` any out-of-scope new files (the researcher's allowed write scope is `<task>/research/` only).

**Gate:** PRD complete → Phase 2.

## Phase 2 — PLAN

### Step 2.1: Refresh phase context `[AI]`

```bash
ark context --scope phase --for plan --format json
```

### Step 2.2: Advance phase `[AI]`

```bash
ark agent task plan
```

Transitions to `Plan` and seeds `PLAN.md` (standard) or `00_PLAN.md` (deep).

### Step 2.3: Fill the PLAN `[AI]`

Per the template: `## Summary`, `## Log` (*None in 00_PLAN*), `## Spec` (Goals/NG/Architecture/Data Structure/API Surface/Constraints), `## Runtime`, `## Implementation`, `## Trade-offs`, `## Validation` with Acceptance Mapping.

**Spec discipline (the `## Spec` section is what gets promoted to a feature SPEC verbatim on deep commit — write it like a contract, not a narrative):**

- Goals: one line each, ≤80 chars, verb-led (the *what*). Soft cap of 5.
- Non-goals: list only when a reader would assume in-scope. Soft cap of 3.
- Constraints: one declarative sentence each, ≤120 chars. The *why* goes in Trade-offs, not the Constraint body.
- If a goal sounds like a procedure ("Two flags control X..."), it is a Constraint, not a Goal.

**Deep — dispatch `ark-researcher`** for library/API choices or pattern comparisons that PLAN authoring cannot resolve from training. Same post-check: `git status`, `git restore` out-of-scope edits, `git clean -fd` out-of-scope new files.

**Gate:** every Goal `G-N` mapped to ≥1 Validation `V-*-N` in the Acceptance Mapping table.

### Step 2.4: Advance `[AI]`

```bash
# standard
ark agent task execute   # → Phase 4

# deep
ark agent task review    # → Phase 3
```

## Phase 3 — REVIEW (deep only, looped)

### Step 3.1: Refresh phase context `[AI]`

```bash
ark context --scope phase --for review --format json
```

### Step 3.2: Act as reviewer `[AI]` (preferably a fresh agent)

Ask the user: *"Should I self-review, or will you run the reviewer?"*

- **Self:** switch framing — *you are now the reviewer*. Read the latest `NN_PLAN.md` against the PRD and project specs; fill `NN_REVIEW.md` with verdict, findings (`R-NNN`), trade-off advice (`TR-N`).
- **Agent:** dispatch `ark-reviewer`. `git status` after; `git restore` edits outside `NN_REVIEW.md` and `git clean -fd` any new files.

**Reject (HIGH)** if the latest PLAN's `## Spec` references prior iterations instead of restating in full.

### Step 3.3: Loop if revisions needed `[AI]`

If verdict is *Rejected* or *Approved with Revisions*:

1. Copy `.ark/templates/PLAN.md` → `(NN+1)_PLAN.md`; copy `.ark/templates/REVIEW.md` → `(NN+1)_REVIEW.md`.
2. Edit `task.toml`: bump `iteration` to `NN+1`, set `phase = "plan"`, refresh `updated_at`.
3. Fill `(NN+1)_PLAN.md`'s `## Log` Response Matrix — every prior CRITICAL/HIGH finding listed with Accepted/Rejected/Deferred + reasoning. `## Spec` stays self-contained.
4. `ark agent task review` → fill `(NN+1)_REVIEW.md`.
5. Repeat until verdict is *Approved* with zero open CRITICAL.

`task.toml.max_iterations` (typically 3–5 for deep). If exhausted without approval, halt and ask the user.

### Step 3.4: Advance `[AI]`

```bash
ark agent task execute
```

## Phase 4 — EXECUTE

### Step 4.1: Refresh phase context `[AI]`

```bash
ark context --scope phase --for execute --format json
```

### Step 4.2: Implement `[AI]`

Work through the latest PLAN's Implementation phases. Follow project specs and related feature SPECs. **If implementation reveals design gaps, update the latest PLAN's `## Spec` to reflect reality** — do not silently diverge.

Run project checks (tests, lints, builds).

### Step 4.3: Advance `[AI]`

```bash
ark agent task verify
```

Seeds `VERIFY.md` with auto-populated sections.

## Phase 5 — VERIFY

### Step 5.1: Refresh phase context `[AI]`

```bash
ark context --scope phase --for verify --format json
```

### Step 5.2: Maintain VERIFY.md `[AI]` (preferably a fresh agent)

Ask the user: *"Should I self-verify, or will you run the verifier?"*

- **Self:** apply the higher quality bar — plan fidelity, correctness, code quality, abstraction, SPEC drift. Resolve every item PASS / FAIL / N/A; capture cross-cutting observations as Findings (`V-NNN`) with a Resolution.
- **Agent:** dispatch `ark-verifier`. Runs the project's build / test / lint / format-check; fills `VERIFY.md`. Does not self-fix — FAIL items return to the main session. `git status` after; `git restore` edits outside `VERIFY.md` and `git clean -fd` any new files.

**No verdict line — completion = no `PENDING`.**

Stems `ark-researcher`/`ark-reviewer`/`ark-verifier` are reserved; user agents at those stems are overwritten on `init`/`upgrade`/`load`.

### Step 5.3: Close out `[AI]` then `[USER]`

- **All items resolved** → tell the user: *"Stage your work with `git add <files>`, then run `/ark:commit -m \"<message>\"`."* Do NOT commit automatically.
- **Open Findings** → halt, summarize, ask the user how to proceed (fix tasks, tier promotion via `ark agent task promote`, accept with acknowledgement, discard).

## Failure Modes

| Code | Cause | Recovery |
|------|-------|----------|
| `IllegalPhaseTransition` | `task <verb>` called from wrong phase | Re-check `task.toml.phase`; advance from the correct phase |
| `WrongTier` | tier-specific verb on the wrong tier | Promote with `ark agent task promote --to <tier>` |
| `TaskNotFound` | slug not in `tasks.active` | Check active set with `ark context --scope session` |

## See Also

- `workflow.md` §3 (tiers), §4 (phase contracts), §5 (lifecycle), §6 (specs)
- `/ark:commit` — closure contract; `/ark:resume`, `/ark:discard` — focus / cleanup
