# `xemu-init-logo` PLAN `00`

> Status: Draft
> Feature: `xemu-init-logo`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

Add a startup banner for `xcore::init_xcore`: an ASCII-art "XEMU" logo emitted via `println!` (bypassing `xlogger`) as the first statement of `init_xcore`, before `info!("Hello xcore!")`. ANSI color wraps the banner only when stdout is a TTY (`std::io::IsTerminal::is_terminal`). A kill-switch env var `X_NO_LOGO` (non-empty => suppress) lets difftest and scripted runs silence it. The logo lives in a new `xcore::utils::logo` submodule as crate-private API; tests validate shape, TTY gating, and env-var gating through an internal helper that accepts env + is_tty as parameters (so tests do not mutate process env).

## Log `None in 00_PLAN`

---

## Spec `Core specification`

[**Goals**]

- G-1: `init_xcore` prints an ASCII "XEMU" banner to stdout as its first output, before any other log line (maps to PRD Outcome #1).
- G-2: Banner is ANSI-colored when stdout is a terminal; plain ASCII (no `\x1b` escape bytes) when stdout is redirected (maps to PRD Outcome #2).
- G-3: Banner is 7-bit ASCII only, fits in an 80-column terminal, and does not depend on `xlogger` (uses `println!` / `writeln!` to stdout directly) (maps to PRD Outcome #3).
- G-4: `cargo test -p xcore` continues to pass; the banner does not pollute test output beyond the single print per `init_xcore` call (maps to PRD Outcome #4).
- G-5: Setting env var `X_NO_LOGO` to a non-empty value suppresses the banner entirely (maps to PRD Outcome #5).

- NG-1: No version string, commit hash, build timestamp, or config echo in the banner.
- NG-2: No runtime configurability beyond the `X_NO_LOGO` kill-switch. No custom banner injection, no color override, no alternate banner variants.
- NG-3: No Windows-specific ANSI enablement (e.g. `enable_ansi_support`). Modern Windows 10+ terminals handle ANSI; legacy terminals that do not will render raw escape codes — accepted fallback, documented under constraints.
- NG-4: Banner is NOT routed through the `log` crate / `xlogger`. It must survive any log-level filter.
- NG-5: No public re-export of the logo API from `lib.rs`. Crate-internal only.

[**Architecture**]

```
caller (e.g. xdb::main)
   │
   └─► xcore::init_xcore(config)                 [lib.rs:50]
          │
          ├─► utils::print_logo()                [NEW — utils/logo.rs]
          │     │
          │     ├─ check env("X_NO_LOGO")        ── non-empty ⇒ return
          │     ├─ check std::io::stdout().is_terminal()
          │     │     ├─ true  ⇒ emit ESC[36m LOGO ESC[0m
          │     │     └─ false ⇒ emit LOGO unchanged (no escapes)
          │     └─ write via writeln!(stdout, ...).ok()   (swallow broken-pipe)
          │
          ├─► info!("Hello xcore!")              [unchanged]
          └─► ... existing init sequence ...
```

The logo module is a leaf utility: it depends only on `std::io` and `std::env`. It has no dependency on `xlogger`, `log`, or any other xemu crate module. Control flow is strictly top-down from `init_xcore`; the banner has no effect on CPU/Bus/Device construction.

[**Data Structure**]

A single module-scope constant holding the ASCII art, plus two private helpers factored for testability. The executor picks the exact glyph rendering at implementation time (5-line block-letter "XEMU", ≤80 cols, 7-bit ASCII). `LOGO` is a `&'static str` with embedded newlines.

```rust
// xemu/xcore/src/utils/logo.rs

/// Five-line block-letter "XEMU" banner. ASCII-only, ≤80 columns per line.
/// Picked by Executor at implementation time; kept as a single const so tests
/// can inspect it without reflection.
const LOGO: &str = "...";

/// ANSI SGR code applied uniformly to the banner when stdout is a TTY.
/// `36` = cyan. Wrapped as `"\x1b[36m"` prefix + `"\x1b[0m"` reset suffix.
const ANSI_COLOR: &str = "\x1b[36m";
const ANSI_RESET: &str = "\x1b[0m";

/// Name of the kill-switch env var. Non-empty value ⇒ banner suppressed.
const ENV_NO_LOGO: &str = "X_NO_LOGO";
```

No structs, enums, or traits are introduced. The feature is stateless.

[**API Surface**]

```rust
// xemu/xcore/src/utils/logo.rs

/// Print the xemu startup banner to stdout.
///
/// - Writes nothing when `X_NO_LOGO` is set to a non-empty value.
/// - Wraps `LOGO` with ANSI color when `std::io::stdout().is_terminal()` is
///   true; otherwise emits `LOGO` unchanged (no escape bytes).
/// - Swallows broken-pipe / closed-stdout errors; never panics.
pub(crate) fn print_logo();

/// Testable core: renders the banner to a `String` given explicit env lookup
/// and tty flag. `print_logo` wires this to the real stdout + env.
fn render(no_logo_env: Option<&str>, is_tty: bool) -> String;
```

```rust
// xemu/xcore/src/utils/mod.rs — addition
mod logo;
pub(crate) use logo::print_logo;
```

```rust
// xemu/xcore/src/lib.rs — modification inside init_xcore
pub fn init_xcore(config: MachineConfig) -> XResult {
    use std::sync::Mutex;
    utils::print_logo();              // NEW — first statement
    info!("Hello xcore!");            // unchanged
    // ... unchanged ...
}
```

Semantics:

- `print_logo` is infallible from the caller's perspective (returns `()`).
- `render` returns an empty `String` when suppressed. Otherwise returns either plain `LOGO` (possibly with a trailing newline for terminal hygiene) or `ANSI_COLOR + LOGO + ANSI_RESET` when `is_tty`.

[**Constraints**]

- C-1: `LOGO` is strictly 7-bit ASCII (every byte `<= 0x7E`, `>= 0x20` or `\n`).
- C-2: No line of `LOGO` exceeds 80 columns (bytes between newlines, or between start and first newline).
- C-3: The logo path must not depend on `xlogger` or the `log` crate; it writes only to `std::io::stdout()`.
- C-4: `X_NO_LOGO` semantics match the `X_*` idiom used by `xdb::env` (`xemu/xdb/src/main.rs:44`): a **non-empty** env value suppresses; unset or empty string does not suppress.
- C-5: The banner code path must not panic. Broken-pipe / closed-stdout errors are swallowed via `writeln!(..., ...).ok()`.
- C-6: Windows caveat — no explicit ANSI enablement call is made. On legacy Windows terminals that do not interpret SGR escapes, users will see raw `\x1b[36m` bytes when stdout is a TTY; this is an accepted fallback, not a bug.
- C-7: `print_logo` is `pub(crate)` and not re-exported from `lib.rs`. Not part of the public API surface.
- C-8: Banner is emitted exactly once per `init_xcore` call. `init_xcore` is already documented as "must be called exactly once" (`xemu/xcore/src/lib.rs:49`); no additional de-duplication logic is needed.

## Runtime `runtime logic`

[**Main Flow**]

1. Caller invokes `xcore::init_xcore(config)`.
2. `init_xcore` immediately calls `utils::print_logo()` as its first statement.
3. `print_logo` reads `std::env::var("X_NO_LOGO")`. If the result is `Ok(s)` and `s` is non-empty, it returns without writing anything.
4. Otherwise `print_logo` queries `std::io::stdout().is_terminal()`.
5. `render(env, is_tty)` builds the output string: `LOGO` verbatim when `!is_tty`, or `ANSI_COLOR + LOGO + ANSI_RESET` when `is_tty`.
6. `print_logo` writes the rendered string via `writeln!(std::io::stdout(), "{}", s).ok()` — the `.ok()` swallows any I/O error.
7. Control returns to `init_xcore`, which proceeds to `info!("Hello xcore!")` and the existing CPU construction path.

[**Failure Flow**]

1. Stdout is a closed pipe (e.g. `xdb | head -0` after the head exits). `writeln!` returns `Err(BrokenPipe)`; `.ok()` discards it; `print_logo` returns normally; `init_xcore` continues. No panic (C-5).
2. Env var read fails for any reason other than `NotPresent` (e.g. non-UTF-8 value). `std::env::var` returns `Err`; treated as "not set" → banner prints. This matches the `env()` helper idiom in `xemu/xdb/src/main.rs:44`.
3. `is_terminal()` query fails on an exotic platform: `IsTerminal` returns `false` for any fd it cannot classify as a TTY; banner emits plain ASCII (safe fallback).
4. Terminal does not interpret ANSI (legacy Windows): user sees raw escape bytes mixed with the banner. Accepted per NG-3 / C-6.

[**State Transitions**]

N/A — the banner path is stateless. No persistent state is read or written; no lifecycle transitions are modeled.

## Implementation `split task into phases`

[**Phase 1 — Logo module**]

- Create `xemu/xcore/src/utils/logo.rs` with:
  - `const LOGO: &str` — a 5-line block-letter "XEMU" rendering, each line ≤80 cols, 7-bit ASCII only. Executor picks the exact glyph shape; prefer clean block letters over decorative fonts.
  - `const ANSI_COLOR: &str = "\x1b[36m"` (cyan) and `const ANSI_RESET: &str = "\x1b[0m"`.
  - `const ENV_NO_LOGO: &str = "X_NO_LOGO"`.
  - `fn render(no_logo_env: Option<&str>, is_tty: bool) -> String` — pure function; empty string when `no_logo_env.is_some_and(|s| !s.is_empty())`, else plain or color-wrapped `LOGO`.
  - `pub(crate) fn print_logo()` — reads env and `is_terminal`, calls `render`, writes via `writeln!(std::io::stdout(), "{}", s).ok()`.
- Update `xemu/xcore/src/utils/mod.rs`: add `mod logo;` and `pub(crate) use logo::print_logo;`.

[**Phase 2 — Wire into init_xcore**]

- In `xemu/xcore/src/lib.rs`, add `utils::print_logo();` as the very first statement of `init_xcore` (before the `use std::sync::Mutex;` import is fine; before `info!("Hello xcore!")` is required).
- No changes to function signature, error type, or caller contracts.

[**Phase 3 — Unit tests**]

- Add a `#[cfg(test)] mod tests` block inside `utils/logo.rs` covering:
  - `logo_is_pure_ascii`: every byte of `LOGO` satisfies `b.is_ascii()`.
  - `logo_lines_within_80_cols`: `LOGO.lines().all(|l| l.len() <= 80)`.
  - `logo_non_empty`: `!LOGO.trim().is_empty()`.
  - `render_suppressed_when_env_set`: `render(Some("1"), true)` and `render(Some("anything"), false)` both return `""`.
  - `render_not_suppressed_when_env_empty`: `render(Some(""), true)` and `render(None, true)` both return non-empty.
  - `render_plain_when_not_tty`: `render(None, false)` contains no `\x1b` byte and contains `LOGO`.
  - `render_colored_when_tty`: `render(None, true)` starts with `ANSI_COLOR` and ends with `ANSI_RESET` (modulo trailing newline).
- Tests must not touch `std::env` or real stdout — they exercise `render` directly with explicit parameters.
- `V-IT-1` is satisfied by running the existing `cargo test -p xcore` suite end-to-end; no new integration test file is required because the feature has no cross-module runtime behavior worth asserting in isolation.

## Trade-offs `ask reviewer for advice`

- T-1: **`println!` vs `info!` for the banner.**
  - Option A — `println!`/`writeln!` to stdout (chosen).
    - Adv.: survives any `X_LOG` filter; no dependency on `xlogger` init order; matches the "banner always visible when not suppressed" contract (PRD Outcome #3).
    - Disadv.: does not carry a timestamp / level prefix; cannot be rerouted by `xlogger` consumers.
  - Option B — `info!("{LOGO}")`.
    - Adv.: single sink; consistent formatting with other log lines.
    - Disadv.: banner disappears when `X_LOG=warn` or higher; `xlogger` prepends level prefix that clashes with multi-line ASCII art. **Rejected** per PRD explicit wording.
  - Decision: A. Matches PRD exactly.

- T-2: **ANSI escapes on non-TTY.**
  - Option A — strip escapes when `!is_terminal()` (chosen).
    - Adv.: clean log files and piped output; no escape noise in CI logs or difftest captures.
    - Disadv.: slight code branch; requires `IsTerminal` (stable 1.70+, available in 2024 edition — verified via xemu `Cargo.toml`).
  - Option B — always emit escapes.
    - Adv.: simpler code.
    - Disadv.: pollutes files and pipes with `\x1b[36m` noise; breaks grep/diff tooling. **Rejected.**
  - Decision: A.

- T-3: **Broken-pipe handling: panic vs. swallow.**
  - Option A — `writeln!(stdout, ...).ok()` (chosen).
    - Adv.: `init_xcore` never panics from the banner path even under `xdb | head -0`; aligns with C-5.
    - Disadv.: silently loses the banner on broken pipe — acceptable since the banner is purely informational.
  - Option B — `println!` (Rust default panics on broken-pipe write).
    - Adv.: shortest code.
    - Disadv.: emulator now panics mid-boot on a closed stdout. **Rejected.**
  - Decision: A. Explicitly validated by V-F-1.

- T-4: **Logo module placement: `xcore::utils` vs `xdb`.**
  - Option A — `xcore::utils::logo` (chosen).
    - Adv.: banner fires for every `xcore::init_xcore` embedder, current (xdb) or future; single source of truth.
    - Disadv.: xcore (the "library") now writes to stdout directly, which slightly broadens its I/O surface. Mitigated by keeping the API `pub(crate)`.
  - Option B — `xdb::print_logo()`, called before `xcore::init_xcore`.
    - Adv.: xcore stays I/O-pure beyond its existing `xlogger` usage.
    - Disadv.: PRD explicitly says banner is "the first output of `init_xcore`"; placing it in xdb violates that contract and forces every future embedder to duplicate the call. **Rejected.**
  - Decision: A.

- T-5: **ANSI color choice: cyan vs bright-blue.**
  - Adv. cyan (`36`): conventional tooling color (e.g. cargo, npm); readable on both dark and light terminals.
  - Adv. bright-blue (`94`): stronger brand feel; renders as a distinct accent on most palettes.
  - Disadv.: either choice is subjective. Non-load-bearing.
  - Decision: cyan (`36`). Swappable in one line if Executor prefers bright-blue at implementation time — record the final choice in EXECUTE notes.

## Validation `test design`

[**Unit Tests**]

- V-UT-1: `LOGO` is pure 7-bit ASCII — `assert!(LOGO.bytes().all(|b| b.is_ascii()))`. Covers C-1.
- V-UT-2: Every line in `LOGO` is ≤80 cols — `assert!(LOGO.lines().all(|l| l.len() <= 80))`. Covers C-2.
- V-UT-3: `LOGO` is non-empty and non-whitespace — `assert!(!LOGO.trim().is_empty())`. Covers G-1.
- V-UT-4: When the `X_NO_LOGO` param is `Some(non_empty)`, `render` returns `""` regardless of `is_tty`. Covers G-5, C-4.
- V-UT-5: When `is_tty == false` and env is unset, `render` output contains no `\x1b` byte and contains `LOGO` verbatim. Covers G-2.
- V-UT-6: When `is_tty == true` and env is unset, `render` output starts with `\x1b[36m` and ends with `\x1b[0m` (possibly followed by a single `\n`). Covers G-2.
- V-UT-7: When env is `Some("")`, `render` behaves as if env were `None` (banner prints). Covers C-4 (empty-string semantics match `xdb::env`).

[**Integration Tests**]

- V-IT-1: `cargo test -p xcore` completes with zero failures after the change. Covers G-4.

[**Failure / Robustness Validation**]

- V-F-1: `print_logo` does not panic when stdout is a closed pipe. Validated indirectly by the `.ok()`-swallow pattern (inspection); executor may add a dedicated test using a temporarily-redirected stdout if cheap, otherwise documented as code-review gate. Covers C-5.

[**Edge Case Validation**]

- V-E-1: `X_NO_LOGO=""` (empty string) is treated as NOT set — banner still prints. Covered by V-UT-7. Matches the `X_*` idiom in `xemu/xdb/src/main.rs:44`.
- V-E-2: `X_NO_LOGO` unset — banner prints in both TTY and non-TTY modes. Covered by V-UT-5 and V-UT-6.
- V-E-3: Visual inspection of the rendered banner on a real terminal — manual check during EXECUTE. Aesthetic constraint ("looks beautiful") is non-automated; Executor confirms at implementation time. Covers the subjective portion of G-1.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (banner prints first) | V-UT-3, V-E-3 (manual visual), inspection of Phase 2 diff placing `print_logo()` before `info!` |
| G-2 (ANSI on TTY, plain otherwise) | V-UT-5, V-UT-6 |
| G-3 (ASCII-only, ≤80 cols, no xlogger) | V-UT-1, V-UT-2; no-xlogger-dep enforced by code review of Phase 1 (`use` statements) |
| G-4 (`cargo test -p xcore` passes) | V-IT-1 |
| G-5 (`X_NO_LOGO` suppresses) | V-UT-4, V-UT-7 |
| C-1 | V-UT-1 |
| C-2 | V-UT-2 |
| C-3 | Code-review gate on Phase 1 imports (no `log` / `xlogger` use) |
| C-4 | V-UT-4, V-UT-7 |
| C-5 | V-F-1 |
| C-6 | Documented; no automated test — accepted fallback per NG-3 |
| C-7 | Code-review gate on Phase 1 / Phase 2 visibility (`pub(crate)`, no re-export) |
| C-8 | Inherited from existing `init_xcore` "call exactly once" contract; no new check required |
