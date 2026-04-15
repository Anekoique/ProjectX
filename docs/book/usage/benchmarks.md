# Benchmarks

xemu ships three benchmark kernels under `xkernels/tests/benchmark/`.

| Benchmark | Iterations | Characteristic |
|-----------|-----------:|----------------|
| Dhrystone | 500 000 | ALU / GPR / call-heavy |
| CoreMark | 1 000 | Mixed integer + list / matrix |
| MicroBench | 10 sub-benches | Includes C++ workloads (`qsort-cpp`, `string`) |

## Running

```bash
cd xkernels/tests/benchmark/dhrystone   && make run
cd xkernels/tests/benchmark/coremark    && make run
cd xkernels/tests/benchmark/microbench  && make run
```

Always run with `DEBUG=n` for stable timing.

## Published scores (MacBook Air M4)

| Benchmark | Marks |
|-----------|------:|
| MicroBench | 718 |
| CoreMark | 499 |
| Dhrystone | 255 |

## Perf pipeline

To regenerate the measurement baseline:

```bash
bash scripts/perf/bench.sh   # writes docs/perf/baselines/<today>/data/bench.csv
bash scripts/perf/sample.sh  # writes <today>/data/<workload>.sample.txt
python3 scripts/perf/render.py   # writes <today>/graphics/*.svg
```

Run from `ProjectX/` root. See
[`../internals/performance.md`](../internals/performance.md) for how
buckets are interpreted and
[`../PROGRESS.md`](../../PROGRESS.md) §Phase 9 for landed optimisations.

## Reproducing the published numbers

- Use `make run` (not `target/release/xdb` directly). The Makefile
  sets the right boot layout.
- Leave `DEBUG` unset (defaults to `n`).
- Close other CPU-heavy processes. macOS `samply` especially is
  sensitive to background load; `user_s` is the stable metric,
  `real_s` is noisy.
- Take the **mean of 3 runs** for any comparison.
