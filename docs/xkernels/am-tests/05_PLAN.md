# `am-tests` PLAN `05`

> Status: Revised
> Feature: `am-tests`
> Iteration: `05`
> Owner: Executor
> Depends on:
> - Previous Plan: `04_PLAN.md`
> - Review: `04_REVIEW.md`
> - Master Directive: `04_MASTER.md`

---

## Summary

Bare-metal am-tests for xemu. Validates CSR subsystem, ACLINT, PLIC, UART from the guest perspective. Built via xam pipeline, run on xemu.

This round fixes the two persistent blockers that have survived rounds 01–04:

1. **Object name collision (R-001)**: Standalone wrappers use distinct filenames (`run_xxx.c`) from test bodies (`xxx.c`). No collision. No build system tricks.
2. **Trap t0 clobber (R-002)**: Save `t0` FIRST, before using it as scratch to compute original sp. Exact save/restore order documented and traced instruction-by-instruction.

Also narrows menu goal to run-all (R-003/TR-2), adds timeout + output assertion for menu mode (R-004).

## Log

[**Review Adjustments**]

- R-001 (object name collision): Resolved. `standalone/run_xxx.c` (kernel main) vs `tests/xxx.c` (test body). Different basenames → different `.o` files. No collision possible. Traced: `KERNEL_NAME = run_uart_putc` → `$(OUT_DIR)/run_uart_putc.o`. `EXTRA_SRCS = tests/uart-putc.c` → `$(OUT_DIR)/uart-putc.o`. Two distinct objects.
- R-002 (t0 clobber): Resolved. Entry saves `x5/t0` at slot 5 as the FIRST register save. Then uses `t0` as scratch to compute `sp + FRAME_SIZE` for the original sp slot. Exact instruction trace:
  ```
  addi sp, sp, -FRAME_SIZE    // allocate frame
  sd   x5, 5*8(sp)            // SAVE t0 FIRST (slot 5 = truthful x5)
  sd   x0, 0*8(sp)            // slot 0 = 0
  sd   x1, 1*8(sp)            // slot 1 = ra
  addi x5, sp, FRAME_SIZE     // t0 = original sp (scratch use AFTER save)
  sd   x5, 2*8(sp)            // slot 2 = original sp (truthful)
  sd   x3, 3*8(sp)            // slot 3 = gp
  sd   x4, 4*8(sp)            // slot 4 = tp
  // x5 already saved at slot 5
  sd   x6, 6*8(sp)            // slot 6 = t1
  ...                          // x7-x31 saved normally
  ```
  Restore: `ld x5, 5*8(sp)` restores the original t0 value. sp restored by `addi sp, sp, FRAME_SIZE`.
- R-003 (menu promises selection): Resolved. G-10 narrowed to "single binary that lists tests and runs all". No input/selection contract.
- R-004 (menu not machine-checkable): Resolved. `run-menu` target has timeout and asserts `ALL PASSED` in output.

[**Master Compliance**]

- M-001 (/pua to fix persistent problems): Applied. Both R-001 and R-002 traced instruction-by-instruction to verify correctness before writing the plan.
- M-002 (entire PLAN with integrity): Applied. Full plan document below.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | `run_xxx.c` vs `xxx.c` — distinct basenames, no collision |
| Review | R-002 | Accepted | Save t0 first, then use as scratch. Instruction trace verified. |
| Review | R-003 | Accepted | Menu = run-all only, no selection |
| Review | R-004 | Accepted | run-menu has timeout + output assertion |
| Master | M-001 | Applied | Traced both fixes instruction-by-instruction |
| Master | M-002 | Applied | Full plan document |
| Trade-off | TR-1 | Adopted | Distinct filenames instead of path-preserving objects |
| Trade-off | TR-2 | Adopted | Menu narrowed to run-all |

---

## Spec

[**Goals**]

- G-1: UART output — `uart-putc` via `printf()` → `_putch`, output asserted
- G-2: Timer read — mtime advances between reads
- G-3: Software interrupt — MSIP trigger, handler, mcause verified
- G-4: PLIC register accessibility — priority, enable, threshold r/w; empty claim = 0
- G-5: CSR M-mode WARL — misa, mstatus, mie, mtvec read-back
- G-6: Trap ecall — ecall → handler → mepc+4 → mret
- G-7: Timer interrupt — MTIE + mtimecmp → M-mode timer interrupt
- G-8: CI — required gated, optional attempted/reported (non-gating)
- G-9: xam HAL — `_putch`, `mtime()`, `set_mtimecmp()`, `init_trap()`
- G-10: Run-all binary — single binary that lists and runs all tests sequentially

- NG-1: S-mode / U-mode (optional, not required)
- NG-2: Multi-hart
- NG-3: Interactive test selection (no input HAL)
- NG-4: End-to-end UART→PLIC→mip

[**Architecture**]

```
xkernels/tests/am-tests/
├── Makefile
├── include/
│   ├── test.h            — check(), MMIO, CSR macros, TrapFrame, HAL externs
│   └── amtest.h          — test declarations + table
├── src/
│   └── main.c            — run-all: list tests, run each, print result
├── tests/                — test bodies (no main)
│   ├── uart-putc.c       — void test_uart_putc(void)
│   ├── timer-read.c
│   ├── timer-irq.c
│   ├── soft-irq.c
│   ├── plic-access.c
│   ├── csr-warl.c
│   └── trap-ecall.c
├── standalone/           — per-test wrappers (distinct filenames!)
│   ├── run_uart_putc.c   — int main() { test_uart_putc(); return 0; }
│   ├── run_timer_read.c
│   ├── run_timer_irq.c
│   ├── run_soft_irq.c
│   ├── run_plic_access.c
│   ├── run_csr_warl.c
│   └── run_trap_ecall.c
└── optional/
    ├── smode-entry.c
    └── plic-uart-irq.c

xam/xhal/src/platform/xemu/
├── mod.rs        — _trm_init (existing)
├── boot.rs       — _start (existing)
├── misc.rs       — terminate/halt (existing)
├── console.rs    — _putch() → UART THR (new)
├── timer.rs      — mtime()/set_mtimecmp() (new)
└── trap.rs       — TrapFrame, init_trap(), __am_trap_entry asm (new)
```

**Object name trace (per-test build):**

```
K = standalone/run_uart_putc.c
KERNEL_NAME = run_uart_putc
KERNEL OBJ  = $(OUT_DIR)/run_uart_putc.o      ← from standalone/run_uart_putc.c

EXTRA_SRCS = tests/uart-putc.c
EXTRA_OBJS = $(OUT_DIR)/uart-putc.o            ← from tests/uart-putc.c

→ No collision. Two distinct .o files linked together.
```

**Console path:**

```
printf("Hello") → xlib vsnprintf → _putch(c) → xam console.rs → UART THR MMIO → xemu stdout
```

[**Invariants**]

- I-1: Tests are standalone. No inter-test dependencies.
- I-2: Pass = `main()` returns 0. Fail = `check()` → `halt(1)`.
- I-3: Trap tests call `init_trap(handler)` from xam.
- I-4: ACLINT access: split lo/hi 32-bit on both RV32/RV64.
- I-5: Runner timeout 5s per test. Hang = FAIL.
- I-6: TrapFrame: every field truthful. t0 saved before scratch use.
- I-7: `standalone/run_xxx.c` and `tests/xxx.c` have distinct basenames.

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

| Slot | Field | Saved value |
|------|-------|-------------|
| 0 | zero | 0 (explicit `sd x0`) |
| 1 | ra | x1 |
| 2 | sp | original sp (= current sp + FRAME_SIZE, via t0 scratch AFTER t0 is saved) |
| 3 | gp | x3 |
| 4 | tp | x4 |
| 5 | t0 | x5 (**saved first**, before any scratch use) |
| 6-31 | t1..t6 | x6-x31 |
| 32 | mepc | csrr mepc |
| 33 | mcause | csrr mcause |

**test.h:**

```c
#include <stdio.h>

extern void halt(int code);

#define check(cond) do { \
    if (!(cond)) { printf("FAIL: %s:%d\n", __FILE__, __LINE__); halt(1); } \
} while (0)

#define REG32(a)  (*(volatile unsigned int *)(a))
#define REG8(a)   (*(volatile unsigned char *)(a))

#define ACLINT     0x02000000UL
#define MSIP       REG32(ACLINT + 0x0000)
#define PLIC       0x0C000000UL
#define PLIC_PRI(s)   REG32(PLIC + (s) * 4)
#define PLIC_EN(c)    REG32(PLIC + 0x2000 + (c) * 0x80)
#define PLIC_THR(c)   REG32(PLIC + 0x200000 + (c) * 0x1000)
#define PLIC_CLM(c)   REG32(PLIC + 0x200004 + (c) * 0x1000)

#define csrr(c)   ({ unsigned long __v; asm volatile ("csrr %0, " #c : "=r"(__v)); __v; })
#define csrw(c,v) asm volatile ("csrw " #c ", %0" :: "r"((unsigned long)(v)))
#define csrs(c,v) asm volatile ("csrs " #c ", %0" :: "r"((unsigned long)(v)))
#define csrc(c,v) asm volatile ("csrc " #c ", %0" :: "r"((unsigned long)(v)))

extern unsigned long long mtime(void);
extern void set_mtimecmp(unsigned long long val);
extern void init_trap(void (*handler)(TrapFrame *));
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
- C-7: TrapFrame: every field truthful. t0 saved before scratch.
- C-8: `standalone/run_xxx.c` and `tests/xxx.c` have distinct basenames.

---

## Implement

### xam Changes

**xam/scripts/build_c.mk** — add 4 lines after existing line 4:

```makefile
EXTRA_SRCS ?=
EXTRA_OBJS  = $(addprefix $(OUT_DIR)/, $(notdir $(patsubst %.c,%.o,$(patsubst %.S,%.o,$(EXTRA_SRCS)))))
OBJS       += $(EXTRA_OBJS)
VPATH      += $(sort $(dir $(EXTRA_SRCS)))
```

**xam/xhal/src/platform/xemu/console.rs:**

```rust
const UART_THR: *mut u8 = 0x1000_0000 as *mut u8;
const UART_LSR: *const u8 = 0x1000_0005 as *const u8;

/// Strong override of xlib's weak `_putch`. Enables `printf()` for all C programs.
#[unsafe(no_mangle)]
pub extern "C" fn _putch(c: i8) {
    unsafe {
        while UART_LSR.read_volatile() & 0x20 == 0 {}
        UART_THR.write_volatile(c as u8);
    }
}
```

**xam/xhal/src/platform/xemu/timer.rs:**

```rust
const ACLINT: usize = 0x0200_0000;
const MTIME_LO:    *const u32 = (ACLINT + 0xBFF8) as _;
const MTIME_HI:    *const u32 = (ACLINT + 0xBFFC) as _;
const MTIMECMP_LO: *mut u32   = (ACLINT + 0x4000) as _;
const MTIMECMP_HI: *mut u32   = (ACLINT + 0x4004) as _;

#[unsafe(no_mangle)]
pub extern "C" fn mtime() -> u64 {
    unsafe {
        loop {
            let hi1 = MTIME_HI.read_volatile();
            let lo  = MTIME_LO.read_volatile();
            let hi2 = MTIME_HI.read_volatile();
            if hi1 == hi2 { return ((hi1 as u64) << 32) | lo as u64; }
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn set_mtimecmp(val: u64) {
    unsafe {
        MTIMECMP_HI.write_volatile(0xFFFF_FFFF);
        MTIMECMP_LO.write_volatile(val as u32);
        MTIMECMP_HI.write_volatile((val >> 32) as u32);
    }
}
```

**xam/xhal/src/platform/xemu/trap.rs:**

```rust
use core::arch::global_asm;

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

type Handler = extern "C" fn(*mut TrapFrame);
static mut HANDLER: Option<Handler> = None;

#[unsafe(no_mangle)]
pub extern "C" fn init_trap(handler: Handler) {
    unsafe {
        HANDLER = Some(handler);
        core::arch::asm!("csrw mtvec, {}", in(reg) __am_trap_entry as usize);
    }
}

#[unsafe(no_mangle)]
extern "C" fn __trap_dispatch(tf: *mut TrapFrame) {
    unsafe { if let Some(h) = HANDLER { h(tf); } }
}

extern "C" { fn __am_trap_entry(); }

global_asm!(r#"
.equ XLENB, 8
.equ FRAME_SIZE, 34 * XLENB

.align 4
.globl __am_trap_entry
__am_trap_entry:
    addi sp, sp, -FRAME_SIZE

    // Save t0 FIRST — before using it as scratch
    sd   x5,  5*XLENB(sp)

    // Now use t0 freely as scratch
    sd   x0,  0*XLENB(sp)      // zero = 0
    sd   x1,  1*XLENB(sp)      // ra
    addi x5, sp, FRAME_SIZE
    sd   x5,  2*XLENB(sp)      // sp = original sp
    sd   x3,  3*XLENB(sp)      // gp
    sd   x4,  4*XLENB(sp)      // tp
    // x5 already saved at slot 5
    sd   x6,  6*XLENB(sp)      // t1
    sd   x7,  7*XLENB(sp)      // t2
    sd   x8,  8*XLENB(sp)      // s0
    sd   x9,  9*XLENB(sp)      // s1
    sd   x10, 10*XLENB(sp)     // a0
    sd   x11, 11*XLENB(sp)     // a1
    sd   x12, 12*XLENB(sp)     // a2
    sd   x13, 13*XLENB(sp)     // a3
    sd   x14, 14*XLENB(sp)     // a4
    sd   x15, 15*XLENB(sp)     // a5
    sd   x16, 16*XLENB(sp)     // a6
    sd   x17, 17*XLENB(sp)     // a7
    sd   x18, 18*XLENB(sp)     // s2
    sd   x19, 19*XLENB(sp)     // s3
    sd   x20, 20*XLENB(sp)     // s4
    sd   x21, 21*XLENB(sp)     // s5
    sd   x22, 22*XLENB(sp)     // s6
    sd   x23, 23*XLENB(sp)     // s7
    sd   x24, 24*XLENB(sp)     // s8
    sd   x25, 25*XLENB(sp)     // s9
    sd   x26, 26*XLENB(sp)     // s10
    sd   x27, 27*XLENB(sp)     // s11
    sd   x28, 28*XLENB(sp)     // t3
    sd   x29, 29*XLENB(sp)     // t4
    sd   x30, 30*XLENB(sp)     // t5
    sd   x31, 31*XLENB(sp)     // t6

    csrr x5, mepc
    sd   x5, 32*XLENB(sp)      // mepc
    csrr x5, mcause
    sd   x5, 33*XLENB(sp)      // mcause

    mv   a0, sp
    call __trap_dispatch

    ld   x5, 32*XLENB(sp)
    csrw mepc, x5

    ld   x1,  1*XLENB(sp)      // ra
    // skip x2/sp — restored by addi below
    ld   x3,  3*XLENB(sp)      // gp
    ld   x4,  4*XLENB(sp)      // tp
    ld   x5,  5*XLENB(sp)      // t0 (original value)
    ld   x6,  6*XLENB(sp)      // t1
    ld   x7,  7*XLENB(sp)      // t2
    ld   x8,  8*XLENB(sp)
    ld   x9,  9*XLENB(sp)
    ld   x10, 10*XLENB(sp)
    ld   x11, 11*XLENB(sp)
    ld   x12, 12*XLENB(sp)
    ld   x13, 13*XLENB(sp)
    ld   x14, 14*XLENB(sp)
    ld   x15, 15*XLENB(sp)
    ld   x16, 16*XLENB(sp)
    ld   x17, 17*XLENB(sp)
    ld   x18, 18*XLENB(sp)
    ld   x19, 19*XLENB(sp)
    ld   x20, 20*XLENB(sp)
    ld   x21, 21*XLENB(sp)
    ld   x22, 22*XLENB(sp)
    ld   x23, 23*XLENB(sp)
    ld   x24, 24*XLENB(sp)
    ld   x25, 25*XLENB(sp)
    ld   x26, 26*XLENB(sp)
    ld   x27, 27*XLENB(sp)
    ld   x28, 28*XLENB(sp)
    ld   x29, 29*XLENB(sp)
    ld   x30, 30*XLENB(sp)
    ld   x31, 31*XLENB(sp)

    addi sp, sp, FRAME_SIZE
    mret
"#);
```

**xam/xhal/src/platform/xemu/mod.rs** (updated):

```rust
mod boot;
pub mod console;
pub mod misc;
pub mod timer;
pub mod trap;

unsafe extern "C" { fn main() -> i32; }

#[unsafe(no_mangle)]
pub extern "C" fn _trm_init() -> ! {
    let ret = unsafe { main() };
    self::misc::terminate(ret)
}
```

### Test Source Code

**tests/uart-putc.c:**
```c
#include "test.h"
void test_uart_putc(void) { printf("Hello from UART!\n"); }
```

**tests/timer-read.c:**
```c
#include "test.h"
void test_timer_read(void) {
    unsigned long long t1 = mtime();
    for (volatile int i = 0; i < 1000; i++);
    unsigned long long t2 = mtime();
    check(t2 > t1);
    printf("timer-read: OK (%llu -> %llu)\n", t1, t2);
}
```

**tests/csr-warl.c:**
```c
#include "test.h"
void test_csr_warl(void) {
    check((csrr(misa) >> 62) == 2);
    csrs(mstatus, 1 << 3);
    check(csrr(mstatus) & (1 << 3));
    csrc(mstatus, 1 << 3);
    check(!(csrr(mstatus) & (1 << 3)));
    csrw(mie, 0xAAA);
    check(csrr(mie) == 0xAAA);
    csrw(mtvec, ~0UL);
    check((csrr(mtvec) & 0x2) == 0);
    printf("csr-warl: OK\n");
}
```

**tests/trap-ecall.c:**
```c
#include "test.h"
static volatile int fired = 0;
static void handler(TrapFrame *tf) {
    check(tf->mcause == 11);
    tf->mepc += 4;
    fired = 1;
}
void test_trap_ecall(void) {
    init_trap((void(*)(TrapFrame*))handler);
    asm volatile ("ecall");
    check(fired);
    printf("trap-ecall: OK\n");
}
```

**tests/timer-irq.c:**
```c
#include "test.h"
static volatile int fired = 0;
static void handler(TrapFrame *tf) {
    check(tf->mcause == ((1UL << 63) | 7));
    set_mtimecmp(~0ULL);
    fired = 1;
}
void test_timer_irq(void) {
    init_trap((void(*)(TrapFrame*))handler);
    set_mtimecmp(mtime() + 1000);
    csrs(mie, 1 << 7);
    csrs(mstatus, 1 << 3);
    while (!fired);
    printf("timer-irq: OK\n");
}
```

**tests/soft-irq.c:**
```c
#include "test.h"
static volatile int fired = 0;
static void handler(TrapFrame *tf) {
    check(tf->mcause == ((1UL << 63) | 3));
    MSIP = 0;
    fired = 1;
}
void test_soft_irq(void) {
    init_trap((void(*)(TrapFrame*))handler);
    csrs(mie, 1 << 3);
    csrs(mstatus, 1 << 3);
    MSIP = 1;
    while (!fired);
    printf("soft-irq: OK\n");
}
```

**tests/plic-access.c:**
```c
#include "test.h"
void test_plic_access(void) {
    PLIC_PRI(10) = 5;    check(PLIC_PRI(10) == 5);
    PLIC_EN(0) = 1 << 10; check(PLIC_EN(0) == (1u << 10));
    PLIC_THR(0) = 3;     check(PLIC_THR(0) == 3);
    check(PLIC_CLM(0) == 0);
    printf("plic-access: OK\n");
}
```

**standalone/run_uart_putc.c:**
```c
#include "amtest.h"
int main(void) { test_uart_putc(); return 0; }
```

(Same 2-line pattern for all 7: `run_timer_read.c`, `run_timer_irq.c`, `run_soft_irq.c`, `run_plic_access.c`, `run_csr_warl.c`, `run_trap_ecall.c`.)

**src/main.c:**
```c
#include <stdio.h>
#include "amtest.h"

int main(void) {
    printf("=== am-tests [%d] ===\n", (int)NUM_TESTS);
    for (int i = 0; i < (int)NUM_TESTS; i++)
        printf("  [%d] %s\n", i, tests[i].name);
    printf("\nRunning all...\n");
    for (int i = 0; i < (int)NUM_TESTS; i++) {
        printf("--- %s ---\n", tests[i].name);
        tests[i].func();
    }
    printf("=== ALL PASSED ===\n");
    return 0;
}
```

### Makefile

```makefile
# am-tests Makefile — per-test runner (CI) + run-all binary (manual)
.PHONY: run run-menu build-optional clean

TIMEOUT  ?= 5
AM_TESTS := $(shell pwd)
RESULT   := .result

GREEN := \033[1;32m
RED   := \033[1;31m
NONE  := \033[0m

REQUIRED := $(basename $(notdir $(wildcard tests/*.c)))
OPTIONAL := $(basename $(notdir $(wildcard optional/*.c)))

# ── Per-test runner (CI-gated) ──

run:
	@> $(RESULT)
	@$(MAKE) --no-print-directory $(addprefix Run., $(REQUIRED))
	@cat $(RESULT)
	@grep -q "FAIL" $(RESULT); F=$$?; rm -f $(RESULT); test $$F -ne 0

Run.%:
	@printf "NAME = run_$*\nK = standalone/run_$*.c\nEXTRA_SRCS = tests/$*.c\n\
	INC_PATH += $(AM_TESTS)/include\ninclude $${AM_HOME}/Makefile\n" > Makefile.$*
	@timeout $(TIMEOUT) make -s -f Makefile.$* run 2>&1 > .output.$* || true
	@if grep -q "GOOD TRAP" .output.$* \
	  $(if $(filter uart-putc,$*), && grep -q "Hello from UART" .output.$*,) ; then \
		printf "[%14s] $(GREEN)PASS$(NONE)\n" $* >> $(RESULT); \
	else \
		printf "[%14s] $(RED)***FAIL***$(NONE)\n" $* >> $(RESULT); \
	fi
	@rm -f Makefile.$* .output.$*

# ── Run-all binary (manual) ──

run-menu:
	@printf "NAME = am-tests\nK = src/main.c\nEXTRA_SRCS = $(wildcard tests/*.c)\n\
	INC_PATH += $(AM_TESTS)/include\ninclude $${AM_HOME}/Makefile\n" > Makefile.menu
	@timeout 30 make -s -f Makefile.menu run 2>&1 | tee .output.menu; \
	grep -q "ALL PASSED" .output.menu && echo "run-menu: OK" || echo "run-menu: FAIL"
	@rm -f Makefile.menu .output.menu

# ── Optional compile check ──

build-optional: $(addprefix Build., $(OPTIONAL))

Build.%:
	@printf "NAME = $*\nK = optional/$*.c\nINC_PATH += $(AM_TESTS)/include\n\
	include $${AM_HOME}/Makefile\n" > Makefile.$*
	@make -s -f Makefile.$* kernel
	@rm -f Makefile.$*

clean:
	rm -rf Makefile.* build/ $(RESULT) .output.*
```

### CI

```yaml
  test-am:
    name: AM Tests
    needs: [fmt, clippy]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Setup Rust toolchain
        run: rustup target add riscv64gc-unknown-none-elf
      - name: Install cross toolchain
        run: |
          curl -fsSL -o /tmp/musl-cross.tar.xz \
            "https://github.com/cross-tools/musl-cross/releases/download/20250929/riscv64-unknown-linux-musl.tar.xz"
          tar xf /tmp/musl-cross.tar.xz -C /opt
          echo "/opt/riscv64-unknown-linux-musl/bin" >> "$GITHUB_PATH"
      - uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/registry
            ~/.cargo/git
            ~/.cargo/bin
            xemu/target
            xam/target
            xlib/build
          key: test-am-${{ runner.os }}-${{ hashFiles('**/Cargo.lock') }}
      - name: Install axconfig-gen
        run: command -v axconfig-gen || cargo install axconfig-gen
      - name: Build xemu
        working-directory: xemu
        run: cargo build
      - name: Run am-tests (required)
        working-directory: xkernels/tests/am-tests
        run: make run LOG=off
      - name: Build optional am-tests
        working-directory: xkernels/tests/am-tests
        continue-on-error: true
        run: make build-optional LOG=off
```

---

## Validation

- V-IT-1: `uart-putc` — exits 0 + "Hello from UART!" in output
- V-IT-2: `timer-read` — mtime advances, exits 0
- V-IT-3: `csr-warl` — WARL masks verified, exits 0
- V-IT-4: `trap-ecall` — ecall→handler→mepc+4→mret, exits 0
- V-IT-5: `timer-irq` — timer interrupt fires, mcause correct, exits 0
- V-IT-6: `soft-irq` — software interrupt fires, mcause correct, exits 0
- V-IT-7: `plic-access` — registers r/w, empty claim=0, exits 0
- V-IT-8: `run-menu` — all tests pass, "ALL PASSED" in output
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
| G-10 Run-all | V-IT-8 |
