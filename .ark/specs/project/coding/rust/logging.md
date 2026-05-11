# Logging

> Rules for log macro choice, level selection, and crate-level prefixes. Consistent logging is what makes a kernel-shaped codebase debuggable.

## R1 — First-party crates use OSTD logging macros, not `log` or `println!`

**Applies to:** `ostd/**/*.rs`, `visor/**/*.rs`, `vsdk/**/*.rs`
**Evidence:** grep

OSTD provides `debug!`, `info!`, `notice!`, `warn!`, `error!`, `crit!`, `alert!`, `emerg!`. Import via `use ostd::prelude::*` or `use ostd::log::{...}`. Do not use the third-party `log` crate directly — OSTD bridges it for upstream crates that depend on `log`, but first-party code uses OSTD's macros. Custom output functions, `println!`, and hand-rolled serial print macros are not acceptable in production code. Exception: code that runs before the logging subsystem is initialized may use early-boot output helpers.

```rust
// Bad — third-party log crate directly
log::info!("VirtIO block device initialized: {} sectors", num_sectors);

// Bad — println in a kernel context
println!("VirtIO block device initialized: {} sectors", num_sectors);

// Good
info!("VirtIO block device initialized: {} sectors", num_sectors);
```

## R2 — Log levels match `syslog(2)` severity semantics

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

| Level | Use for |
|-------|---------|
| `emerg!` | System unusable; immediately before `abort()`. |
| `alert!` | Action must be taken immediately. |
| `crit!` | Critical conditions: unrecoverable resource exhaustion. |
| `error!` | Serious but recoverable failures: invariant violations, I/O errors. |
| `warn!` | Recoverable problems: fallback paths, deprecated usage. |
| `notice!` | Normal but significant: CPU online, security feature activated. |
| `info!` | Routine informational: subsystem init, configuration changes. |
| `debug!` | Development diagnostics: state transitions, per-packet tracing. |

A log statement that fires on every syscall or every timer tick must be `debug!`. Reserve `crit!`/`emerg!` for failures immediately before halt or abort.

```rust
// Bad — info! on a per-tick path floods the log
fn timer_tick(&mut self) {
    info!("tick at {}", self.now());
    ...
}

// Good
fn timer_tick(&mut self) {
    debug!("tick at {}", self.now());
    ...
}
```

## R3 — Each crate defines a `__log_prefix!` macro at its root

**Applies to:** `**/lib.rs`, subsystem `mod.rs` overrides
**Evidence:** grep

Define `__log_prefix!` at the crate root before any `mod` declarations, so every log message identifies its origin. Convention: lowercase crate name (without `aster_` prefix), followed by `: `. Subsystem modules within a crate may override the prefix at the top of their `mod.rs`; child modules inherit the override automatically. Do not put `#[rustfmt::skip]` or any other attribute on `__log_prefix!` — it triggers a compiler ambiguity error (E0659). Manual bracket prefixes like `[IOMMU]` or `[Virtio]:` in log strings are not acceptable; the `__log_prefix!` mechanism replaces them.

```rust
// Bad — manual bracket prefix in the message
info!("[Virtio] block device initialized");

// Good — crate-level prefix
// in lib.rs:
macro_rules! __log_prefix {
    () => { "virtio: " };
}

info!("block device initialized");
// emits: virtio: block device initialized
```
