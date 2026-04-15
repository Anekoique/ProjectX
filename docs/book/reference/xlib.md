# xlib (klib)

Freestanding C library for programs built by `xam` and run on xemu.
Modelled after NEMU's abstract-machine klib — minimal, deterministic,
platform-independent.

See [`../../spec/klib/SPEC.md`](../../spec/klib/SPEC.md) for the
design.

## What's included

### `<string.h>` — string.c

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

### `<stdio.h>` — stdio.c + format.c

```c
int printf(const char *fmt, ...);
int sprintf(char *buf, const char *fmt, ...);
int snprintf(char *buf, size_t size, const char *fmt, ...);
int vsprintf(char *buf, const char *fmt, va_list ap);
int vsnprintf(char *buf, size_t size, const char *fmt, va_list ap);
int puts(const char *s);
int putch(char ch);
```

Format specifiers: `%d %i %u %x %X %s %c %p %o %%`, with `l` / `ll`
length modifiers, field width, `0`-padding, left-alignment.
No floating-point `printf`.

### `<assert.h>`

```c
#define assert(x) ...
```

C- and C++-compatible (carries `extern "C"` guards).

### `<stdlib.h>` — stdlib.c

```c
int     atoi(const char *s);
int     abs(int x);
void    srand(unsigned seed);
int     rand(void);
```

No `malloc` / `free` — cpu-tests don't need them, benchmarks use
local allocators.

### `<ctype.h>` — ctype.c

`isspace`, `isdigit`, `isalpha`, `isalnum`, `toupper`, `tolower`,
etc. Standard shapes.

## What's not included

- POSIX APIs, `FILE *` streams.
- Floating-point `printf`.
- Locale support.
- Thread-safe allocation.

This is intentional — xlib targets bare-metal test and benchmark
kernels, not a hosted C environment.

## Using from your kernel

```c
#include <klib.h>          /* umbrella header */
```

This pulls in `<stddef.h>`, `<stdint.h>`, `<stdbool.h>`, `<stdarg.h>`,
`<string.h>`, `<stdio.h>`, `<stdlib.h>`, `<ctype.h>`. The xam build
system prepends `-I$(XLIB_HOME)/include` before system includes.

## klib-macros.h

Convenience macros used by benchmarks:

```c
#define LENGTH(arr)        (sizeof(arr) / sizeof((arr)[0]))
#define ROUNDUP(x, n)      (((x) + (n) - 1) & ~((n) - 1))
#define ROUNDDOWN(x, n)    ((x) & ~((n) - 1))
#define MIN(a, b)          ((a) < (b) ? (a) : (b))
#define MAX(a, b)          ((a) > (b) ? (a) : (b))
```
