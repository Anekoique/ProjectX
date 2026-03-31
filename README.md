# ProjectX

A RISC-V system emulator that boots OpenSBI, xv6, and Linux.

## Components

| Directory | Description |
|-----------|-------------|
| `xemu/` | RISC-V emulator (Rust workspace: xcore, xdb, xlogger) |
| `xam/` | Bare-metal HAL for xemu (Rust) |
| `xkernels/` | Test kernels (am-tests, cpu-tests, benchmarks) |
| `resource/` | External boot artifacts (OpenSBI, xv6, Linux) |
| `docs/` | Development plan, iteration documents |

## Quick Start

```bash
# Run bare-metal tests
cd xkernels/tests/am-tests && make run

# Boot xv6 (interactive shell)
cd resource && make xv6

# Boot Linux (via OpenSBI, slow on interpreted emulator)
cd resource && make linux
```

## Benchmark (MacBook Air M4)

- MicroBench 55 Marks
- CoreMark   38 Marks
- DhryStone  20 Marks

## Architecture

See [docs/DEV.md](docs/DEV.md) for the full development plan and status.
