# `Keyboard (UART Stdin RX)` PLAN `00`

> Status: Draft
> Feature: `keyboard`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

Add stdin-based UART RX support so that the emulator can receive keyboard input after an OS boots. The existing UART already has a thread-safe `rx_buf` pipeline and TCP-based RX; this plan adds a `with_stdin()` constructor that spawns a background thread reading raw stdin bytes into the same `rx_buf`. Terminal raw mode is required to deliver individual keystrokes without line buffering.

The change is minimal: one new constructor on `Uart`, terminal raw-mode setup in the main binary, and wiring the constructor at the `RVCore::new()` call site.

## Log

None (initial iteration).

---

## Spec

[**Goals**]
- G-1: UART receives stdin bytes as RX data, delivered via the existing `rx_buf → tick() → rx_fifo → irq_line` pipeline.
- G-2: Terminal enters raw mode so individual keypresses are delivered immediately (no line buffering, no echo).
- G-3: Clean terminal restore on exit (normal exit, Ctrl-C, panic).
- G-4: Existing TX (stdout) continues to work unchanged.
- G-5: TCP mode (`with_tcp`) remains available as an alternative.

- NG-1: No GUI keyboard support — stdin only.
- NG-2: No special key translation (escape sequences pass through as-is).

[**Architecture**]

```
Terminal (raw mode)
    │
    ▼
stdin reader thread ──push──► rx_buf (Arc<Mutex<VecDeque<u8>>>)
                                │
                            tick() drains
                                │
                                ▼
                           rx_fifo (VecDeque<u8>)
                                │
                      ┌─────────┴─────────┐
                      │                   │
               RBR read (pop)      irq_line() → PLIC → CPU
```

The architecture reuses the existing `rx_buf` shared buffer. `with_stdin()` is structurally identical to `with_tcp()` — only the data source differs.

[**Invariants**]
- I-1: `rx_buf` is the single point of RX injection regardless of backend (TCP or stdin).
- I-2: The stdin thread terminates when stdin reaches EOF or the process exits.
- I-3: Terminal attributes are restored before process exit under all conditions.
- I-4: Only one RX backend is active (stdin or TCP, selected at construction).

[**Data Structure**]

No new structs. The existing `Uart` gains one additional constructor:

```rust
impl Uart {
    /// UART with stdin RX backend. Spawns a reader thread.
    /// Caller is responsible for setting terminal raw mode.
    pub fn with_stdin() -> Self { ... }
}
```

[**API Surface**]

```rust
// New constructor — uart.rs
impl Uart {
    pub fn with_stdin() -> Self;
}

// Terminal raw mode helpers — new module or in main
fn enable_raw_mode() -> std::io::Result<libc::termios>;
fn restore_terminal(orig: &libc::termios);
```

[**Constraints**]
- C-1: Raw mode must be set before the UART stdin thread starts reading, otherwise bytes are line-buffered.
- C-2: The stdin reader must not panic on read errors — it should exit silently on EOF/error.
- C-3: Terminal restore must handle: normal exit, `Ctrl-C` (SIGINT), and panic. Use `ctrlc` crate or `atexit`-style hook.
- C-4: macOS/Linux only (POSIX termios). No Windows support needed.

---

## Implement

### Execution Flow

[**Main Flow**]
1. `main()` saves original terminal attributes and enables raw mode via `tcgetattr`/`tcsetattr`.
2. `RVCore::new()` creates UART with `Uart::with_stdin()` instead of `Uart::new()`.
3. Stdin reader thread spawns, reads one byte at a time, pushes to `rx_buf`.
4. Each CPU cycle: `bus.tick()` → `uart.tick()` drains `rx_buf` → `rx_fifo`.
5. OS reads UART RBR register → pops from `rx_fifo`.
6. On exit, terminal is restored to original mode.

[**Failure Flow**]
1. Stdin EOF (pipe/redirect) → reader thread exits, UART becomes TX-only. No error.
2. `tcsetattr` failure → warn and continue (TX-only, no RX).
3. Process killed → terminal may be left in raw mode (acceptable; user runs `reset`).

[**State Transition**]
- Terminal: Cooked → Raw (on init) → Cooked (on exit/signal)
- Stdin thread: Running → Stopped (on EOF/error/process exit)

### Implementation Plan

[**Phase 1: `Uart::with_stdin()`**]

Add to `uart.rs`:

```rust
pub fn with_stdin() -> Self {
    let buf = Arc::new(Mutex::new(VecDeque::<u8>::new()));
    let rx = buf.clone();
    std::thread::spawn(move || {
        use std::io::Read;
        let mut stdin = std::io::stdin().lock();
        let mut b = [0u8; 1];
        while stdin.read_exact(&mut b).is_ok() {
            rx.lock().unwrap().push_back(b[0]);
        }
    });
    Self {
        rx_buf: buf,
        ..Self::new()
    }
}
```

[**Phase 2: Terminal raw mode**]

Add a `terminal` module (in xcore or a shared location) with POSIX termios helpers:

```rust
use std::os::fd::AsRawFd;

pub fn enable_raw_mode() -> std::io::Result<libc::termios> {
    let fd = std::io::stdin().as_raw_fd();
    let mut orig = std::mem::MaybeUninit::uninit();
    if unsafe { libc::tcgetattr(fd, orig.as_mut_ptr()) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let orig = unsafe { orig.assume_init() };
    let mut raw = orig;
    unsafe { libc::cfmakeraw(&mut raw) };
    // Keep ISIG so Ctrl-C still works for clean shutdown
    raw.c_lflag |= libc::ISIG;
    if unsafe { libc::tcsetattr(fd, libc::TCSANOW, &raw) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(orig)
}

pub fn restore_terminal(orig: &libc::termios) {
    let fd = std::io::stdin().as_raw_fd();
    unsafe { libc::tcsetattr(fd, libc::TCSANOW, orig) };
}
```

[**Phase 3: Wire into RVCore and main**]

1. Change `RVCore::new()` to use `Uart::with_stdin()`:
   ```rust
   bus.add_mmio("uart0", 0x1000_0000, 0x100, Box::new(Uart::with_stdin()), 10);
   ```

2. In xemu main (or xdb main for interactive mode), enable raw mode and install restore hook:
   ```rust
   let orig_term = terminal::enable_raw_mode().ok();
   // Install Ctrl-C handler
   if let Some(ref orig) = orig_term {
       let t = *orig;
       ctrlc::set_handler(move || {
           terminal::restore_terminal(&t);
           std::process::exit(0);
       }).ok();
   }
   // ... run emulator ...
   // On normal exit:
   if let Some(ref orig) = orig_term {
       terminal::restore_terminal(orig);
   }
   ```

---

## Trade-offs

- T-1: **`libc` termios vs `crossterm`/`termion` crate**
  - `libc` termios: zero extra dependencies, full control, POSIX-only. Fits our minimal-dependency philosophy.
  - `crossterm`: cross-platform, higher-level API, adds a dependency tree.
  - Lean toward `libc` since we're macOS/Linux only.

- T-2: **Always stdin vs env-var opt-in**
  - Always-on: simpler, UART always has RX from stdin.
  - Opt-in (e.g. `X_STDIN=1`): doesn't interfere with piped input or non-interactive contexts.
  - Lean toward always-on in interactive mode (non-batch), TX-only in batch mode. Batch mode already skips the REPL.

- T-3: **Raw mode in xcore vs in binary (xemu/xdb)**
  - In xcore: centralized but couples terminal handling to the library.
  - In binary: each binary decides. Keeps xcore as a pure emulation library.
  - Lean toward binary-level (main.rs) for raw mode, xcore only provides `Uart::with_stdin()`.

- T-4: **Ctrl-C handling: keep ISIG vs catch signal manually**
  - `ISIG` in raw mode: kernel sends SIGINT on Ctrl-C, we hook it for cleanup.
  - Manual: disable ISIG, read Ctrl-C as byte 0x03, handle in emulator. More control but more complexity.
  - Lean toward keeping ISIG for simplicity — we just need clean exit, not in-emulator Ctrl-C handling.

---

## Validation

[**Unit Tests**]
- V-UT-1: `with_stdin()` constructor creates a UART whose `rx_buf` is externally injectable (same as `new()` tests — the shared buffer API is identical).
- V-UT-2: Existing UART tests continue to pass (no regression).

[**Integration Tests**]
- V-IT-1: Manual test: run emulator with a simple echo program, type characters, verify they appear (UART RX → TX loop).
- V-IT-2: Piped input: `echo "hello" | X_FILE=echo.bin make run` — UART receives bytes then EOF, no crash.

[**Failure / Robustness Validation**]
- V-F-1: Stdin EOF terminates reader thread without panic or error message.
- V-F-2: Terminal restored after normal exit.
- V-F-3: Terminal restored after Ctrl-C.

[**Edge Case Validation**]
- V-E-1: Rapid input (paste large text) — `rx_buf` grows without bound (acceptable for now; OS firmware reads slowly).
- V-E-2: Binary/non-UTF8 input (e.g., escape sequences) passes through unchanged.
- V-E-3: Batch mode (`X_BATCH=y`) uses `Uart::new()` (TX-only), no raw mode interference.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (stdin → RX) | V-IT-1, V-IT-2 |
| G-2 (raw mode) | V-IT-1 (keystrokes immediate) |
| G-3 (terminal restore) | V-F-2, V-F-3 |
| G-4 (TX unchanged) | V-UT-2, V-IT-1 |
| G-5 (TCP available) | Existing TCP tests pass |
| C-1 (raw before read) | Code ordering in main |
| C-2 (no panic on EOF) | V-F-1 |
| C-3 (signal handling) | V-F-3 |
| C-4 (POSIX only) | libc termios, macOS/Linux |
