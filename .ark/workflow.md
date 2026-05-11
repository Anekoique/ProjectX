# Ark Workflow

The CLI drives the workflow. Every transition is an `ark` command. Read this once; return to a section when you need detail.

---

## Quick Start

### Step 1: Get session context

```bash
ark context
```

Shows git state, active tasks, project specs, feature specs, recent archive, current focus.

### Step 2: Read project specs

Read every SPEC listed under `specs.project` in the context output. These conventions always apply.

```bash
cat .ark/specs/project/INDEX.md
cat .ark/specs/project/<name>/SPEC.md   # one per row
```

### Step 3: Pick a tier and start

```bash
/ark:quick "<title>"               # trivial, reversible — PRD only
/ark:design "<title>"              # standard — PRD + PLAN + VERIFY
/ark:design --deep "<title>"       # deep — adds REVIEW loop + promoted SPEC
```

---

## Tiers

Pick the smallest tier that fits.

- **Quick** — reversible in one commit, no new abstractions. Artifact: `PRD.md`.
- **Standard** — feature work with testable scope, no API/architecture break. Artifacts: `PRD.md`, `PLAN.md`, `VERIFY.md`.
- **Deep** — architectural, cross-cutting, or new subsystem. Artifacts: `PRD.md`, `NN_PLAN.md` ⇄ `NN_REVIEW.md` (looped), `VERIFY.md`, promoted `SPEC.md`.

Promote mid-flight when scope grows: `ark agent task promote --to <tier>`. Prior artifacts are preserved.

When in doubt, pick lower. Promotion is cheap; demotion is awkward.

---

## Layout

```
.ark/
├── workflow.md
├── templates/                # seed templates (read-only)
├── tasks/<slug>/             # active task
│   ├── task.toml             # phase, tier, dates
│   ├── PRD.md
│   ├── PLAN.md / NN_PLAN.md
│   ├── NN_REVIEW.md          # deep only
│   └── VERIFY.md             # standard + deep
├── tasks/archive/YYYY-MM/    # closed tasks
└── specs/
    ├── project/<name>/SPEC.md     # user-authored
    └── features/<name>/SPEC.md    # promoted on deep commit
```

---

## Lifecycle

Each phase: pull context, run the CLI, write the artifact, advance.

```
DESIGN  → PLAN  → [REVIEW ⇄ PLAN]  → EXECUTE  → VERIFY  → COMMIT  → (later) ARCHIVE
            quick skips PLAN/REVIEW/VERIFY; standard skips REVIEW
```

### DESIGN — write PRD

```bash
ark context --scope phase --for design --format json
ark agent task new --slug <slug> --title "<title>" --tier <quick|standard|deep> [--worktree]
```

- Read every project SPEC and any related feature SPECs from the context output.
- Brainstorm: quick = none; standard ≤3 questions; deep = thorough.
- Fill `PRD.md`: **What** / **Why** / **Outcome** / **Related Specs**.
- Deep tier: `--worktree` is required; then `cd .ark/worktrees/<branch>/`.

**Gate:** PRD has What, Why, Outcome filled. Quick → EXECUTE; standard/deep → PLAN.

### PLAN — elaborate how

```bash
ark context --scope phase --for plan --format json
ark agent task plan
```

Fill the seeded plan file (`PLAN.md` standard, `00_PLAN.md` deep):

- **Summary** — one paragraph.
- **Log** — `None in 00_PLAN`; on later iterations, list Added / Changed / Removed / Unresolved + Response Matrix.
- **Spec** — Goals (`G-N`), Non-goals, Architecture, Data Structure, API Surface, Constraints (`C-N`). **Self-contained every iteration.** Promoted verbatim to `specs/features/<slug>/SPEC.md` on deep commit.
- **Runtime** — main / failure flow + state transitions.
- **Implementation** — phases.
- **Trade-offs** — options with adv. / disadv.
- **Validation** — Unit / Integration / Failure / Edge tests + Acceptance Mapping.

**Gate:** every `G-N` mapped to ≥1 `V-*-N` in Acceptance Mapping.

```bash
ark agent task execute   # standard → EXECUTE
ark agent task review    # deep → REVIEW
```

### REVIEW — deep only, looped

```bash
ark context --scope phase --for review --format json
```

Reviewer (ideally a fresh agent or different model) fills `NN_REVIEW.md`:

- Verdict: Approved / Approved with Revisions / Rejected.
- Findings (`R-NNN`): Severity, Section, Problem, Why it matters, Recommendation.
- Trade-off Advice (`TR-N`).

Reject as HIGH if the latest PLAN's `## Spec` references prior iterations instead of restating in full.

If verdict is *Rejected* or *Approved with Revisions*:

```bash
cp .ark/tasks/<slug>/NN_PLAN.md   .ark/tasks/<slug>/$(printf '%02d' $((NN+1)))_PLAN.md
cp .ark/tasks/<slug>/NN_REVIEW.md .ark/tasks/<slug>/$(printf '%02d' $((NN+1)))_REVIEW.md
# edit task.toml: bump iteration, set phase = "plan"
ark agent task review    # transitions back after the new PLAN is filled
```

Fill the new plan's `## Log` Response Matrix — every prior CRITICAL/HIGH finding listed with Accepted / Rejected / Deferred + reasoning. Repeat until verdict is Approved with zero open CRITICAL.

`task.toml.max_iterations` (typically 3–5). If exhausted, halt and ask the user.

```bash
ark agent task execute   # → EXECUTE
```

### EXECUTE — implement

```bash
ark context --scope phase --for execute --format json
```

- Work through the latest PLAN's Implementation phases.
- Follow project SPECs and related feature SPECs.
- If implementation reveals design gaps, **update the latest PLAN's `## Spec`**. Do not silently diverge.
- Run project checks (tests, lints, builds).

```bash
ark agent task verify    # → VERIFY (seeds VERIFY.md)
```

### VERIFY — audit shipped code

```bash
ark context --scope phase --for verify --format json
```

`VERIFY.md` is seeded with auto-populated checklist sections. Resolve each item:

- **Project Spec Compliance** — one item per registered SPEC. Mark PASS / FAIL / N/A with explanation.
- **Related Feature Spec Compliance** — one item per SPEC the PRD listed.
- **PRD Constraints** — one item per Outcome criterion.
- **Plan Fidelity** — one item per Goal `G-N`. PASS when delivered, FAIL when not, N/A when withdrawn (PLAN's Log explains).
- **SPEC Drift** — PASS once any modified feature SPEC has a `[**CHANGELOG**]` entry.

Add **Findings** (`V-NNN`) for cross-cutting issues that don't map to a single seeded item: Severity, Location, Problem, Why it matters, Recommendation, Resolution (`PENDING` / `FIXED in <ref>` / `ACCEPTED — <reason>`).

**Gate:** no item is `PENDING`. No verdict line. Quality bar covers plan fidelity, correctness, code quality, abstraction, SPEC drift — not just "does it work".

When all items resolve, tell the user:

> Stage your work with `git add <files>`, then run `/ark:commit -m "<message>"`.

Do not commit automatically — staging is the user's step.

### COMMIT — close atomically

```bash
git add <files>                                          # USER stages work first
ark context --scope phase --for commit --format json
ark agent task commit -m "<message>"
# or:  ark agent task commit --no-commit
```

**Preconditions:**
- User has staged their work (CLI errors `NothingStaged` on empty index).
- Quick: `phase = execute`. Standard / Deep: `phase = verify`.
- Deep: VERIFY has no `PENDING` (refused as `VerifyIncomplete`). Standard: PENDING warns but proceeds.

**Compose the message:**
- If `-m` was passed, use verbatim.
- Otherwise: `git diff --cached` to see staged work, `git log -n 5 --oneline` for style, generate Conventional Commits subject (≤70 chars). **Show the message and ask for confirmation before invoking the CLI.**

**Journal entry (workspace, when `.ark/.developer` exists):** append a session block to the path returned by `ark context --scope record`:

```markdown
## Session N: <title>

### Summary

<one line ≤120 chars; user-visible effect>

### Main Changes

| Area   | Description |
| ------ | ----------- |
| <area> | <≤80 chars> |
```

Do not write `**Date**`, `**Slug**`, `**Branch**`, etc. — `task commit` stamps them. Keep Main Changes ≤4 rows; skip incidental rows rather than pad.

**The CLI does, in order:**
1. VERIFY gate.
2. Deep: extract `## Spec` to `specs/features/<slug>/SPEC.md`; upsert features INDEX.
3. Save `task.toml` with `phase = Committed`, `committed_at = now`.
4. Stage exactly the Ark-managed files (no `git add -A`).
5. `git commit -m "<message>"`.

**On `git commit` failure:** scoped rollback restores all snapshots and unstages only what Ark added. User's pre-existing index entries survive.

`--no-commit` flips phase + extracts SPEC but skips `git commit`. User owns any follow-up commit.

**Failure modes:** `NothingStaged` → user runs `git add`. `VerifyIncomplete` → resolve PENDING items. `CommitMessageRequired` → slash command logic bug. `GitCommitFailed` → surface stderr; rollback already happened. `IllegalPhaseTransition` → wrong phase; tell the user the current one.

### ARCHIVE — manager-only bulk move

```bash
ark archive                       # all committed tasks
ark archive --month 2026-05       # one bucket
ark archive --dry-run             # list candidates only
```

Moves every `phase = Committed` task to `tasks/archive/YYYY-MM/<slug>/`. Month derived from each task's own `committed_at`. Side-effect-free — no SPEC promotion (already happened on commit), no journal writes.

Reopen by hand: move the archived dir back to `.ark/tasks/<slug>/`, set `phase = "verify"`, clear `archived_at`. Refused if a same-slug active task exists.

---

## Worktrees

Run multiple tasks in parallel without collisions. Each worktree is a separate git working tree at `.ark/worktrees/<branch>/`.

```bash
ark agent task new --slug <slug> --title "<t>" --tier <t> --worktree
# branch defaults to feat/<slug>; override:
#   --branch-type fix|refactor|docs|...
#   --branch <full-name>
cd .ark/worktrees/<branch>/        # required before any subsequent ark command
```

**Deep tier MUST use `--worktree`.** PLAN ⇄ REVIEW iteration generates many revisions; isolate them in a dedicated branch.

Configure under `[worktree]` in `.ark/config.toml`: `worktree_dir`, `branch_prefix`, files to copy, `post_create` hooks. Preserved across `ark upgrade`.

After the branch is merged, clean up from the parent checkout:

```bash
ark agent task worktree list                          # what's active
ark cleanup                                           # dry-run: every prunable worktree
ark cleanup --apply [--delete-branch] [--force]       # remove dirs + (optionally) branches
ark cleanup --slug <s> --apply                        # remove just one
```

`ark cleanup` surfaces every worktree whose backing task is Committed, Archived, or whose branch is gone locally. Archive does NOT auto-clean worktrees — cleanup is a deliberate step.

---

## Specs

Two layers, opposite ownership rules.

**Project specs** — `specs/project/<name>/SPEC.md`. User-authored conventions. Apply to every task. **Read every entry in `specs/project/INDEX.md` before any task.** Agents never edit project SPECs without explicit instruction.

**Feature specs** — `specs/features/<name>/SPEC.md`. Auto-extracted from deep-tier PLANs at commit. Scan `specs/features/INDEX.md`; read only the SPECs your task touches. **List them in the PRD's `[**Related Specs**]` block** so VERIFY can check adherence.

**SPEC promotion (deep commit, automatic):** `task commit` extracts the final PLAN's `## Spec` to `specs/features/<slug>/SPEC.md` and appends a row to `specs/features/INDEX.md`. Both land in the closing commit. Modifying an existing SPEC appends a `[**CHANGELOG**]` entry instead.

**Divergence:** REVIEW must flag a PLAN that contradicts an existing feature SPEC as CRITICAL. The PLAN either conforms or explicitly updates the SPEC.

---

## CLI surfaces

Three commands. Different stability promises.

**`ark context`** — read-only projections. Semver-stable; slash commands depend on it. Auto-invoked at session start.

```bash
ark context                                            # session scope (default)
ark context --scope phase --for <phase>               # phase scope
ark context --scope record                             # journal scope
ark context [...] --format json                       # machine output
```

Phases: `design`, `plan`, `review`, `execute`, `verify`, `commit`. The `commit` projection is body-free — slash commands read VERIFY.md and the latest plan from the artifact paths it returns.

**`ark archive`** — manager-only bulk move. Semver-stable. Only job: relocate committed tasks. No SPEC promotion, no journal writes.

**`ark cleanup`** — worktree prune. Semver-stable. Dry-run by default; `--apply` removes worktrees of Committed / Archived tasks and worktrees whose branch is gone. Reuses `worktree cleanup` per slug; never touches `task.toml` or state.

**`ark agent`** — structural mutation. **Hidden, not semver-stable.** Errors out on illegal transitions:

- `IllegalPhaseTransition` — wrong phase for this verb.
- `WrongTier` — tier-specific verb on the wrong tier.
- `TaskNotFound` — slug not in `tasks.active`.
- `NoFocus` — non-targeted verb run with no `[focus]` bound; run `ark agent task new` or `task resume --slug <s>` first.

```bash
ark agent task new --slug <s> --title "<t>" --tier <t> [--worktree]
ark agent task plan                                   # → Plan
ark agent task review                                 # → Review (deep)
ark agent task execute                                # → Execute
ark agent task verify                                 # → Verify
ark agent task commit -m "<msg>" | --no-commit       # → Committed (atomic)
ark agent task archive                                # → Archived (single task)
ark agent task resume   --slug <s>                    # focus this session
ark agent task discard  --slug <s> [--force]          # delete unarchived task
ark agent task promote  --to <tier>                   # change tier mid-flight
ark agent task worktree list | cleanup [--delete-branch]
```

`task new`, `task resume`, `task discard` require `--slug`. Other verbs resolve the slug from `.ark/.state.toml`'s `[focus]` field — `task new` and `task resume` write it; `task archive` and `task discard` clear it when their slug matches. With no focus bound, the CLI errors `NoFocus` and asks you to `task resume` first.

**Hand-edited operations (no CLI):**
- Deep-tier iteration: copy `NN_PLAN.md` / `NN_REVIEW.md` to `(NN+1)_*`, bump `task.toml.iteration`, reset `phase = "plan"`.
- Reopen archived task: move dir back to `.ark/tasks/<slug>/`, set `phase = "verify"`, clear `archived_at`.

---

## Focus model

`.ark/.state.toml` (gitignored, per-checkout — each worktree owns its own) carries developer identity, the active-slug set, and a single `focus` slug naming the task this checkout is currently driving. One focus per checkout: deep-tier worktrees own their own `.state.toml` and so their own focus.

- `task new --slug <s>` creates the task and binds focus to `<s>`. Warns when an existing focus is rebound and suggests `--worktree` for parallel work.
- `task resume --slug <s>` switches focus to `<s>`. Idempotent. Same rebind warning as `task new`.
- `task commit` clears focus on success; the slug stays in `tasks.active` until `ark archive` runs.
- `task archive` clears focus when the archived slug matched.
- `task discard --slug <s>` removes an unarchived task and clears focus when it matched. Refused (`TaskStillActive`) if seeded files diverge from templates; pass `--force` to override. **Never pass `--force` on the user's behalf without explicit go-ahead.**

For genuinely parallel work, use a worktree (`task new --worktree`) — each worktree has its own focus, so the two tasks don't share a slot.

---

## Principles

1. **Right ceremony for the right task.** Three tiers — pick the smallest that fits.
2. **Intent before edits.** Write the PRD before touching code.
3. **Review is a gate, not a ritual.** Verdicts block progress; do not fabricate compliance.
4. **Archive is memory.** Every completed task leaves a traceable record.
