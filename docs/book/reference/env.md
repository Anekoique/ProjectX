# Environment variables

Recognised by the `make run` / `make linux` / etc. entry points.

| Var | Values | Default | Effect |
|-----|--------|---------|--------|
| `DEBUG` | `y` / `n` | `n` | `y` routes UART to a PTY, enables richer logging, and turns off release optimisations. Always set `DEBUG=n` when benchmarking. |
| `LOG` | `trace` / `debug` / `info` / `warn` / `error` / `off` | `info` | xlogger verbosity. `trace` is per-instruction. |
| `X_HARTS` | integer ≥ 1 | `1` | Guest hart count (cooperative scheduler). |
| `X_FILE` | path | set by per-target Makefile | ELF to execute. Don't set manually — let `make run` resolve it. |
| `DIFFTEST` | `0` / `1` | `0` | Compile-in QEMU / Spike difftest backends. |
| `AM_HOME` | path | `${workspace}/xam` | Where xam HAL sources live. |
| `XEMU_HOME` | path | `${workspace}/xemu` | Where xemu workspace lives. |
| `XLIB_HOME` | path | `${workspace}/xlib` | Where xlib (klib) sources live. |

## CI-only

| Var | Effect |
|-----|--------|
| `ECC_DISABLED_HOOKS` | Disable specific Everything-Claude-Code plugin hooks by hook ID. |
| `ECC_HOOK_PROFILE` | `minimal` / `standard` / `strict` — coarse toggle. |

## Runtime (xdb REPL)

Not env vars — commands inside the monitor. See
[The xdb debugger](../usage/debugger.md).
