# The xdb debugger

`xdb` is the xemu monitor — a REPL with GDB-flavoured commands for
breakpoints, watchpoints, memory / register inspection, and
single-stepping.

## Invoking

When you `make run` any target without `-batch` flags, xemu drops
into the xdb REPL after loading. The prompt looks like:

```
(xdb)
```

## Command reference

### Execution

| Command | Effect |
|---------|--------|
| `c` / `continue` | Run until a breakpoint, watchpoint, or program exit. |
| `s [N]` / `step [N]` | Single-step N instructions (default 1). |
| `r` / `run` | Reset and restart. |
| `q` / `quit` | Exit xdb. |

### Breakpoints

| Command | Effect |
|---------|--------|
| `b <addr>` | Set breakpoint at physical / virtual address. Stable ID returned. |
| `info b` | List breakpoints with IDs. |
| `d <id>` | Delete breakpoint by ID. |

Breakpoints are address-based. After a step-after-hit, xdb skips the
same breakpoint once to avoid refiring.

### Watchpoints

| Command | Effect |
|---------|--------|
| `w <expr>` | Watch a value — fires when the expression changes. Validated at creation. |
| `info w` | List watchpoints. |
| `d <id>` | Delete. |

Expressions can reference registers (`$a0`), dereference memory
(`*0x80000000`), arithmetic (`$sp + 8`), comparisons, and parentheses.

### Inspection

| Command | Effect |
|---------|--------|
| `x/N<f> <addr>` | GDB-style memory examine — `f` = `i` (instruction), `x` (hex word), `b` (byte). |
| `info reg [<name>]` | Dump all registers, or named GPR / CSR / `pc`. |
| `p <expr>` | Evaluate and print an expression. |

Example:

```
(xdb) x/4i $pc
(xdb) x/16x 0x80200000
(xdb) info reg a0
(xdb) p $sp - 0x10
```

### Differential testing

```
(xdb) dt attach qemu
(xdb) dt attach spike
(xdb) dt status
(xdb) dt detach
```

See [Differential testing](./difftest.md) for what's compared and how
to interpret divergences.

## Logging while inside xdb

Set `LOG=trace` (per-instruction) or `LOG=debug` (per memory / CSR
event) before `make run`. Logs interleave with REPL output but do not
interrupt command entry.
