# `Keyboard (UART PTY RX)` IMPL `01`

> Feature: `keyboard`
> Iteration: `01`
> Owner: Executor
> Approved Plan: `01_PLAN.md` (revised: stdin → PTY approach)

---

## Summary

Implemented UART serial console input via PTY (pseudo-terminal). The original
plan proposed stdin-based RX, but review (00_REVIEW R-001) identified a
fundamental conflict with the xdb debugger frontend which also consumes stdin.
The final implementation creates a PTY pair — the master fd serves as UART
TX/RX, the slave path is printed so the user attaches with `screen` in a
separate terminal.

## Completed Scope

### UART PTY Backend (`xcore/src/device/uart.rs`)

- `Uart::with_pty()` — creates PTY pair via `openpty()`, configures slave
  (raw input, `OPOST|ONLCR` output), spawns reader thread on master fd
- `TxSink` enum routes TX to stdout (default) or PTY master fd
- `\n` → `\r\n` translation on PTY TX (master doesn't do output processing)
- Slave fd kept alive in `_pty_slave` field to prevent PTY teardown
- Helper functions `open_pty()`, `spawn_pty_reader()`, `pty_write()` for
  clean separation of concerns
- Removed dead `with_tcp()` and `with_stdin()` constructors

### Bus Device Replacement (`xcore/src/device/bus.rs`)

- `Bus::replace_device(name, dev)` — swaps device in a named MMIO region
- `CPU::replace_device()` pass-through in `cpu/mod.rs`
- `Uart` re-exported from `xcore::Uart` for binary-layer access

### Build System (`Makefile`)

- `BATCH` variable replaced with `DEBUG` (controls `--features debug`)
- Default `DEBUG=y` for developer workflow (xdb REPL)
- am-tests exports `DEBUG=n` → batch execution, no REPL
- `SERIAL`/`X_SERIAL`/`X_BATCH` variables removed entirely

### Execution Modes (`xdb/src/main.rs`)

- PTY UART always created at startup (both debug and batch modes)
- `cfg!(feature = "debug")` selects REPL vs batch; `xdb_repl()` extracted
- No terminal raw mode, no stdin conflict, no signal handling needed

### am-tests Keyboard Test (`xkernels/tests/am-tests/`)

- `keyboard.c` — polls UART RBR for data, echoes characters, `q` to quit
- Wired into `main.c` as test key `k` (interactive)
- Redirect fix: `> .output.$* 2>&1` (was `2>&1 > .output.$*`)

## Deviations from Approved Plan

| Plan | Implementation | Reason |
|------|---------------|--------|
| stdin-based RX | PTY-based RX | R-001: stdin conflict with xdb REPL |
| `terminal.rs` raw mode | Removed entirely | PTY eliminates need for host terminal manipulation |
| `SERIAL` env var | Removed | PTY always active, no opt-in needed |
| `with_stdin()` constructor | Removed | Replaced by `with_pty()` |
| `ctrlc`/signal handling | Removed | No raw mode = no Ctrl-C interception needed |

## Verification

- 269 xcore unit tests pass (including new `pty_creates_working_uart`)
- 7 am-tests pass (`make run` from am-tests)
- `make run` (xemu, `DEBUG=y`) — xdb REPL works, PTY path printed
- `make run` (xemu, `DEBUG=n`) — batch execution works, clean exit
- clippy clean, fmt clean, both `--features debug` and default builds
