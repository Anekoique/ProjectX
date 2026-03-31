# xemu

RISC-V emulator supporting RV32/RV64 with full privileged execution. Boots OpenSBI, xv6-riscv, and Linux.

## Features

- **ISA**: RV32I/RV64I, M, A, C, Zicsr, Zifencei
- **Privilege**: M/S/U modes, trap delegation, mret/sret
- **Memory**: SV32/SV39 MMU, TLB, PMP (16 entries), hardware A/D update
- **Devices**: ACLINT (timer/IPI), PLIC (32 sources), UART 16550 (PTY RX, THRE TX)
- **CSRs**: Full WARL model, shadow registers, Sstc (stimecmp), menvcfg
- **Boot**: OpenSBI firmware, xv6 direct, Linux via OpenSBI + initramfs
- **Debug**: xdb REPL (breakpoints, watchpoints, expression eval, disassembly)
- **Difftest**: QEMU (GDB RSP) + Spike (FFI) backends

## Build

```bash
make run                    # run with xdb debugger
make run FILE=binary.bin    # load a binary
make test                   # 278 unit tests
make clippy                 # lint
```

## Boot OS

```bash
# xv6 (direct M-mode boot, ramdisk)
cd ../resource && make xv6

# Linux (OpenSBI + kernel + initramfs)
cd ../resource && make linux
```

## Workspace

| Crate | Description |
|-------|-------------|
| `xcore` | Emulator engine (CPU, MMU, devices, bus) |
| `xdb` | Debugger frontend (CLI REPL, difftest) |
| `xlogger` | Colored timestamped logging |

## Memory Map

```
0x0200_0000  ACLINT (MSWI + MTIMER + SSWI)
0x0C00_0000  PLIC (32 sources, 2 contexts)
0x1000_0000  UART0 (NS16550A)
0x8000_0000  DRAM (128 MB)
```
