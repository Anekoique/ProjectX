# ProjectX

Reimagining NJU ProjectN / nemu in Rust: Build a computer system from scratch with AGENTS(Plan and Impl with Claude Opus 4.6 / Review with Codex). See the [dev rounds](./docs). Most of the work is completed within one week.

A RISC-V system emulator that boots OpenSBI, xv6, and Linux to an interactive shell.

## Components

| Directory | Description |
|-----------|-------------|
| `xemu/` | RISC-V emulator (xcore: core of emu, xdb: monitor) |
| `xam/` | Bare-metal HAL for xemu (abstract-machine) |
| `xkernels/` | Test kernels (am-tests, cpu-tests, benchmarks) |
| `resource/` | External boot artifacts (OpenSBI, xv6, Linux) |
| `docs/` | Development plan, iteration documents |

## Quick Start

```bash
# Run bare-metal tests
cd xkernels/tests/am-tests && make run

# Boot xv6 (interactive shell)
cd resource && make xv6

# Boot Linux (OpenSBI + kernel + initramfs → shell)
cd resource && make linux
```

## Benchmark 

Platform: MacBook Air M4

| Benchmark | Marks |
|-----------|-------|
| MicroBench | 687 |
| CoreMark | 446 |
| DhryStone | 248 |

## OS Boot

| OS | Boot Time | Status |
|----|-----------|--------|
| OpenSBI v1.3.1 | ~0.1s | M-mode firmware |
| xv6-riscv | ~0.3s | Interactive shell (ramdisk) |
| Linux 6.1.44 | ~3s to shell | OpenSBI + kernel + initramfs |

## Architecture

See [docs/DEV.md](docs/DEV.md) for the full development plan and status.

## License

MIT