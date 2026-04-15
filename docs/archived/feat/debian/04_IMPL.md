# `Debian Boot` IMPL `04`

> Feature: `debian`
> Iteration: `04`
> Plan: `04_PLAN.md`

## Completed Scope

All phases from 04_PLAN implemented and verified:

1. **VirtIO MMIO legacy (v1) block device** — full register set, virtqueue processing, block I/O
2. **Bus-mediated DMA** — `DmaCtx` with `LeBytes` trait, `maybe_process_dma` split-borrow
3. **Machine configuration** — `MachineConfig`, `BootLayout`, `OnceLock`-based `XCPU`
4. **Device tree & build** — `xemu-debian.dts`, `debian.mk`, Makefile wiring
5. **Debian boot verified** — Debian 13 Trixie (riscv64) booted to shell, Python3 executed

## Deviations from Plan

1. **`GuestPageSize` survives reset** — discovered that Linux writes `GuestPageSize` during probe BEFORE the device reset. `Virtqueue::reset()` was incorrectly clearing `page_size`. Fixed by preserving `page_size` across resets since it's a transport-level register.

2. **Disk image source** — plan proposed DQIB or mmdebstrap. Used a pre-built `debian-riscv64.img` (4GB raw ext4) instead. Bootlin rootfs.ext2 (60MB) used as intermediate validation.

3. **RAM size** — plan specified 256MB. Increased to 1GB to accommodate the 4GB Debian image (ext4 metadata needs more RAM for buffer cache).

4. **Kernel** — uses existing bootlin kernel (6.1.44) which has `virtio_blk=y` built-in. Debian's systemd init doesn't start (kernel too minimal), but `/bin/sh` works with full userspace access.

## Verification Results

| Check | Result |
|-------|--------|
| `make fmt` | Clean |
| `make clippy` | No warnings |
| `make test` | 341 passed, 0 failed |
| `make run` (default) | HIT GOOD TRAP — no regression |
| `make linux` (initramfs) | Boots to buildroot shell — no regression |
| `make debian` | Debian 13 Trixie boots, ext4 mounted, shell interactive |

### Debian Boot Validation

```
# cat /etc/os-release
PRETTY_NAME="Debian GNU/Linux 13 (trixie)"
VERSION="13 (trixie)"

# uname -a
Linux (none) 6.1.44 #1 SMP riscv64 GNU/Linux

# dpkg -l | wc -l
288

# ls /bin | wc -l
546

# echo 'print("hello from xemu!")' > /tmp/test.py && python3 /tmp/test.py
hello from xemu!
```

## Files Changed

### Created
- `xemu/xcore/src/device/virtio/mod.rs`
- `xemu/xcore/src/device/virtio/defs.rs` — `BlkReqType`, `BlkStatus`, `DescFlags`
- `xemu/xcore/src/device/virtio/queue.rs` — `Virtqueue`, `Desc`, `poll()`
- `xemu/xcore/src/device/virtio_blk.rs` — `VirtioBlk` with `mmio_regs!` dispatch
- `resource/xemu-debian.dts` — 1GB RAM, virtio,mmio node
- `resource/debian.mk` — build/run targets

### Modified
- `xemu/xcore/src/config/mod.rs` — `MachineConfig`, `BootLayout`
- `xemu/xcore/src/error.rs` — `XError::AlreadyInitialized`
- `xemu/xcore/src/lib.rs` — `init_xcore(config)`, public `config` module
- `xemu/xcore/src/device/mod.rs` — `hard_reset`, `take_notify`, `process_dma` on `Device` trait
- `xemu/xcore/src/device/ram.rs` — `read_bytes`, `write_bytes`
- `xemu/xcore/src/device/bus.rs` — `DmaCtx`, `LeBytes`, `maybe_process_dma`, `hard_reset` in `reset_devices`
- `xemu/xcore/src/cpu/mod.rs` — `OnceLock` XCPU, `BootLayout` in `CPU`, dynamic FDT address
- `xemu/xcore/src/cpu/riscv/mod.rs` — `RVCore::with_config()`, conditional VirtioBlk
- `xemu/xdb/src/main.rs` — `machine_config()`, `X_DISK` env
- `xemu/Makefile` — `DISK`/`X_DISK` export
- `resource/Makefile` — includes `debian.mk`
