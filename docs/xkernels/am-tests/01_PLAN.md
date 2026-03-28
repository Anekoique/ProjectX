# `am-tests` PLAN `01`

> Status: Revised
> Feature: `am-tests`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `00_MASTER.md`

---

## Summary

Bare-metal validation tests for xemu: UART output, ACLINT timer/msip, CSR WARL, trap ecall roundtrip, timer interrupt handler. Built via xam pipeline, run on xemu. Tests are split into `required` (validates implemented features) and `optional` (future features, run manually). CI integration via GitHub Actions.

Includes a shared trap-entry stub (`trap.S`) with explicit save/restore/mret contract, 64-bit ACLINT helpers, timeout-based runner, and PLIC scoped to register-level validation only (no host-assisted IRQ injection).

## Log

[**Feature Introduce**]

- Shared trap harness: `trap.S` saves all 32 GPRs + mepc + mcause on stack, calls C handler, restores and `mret`. Convention: `mscratch` holds trap stack pointer. All trap-based tests reuse this one entry stub.
- 64-bit ACLINT helpers: `mtime_read()`, `mtimecmp_write(val)` using lo/hi split access on RV32, direct 64-bit access on RV64.
- Runner with timeout: `make run` passes `TIMEOUT=5` to xemu (5-second wall-clock limit per test). Hang = FAIL.
- UART output capture: runner redirects xemu stdout, greps for expected string. `uart-hello` asserts exact output.
- Required vs optional tests: `make run` runs required only. `make run-all` includes optional. `make run TEST=timer-irq` runs one.
- CI job: `test-am` added to `.github/workflows/ci.yml`, depends on fmt+clippy, runs `make run` in `xkernels/tests/am-tests/`.

[**Review Adjustments**]

- R-001 (trap-entry contract): Resolved. Shared `trap.S` with full register save/restore, mepc in context, mret. All trap tests reuse it.
- R-002 (64-bit ACLINT model): Resolved. Explicit lo/hi helpers with correct offsets: `MTIME_LO=0xBFF8`, `MTIME_HI=0xBFFC`, `MTIMECMP_LO=0x4000`, `MTIMECMP_HI=0x4004`.
- R-003 (PLIC scope): Resolved. PLIC narrowed to register-level validation. No host-assisted IRQ injection. End-to-end UART→PLIC→mip deferred.
- R-004 (output/hang automation): Resolved. Runner captures stdout, greps expected output. Timeout per test.
- R-005 (CSR scope): Resolved. G-5 narrowed to "M-mode CSR WARL semantics" — no privilege transition claims.

[**Master Compliance**]

- M-001 (learn from nemu am-kernels): Applied. Studied KXemu tests/riscv/basic (mtimer, msi, smode, iuart) and ysyx abstract-machine CTE. Trap harness modeled after KXemu's `trap.S`. Timer programming modeled after KXemu's `mtimer.S`.
- M-002 (optional tests for future): Applied. Added `optional/` directory with `smode-entry.c` (S-mode via mret), `plic-uart-irq.c` (PLIC + UART external interrupt), `vm-basic.c` (Sv39 page table). These compile but are expected to fail until corresponding features are verified/extended. Run with `make run-all`.
- M-003 (CI for am-tests): Applied. New `test-am` job in `.github/workflows/ci.yml`.

### Changes from Previous Round

[**Added**]
- `trap.S` shared trap entry stub with register save/restore and mret
- `test.h` 64-bit ACLINT helpers (`mtime_read`, `mtimecmp_write`)
- Runner timeout, output capture, per-test selection
- Optional test directory with future-scope tests
- CI job `test-am`

[**Changed**]
- G-4 PLIC: end-to-end UART IRQ → register-only validation. Why: R-003.
- G-5 CSR: "privilege transitions" → "M-mode WARL semantics". Why: R-005.
- ACLINT macros: single 32-bit helpers → explicit lo/hi 64-bit helpers. Why: R-002.

[**Removed**]
- Host-assisted UART IRQ injection from PLIC test scope. Why: R-003.
- Privilege transition claims from CSR test. Why: R-005.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Shared trap.S with save/restore/mret |
| Review | R-002 | Accepted | 64-bit lo/hi ACLINT helpers |
| Review | R-003 | Accepted | PLIC narrowed to register-only |
| Review | R-004 | Accepted | Timeout + output capture in runner |
| Review | R-005 | Accepted | CSR scope narrowed to M-mode WARL |
| Master | M-001 | Applied | Studied KXemu mtimer.S, trap.S, msi.c, iuart.c |
| Master | M-002 | Applied | Optional tests: smode-entry, plic-uart-irq, vm-basic |
| Master | M-003 | Applied | CI job test-am |
| Trade-off | TR-1 | Adopted | Raw MMIO + shared 64-bit helpers |
| Trade-off | TR-2 | Adopted | Register-only PLIC for this round |

---

## Spec

[**Goals**]

- G-1: UART output — write to THR via MMIO, verify output captured by runner
- G-2: ACLINT timer — read mtime (64-bit), write mtimecmp, verify timer fires
- G-3: ACLINT MSIP — write msip register, handle M-mode software interrupt
- G-4: PLIC registers — read/write priority, enable, threshold, claim/complete (no live device IRQ)
- G-5: CSR M-mode WARL — mstatus, mie, mtvec, misa read-back, WARL mask verification
- G-6: Trap ecall roundtrip — ecall → handler → mepc+4 → mret → continue
- G-7: Timer interrupt — enable MTIE, set mtimecmp, take M-mode timer interrupt
- G-8: CI integration — am-tests run automatically in GitHub Actions
- G-9: Optional tests — future-scope tests that compile but may fail (smode, plic-uart-irq, vm)

- NG-1: S-mode / U-mode validation (optional, not required to pass)
- NG-2: Multi-hart tests
- NG-3: End-to-end UART→PLIC→mip (deferred)

[**Architecture**]

```
xkernels/tests/am-tests/
├── Makefile                 — runner: build + run + timeout + output check
├── include/
│   └── test.h               — MMIO helpers, CSR macros, ACLINT 64-bit helpers, check()
├── src/
│   └── trap.S               — shared trap entry: save regs + mepc/mcause, call handler, mret
├── tests/
│   ├── uart-hello.c          — G-1
│   ├── timer-read.c          — G-2 (mtime read, mtimecmp write)
│   ├── timer-irq.c           — G-7 (timer interrupt handler)
│   ├── msip.c                — G-3
│   ├── plic-regs.c           — G-4 (register read/write)
│   ├── csr-rw.c              — G-5
│   └── trap-ecall.c          — G-6
└── optional/
    ├── smode-entry.c          — G-9: enter S-mode via mret
    ├── plic-uart-irq.c        — G-9: PLIC + UART external interrupt
    └── vm-basic.c             — G-9: Sv39 page table walk
```

[**Invariants**]

- I-1: Each test is a standalone C program. No inter-test dependencies.
- I-2: Pass = `main()` returns 0 → `halt(0)`. Fail = `check()` → `halt(1)`.
- I-3: All trap-based tests link `trap.S` and set mtvec to `__am_trap_entry`.
- I-4: Trap context: 32 GPRs + mepc + mcause saved on stack. Handler is `void trap_handler(TrapContext *)`.
- I-5: Runner enforces timeout (default 5s). Hang = FAIL.
- I-6: Optional tests may fail — they are not gated in CI.

[**Data Structure**]

```c
// include/test.h

#include <klib.h>

extern void halt(int code);
#define check(cond) do { if (!(cond)) { printf("FAIL: %s:%d\n", __FILE__, __LINE__); halt(1); } } while (0)

// ── MMIO ──
#define REG32(addr)       (*(volatile unsigned int *)(addr))
#define REG8(addr)        (*(volatile unsigned char *)(addr))
#define REG64(addr)       (*(volatile unsigned long long *)(addr))

// ── ACLINT (0x0200_0000) ──
#define ACLINT_BASE       0x02000000UL
#define ACLINT_MSIP       REG32(ACLINT_BASE + 0x0000)
#define ACLINT_MTIMECMP_LO REG32(ACLINT_BASE + 0x4000)
#define ACLINT_MTIMECMP_HI REG32(ACLINT_BASE + 0x4004)
#define ACLINT_MTIME_LO   REG32(ACLINT_BASE + 0xBFF8)
#define ACLINT_MTIME_HI   REG32(ACLINT_BASE + 0xBFFC)

#if __riscv_xlen == 64
#define ACLINT_MTIME      REG64(ACLINT_BASE + 0xBFF8)
#define ACLINT_MTIMECMP   REG64(ACLINT_BASE + 0x4000)
static inline unsigned long long mtime_read(void) { return ACLINT_MTIME; }
static inline void mtimecmp_write(unsigned long long v) { ACLINT_MTIMECMP = v; }
#else
static inline unsigned long long mtime_read(void) {
    unsigned int lo, hi1, hi2;
    do { hi1 = ACLINT_MTIME_HI; lo = ACLINT_MTIME_LO; hi2 = ACLINT_MTIME_HI; } while (hi1 != hi2);
    return ((unsigned long long)hi1 << 32) | lo;
}
static inline void mtimecmp_write(unsigned long long v) {
    ACLINT_MTIMECMP_HI = 0xFFFFFFFF; // prevent spurious interrupt
    ACLINT_MTIMECMP_LO = (unsigned int)v;
    ACLINT_MTIMECMP_HI = (unsigned int)(v >> 32);
}
#endif

// ── PLIC (0x0C00_0000) ──
#define PLIC_BASE          0x0C000000UL
#define PLIC_PRIORITY(src) REG32(PLIC_BASE + (src) * 4)
#define PLIC_PENDING       REG32(PLIC_BASE + 0x1000)
#define PLIC_ENABLE(ctx)   REG32(PLIC_BASE + 0x2000 + (ctx) * 0x80)
#define PLIC_THRESHOLD(ctx) REG32(PLIC_BASE + 0x200000 + (ctx) * 0x1000)
#define PLIC_CLAIM(ctx)    REG32(PLIC_BASE + 0x200004 + (ctx) * 0x1000)

// ── UART (0x1000_0000) ──
#define UART_BASE          0x10000000UL
#define UART_THR           REG8(UART_BASE + 0)
#define UART_LSR           REG8(UART_BASE + 5)
#define UART_LSR_THRE      (1 << 5)

static inline void uart_putc(char c) {
    while (!(UART_LSR & UART_LSR_THRE));
    UART_THR = c;
}

static inline void uart_puts(const char *s) {
    while (*s) uart_putc(*s++);
}

// ── CSR ──
#define csrr(csr) ({ unsigned long __v; asm volatile ("csrr %0, " #csr : "=r"(__v)); __v; })
#define csrw(csr, val) asm volatile ("csrw " #csr ", %0" :: "r"((unsigned long)(val)))
#define csrs(csr, val) asm volatile ("csrs " #csr ", %0" :: "r"((unsigned long)(val)))
#define csrc(csr, val) asm volatile ("csrc " #csr ", %0" :: "r"((unsigned long)(val)))

// ── Trap context (matches trap.S layout) ──
typedef struct {
    unsigned long gpr[32];  // x0-x31 (x0 slot unused)
    unsigned long mepc;
    unsigned long mcause;
} TrapContext;

// User-defined trap handler (weak default = halt)
void trap_handler(TrapContext *ctx) __attribute__((weak));
```

```asm
// src/trap.S — shared trap entry
// Convention: mscratch holds the address of a pre-allocated trap stack (or 0 = use current sp)

.globl __am_trap_entry
.align 4
__am_trap_entry:
    // Save all 32 GPRs + mepc + mcause on current stack
    addi sp, sp, -(34*8)     // 34 slots: 32 gpr + mepc + mcause (RV64)
    sd x1,  1*8(sp)
    sd x2,  2*8(sp)          // original sp (before addi)
    sd x3,  3*8(sp)
    sd x4,  4*8(sp)
    // ... x5-x31
    sd x5,  5*8(sp)
    sd x6,  6*8(sp)
    sd x7,  7*8(sp)
    sd x8,  8*8(sp)
    sd x9,  9*8(sp)
    sd x10, 10*8(sp)
    sd x11, 11*8(sp)
    sd x12, 12*8(sp)
    sd x13, 13*8(sp)
    sd x14, 14*8(sp)
    sd x15, 15*8(sp)
    sd x16, 16*8(sp)
    sd x17, 17*8(sp)
    sd x18, 18*8(sp)
    sd x19, 19*8(sp)
    sd x20, 20*8(sp)
    sd x21, 21*8(sp)
    sd x22, 22*8(sp)
    sd x23, 23*8(sp)
    sd x24, 24*8(sp)
    sd x25, 25*8(sp)
    sd x26, 26*8(sp)
    sd x27, 27*8(sp)
    sd x28, 28*8(sp)
    sd x29, 29*8(sp)
    sd x30, 30*8(sp)
    sd x31, 31*8(sp)
    csrr t0, mepc
    sd t0, 32*8(sp)
    csrr t0, mcause
    sd t0, 33*8(sp)

    mv a0, sp                // TrapContext *
    call trap_handler

    ld t0, 32*8(sp)
    csrw mepc, t0
    // Restore x1, x3-x31 (skip x2/sp — restored last)
    ld x1,  1*8(sp)
    ld x3,  3*8(sp)
    ld x4,  4*8(sp)
    ld x5,  5*8(sp)
    ld x6,  6*8(sp)
    ld x7,  7*8(sp)
    ld x8,  8*8(sp)
    ld x9,  9*8(sp)
    ld x10, 10*8(sp)
    ld x11, 11*8(sp)
    ld x12, 12*8(sp)
    ld x13, 13*8(sp)
    ld x14, 14*8(sp)
    ld x15, 15*8(sp)
    ld x16, 16*8(sp)
    ld x17, 17*8(sp)
    ld x18, 18*8(sp)
    ld x19, 19*8(sp)
    ld x20, 20*8(sp)
    ld x21, 21*8(sp)
    ld x22, 22*8(sp)
    ld x23, 23*8(sp)
    ld x24, 24*8(sp)
    ld x25, 25*8(sp)
    ld x26, 26*8(sp)
    ld x27, 27*8(sp)
    ld x28, 28*8(sp)
    ld x29, 29*8(sp)
    ld x30, 30*8(sp)
    ld x31, 31*8(sp)
    addi sp, sp, 34*8
    mret
```

[**API Surface**]

No library API. Each test is `main()` using `test.h` macros. Trap tests link `trap.S` and define `trap_handler()`.

[**Constraints**]

- C-1: RV64 M-mode only (single hart)
- C-2: Built via xam Makefile pipeline (`K=tests/xxx.c AM_HOME=... make run`)
- C-3: Trap tests link `src/trap.S` as additional source
- C-4: Runner timeout: 5 seconds default per test
- C-5: Optional tests compile but are not required to pass in CI

---

## Implement

### Implementation Plan

[**Step 0: Infrastructure**]

- `include/test.h` — all macros and helpers as shown above
- `src/trap.S` — shared trap entry stub as shown above
- `Makefile` — test runner with timeout, output capture, required/optional split

```makefile
# Makefile sketch
REQUIRED = $(basename $(notdir $(wildcard tests/*.c)))
OPTIONAL = $(basename $(notdir $(wildcard optional/*.c)))

run: $(addprefix Makefile., $(REQUIRED))
	@cat $(RESULT)
	@grep -q "FAIL" $(RESULT); F=$$?; rm $(RESULT); test $$F -ne 0

run-all: $(addprefix Makefile., $(REQUIRED) $(OPTIONAL))
	@cat $(RESULT)

Makefile.%: tests/%.c optional/%.c
	@printf "NAME = $*\nK = $<\ninclude $${AM_HOME}/Makefile\n" > $@
	@if timeout $(TIMEOUT) make -s -f $@ run 2>&1 | tee .output.$* | tail -1 | grep -q "GOOD TRAP"; then \
		printf "[%14s] $(GREEN)PASS$(NONE)\n" $* >> $(RESULT); \
	else \
		printf "[%14s] $(RED)***FAIL***$(NONE)\n" $* >> $(RESULT); \
	fi
```

[**Step 1: uart-hello.c**]

```c
#include "test.h"
int main() {
    uart_puts("Hello from UART!\n");
    return 0;
}
```

Validates: UART THR write via MMIO, LSR THRE polling, console output.

[**Step 2: timer-read.c**]

```c
#include "test.h"
int main() {
    unsigned long long t1 = mtime_read();
    for (volatile int i = 0; i < 1000; i++);
    unsigned long long t2 = mtime_read();
    check(t2 > t1);

    // Write mtimecmp and read back
    mtimecmp_write(0xDEADBEEFCAFEULL);
    #if __riscv_xlen == 64
    check(ACLINT_MTIMECMP == 0xDEADBEEFCAFEULL);
    #else
    check(ACLINT_MTIMECMP_LO == 0xBEEFCAFE);
    check(ACLINT_MTIMECMP_HI == 0xDEAD);
    #endif

    printf("mtime: %llu -> %llu (delta=%llu)\n", t1, t2, t2 - t1);
    return 0;
}
```

[**Step 3: csr-rw.c**]

```c
#include "test.h"
int main() {
    // misa: check RV64 and expected extensions
    unsigned long misa = csrr(misa);
    #if __riscv_xlen == 64
    check((misa >> 62) == 2); // MXL = 64
    #else
    check((misa >> 30) == 1); // MXL = 32
    #endif

    // mstatus: write/read, verify WARL (MIE writable, read-only bits preserved)
    unsigned long ms0 = csrr(mstatus);
    csrs(mstatus, 1 << 3); // set MIE
    check(csrr(mstatus) & (1 << 3));
    csrc(mstatus, 1 << 3); // clear MIE
    check(!(csrr(mstatus) & (1 << 3)));

    // mie: writable bits
    csrw(mie, 0xAAA);
    unsigned long mie = csrr(mie);
    check(mie == 0xAAA); // all standard interrupt bits writable

    // mtvec: alignment mask (low 2 bits = mode)
    csrw(mtvec, 0xFFFFFFFF);
    unsigned long mtvec = csrr(mtvec);
    check((mtvec & 0x2) == 0); // bit 1 reserved

    printf("misa=%lx mstatus=%lx mie=%lx mtvec=%lx\n", misa, ms0, mie, mtvec);
    return 0;
}
```

[**Step 4: trap-ecall.c**]

```c
#include "test.h"

static volatile int handler_called = 0;
static volatile unsigned long saved_mepc = 0;

void trap_handler(TrapContext *ctx) {
    check(ctx->mcause == 11); // EcallFromM
    saved_mepc = ctx->mepc;
    ctx->mepc += 4; // skip ecall
    handler_called = 1;
}

int main() {
    csrw(mtvec, __am_trap_entry); // from trap.S
    asm volatile ("ecall");
    check(handler_called);
    check(saved_mepc != 0);
    printf("trap-ecall: mepc=%lx OK\n", saved_mepc);
    return 0;
}
```

[**Step 5: timer-irq.c**]

```c
#include "test.h"

static volatile int timer_fired = 0;

void trap_handler(TrapContext *ctx) {
    unsigned long cause = ctx->mcause;
    // M-mode timer interrupt: MSB set + code 7
    check(cause == ((1UL << (__riscv_xlen - 1)) | 7));
    mtimecmp_write(0xFFFFFFFFFFFFFFFFULL); // disarm
    timer_fired = 1;
}

int main() {
    csrw(mtvec, __am_trap_entry);
    unsigned long long now = mtime_read();
    mtimecmp_write(now + 1000); // fire soon
    csrs(mie, 1 << 7);    // MTIE
    csrs(mstatus, 1 << 3); // MIE
    while (!timer_fired);
    printf("timer-irq: OK (fired at mtime~%llu)\n", mtime_read());
    return 0;
}
```

[**Step 6: msip.c**]

```c
#include "test.h"

static volatile int msip_fired = 0;

void trap_handler(TrapContext *ctx) {
    unsigned long cause = ctx->mcause;
    check(cause == ((1UL << (__riscv_xlen - 1)) | 3)); // M-mode software interrupt
    ACLINT_MSIP = 0; // clear
    msip_fired = 1;
}

int main() {
    csrw(mtvec, __am_trap_entry);
    csrs(mie, 1 << 3);    // MSIE
    csrs(mstatus, 1 << 3); // MIE
    ACLINT_MSIP = 1;       // trigger
    while (!msip_fired);
    printf("msip: OK\n");
    return 0;
}
```

[**Step 7: plic-regs.c**]

```c
#include "test.h"
int main() {
    // Priority: write and read back
    PLIC_PRIORITY(10) = 5;
    check(PLIC_PRIORITY(10) == 5);

    // Enable: set bit for source 10 in ctx 0 (M-mode)
    PLIC_ENABLE(0) = (1 << 10);
    check(PLIC_ENABLE(0) == (1 << 10));

    // Threshold: set and read
    PLIC_THRESHOLD(0) = 3;
    check(PLIC_THRESHOLD(0) == 3);

    // Claim with nothing pending: should return 0
    check(PLIC_CLAIM(0) == 0);

    printf("plic-regs: OK\n");
    return 0;
}
```

[**Step 8: Optional tests**]

`optional/smode-entry.c` — PMP open all, set MPP=S, mepc=s_entry, mret. In S-mode: ecall back to M. Validates privilege transition.

`optional/plic-uart-irq.c` — Full PLIC+UART: enable UART IER.rx, configure PLIC source 10, enable MEIE. Requires external TCP input to trigger.

`optional/vm-basic.c` — Set up Sv39 L3→L2→L1 page table, write satp, access mapped VA.

[**Step 9: CI**]

Add to `.github/workflows/ci.yml`:

```yaml
  test-am:
    name: AM Tests
    needs: [fmt, clippy]
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - name: Setup Rust toolchain
        run: rustup target add riscv64gc-unknown-none-elf
      - name: Install riscv64 musl cross toolchain
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
          restore-keys: test-am-${{ runner.os }}-
      - name: Install axconfig-gen
        run: command -v axconfig-gen || cargo install axconfig-gen
      - name: Build xemu
        working-directory: xemu
        run: cargo build
      - name: Run am-tests
        working-directory: xkernels/tests/am-tests
        run: make run LOG=off
```

---

## Trade-offs

- T-1: **Raw MMIO + shared helpers** — raw volatile access for device validation, shared `mtime_read()`/`mtimecmp_write()` for 64-bit ACLINT. Best of both: direct device testing with correct 64-bit access patterns.
- T-2: **PLIC register-only** — no host-assisted IRQ injection. Deterministic, self-contained. End-to-end UART→PLIC deferred.
- T-3: **Required vs optional** — CI gates on required tests only. Optional tests are compile-checked but failure doesn't block.

---

## Validation

[**Integration Tests**]

- V-IT-1: `uart-hello` — output "Hello from UART!\n", exits 0
- V-IT-2: `timer-read` — mtime advances, mtimecmp read/write correct, exits 0
- V-IT-3: `csr-rw` — misa/mstatus/mie/mtvec WARL verified, exits 0
- V-IT-4: `trap-ecall` — ecall → handler → mepc+4 → mret, exits 0
- V-IT-5: `timer-irq` — timer interrupt fires, mcause correct, exits 0
- V-IT-6: `msip` — software interrupt fires, mcause correct, exits 0
- V-IT-7: `plic-regs` — PLIC priority/enable/threshold/claim read/write, exits 0
- V-IT-8: CI `test-am` job passes

[**Acceptance Mapping**]

| Goal | Validation |
|------|------------|
| G-1 UART | V-IT-1 |
| G-2 ACLINT timer | V-IT-2, V-IT-5 |
| G-3 ACLINT MSIP | V-IT-6 |
| G-4 PLIC regs | V-IT-7 |
| G-5 CSR WARL | V-IT-3 |
| G-6 Trap ecall | V-IT-4 |
| G-7 Timer IRQ | V-IT-5 |
| G-8 CI | V-IT-8 |
| G-9 Optional | Compile-checked, not gated |
