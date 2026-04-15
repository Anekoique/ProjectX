# xemu Performance Runs

This directory holds two kinds of content:

- **Dated-run subdirectories** (`YYYY-MM-DD/`) — one complete profile
  captured on a single day: `REPORT.md` + raw `data/` + rendered
  `graphics/`.
- **Iteration-task subdirectories** (camelCase `<tag>/`) — RLCR
  iteration artefacts (PLAN / REVIEW / MASTER / IMPL) for the
  performance phases defined in [`../PERF_DEV.md`](../PERF_DEV.md).

```
docs/perf/
├── README.md                  # this file
├── 2026-04-14/                # pre-P1 baseline
│   ├── REPORT.md
│   ├── data/                  # bench.csv, sample text, per-run time files
│   └── graphics/              # SVG charts re-generable from data/
├── 2026-04-15/                # post-P1 baseline (busFastPath landed)
├── 2026-04-16/                # post-hotPath (P3+P4+P5+P6 landed)
├── busFastPath/               # Phase P1 iteration artefacts
│   ├── 00_PLAN.md … 03_PLAN.md
│   ├── 00_REVIEW.md … 03_REVIEW.md
│   ├── 00_MASTER.md … 03_MASTER.md
│   └── 00_IMPL.md
└── hotPath/                   # Phases P3+P4+P5+P6 bundle iteration
    ├── 00_PLAN.md … 04_PLAN.md
    ├── 00_REVIEW.md … 04_REVIEW.md
    ├── 00_MASTER.md … 04_MASTER.md
    └── 00_IMPL.md
```

## Timeline

| Date | What landed | Run directory |
|------|-------------|---------------|
| 2026-04-14 | Pre-P1 baseline capture | [`2026-04-14/`](./2026-04-14/) |
| 2026-04-15 | **P1** — single-hart bus fast path ([`busFastPath/`](./busFastPath/)) | [`2026-04-15/`](./2026-04-15/) |
| 2026-04-16 | **hotPath** — P3 Mtimer + P4 icache + P5 MMU inline + P6 memmove ([`hotPath/`](./hotPath/)) | [`2026-04-16/`](./2026-04-16/) (REPORT pending) |

Cumulative user-time vs pre-P1: dhrystone **−57 %**, coremark
**−58 %**, microbench **−62 %**.

## Quickstart — capture a new run

All three steps read `scripts/perf/config.sh` for workload paths and
the default output dir (`docs/perf/<today>`).

```bash
# 1. Wall-clock + peak-RSS, 3 iters per workload.
bash scripts/perf/bench.sh

# 2. CPU sampling profile (macOS; uses /usr/bin/sample).
bash scripts/perf/sample.sh

# 3. Render every SVG (bars, hotspot pies, flame-style bars).
python3 scripts/perf/render.py

# 4. Write your REPORT.md in docs/perf/<today>/ summarising the findings.
```

Override the output directory with `--out` / `--dir`:

```bash
bash scripts/perf/bench.sh  --out docs/perf/2026-05-01
bash scripts/perf/sample.sh --out docs/perf/2026-05-01
python3 scripts/perf/render.py --dir docs/perf/2026-05-01
```

Sample only one workload:

```bash
bash scripts/perf/sample.sh coremark
```

## Comparing runs

`data/bench.csv` is the canonical wall-clock record for each run.

```bash
# Quick side-by-side of two runs:
diff -u docs/perf/2026-04-14/data/bench.csv docs/perf/2026-05-01/data/bench.csv

# Or collect every run into one table:
for d in docs/perf/20*; do
  awk -v run=${d##*/} 'NR>1 {print run","$0}' "$d/data/bench.csv"
done
```

`graphics/*.svg` embed in any Markdown or browser; they are deliberately
dependency-free (hand-written SVG) so they survive tooling churn.

## Dependencies

- **Required:** `make`, `python3` (stdlib only), a recent `bash`/`zsh`,
  a working xemu toolchain (see root `README.md`).
- **macOS, for `sample.sh`:** `/usr/bin/sample` (ships with the OS — no
  install step).
- **Optional:** `samply` 0.13+ for a browser-based flamegraph UI
  (`samply record -- make run`). Works without codesign on Linux; on
  macOS run `samply setup` once. The Apple `sample` output already
  covers the same information and is included by default.

## Conventions

- Directory name per run: `YYYY-MM-DD` (ISO-8601 date, sorts naturally).
- Multiple runs on the same day: `YYYY-MM-DD-<label>` (e.g.
  `2026-05-01-after-icache`). The `render.py --dir` flag still works.
- The **baseline** run is whichever one `docs/PERF_DEV.md` currently
  points to. Update that link whenever a new baseline is landed.

## Also see

- [`../PERF_DEV.md`](../PERF_DEV.md) — performance development roadmap
  (priorities, phases, exit gates).
- [`../DEV.md`](../DEV.md) — top-level project roadmap.
- [`../../scripts/perf/README.md`](../../scripts/perf/README.md) — script
  reference (args, output layout, extending the workload list).
