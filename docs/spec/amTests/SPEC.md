# `amTests` SPEC

> Source: [`/docs/archived/feat/amTests/06_PLAN.md`](/docs/archived/feat/amTests/06_PLAN.md).
> Iteration history, trade-off analysis, and implementation
> plan live under `docs/archived/feat/amTests/`.

---


[**Goals**]

- G-1: UART output — via `printf()` → `_putch`, output asserted
- G-2: Timer read — mtime advances
- G-3: Software interrupt — MSIP, mcause verified
- G-4: PLIC register access — priority, enable, threshold r/w; empty claim = 0
- G-5: CSR M-mode WARL — misa, mstatus, mie, mtvec
- G-6: Trap ecall — ecall → handler → mepc+4 → mret
- G-7: Timer interrupt — MTIE + mtimecmp → M-mode timer
- G-8: CI — required gated, optional attempted (non-gating)
- G-9: xam HAL — `_putch`, `mtime()`, `set_mtimecmp()`, `init_trap()`
- G-10: Run-all binary — lists and runs all tests

[**Architecture**]

```
xam/xhal/src/platform/xemu/
├── mod.rs          — _trm_init (existing)
├── boot.rs         — _start (existing)
├── misc.rs         — terminate/halt (existing)
├── console.rs      — _putch() → UART THR (new)
├── timer.rs        — mtime()/set_mtimecmp() (new)
├── trap.rs         — TrapFrame, init_trap() (new)
└── trap.S          — __am_trap_entry asm (new)

xkernels/tests/am-tests/
├── Makefile
├── include/
│   ├── test.h       — check(), MMIO, CSR, TrapFrame, HAL externs
│   └── amtest.h     — test table
├── src/
│   └── main.c       — run-all entry
├── tests/           — each file = test body + conditional main
│   ├── uart-putc.c
│   ├── timer-read.c
│   ├── timer-irq.c
│   ├── soft-irq.c
│   ├── plic-access.c
│   ├── csr-warl.c
│   └── trap-ecall.c
└── optional/
    ├── smode-entry.c
    └── plic-uart-irq.c
```

**Build modes:**

| Mode | Command | How it builds |
|------|---------|---------------|
| Per-test | `make run` | `K = tests/uart-putc.c` → one binary (has `main()`) |
| Run-all | `make run-menu` | `K = src/main.c`, `EXTRA_SRCS = tests/*.c`, `CFLAGS += -DAM_MENU` → one binary (menu `main()`, no per-test `main()`) |
| Optional | `make build-optional` | `K = optional/xxx.c` → compile only |

**Test file pattern:**

```c
// tests/uart-putc.c
#include "test.h"
void test_uart_putc(void) { printf("Hello from UART!\n"); }
#ifndef AM_MENU
int main(void) { test_uart_putc(); return 0; }
#endif
```

In per-test mode: `AM_MENU` not defined → `main()` included → standalone binary.
In menu mode: `-DAM_MENU` → `main()` compiled out → `test_uart_putc()` linked with `src/main.c`.

No standalone directory. No naming mismatch. No object collision.

[**Invariants**]

- I-1: **Per-test runner**: each test runs in its own xemu instance — fully isolated. **Run-all mode**: tests share M-mode machine state (mtvec, mie, mstatus may be left modified by previous test).
- I-2: Pass = `main()` returns 0. Fail = `check()` → `halt(1)`.
- I-3: ACLINT access: split lo/hi 32-bit.
- I-4: TrapFrame: every field truthful. t0 saved before scratch use.
- I-5: Runner timeout 5s.

[**Data Structure**]

**TrapFrame (Rust, xam trap.rs):**

```rust
#[repr(C)]
#[derive(Debug, Clone, Copy)]
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
#[derive(Debug, Clone, Copy)]
pub struct TrapFrame {
    pub regs: GeneralRegs,
    pub mepc: usize,
    pub mcause: usize,
}
```

**TrapFrame (C, test.h):**

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

**test.h:**

```c
#ifndef TEST_H
#define TEST_H

#include <stdio.h>

extern void halt(int code);
#define check(cond) do { \
    if (!(cond)) { printf("FAIL: %s:%d\n", __FILE__, __LINE__); halt(1); } \
} while (0)

#define REG32(a) (*(volatile unsigned int *)(a))
#define REG8(a)  (*(volatile unsigned char *)(a))

#define ACLINT    0x02000000UL
#define MSIP      REG32(ACLINT + 0x0000)
#define PLIC      0x0C000000UL
#define PLIC_PRI(s)  REG32(PLIC + (s) * 4)
#define PLIC_EN(c)   REG32(PLIC + 0x2000 + (c) * 0x80)
#define PLIC_THR(c)  REG32(PLIC + 0x200000 + (c) * 0x1000)
#define PLIC_CLM(c)  REG32(PLIC + 0x200004 + (c) * 0x1000)

#define csrr(c)   ({ unsigned long __v; asm volatile ("csrr %0, " #c : "=r"(__v)); __v; })
#define csrw(c,v) asm volatile ("csrw " #c ", %0" :: "r"((unsigned long)(v)))
#define csrs(c,v) asm volatile ("csrs " #c ", %0" :: "r"((unsigned long)(v)))
#define csrc(c,v) asm volatile ("csrc " #c ", %0" :: "r"((unsigned long)(v)))

extern unsigned long long mtime(void);
extern void set_mtimecmp(unsigned long long val);
extern void init_trap(void (*handler)(TrapFrame *));

#endif
```

**amtest.h:**

```c
#ifndef AMTEST_H
#define AMTEST_H

typedef void (*TestFunc)(void);
typedef struct { const char *name; TestFunc func; } TestEntry;

void test_uart_putc(void);
void test_timer_read(void);
void test_timer_irq(void);
void test_soft_irq(void);
void test_plic_access(void);
void test_csr_warl(void);
void test_trap_ecall(void);

static const TestEntry tests[] = {
    {"uart-putc",   test_uart_putc},
    {"timer-read",  test_timer_read},
    {"timer-irq",   test_timer_irq},
    {"soft-irq",    test_soft_irq},
    {"plic-access", test_plic_access},
    {"csr-warl",    test_csr_warl},
    {"trap-ecall",  test_trap_ecall},
};
#define NUM_TESTS (sizeof(tests) / sizeof(tests[0]))

#endif
```

[**Constraints**]

- C-1: RV64 M-mode, single hart
- C-2: Built via xam pipeline
- C-3: Console via `_putch` → UART THR
- C-4: ACLINT split lo/hi 32-bit
- C-5: Runner timeout 5s
- C-6: PLIC = register accessibility only
- C-7: TrapFrame truthful, t0 saved first
- C-8: `#ifndef AM_MENU` guard in test files

---
