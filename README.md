# ProjectX

Reimagining NJU ProjectN / nemu in Rust: Build a computer system from scratch.

A RISC-V system emulator that boots OpenSBI, xv6, and Linux to an interactive shell.

AGENTS (Plan and Impl with Claude Opus 4.6 / Review with Codex) iteratively develop the system through a spec and doc-driven workflow on top of the framework I built.

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

# Boot Linux SMP (2 harts)
cd resource && make linux-2hart

# Boot Debian via VirtIO-blk (single-hart or SMP)
cd resource && make debian         # single-hart
cd resource && make debian-2hart   # 2 harts
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
| Linux 6.1.44 | ~3s to shell | OpenSBI + kernel + initramfs (single-hart or SMP via `X_HARTS`) |
| Debian 13 Trixie | ~20s to shell | VirtIO-blk rootfs (single-hart or SMP) |

## Architecture

See [docs/DEV.md](docs/DEV.md) for the full development plan and status.

## License

MIT