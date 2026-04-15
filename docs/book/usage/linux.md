# Linux (OpenSBI + initramfs)

Boots a full Linux 6.1.44 kernel to an interactive shell.

## Running

```bash
cd resource
make linux              # single-hart
make linux-2hart        # 2 harts (cooperative scheduler)
```

Expected time to prompt: ~3 s.

## Boot chain

```
xemu M-mode  →  OpenSBI v1.3.1  →  Linux (S-mode)  →  static init (busybox lp64d)
```

- **OpenSBI v1.3.1** — fw_jump configuration, generic platform.
- **Linux 6.1.44** — bootlin kernel with `rv64imafdc`, Sstc timer.
- **initramfs** — bootlin rootfs (busybox + glibc lp64d), auto-downloaded
  at first build and packed into `initrd.cpio.gz`.

## Init prompt

The initramfs runs a minimal static init with built-in commands:

```
ls   pwd   cd   cat   echo   uname   poweroff
```

`poweroff` invokes the SiFive test finisher via SBI shutdown — clean
exit.

## DTS

`resource/xemu-linux.dts` declares:

- 1 GiB RAM at `0x8000_0000`
- 1 or 2 harts (`cpus@0`, `cpus@1`)
- ACLINT, PLIC, UART, test-finisher nodes
- `riscv,isa = "rv64imafdcsu_sstc"`
- `timebase-frequency = 10_000_000`

## SMP notes

`make linux-2hart` boots two harts on a single-threaded cooperative
round-robin scheduler. Both cores share the same `Bus` instance.
True per-hart OS threads are gated by the Phase 11 RFC; see
[`../PROGRESS.md`](../../PROGRESS.md) §Phase 11.
