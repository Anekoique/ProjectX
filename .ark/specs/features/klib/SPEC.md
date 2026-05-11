[**Goals**]

- G-1: Provide a freestanding C library for guests built by `xam` and run on `xemu` — `mem*` / `str*` / `*printf`.
- G-2: Cover the `cpu-tests` and benchmark needs (`hello-str`, `string`, `coremark`, `microbench`, `dhrystone`).
- G-3: Keep platform-independent logic in `xlib`; the platform output sink (`_putch`) is supplied by `xam/xhal`.
- G-4: One implementation path per feature — `vsnprintf` core fans out into all `*printf` wrappers; no duplicated integer formatting.

[**Non-goals**]

- NG-1: No POSIX APIs, `FILE *` streams, or floating-point `printf`.
- NG-2: No locale support, no thread-safe allocation, no setjmp / longjmp.
- NG-3: No full hosted C environment — assume `-ffreestanding -fno-builtin`.

[**Architecture**]

```
xlib/
├── Makefile                cross-compile via riscv64-linux-musl-gcc
├── include/
│   ├── klib.h              umbrella header (pulls in string / stdio / stdlib / ctype / assert)
│   ├── klib-macros.h       LENGTH / ROUNDUP / panic / assert macros
│   ├── string.h            mem* + str* declarations
│   ├── stdio.h             *printf declarations
│   ├── stdlib.h            atoi / rand / abs
│   ├── ctype.h             isspace / isdigit / tolower / toupper
│   └── assert.h            assert macro (C/C++-safe)
└── src/
    ├── string.c            mem* + str* implementations
    ├── format.c            vsnprintf formatting engine
    └── stdio.c             *printf wrappers (call format.c::vsnprintf → _putch byte-by-byte)
```

Compiler flags: `-Wall -Werror -ffreestanding -fno-builtin -fno-stack-protector -march=rv64gc -mabi=lp64d -mcmodel=medany`.

[**Data Structure**]

```c
/* No public structs — xlib is API-surface-only. */
/* Internal format-engine context lives in src/format.c. */
```

[**API Surface**]

```c
/* string.h */
void  *memset (void *s, int c, size_t n);
void  *memcpy (void *dst, const void *src, size_t n);
void  *memmove(void *dst, const void *src, size_t n);
int    memcmp (const void *s1, const void *s2, size_t n);
size_t strlen (const char *s);
char  *strcpy (char *dst, const char *src);
char  *strncpy(char *dst, const char *src, size_t n);
char  *strcat (char *dst, const char *src);
int    strcmp (const char *s1, const char *s2);
int    strncmp(const char *s1, const char *s2, size_t n);
char  *strchr (const char *s, int c);
char  *strrchr(const char *s, int c);

/* stdio.h */
int vsnprintf(char *buf, size_t size, const char *fmt, va_list ap);
int  snprintf(char *buf, size_t size, const char *fmt, ...);
int vsprintf (char *buf,              const char *fmt, va_list ap);
int  sprintf (char *buf,              const char *fmt, ...);
int   printf (                        const char *fmt, ...);
int   putchar(int ch);
int      puts(const char *s);

/* stdlib.h, ctype.h, assert.h — small inline helpers and macros */
```

[**Constraints**]

- C-1: `format.c` owns the formatting engine; `stdio.c` is wrappers + output-path glue only — `xlib/src/format.c`, `xlib/src/stdio.c`.
- C-2: Format specifiers supported: `%d %i %u %x %X %s %c %%`. `%p %o l/ll/h` modifiers are deferred until a test demands them — `xlib/src/format.c`.
- C-3: Build flags are `-Wall -Werror -ffreestanding -fno-builtin -fno-stack-protector` plus `-O3` in release — `xlib/Makefile`.
- C-4: Output sink (`_putch`) is supplied by `xam/xhal`; `xlib` never touches MMIO directly — `xlib/src/stdio.c`.
- C-5: Public APIs follow ANSI / POSIX names (`memset`, `strcmp`, `vsnprintf`) — no invented synonyms.
- C-6: `vsnprintf` writes at most `size - 1` bytes and always NUL-terminates when `size > 0` — `xlib/src/format.c`.
- C-7: `assert.h` is C and C++ safe (no `_Static_assert` in C++ headers); `extern "C"` guards wrap function declarations for C++ inclusion — `xlib/include/assert.h`.

[**CHANGELOG**]

- `2026-05-11` `port-to-ark`: rebuilt from current code under `xlib/`. Pre-port running notes preserved at `.ark/tasks/archive/legacy/klib/SPEC_LEGACY.md`.
