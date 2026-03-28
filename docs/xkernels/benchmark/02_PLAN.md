# `benchmark-adaptation` PLAN `02`

> Status: Revised
> Feature: `benchmark-adaptation`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `01_MASTER.md`

---

## Summary

Resolves all three blockers from round 01: explicit `SRCS` lists in every benchmark Makefile, microbench `mainargs` plumbing via compile-time `-DMAINARGS`, and `assert()` shimmed directly inside `benchmark.h`. Defers `malloc`/`free` to a future round per TR-2. Code diffs are tightened per M-001.

## Log

[**Feature Introduce**]

- Exact `SRCS` lists for coremark (6 files), dhrystone (1 file), microbench (12 files)
- Microbench `mainargs` via `const char mainargs[] = MAINARGS;` in `bench.c` + Makefile `CFLAGS += -DMAINARGS='"..."'`
- `assert()` macro directly in `benchmark.h` (no separate `assert.h` needed)
- `malloc`/`free` deferred to future round

[**Review Adjustments**]

- R-001: Every benchmark Makefile now defines `SRCS` explicitly
- R-002: `mainargs` plumbed via am-tests pattern: `CFLAGS += -DMAINARGS`, C source defines `const char mainargs[]`
- R-003: `assert()` added to `benchmark.h` include block so all microbench sources see it

[**Master Compliance**]

- M-001 (01): Code diffs cleaned up and streamlined

### Changes from Previous Round

[**Added**]
- `SRCS` variable in all three benchmark Makefiles
- `mainargs` plumbing for microbench
- `assert()` macro in `benchmark.h`
- New `xlib/include/assert.h` for general use

[**Changed**]
- Microbench validation: `make MAINARGS=test run` → actually works through `-DMAINARGS`

[**Removed**]
- `malloc`/`free` xlib addition (deferred; responds to 00_MASTER M-002 in trade-off T-2)

[**Unresolved**]
- None

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review 01 | R-001 | Accepted | Exact `SRCS` in every Makefile |
| Review 01 | R-002 | Accepted | `mainargs` via `-DMAINARGS` + source define |
| Review 01 | R-003 | Accepted | `assert()` in `benchmark.h` |
| Review 01 | R-004 | Accepted | `malloc`/`free` deferred |
| Review 01 | TR-1 | Accepted | Target-local `mainargs` pattern |
| Review 01 | TR-2 | Accepted | Deferred; M-002 from round 00 explicitly responded to |
| Master 01 | M-001 | Applied | Diffs cleaned and streamlined |
| Master 00 | M-002 | Deferred | No current consumer; will add when a benchmark or test requires it |

---

## Spec

[**Goals**]
- G-1: Add `_heap_start` / `_heap_end` linker symbols
- G-2: Add `uptime()` to xam HAL
- G-3: Add `xlib/include/assert.h`
- G-4: Adapt alu-tests (pre-generated test.c)
- G-5: Adapt coremark
- G-6: Adapt dhrystone
- G-7: Adapt microbench (all 10 sub-benchmarks)

- NG-1: No xemu changes
- NG-2: No `malloc`/`free` this round (deferred)
- NG-3: No changes to existing am-tests or cpu-tests

[**Architecture**]

```
linker.lds.S
  ├─ _heap_start = _ekernel
  └─ _heap_end   = 0x88000000

timer.rs
  └─ uptime() -> u64   // mtime() / 10

xlib/include/assert.h   (NEW)
  └─ assert(cond) macro

Benchmark Makefiles  (each defines SRCS explicitly)
  ├─ coremark/Makefile   → SRCS = 6 .c files
  ├─ dhrystone/Makefile  → SRCS = dry.c
  └─ microbench/Makefile → SRCS = 12 files, CFLAGS += -DMAINARGS

alu-tests/Makefile → SRCS = tests/test.c
```

[**Invariants**]
- I-1: `_heap_start` == `_ekernel`
- I-2: `_heap_end` == `0x88000000`
- I-3: `uptime()` returns monotonically increasing microseconds
- I-4: No NJU headers (`am.h`, `klib.h`, `klib-macros.h`) remain in adapted code
- I-5: All benchmarks build via `SRCS` + `include $(AM_HOME)/Makefile`

[**Data Structure**]

No new types. Only linker symbols and a C function.

[**API Surface**]

```rust
// timer.rs — new export
#[unsafe(no_mangle)]
pub extern "C" fn uptime() -> u64 {
    mtime() / 10
}
```

```c
// xlib/include/assert.h — new header
#define assert(cond) do { \
    if (!(cond)) { \
        printf("assert fail: %s:%d\n", __FILE__, __LINE__); \
        halt(1); \
    } \
} while (0)
```

```
// linker.lds.S — new symbols
_heap_start = .;           // at _ekernel
_heap_end = 0x88000000;    // end of RAM
```

[**Constraints**]
- C-1: 128MB RAM. Heap = RAM_END − kernel_size.
- C-2: `mtime` @ 10MHz → `uptime() = mtime() / 10`.
- C-3: `build_c.mk` uses `notdir` on `SRCS` → no basename collisions allowed (verified: none).
- C-4: `VPATH` auto-derived from `SRCS` dirs → compiler finds sources.

---

## Implement

### Execution Flow

[**Main Flow**]
1. Linker symbols → 2. `uptime()` → 3. `assert.h` → 4. alu-tests → 5. coremark → 6. dhrystone → 7. microbench → 8. verify all

### Implementation Plan

---

#### Phase 1: Linker script

**File: `xam/xhal/linker.lds.S`**

```diff
     _ebss = .;
   }

-  _ekernel = .;
+  _ekernel = .;
+  _heap_start = .;

   /DISCARD/ : {
     *(.comment) *(.gnu*) *(.note*) *(.eh_frame*)
   }
 }
+_heap_end = 0x88000000;
```

`_heap_start` placed at `_ekernel`. `_heap_end` outside `SECTIONS` block to avoid section alignment.

---

#### Phase 2: `uptime()` in timer.rs

**File: `xam/xhal/src/platform/xemu/timer.rs`** — append:

```rust
/// Microseconds since boot. xemu mtime ticks at 10 MHz.
#[unsafe(no_mangle)]
pub extern "C" fn uptime() -> u64 {
    mtime() / 10
}
```

---

#### Phase 3: `assert.h`

**File: `xlib/include/assert.h`** (NEW):

```c
#ifndef __ASSERT_H__
#define __ASSERT_H__

extern void halt(int code);
extern int printf(const char *fmt, ...);

#define assert(cond) do {                                    \
    if (!(cond)) {                                           \
        printf("assert fail: %s:%d\n", __FILE__, __LINE__);  \
        halt(1);                                             \
    }                                                        \
} while (0)

#endif
```

---

#### Phase 4: alu-tests

**Step 1**: Generate test.c:
```bash
cd xkernels/tests/alu-tests
gcc gen_alu_test.c -o gen && ./gen > tests/test.c && rm gen
```

Generated code already works with xam:
- Uses `<stdio.h>` (provided by xlib)
- Uses `int main(void)` (compatible with xam calling convention)
- Returns `exit_code` (→ `_trm_init` → `terminate(ret)`)

**Step 2**: Create Makefile:

**File: `xkernels/tests/alu-tests/Makefile`** (NEW):

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

#### Phase 5: coremark

**File: `coremark/Makefile`**:

```makefile
ARCH     ?= riscv64
PLATFORM ?= xemu
MODE     ?= release

export BATCH ?= y
export LOG   ?= off

K    = coremark
SRCS = src/core_main.c src/core_list_join.c src/core_matrix.c \
       src/core_state.c src/core_util.c src/core_portme.c

include $(AM_HOME)/Makefile
```

**File: `coremark/include/core_portme.h`** — lines 9–11:

```diff
-#include <am.h>
-#include <klib.h>
-#include <klib-macros.h>
+#include <stdint.h>
+#include <stddef.h>
+#include <stdio.h>
+#include <string.h>
```

**File: `coremark/include/core_portme.h`** — line 150:

```diff
-#define MAIN_HAS_NOARGC 0
+#define MAIN_HAS_NOARGC 1
```

This activates the existing `main(void)` path in `core_main.c` line 89. No changes to `core_main.c` body needed.

**File: `coremark/src/core_portme.c`** — line 39:

```diff
-static uint32_t uptime_ms() { return io_read(AM_TIMER_UPTIME).us / 1000; }
+extern uint64_t uptime(void);
+static uint32_t uptime_ms(void) { return (uint32_t)(uptime() / 1000); }
```

**File: `coremark/src/core_main.c`** — line 104:

```diff
-  ioe_init();
+  /* ioe_init() removed — xam HAL needs no runtime init */
```

---

#### Phase 6: dhrystone

**File: `dhrystone/Makefile`**:

```makefile
ARCH     ?= riscv64
PLATFORM ?= xemu
MODE     ?= release

export BATCH ?= y
export LOG   ?= off

K    = dhrystone
SRCS = dry.c

include $(AM_HOME)/Makefile
```

**File: `dhrystone/dry.c`** — lines 353–357:

```diff
-#include <am.h>
-#include <klib.h>
-#include <klib-macros.h>
-
-static uint32_t uptime_ms() { return io_read(AM_TIMER_UPTIME).us / 1000; }
+#include <stdio.h>
+#include <string.h>
+#include <stdint.h>
+
+extern uint64_t uptime(void);
+static uint32_t uptime_ms(void) { return (uint32_t)(uptime() / 1000); }
```

**File: `dhrystone/dry.c`** — line 763:

```diff
-  ioe_init();
```

---

#### Phase 7: microbench

**File: `microbench/Makefile`**:

```makefile
ARCH     ?= riscv64
PLATFORM ?= xemu
MODE     ?= release

export BATCH ?= y
export LOG   ?= off

MAINARGS ?= test

K    = microbench
SRCS = src/bench.c \
       src/qsort/qsort.c src/queen/queen.c src/bf/bf.c \
       src/fib/fib.c src/sieve/sieve.c src/md5/md5.c \
       src/lzip/lzip.c src/lzip/quicklz.c \
       src/15pz/15pz.cc src/dinic/dinic.cc src/ssort/ssort.cc

CFLAGS += -DMAINARGS=\"$(MAINARGS)\"

include $(AM_HOME)/Makefile
```

**File: `microbench/include/benchmark.h`** — replace header block:

```diff
-#include <am.h>
-#include <klib.h>
-#include <klib-macros.h>
+#include <stdint.h>
+#include <stddef.h>
+#include <stdio.h>
+#include <string.h>
+#include <assert.h>
+
+// Compatibility macros
+#define ROUNDUP(a, sz)  ((((uintptr_t)(a)) + ((sz) - 1)) & ~((sz) - 1))
+#define LENGTH(arr)     (sizeof(arr) / sizeof((arr)[0]))
+
+// Heap area (linker symbols)
+extern char _heap_start[];
+extern char _heap_end[];
+
+// HAL
+extern void halt(int code);
+extern uint64_t uptime(void);
```

**File: `microbench/src/bench.c`** — replace includes and uptime:

```diff
-#include <am.h>
-#include <benchmark.h>
-#include <limits.h>
-#include <klib-macros.h>
+#include <benchmark.h>
+#include <limits.h>
```

Add `mainargs` definition at top of file (after includes):

```c
const char mainargs[] = MAINARGS;
```

Replace uptime:

```diff
-static uint64_t uptime() { return io_read(AM_TIMER_UPTIME).us; }
+// uptime() declared in benchmark.h, provided by xam HAL
```

Replace heap references:

```diff
-  hbrk = (void *)ROUNDUP(heap.start, 8);
+  hbrk = (void *)ROUNDUP(_heap_start, 8);

-  uintptr_t freesp = (uintptr_t)heap.end - (uintptr_t)heap.start;
+  uintptr_t freesp = (uintptr_t)_heap_end - (uintptr_t)_heap_start;

-  assert((uintptr_t)heap.start <= (uintptr_t)hbrk && (uintptr_t)hbrk < (uintptr_t)heap.end);
+  assert((uintptr_t)_heap_start <= (uintptr_t)hbrk && (uintptr_t)hbrk < (uintptr_t)_heap_end);
```

Remove `ioe_init()`:

```diff
-  ioe_init();
```

---

### Phase 8: Verification

```bash
# alu-tests
cd xkernels/tests/alu-tests && make run
# Expected: program runs, all checks pass, GOOD TRAP

# coremark
cd xkernels/benchmarks/coremark && make run
# Expected: prints CoreMark results, GOOD TRAP

# dhrystone
cd xkernels/benchmarks/dhrystone && make run
# Expected: prints Dhrystone results, GOOD TRAP

# microbench (test mode — fast, small heap)
cd xkernels/benchmarks/microbench && make run
# Expected: runs 10 benchmarks with "test" setting, prints results, GOOD TRAP

# microbench (ref mode — full scoring)
cd xkernels/benchmarks/microbench && make MAINARGS=ref run
# Expected: runs all benchmarks, some may skip on insufficient heap
```

---

## Trade-offs

- T-1: **`mainargs` via `-DMAINARGS` + source define** — Same pattern as am-tests. Target-local, no build system changes. Per TR-1.
- T-2: **`malloc`/`free` deferred** — Per TR-2. No current consumer in this round. Responding to 00_MASTER M-002: deferred to a dedicated xlib-enhancement round when a real consumer (e.g., xos userspace, future benchmark) requires it. The bump allocator design from 01_PLAN remains valid for that future round.
- T-3: **`assert()` in `benchmark.h` vs separate header** — We do both: new `xlib/include/assert.h` for general reuse, and `benchmark.h` includes it. This way microbench sources get it automatically, and other programs can also use `<assert.h>`.

## Validation

[**Integration Tests**]
- V-IT-1: alu-tests — all arithmetic checks pass, GOOD TRAP
- V-IT-2: coremark — 1000 iterations complete, prints score, GOOD TRAP
- V-IT-3: dhrystone — runs to completion, prints DMIPS, GOOD TRAP
- V-IT-4: microbench "test" — all 10 sub-benchmarks run, GOOD TRAP

[**Failure / Robustness Validation**]
- V-F-1: microbench "huge" — benchmarks exceeding heap print "insufficient memory" and are skipped

[**Edge Case Validation**]
- V-E-1: alu-tests exercise INT_MIN, INT_MAX, 0, -1 for all arithmetic/logic/comparison ops

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (heap symbols) | microbench reads `_heap_start`/`_heap_end` correctly |
| G-2 (uptime) | coremark/dhrystone/microbench timing works |
| G-3 (assert.h) | microbench compiles with `assert()` calls |
| G-4 (alu-tests) | V-IT-1 |
| G-5 (coremark) | V-IT-2 |
| G-6 (dhrystone) | V-IT-3 |
| G-7 (microbench) | V-IT-4 |
| C-3 (no basename collision) | Verified — all 12 sources have unique basenames |
| C-4 (VPATH) | `build_c.mk` derives VPATH from `SRCS` dirs automatically |
| I-5 (build system) | All Makefiles define `SRCS` + `include $(AM_HOME)/Makefile` |
