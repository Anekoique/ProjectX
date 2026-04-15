# Adding a benchmark

Existing benchmarks: Dhrystone, CoreMark, MicroBench. Adding a new
one means a new test kernel + a measurement entry in the perf
pipeline.

## Kernel

Create a new directory under `xkernels/tests/benchmark/<name>/`:

```
xkernels/tests/benchmark/<name>/
├── Makefile          # uses xam/scripts/build_c.mk or build_rs.mk
├── src/
│   └── main.c        # or .rs — the benchmark itself
└── README.md         # what it measures, expected score range
```

The Makefile should delegate to xam's build system. Link against
xlib for `printf` / `memcpy`.

Exit via the SiFive test finisher:

```c
#include <klib.h>
extern void xam_halt(int code);

int main() {
    uint64_t t0 = uptime();
    /* ... work ... */
    uint64_t t1 = uptime();
    printf("score = %lu\n", compute_score(t1 - t0));
    xam_halt(0);
    return 0;
}
```

## Measurement pipeline

Add the new benchmark to `scripts/perf/bench.sh` so CI / manual
runs capture it:

```bash
BENCHES=(dhrystone coremark microbench <name>)
```

Also teach `scripts/perf/sample.sh` how to capture its sample
profile (usually the same per-workload path).

## Baseline

After landing:

1. Run `bash scripts/perf/bench.sh --runs 3`. This writes the new
   workload into `docs/perf/baselines/<today>/data/bench.csv`.
2. Run `bash scripts/perf/sample.sh` to produce the sample traces.
3. Run `python3 scripts/perf/render.py` for the SVG flamegraphs.
4. Commit the new `data/` + `graphics/` files.

## Reporting in PROGRESS.md

Add a row to the "Benchmark" table in the project root `README.md`
if the benchmark is user-facing enough to publish. Update
[`../PROGRESS.md`](../../PROGRESS.md) §Phase 9 if the workload
introduces a new cost centre worth tracking per-phase.

## Performance hygiene

- Don't add a benchmark that depends on wall-clock non-determinism
  (interrupt timing, stdin blocking). Use deterministic work loops.
- Use `uptime()` (microseconds) for in-guest timing; it's derived
  from ACLINT mtime, which is frozen during xdb pause.
- Take 3 runs for the published number; `user_s` is the stable
  metric.
