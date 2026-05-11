# `am-tests` Implementation Summary

> Feature: `am-tests`
> Iterations: 00–06 (PLAN) + implementation
> Branch: `tests/am-test`

---

## What Was Built

7 bare-metal tests that validate xemu's CSR subsystem, ACLINT, PLIC, and UART from the guest's perspective. Built via the xam abstract machine pipeline, run on xemu.

### Tests

| Key | Name | Validates |
|-----|------|-----------|
| u | uart-putc | printf → _putch → UART THR MMIO → xemu stdout |
| r | timer-read | ACLINT mtime advances between reads |
| t | timer-irq | M-mode timer interrupt: MTIE + mtimecmp → handler |
| s | soft-irq | M-mode software interrupt: ACLINT MSIP → handler |
| p | plic-access | PLIC priority/enable/threshold register r/w, empty claim=0 |
| c | csr-warl | mstatus MIE, mie writable bits, mtvec alignment mask |
| e | trap-ecall | ecall → handler → mepc+4 → mret roundtrip |

### Usage

```
make run              # run all 7 tests (CI)
make run TEST=t       # run timer-irq only
make run TEST=u BATCH=n  # enter xdb for debugging
```

---

## xam Changes

### New platform modules (`xhal/src/platform/xemu/`)

- **`console.rs`** — `_putch()` writes to UART THR at `0x1000_0000`. Overrides xlib's weak stub. Enables `printf()` for all C programs.
- **`timer.rs`** — `mtime()` reads 64-bit mtime via split lo/hi (atomic hi-lo-hi loop). `set_mtimecmp()` writes with hi=MAX guard to prevent spurious fire.
- **`trap.rs`** — `TrapFrame` (32 GPRs + mepc + mcause), `init_trap(handler)` sets mtvec. `__trap_dispatch` calls registered handler.
- **`trap.S`** — Trap entry: saves t0 first (before scratch use), computes original sp, saves all 31 GPRs + mepc + mcause. Restores and `mret`.
- **`mod.rs`** — `main(const char *args)` signature. Weak `mainargs` default via `global_asm!`.

### Build system changes

- **`build_c.mk`** — Added `SRCS` support (nemu-style multi-source builds alongside single-source `K`)
- **`kernel.mk`** — `K` is the program name, output = `$(K)_$(PLATFORM).elf`
- **`Makefile`** — `BUILD_KERNEL` triggers on `K` or `SRCS`
- **`xdb/main.rs`** — Load file in both batch and interactive modes (BATCH=n loads before REPL)

### am-tests structure

```
xkernels/tests/am-tests/
├── Makefile              # test runner (same pattern as cpu-tests)
├── include/
│   ├── test.h            # check(), MMIO macros, CSR macros, TrapFrame, HAL externs
│   └── amtest.h          # test function declarations
└── src/
    ├── main.c            # dispatch: switch on mainargs[0]
    └── tests/
        ├── uart-putc.c   # printf via UART MMIO
        ├── timer-read.c  # mtime advancement
        ├── timer-irq.c   # M-mode timer interrupt handler
        ├── soft-irq.c    # M-mode software interrupt
        ├── plic-access.c # PLIC register read/write
        ├── csr-warl.c    # CSR WARL mask verification
        └── trap-ecall.c  # ecall → handler → mret
```

### CI

Added `test-am` job to `.github/workflows/ci.yml` — builds xemu, runs `make run` in am-tests.

---

## Key Design Decisions

- **`mainargs` via `-DMAINARGS`**: Compile-time argument passing. `main.c` defines `const char mainargs[] = MAINARGS` which overrides xam's weak default. Simple, no tools needed.
- **`_putch` as single console ABI**: xam provides `_putch` → UART THR. xlib's `printf` chains through it. One console path.
- **Split lo/hi ACLINT access**: Both RV32/RV64 use 32-bit MMIO for mtime/mtimecmp. Matches xemu's register decode model.
- **Trap entry saves t0 first**: Before using t0 as scratch for original sp computation. Every TrapFrame field is truthful.
- **Test runner follows cpu-tests pattern**: Generated `.mk.*` Makefile per test, `make -s -f .mk.$* run`, grep for `GOOD TRAP`.

## Known Issues

- **misa = 0**: xemu doesn't initialize misa CSR. `csr-warl` test reads it but doesn't assert a specific value.
- **Run-all shared state**: Running all tests via `make run` uses separate xemu instances (isolated). A single-binary run-all (`MAINARGS=a`) shares M-mode state between tests.
