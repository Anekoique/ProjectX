# Device memory map

Default xemu machine layout, QEMU-virt-compatible in shape with
documented deltas.

| Device | Base | Size | IRQ |
|--------|-----:|-----:|:---:|
| Test finisher (test-only) | `0x0010_0000` | `0x10` | — |
| ACLINT | `0x0200_0000` | `0x1_0000` | — |
| PLIC | `0x0C00_0000` | `0x400_0000` | — |
| UART0 (NS16550) | `0x1000_0000` | `0x100` | 10 |
| VirtIO MMIO (Debian target) | `0x1000_1000` | `0x1000` | 1 |
| RAM | `0x8000_0000` | 128 MiB (tests) / 1 GiB (Linux) | — |

## Intentional deltas from QEMU virt

- **ACLINT replaces CLINT.** Wire-compatible MMIO layout; offers
  clean MSWI / MTIMER / SSWI split.
- **Test finisher** is test-only. Not wired into the default
  machine used by Linux / Debian.
- **`timebase-frequency = 10_000_000`** (10 MHz), matching the host
  wall-clock sampling rate.

## PLIC source assignments

| Source | Owner |
|-------:|-------|
| 0 | "no interrupt" (reserved) |
| 1 | VirtIO-blk |
| 10 | UART0 |

Higher source numbers are reserved for future devices.

## IrqState bitmap

`Arc<AtomicU64>` where:

| Bit | `mip` name | Writer |
|----:|-----------|--------|
| 1 | SSIP | ACLINT SSWI |
| 3 | MSIP | ACLINT MSWI |
| 7 | MTIP | ACLINT MTIMER |
| 9 | SEIP | PLIC context 1 |
| 11 | MEIP | PLIC context 0 |

`sync_interrupts()` on CPU step merges this into `mip`.

## Boot layout (where the ELF lands)

- **Bare-metal tests** — entry at `0x8000_0000`.
- **xv6** — entry at `0x8000_0000` (M-mode).
- **Linux / Debian** — OpenSBI lands at `0x8000_0000` (M-mode), then
  jumps to the kernel at `0x8020_0000` (S-mode).
- **FDT** — `BootLayout::fdt_addr` persists the DTB address so the
  kernel can find it at `a1` on entry.
