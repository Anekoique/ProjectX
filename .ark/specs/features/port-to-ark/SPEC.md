
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
