# `am-tests` PLAN `00`

> Status: Draft
> Feature: `am-tests`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

Add bare-metal validation tests (`am-tests`) to xkernels that exercise the xemu CSR subsystem, ACLINT, PLIC, and UART through the xam abstract machine layer. Inspired by KXemu's `tests/am-tests`, `privileged-tests`, and `riscv/basic` test suites.

These tests run as real RV64 programs on xemu — not unit tests, but end-to-end integration tests that validate the emulator from the guest's perspective.

## Log

None (initial plan).

---

## Spec

[**Goals**]

- G-1: UART output test — write to UART MMIO, verify console output works from guest
- G-2: ACLINT timer test — set mtimecmp, spin until MTIP, verify mtime reads and timer interrupt fires
- G-3: ACLINT MSIP test — write msip register, verify M-mode software interrupt
- G-4: PLIC test — configure PLIC priority/enable/threshold, trigger source via UART IRQ, claim/complete
- G-5: CSR test — read/write mstatus, mie, mip, mtvec, mepc, mcause; verify WARL masks and privilege transitions
- G-6: Trap/interrupt framework test — install trap handler via mtvec, ecall → handler → mret, verify context save/restore
- G-7: Timer interrupt handler test — enable MTIE, set mtimecmp, take timer interrupt in handler, verify mcause

- NG-1: S-mode or U-mode tests (future — requires MMU setup)
- NG-2: Multi-hart tests
- NG-3: VGA/keyboard/disk tests (devices not implemented)

[**Architecture**]

```
xkernels/tests/am-tests/
├── Makefile              — test runner (builds each test via xam, runs on xemu)
├── include/
│   └── test.h            — check() macro, MMIO address constants
├── tests/
│   ├── uart-hello.c      — G-1: UART THR write → console output
│   ├── timer-read.c      — G-2: read mtime, verify it advances
│   ├── timer-irq.c       — G-7: mtimecmp → timer interrupt in handler
│   ├── msip.c            — G-3: write ACLINT msip, verify MSIP interrupt
│   ├── plic-claim.c      — G-4: configure PLIC, claim/complete cycle
│   ├── csr-rw.c          — G-5: CSR read/write, WARL mask validation
│   └── trap-ecall.c      — G-6: install handler, ecall, mret roundtrip
```

Each test is a standalone C program linked with xlib (printf) and xhal (boot, halt). Built via the existing `xam/Makefile` pipeline: `K=tests/uart-hello.c make run`.

**Test flow:**

1. `_start` (xhal boot.rs): set SP, call `_trm_init`
2. `_trm_init`: call `main()`
3. `main()`: run test, call `check()` assertions
4. Return 0 → `terminate(0)` → `ebreak` with a0=0 → PASS
5. `check(false)` → `halt(1)` → `ebreak` with a0=1 → FAIL

**MMIO address constants (from RVCore::new()):**

| Device | Base | Key offsets |
|--------|------|-------------|
| ACLINT | `0x0200_0000` | msip=+0x0, mtimecmp=+0x4000, mtime=+0xBFF8, setssip=+0xC000 |
| PLIC | `0x0C00_0000` | priority=+0x0, pending=+0x1000, enable=+0x2000, threshold=+0x200000, claim=+0x200004 |
| UART | `0x1000_0000` | THR/RBR=+0x0, IER=+0x1, LSR=+0x5 |

[**Invariants**]

- I-1: All tests are self-contained C programs — no inter-test dependencies
- I-2: Pass = `main()` returns 0; Fail = `check()` calls `halt(1)`
- I-3: Tests use raw MMIO volatile reads/writes for device access — no IOE abstraction layer (keep it simple)
- I-4: Tests use inline asm for CSR access (`csrr`, `csrw`, `csrrs`, `csrrc`)
- I-5: Trap handler tests install their own mtvec and use `ecall`/`mret`

[**Data Structure**]

```c
// include/test.h
#include <klib.h>

#define check(cond) do { if (!(cond)) halt(1); } while (0)

// MMIO helpers
#define MMIO_READ32(addr)       (*(volatile unsigned int *)(addr))
#define MMIO_WRITE32(addr, val) (*(volatile unsigned int *)(addr) = (val))
#define MMIO_READ8(addr)        (*(volatile unsigned char *)(addr))
#define MMIO_WRITE8(addr, val)  (*(volatile unsigned char *)(addr) = (val))

// Device bases
#define ACLINT_BASE  0x02000000UL
#define PLIC_BASE    0x0C000000UL
#define UART_BASE    0x10000000UL

// ACLINT offsets
#define ACLINT_MSIP       (ACLINT_BASE + 0x0000)
#define ACLINT_MTIMECMP   (ACLINT_BASE + 0x4000)
#define ACLINT_MTIME      (ACLINT_BASE + 0xBFF8)

// PLIC offsets
#define PLIC_PRIORITY(src)  (PLIC_BASE + (src) * 4)
#define PLIC_PENDING        (PLIC_BASE + 0x1000)
#define PLIC_ENABLE(ctx)    (PLIC_BASE + 0x2000 + (ctx) * 0x80)
#define PLIC_THRESHOLD(ctx) (PLIC_BASE + 0x200000 + (ctx) * 0x1000)
#define PLIC_CLAIM(ctx)     (PLIC_BASE + 0x200004 + (ctx) * 0x1000)

// CSR helpers
#define csrr(csr) ({ unsigned long __v; asm volatile ("csrr %0, " #csr : "=r"(__v)); __v; })
#define csrw(csr, val) asm volatile ("csrw " #csr ", %0" :: "r"(val))
#define csrs(csr, val) asm volatile ("csrs " #csr ", %0" :: "r"(val))
#define csrc(csr, val) asm volatile ("csrc " #csr ", %0" :: "r"(val))
```

[**API Surface**]

No library API — each test is a standalone `main()` using MMIO macros and CSR helpers.

[**Constraints**]

- C-1: Tests compiled with `riscv64-linux-musl-gcc`, linked via xam Makefile pipeline
- C-2: M-mode only (all tests run in machine mode)
- C-3: No OS dependencies — bare-metal, uses only xlib (printf, string) and xhal (boot, halt)
- C-4: UART output uses raw MMIO writes (not `_putch` hook — validates UART device directly)

---

## Implement

### Execution Flow

[**Main Flow**]

1. Host: `make run-am-tests` iterates over all `.c` files in `tests/`
2. For each test: `make -f Makefile.<name> run` → xam build → `xemu run FILE=out.bin BATCH=y`
3. xemu loads binary at 0x80000000, runs until `ebreak`
4. Exit code 0 = PASS, non-zero = FAIL

[**Failure Flow**]

1. `check(false)` → `halt(1)` → `ebreak` with a0=1 → xemu reports BAD TRAP → FAIL
2. Infinite loop / hang → xemu timeout (if configured) or manual kill

### Implementation Plan

[**Step 0: Infrastructure**]

- `xkernels/tests/am-tests/include/test.h` — MMIO macros, CSR helpers, check() macro, device addresses
- `xkernels/tests/am-tests/Makefile` — test runner (find all .c, build each via xam, run on xemu, collect results)

[**Step 1: UART test**]

- `tests/uart-hello.c` — write "Hello from UART!\n" to THR one byte at a time via MMIO. Verify LSR.THRE is set. Return 0.
- Validates: UART MMIO accessible from guest, THR write produces output, LSR readable.

[**Step 2: Timer tests**]

- `tests/timer-read.c` — read mtime twice with a busy loop in between, verify second > first. Also read mtimecmp, write new value, read back.
- `tests/timer-irq.c` — set mtvec to handler, enable MIE + MTIE, set mtimecmp to mtime + small delta, spin until handler sets a flag. Handler verifies mcause == 0x8000...0007 (M-mode timer interrupt), clears by writing mtimecmp = MAX, sets flag.

[**Step 3: MSIP test**]

- `tests/msip.c` — set mtvec to handler, enable MIE + MSIE, write 1 to ACLINT msip, spin until handler fires. Handler verifies mcause == 0x8000...0003 (M-mode software interrupt), clears msip, sets flag.

[**Step 4: CSR test**]

- `tests/csr-rw.c` — write/read mstatus (verify WARL mask), write/read mie (verify writable bits), write/read mtvec (verify alignment mask), read misa (verify RV64 + extensions), write medeleg/mideleg. Each read-back verified with check().

[**Step 5: Trap test**]

- `tests/trap-ecall.c` — set mtvec, execute `ecall`. Handler verifies mcause == 11 (EcallFromM), mepc == ecall instruction address, increments mepc by 4, returns via mret. After ecall, verify execution continues at next instruction.

[**Step 6: PLIC test**]

- `tests/plic-claim.c` — configure PLIC: set priority for source 10, enable source 10 for ctx 0, set threshold 0. Push a byte into UART rx (requires xemu UART input — may need to inject via test setup). Alternatively, test PLIC register read/write without triggering an actual device interrupt (register-level validation).

---

## Trade-offs

- T-1: **Raw MMIO vs IOE abstraction** — using raw volatile MMIO access instead of an IOE dispatch layer. Simpler, directly validates device registers, but less portable. Chose raw MMIO because the goal is to validate xemu device implementation, not test an IOE layer.

- T-2: **M-mode only vs multi-privilege** — all tests run in M-mode. S/U-mode tests would require MMU page table setup. Chose M-mode for this round since Phase 3 (MMU) is tested separately.

- T-3: **PLIC end-to-end vs register-only** — a full PLIC test requires a device to assert an IRQ line. UART RX is opt-in TCP, so injecting input from the test is complex. Initial PLIC test covers register read/write and claim/complete logic without live device interrupts.

---

## Validation

[**Unit Tests**]

N/A — these ARE the tests (bare-metal integration tests).

[**Integration Tests**]

- V-IT-1: `uart-hello` — runs, produces "Hello from UART!" on stdout, exits 0
- V-IT-2: `timer-read` — mtime advances, mtimecmp read/write works, exits 0
- V-IT-3: `timer-irq` — timer interrupt fires in handler, exits 0
- V-IT-4: `msip` — software interrupt fires in handler, exits 0
- V-IT-5: `csr-rw` — CSR WARL masks correct, exits 0
- V-IT-6: `trap-ecall` — ecall → handler → mret roundtrip, exits 0
- V-IT-7: `plic-claim` — PLIC registers readable/writable, exits 0

[**Acceptance Mapping**]

| Goal | Validation |
|------|------------|
| G-1 UART | V-IT-1 |
| G-2 ACLINT timer | V-IT-2, V-IT-3 |
| G-3 ACLINT MSIP | V-IT-4 |
| G-4 PLIC | V-IT-7 |
| G-5 CSR | V-IT-5 |
| G-6 Trap | V-IT-6 |
| G-7 Timer IRQ | V-IT-3 |
