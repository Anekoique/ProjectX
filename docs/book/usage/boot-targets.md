# Boot targets

xemu supports four kinds of boot:

| Target | Privilege entry | Firmware | Guest payload |
|--------|-----------------|----------|---------------|
| am-tests | M-mode | none | bare-metal test kernel |
| xv6 | M-mode | xv6 bootstrap | xv6 kernel + ramdisk |
| Linux | M-mode | OpenSBI v1.3.1 | Linux 6.1.44 + initramfs |
| Debian | M-mode | OpenSBI + bootlin kernel | Linux + ext4 rootfs via VirtIO-blk |

All targets share the same machine layout (RAM at `0x8000_0000`,
ACLINT at `0x0200_0000`, PLIC at `0x0C00_0000`, UART0 at
`0x1000_0000`). The differences are in **what gets loaded** and
**whether a firmware layer is present**.

See the individual pages for each:

- [Bare-metal tests (am-tests)](./am-tests.md)
- [xv6-riscv](./xv6.md)
- [Linux (OpenSBI + initramfs)](./linux.md)
- [Debian 13 (VirtIO-blk rootfs)](./debian.md)

## Common environment variables

| Var | Default | Effect |
|-----|---------|--------|
| `DEBUG` | `n` | `y` routes UART to a PTY (requires `screen` attach) and enables extra logging |
| `LOG` | `info` | `trace` / `debug` / `info` — controls xlogger verbosity |
| `X_HARTS` | `1` | Number of guest harts (single-threaded cooperative scheduler) |
| `DIFFTEST` | `0` | `1` enables per-instruction comparison against QEMU / Spike |

Use `make run` (or the per-target aliases like `make linux`) — not
`target/release/xdb` directly. The Makefiles wire up `X_FILE`, boot
layout, and DTB compilation for you.
