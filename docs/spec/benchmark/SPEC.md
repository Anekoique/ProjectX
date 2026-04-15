# `benchmark` SPEC

> Source: [`/docs/archived/feat/benchmark/02_PLAN.md`](/docs/archived/feat/benchmark/02_PLAN.md).
> Iteration history, trade-off analysis, and implementation
> plan live under `docs/archived/feat/benchmark/`.

---


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
