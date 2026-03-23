# xlib / klib Structure Design

## Positioning

`xlib` should be a small freestanding support library for C programs built by `xam` and run on `xemu`.

It is **not** a general-purpose libc.

Its first job is to unblock:

- C `cpu-tests` in [docs/DEV.md](/Users/anekoique/ProjectX/docs/DEV.md)
- existing `xkernels` code that already includes `klib.h` and `klib-macros.h`

So the right design target is:

- minimal
- deterministic
- freestanding
- compatible with the existing `xkernels` tree

## What The Current Plan Misses

The current [PLAN.md](/Users/anekoique/ProjectX/docs/klib/PLAN.md) is directionally right, but three points need to be corrected before implementation:

1. `xkernels` does not only use standard headers.
   Existing benchmarks already include `klib.h` and `klib-macros.h`, so these must be first-class headers, not optional compatibility extras.

2. `printf` needs an explicit platform boundary.
   `xhal` currently exports `halt()`, but there is no C console output hook yet. `sprintf` can be implemented immediately; `printf` should be layered on top of a small output hook added later.

3. `-nostdinc` is too early.
   Current code already includes headers like `<limits.h>`. Turning on `-nostdinc` before `xlib` provides a complete enough header set will create unnecessary breakage. For the first implementation stage, header precedence is enough.

## Design Goals

- Keep the public surface small and obvious.
- Match the needs of `cpu-tests` first, then benchmarks.
- Keep platform-independent logic in `xlib`; keep platform I/O policy in `xhal`.
- Avoid duplicated state or compatibility shims that fight the C standard interfaces.
- Prefer one clean implementation path per feature:
  `vsnprintf` core -> wrappers, canonical storage of string helpers, one library archive.

## Non-Goals

For the first iterations, `xlib` should not try to provide:

- POSIX APIs
- `FILE *` streams
- floating-point `printf`
- locale support
- thread-safe allocation
- a full hosted C environment

## Proposed Layout

```text
xlib/
├── Makefile
├── include/
│   ├── klib.h
│   ├── klib-macros.h
│   ├── string.h
│   ├── stdio.h
│   ├── stdlib.h
│   ├── ctype.h
│   ├── stdarg.h        # optional in phase 1, needed if we want a self-owned leaf header set
│   ├── stdbool.h       # same as above
│   ├── stddef.h        # same as above
│   └── stdint.h        # same as above
└── src/
    ├── string.c
    ├── format.c
    ├── stdio.c
    ├── stdlib.c
    └── ctype.c
```

This keeps the structure flat and readable.

- `format.c` exists to keep formatted-output logic out of `stdio.c`
- `stdio.c` contains only wrappers and output-path glue
- `klib.h` and `klib-macros.h` are the compatibility façade for existing benchmarks

## Public Header Design

### `klib.h`

`klib.h` should be an umbrella header for the freestanding subset that `xkernels` actually uses:

```c
#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include <stdio.h>
#include <stdlib.h>
#include <ctype.h>
```

Do not stuff extra platform policy into it.

Its role is compatibility and convenience, not becoming a second `libc`.

### `klib-macros.h`

This should stay intentionally small. Based on current tree usage, the initial set should be:

- `LENGTH(arr)`
- `MIN(a, b)`
- `MAX(a, b)`
- `ROUNDUP(x, a)`
- `ROUNDDOWN(x, a)`
- `assert(cond)`

That is enough for current `microbench` usage.

Avoid turning this file into a generic macro dump.

## Source File Responsibilities

### `string.c`

Own all byte/string primitives:

- `memset`, `memcpy`, `memmove`, `memcmp`
- `strlen`
- `strcpy`, `strncpy`, `strcat`
- `strcmp`, `strncmp`
- `strchr`, `strrchr`

This file should be fully self-contained and have no platform dependencies.

### `format.c`

Own the formatting engine only.

Recommended rule: one formatting core, no duplicated integer formatting logic in wrappers.

Phase 1 support should be limited to what the tree already needs:

- `%d`, `%i`
- `%u`
- `%x`, `%X`
- `%s`
- `%c`
- `%%`

Nice-to-have but not required for milestone 1:

- `%p`
- `%o`
- `l` / `ll`
- width, `0` padding, left-align

The implementation should grow only as required by tests and benchmarks, not by speculative completeness.

### `stdio.c`

Own wrappers only:

- `vsnprintf`
- `snprintf`
- `vsprintf`
- `sprintf`
- `printf`

But `printf` should be treated specially:

- `sprintf` family can land in milestone 1
- `printf` should depend on a tiny platform output hook, added only when there is an actual console path in `xhal`

Do not hardcode emulator-specific output into `xlib`.

### `stdlib.c`

Start narrow:

- `abs`
- `atoi`
- `strtol`
- `srand`
- `rand`

`malloc/free` should be deferred until there is a concrete user.

Right now:

- C `cpu-tests` do not need them
- `dhrystone` and `microbench` already use local allocators

So allocator code should not be part of the first vertical slice.

### `ctype.c`

Keep this tiny and table-free for now:

- `isspace`, `isdigit`, `isalpha`, `isalnum`
- `isupper`, `islower`, `isprint`, `isxdigit`
- `toupper`, `tolower`

## Build Integration

## Principle

`xlib` should build as one archive and be linked only for C kernels/programs.

That keeps ownership simple:

- `xhal` provides platform ABI
- `xlib` provides freestanding C helpers
- `xkernels` consumes both

## Recommended Integration

In `xam`:

- add `XLIB_HOME ?= $(abspath $(AM_HOME)/../xlib)`
- add `-I$(XLIB_HOME)/include` before existing include paths
- link `libxlib.a` after kernel objects and with `libxhal.a`

Recommended artifact model:

- `xlib/Makefile` builds `build/$(ARCH)-$(PLATFORM)/libxlib.a`
- `xam/scripts/build_c.mk` invokes that archive as a dependency

This avoids recompiling `xlib` into every single kernel object directory.

## Header Policy

For the first phase:

- do **not** enable `-nostdinc`
- do prepend `xlib/include` to the include path

Reason:

- current code uses headers like `<limits.h>`
- `xlib` does not yet provide a complete leaf-header set
- forcing full header replacement early makes the implementation larger without helping the first deliverable

Once `xlib` really owns the freestanding leaf headers it needs, `-nostdinc` can be reconsidered.

## Output Boundary

`printf` should not invent its own device model.

The clean layering is:

- `format.c`: string formatting only
- `stdio.c`: wrapper that emits chars through one platform hook
- `xhal`: defines the actual output primitive when console support exists

Until that hook exists, the project should prioritize:

- `vsnprintf`
- `snprintf`
- `vsprintf`
- `sprintf`

That is enough to pass the currently failing C `cpu-tests`.

## Recommended Milestones

### Milestone 1: Minimal Vertical Slice

Goal: pass `hello-str` and `string`.

Implement:

- `klib.h`
- `string.h`
- `stdio.h`
- `string.c`
- `format.c`
- `stdio.c` with `vsnprintf` + `sprintf` family only
- `xam` include/link integration

Do not implement yet:

- `printf`
- `malloc/free`
- `ctype`
- `-nostdinc`

### Milestone 2: Benchmark Compatibility Layer

Goal: make the existing benchmark tree compile cleanly.

Implement:

- `klib-macros.h`
- `assert`, `ROUNDUP`, `ROUNDDOWN`, `LENGTH`
- enough format support for benchmark output
- platform output hook for `printf`

### Milestone 3: Utility Expansion

Add only the pieces with a real caller:

- `stdlib.c`
- `ctype.c`
- extra format flags or length modifiers

### Milestone 4: Fully Self-Owned Headers

Only after the library surface is stable:

- add/own the freestanding leaf headers that are still missing
- decide whether `-nostdinc` is worth enabling

## What To Build First

The clean first implementation target is:

1. `string.c`
2. `format.c`
3. `sprintf` family
4. `klib.h`
5. `xam` build integration

This gives the fastest path to the actual project goal in [docs/DEV.md](/Users/anekoique/ProjectX/docs/DEV.md): C `cpu-tests` support.

Only after that should the library grow toward benchmark compatibility.
