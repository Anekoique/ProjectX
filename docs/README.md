# `docs/` — ProjectX documentation

## Layout

| Path | Contents |
|------|----------|
| [`PROGRESS.md`](./PROGRESS.md) | Development plan, phase status, roadmap (incl. Phase 9 perf). |
| [`book/`](./book/) | User manual and internals reference — start [here](./book/introduction.md). |
| [`spec/<feature>/SPEC.md`](./spec/) | Landed specifications per feature. |
| [`tasks/<feature>/`](./tasks/) | In-flight features — PLAN ↔ REVIEW ↔ MASTER rounds in progress. |
| [`archived/<category>/<feature>/`](./archived/) | Landed-feature iteration history, grouped by category: `feat` / `fix` / `refactor` / `perf` / `review`. |
| [`perf/baselines/<date>/`](./perf/baselines/) | Dated measurement baselines (bench.csv, sample traces, graphics). |
| [`template/`](./template/) | Templates for new iteration documents. |

## Workflow

See [`/AGENTS.md`](../AGENTS.md) for the iteration lifecycle (PLAN →
REVIEW → MASTER → IMPL), the 5-round loop cap, and authorship rules.
See [`tasks/README.md`](./tasks/README.md) for the active-feature
lifecycle and archive-category heuristics.

## Archive categories

- **`feat/`** — new capability (e.g. `boot`, `devices`, `multiHart`, `cicd`)
- **`fix/`** — bug or MANUAL_REVIEW finding that isn't a reorg (e.g. `directIrq`, `plicGateway`)
- **`refactor/`** — reshaping without new capability (e.g. `archModule`, `aclintSplit`, `err2trap`)
- **`perf/`** — measurable speedup under an exit gate (e.g. `perfBusFastPath`, `perfHotPath`)
- **`review/`** — audits and retrospectives not tied to a single feature (e.g. `MANUAL_REVIEW.md`)

## Features by area

- **CPU / ISA:** [`csr`](./spec/csr/), [`float`](./spec/float/),
  [`inst`](./spec/inst/)
- **Memory:** [`mm`](./spec/mm/), [`memOpt`](./spec/memOpt/)
- **Devices:** [`devices`](./spec/devices/),
  [`aclintSplit`](./spec/aclintSplit/),
  [`plicGateway`](./spec/plicGateway/), [`keyboard`](./spec/keyboard/)
- **Interrupts:** [`directIrq`](./spec/directIrq/),
  [`multiHart`](./spec/multiHart/)
- **Traps / errors:** [`err2trap`](./spec/err2trap/)
- **Boot / OS:** [`boot`](./spec/boot/), [`debian`](./spec/debian/)
- **Debugger:** [`trace`](./spec/trace/), [`difftest`](./spec/difftest/)
- **Architecture:** [`archModule`](./spec/archModule/),
  [`archLayout`](./spec/archLayout/)
- **Library:** [`klib`](./spec/klib/)
- **Performance:** [`perfBusFastPath`](./spec/perfBusFastPath/),
  [`perfHotPath`](./spec/perfHotPath/)
- **Testing:** [`amTests`](./spec/amTests/),
  [`benchmark`](./spec/benchmark/)
- **Infra:** [`cicd`](./spec/cicd/)
