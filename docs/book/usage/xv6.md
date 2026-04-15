# xv6-riscv

Boots the MIT xv6-riscv kernel to an interactive shell.

## Running

```bash
cd resource
make xv6
```

Expected time to prompt: ~0.3 s.

## What happens

- xemu starts in M-mode at `0x8000_0000`.
- The xv6 bootstrap switches to S-mode, sets up page tables, and
  starts the kernel scheduler.
- Console (`sh`) runs off a ramdisk embedded in the kernel image.

No firmware is loaded — xv6 runs directly. This makes it the simplest
"real OS" target and a good sanity check after touching the trap
framework or the MMU.

## Exiting

`Ctrl-A X` — QEMU-style escape, intercepted by xemu's UART.
