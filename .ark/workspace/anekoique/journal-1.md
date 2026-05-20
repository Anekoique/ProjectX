# Journal 1

## Session 1: Port ProjectX workflow to Ark

**Date**: 2026-05-11
**Slug**: port-to-ark
**Branch**: `feat/port-to-ark`
**Base Branch**: `main`
**Start Head**: `ae44a01`
**Closing Commit**: <PENDING:port-to-ark>

### Summary

Retire docs/-based workflow; consolidate task / archive / feature-spec under .ark/.

### Main Changes

| Area | Description |
|------|-------------|
| Specs | 12 feature SPECs rebuilt from current code in Ark template shape (kebab-case). |
| Archive | ~270 legacy iteration files relocated to .ark/tasks/archive/legacy/ via git mv. |
| Docs | docs/{tasks,spec,archived,template,README.md} removed; PROGRESS + book retargeted. |
| Workflow | AGENTS.md slimmed to standards + Ark pointer; .rs doc-cites + CI script updated. |

### Git Commits

| Hash | Message |
|------|---------|
| _(none)_ |   |

## Session 2: Add xvisor basic framework

**Date**: 2026-05-19
**Slug**: framework
**Branch**: `feat/xvisor-framework`
**Base Branch**: `main`
**Start Head**: `2c5c22c`
**Closing Commit**: <PENDING:framework>

### Summary

Boot a HS-mode Rust hypervisor under QEMU virt + OpenSBI fw_jump; print banner, halt via SiFive-test finisher.

### Main Changes

| Area | Description |
|------|-------------|
| xvisor crate | New no_std bin: naked _start, PerCpu via tp, DTB capture, stvec wfi trampoline. |
| HAL | hal::{arch::riscv, platform::qemu} with cfg_attr-selected backends + loongarch/xemu stubs. |
| Build | xvisor/Makefile, build.rs, linker.ld; root rust-toolchain.toml adds riscv64gc target. |
| Tooling | .vscode/settings.json links xvisor; xemu/feature SPECs untouched. |

### Git Commits

| Hash | Message |
|------|---------|
| _(none)_ |   |

## Session 3: Add xvisor trap framework

**Date**: 2026-05-20
**Slug**: trap
**Branch**: `feat/xvisor-trap`
**Base Branch**: `feat/xvisor`
**Start Head**: `13515a6`
**Closing Commit**: <PENDING:trap>

### Summary

HS-mode trap entry + Rust dispatcher; ebreak round-trip via `trap-canary` cargo feature; wfi parking-pad retired.

### Main Changes

| Area | Description |
|------|-------------|
| trap | trap.S save/restore (global_asm!) + Rust dispatcher; sepc advance via instruction_width. |
| boot | `install_trap_vector` writes real `trap_entry` into stvec; wfi trampoline deleted. |
| build | `trap-canary` cargo feature gates ebreak demo; build.rs reruns on trap.S edits. |
| harness | `make trap-test` (tee + grep) under bash pipefail; `cause` arch-local, host tests deferred. |

### Git Commits

| Hash | Message |
|------|---------|
| _(none)_ |   |
