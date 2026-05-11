# `port-to-ark` REVIEW `00`

> Status: Closed
> Feature: `port-to-ark`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking: `0`
- Non-blocking: `7`

## Summary

The plan is substantively sound: the slug inventory (24), Phase-1 `git mv` mapping (14 feat + 2 fix + 4 refactor + 2 perf + 1 review file), the 6 known Rust doc-comment hits, the `cicd`/`memOpt`/`inst` irregular sources, and the 2 `ARK:FEATURES` markers all verify against the tree. The blocking risk is **lossy SPEC rewrites for Bucket B** (`csr` alone is 834 lines, `mm` 1206 lines) under a recipe given in two short paragraphs — without a worked example or explicit "preserve under CHANGELOG/provenance" rule the executor can quietly delete material. The **destructive Phase 5** lacks an explicit rollback procedure, and several **PROGRESS.md cross-link variants** (parent-dir refs `./spec/` and `./archived/`, MANUAL_REVIEW.md is correctly covered) are unhandled. Nothing here is fatal; one tightened iteration clears it.

---

## Findings

### R-001 `Bucket-B SPEC recipe under-specifies preservation`

- **Severity:** HIGH
- **Section:** Implementation/Phase 2 (Bucket B)
- **Problem:** The recipe collapses 834-line `csr` / 1206-line `mm` / 480-line `err2trap` / 211-line `memOpt` / 192-line `klib` design docs into ≤5 `Goals` + 5–10 `Constraints` + trimmed Architecture. There's no rule for what happens to "Design Decisions", "Phase 1/2/3", "Why" prose, or the bitflags/macro tables that don't fit any of the seven sections; the only mitigation is "legacy files survive under `.ark/tasks/archive/legacy/<slug>/`" — but `csr`'s archive (`PLAN.md` / `PLAN_REVIEW.md` / `IMPL_REVIEW.md`) is a different document than the running-notes SPEC the executor is being asked to collapse.
- **Why it matters:** The promoted SPEC is the canonical source going forward; lost rationale (the *why* behind `PendingTrap` vs `XError`, the `[Word; 4096]` choice, the locking-strategy discussion in `memOpt`) cannot be recovered without re-reading deleted prose. Bucket B is the qualitative risk the plan flags itself.
- **Recommendation:** (a) Add an explicit rule: "When prose does not fit the seven sections, copy the running-notes SPEC verbatim into `.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md` before rewriting, and reference it from the new SPEC's `[**CHANGELOG**]`." (b) Pre-draft one Bucket-B example (`csr` is the densest — work it end-to-end in the next iteration as a normative template the other four follow).

### R-002 `Phase 5 has no rollback procedure`

- **Severity:** HIGH
- **Section:** Runtime/Failure Flow + Implementation/Phase 5
- **Problem:** Phase 5 is `git rm -r docs/{tasks,spec,archived,template}` — irreversible inside the working tree. Failure Flow handles four small mishaps (rust gates, stray `rg` hit, missing INDEX row, `git mv` collision) but never names the procedure for "Phase 6 `make test` fails and the cause traces to a deleted doc the executor wants back."
- **Why it matters:** The plan correctly observes the worktree is the rollback surface, but never says *how* to use it. Executors mid-migration will reach for `git checkout HEAD -- docs/` and discover the rm was already committed if T-5 ("per-phase commits") was followed.
- **Recommendation:** Add a Failure Flow row: "If Phase 6 fails irrecoverably, `git reset --hard <pre-Phase-5-commit>` restores the deleted trees; the worktree branch absorbs the loss without affecting `main`." State the pre-Phase-5 commit hash will be captured (Runtime step 1 already records baseline gates — also record `git rev-parse HEAD` there).

### R-003 `PROGRESS.md substitution rules miss two cross-link variants`

- **Severity:** MEDIUM
- **Section:** Implementation/Phase 3.3
- **Problem:** The "Manual Review TODOs" preamble (PROGRESS.md:329–331) uses parent-directory references that the plan's rules don't match: `[`docs/spec/<feature>/SPEC.md`](./spec/)` and `[`docs/archived/<feature>/`](./archived/)`. Both targets become invalid after Phase 5; the plan's only "closing paragraph" rewrite covers the next paragraph (lines 343–346), not these two preamble links.
- **Why it matters:** Two markdown-rendered broken links land in PROGRESS.md, defeating PRD Outcome #6 ("phase tables, baselines, and roadmap content are otherwise unchanged" *plus* "every reference … resolves to the new path"). V-F-1's `rg "docs/(tasks|spec|archived|template)"` will catch the path-text, but Phase-3 fix-up is cleaner than a Phase-5 surprise.
- **Recommendation:** Add a fourth substitution rule for parent-dir refs: `./spec/` → `../.ark/specs/features/` and `./archived/` → `../.ark/tasks/archive/legacy/`. Or rewrite the preamble inline (it's two sentences) to point at the new paths.

### R-004 `Bucket-A "first sentence wins" Constraint collapse is lossy`

- **Severity:** MEDIUM
- **Section:** Implementation/Phase 2 (Bucket A)
- **Problem:** Bucket A maps `Invariants` → `Constraints` by collapsing "multi-sentence elaborations to their first sentence per Ark template guidance." `aclintSplit` and others have invariants like "I-N: <rule>. Test: <path>" where the test cite is on the *second* sentence; collapsing drops the source-of-truth that the Ark SPEC template explicitly asks for ("Cite the source of truth … when one exists").
- **Why it matters:** Half the migrated SPECs lose their evidence cites, weakening VERIFY's spec-compliance pass.
- **Recommendation:** Refine the rule: "Collapse to one sentence ≤120 chars *but* keep any inline `Test:`/`Evidence:`/file-path cite by folding it into the same sentence (e.g., `C-1: <rule> — verified by <path>.`)."

### R-005 `Commit-message scheme unstated for the six per-phase commits`

- **Severity:** MEDIUM
- **Section:** Trade-offs/T-5
- **Problem:** T-5 mandates per-phase commits ("six commits land on the worktree's branch") but doesn't propose subjects, and `coding/git/SPEC.md` R1–R4 require Conventional Commits with lowercase imperative subjects ≤72 chars. The destructive Phase-5 commit especially deserves an explicit subject (it touches ~270 files).
- **Why it matters:** Without a recommended scheme, the executor invents one ad-hoc and may produce subjects that fail `coding/git/SPEC.md` — adding a follow-up rewrite step.
- **Recommendation:** Add a per-phase subject table to T-5 (or Phase 6), e.g.: `chore(docs): relocate legacy archive under .ark/tasks/archive/legacy`, `docs(specs): rewrite 24 feature SPECs to Ark template`, `docs: populate features INDEX + slim AGENTS + rewrite PROGRESS links`, `docs(xemu): retarget archived-PLAN cites at .ark/tasks/archive/legacy`, `chore(docs): delete legacy docs/{tasks,spec,archived,template} trees`, `chore: re-baseline make gates post-Ark migration`.

### R-006 `"~140 files" claim is off by ~2×`

- **Severity:** LOW
- **Section:** Summary + Architecture annotation
- **Problem:** Summary says "move the ~140 archived iteration files"; Architecture annotation says `archived/ (-) deleted (~140 files)`. Actual count is 270 markdown files under `docs/archived/` (and 295 across all four deleted trees). Phase-1's "270+ rename entries" check is correct, so the Summary contradicts the body.
- **Why it matters:** A reviewer-or-executor cross-check on the `git status --short | wc -l` post-Phase-1 expectation will flag the discrepancy.
- **Recommendation:** Replace both "~140" mentions with "~270 archived files (one rename entry each)".

### R-007 `Two .DS_Store files, not one`

- **Severity:** LOW
- **Section:** Implementation/Phase 1 hygiene + V-E-7
- **Problem:** V-E-7 names `docs/archived/feat/csr/.DS_Store`; the tree also has `docs/.DS_Store`. Both are deleted by `find docs -name .DS_Store -delete`, so behaviour is correct, but the inventory undercounts.
- **Why it matters:** Cosmetic only — the `find` covers both.
- **Recommendation:** Update V-E-7 to "two known `.DS_Store` files at `docs/.DS_Store` and `docs/archived/feat/csr/.DS_Store`."

---

## Trade-off Advice

### TR-1 `Pre-stage Bucket-B SPECs before Phase 5`

- **Related Plan Item:** `T-1` (full rewrite vs lazy migration)
- **Topic:** Quality vs Velocity
- **Reviewer Position:** Need More Justification
- **Advice:** Pre-draft and land the five Bucket-B SPECs in a dedicated commit *before* Phase 5's destructive `git rm`. Land them in the worktree under their new paths so a reviewer can `diff` legacy → migrated side-by-side while the original still exists.
- **Rationale:** R-001's lossiness risk is hardest to catch after the source is deleted. Per-phase commits already exist; adding "Phase 2a — land Bucket-B SPECs" before Phase 5 is a small reorder that buys you a reviewable diff window.
- **Required Action:** Adopt or justify rejection. Worth one paragraph in the next PLAN's `## Log` either way.
