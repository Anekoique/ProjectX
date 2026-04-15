# Overview

## Top-level layout

```
ProjectX/
├── xemu/           RISC-V emulator
│   ├── xcore/       execution engine — CPU, MMU, devices, bus
│   ├── xdb/         debugger / monitor (REPL, breakpoints, difftest)
│   └── xlogger/     logging — colored, levelled, per-instruction trace
├── xam/            bare-metal HAL (abstract-machine) — targets xemu
├── xlib/           freestanding C library (klib) — printf, string, stdio
├── xkernels/       test kernels
│   └── tests/       am-tests, cpu-tests, alu-tests, benchmarks
├── resource/       external boot artifacts — OpenSBI, xv6, Linux, Debian
├── scripts/        CI + perf measurement scripts
└── docs/           this documentation
```

## Component relationships

```
xkernel source (C / Rust)
    │
    │ compile with
    ▼
xam HAL  +  xlib (klib)
    │
    │ produces
    ▼
ELF image
    │
    │ loaded by
    ▼
xemu (xdb binary)
    │
    │ executes through
    ▼
xcore: CPU → MMU → Bus → Devices (ACLINT / PLIC / UART / VirtIO)
```

## Crates at a glance

| Crate | Role | Key types |
|-------|------|-----------|
| `xcore` | Execution engine | `CPU`, `RVCore`, `Bus`, `Mmu`, `Pmp`, `Aclint`, `Plic`, `Uart`, `VirtioBlk` |
| `xdb` | Binary + debugger | `Monitor`, breakpoint / watchpoint tables, command REPL |
| `xlogger` | Log facade | `trace!` / `debug!` / `info!` macros with color + timestamp |
| `xam` | Guest HAL | `_putch`, `mtime`, `uptime`, `init_trap`, `TrapFrame` |
| `xlib` | Guest C library | `printf`, `memset`, `memcpy`, `strlen`, `strcmp`, `assert.h` |

## Boot target summary

| Target | Make command | Firmware | Rootfs |
|--------|--------------|----------|--------|
| am-tests | `cd xkernels/tests/am-tests && make run` | none (bare) | — |
| xv6 | `cd resource && make xv6` | xv6 bootstrap | ramdisk |
| Linux | `cd resource && make linux` | OpenSBI v1.3.1 | initramfs |
| Linux SMP | `cd resource && make linux-2hart` | OpenSBI | initramfs |
| Debian 13 | `cd resource && make debian` | OpenSBI + bootlin kernel | ext4 over VirtIO-blk |
| Debian SMP | `cd resource && make debian-2hart` | OpenSBI + bootlin kernel | ext4 over VirtIO-blk |

See [Boot targets](./usage/boot-targets.md) for each in detail.
