# `port-to-ark` PRD

---

[**What**]

Migrate ProjectX's bespoke `docs/`-based PLAN/REVIEW/MASTER workflow onto the Ark CLI, consolidating all task, archive, and spec management under `.ark/`, and retiring the legacy workflow primitives.

[**Why**]

The repo currently runs two parallel doc-driven workflows: the original numbered-iteration scheme under `docs/tasks/`, `docs/spec/`, `docs/archived/`, and `docs/template/` (described in `AGENTS.md`), and the Ark CLI installed in `.ark/` (`workflow.md`, `specs/`, `tasks/archive/`). Two systems means two sources of truth — drift between `AGENTS.md` and `.ark/workflow.md` is already visible, and new contributors must learn both. Standardizing on Ark gives:

- A single CLI surface (`ark context`, `ark agent task ...`, `ark archive`, `ark cleanup`) that enforces phase gates instead of relying on human compliance with `AGENTS.md`.
- Native worktree isolation for deep-tier loops (already required by Ark).
- Automatic SPEC promotion + INDEX maintenance via `ark agent task commit`, replacing the manual extract-spec-from-final-PLAN step.
- Tooling-enforced VERIFY checklists (project-spec, feature-spec, plan-fidelity, drift) instead of free-form `NN_IMPL.md` audits.

The migration also normalizes 24 historical feature SPECs (many of which are PLAN documents copied verbatim with a banner) onto the canonical Ark `SPEC.md` template (Goals / Non-goals / Architecture / Data Structure / API Surface / Constraints / CHANGELOG), so VERIFY's spec-compliance pass has structured input rather than free prose.

[**Outcome**]

1. **`docs/tasks/`, `docs/spec/`, `docs/archived/`, `docs/template/` are deleted** from the tree. `docs/book/`, `docs/perf/`, `docs/PROGRESS.md`, `docs/README.md` are preserved (with cross-links rewritten to `.ark/` paths where they referenced the deleted trees).
2. **All 24 historical per-feature SPECs are rewritten** to the `.ark/templates/SPEC.md` template shape and live under `.ark/specs/features/<feature>/SPEC.md`. The seven sections (`Goals`/`Non-goals`/`Architecture`/`Data Structure`/`API Surface`/`Constraints`/`CHANGELOG`) are populated from each feature's prior content (final PLAN's `## Spec` block where one exists; the running notes for `inst`/`csr`/`klib`/`mm`/`err2trap`).
3. **`.ark/specs/features/INDEX.md`'s `ARK:FEATURES` table is populated by hand** with one row per migrated feature (`Feature | Scope | Promoted` — date taken from each feature's final archived round commit, or the migration date if unknown). The markers stay in place so `ark agent spec register` can append future rows.
4. **All iteration history (24 features) is moved verbatim** to `.ark/tasks/archive/legacy/<feature>/` preserving the full `NN_PLAN.md` / `NN_REVIEW.md` / `NN_MASTER.md` / `NN_IMPL.md` series. The single review-only artifact (`docs/archived/review/MANUAL_REVIEW.md`) is moved to `.ark/tasks/archive/legacy/MANUAL_REVIEW.md`.
5. **`AGENTS.md` is rewritten** to a slim file: keeps `## Development Standards`, replaces the entire Workflow / Roles / Iteration Rules / Implementation / Response Rules sections with a short pointer to `.ark/workflow.md`. The `<!-- ARK:START --> ... <!-- ARK:END -->` block remains. `CLAUDE.md` (symlink) is unchanged.
6. **`docs/PROGRESS.md`'s cross-links are rewritten** so every reference to `docs/spec/<feature>/SPEC.md` resolves to `.ark/specs/features/<feature>/SPEC.md`, and every reference to `docs/archived/<category>/<feature>/` resolves to `.ark/tasks/archive/legacy/<feature>/`. The phase tables, baselines, and roadmap content are otherwise unchanged.
7. **No source code under `xemu/`, `xam/`, `xlib/`, `xkernels/`, `resource/`, `scripts/` is touched.** Code-level docstrings that cite `docs/archived/...` paths are updated to point at the new `.ark/tasks/archive/legacy/...` location (grep across `*.rs` finds these).
8. **Build/test gates pass unchanged.** `make fmt`, `make clippy`, `make run`, `make test` produce identical results pre- and post-migration. No `.rs` semantic changes; only string-level path updates inside doc comments.
9. **Verification:** every legacy SPEC has a corresponding Ark-shaped SPEC at the migrated path; every legacy archive dir has a counterpart under `.ark/tasks/archive/legacy/`; no remaining file under the repo references the deleted `docs/tasks/`, `docs/spec/`, `docs/archived/`, `docs/template/` paths (`rg "docs/(tasks|spec|archived|template)"` returns zero results outside of the new SPECs' own `## CHANGELOG` provenance lines, if any).

[**Related Specs**]

None — this is the migration that establishes the `specs/features/` tree. The 24 SPECs created by this task are this task's *outputs*, not inputs. Future tasks will list the relevant SPECs here per the standard PRD shape.
