# `benchmark-adaptation` PLAN `01`

> Status: Revised
> Feature: `benchmark-adaptation`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `00_MASTER.md`

---

## Summary

Revised plan with concrete code-level detail for adapting benchmarks and alu-tests to xam. Key changes from round 00: heap via linker symbols (not Rust statics), malloc/free added to xlib, explicit per-file NJU→xam migration diffs, and C++ compatibility confirmed safe.

## Log

[**Feature Introduce**]

- Concrete code diffs for every file that changes
- `malloc`/`free` added to xlib's `stdlib.h` / new `stdlib.c` (per M-002)
- Detailed NJU→xam migration table per benchmark
- C++ sub-benchmarks confirmed safe (no runtime deps)

[**Review Adjustments**]

- R-001: Heap bounds moved to linker script as `_heap_start = _ekernel; _heap_end = 0x88000000;`
- R-002: CoreMark uses `MAIN_HAS_NOARGC=1` in `core_portme.h`, no changes to `core_main.c` body
- R-003: Full NJU compatibility surface enumerated for microbench — `ROUNDUP`, `LENGTH`, `ioe_init()` all handled via local `benchmark.h` shim
- R-004: All three C++ files verified — zero C++ runtime dependencies (no `new`/`delete`, no `std::`, no exceptions, no RTTI). All use `bench_alloc()`.

[**Master Compliance**]

- M-001: Detailed code diffs provided for every changed file
- M-002: `malloc`/`free` added to xlib as bump allocator using `_heap_start`/`_heap_end`
- M-003: Per-file NJU→xam migration details with before/after code

### Changes from Previous Round

[**Added**]
- xlib `malloc`/`free` implementation (bump allocator)
- Per-file migration diffs for all benchmarks
- `MAIN_HAS_NOARGC=1` strategy for CoreMark
- Local `benchmark.h` compatibility shim for microbench

[**Changed**]
- Heap: linker symbols instead of Rust statics
- No `heap.rs` module — linker script only

[**Removed**]
- Rust `heap.rs` module (unnecessary)

[**Unresolved**]
- None

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Heap bounds as linker symbols in `linker.lds.S` |
| Review | R-002 | Accepted | CoreMark uses `MAIN_HAS_NOARGC=1`, no body changes |
| Review | R-003 | Accepted | Full compatibility surface in local `benchmark.h` |
| Review | R-004 | Accepted | C++ verified safe, all included |
| Master | M-001 | Applied | Detailed code diffs for every file |
| Master | M-002 | Applied | `malloc`/`free` added to xlib |
| Master | M-003 | Applied | Per-file NJU→xam migration table |

---

## Spec

[**Goals**]
- G-1: Add heap linker symbols to `linker.lds.S`
- G-2: Add `uptime()` to xam `timer.rs`
- G-3: Add `malloc`/`free` to xlib
- G-4: Adapt alu-tests
- G-5: Adapt coremark
- G-6: Adapt dhrystone
- G-7: Adapt microbench (all 10 benchmarks including C++ ones)

- NG-1: No xemu emulator changes
- NG-2: No VGA/keyboard/audio
- NG-3: No changes to am-tests or cpu-tests

[**Architecture**]

```
linker.lds.S
  ├─ _heap_start = _ekernel
  └─ _heap_end   = 0x88000000

xam/xhal/src/platform/xemu/timer.rs
  └─ uptime() -> u64          // mtime() / 10 → microseconds

xlib/src/stdlib.c  (NEW)
  ├─ malloc(size) -> void*    // bump allocator from _heap_start
  └─ free(ptr)                // no-op (bump allocator)

xlib/include/stdlib.h  (UPDATED)
  ├─ extern void *malloc(size_t size);
  └─ extern void free(void *ptr);

Benchmarks (source-level adaptation):
  ├─ coremark     → MAIN_HAS_NOARGC=1, uptime() timing, remove NJU headers
  ├─ dhrystone    → uptime() timing, remove NJU headers
  └─ microbench   → uptime() + heap_start/heap_end + local compat macros

alu-tests (pre-generated):
  └─ test.c       → printf-based validation, xam main() ABI
```

[**Invariants**]
- I-1: `_heap_start` == `_ekernel` (first byte after kernel image)
- I-2: `_heap_end` == `0x88000000` (CONFIG_MBASE + CONFIG_MSIZE)
- I-3: `uptime()` returns monotonically increasing microseconds
- I-4: `malloc()` returns 8-byte aligned pointers from `[_heap_start, _heap_end)`
- I-5: No NJU `am.h` / `klib.h` / `klib-macros.h` / `ioe_init` / `io_read` in adapted code
- I-6: All benchmarks use xam's `main(const char *args)` ABI (or `main(void)` via NOARGC)

[**Data Structure**]

```c
// xlib/src/stdlib.c — bump allocator state
static char *heap_brk = NULL;  // current heap break, init to &_heap_start on first malloc

extern char _heap_start[];
extern char _heap_end[];
```

[**API Surface**]

New xam HAL export:
```rust
// timer.rs
#[unsafe(no_mangle)]
pub extern "C" fn uptime() -> u64 {
    mtime() / 10
}
```

New xlib exports:
```c
// stdlib.h
void *malloc(size_t size);
void  free(void *ptr);
```

New linker symbols:
```
// linker.lds.S
_heap_start = _ekernel;
_heap_end = 0x88000000;
```

[**Constraints**]
- C-1: 128MB RAM total. Heap = RAM end − kernel image size. Microbench "huge" needs 64MB heap.
- C-2: mtime @ 10MHz → `uptime() = mtime() / 10` gives microseconds.
- C-3: `malloc` is a simple bump allocator — no `realloc`, no fragmentation handling. Sufficient for benchmarks.
- C-4: alu-tests `test.c` is ~8000 lines. Pre-generated and committed.
- C-5: Each Makefile uses `K = $(abspath .)` / `include $(AM_HOME)/Makefile` pattern.

---

## Implement

### Execution Flow

[**Main Flow**]
1. Add linker symbols → 2. Add `uptime()` → 3. Add `malloc`/`free` → 4. Adapt alu-tests → 5. Adapt coremark → 6. Adapt dhrystone → 7. Adapt microbench → 8. Verify all

[**Failure Flow**]
1. microbench "huge" exceeds heap → already handled gracefully (prints "insufficient memory")
2. C++ link failure → excluded from build (but verified safe, unlikely)

### Implementation Plan

---

#### Phase 1: Linker script — add heap symbols

**File: `xam/xhal/linker.lds.S`**

Add after `_ekernel = .;`:
```
    _heap_start = .;

    /DISCARD/ : {
        *(.comment) *(.gnu*) *(.note*) *(.eh_frame*)
    }
}
_heap_end = 0x88000000;
```

The `_heap_start` is placed at `_ekernel` (end of BSS). `_heap_end` is outside SECTIONS to avoid alignment.

---

#### Phase 2: xam HAL — add `uptime()`

**File: `xam/xhal/src/platform/xemu/timer.rs`**

Append:
```rust
/// Returns microseconds since boot.
/// xemu ACLINT mtime ticks at 10 MHz (100 ns/tick), so us = mtime / 10.
#[unsafe(no_mangle)]
pub extern "C" fn uptime() -> u64 {
    mtime() / 10
}
```

---

#### Phase 3: xlib — add `malloc`/`free`

**File: `xlib/include/stdlib.h`** (currently empty stub)

Replace with:
```c
#ifndef __STDLIB_H__
#define __STDLIB_H__

#include <stddef.h>

void *malloc(size_t size);
void  free(void *ptr);

#endif
```

**File: `xlib/src/stdlib.c`** (NEW)

```c
#include <stdlib.h>
#include <stdint.h>

extern char _heap_start[];
extern char _heap_end[];

static char *brk = NULL;

void *malloc(size_t size) {
    if (brk == NULL) {
        brk = _heap_start;
    }
    // Align to 8 bytes
    size = (size + 7) & ~(size_t)7;
    if (brk + size > _heap_end) {
        return NULL;
    }
    void *p = brk;
    brk += size;
    return p;
}

void free(void *ptr) {
    // bump allocator: no-op
    (void)ptr;
}
```

**File: `xlib/Makefile`** — verify SRCS picks up `src/stdlib.c` (should auto-glob).

---

#### Phase 4: alu-tests

**Step 1**: Generate test.c on host:
```bash
cd xkernels/tests/alu-tests
gcc gen_alu_test.c -o gen_alu_test
./gen_alu_test > tests/test.c
rm gen_alu_test
```

**Step 2**: Adapt generated `test.c` for xam:

The generator produces:
```c
#include <stdio.h>
int main(void) {
  int exit_code = 0;
  // ... thousands of test cases ...
  return exit_code;
}
```

This already works with xam because:
- xlib provides `<stdio.h>` with `printf`
- `main(void)` is compatible with xam's `main(const char *args)` calling convention (extra arg ignored)
- `return exit_code` → `_trm_init` calls `terminate(ret)` → GOOD TRAP if 0

No modifications needed to the generated code.

**Step 3**: Create `xkernels/tests/alu-tests/Makefile`:
```makefile
ARCH     ?= riscv64
PLATFORM ?= xemu
MODE     ?= release

K    = alu-test
SRCS = tests/test.c

include $(AM_HOME)/Makefile
```

**Step 4**: Create wrapper Makefile for batch runner (follows cpu-tests pattern):
```makefile
.PHONY: all run clean latest

export BATCH ?= y
export LOG   ?= off

all: latest
	@$(MAKE) -s run 2>&1 || true

run: latest
	@$(MAKE) -f Makefile.run run

Makefile.run: latest
	@printf "K = alu-test\nSRCS = tests/test.c\ninclude $${AM_HOME}/Makefile\n" > $@

clean:
	rm -rf Makefile.run build/

latest:
```

Actually — simpler: since there's only one test binary, just use the direct Makefile:

```makefile
ARCH     ?= riscv64
PLATFORM ?= xemu
MODE     ?= release

export BATCH ?= y
export LOG   ?= off

K    = alu-test
SRCS = tests/test.c

include $(AM_HOME)/Makefile
```

---

#### Phase 5: Adapt coremark

**NJU→xam migration table:**

| NJU API | Location | xam Replacement |
|---------|----------|-----------------|
| `#include <am.h>` | `core_portme.h:9` | Remove |
| `#include <klib.h>` | `core_portme.h:10` | `#include <stdio.h>` + `#include <stdint.h>` + `#include <string.h>` |
| `#include <klib-macros.h>` | `core_portme.h:11` | Remove |
| `MAIN_HAS_NOARGC 0` | `core_portme.h:150` | Change to `1` |
| `io_read(AM_TIMER_UPTIME).us` | `core_portme.c:39` | `uptime()` (returns us) |
| `ioe_init()` | `core_main.c:104` | Remove line |

**Detailed changes:**

**`include/core_portme.h`** — lines 9–11:
```c
// BEFORE:
#include <am.h>
#include <klib.h>
#include <klib-macros.h>

// AFTER:
#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
```

**`include/core_portme.h`** — line 150:
```c
// BEFORE:
#define MAIN_HAS_NOARGC 0

// AFTER:
#define MAIN_HAS_NOARGC 1
```

This makes `core_main.c` use `main(void)` (line 89), which is compatible with xam's calling convention.

**`src/core_portme.c`** — line 39:
```c
// BEFORE:
static uint32_t uptime_ms() { return io_read(AM_TIMER_UPTIME).us / 1000; }

// AFTER:
extern uint64_t uptime(void);
static uint32_t uptime_ms(void) { return (uint32_t)(uptime() / 1000); }
```

**`src/core_main.c`** — line 104:
```c
// BEFORE:
  ioe_init();

// AFTER:
  // (line removed)
```

**`Makefile`** — no changes needed (already uses `K = $(abspath .)` pattern).

---

#### Phase 6: Adapt dhrystone

**NJU→xam migration table:**

| NJU API | Location | xam Replacement |
|---------|----------|-----------------|
| `#include <am.h>` | `dry.c:353` | Remove |
| `#include <klib.h>` | `dry.c:354` | `#include <stdio.h>` + `#include <string.h>` + `#include <stdint.h>` |
| `#include <klib-macros.h>` | `dry.c:355` | Remove |
| `io_read(AM_TIMER_UPTIME).us` | `dry.c:357` | `uptime()` |
| `ioe_init()` | `dry.c:763` | Remove line |

**Detailed changes:**

**`dry.c`** — lines 353–357:
```c
// BEFORE:
#include <am.h>
#include <klib.h>
#include <klib-macros.h>

static uint32_t uptime_ms() { return io_read(AM_TIMER_UPTIME).us / 1000; }

// AFTER:
#include <stdio.h>
#include <string.h>
#include <stdint.h>

extern uint64_t uptime(void);
static uint32_t uptime_ms(void) { return (uint32_t)(uptime() / 1000); }
```

**`dry.c`** — line 763:
```c
// BEFORE:
  ioe_init();

// AFTER:
  // (line removed)
```

**`Makefile`** — no changes needed.

---

#### Phase 7: Adapt microbench

**NJU→xam migration table:**

| NJU API | Location | xam Replacement |
|---------|----------|-----------------|
| `#include <am.h>` | `benchmark.h:4` | Remove |
| `#include <klib.h>` | `benchmark.h:5` | `#include <stdio.h>` + `#include <string.h>` + `#include <stdint.h>` + `#include <stdlib.h>` |
| `#include <klib-macros.h>` | `benchmark.h:6` | Remove; add local macros |
| `#include <am.h>` | `bench.c:1` | Remove |
| `#include <klib-macros.h>` | `bench.c:4` | Remove |
| `io_read(AM_TIMER_UPTIME).us` | `bench.c:11` | `uptime()` |
| `heap.start` / `heap.end` | `bench.c:47,55,161,165` | `_heap_start` / `_heap_end` |
| `ROUNDUP(x, align)` | `bench.c:47` | Local macro |
| `LENGTH(arr)` | `bench.c:103,140` | Local macro |
| `ioe_init()` | `bench.c:94` | Remove |
| `halt(code)` | `bench.c:92` | Already provided by xam |
| `assert(cond)` | `bench.c` | Already in `<assert.h>` or define locally |

**Detailed changes:**

**`include/benchmark.h`** — lines 1–7:
```c
// BEFORE:
#ifndef __BENCHMARK_H__
#define __BENCHMARK_H__

#include <am.h>
#include <klib.h>
#include <klib-macros.h>

// AFTER:
#ifndef __BENCHMARK_H__
#define __BENCHMARK_H__

#include <stdint.h>
#include <stddef.h>
#include <stdio.h>
#include <string.h>
#include <stdlib.h>

// Compatibility macros (replacing klib-macros.h)
#define ROUNDUP(a, sz)   ((((uintptr_t)(a)) + ((sz) - 1)) & ~((sz) - 1))
#define LENGTH(arr)      (sizeof(arr) / sizeof((arr)[0]))

// Heap area (linker symbols)
extern char _heap_start[];
extern char _heap_end[];

// HAL
extern void halt(int code);
extern uint64_t uptime(void);
```

**`src/bench.c`** — header + uptime:
```c
// BEFORE:
#include <am.h>
#include <benchmark.h>
#include <limits.h>
#include <klib-macros.h>
...
static uint64_t uptime() { return io_read(AM_TIMER_UPTIME).us; }

// AFTER:
#include <benchmark.h>
#include <limits.h>
// uptime() declared in benchmark.h, provided by xam HAL
```

**`src/bench.c`** — heap references:
```c
// BEFORE (line 47):
  hbrk = (void *)ROUNDUP(heap.start, 8);
// AFTER:
  hbrk = (void *)ROUNDUP(_heap_start, 8);

// BEFORE (line 55):
  uintptr_t freesp = (uintptr_t)heap.end - (uintptr_t)heap.start;
// AFTER:
  uintptr_t freesp = (uintptr_t)_heap_end - (uintptr_t)_heap_start;

// BEFORE (line 161):
  assert((uintptr_t)heap.start <= (uintptr_t)hbrk && (uintptr_t)hbrk < (uintptr_t)heap.end);
// AFTER:
  assert((uintptr_t)_heap_start <= (uintptr_t)hbrk && (uintptr_t)hbrk < (uintptr_t)_heap_end);

// BEFORE (line 92):
    halt(1);
// AFTER (unchanged — halt() provided by xam HAL):
    halt(1);

// BEFORE (line 94):
  ioe_init();
// AFTER:
  // (line removed)
```

**`src/bench.c`** — assert: The file uses `assert()`. xlib doesn't provide `<assert.h>`. Add a minimal one:

**`xlib/include/assert.h`** (NEW):
```c
#ifndef __ASSERT_H__
#define __ASSERT_H__

extern void halt(int code);
extern int printf(const char *fmt, ...);

#define assert(cond) do { \
    if (!(cond)) { \
        printf("assert fail: %s:%d\n", __FILE__, __LINE__); \
        halt(1); \
    } \
} while (0)

#endif
```

**`Makefile`** — no changes needed.

---

### Phase 8: Verification

For each program:
```bash
cd xkernels/tests/alu-tests && make run
cd xkernels/benchmarks/coremark && make run
cd xkernels/benchmarks/dhrystone && make run
cd xkernels/benchmarks/microbench && MAINARGS=test make run
```

Expected: all print results and exit with GOOD TRAP (return 0).

---

## Trade-offs

- T-1: **Heap as linker symbols** — Adopted per R-001/TR-1. Zero-code solution.
- T-2: **Pre-generate alu-tests** — Kept per TR-2.
- T-3: **Bump allocator for malloc** — Simplest implementation. No free/realloc. Sufficient for benchmarks which either use their own allocator (microbench `bench_alloc`) or don't use malloc at all. Future programs can upgrade to a real allocator.
- T-4: **`assert.h` in xlib** — Needed by microbench. Minimal implementation using `halt()`. Could alternatively use `check()` from am-tests, but benchmarks expect standard `assert()`.

## Validation

[**Unit Tests**]
- V-UT-1: N/A — HAL additions are trivial, validated via integration.

[**Integration Tests**]
- V-IT-1: alu-tests — all arithmetic checks pass (GOOD TRAP)
- V-IT-2: coremark — runs 1000 iterations, prints score, returns 0
- V-IT-3: dhrystone — prints DMIPS, returns 0
- V-IT-4: microbench "test" — all 10 benchmarks run, prints PASS/score

[**Failure / Robustness Validation**]
- V-F-1: microbench "huge" — graceful skip for benchmarks exceeding heap
- V-F-2: malloc returns NULL when heap exhausted

[**Edge Case Validation**]
- V-E-1: alu-tests cover INT_MIN, INT_MAX, 0, -1 for all ops

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (heap symbols) | microbench reads `_heap_start`/`_heap_end` correctly |
| G-2 (uptime) | coremark/dhrystone/microbench timing works |
| G-3 (malloc/free) | malloc returns valid pointer, free is no-op |
| G-4 (alu-tests) | V-IT-1 |
| G-5 (coremark) | V-IT-2 |
| G-6 (dhrystone) | V-IT-3 |
| G-7 (microbench) | V-IT-4 |
| C-3 (bump allocator) | V-F-2 |
| C-5 (Makefile pattern) | All use K/include pattern |
