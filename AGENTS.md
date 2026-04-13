# AGENTS.md

## Development Standards

- **Technical Research**: Always use web search to retrieve the latest official documentation.
- **Code Excellence**: Maintain a **clean, concise, and elegant** codebase. All implementations must strictly conform to the existing framework's architectural style.
- **Code Style:** Use a moderate amount of **functional** programming techniques.
- **Verification**: After making any coding-related modification, you must run `make fmt`, `make clippy`, `make run`, and `make test` to ensure correctness.

## Development Workflow

The project evolves through numbered iterations.  
All iteration artifacts reside in `docs/<feature>/`.

### Roles

| Role     | Document             | Responsibility |
| -------- | -------------------- | ------------------------------------------------------------ |
| Executor | `NN_PLAN.md`         | Proposes the implementation plan, including summary, spec alignment, architecture, invariants, API surface, implementation steps, validation strategy, and trade-off analysis. |
| Reviewer | `NN_REVIEW.md`       | Audits the plan for correctness, completeness, spec compliance, edge cases, maintainability, risk, and provides trade-off advice when needed. Findings are classified by severity: CRITICAL / HIGH / MEDIUM / LOW. |
| Master   | `NN_MASTER.md`       | Issues final directives when conflicts, ambiguity, or strategic trade-offs require a decisive override. Executor must unconditionally comply with all `MUST` directives in the next iteration. |
| Executor | `NN_IMPL.md`         | Records the actual implementation result, including completed scope, deviations from the approved PLAN, verification results, and acceptance mapping. This document supplements the code; it does not replace code review. |
| Reviewer | `NN_IMPL_REVIEW.md`  | Audits the implementation result with the **code as the primary artifact** and `NN_IMPL.md` as supporting context. The review focuses on correctness, plan compliance, validation adequacy, regressions, and unresolved gaps. Findings use IDs in the form `IR-XXX`. |
| Master   | `NN_IMPL_MASTER.md`  | Issues concise final directives for the implementation result. `MUST` directives are binding before merge / release. |

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

   | Artifact | Sub-agent |
   |----------|-----------|
   | `NN_PLAN.md` | **`plan-executor`** |
   | `NN_REVIEW.md` | **`plan-reviewer`** |
   | `NN_IMPL_REVIEW.md` | **`code-reviewer`** (or a language-specific reviewer) |

   MASTER documents (`NN_MASTER.md`, `NN_IMPL_MASTER.md`) are authored by the human user
   and are the only artifacts the main session may write by hand. Do not invoke
   `codex`, the `ask-codex` skill, or any other external reviewer for these artifacts;
   do not write PLAN or REVIEW files from the main session. Each sub-agent runs in
   isolation, reads the target artifact and repo state, and writes its output file
   itself.

### Iteration Lifecycle

Round N:
- Executor dispatches the `plan-executor` sub-agent → `NN_PLAN.md`
- Executor dispatches the `plan-reviewer` sub-agent → `NN_REVIEW.md`
- Master directs → `NN_MASTER.md` (optional)

Repeat until:
- no unresolved CRITICAL issues remain,
- HIGH issues are resolved or waived,
- and the plan is approved for implementation.

### Implementation Lifecycle

After an approved PLAN:
- Executor implements the code and records the result → `NN_IMPL.md`
- Executor dispatches the implementation-reviewer sub-agent (`code-reviewer` or a
  language-specific reviewer such as `rust-reviewer`) → `NN_IMPL_REVIEW.md`
- Master directs if needed → `NN_IMPL_MASTER.md` (optional)

Implementation review must use the **actual code changes as the primary review target**.  
`NN_IMPL.md` is only a supplement for:
- summarizing completed scope,
- recording deviations from the approved PLAN,
- reporting verification results,
- and mapping acceptance status.

Implementation is accepted only when:
- no unresolved implementation CRITICAL issues remain,
- the code is correct and reviewable,
- implementation matches the approved PLAN or explicitly records deviations,
- validation is adequate,
- and merge / release is approved.

### Response Rules

- Every prior CRITICAL / HIGH finding must appear in the next PLAN Response Matrix.
- Every MASTER directive must appear in the next PLAN Response Matrix.
- Rejections of REVIEW advice or trade-off suggestions must include explicit reasoning.
- `MUST` directives are binding.
- `SHOULD` directives require explicit response if rejected.
- Every meaningful deviation in `NN_IMPL.md` must be explicitly recorded.
- If an implementation deviation changes architecture, API semantics, invariants, or constraints, a new PLAN iteration must be opened.