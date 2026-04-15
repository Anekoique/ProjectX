# xemu Performance Runs

Each subdirectory is one complete profile captured on a single day:

```
docs/perf/
├── README.md                  # this file
├── 2026-04-14/
│   ├── REPORT.md              # human-readable writeup for the run
│   ├── data/                  # raw: bench.csv, sample text, per-run time files
│   └── graphics/              # SVG charts re-generable from data/
└── <YYYY-MM-DD>/...
```

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
