# shellcheck shell=bash
# scripts/perf/config.sh — single source of truth for perf runs.
# Sourced by bench.sh / sample.sh.  Not executable.

PERF_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]:-$0}")" && pwd)"
PROJECT_ROOT="$(cd "$PERF_ROOT/../.." && pwd)"

BENCH_ROOT="$PROJECT_ROOT/xkernels/benchmarks"
# XEMU_BIN depends on PERF_MODE (set below).  Cargo places the perf
# profile's artifacts under `target/perf/`, release under `target/release/`.
# We resolve the path after PERF_MODE is known.

# Workloads: (name, dir, sample-seconds).  Sample-seconds is the attach
# window for CPU profiling; the wall-clock bench ignores it.
WORKLOADS=(
  "dhrystone  $BENCH_ROOT/dhrystone   6"
  "coremark   $BENCH_ROOT/coremark   12"
  "microbench $BENCH_ROOT/microbench 15"
)

# Default output directory: docs/perf/<today>.
DEFAULT_OUT="$PROJECT_ROOT/docs/perf/$(date +%Y-%m-%d)"

# Build profile for `make run` during perf captures.
#
# `release` (default here) — what the `make run` chain expects end-to-end;
#   matches the binary users ship.  Without DWARF, stacks fall back to
#   addresses, but symbolication via the `.o` files + `nm` still works.
# `perf` — inherits release + debug=line-tables-only (Cargo profile in
#   xemu/Cargo.toml).  Requires a matching xam rebuild; for now,
#   `cargo build --profile perf` is the direct-invocation path used by
#   `samply record -- target/perf/xdb` when you need line-level stacks.
PERF_MODE="${PERF_MODE:-release}"
XEMU_BIN="$PROJECT_ROOT/xemu/target/$PERF_MODE/xdb"

# NOTE on frame pointers: we intentionally do NOT export RUSTFLAGS here.
# Injecting RUSTFLAGS=-C force-frame-pointers=yes leaks into every
# nested `cargo` invocation that the kernel-side Makefiles trigger
# (xam + xkernels), which changes guest ELF layout and skews
# measurements.  If you need frame-pointer stacks for a specific
# capture, invoke cargo directly:
#   RUSTFLAGS="-C force-frame-pointers=yes" cargo build \
#     --manifest-path xemu/Cargo.toml --profile perf
# then point scripts/perf/ at the resulting `target/perf/xdb` via
# PERF_MODE=perf.  Apple `sample` handles Rust's default non-FP stacks
# well enough to get the data we need for the 2026-04-14 baseline.

# Git revision of the perf-scripts tree; gets stamped into every run's
# bench.summary so a committed artifact can be traced back to a tool rev.
PERF_REV="$(cd "$PROJECT_ROOT" && git rev-parse --short HEAD 2>/dev/null || echo unknown)"
