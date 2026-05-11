# `port-to-ark` PLAN `01`

> Status: Draft
> Feature: `port-to-ark`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`

---

## Summary

Retire the bespoke `docs/`-based PLAN/REVIEW/MASTER workflow and consolidate every task, archive, and feature-spec primitive under `.ark/`. The migration ships in seven independently inspectable, per-phase commits: (1) move the ~270 archived iteration files verbatim under `.ark/tasks/archive/legacy/<slug>/` via `git mv`, collapsing the `feat/`/`fix/`/`refactor/`/`perf/` subcategories; (2a) author the five Bucket-B SPECs (`csr`, `klib`, `mm`, `memOpt`, `err2trap`) at `.ark/specs/features/<slug>/SPEC.md` and stage each pre-workflow source verbatim under `.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md` so the reviewer can diff legacy → migrated while the original still exists; (2b) author the remaining 19 SPECs (17 Bucket-A iteration-history + 2 Bucket-C); (3) populate `.ark/specs/features/INDEX.md` between the `ARK:FEATURES` markers, slim `AGENTS.md` to standards + Ark pointer, and rewrite every `docs/PROGRESS.md` cross-link; (4) substitute the 6 `docs/archived/...` paths cited inside Rust doc-comments under `xemu/` with their new `.ark/tasks/archive/legacy/...` destinations; (5) `git rm -r docs/{tasks,spec,archived,template}` (legacy sources go now, after Bucket-B diff window closes); (6) re-run `make fmt && make clippy && make run && make test` to confirm no regressions. Each phase lands as one Conventional Commits subject that satisfies `coding/git/SPEC.md` R1–R4. The Phase-5 destruction is reversible via `git reset --hard $BASELINE_HEAD` captured at Runtime step 1 — the worktree-isolation (`feat/port-to-ark`) means any rollback never propagates to `main`.

## Log

[**Added**]

- Phase 2 (now Phase 2a) gains an explicit preservation rule: BEFORE rewriting each Bucket-B SPEC, copy the running-notes source verbatim into `.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md`. Migrated `CHANGELOG` opens with the pointer.
- Phase 2 splits into **2a — Bucket-B SPECs (5)** and **2b — Bucket-A + Bucket-C SPECs (19)** so the reviewable diff window predates Phase 5's destruction (TR-1 acceptance).
- Phase 2a contains a normative, end-to-end worked example for the `csr` SPEC pre-filled in the plan body — the other four Bucket-B SPECs follow the same algorithm.
- Runtime step 1 records `$BASELINE_HEAD = git rev-parse HEAD` as the rollback target.
- Failure Flow row covering `git reset --hard $BASELINE_HEAD` for irrecoverable Phase 6 failures.
- Per-phase commit subject table in T-5 covering seven Conventional Commits lines.
- Phase 3.3 gains a fourth substitution rule for the parent-dir refs `./spec/` and `./archived/` inside the "Manual Review TODOs" preamble at `docs/PROGRESS.md:328–331`.
- Phase 2 Bucket-A rule keeps inline `Test:` / `Evidence:` cites by folding them into the same constraint sentence with an em-dash.

[**Changed**]

- Summary: "~140 archived iteration files" → "~270 archived iteration files (one rename entry each)". Architecture annotation `archived/ (-) deleted (~140 files)` → `archived/ (-) deleted (~270 files)`.
- Trade-offs T-5: "six per-phase commits" → "seven per-phase commits" (Phase 1 + 2a + 2b + 3 + 4 + 5 + 6).
- V-E-7: lists both known `.DS_Store` files (`docs/.DS_Store` and `docs/archived/feat/csr/.DS_Store`).
- Phase 5 gains an inline rollback note pointing at the pre-Phase-5 commit hash.

[**Removed**]

- None.

[**Unresolved**]

- None. All seven REVIEW findings (R-001..R-007) and TR-1 are addressed in this iteration. MEDIUM/LOW responses are documented in the Response Matrix below; no finding is deferred to a later round.

[**Response Matrix**]

| ID | Severity | Disposition | Resolution |
|----|----------|-------------|------------|
| R-001 | HIGH | Accepted | (a) Phase 2 Bucket-B preservation rule: legacy source copied verbatim to `.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md`; migrated SPEC's `[**CHANGELOG**]` opens with the pointer line. (b) Plan body inlines a normative worked example for `csr` (Phase 2a §Worked example — `csr`); the other four Bucket-B SPECs follow the same algorithm. |
| R-002 | HIGH | Accepted | Runtime step 1 captures `$BASELINE_HEAD = git rev-parse HEAD`. Failure Flow row added: "If Phase 6 fails irrecoverably, `git reset --hard $BASELINE_HEAD` restores the deleted trees; worktree-isolation (`feat/port-to-ark`) confines rollback to this branch and never affects `main`." Phase 5 mirrors the note inline. |
| R-003 | MEDIUM | Accepted | Phase 3.3 substitution rule 4 added: `./spec/<feature>/SPEC.md` → `../.ark/specs/features/<feature>/SPEC.md`; `./archived/<category>/<feature>/` → `../.ark/tasks/archive/legacy/<feature>/`. Targets the preamble at `docs/PROGRESS.md:328–331`. |
| R-004 | MEDIUM | Accepted | Phase 2 Bucket-A rule rewritten: "Collapse to one declarative sentence ≤120 chars, but fold any inline `Test:` / `Evidence:` / file-path cite into the same sentence with an em-dash, e.g. `C-1: <rule> — verified by tests/csr.rs::warl_write.`" |
| R-005 | MEDIUM | Accepted | Trade-offs T-5 grows a per-phase commit subject table. Subjects are lowercase imperative, ≤72 chars, conform to `coding/git/SPEC.md` R1–R4. Seven subjects (one per phase incl. Phase 2 split). |
| R-006 | LOW | Accepted | "~140 files" → "~270 archived files" in Summary and Architecture annotation. Authoritative count is the reviewer's; verified at Phase 1 entry with `find /Users/anekoique/ProjectX/docs/archived -type f -name "*.md" \| wc -l`. |
| R-007 | LOW | Accepted | V-E-7 updated to enumerate both known `.DS_Store` files (`docs/.DS_Store` and `docs/archived/feat/csr/.DS_Store`); the existing `find docs -name .DS_Store -delete` already covers both — wording only. |
| TR-1 | Trade-off | Accepted | Phase 2 split into 2a (Bucket-B, lands SPECs while legacy `docs/spec/<slug>/SPEC.md` still exists) and 2b (Bucket-A + Bucket-C). Legacy source survives until Phase 5; reviewer can `diff docs/spec/csr/SPEC.md .ark/specs/features/csr/SPEC.md` side-by-side. Commit count increases from six to seven (T-5 updated accordingly). |

---

## Spec

[**Goals**]

- G-1: Remove `docs/{tasks,spec,archived,template}` from the repository.
- G-2: Land 24 migrated SPECs at `.ark/specs/features/<slug>/SPEC.md` in Ark template shape.
- G-3: Populate `.ark/specs/features/INDEX.md` `ARK:FEATURES` table with one row per migrated feature.
- G-4: Relocate legacy iteration history verbatim to `.ark/tasks/archive/legacy/<slug>/`.
- G-5: Slim `AGENTS.md` to `## Development Standards` plus a pointer at `.ark/workflow.md`; rewrite `docs/PROGRESS.md` cross-links.

[**Non-goals**]

- NG-1: No Rust source-code semantic edits — only doc-comment path-string substitutions inside `///` / `//!` blocks.
- NG-2: No rewrite of `docs/book/` mdBook prose beyond cross-link fixes (none currently target the deleted trees).
- NG-3: No retroactive iteration on legacy SPECs — they migrate as-is; future iterations re-stamp `Promoted` per feature.

[**Architecture**]

End-state directory layout (annotated). `(-)` marks deletions, `(+)` marks new content, `(*)` marks an in-place rewrite.

```
ProjectX/
├── AGENTS.md                                    (*) slimmed to Standards + Ark pointer
├── CLAUDE.md                                    symlink → AGENTS.md (unchanged)
├── docs/
│   ├── book/                                    unchanged
│   ├── perf/                                    unchanged (baselines)
│   ├── PROGRESS.md                              (*) cross-links rewritten
│   ├── README.md                                unchanged
│   ├── tasks/                                   (-) deleted
│   ├── spec/                                    (-) deleted (24 dirs)
│   ├── archived/                                (-) deleted (~270 files)
│   └── template/                                (-) deleted
├── .ark/
│   ├── workflow.md                              unchanged
│   ├── templates/                               unchanged
│   ├── specs/
│   │   ├── INDEX.md                             unchanged
│   │   ├── project/                             unchanged
│   │   └── features/
│   │       ├── INDEX.md                         (*) ARK:FEATURES table populated
│   │       ├── aclintSplit/SPEC.md              (+)
│   │       ├── amTests/SPEC.md                  (+)
│   │       ├── archLayout/SPEC.md               (+)
│   │       ├── archModule/SPEC.md               (+)
│   │       ├── benchmark/SPEC.md                (+)
│   │       ├── boot/SPEC.md                     (+)
│   │       ├── cicd/SPEC.md                     (+)
│   │       ├── csr/SPEC.md                      (+)
│   │       ├── debian/SPEC.md                   (+)
│   │       ├── devices/SPEC.md                  (+)
│   │       ├── difftest/SPEC.md                 (+)
│   │       ├── directIrq/SPEC.md                (+)
│   │       ├── err2trap/SPEC.md                 (+)
│   │       ├── float/SPEC.md                    (+)
│   │       ├── inst/SPEC.md                     (+)
│   │       ├── keyboard/SPEC.md                 (+)
│   │       ├── klib/SPEC.md                     (+)
│   │       ├── memOpt/SPEC.md                   (+)
│   │       ├── mm/SPEC.md                       (+)
│   │       ├── multiHart/SPEC.md                (+)
│   │       ├── perfBusFastPath/SPEC.md          (+)
│   │       ├── perfHotPath/SPEC.md              (+)
│   │       ├── plicGateway/SPEC.md              (+)
│   │       └── trace/SPEC.md                    (+)
│   └── tasks/
│       ├── port-to-ark/                         this task's artifacts
│       └── archive/
│           └── legacy/
│               ├── MANUAL_REVIEW.md             (+) file, not dir
│               ├── aclintSplit/                 (+) from refactor/
│               ├── amTests/                     (+) from feat/
│               ├── archLayout/                  (+) from refactor/
│               ├── archModule/                  (+) from refactor/
│               ├── benchmark/                   (+) from feat/
│               ├── boot/                        (+) from feat/
│               ├── cicd/                        (+) from feat/
│               ├── csr/                         (+) from feat/ (pre-workflow; +SPEC_LEGACY.md)
│               ├── debian/                      (+) from feat/
│               ├── devices/                     (+) from feat/
│               ├── difftest/                    (+) from feat/
│               ├── directIrq/                   (+) from fix/
│               ├── err2trap/                    (+) from refactor/ (pre-workflow; +SPEC_LEGACY.md)
│               ├── float/                       (+) from feat/
│               ├── keyboard/                    (+) from feat/
│               ├── klib/                        (+) from feat/ (pre-workflow; +SPEC_LEGACY.md)
│               ├── mm/                          (+) from feat/ (pre-workflow; carries memOpt source; +SPEC_LEGACY.md for mm and memOpt)
│               ├── multiHart/                   (+) from feat/
│               ├── perfBusFastPath/             (+) from perf/
│               ├── perfHotPath/                 (+) from perf/
│               ├── plicGateway/                 (+) from fix/
│               └── trace/                       (+) from feat/
└── xemu/                                        (*) doc-comment cite strings updated
    └── xcore/src/...                            no semantic changes
```

The 24 migrated slugs (matching `docs/spec/<slug>/`): `aclintSplit`, `amTests`, `archLayout`, `archModule`, `benchmark`, `boot`, `cicd`, `csr`, `debian`, `devices`, `difftest`, `directIrq`, `err2trap`, `float`, `inst`, `keyboard`, `klib`, `memOpt`, `mm`, `multiHart`, `perfBusFastPath`, `perfHotPath`, `plicGateway`, `trace`. Note: `inst` has no archive counterpart (running-notes only); `memOpt` shares its archive with `mm`.

[**Data Structure**]

This migration ships no Rust types. The conceptual schemas being mapped are:

- Legacy SPEC shape (`Source:` banner + free prose OR `Goals / Architecture / Invariants / Data Structure / API Surface`) → Ark template shape (`Goals / Non-goals / Architecture / Data Structure / API Surface / Constraints / CHANGELOG`).
- Legacy archive layout (`docs/archived/<feat|fix|refactor|perf|review>/<slug>/`) → Ark legacy bucket (`.ark/tasks/archive/legacy/<slug>/`, flat — no category subdir).
- Bucket-B preservation pair: each pre-workflow SPEC produces (a) a migrated `.ark/specs/features/<slug>/SPEC.md` in template shape AND (b) a verbatim copy of the original at `.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md`.

[**API Surface**]

No public Rust API changes. The operational interfaces that change:

- `ark agent task new --slug <s> --tier deep --worktree` replaces the manual `mkdir docs/tasks/<feature>/` step.
- `ark agent task commit` replaces the manual extract-`## Spec`-to-`docs/spec/<feature>/SPEC.md` step.
- `ark archive` replaces the manual `git mv docs/tasks/<feature> docs/archived/<category>/` step.
- `docs/tasks/README.md` (deleted) is no longer a contributor entry point; `AGENTS.md` points at `.ark/workflow.md`.
- Slash commands `/ark:quick`, `/ark:design`, `/ark:design --deep` replace `plan-executor` sub-agent dispatch.

[**Constraints**]

- C-1: Migrated SPECs preserve the seven Ark template section headings in order (`Goals`, `Non-goals`, `Architecture`, `Data Structure`, `API Surface`, `Constraints`, `CHANGELOG`).
- C-2: Every row in `.ark/specs/features/INDEX.md` between the `ARK:FEATURES` markers has a matching `<feature>/SPEC.md` on disk.
- C-3: No file under the repo references `docs/(tasks|spec|archived|template)` after Phase 5; verified by `rg "docs/(tasks|spec|archived|template)" /Users/anekoique/ProjectX`.
- C-4: `make fmt && make clippy && make run && make test` pass with identical outcomes pre- and post-migration.
- C-5: Every legacy iteration file (`NN_PLAN.md`, `NN_REVIEW.md`, `NN_MASTER.md`, `NN_IMPL.md`, plus pre-workflow `PLAN.md`/`PLAN_REVIEW.md`/`*_FIX.md`/`*_OPTIMIZATION_PLAN.md`/`IMPL.md` variants) present pre-migration has a byte-identical counterpart under `.ark/tasks/archive/legacy/<slug>/`.
- C-6: `AGENTS.md` retains `## Development Standards` verbatim and contains the `<!-- ARK:START --> ... <!-- ARK:END -->` block.
- C-7: All archive moves use `git mv` so `git log --follow` continues to surface the original commits.
- C-8: `docs/PROGRESS.md` table rows, phase commentary, and baseline links are unchanged outside cross-link substitutions.

---

## Runtime

[**Main Flow**]

1. Confirm baseline. Record `$BASELINE_HEAD = $(git rev-parse HEAD)` (this is the rollback target referenced in Failure Flow). Run `make fmt && make clippy && make run && make test` on the current `feat/port-to-ark` worktree; record exit codes. This is the C-4 reference.
2. Phase 1 — archive relocation. `git mv` each `docs/archived/<cat>/<slug>/` → `.ark/tasks/archive/legacy/<slug>/`; the executor MUST inspect a `git status --short` summary before continuing. Commit as `chore(docs): relocate legacy archive under .ark/tasks/archive/legacy`.
3. Phase 2a — Bucket-B SPECs. For each of `csr`, `klib`, `mm`, `memOpt`, `err2trap`: (i) copy the legacy running-notes source verbatim into `.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md` (`memOpt`'s preserved copy lands at `.ark/tasks/archive/legacy/mm/SPEC_LEGACY_memOpt.md` since memOpt shares mm's bucket); (ii) author `.ark/specs/features/<slug>/SPEC.md` in the Ark template shape. Commit as `docs(specs): rewrite running-notes specs (csr klib mm memOpt err2trap) to ark template`.
4. Phase 2b — Bucket-A + Bucket-C SPECs. Author the remaining 19 SPECs (17 Bucket-A + 2 Bucket-C). Commit as `docs(specs): rewrite iteration-history specs to ark template shape`.
5. Phase 3 — populate `.ark/specs/features/INDEX.md` between the `ARK:FEATURES:START`/`ARK:FEATURES:END` markers (one row per slug); rewrite `AGENTS.md` to slim shape; rewrite `docs/PROGRESS.md` cross-link substrings. Commit as `docs: populate features index, slim agents.md, retarget progress links`.
6. Phase 4 — `rg "docs/archived" xemu/ xam/` lists doc-comment cites; substitute each with the migrated path. Phase 4 expects zero `.rs` AST changes; only string bodies inside `///`/`//!` differ. Commit as `docs(xemu): retarget archived-plan cites at .ark/tasks/archive/legacy`.
7. Phase 5 — `git rm -r docs/tasks docs/spec docs/archived docs/template`; re-run `rg "docs/(tasks|spec|archived|template)" /Users/anekoique/ProjectX` and expect zero hits outside `.ark/tasks/port-to-ark/`. Commit as `chore(docs): delete legacy docs/{tasks,spec,archived,template} trees`.
8. Phase 6 — re-run `make fmt && make clippy && make run && make test`; compare to step 1. Commit any re-baseline byproducts as `chore: re-baseline make gates post-ark migration` (this commit may be empty if no byproducts; in that case skip).

[**Failure Flow**]

1. Rust gates fail after Phase 4. Doc-comment edits should never break compilation; if they do, an `#[doc = "..."]` macro arg, a `cfg`-conditional include, or a `compile_error!` cite was missed. Roll back the offending file with `git checkout HEAD -- <path>` and re-edit by hand.
2. Post-Phase-5 `rg` returns a hit. Re-open the matching file (most likely a stray cross-reference inside an SPEC just authored) and replace the cite before declaring Phase 5 complete.
3. INDEX row missing a SPEC. Either the SPEC was not authored (return to Phase 2a/2b) or the INDEX row was hand-typed wrong (correct the slug spelling).
4. `git mv` fails because target directory already exists. Inspect `.ark/tasks/archive/legacy/<slug>/` — if a previous phase partially completed, complete the move with `git mv --force` after confirming no data loss.
5. **Irrecoverable failure (any phase).** `git reset --hard $BASELINE_HEAD` restores the worktree to its Runtime-step-1 state. The worktree branch `feat/port-to-ark` absorbs the loss; `main` is unaffected because worktree isolation (`.ark/worktrees/feat/port-to-ark`) is the deep-tier required topology. `$BASELINE_HEAD` was captured in Runtime step 1 and additionally written to `.ark/tasks/port-to-ark/.baseline-head` for durability across shell sessions.
6. **Bucket-B regret window.** Phase 2a lands the migrated Bucket-B SPECs *before* Phase 5 deletes the legacy sources. If review of the migrated SPEC reveals lost content, the legacy `docs/spec/<slug>/SPEC.md` is still on disk through the end of Phase 4; the executor can re-open, copy the missing rationale into the migrated SPEC, and amend the Phase-2a commit (or land a fix-up commit) without touching the rollback target.

[**State Transitions**]

- `docs/` (full tree) → `docs/` pruned to `book/` + `perf/` + `PROGRESS.md` + `README.md` (Phase 5 boundary).
- `.ark/specs/features/` empty → 5 SPECs + populated INDEX (Phase 2a) → 24 SPECs + populated INDEX (Phase 2b + Phase 3).
- `.ark/tasks/archive/` empty → contains `legacy/<slug>/` subtrees (Phase 1) → `legacy/<slug>/SPEC_LEGACY.md` added for Bucket-B slugs (Phase 2a).
- `AGENTS.md` rich workflow doc → slim standards + pointer (Phase 3).
- `xemu/**/*.rs` doc-comments citing `docs/archived/...` → citing `.ark/tasks/archive/legacy/...` or `.ark/specs/features/...` (Phase 4).

---

## Implementation

[**Phase 1 — Archive relocation**]

Move every `docs/archived/<cat>/<slug>/` directory under `.ark/tasks/archive/legacy/<slug>/`, collapsing the category subdir. Use `git mv` (preserves history). The full slug-to-source mapping:

```
git mv docs/archived/feat/amTests          .ark/tasks/archive/legacy/amTests
git mv docs/archived/feat/benchmark        .ark/tasks/archive/legacy/benchmark
git mv docs/archived/feat/boot             .ark/tasks/archive/legacy/boot
git mv docs/archived/feat/cicd             .ark/tasks/archive/legacy/cicd
git mv docs/archived/feat/csr              .ark/tasks/archive/legacy/csr
git mv docs/archived/feat/debian           .ark/tasks/archive/legacy/debian
git mv docs/archived/feat/devices          .ark/tasks/archive/legacy/devices
git mv docs/archived/feat/difftest         .ark/tasks/archive/legacy/difftest
git mv docs/archived/feat/float            .ark/tasks/archive/legacy/float
git mv docs/archived/feat/keyboard         .ark/tasks/archive/legacy/keyboard
git mv docs/archived/feat/klib             .ark/tasks/archive/legacy/klib
git mv docs/archived/feat/mm               .ark/tasks/archive/legacy/mm
git mv docs/archived/feat/multiHart        .ark/tasks/archive/legacy/multiHart
git mv docs/archived/feat/trace            .ark/tasks/archive/legacy/trace
git mv docs/archived/fix/directIrq         .ark/tasks/archive/legacy/directIrq
git mv docs/archived/fix/plicGateway       .ark/tasks/archive/legacy/plicGateway
git mv docs/archived/refactor/aclintSplit  .ark/tasks/archive/legacy/aclintSplit
git mv docs/archived/refactor/archLayout   .ark/tasks/archive/legacy/archLayout
git mv docs/archived/refactor/archModule   .ark/tasks/archive/legacy/archModule
git mv docs/archived/refactor/err2trap     .ark/tasks/archive/legacy/err2trap
git mv docs/archived/perf/perfBusFastPath  .ark/tasks/archive/legacy/perfBusFastPath
git mv docs/archived/perf/perfHotPath      .ark/tasks/archive/legacy/perfHotPath
git mv docs/archived/review/MANUAL_REVIEW.md .ark/tasks/archive/legacy/MANUAL_REVIEW.md
```

`memOpt` has no separate archive dir — its source (`MEM_OPTIMIZATION_PLAN.md`) lives inside `legacy/mm/` after the move. The `memOpt` SPEC's `CHANGELOG` source pointer references that path explicitly.

Pre-Phase-1 hygiene: `find docs -name .DS_Store -delete` removes macOS cruft so it doesn't get migrated.

Post-Phase-1 inspection: `git status --short | wc -l` should show roughly 270+ rename entries. `ls .ark/tasks/archive/legacy/` should list 22 subdirs plus `MANUAL_REVIEW.md`. Commit: `chore(docs): relocate legacy archive under .ark/tasks/archive/legacy`.

[**Phase 2a — Bucket-B SPECs (pre-workflow running-notes features, 5 SPECs)**]

Bucket B covers `csr`, `klib`, `mm`, `memOpt`, `err2trap`. Each has a long-form pre-workflow design doc under a `Source:` banner. The migration is qualitative — the running notes are denser than any seven-section template can hold, so this phase carries the highest quality risk. **TR-1 accepted**: Phase 2a runs before Phase 5 specifically so the reviewer can `diff docs/spec/<slug>/SPEC.md .ark/specs/features/<slug>/SPEC.md` while the original still exists.

**Per-slug procedure** (apply to all five):

1. **Preserve.** `cp docs/spec/<slug>/SPEC.md .ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md`. The file is byte-identical to the original. For `memOpt` (no own archive dir), preserve at `.ark/tasks/archive/legacy/mm/SPEC_LEGACY_memOpt.md`.
2. **Author migrated SPEC** at `.ark/specs/features/<slug>/SPEC.md` in the Ark template shape (`Goals`/`Non-goals`/`Architecture`/`Data Structure`/`API Surface`/`Constraints`/`CHANGELOG`). Algorithm:
   - `Goals` (≤5, ≤80 chars, verb-led): synthesize from the doc's "Current Status" / "What works" / phase-motivation sections. Capability-oriented — the user-visible *what*, not the *how*.
   - `Non-goals` (≤3): lift verbatim if a `## Non-Goals` block exists; otherwise omit the bullet rather than invent.
   - `Architecture`: place the high-level diagram / directory tree under this heading. Trim narrative; the Ark template explicitly says "Diagrams are welcome; prose narration is not."
   - `Data Structure`: lift public Rust types only — `struct` / `enum` / `trait` declarations from the running notes. Drop method bodies. Field-level comments only when meaning is non-obvious.
   - `API Surface`: lift public function signatures. No bodies. If signature + name capture intent, drop the comment.
   - `Constraints` (5–10 ≤120 chars each): synthesize declarative invariants from "Design Decisions", "Design Principles", "Key layering principle", and similar bullets. **Fold any inline `Test:` / `Evidence:` / file-path cite into the same sentence with an em-dash** (e.g. `C-1: <rule> — verified by tests/csr.rs::warl_write.`).
   - `CHANGELOG`: open with the preservation pointer line: `` - `2026-05-11` `port-to-ark`: migrated from running-notes SPEC; full original preserved at `.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md`. ``

3. **No `Source:` banner** in the migrated file — the pointer lives in `CHANGELOG` instead. The blockquote shape used by the pre-workflow SPEC (`> Migrated 2026-05-11 from ...`) is *not* added; the seven sections start at the top.

**Worked example — `csr`**

The migrated `.ark/specs/features/csr/SPEC.md`. The executor authors this verbatim; the algorithm above produced it from `docs/spec/csr/SPEC.md` (834 lines). The other four Bucket-B slugs follow the same algorithm but their bodies are not pre-filled here.

```markdown
[**Goals**]

- G-1: Provide WARL-masked reads and writes for M/S privilege CSRs (mstatus, sstatus, mip, sie, mie, satp, ...).
- G-2: Route every architectural trap — ecall / ebreak / illegal-inst / illegal-CSR / page-fault — through stvec / mtvec via medeleg / mideleg.
- G-3: Shadow S-mode CSRs (sstatus, sip, sie) onto M-mode storage via a single descriptor table — no duplicate state.
- G-4: Generate the difftest CSR whitelist from `csr_table!` `@ difftest` annotations.
- G-5: Hold privilege checking + side effects in `RVCore`; keep `CsrFile` to storage, masks, shadows only.

[**Non-goals**]

- NG-1: No HPM / hpmcounter / hpmevent CSRs beyond write-through stubs.
- NG-2: No F / D / V-extension CSRs (fcsr / vstart / vl) — they belong to the float / vector subsystems.
- NG-3: No vectored `mtvec` mode (BASE+4×cause) — `mtvec` wmask forces direct mode.

[**Architecture**]

```
xemu/xcore/src/arch/riscv/csr/
├── mod.rs            CsrFile + csr_table! macro + find_desc dispatch
├── mstatus.rs        MStatus bitflags, mpp/spp roundtrip, SSTATUS view mask
├── privilege.rs      PrivilegeMode enum (M / S / U), from_bits + ordering
├── trap.rs           PendingTrap, TrapCause, Exception, Interrupt enums
└── trap_handler.rs   trap_entry, do_mret, do_sret on RVCore
```

Layering: `CsrFile` knows storage + masks + shadows; `RVCore` knows privilege + dynamic rules (TSR / TVM / counteren) + side effects + trap generation. PC commit: trap writes `npc`, the execute loop commits `pc = npc`.

[**Data Structure**]

```rust
pub struct CsrFile {
    regs: [Word; 4096],
}

pub struct PendingTrap {
    pub cause: TrapCause,
    pub tval: Word,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrapCause {
    Exception(Exception),
    Interrupt(Interrupt),
}

bitflags! {
    pub struct MStatus: Word {
        const SIE = 1 << 1; const MIE = 1 << 3;
        const SPIE = 1 << 5; const MPIE = 1 << 7;
        const SPP = 1 << 8;  const MPP = 0b11 << 11;
        // ... full bitflags lifted from mstatus.rs
    }
}

#[derive(Clone, Copy)]
struct CsrDesc {
    wmask:     Word,
    storage:   u16,
    view_mask: Word,
    access:    AccessRule,
}

#[derive(Clone, Copy)]
enum AccessRule { Standard, BlockedByMstatus(MStatus), CounterGated }
```

[**API Surface**]

```rust
impl RVCore {
    pub fn csr_read(&mut self, addr: u16) -> Option<Word>;
    pub fn csr_write(&mut self, addr: u16, val: Word) -> bool;
    pub fn raise_trap(&mut self, cause: TrapCause, tval: Word);
    pub fn commit_pending_trap(&mut self);
    pub fn do_mret(&mut self);
    pub fn do_sret(&mut self);
}

impl CsrFile {
    pub fn read_with_desc(&self, desc: &CsrDesc) -> Word;
    pub fn write_with_desc(&mut self, desc: &CsrDesc, val: Word);
    pub fn read_masked(&self, addr: u16) -> Option<Word>;
    pub fn write_masked(&mut self, addr: u16, val: Word) -> bool;
    pub fn get(&self, addr: CsrAddr) -> Word;
    pub fn set(&mut self, addr: CsrAddr, val: Word);
}

macro_rules! csr_table { /* generates CsrAddr enum + find_desc match */ }
```

[**Constraints**]

- C-1: `mstatus` is the master; `sstatus` is a subset view over `mstatus` storage — `sstatus` never has its own slot — `xemu/xcore/src/arch/riscv/csr/mstatus.rs`.
- C-2: Architectural traps set `pending_trap` and return `Ok(())`; `Err(XError)` is reserved for host I/O failures and emulator invariant violations — `xemu/xcore/src/arch/riscv/csr/trap.rs`.
- C-3: Trap entry writes `npc` not `pc`; the execute loop is the single commit point `self.pc = self.npc` — `xemu/xcore/src/arch/riscv/csr/trap_handler.rs`.
- C-4: `CsrFile` is dumb storage — privilege, dynamic rules, side effects, and trap generation all live in `RVCore`.
- C-5: `csr_table!` generates `CsrAddr` and `find_desc` from one source; the two cannot drift apart.
- C-6: WARL masking applies on every write — `write_with_desc` ANDs `view_mask & wmask` over the new bits.
- C-7: `mret` clears `MPRV` when returning to privilege < M; `sret` always clears `MPRV` (returns to S or U).
- C-8: Illegal CSR access — unknown addr, insufficient privilege, RO write, dynamic rule violation — raises `IllegalInstruction` trap, never returns `Err`.

[**CHANGELOG**]

- `2026-05-11` `port-to-ark`: migrated from running-notes SPEC; full original preserved at `.ark/tasks/archive/legacy/csr/SPEC_LEGACY.md`.
```

The other four Bucket-B slugs are authored by the same algorithm:

- **`klib`** (192 lines): Goals around "freestanding C library for xam-built guests", "newlib stub coverage", "no_std-friendly link surface". Architecture = directory tree of `xam/klib/`. Constraints lift from the doc's "Layering" / "Why" bullets.
- **`mm`** (1206 lines): Goals around "Bus + MMU + TLB + PMP + MMIO routing", "dual RV32/RV64", "Sv32/39/48/57 page walking". Architecture = the four-layer responsibility split + the access-path diagram. Data Structure lifts `Bus`, `MmioRegion`, `Mmu`, `Tlb`, `Pmp`, `Pte`, `SvMode`. Constraints lift from the four-layer responsibility table + design-decision bullets. ~10 constraints, each ≤120 chars.
- **`memOpt`** (211 lines): Goals around "hot-path lock reduction follow-up to mm". `SPEC_LEGACY` preserved at `.ark/tasks/archive/legacy/mm/SPEC_LEGACY_memOpt.md`. CHANGELOG points there.
- **`err2trap`** (480 lines): Goals around "split `XError::Trap(...)` into `PendingTrap` + reserved `XError` for host I/O". Constraints lift the `Err` reservation rule, the `pending_trap` write contract, the `npc` commit ordering — same rules `csr` enforces, but framed from the refactor side.

Commit Phase 2a: `docs(specs): rewrite running-notes specs (csr klib mm memOpt err2trap) to ark template`.

[**Phase 2b — Bucket-A + Bucket-C SPECs (19 SPECs)**]

**Bucket A — iteration-history features (17 SPECs).** Slugs: `aclintSplit`, `amTests`, `archLayout`, `archModule`, `benchmark`, `boot`, `debian`, `devices`, `difftest`, `directIrq`, `float`, `keyboard`, `multiHart`, `perfBusFastPath`, `perfHotPath`, `plicGateway`, `trace`. The current `docs/spec/<slug>/SPEC.md` has structured `Goals`/`Non-Goals`/`Architecture`/`Invariants`/`Data Structure`/`API Surface` blocks extracted from a final archived PLAN. Migration mapping:

- `Goals` → `Goals` (verbatim).
- `Non-Goals` → `Non-goals` (case rename only).
- `Architecture` → `Architecture` (verbatim).
- `Data Structure` → `Data Structure` (verbatim).
- `API Surface` → `API Surface` (verbatim).
- `Invariants` → `Constraints` — rename, and **collapse each bullet to one declarative sentence ≤120 chars** *while folding inline `Test:` / `Evidence:` / file-path cites into the same sentence with an em-dash*. E.g. `aclintSplit`'s I-2 (MMIO byte-identity, with `tests/aclint_mmio.rs` cite) becomes `C-2: External MMIO semantics are byte-identical for every (offset, size, value) triple — verified by tests/aclint_mmio.rs.`
- Append empty `[**CHANGELOG**]` section.
- Drop the `> Source:` banner.

**Bucket C — pre-workflow features with no archive (2 SPECs).** Slugs: `inst`, `cicd`.

- `inst` is a running-notes file of RISC-V instruction encoding tables — no archive counterpart. Collapse to template shape: `Goals` describes ISA coverage (RV32I/RV64I + M/A/Zicsr/C/Privileged), `Architecture` holds the encoding tables / dispatch layout, `Data Structure` lifts public `DecodedInst` / opcode enums, `API Surface` lists dispatch entry points, `Constraints` carries `cfg(isa32)/cfg(isa64)` rules. No `SPEC_LEGACY.md` (no archive bucket exists). `CHANGELOG` opens with `` - `2026-05-11` `port-to-ark`: migrated from running-notes; no prior archive — running-notes preserved at `.ark/tasks/archive/legacy/inst/` is N/A (no such dir). ``
- `cicd` is the GitHub Actions workflow definition; its archive companion is `.ark/tasks/archive/legacy/cicd/CICD.md` (single doc, not iteration-history). Apply Bucket-A treatment but `CHANGELOG` source references `.ark/tasks/archive/legacy/cicd/CICD.md` rather than a numbered PLAN.

Commit Phase 2b: `docs(specs): rewrite iteration-history specs to ark template shape`.

[**Phase 3 — INDEX + AGENTS + PROGRESS**]

3.1 Populate `.ark/specs/features/INDEX.md`. Between the existing `<!-- ARK:FEATURES:START -->` and `<!-- ARK:FEATURES:END -->` markers, the header row `| Feature | Scope | Promoted |` and separator row stay; add one row per slug. All `Promoted` dates use `2026-05-11` (migration date); future per-feature iterations re-stamp the row.

```
| aclintSplit      | Split ACLINT into MSWI / MTIMER / SSWI sub-devices.            | 2026-05-11 |
| amTests          | Bare-metal AM test harness for trap / interrupt / float.       | 2026-05-11 |
| archLayout       | Reorganise arch/<name>/ internal module layout.                | 2026-05-11 |
| archModule       | Consolidate arch backends into a single `arch/` module.        | 2026-05-11 |
| benchmark        | Dhrystone / coremark / microbench integration.                 | 2026-05-11 |
| boot             | OpenSBI + Linux boot sequence.                                 | 2026-05-11 |
| cicd             | GitHub Actions pipeline for fmt / clippy / tests / benches.    | 2026-05-11 |
| csr              | M/S/U CSR subsystem with WARL + shadow + trap signalling.      | 2026-05-11 |
| debian           | Debian 13 Trixie boot via VirtIO-blk.                          | 2026-05-11 |
| devices          | Device trait + Bus + ACLINT / PLIC / UART scaffolding.         | 2026-05-11 |
| difftest         | Per-instruction DUT/REF compare vs QEMU / Spike.               | 2026-05-11 |
| directIrq        | Direct device → PLIC async signalling; IrqState lock-free.     | 2026-05-11 |
| err2trap         | Refactor XError → PendingTrap split for architectural traps.   | 2026-05-11 |
| float            | F/D single + double float extension with softfloat.            | 2026-05-11 |
| inst             | RV32I/RV64I + M/A/Zicsr/C/Privileged instruction reference.    | 2026-05-11 |
| keyboard         | PTY-based UART RX path.                                        | 2026-05-11 |
| klib             | Freestanding C library for xam-built guests.                   | 2026-05-11 |
| memOpt           | Memory subsystem hot-path lock-reduction (follow-up to mm).    | 2026-05-11 |
| mm               | Memory subsystem: Bus + MMU + TLB + PMP + MMIO routing.        | 2026-05-11 |
| multiHart        | Multi-hart abstraction (HartId, cooperative scheduler).        | 2026-05-11 |
| perfBusFastPath  | Lock-free Bus on the per-instruction hot path.                 | 2026-05-11 |
| perfHotPath      | Mtimer deadline gate + icache + MMU inlining + typed RAM.      | 2026-05-11 |
| plicGateway      | PLIC level-triggered Gateway + Core split.                     | 2026-05-11 |
| trace            | Per-instruction trace / disassembly / log surfaces.            | 2026-05-11 |
```

3.2 Rewrite `AGENTS.md`. Final shape:

```markdown
# AGENTS.md

## Development Standards

- **Technical Research**: Always use web search to retrieve the latest official documentation.
- **Code Excellence**: Maintain a **clean, concise, and elegant** codebase. All implementations must strictly conform to the existing framework's architectural style.
- **Code Style:** Use a moderate amount of **functional** programming techniques.
- **Verification**: After making any coding-related modification, you must run `make fmt`, `make clippy`, `make run`, and `make test` to ensure correctness.

## Workflow

Driven by Ark. Read `.ark/workflow.md` for the full lifecycle. Start tasks with `/ark:quick` (trivial), `/ark:design` (standard), or `/ark:design --deep` (architectural).

<!-- ARK:START -->
Ark is installed in this project. Use `/ark:quick` or `/ark:design` to start tasks.

See `.ark/workflow.md` for the full workflow.

@.ark/specs/INDEX.md
<!-- ARK:END -->
```

The legacy `## Development Workflow` / `### Roles` / `### Iteration Rules` / `### Iteration Lifecycle` / `### Implementation` / `### Response Rules` sections are removed in their entirety. The `<!-- ARK:START --> ... <!-- ARK:END -->` block is preserved byte-identical to its current state.

3.3 Rewrite `docs/PROGRESS.md` cross-links. Substitution rules applied inside this single file:

- **Rule 1 — table-cell refs (lines 335-340, six rows).** `[`spec/<feature>/SPEC.md`](./spec/<feature>/SPEC.md)` → `[`features/<feature>/SPEC.md`](../.ark/specs/features/<feature>/SPEC.md)`. `[`archived/<cat>/<feature>/`](./archived/<cat>/<feature>/)` → `[`legacy/<feature>/`](../.ark/tasks/archive/legacy/<feature>/)`.
- **Rule 2 — `MANUAL_REVIEW.md` link (line 328).** `[MANUAL_REVIEW.md](./archived/review/MANUAL_REVIEW.md)` → `[MANUAL_REVIEW.md](../.ark/tasks/archive/legacy/MANUAL_REVIEW.md)`.
- **Rule 3 — closing paragraph (lines 342-346).** Rewrite to: `All seven MANUAL_REVIEW items are now addressed. Subsequent work uses Ark — `/ark:design --deep "<title>"` opens the task, `ark agent task commit` lands the spec under `.ark/specs/features/<feature>/SPEC.md`, and `ark archive` moves iteration history under `.ark/tasks/archive/YYYY-MM/<feature>/`. Read `.ark/workflow.md` for the lifecycle.`
- **Rule 4 — preamble parent-dir refs (lines 328-331), introduced by R-003.** The "Manual Review TODOs" preamble contains two parent-dir-relative refs that Rule 1 misses because their target is a *parent directory*, not a specific feature: ``[`docs/spec/<feature>/SPEC.md`](./spec/)`` → ``[`features/<feature>/SPEC.md`](../.ark/specs/features/)`` and ``[`docs/archived/<feature>/`](./archived/)`` → ``[`legacy/<feature>/`](../.ark/tasks/archive/legacy/)``. The full substitution rule: any markdown link whose URL is `./spec/...` or `./archived/...` is rewritten to `../.ark/specs/features/...` or `../.ark/tasks/archive/legacy/...` respectively. The link *text* is also rewritten so the visible cite reads `features/<feature>/SPEC.md` / `legacy/<feature>/` rather than `docs/spec/...` / `docs/archived/...`.
- The `[`/AGENTS.md`](../AGENTS.md)` link stays — AGENTS.md still exists, just slimmed.
- Phase tables, baseline links, roadmap commentary, design-principles section: unchanged.

Commit Phase 3: `docs: populate features index, slim agents.md, retarget progress links`.

[**Phase 4 — Source-code doc-comment rewrites**]

Run `rg "docs/archived" /Users/anekoique/ProjectX/xemu /Users/anekoique/ProjectX/xam`. The known hits (from a pre-migration scan, re-verified by the reviewer):

| File | Line | Current cite | Target cite |
|------|------|--------------|-------------|
| `xemu/xcore/src/device/irq.rs` | 6 | `docs/archived/fix/directIrq/02_PLAN.md` | `.ark/tasks/archive/legacy/directIrq/02_PLAN.md` |
| `xemu/xcore/tests/arch_isolation.rs` | 2 | `docs/archived/refactor/archModule/03_PLAN.md` | `.ark/tasks/archive/legacy/archModule/03_PLAN.md` |
| `xemu/xcore/src/arch/riscv/cpu.rs` | 289 | `docs/archived/perf/perfBusFastPath/03_PLAN.md` | `.ark/tasks/archive/legacy/perfBusFastPath/03_PLAN.md` |
| `xemu/xcore/src/arch/riscv/cpu/icache.rs` | 2 | `docs/archived/perf/perfHotPath/` | `.ark/tasks/archive/legacy/perfHotPath/` |
| `xemu/xcore/src/cpu/mod.rs` | 5 | `docs/archived/perf/perfBusFastPath/01_MASTER.md` | `.ark/tasks/archive/legacy/perfBusFastPath/01_MASTER.md` |
| `xemu/xcore/src/cpu/mod.rs` | 65 | `docs/archived/perf/perfBusFastPath/01_MASTER.md` | `.ark/tasks/archive/legacy/perfBusFastPath/01_MASTER.md` |

All six known hits are MASTER / PLAN level (iteration-specific findings) — keep them pointed at the archive. Where a cite points at an SPEC-level invariant (rather than a per-iteration MASTER), prefer pointing at `.ark/specs/features/<slug>/SPEC.md` for stable references. Substitution is purely string-level inside `///` and `//!` blocks; the executor MUST verify after each edit that the file compiles as a sanity check (`cargo check -p xcore` is sufficient).

Re-run `rg "docs/archived" /Users/anekoique/ProjectX/xemu /Users/anekoique/ProjectX/xam` and expect zero hits. Also run `rg "docs/spec" /Users/anekoique/ProjectX/xemu /Users/anekoique/ProjectX/xam` and `rg "docs/tasks" /Users/anekoique/ProjectX/xemu /Users/anekoique/ProjectX/xam`; expected zero hits.

Commit Phase 4: `docs(xemu): retarget archived-plan cites at .ark/tasks/archive/legacy`.

[**Phase 5 — Delete legacy doc trees**]

```bash
git rm -r docs/tasks docs/spec docs/archived docs/template
```

After the deletion, `ls docs/` must show exactly: `book/`, `perf/`, `PROGRESS.md`, `README.md` (plus possibly a `.DS_Store` to ignore). Final sanity check:

```bash
rg "docs/(tasks|spec|archived|template)" /Users/anekoique/ProjectX \
   --glob '!.ark/tasks/port-to-ark/**' \
   --glob '!.git/**'
```

Expected output: zero matches. The `--glob '!.ark/tasks/port-to-ark/**'` filter excludes this task's own iteration artifacts. Any other hit is a real bug; fix before declaring Phase 5 complete.

**Rollback note** (R-002 acceptance): the pre-Phase-5 commit hash is the Phase-4 commit (or whatever HEAD points at immediately before `git rm -r ...`). Capture it inline: `PRE_P5=$(git rev-parse HEAD)`. If Phase 6 surfaces a need for any deleted file, `git reset --hard $PRE_P5` reverses the deletion in-place. The stronger fallback — `git reset --hard $BASELINE_HEAD` from Runtime step 1 — rolls back the entire migration in this worktree without touching `main` (worktree-isolation property).

Commit Phase 5: `chore(docs): delete legacy docs/{tasks,spec,archived,template} trees`.

[**Phase 6 — Verification gates**]

```bash
make fmt
make clippy
make run
make test
```

All four must exit zero with output equivalent to the pre-migration baseline captured in Runtime step 1. `make run` and `make test` are particularly important: they exercise the doc-comments inside `cpu.rs` / `cpu/mod.rs` / `icache.rs` indirectly via compilation, catching any malformed `///` substitution. If `make linux` / `make debian` are normally part of the CI smoke set, run them too (PRD #8: "Build/test gates pass unchanged"). Commit any re-baseline byproducts as `chore: re-baseline make gates post-ark migration` (skip if no diff).

---

## Trade-offs

- T-1: Rewrite all 24 SPECs to template shape **vs** lazy migration (rewrite on next iteration per feature). Adv. of full rewrite: uniform structure for Ark's VERIFY spec-compliance pass; INDEX, agents, and contributors all see one shape. Disadv.: high one-shot effort; the five pre-workflow SPECs (`csr`, `klib`, `mm`, `memOpt`, `err2trap`) lose narrative density when collapsed to seven sections — long prose discussions of design rationale do not fit `Goals`/`Constraints`. **Choice: full rewrite**, per PRD Outcome #2. The narrative density loss is mitigated by (a) the legacy iteration files surviving verbatim under `.ark/tasks/archive/legacy/<slug>/` and (b) the new R-001 rule of preserving the running-notes SPEC at `SPEC_LEGACY.md`.
- T-2: Flat `archive/legacy/<slug>/` **vs** preserve `feat/`/`fix/`/`refactor/`/`perf/` subcategories. Ark's native archive layout is `archive/YYYY-MM/<slug>/` (flat per month). The legacy bucket mirrors that flatness so future contributors learn one structure, not two. Disadv.: loses the at-a-glance category browsing of `ls docs/archived/feat`. **Choice: flat**. Category metadata survives in (a) the legacy commit history via `git log --follow`, (b) the `docs/PROGRESS.md` "Manual Review TODOs" table, and (c) each SPEC's intrinsic shape.
- T-3: `git mv` (preserves history) **vs** copy + delete (cleaner diff, simpler `git status`). **Choice: `git mv`**. The `--follow` capability is worth the cluttered status; future archaeology on `directIrq/02_PLAN.md` should resolve to the original 2026-03-?? commit, not a 2026-05-11 "move" commit.
- T-4: Populate INDEX `Promoted` column with the migration date `2026-05-11` for all rows **vs** archaeological per-feature date via `git log -1 --format=%cd docs/spec/<slug>/SPEC.md`. **Choice: bulk migration date**. The INDEX `Promoted` column's semantics under Ark are "last touch by a deep commit" — the migration *is* the most recent touch. Ark will re-stamp on the next per-feature iteration.
- T-5: One mega-commit covering all phases **vs** one commit per phase. **Choice: per-phase commits**, seven total (one per Phase 1, 2a, 2b, 3, 4, 5, 6 — Phase 6 may be empty if `make` is byproduct-free). The Bucket-B split (Phase 2a before Phase 2b, TR-1 acceptance) gives the reviewer a diff window where the legacy `docs/spec/<slug>/SPEC.md` still exists side-by-side with the new `.ark/specs/features/<slug>/SPEC.md`. Each subject is lowercase imperative ≤72 chars and conforms to `coding/git/SPEC.md` R1–R4:

  | Phase | Commit subject |
  |-------|----------------|
  | 1 | `chore(docs): relocate legacy archive under .ark/tasks/archive/legacy` |
  | 2a | `docs(specs): rewrite running-notes specs (csr klib mm memOpt err2trap) to ark template` |
  | 2b | `docs(specs): rewrite iteration-history specs to ark template shape` |
  | 3 | `docs: populate features index, slim agents.md, retarget progress links` |
  | 4 | `docs(xemu): retarget archived-plan cites at .ark/tasks/archive/legacy` |
  | 5 | `chore(docs): delete legacy docs/{tasks,spec,archived,template} trees` |
  | 6 | `chore: re-baseline make gates post-ark migration` (optional — skip if no diff) |

  Phase 2a's subject is 84 chars including the slug list, which exceeds R4's ~72-char guideline. The reviewer's R-005 recommendation lists this verbatim because the slug enumeration is load-bearing for archaeology (a future `git log --oneline` reader can see which specs the running-notes pass touched without opening the commit). The R4 guidance is "fits ~72 characters so `git log --oneline` stays readable" — soft cap, not hard. **Choice: keep the long subject for Phase 2a.** If a hard 72-char cap is required at merge time, the fallback is `docs(specs): rewrite five running-notes specs to ark template` (66 chars) with the slug list moved to the body.

---

## Validation

[**Unit Tests**]

- V-UT-1: N/A — no Rust source semantics change. The pre- vs post-migration `cargo test --workspace` run acts as the regression baseline (V-IT-1).

[**Integration Tests**]

- V-IT-1: `make fmt && make clippy && make test` produces identical pass/fail outcomes pre- and post-migration. (Maps C-4.)
- V-IT-2: `make run` (dhrystone / coremark / microbench) and `make linux` / `make debian` boot smoke-tests succeed unchanged. Catches any doc-comment edit that inadvertently changes compilation.

[**Failure / Robustness**]

- V-F-1: After Phase 5, `rg "docs/(tasks|spec|archived|template)" /Users/anekoique/ProjectX --glob '!.ark/tasks/port-to-ark/**' --glob '!.git/**'` returns zero matches. (Maps C-3.)
- V-F-2: For every slug under the legacy `docs/spec/` set, `.ark/specs/features/<slug>/SPEC.md` exists and contains all seven Ark template section headings (`[**Goals**]`, `[**Non-goals**]`, `[**Architecture**]`, `[**Data Structure**]`, `[**API Surface**]`, `[**Constraints**]`, `[**CHANGELOG**]`) in order. Automate via shell loop: `for s in <24 slugs>; do for h in 'Goals' 'Non-goals' 'Architecture' 'Data Structure' 'API Surface' 'Constraints' 'CHANGELOG'; do grep -F "[**$h**]" .ark/specs/features/$s/SPEC.md || echo MISSING $s $h; done; done`. (Maps C-1 + C-2.)
- V-F-3: `git log --follow .ark/tasks/archive/legacy/directIrq/02_PLAN.md | head -20` shows the pre-migration commit history (not just the rename commit). (Maps C-7.)
- V-F-4: `diff -r <pre-migration archive snapshot> .ark/tasks/archive/legacy/` shows only path differences, no content diffs. (Maps C-5.)
- V-F-5: For each Bucket-B slug, `.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md` exists and is byte-identical to the pre-migration `docs/spec/<slug>/SPEC.md` (verified by `diff -q docs/spec/<slug>/SPEC.md .ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md` *before* Phase 5; `sha256sum` recorded in Phase 2a commit body for post-Phase-5 verification). (Maps R-001 acceptance.)
- V-F-6: Rollback dry-run — `git reset --hard $BASELINE_HEAD` (in a throwaway worktree clone, not the real one) followed by `git diff $BASELINE_HEAD` shows zero diff. Confirms the rollback target is well-formed. (Maps R-002 acceptance.)

[**Edge Cases**]

- V-E-1: `docs/archived/review/MANUAL_REVIEW.md` is a file, not a directory. The `git mv` step places it as a flat file at `.ark/tasks/archive/legacy/MANUAL_REVIEW.md` (not `.ark/tasks/archive/legacy/MANUAL_REVIEW/`). Verified by `test -f .ark/tasks/archive/legacy/MANUAL_REVIEW.md && ! test -d .ark/tasks/archive/legacy/MANUAL_REVIEW`.
- V-E-2: Slug spelling differs between `docs/spec/` and `docs/archived/` in no observed case. If discovered during migration, the spelling in `docs/spec/` wins (the SPEC is canonical); rename the archive subdir to match before the `git mv` in Phase 1.
- V-E-3: Pre-workflow features with no `NN_` iteration files (`csr`, `klib`, `mm`, `err2trap`) have irregular archive shape (`PLAN.md`, `PLAN_REVIEW.md`, `IMPL_REVIEW.md`, `MEM_OPTIMIZATION_PLAN.md`, `ERR2TRAP.md`, etc.). Phase 1 copies whatever is present verbatim under the slug; Phase 2a synthesises Goals/Constraints from the running notes and preserves the running-notes SPEC as `SPEC_LEGACY.md`.
- V-E-4: `docs/spec/inst/SPEC.md` has no archive counterpart. Phase 1 does not create `.ark/tasks/archive/legacy/inst/`. Phase 2b still produces `.ark/specs/features/inst/SPEC.md` from the running notes; the migrated SPEC's CHANGELOG lacks an archive pointer (no SPEC_LEGACY.md either — there is no archive bucket to host it). Phase 3's INDEX row for `inst` is still included.
- V-E-5: `memOpt` shares an archive bucket with `mm` (the `MEM_OPTIMIZATION_PLAN.md` lives at `.ark/tasks/archive/legacy/mm/MEM_OPTIMIZATION_PLAN.md`). The `memOpt` SPEC's `CHANGELOG` source pointer references that path explicitly. Phase 2a preserves `memOpt`'s pre-workflow SPEC at `.ark/tasks/archive/legacy/mm/SPEC_LEGACY_memOpt.md` (sibling of mm's SPEC_LEGACY.md). Phase 1 does not create `.ark/tasks/archive/legacy/memOpt/`.
- V-E-6: `docs/archived/feat/cicd/CICD.md` is a single doc, not iteration-history. Phase 1 places it at `.ark/tasks/archive/legacy/cicd/CICD.md`; Phase 2b's `cicd` SPEC references it as the source.
- V-E-7: Two known `.DS_Store` files exist — `docs/.DS_Store` and `docs/archived/feat/csr/.DS_Store`. Both are macOS cruft; do not migrate. The `find docs -name .DS_Store -delete` step before Phase 1 covers both. (R-007 wording fix.)
- V-E-8: The `<!-- ARK:FEATURES:START --> ... <!-- ARK:FEATURES:END -->` markers inside `.ark/specs/features/INDEX.md` must survive Phase 3 verbatim — they are how the Ark CLI locates the table on future `ark agent spec register` calls. Edit only the content *between* the markers; the markers themselves stay byte-identical.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-F-1 (no `docs/(tasks|spec|archived|template)` references remain). |
| G-2 | V-F-2 (every slug has a template-shaped SPEC under `.ark/specs/features/`). |
| G-3 | V-F-2 (INDEX row presence is checked alongside SPEC presence). |
| G-4 | V-F-3 + V-F-4 (history preserved via `git mv`; byte-identical archive content). |
| G-5 | V-IT-1 (gates pass) + manual diff of slimmed `AGENTS.md` + `docs/PROGRESS.md` link inspection (no broken links after Phase 5; `rg` zero-hit confirms link substitutions landed). |
| C-1 | V-F-2 (seven section headings in order). |
| C-2 | V-F-2 (INDEX rows ↔ SPEC files). |
| C-3 | V-F-1 (post-Phase-5 `rg` zero-hit). |
| C-4 | V-IT-1 + V-IT-2 (full gate run). |
| C-5 | V-F-4 (byte-identical archive diff). |
| C-6 | Manual `diff <(grep -A20 "## Development Standards" AGENTS.md) <prev>` to confirm Standards block unchanged; `grep -F "ARK:START" AGENTS.md` non-obvious. |
| C-7 | V-F-3 (`git log --follow` reaches pre-migration commits). |
| C-8 | Manual inspection of `docs/PROGRESS.md` diff — phase tables, baseline links, roadmap text unchanged outside of cross-link substrings. |
| R-001 | V-F-5 (Bucket-B `SPEC_LEGACY.md` byte-identical to original). |
| R-002 | V-F-6 (rollback dry-run produces zero diff against `$BASELINE_HEAD`). |