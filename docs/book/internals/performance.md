# Performance: hot path & baselines

## Short answer

Over five phases (P1 + P3 + P4 + P5 + P6) the user-time per benchmark
dropped by ~57–62 % vs the pre-P1 baseline:

| Benchmark | Pre-P1 | Post-hotPath | Δ |
|-----------|-------:|-------------:|--:|
| Dhrystone | 8.09 s | 3.48 s | **−57 %** |
| CoreMark | 14.02 s | 5.82 s | **−58 %** |
| MicroBench | 85.82 s | 32.91 s | **−62 %** |

See [`../PROGRESS.md`](../../PROGRESS.md) §Phase 9 for the full table
and [`../spec/perfBusFastPath/SPEC.md`](../../spec/perfBusFastPath/SPEC.md),
[`../spec/perfHotPath/SPEC.md`](../../spec/perfHotPath/SPEC.md) for
per-phase design.

## Where time goes today

On the post-hotPath profile, the dominant buckets are roughly:

| Bucket | Share | Character |
|--------|------:|-----------|
| `xdb::main` (dispatch + decode + execute) | ~30 % | Interpreter core |
| MMU entry (`checked_*` + `access_bus`) | ~10 % | Per load/store |
| Mtimer deadline gate | <1 % | Per-step (post-P3) |
| Typed RAM access | <2 % | Per load/store (post-P6) |
| Device ticks (UART / PLIC / VirtIO) | <1 % | Slow path, every 64 steps |

The pre-P1 baseline had `pthread_mutex_*` at 33–40 % — now 0 %
(`Bus` is owned, not behind `Arc<Mutex<_>>`).

## The five landed phases

| Phase | Subject | Win | Risk |
|-------|---------|----:|:----:|
| **P1** busFastPath | Drop `Arc<Mutex<Bus>>`, own inline | −45…−52 % wall | Low |
| **P3** Mtimer deadline | Cache `next_fire_mtime`, short-circuit tick | Mtimer bucket → <1 % | Very low |
| **P4** icache | Per-hart decoded-inst cache, 4 K entries | `xdb::main` bucket −10 pp | Medium (invalidation) |
| **P5** MMU inline | `#[inline]` pressure through fast path | MMU bucket −3 pp | Low |
| **P6** memmove bypass | Typed reads on aligned 1/2/4/8-byte accesses | memmove bucket → <2 % | Low-Medium (unsafe) |

## Measurement pipeline

Always run from `ProjectX/` root:

```bash
bash scripts/perf/bench.sh       # → docs/perf/baselines/<today>/data/bench.csv
bash scripts/perf/sample.sh      # → <today>/data/<workload>.sample.txt
python3 scripts/perf/render.py   # → <today>/graphics/*.svg
```

- **3 runs per workload** — `user_s` is the stable metric,
  `real_s` is noisy on macOS under system load.
- Use `DEBUG=n`. PTY mode perturbs timing.
- Commit `data/` and `graphics/` with the phase's MASTER document.

## Phase exit gate pattern

A phase is not done until:

1. `cargo test --workspace` + `make linux` + `make debian` all green
   (and `-2hart` variants where applicable).
2. `bench.sh` rerun (3 iters per workload).
3. `sample.sh` rerun for each of the three benches.
4. Per-phase exit gate hit with ≥ 1 pp margin on the bucket it
   targets.
5. REPORT.md deltas committed to the phase's archived MASTER.

## What's next

- **P7 multi-hart re-profile** — pending; shapes the Phase 11 SMP
  work. Not an optimisation in itself — a measurement task.
- **Phase 11 (RFC)** — true per-hart OS threads. Requires atomic
  RAM, per-hart reservations, per-device MMIO locking. Not in any
  current perf phase.
