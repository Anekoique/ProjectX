# `port-to-ark` REVIEW `02`

> Status: Closed
> Feature: `port-to-ark`
> Iteration: `02`
> Owner: Reviewer
> Target Plan: `02_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved
- Blocking: `0`
- Non-blocking: `1`

## Summary

Round 02 lands every round-01 finding cleanly. The R-001 CRITICAL rewrite of the
`csr` worked example was spot-checked against the live tree: `cpu/csr.rs` +
`cpu/csr/{mip,mstatus,ops,privilege}.rs` and `cpu/trap.rs` +
`cpu/trap/{cause,exception,handler,interrupt}.rs` match the Architecture diagram
exactly; `AccessRule` (csr.rs:23), `CsrDesc` with `wmask/storage/view_mask/view_shift/access`
(csr.rs:37), `CsrFile` (csr.rs:265), `PendingTrap` (cause.rs:40), `TrapCause` (cause.rs:8)
all resolve to real declarations; every `CsrFile` method in the API Surface exists at
csr.rs:289–344; `commit_trap`/`check_pending_interrupts`/`do_mret`/`do_sret` all live
at trap/handler.rs:17–149; and the three line cites are accurate (csr.rs:66 sits
inside `macro_rules! csr_table`, csr.rs:316 is `pub fn write_with_desc` whose body
literally does `desc.view_mask & desc.wmask`, trap/handler.rs:129 is `pub fn do_mret`).
R-002 Rule 5 is added with lines 23/190 explicitly named; R-003 promotes `memOpt`
to its own dir with canonical `SPEC_LEGACY.md`; R-004 puts the 62-char primary
subject in the T-5 table with slug list in body; R-005 cleans the `inst` CHANGELOG;
R-006 swaps the throwaway worktree for `git cat-file -e`. Spec block remains
self-contained per workflow rule; every PRD Outcome maps to a Goal or Constraint;
no new HIGH/CRITICAL defects introduced. The one observation (lines 330–331 of
PROGRESS.md are covered by Rule 5's generic clause but not enumerated in the
"known hits" callout) is MEDIUM polish — the executor can resolve it inline during
EXECUTE, per the loop-cap rule. **Approved for EXECUTE.**

---

## Findings

### R-001 `PROGRESS.md lines 330–331 are covered by Rule 5 generically but not in the "known hits" list`

- **Severity:** MEDIUM
- **Section:** Implementation / Phase 3.3 / Rule 5 (line 516)
- **Problem:** Rule 5's body is general ("Any markdown inline-code that reads `` `docs/(spec|archived)/<...>` `` inside a link's display-text MUST also be rewritten"), but its "Known hits" callout names only `docs/PROGRESS.md:23` and `:190`. A `grep` of the file also returns hits at lines 330 and 331 (`` [`docs/spec/<feature>/SPEC.md`](./spec/) `` and `` [`docs/archived/<feature>/`](./archived/) ``) which match the rule's pattern but aren't enumerated. Rule 4 covers the URL substitution for those two lines; Rule 5 covers the link-text substitution generically. Without explicit enumeration, the executor may treat the "Known hits" list as exhaustive and skip 330–331.
- **Why it matters:** If the executor reads the list as a checklist rather than as illustrative examples, V-F-1's `rg "docs/(tasks|spec|archived|template)"` at Phase 5 will flag two surviving hits and force a mid-flight amend. Same defect class as the original R-002 but smaller blast radius (two lines, not infinite).
- **Recommendation:** During EXECUTE, when applying Rule 5, treat the rule's body as authoritative and re-grep `docs/PROGRESS.md` for any `` `docs/(spec|archived)/...` `` inline-code-in-link-text occurrence — expect four hits (23, 190, 330, 331), not two. No plan revision needed; fix inline during implementation per the loop-cap rule.

---

## Trade-off Advice

_None — round 02 is a focused delta as TR-1 advised; the structure is sound, validation coverage is complete, and EXECUTE can proceed._
