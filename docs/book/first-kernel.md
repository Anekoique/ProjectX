# Running your first kernel

The fastest way to see xemu work is to run the **am-tests** suite —
bare-metal kernels that exercise the HAL, UART, ACLINT, PLIC, CSRs,
traps, and interrupts.

```bash
cd xkernels/tests/am-tests
make run
```

You should see UART output, a summary line per sub-test, and a clean
SiFive test-finisher exit.

## Running a single am-test

```bash
cd xkernels/tests/am-tests
make run TEST=u    # UART echo
make run TEST=c    # CSR sanity
make run TEST=t    # trap + interrupt routing
make run TEST=k    # keyboard — interactive PTY echo
```

See `xkernels/tests/am-tests/src/tests/` for the full set.

## Running the cpu-tests

Two parallel suites — Rust (`cpu-tests-rs`, 31 tests) and C
(`cpu-tests-c`, 35 tests):

```bash
cd xkernels/tests/cpu-tests-rs && make run
cd xkernels/tests/cpu-tests-c  && make run
```

Both are bare-metal — no OS, just instruction-sequence fixtures.

## Troubleshooting

**"cannot find riscv64-unknown-linux-musl-gcc"** — install the
cross-compiler and export its `bin/` on your `PATH`. See
[Building xemu](./building.md).

**No UART output** — check you're not in `DEBUG=y` mode unintentionally
(it routes UART to a PTY, requiring `screen` to attach). For plain
stdio, use `DEBUG=n` or omit the flag.

**Test hangs** — hit `Ctrl-A X` to exit. xemu intercepts the same
escape sequence QEMU uses.

## What to read next

- [The xdb debugger](./usage/debugger.md) — step, break, examine
  memory / registers.
- [Boot targets](./usage/boot-targets.md) — run xv6, Linux, Debian.
- [Architecture overview](./internals/architecture.md) — how xemu is
  built internally.
