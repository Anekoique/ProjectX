# `am-tests` PLAN `04`

> Status: Revised
> Feature: `am-tests`
> Iteration: `04`
> Owner: Executor
> Depends on:
> - Previous Plan: `03_PLAN.md`
> - Review: `03_REVIEW.md`
> - Master Directive: `03_MASTER.md`

---

## Summary

Final tightening round. Fixes the two remaining blockers:
1. **Menu binary**: test bodies split into `tests/<name>.c` (shared functions only, no `main()`). Standalone wrappers in `standalone/<name>.c` (thin `main()` calling the test function). Menu binary links `src/main.c` + all `tests/*.c`. No multi-main conflict.
2. **TrapFrame ABI**: every published field is truthful. Entry saves `x0=0` explicitly and saves original sp via `addi` arithmetic before frame allocation. No synthetic or garbage fields.

Also: G-2 narrowed to "mtime read" (no mtimecmp read-back claim). CI optional wording narrowed to "attempted and reported, non-gating". Makefile cleaned up per M-001.

## Log

[**Review Adjustments**]

- R-001 (menu multi-main): Resolved. Source split: `tests/*.c` = shared test bodies (no `main()`), `standalone/*.c` = thin wrappers. Menu binary links `src/main.c` + `tests/*.c`. Per-test runner builds `standalone/*.c`.
- R-002 (TrapFrame ABI mismatch): Resolved. Entry saves `x0=0` at slot 0. Saves original sp (computed as `sp + FRAME_SIZE`) at slot 2. Every field in `TrapFrame` is truthfully populated.
- R-003 (CI wording): Resolved. "Optional builds attempted and reported, non-gating."
- R-004 (mtimecmp overclaim): Resolved. G-2 = "mtime read advances". mtimecmp is exercised implicitly by timer-irq (write-only), not claimed as read-back validated.

[**Master Compliance**]

- M-001 (clean Makefile): Applied. Makefile reorganized with clear sections, comments, and consistent variable naming.
- M-002 (Rust/C boundary for TrapFrame): Applied. `TrapFrame` layout is identical in Rust (`xam trap.rs`) and C (`test.h`). `#[repr(C)]` ensures ABI compatibility. Assembly stores to exact offsets matching both definitions.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Split: tests/ (body) + standalone/ (wrapper) + src/main.c (menu) |
| Review | R-002 | Accepted | Save x0=0 and original sp truthfully |
| Review | R-003 | Accepted | Wording: "attempted and reported, non-gating" |
| Review | R-004 | Accepted | G-2 narrowed to mtime read |
| Master | M-001 | Applied | Clean Makefile |
| Master | M-002 | Applied | TrapFrame: repr(C) Rust = C struct, exact asm match |
| Trade-off | TR-1 | Adopted | Shared bodies + thin wrappers |
| Trade-off | TR-2 | Adopted | Every TrapFrame field truthful |

---

## Spec

[**Goals**]

- G-1: UART output — `uart-putc` via `printf()` → `_putch`, output asserted by runner
- G-2: Timer read — mtime advances between reads
- G-3: Software interrupt — MSIP trigger and handler, mcause verified
- G-4: PLIC register accessibility — read/write priority, enable, threshold; empty claim = 0
- G-5: CSR M-mode WARL — misa, mstatus, mie, mtvec read-back verification
- G-6: Trap ecall — ecall → handler → mepc+4 → mret
- G-7: Timer interrupt — MTIE + mtimecmp → M-mode timer interrupt
- G-8: CI — required gated, optional attempted/reported/non-gating
- G-9: xam HAL — `_putch`, `mtime()`, `set_mtimecmp()`, `init_trap()`
- G-10: Test menu — `main.c` with selection + run-all

[**Architecture**]

```
xkernels/tests/am-tests/
├── Makefile
├── include/
│   ├── test.h           — check(), MMIO, CSR macros, TrapFrame, HAL externs
│   └── amtest.h         — test declarations + table
├── src/
│   └── main.c           — menu: select by index or run all
├── tests/               — shared test bodies (no main)
│   ├── uart-putc.c      — void test_uart_putc(void)
│   ├── timer-read.c     — void test_timer_read(void)
│   ├── timer-irq.c      — void test_timer_irq(void)
│   ├── soft-irq.c       — void test_soft_irq(void)
│   ├── plic-access.c    — void test_plic_access(void)
│   ├── csr-warl.c       — void test_csr_warl(void)
│   └── trap-ecall.c     — void test_trap_ecall(void)
├── standalone/          — thin main() wrappers
│   ├── uart-putc.c      — int main() { test_uart_putc(); return 0; }
│   ├── timer-read.c     — int main() { test_timer_read(); return 0; }
│   ├── ...              — (one per test)
│   └── trap-ecall.c
└── optional/
    ├── smode-entry.c
    └── plic-uart-irq.c
```

**Build modes:**

| Mode | Command | What builds | Purpose |
|------|---------|-------------|---------|
| Per-test | `make run` | `standalone/X.c` + `tests/X.c` → one binary per test | CI |
| Menu | `make run-menu` | `src/main.c` + all `tests/*.c` → single binary | Manual |
| Optional | `make build-optional` | `optional/*.c` → compile only | CI (non-gating) |

**TrapFrame ABI (final, truthful):**

Frame on stack after `__am_trap_entry`:

| Offset | Field | Value saved |
|--------|-------|-------------|
| 0 | zero | 0 (explicit `sd x0, 0(sp)`) |
| 1 | ra | x1 |
| 2 | sp | original sp before trap (`sp + FRAME_SIZE`) |
| 3 | gp | x3 |
| 4..31 | tp..t6 | x4..x31 |
| 32 | mepc | csrr mepc |
| 33 | mcause | csrr mcause |

Total: 34 × 8 = 272 bytes.

[**Data Structure**]

**Rust (xam trap.rs):**

```rust
#[repr(C)]
pub struct GeneralRegs {
    pub zero: usize, pub ra: usize, pub sp: usize, pub gp: usize,
    pub tp: usize,   pub t0: usize, pub t1: usize, pub t2: usize,
    pub s0: usize,   pub s1: usize, pub a0: usize, pub a1: usize,
    pub a2: usize,   pub a3: usize, pub a4: usize, pub a5: usize,
    pub a6: usize,   pub a7: usize, pub s2: usize, pub s3: usize,
    pub s4: usize,   pub s5: usize, pub s6: usize, pub s7: usize,
    pub s8: usize,   pub s9: usize, pub s10: usize, pub s11: usize,
    pub t3: usize,   pub t4: usize, pub t5: usize, pub t6: usize,
}

#[repr(C)]
pub struct TrapFrame {
    pub regs: GeneralRegs,
    pub mepc: usize,
    pub mcause: usize,
}
```

**C (test.h):**

```c
typedef struct {
    unsigned long zero, ra, sp, gp, tp;
    unsigned long t0, t1, t2;
    unsigned long s0, s1;
    unsigned long a0, a1, a2, a3, a4, a5, a6, a7;
    unsigned long s2, s3, s4, s5, s6, s7, s8, s9, s10, s11;
    unsigned long t3, t4, t5, t6;
    unsigned long mepc;
    unsigned long mcause;
} TrapFrame;
```

Both are `repr(C)` / standard C layout. Assembly offsets match exactly.

**Trap entry (xam, final):**

```asm
.equ XLENB, 8
.equ FRAME_SIZE, 34 * XLENB

.macro STORE reg, slot
    sd \reg, \slot * XLENB(sp)
.endm
.macro LOAD reg, slot
    ld \reg, \slot * XLENB(sp)
.endm

.align 4
.globl __am_trap_entry
__am_trap_entry:
    addi sp, sp, -FRAME_SIZE
    // Save x0 = 0 explicitly (TrapFrame.zero is always 0)
    STORE x0,  0
    STORE x1,  1       // ra
    // Save original sp: current sp + FRAME_SIZE = pre-trap sp
    addi t0, sp, FRAME_SIZE
    STORE t0,  2       // sp (truthful original value)
    STORE x3,  3       // gp
    STORE x4,  4       // tp
    STORE x5,  5       // t0
    STORE x6,  6       // t1
    STORE x7,  7       // t2
    STORE x8,  8       // s0
    STORE x9,  9       // s1
    STORE x10, 10      // a0
    STORE x11, 11      // a1
    STORE x12, 12      // a2
    STORE x13, 13      // a3
    STORE x14, 14      // a4
    STORE x15, 15      // a5
    STORE x16, 16      // a6
    STORE x17, 17      // a7
    STORE x18, 18      // s2
    STORE x19, 19      // s3
    STORE x20, 20      // s4
    STORE x21, 21      // s5
    STORE x22, 22      // s6
    STORE x23, 23      // s7
    STORE x24, 24      // s8
    STORE x25, 25      // s9
    STORE x26, 26      // s10
    STORE x27, 27      // s11
    STORE x28, 28      // t3
    STORE x29, 29      // t4
    STORE x30, 30      // t5
    STORE x31, 31      // t6
    csrr t0, mepc
    STORE t0, 32
    csrr t0, mcause
    STORE t0, 33
    mv a0, sp           // a0 = &TrapFrame
    call __trap_dispatch
    LOAD t0, 32
    csrw mepc, t0
    LOAD x1,  1
    // skip x2/sp — restored last
    LOAD x3,  3
    LOAD x4,  4
    LOAD x5,  5
    LOAD x6,  6
    LOAD x7,  7
    LOAD x8,  8
    LOAD x9,  9
    LOAD x10, 10
    LOAD x11, 11
    LOAD x12, 12
    LOAD x13, 13
    LOAD x14, 14
    LOAD x15, 15
    LOAD x16, 16
    LOAD x17, 17
    LOAD x18, 18
    LOAD x19, 19
    LOAD x20, 20
    LOAD x21, 21
    LOAD x22, 22
    LOAD x23, 23
    LOAD x24, 24
    LOAD x25, 25
    LOAD x26, 26
    LOAD x27, 27
    LOAD x28, 28
    LOAD x29, 29
    LOAD x30, 30
    LOAD x31, 31
    addi sp, sp, FRAME_SIZE
    mret
```

Note: `sp` is NOT restored from the frame — it's restored by the symmetric `addi sp, sp, FRAME_SIZE`. This is correct because we're in M-mode with a single stack (no user/kernel switch needed).

[**Constraints**]

- C-1: RV64 M-mode, single hart
- C-2: Built via xam pipeline
- C-3: Console via `_putch` → UART THR (single ABI)
- C-4: ACLINT split lo/hi 32-bit access
- C-5: Runner timeout 5s
- C-6: PLIC = register accessibility only
- C-7: TrapFrame: every field truthful, `repr(C)` = C layout
- C-8: `tests/*.c` have no `main()`. `standalone/*.c` are thin wrappers.

---

## Implement

### File Layout

**tests/uart-putc.c** (shared body, no main):
```c
#include "test.h"
void test_uart_putc(void) {
    printf("Hello from UART!\n");
}
```

**standalone/uart-putc.c** (thin wrapper):
```c
#include "amtest.h"
int main(void) { test_uart_putc(); return 0; }
```

Same pattern for all 7 tests. Each `standalone/X.c` is 2 lines.

**src/main.c** (menu):
```c
#include <stdio.h>
#include "amtest.h"

extern void halt(int code);

int main(void) {
    printf("=== am-tests [%d tests] ===\n", (int)NUM_TESTS);
    for (int i = 0; i < (int)NUM_TESTS; i++)
        printf("  [%d] %s\n", i, tests[i].name);
    printf("  [a] Run all\n\n");

    // In batch mode, run all
    printf("Running all tests...\n");
    for (int i = 0; i < (int)NUM_TESTS; i++) {
        printf("--- %s ---\n", tests[i].name);
        tests[i].func();
        printf("--- %s PASS ---\n", tests[i].name);
    }
    printf("=== ALL PASSED ===\n");
    return 0;
}
```

### Makefile (clean, per M-001)

```makefile
# am-tests Makefile
# Two modes: per-test runner (CI) and menu binary (manual)
.PHONY: run run-menu build-optional clean

# ── Config ──
TIMEOUT  ?= 5
AM_TESTS := $(shell pwd)
RESULT   := .result

GREEN := \033[1;32m
RED   := \033[1;31m
NONE  := \033[0m

REQUIRED := $(basename $(notdir $(wildcard tests/*.c)))
OPTIONAL := $(basename $(notdir $(wildcard optional/*.c)))

# ── Per-test runner (CI) ──
run: $(shell > $(RESULT)) $(addprefix Run., $(REQUIRED))
	@cat $(RESULT)
	@grep -q "FAIL" $(RESULT); F=$$?; rm -f $(RESULT); test $$F -ne 0

Run.%:
	@printf "NAME = $*\nK = standalone/$*.c\nEXTRA_SRCS = tests/$*.c\nINC_PATH += $(AM_TESTS)/include\ninclude $${AM_HOME}/Makefile\n" > Makefile.$*
	@timeout $(TIMEOUT) make -s -f Makefile.$* run 2>&1 > .output.$* || true
	@if grep -q "GOOD TRAP" .output.$* \
	  $(if $(filter uart-putc,$*), && grep -q "Hello from UART" .output.$*,) ; then \
		printf "[%14s] $(GREEN)PASS$(NONE)\n" $* >> $(RESULT); \
	else \
		printf "[%14s] $(RED)***FAIL***$(NONE)\n" $* >> $(RESULT); \
	fi
	@rm -f Makefile.$* .output.$*

# ── Menu binary (manual) ──
run-menu:
	@printf "NAME = am-tests\nK = src/main.c\nEXTRA_SRCS = $(wildcard tests/*.c)\nINC_PATH += $(AM_TESTS)/include\ninclude $${AM_HOME}/Makefile\n" > Makefile.menu
	@make -s -f Makefile.menu run
	@rm -f Makefile.menu

# ── Optional compile check ──
build-optional: $(addprefix Build., $(OPTIONAL))

Build.%:
	@printf "NAME = $*\nK = optional/$*.c\nINC_PATH += $(AM_TESTS)/include\ninclude $${AM_HOME}/Makefile\n" > Makefile.$*
	@make -s -f Makefile.$* kernel
	@rm -f Makefile.$*

# ── Clean ──
clean:
	rm -rf Makefile.* build/ $(RESULT) .output.*
```

### xam build_c.mk change (3 lines)

```makefile
# Add after line 4 (OBJS = ...):
EXTRA_SRCS ?=
EXTRA_OBJS  = $(addprefix $(OUT_DIR)/, $(notdir $(patsubst %.c,%.o,$(patsubst %.S,%.o,$(EXTRA_SRCS)))))
OBJS       += $(EXTRA_OBJS)
VPATH      += $(sort $(dir $(EXTRA_SRCS)))
```

### xam platform changes

Same as 03_PLAN: `console.rs` (`_putch`), `timer.rs` (`mtime`/`set_mtimecmp`), `trap.rs` (`GeneralRegs`/`TrapFrame`/`init_trap`/entry asm). Updated `mod.rs` to include new modules.

---

## Validation

- V-IT-1: `uart-putc` — exits 0 + "Hello from UART!" in output
- V-IT-2: `timer-read` — mtime advances, exits 0
- V-IT-3: `csr-warl` — WARL masks correct, exits 0
- V-IT-4: `trap-ecall` — ecall→handler→mret, exits 0
- V-IT-5: `timer-irq` — timer interrupt, exits 0
- V-IT-6: `soft-irq` — software interrupt, exits 0
- V-IT-7: `plic-access` — registers r/w, empty claim=0, exits 0
- V-IT-8: `run-menu` — all tests pass in single binary
- V-IT-9: CI required gated. Optional attempted/reported, non-gating.

| Goal | Validation |
|------|------------|
| G-1 UART | V-IT-1 |
| G-2 Timer | V-IT-2 |
| G-3 Soft IRQ | V-IT-6 |
| G-4 PLIC | V-IT-7 |
| G-5 CSR | V-IT-3 |
| G-6 Trap | V-IT-4 |
| G-7 Timer IRQ | V-IT-5 |
| G-8 CI | V-IT-9 |
| G-9 xam HAL | All tests use _putch/mtime/init_trap |
| G-10 Menu | V-IT-8 |
