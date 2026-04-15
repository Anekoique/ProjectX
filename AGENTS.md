# AGENTS.md

## Development Standards

- **Technical Research**: Always use web search to retrieve the latest official documentation.
- **Code Excellence**: Maintain a **clean, concise, and elegant** codebase. All implementations must strictly conform to the existing framework's architectural style.
- **Code Style:** Use a moderate amount of **functional** programming techniques.
- **Verification**: After making any coding-related modification, you must run `make fmt`, `make clippy`, `make run`, and `make test` to ensure correctness.

## Development Workflow

The project evolves through numbered iterations.

**Layout.** A feature passes through three locations:

1. `docs/tasks/<feature>/` — in-flight. `NN_PLAN.md` / `NN_REVIEW.md` /
   `NN_MASTER.md` rounds accumulate here during the loop.
2. `docs/spec/<feature>/SPEC.md` — landed canonical spec, authored at
   the end of the loop by extracting the final PLAN's `## Spec` section
   (Goals / Architecture / Invariants / Data Structure / API Surface /
   Constraints). Updated when the feature next iterates.
3. `docs/archived/<category>/<feature>/` — iteration history, moved out
   of `tasks/` once the feature lands. Categories:
   - **`feat/`** — new capability
   - **`fix/`** — bug or MANUAL_REVIEW finding that isn't a reorg
   - **`refactor/`** — reshape without new capability
   - **`perf/`** — measurable speedup under an exit gate
   - **`review/`** — audits / retrospectives not tied to one feature

Baselines for measurement-heavy features (perf) live at
`docs/perf/baselines/<date>/` alongside the roadmap in `docs/PROGRESS.md`.
Pre-workflow feature docs (`docs/archived/feat/csr/`, `klib/`, `mm/`,
`refactor/err2trap/`) are preserved verbatim with a banner in their
`spec/<feature>/SPEC.md`; rewrite to the template shape when the
feature next sees meaningful iteration. The `inst` spec has no archive
(the source was a running-notes file, not iteration artifacts).

See [`docs/tasks/README.md`](./docs/tasks/README.md) for the active-to-landed
lifecycle and archive-category heuristics.

### Roles

| Role     | Document             | Responsibility |
| -------- | -------------------- | ------------------------------------------------------------ |
| Executor | `NN_PLAN.md`         | Proposes the implementation plan, including summary, spec alignment, architecture, invariants, API surface, implementation steps, validation strategy, and trade-off analysis. |
| Reviewer | `NN_REVIEW.md`       | Audits the plan for correctness, completeness, spec compliance, edge cases, maintainability, risk, and provides trade-off advice when needed. Findings are classified by severity: CRITICAL / HIGH / MEDIUM / LOW. |
| Master   | `NN_MASTER.md`       | Issues final directives when conflicts, ambiguity, or strategic trade-offs require a decisive override. Executor must unconditionally comply with all `MUST` directives in the next iteration. |

### Iteration Rules

1. Each iteration starts with a PLAN. At the beginning of each round, create the PLAN / REVIEW / MASTER files from the templates first.
2. REVIEW evaluates the PLAN but does not replace it.
3. MASTER is optional, but if present, it is authoritative.
4. The next PLAN must explicitly resolve all blocking REVIEW findings and all MASTER directives.
5. Implementation may begin only when blocking issues are resolved or explicitly waived by MASTER.
6. All PLAN / REVIEW / MASTER documents must follow the templates in `docs/template`.
7. Never overwrite previous iteration documents. Always create the next numbered file.
8. If implementation requires a meaningful design change, open a new iteration instead of silently deviating from the approved PLAN.
9. All iteration artifacts are produced by sub-agents dispatched from the main session,
   never by manual external tooling and never inlined into the main conversation. The
   canonical mapping is:

   | Artifact | Author |
   |----------|--------|
   | `NN_PLAN.md` | **`plan-executor`** sub-agent |
   | `NN_REVIEW.md` | **External reviewer** (e.g. `codex`, a human reviewer, or any off-session agent the user invokes) |

   MASTER documents (`NN_MASTER.md`) are authored by the human user
   and, together with `NN_REVIEW.md`, are the only artifacts the main
   session does not write. The main session never authors `NN_REVIEW.md`
   — reviews always come from outside the session — and never authors
   `NN_PLAN.md` by hand (always via the `plan-executor` sub-agent).

### Iteration Lifecycle

Round N:
1. Executor dispatches the `plan-executor` sub-agent → `NN_PLAN.md`
2. Executor **stops**. The user (or an external agent invoked by the user)
   writes `NN_REVIEW.md`.
3. Master may direct → `NN_MASTER.md` (optional).
4. The user signals the main session to dispatch round N+1's plan-executor
   (or to begin implementation).

Repeat until:
- no unresolved CRITICAL issues remain,
- HIGH issues are resolved or waived,
- and the plan is approved for implementation.

The main session does not self-chain PLAN → REVIEW → next PLAN. The
review step is out-of-session; each round therefore pauses after PLAN.

**Loop cap.** The PLAN ↔ REVIEW loop runs **at most 5 rounds** (`00` – `04`)
per feature. If the reviewer returns APPROVED earlier (no CRITICAL / HIGH
findings) or the cap is reached, proceed to implementation regardless of
any remaining MEDIUM / LOW findings. MEDIUM / LOW findings that survive
the cap are addressed inline during implementation, not used to extend
the loop indefinitely.

### Implementation

After the final approved PLAN:

1. Implementation (code changes **and** `NN_IMPL.md`) is authored by the
   **main session directly**, not by a sub-agent.
2. There is **no** post-implementation review artifact. `NN_IMPL_REVIEW.md`
   and `NN_IMPL_MASTER.md` have been retired. Any review-style findings
   on the implementation are applied inline as follow-up code edits in
   the same session; do not dispatch a reviewer sub-agent after IMPL.
3. If the user asks for an audit of IMPL, perform it in the main session
   (read diff + run gates + surface findings as a plain message), then
   apply fixes inline with Edit / Write.

### Response Rules

- Every prior CRITICAL / HIGH finding must appear in the next PLAN Response Matrix.
- Every MASTER directive must appear in the next PLAN Response Matrix.
- Rejections of REVIEW advice or trade-off suggestions must include explicit reasoning.
- `MUST` directives are binding.
- `SHOULD` directives require explicit response if rejected.
- If an implementation deviation changes architecture, API semantics, invariants, or constraints, a new PLAN iteration must be opened.