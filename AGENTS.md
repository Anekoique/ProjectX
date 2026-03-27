# AGENTS.md

## Development Standards

- **Technical Research**: Always use web search to retrieve the latest official documentation.
- **Code Excellence**: Maintain a **clean, concise, and elegant** codebase. All implementations must strictly conform to the existing framework's architectural style.
- **Verification**: After any modification, you must run `make fmt`, `make clippy`, `make run`, and `make test` to ensure correctness.

## Development Workflow

The project evolves through numbered iterations.  
All iteration artifacts reside in `docs/<feature>/`.

### Roles

| Role     | Document       | Responsibility |
| -------- | -------------- | ------------------------------------------------------------ |
| Executor | `NN_PLAN.md`   | Proposes the implementation plan, including summary, spec alignment, architecture, invariants, API surface, implementation steps, validation strategy, and trade-off analysis. |
| Reviewer | `NN_REVIEW.md` | Audits the plan for correctness, completeness, spec compliance, edge cases, maintainability, risk, and provides trade-off advice when needed. Findings are classified by severity: CRITICAL / HIGH / MEDIUM / LOW. |
| Master   | `NN_MASTER.md` | Issues final directives when conflicts, ambiguity, or strategic trade-offs require a decisive override. Executor must unconditionally comply with all `MUST` directives in the next iteration. |

### Iteration Rules

1. Each iteration starts with a PLAN.
2. REVIEW evaluates the PLAN but does not replace it. MASTER is optional, but if present, it is authoritative.
4. The next PLAN must explicitly resolve all blocking REVIEW findings and all MASTER directives.
5. Implementation may begin only when blocking issues are resolved or explicitly waived by MASTER.
6. All PLAN / REVIEW / MASTER documents must follow the templates in `docs/template`.
7. Never overwrite previous iteration documents. Always create the next numbered file.
8. If implementation requires a meaningful design change, open a new iteration instead of silently deviating from the approved PLAN.

### Iteration Lifecycle

Round iteration:
- Executor writes `00_PLAN.md` -> `01_PLAN.md`
- Reviewer audits → `00_REVIEW.md` -> `01_REVIEW.md`
- Master directs → `00_MASTER.md` (optional) -> `01_MASTER.md`

Repeat until:
- no unresolved CRITICAL issues remain,
- HIGH issues are resolved or waived,
- and the plan is approved for implementation.

### Response Rules

- Every prior CRITICAL / HIGH finding must appear in the next PLAN Response Matrix.
- Every MASTER directive must appear in the next PLAN Response Matrix.
- Rejections of REVIEW advice or trade-off suggestions must include explicit reasoning.
- `MUST` directives are binding.
- `SHOULD` directives require explicit response if rejected.