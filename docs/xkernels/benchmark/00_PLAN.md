# `benchmark-adaptation` PLAN `00`

> Status: Draft
> Feature: `benchmark-adaptation`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

Adapt the existing xkernels benchmarks (coremark, dhrystone, microbench) and alu-tests to run on our xam HAL, replacing NJU AbstractMachine API dependencies with our own. This requires two HAL additions (`heap` area and `uptime()`) and source-level API migration in each benchmark.

## Log

[**Feature Introduce**]

This plan covers four deliverables:
1. **xam HAL: heap area** — Expose `_ekernel` to RAM end as a heap region, exporting C-visible `heap_start` / `heap_end` symbols.
2. **xam HAL: `uptime()`** — Returns wall-clock microseconds since boot, derived from `mtime()`.
3. **alu-tests adaptation** — Pre-generate test.c from gen_alu_test.c, build as xam C program, integrate into CI.
4. **Benchmark adaptation** — Migrate coremark, dhrystone, microbench from NJU `am.h`/`ioe_init`/`io_read` to our xam APIs.

[**Review Adjustments**]

N/A — first iteration.

[**Master Compliance**]

N/A — first iteration.

### Changes from Previous Round

N/A — first iteration.

### Response Matrix

N/A — first iteration.

---

## Spec

[**Goals**]
- G-1: Add `heap` area to xam — expose `_ekernel` (linker symbol) to end of RAM (`CONFIG_MBASE + CONFIG_MSIZE`) as heap region accessible from C via `heap_start` / `heap_end` pointers.
- G-2: Add `uptime()` to xam — returns `uint64_t` microseconds since boot. mtime ticks at 10MHz (100ns/tick), so `uptime_us = mtime() / 10`.
- G-3: Adapt alu-tests — pre-generate test.c, build with xam build system, run in CI.
- G-4: Adapt coremark — replace `io_read(AM_TIMER_UPTIME)` with `uptime()`, remove `ioe_init()`, replace `am.h`/`klib.h` includes with `stdio.h`/`string.h`/`stdint.h`.
- G-5: Adapt dhrystone — same API migration as coremark.
- G-6: Adapt microbench — same API migration plus replace `heap.start`/`heap.end` with `heap_start`/`heap_end`, replace `halt()` (already provided by xam).

- NG-1: Do NOT add VGA, keyboard, or audio — those are future DEV.md phases.
- NG-2: Do NOT change xemu emulator code — pure HAL and test/benchmark adaptation only.
- NG-3: Do NOT change the existing am-tests or cpu-tests.

[**Architecture**]

```
Linker script (linker.lds.S)
  └─ defines _ekernel symbol (already exists)

xam HAL (Rust, #[no_mangle] extern "C")
  ├─ heap_start: *const u8    ── &_ekernel
  ├─ heap_end:   *const u8    ── CONFIG_MBASE + CONFIG_MSIZE
  └─ uptime() -> u64          ── mtime() / 10

Benchmarks (C)
  ├─ coremark   → uptime() for timing
  ├─ dhrystone  → uptime() for timing
  └─ microbench → uptime() for timing + heap_start/heap_end for allocation

alu-tests (C)
  └─ pre-generated test.c → printf + return exit_code
```

[**Invariants**]
- I-1: `heap_start` always points to `_ekernel` (first byte after kernel image).
- I-2: `heap_end` always points to `CONFIG_MBASE + CONFIG_MSIZE` (end of physical RAM).
- I-3: `uptime()` returns monotonically increasing microseconds, consistent with `mtime()`.
- I-4: No NJU `am.h` / `klib.h` / `klib-macros.h` includes remain in adapted code.
- I-5: All adapted programs use `main(const char *args)` signature (xam ABI).

[**Data Structure**]

```rust
// In xam/xhal/src/platform/xemu/heap.rs
unsafe extern "C" {
    static _ekernel: u8;
}

const HEAP_END: usize = 0x8000_0000 + 0x800_0000; // CONFIG_MBASE + CONFIG_MSIZE
```

C-visible interface:
```c
// Accessible from C as:
extern char heap_start;   // = &_ekernel
extern char heap_end;     // = CONFIG_MBASE + CONFIG_MSIZE
extern uint64_t uptime(void);
extern uint64_t mtime(void);
extern void halt(int code);
extern void _putch(int8_t c);
```

[**API Surface**]

```rust
// heap.rs
#[unsafe(no_mangle)]
pub static heap_start: usize;  // set at init time

#[unsafe(no_mangle)]
pub static heap_end: usize;    // compile-time constant

// timer.rs (addition)
#[unsafe(no_mangle)]
pub extern "C" fn uptime() -> u64;  // returns mtime() / 10
```

[**Constraints**]
- C-1: xemu RAM is 128MB (0x80000000–0x87FFFFFF). Heap is whatever remains after kernel image. Microbench "huge" setting needs up to 64MB heap — may or may not fit depending on kernel size.
- C-2: mtime ticks at 10MHz (configured in xemu ACLINT). `uptime() = mtime() / 10` gives microseconds.
- C-3: alu-tests generator produces ~7000+ lines of C. Must be pre-generated and committed, not generated at build time (cross-compilation environment may lack host gcc).
- C-4: Build system must reuse existing `K`/`SRCS`/`include $(AM_HOME)/Makefile` pattern.
- C-5: Coremark uses `main(int argc, char *argv[])` but xam passes `main(const char *args)` — need signature adaptation.

---

## Implement

### Execution Flow

[**Main Flow**]
1. Add `_ekernel` heap section marker to linker script (already exists).
2. Create `heap.rs` in xam — export `heap_start` and `heap_end` as C-visible symbols.
3. Add `uptime()` to `timer.rs` — `mtime() / 10`.
4. Pre-generate alu-tests `test.c`, create Makefile, add to CI.
5. Adapt coremark: replace NJU headers/APIs, fix `main()` signature.
6. Adapt dhrystone: same as coremark.
7. Adapt microbench: same plus heap migration.
8. Verify all run on xemu via `make run`.

[**Failure Flow**]
1. If heap too small for microbench "huge" → microbench already handles this gracefully (prints "insufficient memory", skips benchmark).
2. If alu-tests too large to compile → split generated file or use smaller test set.

[**State Transition**]

N/A — no runtime state machines.

### Implementation Plan

[**Phase 1: xam HAL extensions**]
- Create `xam/xhal/src/platform/xemu/heap.rs` — export `heap_start` / `heap_end`.
- Add `uptime()` to `timer.rs`.
- Register `heap` module in `mod.rs`.
- Verify with a trivial C program that calls `uptime()` and reads `heap_start`/`heap_end`.

[**Phase 2: alu-tests**]
- Run `gen_alu_test.c` on host to produce `test.c`.
- Adapt `test.c` for xam: replace `<stdio.h>` include, add `halt()` call, use xam `main(const char *args)` signature.
- Create `xkernels/tests/alu-tests/Makefile` following cpu-tests pattern.
- Add to CI workflow.

[**Phase 3: Benchmark adaptation**]
- **coremark**: Replace `#include <am.h>/<klib.h>` with standard headers. Replace `io_read(AM_TIMER_UPTIME).us` with `uptime()`. Remove `ioe_init()`. Fix `main()` signature.
- **dhrystone**: Same pattern. Replace `uptime_ms()` implementation.
- **microbench**: Same plus replace `heap.start`/`heap.end` with `(void *)&heap_start`/`(void *)&heap_end`. Replace `halt(1)` (already provided by xam). Replace `LENGTH` macro with local definition.
- Update each Makefile if needed (should already work with `K = $(abspath .)` pattern).
- Run each benchmark on xemu, verify output.

## Trade-offs

- T-1: **heap as linker symbols vs runtime struct** — Linker symbols (`heap_start`/`heap_end`) are simpler and match the NJU `heap.start`/`heap.end` pattern closely. A runtime `Heap` struct would be more Rust-idiomatic but adds complexity for no benefit since C code needs raw pointers anyway. **Prefer linker symbols.**

- T-2: **Pre-generate alu-tests vs generate at build time** — Pre-generating commits a large generated file but is portable (no host toolchain dependency). Build-time generation is cleaner but requires host gcc. **Prefer pre-generation** per C-3.

- T-3: **`uptime()` as mtime/10 vs adding configurable frequency** — Hardcoding the 10MHz divisor is simple and correct for xemu. A configurable frequency adds complexity for a single-platform HAL. **Prefer hardcoded divisor.**

## Validation

[**Unit Tests**]
- V-UT-1: N/A — HAL functions are trivial (division, pointer export). Tested via integration.

[**Integration Tests**]
- V-IT-1: alu-tests pass — all generated arithmetic checks return exit_code 0 → GOOD TRAP.
- V-IT-2: coremark runs to completion — prints "CoreMark" results, returns 0.
- V-IT-3: dhrystone runs to completion — prints Dhrystone results, returns 0.
- V-IT-4: microbench "test" setting runs — prints benchmark results, returns 0.

[**Failure / Robustness Validation**]
- V-F-1: microbench with "huge" setting gracefully skips benchmarks exceeding available heap.

[**Edge Case Validation**]
- V-E-1: alu-tests exercise boundary values (INT_MIN, INT_MAX, 0, -1) for all arithmetic ops.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (heap) | heap_start/heap_end accessible from microbench, correct addresses |
| G-2 (uptime) | coremark/dhrystone/microbench use uptime() for timing |
| G-3 (alu-tests) | V-IT-1: all checks pass |
| G-4 (coremark) | V-IT-2: runs to completion |
| G-5 (dhrystone) | V-IT-3: runs to completion |
| G-6 (microbench) | V-IT-4: runs with "test" setting |
| C-1 (128MB RAM) | V-F-1: graceful skip for large heap |
| C-4 (build system) | All Makefiles use K/SRCS pattern |
| C-5 (main signature) | All benchmarks compile with xam ABI |
