# Debian 13 Trixie (VirtIO-blk rootfs)

Boots a full Debian 13 system from an ext4 rootfs mounted via a
VirtIO-blk device.

## Running

```bash
cd resource
make debian            # single-hart
make debian-2hart      # 2 harts
```

Expected time to prompt: ~20 s.

## What you get

- **4 GiB ext4 root** at `/dev/vda`, mounted read-write.
- **288 dpkg packages** pre-installed, including Python3 (verified
  during boot test).
- Full Debian shell — `apt`, `vim`, `git`, coreutils.

## Boot chain

```
xemu M-mode  →  OpenSBI v1.3.1  →  bootlin kernel  →  Debian userspace (/sbin/init)
```

The bootlin kernel is used in place of a custom kernel because it
already has F / D extension support and the right Sstc driver, which
matches xemu's exposed ISA.

## First-run setup

On first `make debian`, the build system downloads the pre-built
4 GiB image (`xemu-debian.img`) to `resource/debian/`. Subsequent
runs reuse the snapshot; changes to the guest filesystem are
persisted across runs.

## Two-tier reset

- **VirtIO transport reset** (issued by guest driver during `probe`)
  — preserves disk contents; only resets the queue state.
- **Emulator hard-reset** (via test finisher) — restores the disk
  snapshot, so repeated runs start from a clean Debian install.

## DTS

`resource/xemu-debian.dts`:

- 1 GiB RAM
- `virtio,mmio` node at `0x1000_1000`
- `chosen: bootargs = "root=/dev/vda rw ..."`
- Same ACLINT / PLIC / UART as the Linux target

## Cleanly exiting

`poweroff` from the Debian shell → SBI shutdown → xemu exits.
Or `Ctrl-A X` for an immediate abort.
