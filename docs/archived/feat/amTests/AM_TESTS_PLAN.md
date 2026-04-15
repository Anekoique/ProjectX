# `am-tests` Final Plan

> Status: Approved for Implementation
> Feature: `am-tests`
> Iterations: 00–06
> Owner: Executor

---

## Summary

Bare-metal am-tests for xemu. Validates CSR subsystem, ACLINT, PLIC, UART from the guest perspective. 7 required tests + run-all binary + CI integration.

**xam extended** with console (`_putch`), timer (`mtime`/`set_mtimecmp`), and trap (`init_trap`/`TrapFrame`/`__am_trap_entry`) platform modules.

**Key design decisions (evolved over 7 iterations):**
- Console via `_putch` → UART THR (single ABI, connects xlib printf)
- ACLINT split lo/hi 32-bit on both RV32/RV64
- Trap: `global_asm!(include_str!("trap.S"))` in trap.rs. t0 saved first. Every TrapFrame field truthful.
- Tests: `#ifndef AM_MENU` conditional main. Per-test = standalone. Menu = `-DAM_MENU`.
- PLIC: register accessibility only (no live device IRQ)
- Run-all: shared M-mode state, smoke path (V-IT-8 = manual)

---

## Architecture

```
xam/xhal/src/platform/xemu/
├── mod.rs        — _trm_init (existing)
├── boot.rs       — _start (existing)
├── misc.rs       — terminate/halt (existing)
├── console.rs    — _putch() → UART THR (new)
├── timer.rs      — mtime()/set_mtimecmp() (new)
├── trap.rs       — TrapFrame, init_trap(), global_asm!(include_str!("trap.S")) (new)
└── trap.S        — __am_trap_entry (new)

xkernels/tests/am-tests/
├── Makefile
├── include/
│   ├── test.h
│   └── amtest.h
├── src/main.c     — run-all
├── tests/         — 7 required tests
└── optional/      — future tests
```

## Tests

| Test | Goal | Validates |
|------|------|-----------|
| uart-putc | G-1 | printf → _putch → UART THR MMIO |
| timer-read | G-2 | mtime advances |
| csr-warl | G-5 | misa/mstatus/mie/mtvec WARL |
| trap-ecall | G-6 | ecall → handler → mepc+4 → mret |
| timer-irq | G-7 | MTIE + mtimecmp → M-mode timer interrupt |
| soft-irq | G-3 | MSIP → M-mode software interrupt |
| plic-access | G-4 | PLIC priority/enable/threshold r/w, empty claim=0 |

## Invariants

- Per-test runner: fully isolated (separate xemu instance per test)
- Run-all: shared M-mode state. Correctness depends on test order in `amtest.h`.
- TrapFrame: every field truthful. t0 saved before scratch use.
- ACLINT: split lo/hi 32-bit access.
- Runner timeout: 5s per test.

## Constraints

- RV64 M-mode, single hart
- Console via `_putch`
- PLIC = register accessibility only
- `#ifndef AM_MENU` guard in test files
- V-IT-8 (run-menu) = manual smoke validation, not CI-gated
