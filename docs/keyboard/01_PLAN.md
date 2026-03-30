# `Keyboard (UART Stdin RX)` PLAN `01`

> Status: Approved for Implementation
> Feature: `keyboard`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `00_MASTER.md`

---

## Summary

Add stdin-based UART RX so the emulator receives keyboard input after an OS boots.
The stdin backend activates only in **batch mode** (`X_BATCH=y`), which runs the CPU without the xdb REPL — no stdin contention. Interactive xdb mode keeps TX-only UART; guest RX in xdb is served by the existing TCP backend.

Terminal raw mode is set in the binary layer (`xdb/src/main.rs`), not in xcore.
`RVCore::new()` remains unchanged; the binary injects the UART backend via a new `RVCore::with_uart()` or post-construction bus accessor.

## Log

[**Review Adjustments**]

Addressed all 4 findings from `00_REVIEW.md`.

[**Master Compliance**]

Master approved with directive to fix reviewer problems and keep code clean/concise/elegant.

### Changes from Previous Round

[**Added**]
- Explicit mode matrix (batch vs interactive vs pipe)
- `RVCore::set_uart_backend()` or equivalent to inject UART config from binary layer
- Panic hook for terminal restore

[**Changed**]
- Stdin RX scoped to batch mode only (was: "interactive mode")
- `RVCore::new()` unchanged — binary-level wiring instead
- Terminal restore invariant narrowed to catchable conditions

[**Removed**]
- Piped-stdin validation scenario (not meaningful for batch OS boot)

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Stdin RX only in batch mode; xdb interactive uses TCP for guest RX |
| Review | R-002 | Accepted | `RVCore::new()` unchanged; binary injects UART backend via bus accessor |
| Review | R-003 | Accepted | Explicit mode matrix added; validation aligned |
| Review | R-004 | Accepted | Invariant narrowed; panic hook added; uncatchable kill is out of scope |
| Master | M-001 | Applied | Code clean/concise/elegant |

---

## Spec

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

## Implement

### Execution Flow

[**Main Flow — Batch**]
1. `main()` detects `X_BATCH=y`.
2. Enables raw mode if `isatty(stdin)`.
3. Installs panic hook that restores terminal.
4. Installs Ctrl-C handler that restores terminal and exits.
5. Replaces UART device on bus with `Uart::with_stdin()`.
6. Runs CPU (`run(u64::MAX)`).
7. On return, restores terminal.

[**Main Flow — Interactive xdb**]
1. No raw mode.
2. UART remains TX-only.
3. xdb REPL reads stdin for commands as before.

[**Failure Flow**]
1. Stdin EOF → reader thread exits, UART becomes TX-only. No error.
2. `tcsetattr` failure → warn, proceed without raw mode (TX-only stdin is useless but harmless).
3. `SIGKILL` → terminal left in raw mode (out of scope; user runs `reset`).

### Implementation Plan

[**Step 1: `Uart::with_stdin()` in `xcore/src/device/uart.rs`**]

Identical pattern to `with_tcp()`:

```rust
pub fn with_stdin() -> Self {
    let buf = Arc::new(Mutex::new(VecDeque::<u8>::new()));
    let rx = buf.clone();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut b = [0u8; 1];
        while std::io::stdin().read(&mut b).unwrap_or(0) > 0 {
            rx.lock().unwrap().push_back(b[0]);
        }
    });
    Self { rx_buf: buf, ..Self::new() }
}
```

[**Step 2: `Bus::replace_device()` in `xcore/src/device/bus.rs`**]

```rust
pub fn replace_device(&mut self, name: &str, dev: Box<dyn Device>) {
    let region = self.mmio.iter_mut().find(|r| r.name == name)
        .expect("device not found");
    region.dev = dev;
}
```

[**Step 3: Terminal helpers in `xdb/src/terminal.rs`**]

```rust
use std::os::fd::AsRawFd;
use std::sync::OnceLock;

static ORIG_TERMIOS: OnceLock<libc::termios> = OnceLock::new();

pub fn enable_raw_mode() -> bool {
    if unsafe { libc::isatty(std::io::stdin().as_raw_fd()) } == 0 {
        return false;
    }
    let fd = std::io::stdin().as_raw_fd();
    let mut orig = std::mem::MaybeUninit::uninit();
    if unsafe { libc::tcgetattr(fd, orig.as_mut_ptr()) } != 0 { return false; }
    let orig = unsafe { orig.assume_init() };
    ORIG_TERMIOS.set(orig).ok();
    let mut raw = orig;
    unsafe { libc::cfmakeraw(&mut raw) };
    raw.c_lflag |= libc::ISIG; // keep Ctrl-C
    unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } == 0
}

pub fn restore() {
    if let Some(orig) = ORIG_TERMIOS.get() {
        let fd = std::io::stdin().as_raw_fd();
        unsafe { libc::tcsetattr(fd, libc::TCSANOW, orig) };
    }
}

pub fn install_hooks() {
    let prev = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| { restore(); prev(info); }));
    ctrlc::set_handler(|| { restore(); std::process::exit(0); }).ok();
}
```

[**Step 4: Wire in `xdb/src/main.rs`**]

In batch mode path, after `init_xcore()`:
```rust
terminal::enable_raw_mode();
terminal::install_hooks();
xcore::with_xcpu(|cpu| cpu.replace_uart(Uart::with_stdin()));
```

---

## Trade-offs

- T-1: **`libc` termios vs `crossterm`** → `libc`. Zero deps, POSIX-only is fine.
- T-2: **Stdin RX scope** → Batch mode only. Cleanly avoids xdb stdin conflict.
- T-3: **Raw mode location** → Binary layer (`xdb/src/terminal.rs`). xcore stays pure.
- T-4: **ISIG kept** → Ctrl-C exits cleanly via signal handler. Simplest approach.

---

## Validation

[**Unit Tests**]
- V-UT-1: Existing UART tests pass (no regression).
- V-UT-2: `Bus::replace_device()` swaps a named device.

[**Integration Tests**]
- V-IT-1: `X_BATCH=y X_FILE=echo.bin make run` with TTY — type chars, see echo.
- V-IT-2: Interactive xdb mode unchanged — REPL works, UART TX works.

[**Failure / Robustness**]
- V-F-1: Stdin EOF exits reader thread without panic.
- V-F-2: Normal exit restores terminal.
- V-F-3: Ctrl-C restores terminal.
- V-F-4: Panic restores terminal.

[**Edge Cases**]
- V-E-1: Non-TTY stdin (pipe) — no raw mode, bytes still delivered.
- V-E-2: Binary input / escape sequences pass through unchanged.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (stdin → RX) | V-IT-1 |
| G-2 (raw mode) | V-IT-1 |
| G-3 (terminal restore) | V-F-2, V-F-3, V-F-4 |
| G-4 (TX unchanged) | V-IT-2, V-UT-1 |
| G-5 (xdb unaffected) | V-IT-2 |
| C-1 (raw before read) | Code ordering |
| C-2 (no panic on EOF) | V-F-1 |
| C-4 (POSIX only) | libc termios |
