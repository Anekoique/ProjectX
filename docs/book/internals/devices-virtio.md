# VirtIO-blk

MMIO legacy (version 1) transport backing the Debian rootfs.

## Layout

| Region | Address | Size |
|--------|---------|------|
| VirtIO MMIO | `0x1000_1000` | `0x1000` |

## Transport

- **Legacy MMIO v1** — matches Linux's `virtio_mmio` driver without
  needing the modern `VIRTIO_F_VERSION_1` path.
- **Split virtqueue**, 128 entries.
- **Synchronous DMA processing** — `process_dma` reads descriptors
  from guest RAM, dispatches the request, writes status, rings the
  used ring.

## `DmaCtx`

Bus-mediated guest-memory accessor — the only bridge between VirtIO
code and guest RAM:

```rust
impl<'a> DmaCtx<'a> {
    pub fn read_bytes(&mut self, gpa: u64, buf: &mut [u8]) -> XResult<()>;
    pub fn write_bytes(&mut self, gpa: u64, buf: &[u8]) -> XResult<()>;
    pub fn read_le<T: LeBytes>(&mut self, gpa: u64) -> XResult<T>;
    pub fn write_le<T: LeBytes>(&mut self, gpa: u64, v: T) -> XResult<()>;
}
```

The `LeBytes` trait is the type-safe layer — no `unsafe` aliasing of
guest memory, just bounded `&mut [u8]` views through the Bus.

## `BlkStorage`

Separated from the transport state so Rust's borrow checker can
split them:

```rust
struct VirtioBlk {
    transport: TransportState,   // queue pointers, device status
    storage:   BlkStorage,        // backing snapshot
}
```

`process_dma` borrows `&mut TransportState` + `&mut BlkStorage`
disjointly — no interior mutability, no runtime borrow tracking.

## Two-tier reset

- **Transport reset** — `QueueReady` goes to 0; `QueueSel` is
  cleared; disk contents preserved.
- **Emulator hard reset** — via the test finisher; restores the disk
  to the snapshot recorded at load.

## Debian image

`resource/xemu-debian.img` — 4 GiB ext4 filesystem with Debian 13
Trixie pre-installed. Build system downloads it on first `make debian`.
