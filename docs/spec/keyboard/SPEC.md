# `keyboard` SPEC

> Source: [`/docs/archived/feat/keyboard/01_PLAN.md`](/docs/archived/feat/keyboard/01_PLAN.md).
> Iteration history, trade-off analysis, and implementation
> plan live under `docs/archived/feat/keyboard/`.

---


[**Goals**]
- G-1: Batch mode UART receives stdin bytes via `rx_buf → tick() → rx_fifo → irq_line` pipeline.
- G-2: Terminal enters raw mode (with ISIG) in batch mode when stdin is a TTY.
- G-3: Terminal restored on normal exit, Ctrl-C (SIGINT), and panic. Uncatchable kill (`SIGKILL`) is out of scope.
- G-4: TX (stdout) unchanged in all modes.
- G-5: Interactive xdb mode: UART is TX-only (no stdin conflict). Guest RX available via existing TCP backend.

- NG-1: No GUI keyboard support.
- NG-2: No special key translation.

[**Architecture**]

```
Batch mode (X_BATCH=y):
  main.rs: enable_raw_mode() → set panic hook → run CPU
  ┌────────────────────┐
  │ stdin reader thread │──push──► rx_buf ──tick()──► rx_fifo ──► RBR / irq
  └────────────────────┘
  On exit: restore terminal

Interactive xdb mode (default):
  main.rs: no raw mode
  xdb REPL owns stdin for commands
  UART is TX-only (Uart::new())
  Guest RX: attach TCP backend via `dt attach` or future `serial` command
```

[**Invariants**]
- I-1: `rx_buf` is the single RX injection point regardless of backend.
- I-2: Stdin reader thread exits on EOF or process exit.
- I-3: Terminal restored under all catchable conditions (normal exit, SIGINT, panic).
- I-4: `RVCore::new()` always creates TX-only UART. Backend selection is binary-level.

[**Data Structure**]

No new structs. One new constructor on `Uart`, one terminal helper module.

```rust
impl Uart {
    pub fn with_stdin() -> Self;  // spawns stdin reader thread
}
```

[**API Surface**]

```rust
// uart.rs — new constructor
impl Uart {
    pub fn with_stdin() -> Self;
}

// bus.rs — replace a named MMIO device
impl Bus {
    pub fn replace_device(&mut self, name: &str, dev: Box<dyn Device>);
}

// terminal.rs — new module in xdb (binary layer)
pub fn enable_raw_mode() -> Option<libc::termios>;
pub fn restore(orig: &libc::termios);
```

[**Constraints**]
- C-1: Raw mode set before CPU loop starts (stdin reader sees raw bytes).
- C-2: Stdin reader silently exits on EOF/error.
- C-3: `replace_device` only used during init, before CPU loop.
- C-4: POSIX only (macOS/Linux). libc termios.

---
