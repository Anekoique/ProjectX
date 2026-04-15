# Introduction

**xemu** is a RISC-V system emulator written in Rust. It boots OpenSBI,
xv6, Linux, and Debian 13 to an interactive shell, and is the
execution core of the ProjectX computer system project.

## What xemu is

- A **full-system** emulator (M / S / U privilege modes, MMU, devices)
- **RV32 / RV64 dual-target** — single codebase via `cfg`
- Supports the **IMAFDC + Zicsr + Zifencei** extensions, plus the
  privileged ISA with Sv32 / Sv39 MMU and PMP
- Ships a **debugger** (`xdb`) with breakpoints, watchpoints,
  expression evaluation, disassembly, and reference-comparison
  differential testing against QEMU and Spike
- Uses a **clean, trait-based device bus** with lock-free single-hart
  hot path

## What xemu is not

- Not a JIT. It's a pure interpreter with a decoded-instruction cache.
- Not a cycle-accurate simulator — it's faithful to architectural
  semantics, not microarchitectural timing.
- Not hardware-specific — it models the QEMU-virt-like platform with
  documented deltas.

## How this book is organised

- **Getting Started** — build xemu, run your first kernel.
- **Usage** — drive each boot target, use the debugger, run benchmarks.
- **Internals** — how the CPU, memory subsystem, and devices are
  implemented.
- **Reference** — tables of supported ISA, memory map, environment
  variables, and HAL API.
- **Contributing** — the iteration workflow and how to propose a new
  feature.

For the development roadmap and landed-phase status, see
[`../PROGRESS.md`](../PROGRESS.md). For feature-level specifications,
see [`../spec/`](../spec/).
