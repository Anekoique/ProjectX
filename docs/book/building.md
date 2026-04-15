# Building xemu

## Prerequisites

- **Rust toolchain** — auto-detected from `rust-toolchain.toml`
  (nightly).
- **C cross-compiler** — `riscv64-unknown-linux-musl` for building
  guest C programs. On macOS, install via `brew` or fetch from
  [cross-tools/musl-cross](https://github.com/cross-tools/musl-cross/releases).
- **axconfig-gen** — `cargo install axconfig-gen` (cached in
  `~/.cargo/bin`).
- **clang-format** — system package, used by the `fmt` CI job.

## Build modes

| Mode | Invocation | Notes |
|------|------------|-------|
| Release (default) | `make run` | LTO + `codegen-units = 1`. Use for benchmarks. |
| Debug | `DEBUG=y make run` | Faster to compile; slower at runtime. |
| Difftest-enabled | `DIFFTEST=1 make run` | Links the QEMU / Spike comparison backends. |

Always set `DEBUG=n` before benchmarking.

## Supported hosts

- **macOS** (Apple Silicon, Intel) — primary development target.
- **Linux** (x86_64, aarch64) — CI target. `samply` profiling works
  without entitlement on Linux.

Windows is not supported.

## Cargo workspace

xemu is a single Cargo workspace at `xemu/`. Build the whole thing:

```bash
cd xemu
cargo build --release
cargo test --workspace
```

The resulting binary is `xemu/target/release/xdb`. In normal
development you don't invoke `xdb` directly — `make run` from a
kernel directory wires up the right `X_FILE` and launch flags.
