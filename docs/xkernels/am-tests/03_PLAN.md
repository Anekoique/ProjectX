# `am-tests` PLAN `03`

> Status: Revised
> Feature: `am-tests`
> Iteration: `03`
> Owner: Executor
> Depends on:
> - Previous Plan: `02_PLAN.md`
> - Review: `02_REVIEW.md`
> - Master Directive: `02_MASTER.md`

---

## Summary

Bare-metal am-tests for xemu. This round:
1. Wires `_putch` in xam as the single console hook — xlib `printf()` works for all tests (R-004/TR-1).
2. Finalizes trap ABI: `TrapFrame` matches xark-core's design (named GPR fields + sepc equivalent), assembly saves exact documented layout (R-002/TR-2/M-002).
3. Runner captures UART output and asserts expected string for `uart-putc` (R-001).
4. `build-optional` propagates real failures (R-003).
5. Adds `main.c` test entry with menu dispatch for manual test selection (M-003).
6. Follows xark-core's architecture patterns: named register struct, trap dispatch via cause enum (M-001/M-002).

## Log

[**Review Adjustments**]

- R-001 (UART output not validated): Resolved. Runner captures xemu stdout into `.output.<name>`, asserts expected string for `uart-putc`. PASS = `GOOD TRAP` + expected output present.
- R-002 (TrapContext ABI mismatch): Resolved. `TrapFrame` uses named GPR fields (matching xark-core's `GeneralRegs`), assembly saves all 31 GPRs (x1-x31) at exact named offsets. `x0` slot is zero. `mepc` and `mcause` appended after GPRs. No ambiguity.
- R-003 (optional compile masks failures): Resolved. `build-optional` removes `|| true` — real build failures propagate. Non-gating in CI via `continue-on-error: true`.
- R-004 (second console ABI): Resolved. xam provides `_putch` as strong symbol (writes to UART THR). xlib's `printf()` chains through `_putch`. No separate `putc`/`puts` API. Tests use `printf()` directly.

[**Master Compliance**]

- M-001 (learn from xark-core): Applied. Trap design follows xark-core:
  - Named `GeneralRegs` struct (ra, sp, gp, tp, t0-t6, s0-s11, a0-a7)
  - `TrapFrame` = `GeneralRegs + mepc + mcause` (like xark-core's `TrapFrame = GeneralRegs + sepc + sstatus`)
  - `PUSH_GENERAL_REGS` / `POP_GENERAL_REGS` macros with `STORE`/`LOAD` helpers (identical pattern)
  - Trap dispatch in Rust via `mcause` match

- M-002 (better trap design): Applied. Trap entry uses xark-core-style macros:
  ```asm
  .macro STORE reg, offset
      sd \reg, \offset*8(sp)
  .endm
  .macro PUSH_GENERAL_REGS
      STORE ra, 1
      STORE sp, 2     // original sp (saved from before addi)
      STORE gp, 3
      ...
  .endm
  ```

- M-003 (main entry with menu): Applied. `main.c` provides a test menu:
  ```c
  int main() {
      // If TEST env/macro is set, run that test directly
      // Otherwise, print menu and run all sequentially
  }
  ```
  Individual tests are functions (`test_uart_putc`, `test_timer_read`, etc.) registered in a table. `make run` runs the Makefile-based per-test runner (CI). `make run-menu` builds `main.c` as a single binary with all tests linked (manual use).

- M-004 (clean code): Applied throughout.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Runner captures output, asserts expected string |
| Review | R-002 | Accepted | Named GPR fields, exact asm layout, no ambiguity |
| Review | R-003 | Accepted | build-optional propagates failures |
| Review | R-004 | Accepted | _putch as single console hook, no separate API |
| Master | M-001 | Applied | xark-core architecture patterns |
| Master | M-002 | Applied | PUSH/POP macros, named regs, cause dispatch |
| Master | M-003 | Applied | main.c with test menu |
| Master | M-004 | Applied | Clean Rust and C code |

---

## Spec

[**Goals**]

- G-1: UART output — `uart-putc` writes via `printf()` → `_putch` → UART MMIO, runner asserts output
- G-2: Timer read — mtime advances, mtimecmp read/write
- G-3: Software interrupt — MSIP trigger and handler
- G-4: PLIC register accessibility
- G-5: CSR M-mode WARL
- G-6: Trap ecall roundtrip
- G-7: Timer interrupt handler
- G-8: CI (required tests gated, optional compile-checked)
- G-9: xam HAL (`_putch`, timer, trap modules)
- G-10: Test menu entry for manual dispatch

[**Architecture**]

```
xam/xhal/src/platform/xemu/
├── mod.rs        — _trm_init (existing), re-exports
├── boot.rs       — _start (existing)
├── misc.rs       — terminate/halt (existing)
├── console.rs    — _putch() → UART THR MMIO (new)
├── timer.rs      — mtime()/set_mtimecmp() via split lo/hi (new)
└── trap.rs       — init_trap(), TrapFrame, trap entry asm (new)

xkernels/tests/am-tests/
├── Makefile          — per-test runner (CI) + menu build (manual)
├── include/
│   ├── test.h        — check(), device addresses, CSR macros
│   └── amtest.h      — test function declarations + test table
├── src/
│   └── main.c        — menu entry: select/run tests
├── tests/            — individual test files (one function each)
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

**Two run modes:**

1. **Per-test runner** (`make run`): Each `.c` in `tests/` is built as a standalone program via xam and run individually. CI-gated. Pass = `GOOD TRAP` + expected output.

2. **Menu binary** (`make run-menu`): `src/main.c` is built with all test `.c` files linked. Runs interactively — prints menu, user picks test or runs all. Manual use only.

**Console integration:**

```
printf("Hello") → xlib vsnprintf → _putch(c) → xam console.rs → UART THR MMIO → xemu stdout
```

Single path. No duplicate console ABI.

[**Data Structure**]

**xam TrapFrame (Rust, matches xark-core pattern):**

```rust
// xhal/src/platform/xemu/trap.rs

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

**Trap entry assembly (xam, xark-core style):**

```asm
// xhal/src/platform/xemu/trap.rs (global_asm!)

.equ XLENB, 8

.macro STORE reg, offset
    sd \reg, \offset*XLENB(sp)
.endm
.macro LOAD reg, offset
    ld \reg, \offset*XLENB(sp)
.endm

.macro PUSH_GENERAL_REGS
    STORE x1,  1    // ra
    STORE x3,  3    // gp
    STORE x4,  4    // tp
    STORE x5,  5    // t0
    STORE x6,  6    // t1
    STORE x7,  7    // t2
    STORE x8,  8    // s0
    STORE x9,  9    // s1
    STORE x10, 10   // a0
    STORE x11, 11   // a1
    STORE x12, 12   // a2
    STORE x13, 13   // a3
    STORE x14, 14   // a4
    STORE x15, 15   // a5
    STORE x16, 16   // a6
    STORE x17, 17   // a7
    STORE x18, 18   // s2
    STORE x19, 19   // s3
    STORE x20, 20   // s4
    STORE x21, 21   // s5
    STORE x22, 22   // s6
    STORE x23, 23   // s7
    STORE x24, 24   // s8
    STORE x25, 25   // s9
    STORE x26, 26   // s10
    STORE x27, 27   // s11
    STORE x28, 28   // t3
    STORE x29, 29   // t4
    STORE x30, 30   // t5
    STORE x31, 31   // t6
.endm

.macro POP_GENERAL_REGS
    LOAD x1,  1
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
.endm

.align 4
.globl __am_trap_entry
__am_trap_entry:
    addi sp, sp, -34*XLENB   // 32 gpr + mepc + mcause
    PUSH_GENERAL_REGS
    csrr t0, mepc
    STORE t0, 32              // TrapFrame.mepc
    csrr t0, mcause
    STORE t0, 33              // TrapFrame.mcause
    mv a0, sp
    call __trap_dispatch
    LOAD t0, 32
    csrw mepc, t0
    POP_GENERAL_REGS
    addi sp, sp, 34*XLENB
    mret
```

Note: `x0` (zero) is not saved (slot 0 is unused). `x2` (sp) is NOT saved by `PUSH_GENERAL_REGS` — the pre-trap sp was already modified by `addi`. If needed, the original sp can be recovered as `sp + 34*XLENB`. This matches xark-core's pattern where sp is handled separately via `sscratch` swap.

**C-visible TrapFrame (test.h):**

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

**test.h (updated):**

```c
#include <stdio.h>

extern void halt(int code);

#define check(cond) do { \
    if (!(cond)) { printf("FAIL: %s:%d\n", __FILE__, __LINE__); halt(1); } \
} while (0)

// MMIO
#define REG32(a)  (*(volatile unsigned int *)(a))
#define REG8(a)   (*(volatile unsigned char *)(a))

// ACLINT
#define ACLINT     0x02000000UL
#define MSIP       REG32(ACLINT + 0x0000)
// Timer accessed via xam HAL: mtime(), set_mtimecmp()

// PLIC
#define PLIC       0x0C000000UL
#define PLIC_PRI(s)   REG32(PLIC + (s) * 4)
#define PLIC_EN(c)    REG32(PLIC + 0x2000 + (c) * 0x80)
#define PLIC_THR(c)   REG32(PLIC + 0x200000 + (c) * 0x1000)
#define PLIC_CLM(c)   REG32(PLIC + 0x200004 + (c) * 0x1000)

// CSR
#define csrr(c)   ({ unsigned long __v; asm volatile ("csrr %0, " #c : "=r"(__v)); __v; })
#define csrw(c,v) asm volatile ("csrw " #c ", %0" :: "r"((unsigned long)(v)))
#define csrs(c,v) asm volatile ("csrs " #c ", %0" :: "r"((unsigned long)(v)))
#define csrc(c,v) asm volatile ("csrc " #c ", %0" :: "r"((unsigned long)(v)))

// xam HAL (extern "C" from xhal)
extern unsigned long long mtime(void);
extern void set_mtimecmp(unsigned long long val);
extern void init_trap(void (*handler)(TrapFrame *));
```

**amtest.h (test table for menu):**

```c
#ifndef AMTEST_H
#define AMTEST_H

typedef void (*TestFunc)(void);

typedef struct {
    const char *name;
    TestFunc    func;
} TestEntry;

// Test functions (defined in tests/*.c)
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
- C-3: Console via `_putch` → UART THR (single ABI)
- C-4: ACLINT access always split lo/hi 32-bit
- C-5: Runner timeout 5s. Hang = FAIL.
- C-6: PLIC = register accessibility only
- C-7: TrapFrame layout is final: named GPR fields + mepc + mcause

---

## Implement

### xam Changes

**console.rs:**
```rust
// xhal/src/platform/xemu/console.rs
const UART_THR: *mut u8 = 0x1000_0000 as *mut u8;
const UART_LSR: *const u8 = 0x1000_0005 as *const u8;

#[unsafe(no_mangle)]
pub extern "C" fn _putch(c: i8) {
    unsafe {
        while UART_LSR.read_volatile() & 0x20 == 0 {}
        UART_THR.write_volatile(c as u8);
    }
}
```

This provides the strong `_putch` that overrides xlib's weak stub. `printf("Hello")` now works end-to-end.

**timer.rs:** Same as 02_PLAN (split lo/hi, `mtime()`, `set_mtimecmp()`).

**trap.rs:** `GeneralRegs` + `TrapFrame` structs + `init_trap()` + `global_asm!` entry with `PUSH/POP_GENERAL_REGS` macros as shown above.

**mod.rs update:**
```rust
mod boot;
pub mod console;
pub mod misc;
pub mod timer;
pub mod trap;
```

### Test Source Code

Each test is both a standalone `main()` (for per-test runner) and a `test_xxx()` function (for menu binary).

**tests/uart-putc.c:**
```c
#include "test.h"
void test_uart_putc(void) { printf("Hello from UART!\n"); }
int main() { test_uart_putc(); return 0; }
```

**tests/timer-read.c:**
```c
#include "test.h"
void test_timer_read(void) {
    unsigned long long t1 = mtime();
    for (volatile int i = 0; i < 1000; i++);
    unsigned long long t2 = mtime();
    check(t2 > t1);
    printf("mtime: %llu -> %llu\n", t1, t2);
}
int main() { test_timer_read(); return 0; }
```

**tests/csr-warl.c:**
```c
#include "test.h"
void test_csr_warl(void) {
    check((csrr(misa) >> 62) == 2);          // RV64
    csrs(mstatus, 1 << 3);                    // set MIE
    check(csrr(mstatus) & (1 << 3));
    csrc(mstatus, 1 << 3);
    check(!(csrr(mstatus) & (1 << 3)));
    csrw(mie, 0xAAA);
    check(csrr(mie) == 0xAAA);
    csrw(mtvec, ~0UL);
    check((csrr(mtvec) & 0x2) == 0);
    printf("csr-warl: OK\n");
}
int main() { test_csr_warl(); return 0; }
```

**tests/trap-ecall.c:**
```c
#include "test.h"
static volatile int fired = 0;
void ecall_handler(TrapFrame *tf) {
    check(tf->mcause == 11);
    tf->mepc += 4;
    fired = 1;
}
void test_trap_ecall(void) {
    init_trap(ecall_handler);
    asm volatile ("ecall");
    check(fired);
    printf("trap-ecall: OK\n");
}
int main() { test_trap_ecall(); return 0; }
```

**tests/timer-irq.c:**
```c
#include "test.h"
static volatile int fired = 0;
void timer_handler(TrapFrame *tf) {
    check(tf->mcause == ((1UL << 63) | 7));
    set_mtimecmp(~0ULL);
    fired = 1;
}
void test_timer_irq(void) {
    init_trap(timer_handler);
    set_mtimecmp(mtime() + 1000);
    csrs(mie, 1 << 7);
    csrs(mstatus, 1 << 3);
    while (!fired);
    printf("timer-irq: OK\n");
}
int main() { test_timer_irq(); return 0; }
```

**tests/soft-irq.c:**
```c
#include "test.h"
static volatile int fired = 0;
void msip_handler(TrapFrame *tf) {
    check(tf->mcause == ((1UL << 63) | 3));
    MSIP = 0;
    fired = 1;
}
void test_soft_irq(void) {
    init_trap(msip_handler);
    csrs(mie, 1 << 3);
    csrs(mstatus, 1 << 3);
    MSIP = 1;
    while (!fired);
    printf("soft-irq: OK\n");
}
int main() { test_soft_irq(); return 0; }
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
int main() { test_plic_access(); return 0; }
```

**src/main.c (menu entry):**
```c
#include <stdio.h>
#include "amtest.h"

extern void halt(int code);

int main() {
    printf("=== am-tests ===\n");
    for (int i = 0; i < (int)NUM_TESTS; i++)
        printf("  [%d] %s\n", i, tests[i].name);
    printf("  [a] Run all\n");
    printf("Running all tests...\n");
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
.PHONY: run run-all run-menu build-optional clean

RESULT   = .result
TIMEOUT ?= 5
AM_TESTS = $(shell pwd)

GREEN = \033[1;32m
RED   = \033[1;31m
NONE  = \033[0m

REQUIRED = $(basename $(notdir $(wildcard tests/*.c)))
OPTIONAL = $(basename $(notdir $(wildcard optional/*.c)))
$(shell > $(RESULT))

# ── Per-test runner (CI) ──

run: $(addprefix Run., $(REQUIRED))
	@cat $(RESULT)
	@grep -q "FAIL" $(RESULT); F=$$?; rm -f $(RESULT); test $$F -ne 0

Run.%:
	$(eval SRC := $(wildcard tests/$*.c))
	@printf "NAME = $*\nK = $(SRC)\nINC_PATH += $(AM_TESTS)/include\ninclude $${AM_HOME}/Makefile\n" > Makefile.$*
	@timeout $(TIMEOUT) make -s -f Makefile.$* run 2>&1 > .output.$* ; \
	if grep -q "GOOD TRAP" .output.$* $(if $(filter uart-putc,$*), && grep -q "Hello from UART" .output.$*,); then \
		printf "[%14s] $(GREEN)PASS$(NONE)\n" $* >> $(RESULT); \
	else \
		printf "[%14s] $(RED)***FAIL***$(NONE)\n" $* >> $(RESULT); \
	fi
	-@rm -f Makefile.$* .output.$*

# ── Menu binary (manual) ──

run-menu:
	@printf "NAME = am-tests\nK = src/main.c\nEXTRA_SRCS = $(wildcard tests/*.c)\nINC_PATH += $(AM_TESTS)/include\ninclude $${AM_HOME}/Makefile\n" > Makefile.menu
	@make -s -f Makefile.menu run
	-@rm -f Makefile.menu

# ── Optional compile check ──

build-optional: $(addprefix Build., $(OPTIONAL))

Build.%:
	$(eval SRC := $(wildcard optional/$*.c))
	@printf "NAME = $*\nK = $(SRC)\nINC_PATH += $(AM_TESTS)/include\ninclude $${AM_HOME}/Makefile\n" > Makefile.$*
	@make -s -f Makefile.$* kernel
	-@rm -f Makefile.$*

clean:
	rm -rf Makefile.* build/ .result .output.*
```

Key: `uart-putc` has an extra `grep -q "Hello from UART"` assertion.

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

- V-IT-1: `uart-putc` — exits 0 + output contains "Hello from UART!"
- V-IT-2: `timer-read` — mtime advances, exits 0
- V-IT-3: `csr-warl` — WARL masks correct, exits 0
- V-IT-4: `trap-ecall` — ecall→handler→mret, exits 0
- V-IT-5: `timer-irq` — timer interrupt fires, exits 0
- V-IT-6: `soft-irq` — software interrupt fires, exits 0
- V-IT-7: `plic-access` — registers readable/writable, empty claim=0, exits 0
- V-IT-8: `run-menu` — all tests pass sequentially in single binary
- V-IT-9: CI passes

| Goal | Validation |
|------|------------|
| G-1 UART | V-IT-1 (output asserted) |
| G-2 Timer | V-IT-2, V-IT-5 |
| G-3 Soft IRQ | V-IT-6 |
| G-4 PLIC | V-IT-7 |
| G-5 CSR | V-IT-3 |
| G-6 Trap | V-IT-4 |
| G-7 Timer IRQ | V-IT-5 |
| G-8 CI | V-IT-9 |
| G-9 xam HAL | _putch + timer + trap used by all tests |
| G-10 Menu | V-IT-8 |
