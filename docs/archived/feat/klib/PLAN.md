# xlib (klib) Implementation Plan

## Overview

A small freestanding C library for programs built by `xam` and run on `xemu`. Not a general-purpose libc. Modeled after NEMU's abstract-machine klib — minimal, deterministic, platform-independent.

## Motivation

- 2 cpu-tests fail (`hello-str`, `string`) — need `sprintf`, `strcmp`, `memset`, etc.
- Benchmarks (`coremark`, `microbench`, `dhrystone`) already include `klib.h` and `klib-macros.h`
- `-fno-builtin -ffreestanding` disables compiler builtins; we must provide our own

## Design Principles

- Keep the public surface small and obvious
- Match the needs of `cpu-tests` first, then benchmarks
- Keep platform-independent logic in `xlib`; keep platform I/O policy in `xhal`
- One clean implementation path per feature: `vsnprintf` core → wrappers
- Grow only as required by tests and benchmarks, not by speculative completeness

## Non-Goals

- POSIX APIs, `FILE *` streams, floating-point `printf`
- Locale support, thread-safe allocation
- A full hosted C environment

## Directory Structure

```
xlib/
├── Makefile
├── include/
│   ├── klib.h              ← umbrella header (includes all standard headers)
│   ├── klib-macros.h       ← utility macros (LENGTH, ROUNDUP, assert, etc.)
│   ├── string.h            ← mem*/str* declarations
│   ├── stdio.h             ← sprintf/snprintf declarations
│   ├── stdlib.h            ← atoi, rand, abs
│   └── ctype.h             ← character classification
└── src/
    ├── string.c            ← mem*/str* implementations
    ├── format.c            ← vsnprintf formatting engine
    ├── stdio.c             ← sprintf/printf wrappers (output-path glue)
    ├── stdlib.c            ← atoi, rand, abs
    └── ctype.c             ← isspace, isdigit, tolower, etc.
```

Key separation: `format.c` owns the formatting engine, `stdio.c` owns only wrappers and output-path glue.

## Milestones

### Milestone 1: Minimal Vertical Slice

**Goal**: pass `hello-str` and `string` cpu-tests (35/35 PASS).

#### 1a. `string.h` + `string.c`

```c
void  *memset(void *s, int c, size_t n);
void  *memcpy(void *dst, const void *src, size_t n);
void  *memmove(void *dst, const void *src, size_t n);
int    memcmp(const void *s1, const void *s2, size_t n);

size_t strlen(const char *s);
char  *strcpy(char *dst, const char *src);
char  *strncpy(char *dst, const char *src, size_t n);
char  *strcat(char *dst, const char *src);
int    strcmp(const char *s1, const char *s2);
int    strncmp(const char *s1, const char *s2, size_t n);
char  *strchr(const char *s, int c);
char  *strrchr(const char *s, int c);
```

Fully self-contained, no platform dependencies. ~80 lines.

#### 1b. `format.c` — formatting engine

Phase 1 format specifiers (what the tree already needs):
- `%d`, `%i`, `%u`, `%x`, `%X`, `%s`, `%c`, `%%`

Nice-to-have (defer unless required):
- `%p`, `%o`, `l`/`ll` length modifiers, width, `0`-padding, left-align

Single `vsnprintf` core, no duplicated integer formatting logic. ~100 lines.

#### 1c. `stdio.h` + `stdio.c` — wrappers only

```c
int vsnprintf(char *buf, size_t size, const char *fmt, va_list ap);
int snprintf(char *buf, size_t size, const char *fmt, ...);
int vsprintf(char *buf, const char *fmt, va_list ap);
int sprintf(char *buf, const char *fmt, ...);
```

`printf` is **not** in milestone 1 — it requires a platform output hook in `xhal` that doesn't exist yet.

#### 1d. `klib.h` — umbrella header

```c
#include <stddef.h>
#include <stdint.h>
#include <stdbool.h>
#include <stdarg.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <ctype.h>
```

Convenience only — no platform policy, no extra declarations.

#### 1e. Build integration

- `xlib/Makefile` builds `build/$(ARCH)-$(PLATFORM)/libxlib.a`
- `xam/scripts/build_c.mk`:
  - Add `XLIB_HOME ?= $(abspath $(AM_HOME)/../xlib)`
  - Prepend `-I$(XLIB_HOME)/include` to include path (before system includes)
  - Add `$(LIBXLIB)` to `LINKAGE`
- Do **not** enable `-nostdinc` — code still uses `<limits.h>` etc.

**Do not implement yet**: `printf`, `malloc/free`, `ctype`, `klib-macros.h`, freestanding leaf headers.

### Milestone 2: Benchmark Compatibility

**Goal**: make `coremark`, `microbench`, `dhrystone` compile cleanly.

- `klib-macros.h`: `LENGTH`, `MIN`, `MAX`, `ROUNDUP`, `ROUNDDOWN`, `assert`
- Extended format support as needed by benchmark output
- Platform output hook in `xhal` → enable `printf`

### Milestone 3: Utility Expansion

Add only pieces with a real caller:

- `stdlib.c`: `abs`, `atoi`, `strtol`, `srand`, `rand`
- `ctype.c`: `isspace`, `isdigit`, `isalpha`, `isalnum`, `toupper`, `tolower`, etc.
- Extra format flags or length modifiers

`malloc/free` deferred — cpu-tests don't need them, benchmarks use local allocators.

### Milestone 4: Fully Self-Owned Headers

Only after the library surface is stable:

- Add/own freestanding leaf headers (`stdarg.h`, `stdbool.h`, `stddef.h`, `stdint.h`)
- Decide whether `-nostdinc` is worth enabling

## Output Boundary

`printf` should not invent its own device model. The clean layering is:

```
format.c  →  string formatting only (vsnprintf)
stdio.c   →  wrappers that emit chars through one platform hook
xhal      →  defines the actual output primitive (putch) when console exists
```

Until that hook exists, only `sprintf` family ships.

## Risks

| Risk | Severity | Mitigation |
|------|----------|------------|
| `vsnprintf` edge cases | MEDIUM | Validate with `hello-str` test |
| Header conflicts with musl toolchain | LOW | Include-path ordering, no `-nostdinc` yet |
| Over-engineering | LOW | Grow only as tests/benchmarks demand |

## Validation

After milestone 1:
1. `make run` in `cpu-tests/` — all 35 tests PASS
2. `make run` in `cpu-tests-rs/` — all 31 tests still PASS (no regression)

## Implementation Order

1. `string.c` + `string.h`
2. `format.c` (vsnprintf engine)
3. `stdio.c` + `stdio.h` (wrappers)
4. `klib.h`
5. `xlib/Makefile` + `xam` build integration
6. Validate: 35/35 cpu-tests PASS
