# `port-to-ark` PLAN `02`

> Status: Draft
> Feature: `port-to-ark`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`

---

## Summary

Retire the bespoke `docs/`-based PLAN/REVIEW/MASTER workflow and consolidate every task, archive, and feature-spec primitive under `.ark/`. The migration ships in seven independently inspectable, per-phase commits: (1) move the ~270 archived iteration files verbatim under `.ark/tasks/archive/legacy/<slug>/` via `git mv`, collapsing the `feat/`/`fix/`/`refactor/`/`perf/` subcategories; (2a) author the five Bucket-B SPECs (`csr`, `klib`, `mm`, `memOpt`, `err2trap`) at `.ark/specs/features/<slug>/SPEC.md` and stage each pre-workflow source verbatim under `.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md` so the reviewer can diff legacy → migrated while the original still exists; (2b) author the remaining 19 SPECs (17 Bucket-A iteration-history + 2 Bucket-C); (3) populate `.ark/specs/features/INDEX.md` between the `ARK:FEATURES` markers, slim `AGENTS.md` to standards + Ark pointer, and rewrite every `docs/PROGRESS.md` cross-link; (4) substitute the 6 `docs/archived/...` paths cited inside Rust doc-comments under `xemu/` with their new `.ark/tasks/archive/legacy/...` destinations; (5) `git rm -r docs/{tasks,spec,archived,template}` (legacy sources go now, after Bucket-B diff window closes); (6) re-run `make fmt && make clippy && make run && make test` to confirm no regressions. Each phase lands as one Conventional Commits subject that satisfies `coding/git/SPEC.md` R1–R4. The Phase-5 destruction is reversible via `git reset --hard $BASELINE_HEAD` captured at Runtime step 1 — the worktree-isolation (`feat/port-to-ark`) means any rollback never propagates to `main`.

## Log

[**Added**]

- Phase 3.3 Rule 5 for `docs/`-prefixed link-text rewriting (closes round-01 R-002). Known hits at `docs/PROGRESS.md:23` and `docs/PROGRESS.md:190`.
- `.ark/tasks/archive/legacy/memOpt/` directory with canonical `SPEC_LEGACY.md` + `README.md` pointer (closes round-01 R-003).

[**Changed**]

- Phase 2a Worked example for `csr` rebuilt against verified codebase layout (`cpu/csr.rs` + `cpu/csr/` + `cpu/trap.rs` + `cpu/trap/`); Architecture tree, Data Structure block, API Surface, and Constraints C-1..C-8 cite real paths (closes round-01 R-001 CRITICAL).
- Trade-off T-5 commit table: Phase 2a subject swapped to 62-char primary `docs(specs): rewrite five running-notes specs to ark template`; slug list moved to commit body (closes round-01 R-004).
- `inst` CHANGELOG line in Phase 2b rewritten to drop the self-negating "preserved at … is N/A" clause (closes round-01 R-005).
- V-F-6 lightened to `git cat-file -e` + `git log --oneline` instead of throwaway-worktree dry-run (closes round-01 R-006).
- V-E-5 updated for the new `memOpt/SPEC_LEGACY.md` path; V-F-5 command adjusted likewise (R-003 follow-through).

[**Removed**]

- The conjectural directory tree under `xemu/xcore/src/arch/riscv/csr/` and its `trap.rs` / `trap_handler.rs` entries — replaced by the real layout sourced from the on-disk codebase.

[**Unresolved**]

- None. All six round-01 findings (R-001..R-006) and TR-1 are addressed in this iteration.

[**Response Matrix**]

| ID | Severity | Disposition | Resolution |
|----|----------|-------------|------------|
| R-001 | CRITICAL | Accepted | Phase 2a Worked example for `csr` fully rewritten against the verified on-disk layout: Architecture tree shows `cpu/csr.rs` + `cpu/csr/{mip,mstatus,ops,privilege}.rs` and `cpu/trap.rs` + `cpu/trap/{cause,exception,handler,interrupt}.rs`; Data Structure lifts the real `AccessRule` / `CsrDesc` / `CsrFile` / `PendingTrap` / `TrapCause` declarations; API Surface enumerates the real method signatures from `csr.rs`, `trap.rs`, and `trap/handler.rs`; Constraints C-1..C-8 cite real file paths. Closing prose adds a "spot-checked against the real codebase BEFORE landing" guard. |
| R-002 | HIGH | Accepted | Phase 3.3 grows a fifth substitution rule: any markdown link whose *display text* (inside backticks) reads `` `docs/(spec|archived)/<...>` `` is rewritten — `docs/` prefix dropped, `spec/` → `features/`, `archived/<cat>/` → `legacy/`. Rule body explicitly names `docs/PROGRESS.md:23` and `docs/PROGRESS.md:190` so the executor does not re-derive the hits. |
| R-003 | MEDIUM | Accepted | Phase 2a step 1 + V-E-5 + V-F-5 switch `memOpt`'s legacy SPEC out of `mm/`'s bucket: it now lives at `.ark/tasks/archive/legacy/memOpt/SPEC_LEGACY.md` with a sibling `README.md` cross-pointer to `legacy/mm/MEM_OPTIMIZATION_PLAN.md` for the running-notes iteration history. Every Bucket-B slug now satisfies the canonical `**/SPEC_LEGACY.md` glob. |
| R-004 | MEDIUM | Accepted | Trade-off T-5 table: Phase 2a primary subject swapped to `docs(specs): rewrite five running-notes specs to ark template` (62 chars). The slug list (`csr klib mm memOpt err2trap`) moves into the commit body. The 86-char form is documented as an alternative for wide-terminal preference but flagged as non-conforming with R4. |
| R-005 | MEDIUM | Accepted | Phase 2b's `inst` CHANGELOG line rewritten to a single clean assertion: `` - `2026-05-11` `port-to-ark`: migrated from running-notes; no archive bucket (pre-workflow source had no iteration history). `` The "preserved at … is N/A" clause is dropped entirely. |
| R-006 | LOW | Accepted | V-F-6 rewritten: `git cat-file -e $BASELINE_HEAD || echo "BASELINE_HEAD unreachable"` confirms the rollback target is reachable; `git log --oneline $BASELINE_HEAD..HEAD | wc -l` reports the phase-commit count. No throwaway worktree needed. |
| TR-1 | Trade-off | Accepted | Round 02 is a tight delta: only the Worked Example (Phase 2a) is rewritten at body-scale; R-002, R-003, R-004, R-005, R-006 are 1–3 line patches. Goals / Non-goals / Architecture / Data Structure / API Surface / Constraints in the Spec block are restated verbatim from round 01 (still self-contained per the workflow rule). |

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
│               ├── memOpt/                      (+) new dir; pre-workflow SPEC_LEGACY.md + README.md cross-pointer
│               ├── mm/                          (+) from feat/ (pre-workflow; +SPEC_LEGACY.md; carries MEM_OPTIMIZATION_PLAN.md for memOpt)
│               ├── multiHart/                   (+) from feat/
│               ├── perfBusFastPath/             (+) from perf/
│               ├── perfHotPath/                 (+) from perf/
│               ├── plicGateway/                 (+) from fix/
│               └── trace/                       (+) from feat/
└── xemu/                                        (*) doc-comment cite strings updated
    └── xcore/src/...                            no semantic changes
```

The 24 migrated slugs (matching `docs/spec/<slug>/`): `aclintSplit`, `amTests`, `archLayout`, `archModule`, `benchmark`, `boot`, `cicd`, `csr`, `debian`, `devices`, `difftest`, `directIrq`, `err2trap`, `float`, `inst`, `keyboard`, `klib`, `memOpt`, `mm`, `multiHart`, `perfBusFastPath`, `perfHotPath`, `plicGateway`, `trace`. Note: `inst` has no archive counterpart (running-notes only); `memOpt`'s iteration history lives inside `legacy/mm/` (the running-notes source was a section in mm's pre-workflow SPEC), with a dedicated `legacy/memOpt/` dir holding its own `SPEC_LEGACY.md` + cross-pointer.

[**Data Structure**]

This migration ships no Rust types. The conceptual schemas being mapped are:

- Legacy SPEC shape (`Source:` banner + free prose OR `Goals / Architecture / Invariants / Data Structure / API Surface`) → Ark template shape (`Goals / Non-goals / Architecture / Data Structure / API Surface / Constraints / CHANGELOG`).
- Legacy archive layout (`docs/archived/<feat|fix|refactor|perf|review>/<slug>/`) → Ark legacy bucket (`.ark/tasks/archive/legacy/<slug>/`, flat — no category subdir).
- Bucket-B preservation pair: each pre-workflow SPEC produces (a) a migrated `.ark/specs/features/<slug>/SPEC.md` in template shape AND (b) a verbatim copy of the original at `.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md` (canonical filename for every Bucket-B slug, including `memOpt`).

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
3. Phase 2a — Bucket-B SPECs. For each of `csr`, `klib`, `mm`, `memOpt`, `err2trap`: (i) copy the legacy running-notes source verbatim into `.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md` (for `memOpt`, create the dir `.ark/tasks/archive/legacy/memOpt/` first and add the `README.md` cross-pointer to `legacy/mm/MEM_OPTIMIZATION_PLAN.md`); (ii) author `.ark/specs/features/<slug>/SPEC.md` in the Ark template shape. Commit as `docs(specs): rewrite five running-notes specs to ark template` (slug list in body).
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
- `.ark/tasks/archive/` empty → contains `legacy/<slug>/` subtrees (Phase 1) → `legacy/<slug>/SPEC_LEGACY.md` added for Bucket-B slugs, and `legacy/memOpt/` dir created (Phase 2a).
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

`memOpt` has no separate pre-existing archive dir — its iteration source (`MEM_OPTIMIZATION_PLAN.md`) lives inside `legacy/mm/` after the move. Phase 2a (below) creates the new `legacy/memOpt/` dir explicitly for the preserved SPEC + README cross-pointer.

Pre-Phase-1 hygiene: `find docs -name .DS_Store -delete` removes macOS cruft so it doesn't get migrated.

Post-Phase-1 inspection: `git status --short | wc -l` should show roughly 270+ rename entries. `ls .ark/tasks/archive/legacy/` should list 22 subdirs plus `MANUAL_REVIEW.md`. Commit: `chore(docs): relocate legacy archive under .ark/tasks/archive/legacy`.

[**Phase 2a — Bucket-B SPECs (pre-workflow running-notes features, 5 SPECs)**]

Bucket B covers `csr`, `klib`, `mm`, `memOpt`, `err2trap`. Each has a long-form pre-workflow design doc under a `Source:` banner. The migration is qualitative — the running notes are denser than any seven-section template can hold, so this phase carries the highest quality risk. **TR-1 (round 00) accepted**: Phase 2a runs before Phase 5 specifically so the reviewer can `diff docs/spec/<slug>/SPEC.md .ark/specs/features/<slug>/SPEC.md` while the original still exists.

**Per-slug procedure** (apply to all five):

1. **Preserve.** `cp docs/spec/<slug>/SPEC.md .ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md`. The file is byte-identical to the original. For `memOpt`, this requires creating the dir first: `mkdir -p .ark/tasks/archive/legacy/memOpt/`, then `cp docs/spec/memOpt/SPEC.md .ark/tasks/archive/legacy/memOpt/SPEC_LEGACY.md`. Also add `.ark/tasks/archive/legacy/memOpt/README.md` containing the single line: `Iteration history for the memOpt SPEC lives at ../mm/MEM_OPTIMIZATION_PLAN.md (the original was a section in the mm running-notes).` This keeps every Bucket-B slug under the canonical `<slug>/SPEC_LEGACY.md` glob so VERIFY/SPEC-drift tooling never misses one.
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

The migrated `.ark/specs/features/csr/SPEC.md`. The executor authors this verbatim; the algorithm above produced it from `docs/spec/csr/SPEC.md` (834 lines) **and** the verified on-disk layout under `xemu/xcore/src/arch/riscv/cpu/`. Every file path, type name, and method signature below was checked against `csr.rs` / `csr/{mip,mstatus,ops,privilege}.rs` / `trap.rs` / `trap/{cause,exception,handler,interrupt}.rs` before being written. The other four Bucket-B slugs follow the same algorithm but their bodies are not pre-filled here.

````markdown
[**Goals**]

- G-1: Provide WARL-masked reads and writes for M/S privilege CSRs (mstatus, sstatus, mip, sie, mie, satp, ...).
- G-2: Route every architectural trap — ecall / ebreak / illegal-inst / illegal-CSR / page-fault — through stvec / mtvec via medeleg / mideleg.
- G-3: Shadow S-mode CSRs (sstatus, sip, sie) onto M-mode storage via a single descriptor table — no duplicate state.
- G-4: Generate the difftest CSR whitelist from `csr_table!` `@ difftest` annotations.
- G-5: Hold privilege checking + dynamic rules in `RVCore`; keep `CsrFile` to storage, masks, shadows only.

[**Non-goals**]

- NG-1: No HPM / hpmcounter / hpmevent CSRs beyond write-through stubs.
- NG-2: No F / D / V-extension CSRs (fcsr / vstart / vl) — they belong to the float / vector subsystems.
- NG-3: No vectored `mtvec` mode (BASE+4×cause) — `mtvec` wmask forces direct mode.

[**Architecture**]

```
xemu/xcore/src/arch/riscv/cpu/
├── csr.rs               CsrFile, CsrDesc, AccessRule, csr_table!, find_desc, DIFFTEST_CSRS
├── csr/
│   ├── mip.rs           bitflags! Mip
│   ├── mstatus.rs       bitflags! MStatus + mpp/with_mpp/spp/with_spp helpers
│   ├── ops.rs           impl RVCore CSR read/write entry points
│   └── privilege.rs     PrivilegeMode { M, S, U } + from_bits
├── trap.rs              impl RVCore { trap, trap_exception, illegal_inst, trap_on_err }
└── trap/
    ├── cause.rs         TrapCause + PendingTrap
    ├── exception.rs     Exception enum
    ├── handler.rs       impl RVCore { check_pending_interrupts, commit_trap, do_mret, do_sret }
    └── interrupt.rs     Interrupt enum
```

Layering: `CsrFile` is storage + WARL masks + shadow descriptors; `RVCore` owns privilege checks, dynamic rules (TSR / TVM / counteren), side effects, and trap generation. Trap pipe is Err-driven: architectural traps emit `Err(XError::Trap(PendingTrap{cause, tval}))`, `trap_on_err` drains the Err in `cpu/trap.rs`, and `handler::commit_trap` is the single PC/CSR commit point.

[**Data Structure**]

```rust
pub struct CsrFile { /* see csr.rs */ }

pub struct CsrDesc {
    wmask:      Word,
    storage:    u16,
    view_mask:  Word,
    view_shift: u8,
    access:     AccessRule,
}

pub enum AccessRule {
    Standard,
    BlockedByMstatus(MStatus),
    CounterGated,
    RequireFP,
}

pub struct PendingTrap {
    pub cause: TrapCause,
    pub tval:  Word,
}

pub enum TrapCause {
    Exception(Exception),
    Interrupt(Interrupt),
}

bitflags! {
    pub struct MStatus: Word {
        // full flag set lifted verbatim from cpu/csr/mstatus.rs
        // (SIE / MIE / SPIE / MPIE / SPP / MPP / FS / XS / MPRV / SUM / MXR / TVM / TW / TSR / SD / ...)
    }
}

bitflags! {
    pub struct Mip: Word {
        // full flag set lifted verbatim from cpu/csr/mip.rs
    }
}

pub enum PrivilegeMode { M, S, U }
pub enum Exception { /* see cpu/trap/exception.rs */ }
pub enum Interrupt { /* see cpu/trap/interrupt.rs */ }
```

[**API Surface**]

```rust
impl CsrFile {
    pub fn new() -> Self;
    pub fn get(&self, addr: CsrAddr) -> Word;
    pub fn get_by_addr(&self, addr: u16) -> Word;
    pub fn set(&mut self, addr: CsrAddr, val: Word);
    pub fn read_with_desc(&self, desc: CsrDesc) -> Word;
    pub fn write_with_desc(&mut self, desc: CsrDesc, val: Word);
    pub fn read_masked(&self, addr: u16) -> Option<Word>;
    pub fn write_masked(&mut self, addr: u16, val: Word) -> bool;
    pub fn increment_cycle(&mut self);
    pub fn increment_instret(&mut self);
}

// Trap pipe — crate-internal helpers that produce Err(XError::Trap(...)).
impl RVCore {
    pub(in crate::arch::riscv) fn trap(&mut self, cause: TrapCause, tval: Word) -> XError;
    pub(in crate::arch::riscv) fn trap_exception(&mut self, exc: Exception, tval: Word) -> XError;
    pub(in crate::arch::riscv) fn illegal_inst(&mut self, raw: Word) -> XError;
    pub(in crate::arch::riscv) fn trap_on_err(&mut self, err: XError) -> Result<(), XError>;
}

// Handler — public commit + return-from-trap surface.
impl RVCore {
    pub fn check_pending_interrupts(&mut self) -> bool;
    pub fn commit_trap(&mut self, trap: PendingTrap);
    pub fn do_mret(&mut self);
    pub fn do_sret(&mut self);
}

// Macro generates CsrAddr enum + find_desc + DIFFTEST_CSRS.
macro_rules! csr_table { /* see csr.rs:66 */ }
// Generated items:
//   pub enum CsrAddr { ... }
//   pub(in crate::arch::riscv) fn find_desc(addr: u16) -> Option<CsrDesc>;
//   pub const DIFFTEST_CSRS: &[(CsrAddr, u64)];
```

[**Constraints**]

- C-1: `mstatus` is the master; `sstatus` is a subset view over `mstatus` storage — `xemu/xcore/src/arch/riscv/cpu/csr/mstatus.rs`.
- C-2: Architectural traps emit `Err(XError::Trap(PendingTrap{cause,tval}))`; `trap_on_err` drains them into `commit_trap` — `xemu/xcore/src/arch/riscv/cpu/trap.rs`.
- C-3: `commit_trap` writes the trap-target `pc` and clears the pending state in one place — `xemu/xcore/src/arch/riscv/cpu/trap/handler.rs`.
- C-4: `CsrFile` is dumb storage + WARL masking; privilege/dynamic rules/side effects live in `RVCore` — `xemu/xcore/src/arch/riscv/cpu/csr.rs`.
- C-5: `csr_table!` is the single source of `CsrAddr` and `find_desc`; the two cannot drift apart — `xemu/xcore/src/arch/riscv/cpu/csr.rs:66`.
- C-6: WARL masking applies on every write — `write_with_desc` ANDs `view_mask & wmask` — `xemu/xcore/src/arch/riscv/cpu/csr.rs:316`.
- C-7: `mret` and `sret` restore privilege from `mstatus.MPP` / `mstatus.SPP` and clear `MPRV` per spec — `xemu/xcore/src/arch/riscv/cpu/trap/handler.rs:129`.
- C-8: Illegal CSR access (unknown addr, insufficient privilege, RO write, dynamic-rule violation) raises `Exception::IllegalInstruction` — never returns `Err(XError::Other)` — `xemu/xcore/src/arch/riscv/cpu/csr/ops.rs`.

[**CHANGELOG**]

- `2026-05-11` `port-to-ark`: migrated from running-notes SPEC; full original preserved at `.ark/tasks/archive/legacy/csr/SPEC_LEGACY.md`.
````

The other four Bucket-B slugs are authored by the same algorithm. Each slug's algorithm output is spot-checked against the real codebase layout BEFORE landing; do not synthesize file paths from intuition.

- **`klib`** (192 lines): Goals around "freestanding C library for xam-built guests", "newlib stub coverage", "no_std-friendly link surface". Architecture = directory tree of `xam/klib/`. Constraints lift from the doc's "Layering" / "Why" bullets.
- **`mm`** (1206 lines): Goals around "Bus + MMU + TLB + PMP + MMIO routing", "dual RV32/RV64", "Sv32/39/48/57 page walking". Architecture = the four-layer responsibility split + the access-path diagram. Data Structure lifts `Bus`, `MmioRegion`, `Mmu`, `Tlb`, `Pmp`, `Pte`, `SvMode`. Constraints lift from the four-layer responsibility table + design-decision bullets. ~10 constraints, each ≤120 chars.
- **`memOpt`** (211 lines): Goals around "hot-path lock reduction follow-up to mm". `SPEC_LEGACY.md` lives in its own `.ark/tasks/archive/legacy/memOpt/` dir; the sibling `README.md` points at `../mm/MEM_OPTIMIZATION_PLAN.md` for the iteration history. CHANGELOG references both: the preserved SPEC and the cross-pointer.
- **`err2trap`** (480 lines): Goals around "split `XError::Trap(...)` into `PendingTrap` + reserved `XError` for host I/O". Constraints lift the `Err` reservation rule, the `pending_trap` write contract, the trap-pipe commit ordering — same rules `csr` enforces, but framed from the refactor side.

Commit Phase 2a (primary subject ≤72 chars per `coding/git/SPEC.md` R4):

```
docs(specs): rewrite five running-notes specs to ark template

Rewrites csr, klib, mm, memOpt, err2trap from running-notes to the
seven-section Ark template; preserves each pre-workflow source at
.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md.
```

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

- `inst` is a running-notes file of RISC-V instruction encoding tables — no archive counterpart. Collapse to template shape: `Goals` describes ISA coverage (RV32I/RV64I + M/A/Zicsr/C/Privileged), `Architecture` holds the encoding tables / dispatch layout, `Data Structure` lifts public `DecodedInst` / opcode enums, `API Surface` lists dispatch entry points, `Constraints` carries `cfg(isa32)/cfg(isa64)` rules. No `SPEC_LEGACY.md` (no archive bucket exists). `CHANGELOG` opens with: `` - `2026-05-11` `port-to-ark`: migrated from running-notes; no archive bucket (pre-workflow source had no iteration history). ``
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
- **Rule 4 — preamble parent-dir refs (lines 328-331).** The "Manual Review TODOs" preamble contains two parent-dir-relative refs that Rule 1 misses because their target is a *parent directory*, not a specific feature: ``[`docs/spec/<feature>/SPEC.md`](./spec/)`` → ``[`features/<feature>/SPEC.md`](../.ark/specs/features/)`` and ``[`docs/archived/<feature>/`](./archived/)`` → ``[`legacy/<feature>/`](../.ark/tasks/archive/legacy/)``. The full substitution rule: any markdown link whose URL is `./spec/...` or `./archived/...` is rewritten to `../.ark/specs/features/...` or `../.ark/tasks/archive/legacy/...` respectively.
- **Rule 5 — `docs/`-prefixed link-text rewriting (closes round-01 R-002).** Any markdown inline-code that reads `` `docs/(spec|archived)/<...>` `` inside a link's display-text MUST also be rewritten: drop the `docs/` prefix and replace `spec/` → `features/`, `archived/<cat>/` → `legacy/`. Example: ``[`docs/archived/perf/perfBusFastPath/`](./archived/perf/perfBusFastPath/)`` → ``[`legacy/perfBusFastPath/`](../.ark/tasks/archive/legacy/perfBusFastPath/)``. **Known hits at `docs/PROGRESS.md:23` and `docs/PROGRESS.md:190`** — both use this `docs/`-in-text form. Rule 5 applies in addition to Rules 1/4 (which fix the URL); Rule 5 fixes the visible text so V-F-1's `rg "docs/(tasks|spec|archived|template)"` post-Phase-5 zero-hit holds.
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

**Rollback note** (R-002 round-00 acceptance): the pre-Phase-5 commit hash is the Phase-4 commit (or whatever HEAD points at immediately before `git rm -r ...`). Capture it inline: `PRE_P5=$(git rev-parse HEAD)`. If Phase 6 surfaces a need for any deleted file, `git reset --hard $PRE_P5` reverses the deletion in-place. The stronger fallback — `git reset --hard $BASELINE_HEAD` from Runtime step 1 — rolls back the entire migration in this worktree without touching `main` (worktree-isolation property).

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

- T-1: Rewrite all 24 SPECs to template shape **vs** lazy migration (rewrite on next iteration per feature). Adv. of full rewrite: uniform structure for Ark's VERIFY spec-compliance pass; INDEX, agents, and contributors all see one shape. Disadv.: high one-shot effort; the five pre-workflow SPECs (`csr`, `klib`, `mm`, `memOpt`, `err2trap`) lose narrative density when collapsed to seven sections — long prose discussions of design rationale do not fit `Goals`/`Constraints`. **Choice: full rewrite**, per PRD Outcome #2. The narrative density loss is mitigated by (a) the legacy iteration files surviving verbatim under `.ark/tasks/archive/legacy/<slug>/` and (b) the round-00 R-001 rule of preserving the running-notes SPEC at `SPEC_LEGACY.md` (every Bucket-B slug now under the canonical filename, including `memOpt`).
- T-2: Flat `archive/legacy/<slug>/` **vs** preserve `feat/`/`fix/`/`refactor/`/`perf/` subcategories. Ark's native archive layout is `archive/YYYY-MM/<slug>/` (flat per month). The legacy bucket mirrors that flatness so future contributors learn one structure, not two. Disadv.: loses the at-a-glance category browsing of `ls docs/archived/feat`. **Choice: flat**. Category metadata survives in (a) the legacy commit history via `git log --follow`, (b) the `docs/PROGRESS.md` "Manual Review TODOs" table, and (c) each SPEC's intrinsic shape.
- T-3: `git mv` (preserves history) **vs** copy + delete (cleaner diff, simpler `git status`). **Choice: `git mv`**. The `--follow` capability is worth the cluttered status; future archaeology on `directIrq/02_PLAN.md` should resolve to the original 2026-03-?? commit, not a 2026-05-11 "move" commit.
- T-4: Populate INDEX `Promoted` column with the migration date `2026-05-11` for all rows **vs** archaeological per-feature date via `git log -1 --format=%cd docs/spec/<slug>/SPEC.md`. **Choice: bulk migration date**. The INDEX `Promoted` column's semantics under Ark are "last touch by a deep commit" — the migration *is* the most recent touch. Ark will re-stamp on the next per-feature iteration.
- T-5: One mega-commit covering all phases **vs** one commit per phase. **Choice: per-phase commits**, seven total (one per Phase 1, 2a, 2b, 3, 4, 5, 6 — Phase 6 may be empty if `make` is byproduct-free). The Bucket-B split (Phase 2a before Phase 2b, round-00 TR-1 acceptance) gives the reviewer a diff window where the legacy `docs/spec/<slug>/SPEC.md` still exists side-by-side with the new `.ark/specs/features/<slug>/SPEC.md`. Each subject is lowercase imperative ≤72 chars and conforms to `coding/git/SPEC.md` R1–R4:

  | Phase | Commit subject (primary, R4-conforming) | Chars |
  |-------|------------------------------------------|-------|
  | 1 | `chore(docs): relocate legacy archive under .ark/tasks/archive/legacy` | 69 |
  | 2a | `docs(specs): rewrite five running-notes specs to ark template` | 62 |
  | 2b | `docs(specs): rewrite iteration-history specs to ark template shape` | 68 |
  | 3 | `docs: populate features index, slim agents.md, retarget progress links` | 72 |
  | 4 | `docs(xemu): retarget archived-plan cites at .ark/tasks/archive/legacy` | 70 |
  | 5 | `chore(docs): delete legacy docs/{tasks,spec,archived,template} trees` | 70 |
  | 6 | `chore: re-baseline make gates post-ark migration` (optional — skip if no diff) | 49 |

  Phase 2a's primary subject moves the slug list to the commit body (per round-01 R-004). The body line is: `Rewrites csr, klib, mm, memOpt, err2trap from running-notes to the seven-section Ark template; preserves each pre-workflow source at .ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md.` An 86-char alternative (`docs(specs): rewrite running-notes specs (csr klib mm memOpt err2trap) to ark template`) exists for users who prefer the slug enumeration visible in `git log --oneline` on wide terminals, but it is **non-conforming with R4's ~72-char soft cap** and is documented here only as a fallback, not the default. Conformance wins; archaeology readers can `git log -1 --format=%B` to see the body.

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
- V-F-5: For each Bucket-B slug, `.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md` exists and is byte-identical to the pre-migration `docs/spec/<slug>/SPEC.md` (verified *before* Phase 5; `sha256sum` recorded in Phase 2a commit body for post-Phase-5 verification). Per-slug commands:
  - `diff -q docs/spec/csr/SPEC.md .ark/tasks/archive/legacy/csr/SPEC_LEGACY.md`
  - `diff -q docs/spec/klib/SPEC.md .ark/tasks/archive/legacy/klib/SPEC_LEGACY.md`
  - `diff -q docs/spec/mm/SPEC.md .ark/tasks/archive/legacy/mm/SPEC_LEGACY.md`
  - `diff -q docs/spec/memOpt/SPEC.md .ark/tasks/archive/legacy/memOpt/SPEC_LEGACY.md`
  - `diff -q docs/spec/err2trap/SPEC.md .ark/tasks/archive/legacy/err2trap/SPEC_LEGACY.md`

  Every Bucket-B slug satisfies the canonical `**/SPEC_LEGACY.md` glob — `memOpt` is no longer the exception. (Maps round-00 R-001 acceptance + round-01 R-003 acceptance.)
- V-F-6: Rollback-target reachability — `git cat-file -e $BASELINE_HEAD || echo "BASELINE_HEAD unreachable"` confirms the rollback target is reachable; `git log --oneline $BASELINE_HEAD..HEAD | wc -l` reports the number of phase commits so far (expected: 7 after Phase 6, 6 after Phase 5, 5 after Phase 4, ...). No throwaway worktree clone needed. (Maps R-002 round-00 acceptance, closes round-01 R-006.)

[**Edge Cases**]

- V-E-1: `docs/archived/review/MANUAL_REVIEW.md` is a file, not a directory. The `git mv` step places it as a flat file at `.ark/tasks/archive/legacy/MANUAL_REVIEW.md` (not `.ark/tasks/archive/legacy/MANUAL_REVIEW/`). Verified by `test -f .ark/tasks/archive/legacy/MANUAL_REVIEW.md && ! test -d .ark/tasks/archive/legacy/MANUAL_REVIEW`.
- V-E-2: Slug spelling differs between `docs/spec/` and `docs/archived/` in no observed case. If discovered during migration, the spelling in `docs/spec/` wins (the SPEC is canonical); rename the archive subdir to match before the `git mv` in Phase 1.
- V-E-3: Pre-workflow features with no `NN_` iteration files (`csr`, `klib`, `mm`, `err2trap`) have irregular archive shape (`PLAN.md`, `PLAN_REVIEW.md`, `IMPL_REVIEW.md`, `MEM_OPTIMIZATION_PLAN.md`, `ERR2TRAP.md`, etc.). Phase 1 copies whatever is present verbatim under the slug; Phase 2a synthesises Goals/Constraints from the running notes and preserves the running-notes SPEC as `SPEC_LEGACY.md`.
- V-E-4: `docs/spec/inst/SPEC.md` has no archive counterpart. Phase 1 does not create `.ark/tasks/archive/legacy/inst/`. Phase 2b still produces `.ark/specs/features/inst/SPEC.md` from the running notes; the migrated SPEC's CHANGELOG opens with a clean assertion (no archive bucket exists; the pre-workflow source had no iteration history). Phase 3's INDEX row for `inst` is still included.
- V-E-5: `memOpt`'s iteration source (`MEM_OPTIMIZATION_PLAN.md`) lives at `.ark/tasks/archive/legacy/mm/MEM_OPTIMIZATION_PLAN.md` (a section of mm's pre-workflow running-notes). Phase 2a creates a dedicated `.ark/tasks/archive/legacy/memOpt/` dir containing `SPEC_LEGACY.md` (byte-identical copy of `docs/spec/memOpt/SPEC.md`) plus a `README.md` whose single line points at the iteration history: `Iteration history for the memOpt SPEC lives at ../mm/MEM_OPTIMIZATION_PLAN.md (the original was a section in the mm running-notes).` The `memOpt` migrated SPEC's `CHANGELOG` references `.ark/tasks/archive/legacy/memOpt/SPEC_LEGACY.md` directly — the canonical per-slug filename, satisfying any `**/SPEC_LEGACY.md` glob. (Maps round-01 R-003.)
- V-E-6: `docs/archived/feat/cicd/CICD.md` is a single doc, not iteration-history. Phase 1 places it at `.ark/tasks/archive/legacy/cicd/CICD.md`; Phase 2b's `cicd` SPEC references it as the source.
- V-E-7: Two known `.DS_Store` files exist — `docs/.DS_Store` and `docs/archived/feat/csr/.DS_Store`. Both are macOS cruft; do not migrate. The `find docs -name .DS_Store -delete` step before Phase 1 covers both.
- V-E-8: The `<!-- ARK:FEATURES:START --> ... <!-- ARK:FEATURES:END -->` markers inside `.ark/specs/features/INDEX.md` must survive Phase 3 verbatim — they are how the Ark CLI locates the table on future `ark agent spec register` calls. Edit only the content *between* the markers; the markers themselves stay byte-identical.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-F-1 (no `docs/(tasks|spec|archived|template)` references remain). |
| G-2 | V-F-2 (every slug has a template-shaped SPEC under `.ark/specs/features/`). |
| G-3 | V-F-2 (INDEX row presence is checked alongside SPEC presence). |
| G-4 | V-F-3 + V-F-4 (history preserved via `git mv`; byte-identical archive content). |
| G-5 | V-IT-1 (gates pass) + manual diff of slimmed `AGENTS.md` + `docs/PROGRESS.md` link inspection (no broken links after Phase 5; `rg` zero-hit confirms link substitutions landed, including Rule 5 for `docs/`-prefixed link text). |
| C-1 | V-F-2 (seven section headings in order). |
| C-2 | V-F-2 (INDEX rows ↔ SPEC files). |
| C-3 | V-F-1 (post-Phase-5 `rg` zero-hit). |
| C-4 | V-IT-1 + V-IT-2 (full gate run). |
| C-5 | V-F-4 (byte-identical archive diff). |
| C-6 | Manual `diff <(grep -A20 "## Development Standards" AGENTS.md) <prev>` to confirm Standards block unchanged; `grep -F "ARK:START" AGENTS.md` non-obvious. |
| C-7 | V-F-3 (`git log --follow` reaches pre-migration commits). |
| C-8 | Manual inspection of `docs/PROGRESS.md` diff — phase tables, baseline links, roadmap text unchanged outside of cross-link substrings. |
| R-001 (round-00) | V-F-5 (Bucket-B `SPEC_LEGACY.md` byte-identical to original, canonical filename for every slug). |
| R-002 (round-00) | V-F-6 (rollback target reachable via `git cat-file -e`). |
| R-001 (round-01) | Worked example for `csr` in Phase 2a is rebuilt against the verified on-disk layout (`cpu/csr.rs` + `cpu/csr/` + `cpu/trap.rs` + `cpu/trap/`); spot-check rule extended to the other four Bucket-B slugs. |
| R-002 (round-01) | Phase 3.3 Rule 5 + V-F-1 zero-hit catches any surviving `docs/`-prefixed link text at `docs/PROGRESS.md:23` / `:190`. |
| R-003 (round-01) | V-E-5 + V-F-5 per-slug commands now cover `memOpt/SPEC_LEGACY.md` directly. |
| R-004 (round-01) | T-5 table primary subjects all ≤72 chars; Phase 2a slug list moved to body. |
| R-005 (round-01) | Phase 2b's `inst` CHANGELOG is a clean assertion — no self-negating clause. |
| R-006 (round-01) | V-F-6 replaced with `git cat-file -e` + `git log --oneline` checks; no throwaway worktree. |