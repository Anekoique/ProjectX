# `port-to-ark` REVIEW `01`

> Status: Closed
> Feature: `port-to-ark`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Rejected
- Blocking: `1`
- Non-blocking: `5`

## Summary

Round 01 lands six of the seven round-00 remediations cleanly — Phase-2 split, `$BASELINE_HEAD`
capture, `.DS_Store` enumeration, the ~270 count, the seven-commit table, and the Bucket-A
em-dash rule are all correctly in place. The blocker is R-001's worked example for `csr`:
its Architecture diagram and three of its Constraint source-of-truth cites reference paths
(`xemu/xcore/src/arch/riscv/csr/`, `…/csr/trap.rs`, `…/csr/trap_handler.rs`) that **do not
exist on disk**. The real layout is `cpu/csr.rs` (file) plus `cpu/csr/` (subdir with
`mip.rs`, `mstatus.rs`, `ops.rs`, `privilege.rs`); `PendingTrap` / `TrapCause` /
`Exception` live under `cpu/trap/`, not under `csr/`. Because the worked example is explicitly
normative — "the other four Bucket-B slugs follow the same algorithm" — landing it as drafted
would inject a hallucinated map of the codebase into the canonical SPEC. Two additional
Phase-3 PROGRESS-link variants (lines 23 + 190 with `docs/`-prefixed link text) are also
missed; this is the same defect class R-003 was meant to close. One more focused iteration
clears both.

---

## Findings

### R-001 `Worked-example Architecture and Constraint cites hallucinate the codebase`

- **Severity:** CRITICAL
- **Section:** Implementation / Phase 2a / Worked example — `csr` (lines 277-386)
- **Problem:** The Architecture diagram lists `xemu/xcore/src/arch/riscv/csr/{mod.rs, mstatus.rs, privilege.rs, trap.rs, trap_handler.rs}`. Actual layout is `xemu/xcore/src/arch/riscv/cpu/csr.rs` (single file, no `mod.rs`) plus `cpu/csr/` containing only `mip.rs`, `mstatus.rs`, `ops.rs`, `privilege.rs`. C-1's cite `xemu/xcore/src/arch/riscv/csr/mstatus.rs` (correct path is `cpu/csr/mstatus.rs`), C-2's `csr/trap.rs`, and C-3's `csr/trap_handler.rs` reference files that do not exist — `PendingTrap`/`TrapCause` live at `cpu/trap/cause.rs:40` and the handler at `cpu/trap/handler.rs`.
- **Why it matters:** The plan declares the example "normative — the other four Bucket-B SPECs follow the same algorithm". Landing it ships a SPEC whose source-of-truth links are broken and whose Architecture block misrepresents where the subsystem lives. VERIFY's spec-compliance pass (PRD Why-bullet) loses its anchor. The R-001 round-00 finding promised a *faithful* worked example; this one fails that test.
- **Recommendation:** Re-read `xemu/xcore/src/arch/riscv/cpu/csr.rs` (head + `pub struct CsrFile` at line 265) and `cpu/trap/{cause.rs,handler.rs}` and rewrite the Architecture tree + C-1/C-2/C-3 cites to the actual `cpu/csr/` and `cpu/trap/` paths. Drop `trap.rs` / `trap_handler.rs` from the `csr/` block; they are a sibling subsystem. Spot-check the remaining four Bucket-B slug summaries (lines 390-393) the same way before this lands.

### R-002 `PROGRESS.md lines 23 and 190 retain "docs/" link text after Phase 3`

- **Severity:** HIGH
- **Section:** Implementation / Phase 3.3 substitution rules
- **Problem:** Lines 23 and 190 of `docs/PROGRESS.md` use the form `` see [`docs/archived/perf/perfBusFastPath/`](./archived/perf/perfBusFastPath/) ``. Phase 3.3 Rule 1 matches the URL but only addresses link text shaped `` `archived/<cat>/<feature>/` `` — these two have `docs/archived/` in the visible text. After Phase 3 the URL is rewritten but the inline-code text still reads `docs/archived/...`, which V-F-1's `rg "docs/(tasks|spec|archived|template)"` flags post-Phase-5.
- **Why it matters:** Identical-class defect to round-00 R-003; the plan's response covered the bare parent-dir refs (lines 330-331) but not the slug-qualified ones with `docs/`-prefixed text. The executor will hit the `rg` red-light at Phase 5 and have to amend mid-flight.
- **Recommendation:** Add a fifth substitution rule (or generalise Rule 1) for any markdown link whose *text* matches `` `docs/(spec|archived)/...` `` — strip the `docs/` prefix and rewrite to `features/...` / `legacy/...` (matching Rule 1's existing text convention). Mention lines 23 and 190 specifically so the executor doesn't have to re-derive the location.

### R-003 `SPEC_LEGACY_memOpt.md naming breaks the one-SPEC_LEGACY-per-dir convention`

- **Severity:** MEDIUM
- **Section:** Implementation / Phase 2a step 1 + V-E-5
- **Problem:** Bucket-B preservation is `<slug>/SPEC_LEGACY.md` everywhere except `memOpt`, which lands at `.ark/tasks/archive/legacy/mm/SPEC_LEGACY_memOpt.md` (suffixed) because it shares `mm`'s bucket. The plan does not say whether V-F-5's check (`diff -q docs/spec/<slug>/SPEC.md .ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md`) is altered for `memOpt` or whether grep tooling expecting `**/SPEC_LEGACY.md` will miss it.
- **Why it matters:** Future VERIFY/SPEC-drift tooling that globs `**/SPEC_LEGACY.md` won't see the `memOpt` source; the naming is the only Bucket-B slug that doesn't follow the rule. V-F-5's command also implicitly assumes the canonical name.
- **Recommendation:** Either (a) generalise: `.ark/tasks/archive/legacy/mm/SPEC_LEGACY.md` covers `mm` and a separate `.ark/tasks/archive/legacy/mm/SPEC_LEGACY_memOpt.md` is fine *only if* V-F-5 explicitly enumerates both paths, or (b) create a `legacy/memOpt/` dir containing just `SPEC_LEGACY.md` + a `README.md` pointer back to `legacy/mm/MEM_OPTIMIZATION_PLAN.md`. State the V-F-5 lookup for `memOpt` explicitly either way.

### R-004 `Phase 2a commit subject 84 chars — fallback should be primary recommendation`

- **Severity:** MEDIUM
- **Section:** Trade-offs / T-5 table (line 549) + body (line 556)
- **Problem:** Phase 2a's subject (`docs(specs): rewrite running-notes specs (csr klib mm memOpt err2trap) to ark template`) is 86 chars (confirmed by `wc -c`), exceeding `coding/git/SPEC.md` R4's "~72 chars" soft cap by 14. The plan acknowledges this and offers a 66-char fallback (`rewrite five running-notes specs to ark template`) only "if a hard 72-char cap is required at merge time", but R-9 says PR squash-merges go through this spec — so the merge commit is what lands.
- **Why it matters:** Round-00 R-005 specifically asked for subjects conforming to R1-R4. The slug-list argument is reasonable for body content, not the subject. Inverting the recommendation costs nothing.
- **Recommendation:** Promote the 66-char form to the table; move the slug list to the commit body (R5 territory — "the body explains why"). Keep the 86-char form noted as an alternative for users who prefer wide terminals, but make conformance the default.

### R-005 `inst CHANGELOG pointer line is self-contradicting`

- **Severity:** MEDIUM
- **Section:** Implementation / Phase 2b / Bucket C (line 412)
- **Problem:** The plan's `inst` CHANGELOG line is ``- `2026-05-11` `port-to-ark`: migrated from running-notes; no prior archive — running-notes preserved at `.ark/tasks/archive/legacy/inst/` is N/A (no such dir).`` This is a single bullet that simultaneously cites a path and disclaims its existence; future readers parse it as a broken link or a typo.
- **Why it matters:** A migrated SPEC's CHANGELOG is the audit trail. A self-negating bullet is worse than no bullet — VERIFY's drift pass can't tell whether the path was supposed to be created.
- **Recommendation:** Rewrite as a clean assertion: ``- `2026-05-11` `port-to-ark`: migrated from running-notes; no archive bucket (pre-workflow source had no iteration history).`` Drop the "preserved at … is N/A" clause entirely.

### R-006 `V-F-6 throwaway-worktree dry-run is over-engineered`

- **Severity:** LOW
- **Section:** Validation / Failure / Robustness / V-F-6 (line 578)
- **Problem:** V-F-6 prescribes a throwaway worktree clone to dry-run `git reset --hard $BASELINE_HEAD`. The Phase-1 commit alone makes `git log --oneline` show the migration is single-rooted; `git reflog show feat/port-to-ark | head` or `git cat-file -e $BASELINE_HEAD` is sufficient to confirm the target is reachable.
- **Why it matters:** Ceremony tax. An executor following V-F-6 literally spins up a second worktree to prove a single-rev-parse invariant. Either drop or replace with the lighter check.
- **Recommendation:** Replace V-F-6 with ``git cat-file -e $BASELINE_HEAD 2>&1 || echo "BASELINE_HEAD unreachable"`` and ``git log --oneline $BASELINE_HEAD..HEAD | wc -l`` (expected: number-of-phase-commits-so-far). Confirms rollback target without a second clone.

---

## Trade-off Advice

### TR-1 `Land the worked-example fix as a single-paragraph patch, not a full re-plan`

- **Related Plan Item:** R-001 above
- **Topic:** Iteration discipline vs Plan thoroughness
- **Reviewer Position:** Prefer minimal patch
- **Advice:** The 86-line worked-example block (lines 281-386) is the only large change required for round 02. R-002, R-003, R-005 are one-line patches; R-004 is a table swap; R-006 is one validation row. None of them touch the seven-section Spec block. Round 02 can be near-mechanical against this review's recommendations.
- **Rationale:** Three rounds is the cap (`max_iterations = 3`). Spending round 02 on a focused R-001 rewrite + the four small patches keeps round 03 as a final approval pass rather than a third substantive iteration.
- **Required Action:** Adopt. Round 02 should be a tight delta — no architectural rethink needed; the plan's structure is sound, only the worked example's factual claims need correction.
