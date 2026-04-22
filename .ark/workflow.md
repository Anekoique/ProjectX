# Ark Workflow

How work flows from intent to archive. Read before starting any task.

---

## 1. Principles

1. **Right ceremony for the right task.** Three tiers. Pick the smallest that fits.
2. **Intent before edits.** Write down what the change is before touching code.
3. **Review is a gate, not a ritual.** Verdicts block progress; do not fabricate compliance.
4. **Archive is memory.** Every completed task leaves a traceable record.

---

## 2. Layout

```
.ark/
├── workflow.md
├── templates/             # read-only source templates
│   ├── PRD.md
│   ├── PLAN.md
│   ├── REVIEW.md
│   ├── VERIFY.md
│   └── SPEC.md
├── tasks/
│   ├── <slug>/            # active task
│   │   ├── task.toml      #   phase, tier, dates
│   │   ├── PRD.md         #   all tiers — design-phase artifact
│   │   ├── NN_PLAN.md     #   standard (NN=00) / deep (iterated)
│   │   ├── NN_REVIEW.md   #   deep only — pairs with NN_PLAN
│   │   └── VERIFY.md      #   standard + deep
│   └── archive/YYYY-MM/<slug>/
└── specs/
    ├── project/<name>/SPEC.md     # user-authored conventions
    └── features/<name>/SPEC.md    # promoted on archive (deep)
```

---

## 3. Tiers

| Tier | Command | Artifacts | Path through states |
|------|---------|-----------|---------------------|
| Quick | `/ark:quick` | `PRD.md` | design → execute → archived |
| Standard | `/ark:design` | `PRD.md`, `PLAN.md`, `VERIFY.md` | design → plan → execute → verify → archived |
| Deep | `/ark:design --deep` | `PRD.md`, `NN_PLAN.md`, `NN_REVIEW.md`, `VERIFY.md`, promoted `SPEC.md` | design → plan ⇄ review → execute → verify → archived |

PRD captures *what we're building and why* (What / Why / Outcome) and is produced in the `design` state by every tier. PLAN elaborates *how*. VERIFY checks the shipped code against PRD's Outcome and PLAN's Validation.

```
quick:    reversible + no new abstractions
deep:     breaking / cross-cutting / new subsystem
standard: everything else
```

Promote mid-flight with `ark task promote --to <tier>`; prior artifacts become historical context.

---

## 4. Lifecycle

```
       ┌────────────┐
       │  /ark:*    │  slash command starts a task
       └─────┬──────┘
             ▼
       ┌────────────┐
       │  DESIGN    │  write PRD.md — What / Why / Outcome
       └─────┬──────┘  brainstorm (quick: none, standard: ≤3 Qs, deep: thorough)
             │
             │  (quick skips plan/review/verify)
             ▼
       ┌────────────┐
       │    PLAN    │  write NN_PLAN.md — elaborate how
       └─────┬──────┘
             │
             │         (deep only — plan review loop)
             │         ┌──────────────┐
             │         │    REVIEW    │  NN_REVIEW.md
             ├────────►│              │  loop until Approved
             │         └──────┬───────┘
             │ ◄─── rejected ─┘
             ▼
       ┌────────────┐
       │  EXECUTE   │  implement; update PLAN's Spec section if gaps found
       └─────┬──────┘
             ▼
       ┌────────────┐
       │   VERIFY   │  single-pass gate — plan fidelity, correctness,
       └─────┬──────┘  code quality, organization, abstraction
             │         rejected → halt for user decision
             ▼
       ┌────────────┐
       │  ARCHIVE   │  move to tasks/archive/YYYY-MM/;
       └────────────┘  deep: extract SPEC → specs/features/<name>/
```

| Stage | Artifact | Purpose | Gate to next |
|-------|----------|---------|--------------|
| DESIGN | `PRD.md` | Capture *what* and *why*; set Outcome criteria | PRD drafted |
| PLAN | `NN_PLAN.md` | Elaborate *how* — spec, runtime, implementation, validation | PLAN complete; Goals mapped to Validation |
| REVIEW | `NN_REVIEW.md` | Pre-execute gate (deep only, iterative) — does the plan hold up? | Verdict *Approved*; zero open **CRITICAL** (else loop to plan) |
| EXECUTE | code + updated PLAN Spec | Implement; update PLAN's Spec section if gaps emerge | Implementation complete; checks pass |
| VERIFY | `VERIFY.md` | Post-execute gate (single-pass) — plan fidelity, correctness, quality, organization, abstraction, SPEC drift | Verdict *Approved* or *Approved with Follow-ups* (rejection halts for user decision) |
| ARCHIVE | moved to `tasks/archive/YYYY-MM/<slug>/`; deep extracts SPEC | Preserve as memory | — |

---

## 5. Specs

Two layers: `specs/project/<name>/SPEC.md` (user-authored conventions) and `specs/features/<name>/SPEC.md` (extracted from deep-tier PLANs on archive).

**Read pattern.**
- **Project specs** — read every SPEC listed in `specs/project/INDEX.md` before any task. These are conventions that apply always.
- **Feature specs** — scan `specs/features/INDEX.md`, then read only the SPECs the task touches. Record them in PRD's `[**Related Specs**]` so VERIFY can check adherence.

**Archive promotion (deep tier).** Extract the final PLAN's Spec section to `specs/features/<name>/SPEC.md` and append a row to `specs/features/INDEX.md` (managed block). If the task modified an existing feature SPEC, append a `[**CHANGELOG**]` entry to that SPEC and update the INDEX row's promotion date.

**Divergence.** If a PLAN contradicts an existing feature SPEC, REVIEW flags it. Either the PLAN conforms or explicitly updates the SPEC.

---

## 6. Archive

- Active: `.ark/tasks/<slug>/`
- Archived: `.ark/tasks/archive/YYYY-MM/<slug>/` (month = archive date).
- Deep tier: extract final PLAN's Spec section → `specs/features/<name>/SPEC.md`; append CHANGELOG when modifying an existing SPEC.
- Reopen: `ark task reopen <slug>` — refused if same-slug active exists.
