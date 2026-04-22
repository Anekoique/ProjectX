# `xemu-init-logo` PRD

---

[**What**]
Print an ASCII-art "XEMU" banner to stdout (with ANSI color when stdout is a TTY) at the top of `xcore::init_xcore`, just before the existing `info!("Hello xcore!")` line.

[**Why**]
A startup banner gives xemu a recognizable identity in logs and terminal output, makes it easy to distinguish xemu runs from other tooling, and is the conventional "you know it's running" signal for emulators and VMMs (QEMU, Spike, Firecracker all do some form of this).

[**Outcome**]
- Running any xcore embedder (today: `xdb`) prints the ASCII "XEMU" banner on stdout as the first output of `init_xcore`, before `Hello xcore!`.
- Banner is colored with ANSI escapes when stdout is a terminal; plain ASCII (no escapes) when redirected to a file or pipe — detected via `std::io::IsTerminal`.
- Banner is ASCII-only (7-bit), fits in an 80-column terminal, and has no dependency on `xlogger` (uses `println!`).
- `cargo test -p xcore` still passes (no test output pollution beyond the banner printing once per `init_xcore` call — existing tests either construct `CPU` directly without calling `init_xcore`, or already tolerate init output).
- Disabling the banner is possible via an env var (e.g. `X_NO_LOGO=1`) so scripting / difftest workflows can suppress it when noise is harmful.

[**Related Specs**]
None — no feature specs exist yet.
