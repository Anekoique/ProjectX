# Bare-metal tests (am-tests)

Bare-metal kernels that exercise the HAL and the core device set
without any OS layer.

## Running

```bash
cd xkernels/tests/am-tests
make run                # run all
make run TEST=u         # UART echo
make run TEST=t         # trap + interrupt routing
make run TEST=k         # keyboard (interactive PTY echo)
make run TEST=f         # float sanity
```

## What each letter covers

| Letter | Subject |
|--------|---------|
| `u` | UART TX, THRE interrupt, DLAB divisor |
| `c` | CSR read / write / WARL masks |
| `t` | Trap delegation, `mret` / `sret`, vectored mtvec |
| `i` | Interrupt priority, MIE / SIE gating, global enable |
| `a` | ACLINT (MSWI + MTIMER + SSWI), mtimecmp, Sstc |
| `p` | PLIC claim / complete, level-trigger semantics |
| `k` | Keyboard — PTY-backed UART RX |
| `f` | F / D extension, NaN-boxing, fcsr shifted aliases |

## Exit semantics

am-tests use the SiFive test finisher at `0x0010_0000`:

- Write `0x5555` → graceful exit, status 0
- Write `(code << 16) | 0x3333` → exit with `code`

`xlib/src/stdio.c` provides the `xam_halt()` helper that wraps this.
