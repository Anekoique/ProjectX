# `scripts/perf/` — xemu performance toolbox

Three small composable tools plus a shared config. Every tool writes
into one run directory (default `docs/perf/<today>/`); re-running any
step in isolation is safe.

```
scripts/perf/
├── README.md         # this file
├── config.sh         # PROJECT_ROOT, XEMU_BIN, WORKLOADS, DEFAULT_OUT
├── bench.sh          # wall-clock + peak-RSS bench  → data/bench.csv
├── sample.sh         # Apple `sample` profile       → data/<wl>.sample.txt
├── render.py         # every SVG chart              → graphics/*.svg
└── lib/
    ├── _common.sh    # log/warn/die/parse_out/parse_time_l
    └── _demangle.py  # Rust v0 mangled-name cleaner (imported by render.py)
```

## bench.sh

```
bash scripts/perf/bench.sh [--out DIR] [--runs N]
```

- Runs every `$WORKLOADS` entry through its own `make run`, N times.
- Writes:
  - `DIR/data/bench.csv` — columns `workload,run,real_s,user_s,sys_s,max_rss_kb`
  - `DIR/data/bench.summary` — per-run one-liner log
  - `DIR/data/<workload>.log` — merged stdout of every run
  - `DIR/data/<workload>.run<N>.time` — raw `/usr/bin/time -l` output

Defaults: `--out docs/perf/<today>`, `--runs 3`.

## sample.sh  (macOS only)

```
bash scripts/perf/sample.sh [workload ...] [--out DIR]
```

- For each workload (or all if none given), launches `make run` in the
  background, attaches `/usr/bin/sample` to the `xdb` child PID for
  the workload's `sample-seconds` window (third column in `WORKLOADS`).
- Writes `DIR/data/<workload>.sample.txt` — Apple call-tree + "Sort by
  top of stack" table.
- On Linux, swap in `samply record -- make run` — same three steps,
  same output layout, different file format (Gecko JSON).

## render.py

```
python3 scripts/perf/render.py [--dir DIR]
```

- Stdlib only (no `matplotlib`, no `pandas`).
- Reads `DIR/data/`, writes `DIR/graphics/`.
- One pass produces:
  - `bench_time.svg`, `bench_rss.svg`
  - `hotspot_<workload>.svg` — bucketed pie
  - `selftime_<workload>.svg` — ranked-leaf bar (widths ∝ self-time
    samples). **This is not a flamegraph**: it has no call-stack depth.
    Use `samply record` when you need a true flamegraph.
- Rust v0 symbols are demangled via `lib/_demangle.py` to readable
  `crate::module::leaf` form.

Default `--dir` is the newest dated subdir under `docs/perf/`.

## Adding a workload

Edit `scripts/perf/config.sh`:

```bash
WORKLOADS=(
  "dhrystone  $BENCH_ROOT/dhrystone   6"
  "coremark   $BENCH_ROOT/coremark   12"
  "microbench $BENCH_ROOT/microbench 15"
  "myworkload $BENCH_ROOT/myworkload 10"   # ← new row
)
```

Each row is three fields separated by whitespace:
`<name>  <absolute-dir-with-a-Makefile>  <sample-seconds>`. The bench
harness ignores the third field; the sample harness uses it as the
attach-window duration.

## Design notes

- Every script is a *thin wrapper* around `make run`. The project
  Makefiles remain the single source of truth for how xemu is built
  and launched; this directory never invokes `xdb` directly.
- Output directories are **dated** (`YYYY-MM-DD`), so runs accumulate
  instead of overwriting. Same-day reruns can go to
  `docs/perf/<date>-<label>/` by passing `--out` / `--dir` to override.
- No third-party Python dependencies. The SVGs are hand-built so they
  render anywhere and produce a small, readable diff on commit.
