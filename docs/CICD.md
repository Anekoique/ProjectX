# CI/CD

## Pipeline: `.github/workflows/ci.yml`

Triggers on push to `main` and all pull requests.

### Stage 1 — Fast Checks (parallel, no build deps)

| Job | What |
|-----|------|
| **fmt** | `cargo fmt --all --check` (xemu, xam) + `clang-format --dry-run --Werror` (xlib) |
| **clippy** | `cargo clippy -- -D warnings` (xemu) + cross-target clippy (xam) |

### Stage 2 — Tests (parallel, after stage 1)

| Job | What | Extra deps |
|-----|------|------------|
| **test-unit** | `cargo test -p xcore` | — |
| **test-cpu-rs** | 31 Rust bare-metal tests via `make run` | axconfig-gen, riscv64-linux-musl-cross |
| **test-cpu-c** | 35 C bare-metal tests via `make run` | axconfig-gen, riscv64-linux-musl-cross |

### Environment

```
AM_HOME    = ${{ github.workspace }}/xam
XEMU_HOME  = ${{ github.workspace }}/xemu
```

### Toolchain

- **Rust**: auto-detected from `rust-toolchain.toml` (nightly-2026-03-15)
- **Rust target**: `riscv64gc-unknown-none-elf`
- **C cross-compiler**: `riscv64-unknown-linux-musl` from [cross-tools/musl-cross](https://github.com/cross-tools/musl-cross/releases)
- **axconfig-gen**: `cargo install axconfig-gen` (cached in `~/.cargo/bin`)
- **clang-format**: system package (fmt job only)

### Caching

Cargo registry, git db, `~/.cargo/bin`, and per-workspace `target/` directories are cached. Keys include `Cargo.lock` hashes (and xlib source hashes for C tests).

## Future

- **am-tests**: CSR/privilege validation (when implemented)
- **difftest**: QEMU comparison (Phase 6)
- **coverage**: `cargo-tarpaulin` or `llvm-cov`
- **release**: automated binary builds on tag push
