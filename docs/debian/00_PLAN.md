# `Debian Boot` PLAN `00`

> Status: Draft
> Feature: `debian`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

Add a VirtIO block device (virtio-blk-mmio) to xemu so the emulator can boot a minimal Debian root filesystem from a disk image. This extends the existing Linux boot infrastructure (OpenSBI + kernel + initramfs) by replacing the in-RAM initramfs rootfs with a proper block device-backed ext4 root filesystem, enabling a full-featured userspace that exercises the emulator far more thoroughly than buildroot.

## Log

[**Feature Introduce**]

This is the initial plan. Two major components:
1. **VirtIO MMIO block device** — a new MMIO device implementing the VirtIO legacy (v1) transport with block device backend, supporting read/write operations via split virtqueues.
2. **Debian rootfs build infrastructure** — Makefile targets and scripts to create a minimal Debian riscv64 disk image using `debootstrap`/`mmdebstrap`, and a `debian` make target to boot it.

[**Review Adjustments**]

N/A (first iteration)

[**Master Compliance**]

N/A (first iteration)

### Changes from Previous Round

[**Added**]

Everything is new.

[**Changed**]

N/A

[**Removed**]

N/A

[**Unresolved**]

- U-1: Optimal RAM size for Debian (256MB minimum, 512MB comfortable). Needs testing.
- U-2: Whether modern (v2) transport is needed — starting with legacy (v1) which Linux supports.

### Response Matrix

N/A (first iteration)

---

## Spec

[**Goals**]
- G-1: Implement a VirtIO MMIO block device that Linux's `virtio_mmio` + `virtio_blk` drivers can probe and use.
- G-2: Boot a minimal Debian riscv64 root filesystem to an interactive shell with working `apt`, `dpkg`, networking tools.
- G-3: Provide a `make debian` target in `resource/Makefile` that builds/downloads the Debian image and boots it.

- NG-1: No virtio-net (networking) in this iteration.
- NG-2: No multi-queue or advanced virtio features (discard, write-zeroes, topology).
- NG-3: No packed virtqueue (v1.1+) — split virtqueue only.

[**Architecture**]

```
┌─────────────────────────────────────────────┐
│  Guest Linux Kernel                          │
│  ┌──────────┐  ┌──────────────────────┐     │
│  │virtio_mmio│→│virtio_blk driver     │     │
│  │(DT probe)│  │(read/write sectors)  │     │
│  └────┬─────┘  └──────────┬───────────┘     │
│       │ MMIO R/W          │ virtqueue        │
├───────┼───────────────────┼─────────────────┤
│  Bus  │                   │                  │
│  ┌────▼──────────────────────────────┐      │
│  │  VirtioBlk MMIO device            │      │
│  │  @ 0x10001000, size 0x1000        │      │
│  │  IRQ source: 1 (PLIC)            │      │
│  │                                    │      │
│  │  Registers (legacy v1 transport)  │      │
│  │  Config: capacity in sectors      │      │
│  │  Queue: 1 virtqueue, size 128     │      │
│  │                                    │      │
│  │  Backend: File (raw disk image)   │      │
│  └───────────────┬───────────────────┘      │
│                  │                           │
├──────────────────┼───────────────────────────┤
│  Host            │                           │
│  ┌───────────────▼───────────────────┐      │
│  │  debian.img (ext4, ~512MB)        │      │
│  │  Created by mmdebstrap            │      │
│  └───────────────────────────────────┘      │
└─────────────────────────────────────────────┘
```

Memory map addition:
```
0x10001000  VirtIO block device (MMIO, 0x1000 bytes, PLIC IRQ 1)
```

The device sits in the SoC address space alongside UART (0x10000000). The PLIC routes IRQ 1 to the CPU. The device tree gains a `virtio_block@10001000` node.

[**Invariants**]
- I-1: The VirtIO device must only access guest RAM through the Bus's existing `read`/`write`/`load_ram` methods — no direct host memory aliasing.
- I-2: Descriptor chain walks must be bounded by queue size to prevent infinite loops on malformed descriptors.
- I-3: The disk image file is opened read-write but the device never grows it — all sectors must be within the advertised capacity.
- I-4: Interrupt assertion follows the VirtIO protocol: set `InterruptStatus` bit 0, let PLIC slow-tick routing deliver it.
- I-5: Device state is immutable during register reads — no side effects on read.

[**Data Structure**]

```rust
/// VirtIO MMIO legacy transport + block device.
pub struct VirtioBlk {
    // Transport state
    status: u32,
    device_features: u64,
    driver_features: u64,
    guest_page_size: u32,
    // Queue state
    queue_sel: u32,
    queue_num: u32,
    queue_pfn: u32,
    queue_align: u32,
    last_avail_idx: u16,
    // Interrupt state
    interrupt_status: u32,
    // Block device config
    capacity: u64,           // in 512-byte sectors
    // Backend
    disk: Vec<u8>,           // mmap'd or loaded disk image
}
```

```rust
/// VirtIO descriptor (16 bytes, read from guest memory).
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

/// Block request header (16 bytes, read from guest memory).
struct VirtioBlkReq {
    req_type: u32,
    _reserved: u32,
    sector: u64,
}
```

[**API Surface**]

```rust
impl VirtioBlk {
    /// Create a new VirtIO block device backed by a disk image.
    pub fn new(disk: Vec<u8>) -> Self;
}

impl Device for VirtioBlk {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) { /* no-op, request processing is synchronous on QueueNotify */ }
    fn irq_line(&self) -> bool;
    fn reset(&mut self);
}
```

Request processing is triggered synchronously on `QueueNotify` write (offset 0x050). The device:
1. Reads available ring entries from guest RAM.
2. Walks descriptor chains, reading request headers and data.
3. Performs the I/O on the backing `Vec<u8>`.
4. Writes status byte and used ring entries back to guest RAM.
5. Sets `interrupt_status |= 1`.

**Guest RAM access**: The device needs to read/write guest physical memory to process virtqueue descriptors and data buffers. This requires passing a reference to the Bus's RAM. Two approaches:

- **Option A**: Share a reference to RAM (`Arc<Mutex<Ram>>`) — adds complexity and contention.
- **Option B**: Process requests during `write()` to `QueueNotify`, passing the Bus's RAM as a parameter — but `Device::write()` doesn't have a Bus reference.
- **Option C**: Give VirtioBlk its own `Arc<[u8]>` pointer to the same backing memory as Bus's RAM. RAM is already a contiguous `Vec<u8>`; we can share the backing via `Arc`.

We choose **Option C**: share the RAM backing store. The `Ram` device's inner buffer becomes `Arc<UnsafeCell<Vec<u8>>>` (or simpler: use raw pointer with appropriate safety invariants). The VirtioBlk receives a handle to read/write guest memory directly.

[**Constraints**]
- C-1: Legacy (v1) transport only. Version register returns 1. No FEATURES_OK handshake.
- C-2: Single virtqueue (queue 0). `QueueNumMax` = 128.
- C-3: Disk image loaded into memory as `Vec<u8>` (simplest backend). For a 512MB image this uses 512MB host memory — acceptable for an emulator.
- C-4: Synchronous request processing on `QueueNotify` — no async/threaded I/O.
- C-5: Supported request types: `T_IN` (read, type=0), `T_OUT` (write, type=1). All others return `S_UNSUPP`.
- C-6: Must not break existing `make opensbi`, `make xv6`, `make linux` targets.
- C-7: RAM size must be configurable (increase to 256MB+ for Debian).

---

## Implement

### Execution Flow

[**Main Flow**]
1. Host loads disk image file into `Vec<u8>`.
2. `VirtioBlk::new(disk)` creates the device with `capacity = disk.len() / 512`.
3. Device is registered on Bus at `0x10001000` with IRQ source 1.
4. Guest kernel probes DT, finds `virtio,mmio` node, reads MagicValue/Version/DeviceID.
5. Guest driver negotiates features, configures queue (writes GuestPageSize, QueueNum, QueuePFN).
6. Guest driver sets Status = DRIVER_OK.
7. For each I/O request:
   a. Guest driver builds descriptor chain (header + data + status) in guest memory.
   b. Guest driver writes head index to available ring, increments `avail->idx`.
   c. Guest driver writes queue index (0) to QueueNotify register.
   d. Device reads available ring, walks descriptor chain.
   e. Device reads `virtio_blk_outhdr` (type, sector) from guest memory.
   f. For T_IN: copies sectors from `disk` to guest buffer. For T_OUT: copies guest buffer to `disk`.
   g. Device writes status byte (0 = OK) to guest memory.
   h. Device writes used ring entry (id, len), increments `used->idx`.
   i. Device sets `interrupt_status |= 1`.
   j. PLIC delivers IRQ 1 → SEIP → guest handles completion.

[**Failure Flow**]
1. Unknown request type → write `S_UNSUPP` (2) to status byte, still complete the used ring entry.
2. Sector out of range → write `S_IOERR` (1) to status byte.
3. Malformed descriptor chain (missing next, bad address) → log warning, write `S_IOERR`.
4. Queue not configured (PFN=0) on QueueNotify → ignore silently.

[**State Transition**]

- Reset (status=0) → Acknowledge (status=1) → Driver (status=3) → DriverOK (status=7)
- Any state → Reset (write 0 to status register)
- Any state → Failed (status |= 128)

### Implementation Plan

[**Phase 1: VirtIO Block Device**]

File: `xemu/xcore/src/device/virtio_blk.rs` (~300 lines)

1. Define MMIO register constants (offsets 0x000–0x100+).
2. Implement `VirtioBlk` struct with transport + queue + block state.
3. Implement `Device::read()` — return register values based on offset.
4. Implement `Device::write()` — handle register writes, trigger request processing on QueueNotify.
5. Implement virtqueue processing:
   - Read available ring from guest RAM.
   - Walk descriptor chains.
   - Parse `virtio_blk_outhdr`.
   - Read/write sectors from backing store.
   - Update used ring in guest RAM.
   - Set interrupt status.
6. Implement `Device::irq_line()` → `interrupt_status & 1 != 0`.
7. Implement `Device::reset()` → clear all state.

[**Phase 2: RAM Sharing & Bus Integration**]

1. Modify `Ram` to expose a shared handle for guest memory access by VirtioBlk.
2. Register VirtioBlk on the Bus in `RVCore` initialization (when disk image env var is set).
3. Add `X_DISK` environment variable for disk image path.
4. Increase configurable RAM size (add `X_MSIZE` env var or increase default for Debian).

[**Phase 3: Device Tree & Boot Infrastructure**]

1. Add `virtio_block@10001000` node to `xemu.dts`.
2. Update `chosen.bootargs` to include `root=/dev/vda rw` for Debian boot.
3. Create `resource/debian.mk`:
   - Target to build a minimal Debian riscv64 ext4 image via `mmdebstrap`.
   - `run-debian` target that boots with the disk image.
4. Create `resource/patches/debian/` for any needed customization scripts.

[**Phase 4: Testing & Verification**]

1. Unit tests for VirtioBlk register read/write.
2. Unit tests for virtqueue descriptor chain parsing.
3. Integration test: boot Linux with virtio-blk root filesystem.
4. Verify `make debian` boots to interactive Debian shell.

## Trade-offs

- T-1: **Legacy v1 vs Modern v2 transport**
  - v1: Simpler (no FEATURES_OK, single QueuePFN register). Linux supports it fully. Risk: Linux may eventually deprecate legacy support (but not soon).
  - v2: More correct per spec, separate queue address registers. More complex to implement.
  - Proposal: Start with v1, upgrade to v2 in a later iteration if needed.

- T-2: **Disk backend: Vec<u8> in-memory vs File I/O**
  - Vec<u8>: Simple, fast, no syscall overhead. Uses host memory equal to image size.
  - File I/O (seek+read/write): Lower memory usage, but adds syscall latency per request.
  - Proposal: Use Vec<u8> for simplicity. 512MB host memory is acceptable.

- T-3: **RAM sharing mechanism**
  - Option A: `Arc<Mutex<Vec<u8>>>` — safe but adds lock contention on every access.
  - Option B: Raw pointer with safety comment — zero overhead, matches emulator's single-threaded execution model.
  - Option C: Process virtqueue during Bus `write()` dispatch, passing RAM ref — requires changing Device trait or adding a callback.
  - Proposal: Option B (raw pointer). The emulator is single-threaded; the CPU executes instructions sequentially, so there's no data race.

- T-4: **Debian image creation: mmdebstrap vs pre-built image**
  - mmdebstrap: Reproducible, customizable, but requires QEMU user-mode for cross-arch package installation.
  - Pre-built: Download a known-good image. Less flexible but simpler.
  - Proposal: Provide both — a download target for quick start, and a build-from-scratch target for customization.

## Validation

[**Unit Tests**]
- V-UT-1: MagicValue/Version/DeviceID read returns correct constants.
- V-UT-2: Feature negotiation sequence (read device features, write driver features).
- V-UT-3: Queue configuration (write QueueSel, read QueueNumMax, write QueueNum/QueuePFN).
- V-UT-4: Status register transitions (reset → acknowledge → driver → driver_ok).
- V-UT-5: InterruptStatus/InterruptACK read/write behavior.
- V-UT-6: Config space read returns correct capacity.

[**Integration Tests**]
- V-IT-1: Boot Linux with virtio-blk root filesystem — kernel mounts `/dev/vda`, reaches shell.
- V-IT-2: Read/write files on the mounted filesystem — data persists across operations.
- V-IT-3: Existing `make linux` (initramfs boot) still works unchanged.

[**Failure / Robustness Validation**]
- V-F-1: QueueNotify with unconfigured queue (PFN=0) does not crash.
- V-F-2: Out-of-bounds sector access returns IOERR status.
- V-F-3: Unknown request type returns UNSUPP status.
- V-F-4: Malformed descriptor chain (circular, out-of-bounds) doesn't cause infinite loop.

[**Edge Case Validation**]
- V-E-1: Zero-length read/write request.
- V-E-2: Request spanning last sector of disk.
- V-E-3: Multiple requests in a single available ring batch (avail_idx jumps by >1).

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (VirtIO device works with Linux) | V-IT-1 |
| G-2 (Debian boots to shell) | V-IT-1, manual verification |
| G-3 (make debian target) | V-IT-3, manual verification |
| C-1 (Legacy v1 only) | V-UT-1 (Version=1) |
| C-2 (Single queue, size 128) | V-UT-3 |
| C-5 (T_IN, T_OUT only) | V-F-3 |
| C-6 (No regression) | V-IT-3 |
