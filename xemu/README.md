# xemu

RISC-V emulator supporting RV32/RV64 with full privileged execution. Boots OpenSBI, xv6-riscv, Linux, and Debian.

## Features

- **ISA**: RV32I/RV64I, M, A, F, D, C, Zicsr, Zifencei, Sstc
- **Privilege**: M/S/U modes, trap delegation, mret/sret
- **Memory**: SV32/SV39 MMU, TLB, PMP (16 entries), hardware A/D update
- **Devices**: ACLINT (timer/IPI), PLIC (32 sources), UART 16550, VirtIO-blk (MMIO legacy)
- **CSRs**: Full WARL model, shadow registers, Sstc (stimecmp), menvcfg
- **Boot**: OpenSBI firmware, xv6 direct, Linux initramfs, Debian ext4 via virtio-blk
- **Debug**: xdb REPL (breakpoints, watchpoints, expression eval, disassembly)
- **Difftest**: QEMU (GDB RSP) + Spike (FFI) backends

## Build

```bash
make run                    # run with xdb debugger
make run FILE=binary.bin    # load a binary
make test                   # unit tests
make clippy                 # lint
```

## Boot OS

```bash
cd ../resource

make opensbi    # OpenSBI firmware only
make xv6        # xv6-riscv (M-mode, ramdisk shell)
make linux      # Linux 6.1 (OpenSBI + initramfs shell)
make debian     # Debian 13 Trixie (ext4 root via virtio-blk)
```

Debian boot downloads a pre-built riscv64 ext4 image and mounts it as `/dev/vda` through the VirtIO block device. Exit with **Ctrl-A X**.

## Workspace

| Crate | Description |
|-------|-------------|
| `xcore` | Emulator engine (CPU, MMU, devices, bus) |
| `xdb` | Debugger frontend (CLI REPL, difftest) |
| `xlogger` | Colored timestamped logging |

## Memory Map

```
0x0010_0000  SiFive Test Finisher
0x0200_0000  ACLINT (MSWI + MTIMER + SSWI)
0x0C00_0000  PLIC (32 sources, 2 contexts)
0x1000_0000  UART0 (NS16550A)
0x1000_1000  VirtIO Block (MMIO legacy, PLIC IRQ 1)
0x8000_0000  DRAM (128 MB default, 1 GB with disk)
```
