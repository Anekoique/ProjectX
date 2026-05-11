# `port-to-ark` PLAN `00`

> Status: Draft
> Feature: `port-to-ark`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`

---

## Summary

Retire the bespoke `docs/`-based PLAN/REVIEW/MASTER workflow and consolidate every task, archive, and feature-spec primitive under `.ark/`. The migration ships in five sequential, independently inspectable phases plus a verification gate: (1) move the ~140 archived iteration files verbatim under `.ark/tasks/archive/legacy/<slug>/` via `git mv`, collapsing the `feat/`/`fix/`/`refactor/`/`perf/` subcategories; (2) rewrite all 24 historical `docs/spec/<slug>/SPEC.md` files to the Ark seven-section template at `.ark/specs/features/<slug>/SPEC.md`; (3) populate `.ark/specs/features/INDEX.md` between the `ARK:FEATURES` markers, slim `AGENTS.md` to standards + Ark pointer, and rewrite every `docs/PROGRESS.md` cross-link; (4) substitute the handful of `docs/archived/...` paths cited inside Rust doc-comments under `xemu/` with their new `.ark/tasks/archive/legacy/...` destinations; (5) `git rm -r docs/{tasks,spec,archived,template}` and (6) re-run `make fmt && make clippy && make run && make test` to confirm no regressions. The four buckets — archive relocation, SPEC rewrites, INDEX/AGENTS/PROGRESS edits, source-code path rewrites — are sequenced so the user can pause and inspect after each.

## Log `None in 00_PLAN`

None in 00_PLAN

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
│   ├── archived/                                (-) deleted (~140 files)
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
│               ├── csr/                         (+) from feat/  (pre-workflow)
│               ├── debian/                      (+) from feat/
│               ├── devices/                     (+) from feat/
│               ├── difftest/                    (+) from feat/
│               ├── directIrq/                   (+) from fix/
│               ├── err2trap/                    (+) from refactor/  (pre-workflow)
│               ├── float/                       (+) from feat/
│               ├── keyboard/                    (+) from feat/
│               ├── klib/                        (+) from feat/  (pre-workflow)
│               ├── mm/                          (+) from feat/  (pre-workflow; carries memOpt source)
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

1. Confirm baseline. Run `make fmt && make clippy && make run && make test` on the current `feat/port-to-ark` worktree; record exit codes. This is the C-4 reference.
2. Phase 1 — archive relocation. `git mv` each `docs/archived/<cat>/<slug>/` → `.ark/tasks/archive/legacy/<slug>/`; commit boundary optional, but the executor MUST inspect a `git status --short` summary before continuing.
3. Phase 2 — author 24 SPECs at `.ark/specs/features/<slug>/SPEC.md` from the legacy sources (final archived PLAN's `## Spec` block for iteration-history features; running-notes prose for pre-workflow features).
4. Phase 3 — populate `.ark/specs/features/INDEX.md` between the `ARK:FEATURES:START`/`ARK:FEATURES:END` markers (one row per slug); rewrite `AGENTS.md` to slim shape; rewrite `docs/PROGRESS.md` cross-link substrings.
5. Phase 4 — `rg "docs/archived" xemu/ xam/` lists doc-comment cites; substitute each with the migrated path. Phase 4 expects zero `.rs` AST changes; only string bodies inside `///`/`//!` differ.
6. Phase 5 — `git rm -r docs/tasks docs/spec docs/archived docs/template`; re-run `rg "docs/(tasks|spec|archived|template)" /Users/anekoique/ProjectX` and expect zero hits outside `.ark/tasks/port-to-ark/`.
7. Phase 6 — re-run `make fmt && make clippy && make run && make test`; compare to step 1.

[**Failure Flow**]

1. Rust gates fail after Phase 4. Doc-comment edits should never break compilation; if they do, an `#[doc = "..."]` macro arg, a `cfg`-conditional include, or a `compile_error!` cite was missed. Roll back the offending file with `git checkout HEAD -- <path>` and re-edit by hand.
2. Post-Phase-5 `rg` returns a hit. Re-open the matching file (most likely a stray cross-reference inside an SPEC just authored) and replace the cite before declaring Phase 5 complete.
3. INDEX row missing a SPEC. Either the SPEC was not authored (return to Phase 2) or the INDEX row was hand-typed wrong (correct the slug spelling).
4. `git mv` fails because target directory already exists. Inspect `.ark/tasks/archive/legacy/<slug>/` — if a previous phase partially completed, complete the move with `git mv --force` after confirming no data loss.

[**State Transitions**]

- `docs/` (full tree) → `docs/` pruned to `book/` + `perf/` + `PROGRESS.md` + `README.md` (Phase 5 boundary).
- `.ark/specs/features/` empty → 24 SPECs + populated INDEX (Phase 2 + Phase 3).
- `.ark/tasks/archive/` empty → contains `legacy/<slug>/` subtrees (Phase 1).
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

Post-Phase-1 inspection: `git status --short | wc -l` should show roughly 270+ rename entries. `ls .ark/tasks/archive/legacy/` should list 22 subdirs plus `MANUAL_REVIEW.md`.

[**Phase 2 — SPEC rewrites**]

For each of the 24 features under `docs/spec/<slug>/`, author `.ark/specs/features/<slug>/SPEC.md` in the Ark template shape (`Goals`/`Non-goals`/`Architecture`/`Data Structure`/`API Surface`/`Constraints`/`CHANGELOG`). Source material splits into three buckets:

**Bucket A — iteration-history features (17 SPECs).** Slugs: `aclintSplit`, `amTests`, `archLayout`, `archModule`, `benchmark`, `boot`, `debian`, `devices`, `difftest`, `directIrq`, `float`, `keyboard`, `multiHart`, `perfBusFastPath`, `perfHotPath`, `plicGateway`, `trace`. The current `docs/spec/<slug>/SPEC.md` has structured `Goals`/`Non-Goals`/`Architecture`/`Invariants`/`Data Structure`/`API Surface` blocks extracted from a final archived PLAN. Migration mapping: `Goals` → `Goals` (verbatim), `Non-Goals` → `Non-goals` (rename only), `Architecture` → `Architecture` (verbatim), `Data Structure` → `Data Structure` (verbatim), `API Surface` → `API Surface` (verbatim), `Invariants` → `Constraints` (rename; one declarative sentence per bullet — collapse multi-sentence elaborations to their first sentence per Ark template guidance), append empty `[**CHANGELOG**]` section. Drop the `> Source:` banner. The migrated SPEC's `CHANGELOG` starts empty.

**Bucket B — pre-workflow running-notes features (5 SPECs).** Slugs: `csr`, `klib`, `mm`, `memOpt`, `err2trap`. The current `docs/spec/<slug>/SPEC.md` is the verbatim pre-workflow design doc under a `Source:` banner — long-form prose, headings like `## Architecture Overview`, `## Design Decisions`, `## Phase 1: …`. Collapse to template shape: synthesize ≤5 `Goals` from the doc's stated motivation / "what works" sections; lift `Non-goals` verbatim if a `## Non-Goals` block exists; place the high-level diagram / directory tree under `Architecture` (trim to fit); lift any public types into `Data Structure`; lift any public function signatures into `API Surface`; synthesize 5–10 declarative sentences for `Constraints` from "Key layering principle", "Design Principles", and similar bullets; append empty `CHANGELOG`. Preserve provenance with a single top blockquote: `> Migrated 2026-05-11 from pre-workflow notes at `.ark/tasks/archive/legacy/<slug>/<orig-filename>` — see CHANGELOG for re-iteration history.`

**Bucket C — pre-workflow features with no archive (2 SPECs).** Slugs: `inst`, `cicd`. `inst` is a running-notes file of RISC-V instruction encoding tables — no archive counterpart. Collapse to template shape: `Goals` describes ISA coverage, `Architecture` holds the encoding tables, `API Surface` lists dispatch entry points, `Constraints` carries the `cfg(isa32)/cfg(isa64)` rule and similar. No `Source:` banner. `cicd` is the GitHub Actions workflow definition; its archive companion is `cicd/CICD.md` (single doc, not iteration-history). Bucket A treatment but `CHANGELOG` source references `.ark/tasks/archive/legacy/cicd/CICD.md` rather than a numbered PLAN.

This phase produces 24 separate `SPEC.md` files. Allow time — this is the single largest block of the migration.

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

- `docs/spec/<feature>/SPEC.md` → `.ark/specs/features/<feature>/SPEC.md` (also markdown form `[`spec/<feature>/SPEC.md`](./spec/<feature>/SPEC.md)` → `[`features/<feature>/SPEC.md`](../.ark/specs/features/<feature>/SPEC.md)`).
- `docs/archived/<category>/<feature>/` → `.ark/tasks/archive/legacy/<feature>/` (also markdown form `[`archived/<cat>/<feature>/`](./archived/<cat>/<feature>/)` → `[`legacy/<feature>/`](../.ark/tasks/archive/legacy/<feature>/)`).
- The `MANUAL_REVIEW.md` reference: `./archived/review/MANUAL_REVIEW.md` → `../.ark/tasks/archive/legacy/MANUAL_REVIEW.md`.
- The closing-paragraph references to `docs/tasks/<feature>/`, `docs/spec/<feature>/SPEC.md`, `docs/archived/<category>/<feature>/`, and `[`docs/tasks/README.md`](./tasks/README.md)`: rewrite the whole paragraph to "Subsequent work uses Ark — `/ark:design --deep "<title>"` opens the task, `ark agent task commit` lands the spec under `.ark/specs/features/<feature>/SPEC.md`, and `ark archive` moves iteration history under `.ark/tasks/archive/YYYY-MM/<feature>/`. Read `.ark/workflow.md` for the lifecycle."
- The `[`/AGENTS.md`](../AGENTS.md)` link stays — AGENTS.md still exists, just slimmed.
- Phase tables, baseline links, roadmap commentary, design-principles section: unchanged.

[**Phase 4 — Source-code doc-comment rewrites**]

Run `rg "docs/archived" /Users/anekoique/ProjectX/xemu /Users/anekoique/ProjectX/xam`. The known hits (from a pre-migration scan):

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

[**Phase 6 — Verification gates**]

```bash
make fmt
make clippy
make run
make test
```

All four must exit zero with output equivalent to the pre-migration baseline captured in Runtime step 1. `make run` and `make test` are particularly important: they exercise the doc-comments inside `cpu.rs` / `cpu/mod.rs` / `icache.rs` indirectly via compilation, catching any malformed `///` substitution. If `make linux` / `make debian` are normally part of the CI smoke set, run them too (PRD #8: "Build/test gates pass unchanged").

---

## Trade-offs

- T-1: Rewrite all 24 SPECs to template shape **vs** lazy migration (rewrite on next iteration per feature). Adv. of full rewrite: uniform structure for Ark's VERIFY spec-compliance pass; INDEX, agents, and contributors all see one shape. Disadv.: high one-shot effort; the five pre-workflow SPECs (`csr`, `klib`, `mm`, `memOpt`, `err2trap`) lose narrative density when collapsed to seven sections — long prose discussions of design rationale do not fit `Goals`/`Constraints`. **Choice: full rewrite**, per PRD Outcome #2. The narrative density loss is mitigated by the legacy iteration files surviving verbatim under `.ark/tasks/archive/legacy/<slug>/`.
- T-2: Flat `archive/legacy/<slug>/` **vs** preserve `feat/`/`fix/`/`refactor/`/`perf/` subcategories. Ark's native archive layout is `archive/YYYY-MM/<slug>/` (flat per month). The legacy bucket mirrors that flatness so future contributors learn one structure, not two. Disadv.: loses the at-a-glance category browsing of `ls docs/archived/feat`. **Choice: flat**. Category metadata survives in (a) the legacy commit history via `git log --follow`, (b) the `docs/PROGRESS.md` "Manual Review TODOs" table, and (c) each SPEC's intrinsic shape.
- T-3: `git mv` (preserves history) **vs** copy + delete (cleaner diff, simpler `git status`). **Choice: `git mv`**. The `--follow` capability is worth the cluttered status; future archaeology on `directIrq/02_PLAN.md` should resolve to the original 2026-03-?? commit, not a 2026-05-11 "move" commit.
- T-4: Populate INDEX `Promoted` column with the migration date `2026-05-11` for all rows **vs** archaeological per-feature date via `git log -1 --format=%cd docs/spec/<slug>/SPEC.md`. **Choice: bulk migration date**. The INDEX `Promoted` column's semantics under Ark are "last touch by a deep commit" — the migration *is* the most recent touch. Ark will re-stamp on the next per-feature iteration.
- T-5: One mega-commit covering Phases 1–6 **vs** one commit per phase. **Choice: per-phase commits** (the executor should `git add` and `git commit` between phases so the user can inspect each step). Six commits land on the worktree's branch; the final PR merges as the user prefers.

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

[**Edge Cases**]

- V-E-1: `docs/archived/review/MANUAL_REVIEW.md` is a file, not a directory. The `git mv` step places it as a flat file at `.ark/tasks/archive/legacy/MANUAL_REVIEW.md` (not `.ark/tasks/archive/legacy/MANUAL_REVIEW/`). Verified by `test -f .ark/tasks/archive/legacy/MANUAL_REVIEW.md && ! test -d .ark/tasks/archive/legacy/MANUAL_REVIEW`.
- V-E-2: Slug spelling differs between `docs/spec/` and `docs/archived/` in no observed case. If discovered during migration, the spelling in `docs/spec/` wins (the SPEC is canonical); rename the archive subdir to match before the `git mv` in Phase 1.
- V-E-3: Pre-workflow features with no `NN_` iteration files (`csr`, `klib`, `mm`, `err2trap`) have irregular archive shape (`PLAN.md`, `PLAN_REVIEW.md`, `IMPL_REVIEW.md`, `MEM_OPTIMIZATION_PLAN.md`, `ERR2TRAP.md`, etc.). Phase 1 copies whatever is present verbatim under the slug; their SPEC rewrite (Bucket B in Phase 2) synthesizes `Goals` and `Constraints` from the running notes.
- V-E-4: `docs/spec/inst/SPEC.md` has no archive counterpart. Phase 1 does not create `.ark/tasks/archive/legacy/inst/`. Phase 2 still produces `.ark/specs/features/inst/SPEC.md` from the running notes; the migrated SPEC's CHANGELOG lacks a `Source:` archive pointer. Phase 3's INDEX row for `inst` is still included.
- V-E-5: `memOpt` shares an archive bucket with `mm` (the `MEM_OPTIMIZATION_PLAN.md` lives at `.ark/tasks/archive/legacy/mm/MEM_OPTIMIZATION_PLAN.md`). The `memOpt` SPEC's `CHANGELOG` source pointer references that path explicitly. Phase 1 does not create `.ark/tasks/archive/legacy/memOpt/`.
- V-E-6: `docs/archived/feat/cicd/CICD.md` is a single doc, not iteration-history. Phase 1 places it at `.ark/tasks/archive/legacy/cicd/CICD.md`; Phase 2's `cicd` SPEC references it as the source.
- V-E-7: `.DS_Store` files at `docs/archived/feat/csr/.DS_Store` (and possibly elsewhere) are macOS cruft; do not migrate. Run `find docs -name .DS_Store -delete` before Phase 1.
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
| C-6 | Manual `diff <(grep -A20 "## Development Standards" AGENTS.md) <prev>` to confirm Standards block unchanged; `grep -F "ARK:START" AGENTS.md` non-empty. |
| C-7 | V-F-3 (`git log --follow` reaches pre-migration commits). |
| C-8 | Manual inspection of `docs/PROGRESS.md` diff — phase tables, baseline links, roadmap text unchanged outside of cross-link substrings. |
