# Summary

[Introduction](./introduction.md)

# Getting Started

- [Overview](./overview.md)
- [Building xemu](./building.md)
- [Running your first kernel](./first-kernel.md)

# Usage

- [Boot targets](./usage/boot-targets.md)
  - [Bare-metal tests (am-tests)](./usage/am-tests.md)
  - [xv6-riscv](./usage/xv6.md)
  - [Linux (OpenSBI + initramfs)](./usage/linux.md)
  - [Debian 13 (VirtIO-blk rootfs)](./usage/debian.md)
- [The xdb debugger](./usage/debugger.md)
- [Differential testing (QEMU / Spike)](./usage/difftest.md)
- [Benchmarks](./usage/benchmarks.md)

# Internals

- [Architecture overview](./internals/architecture.md)
- [CPU dispatch & ISA decode](./internals/cpu.md)
- [CSR subsystem](./internals/csr.md)
- [Memory: MMU, TLB, PMP](./internals/memory.md)
- [Traps & interrupts](./internals/traps.md)
- [Devices](./internals/devices.md)
  - [ACLINT (MSWI / MTIMER / SSWI)](./internals/devices-aclint.md)
  - [PLIC](./internals/devices-plic.md)
  - [UART 16550](./internals/devices-uart.md)
  - [VirtIO-blk](./internals/devices-virtio.md)
- [Performance: hot path & baselines](./internals/performance.md)
- [Multi-hart](./internals/multi-hart.md)

# Reference

- [Supported ISA](./reference/isa.md)
- [Device memory map](./reference/memory-map.md)
- [Environment variables](./reference/env.md)
- [xam HAL](./reference/xam-hal.md)
- [xlib (klib)](./reference/xlib.md)

# Contributing

- [Workflow overview](./contributing/workflow.md)
- [Opening a new feature](./contributing/new-feature.md)
- [Writing a SPEC](./contributing/writing-spec.md)
- [Adding a benchmark](./contributing/adding-benchmark.md)
