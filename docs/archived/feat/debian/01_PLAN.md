# `Debian Boot` PLAN `01`

> Status: Revised
> Feature: `debian`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: none

---

## Summary

Implement a VirtIO MMIO legacy block device and a per-target machine configuration layer so xemu can boot a minimal Debian riscv64 root filesystem from a disk image. This revision narrows scope to offline Debian userspace, introduces a bus-mediated DMA interface, defines a coherent machine-config contract for RAM size / DT / boot addresses, and separates DT generation per boot target.

## Log

[**Feature Introduce**]

- Bus-mediated DMA interface: a new `DmaCtx` handle that devices receive on notification, providing safe guest-memory read/write without raw aliasing.
- Machine configuration contract: a `MachineConfig` struct drives RAM size, device registration, DT selection, and FDT load address from a single source of truth.
- Per-target DT generation: each Makefile target (linux, debian) produces its own DTB with appropriate bootargs and device nodes.
- Snapshot-only disk semantics: writes stay in memory, not flushed to host file. Explicit persistence is deferred.

[**Review Adjustments**]

- R-001: Narrowed G-2 to offline Debian userspace (no apt/networking claims).
- R-002: Replaced raw RAM aliasing (T-3 Option B) with bus-mediated `DmaCtx` design.
- R-003: Added `MachineConfig` contract driving bus, RAM, devices, DT, and boot addresses.
- R-004: Split DT into per-target files; `xemu.dts` remains for initramfs Linux, new `xemu-debian.dts` for disk-root Debian.
- R-005: Made snapshot-only semantics explicit; updated I-3 and validation accordingly.

[**Master Compliance**]

N/A (no master directives in round 00)

### Changes from Previous Round

[**Added**]
- `DmaCtx` — safe guest-memory accessor for DMA-capable devices.
- `MachineConfig` — single source of truth for machine topology.
- Per-target DTS files and build rules.
- Explicit snapshot-only disk contract.

[**Changed**]
- G-2 narrowed: no networking/apt claims.
- RAM sharing: raw pointer → bus-mediated `DmaCtx`.
- DT handling: single shared DTB → per-target DTBs.
- I-3 reworded: disk is a snapshot, not a persistent writeback image.

[**Removed**]
- `mmdebstrap` in-tree build (deferred to follow-up round per TR-2).
- G-2 apt/networking tool validation.
- Raw `Arc<UnsafeCell<Vec<u8>>>` RAM aliasing proposal.

[**Unresolved**]
- U-1: Optimal RAM size for Debian still needs empirical testing (plan uses 256MB as starting point).

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | G-2 narrowed to offline Debian boot/login/local command execution; apt/networking removed from scope and validation |
| Review | R-002 | Accepted | Raw aliasing replaced with bus-mediated `DmaCtx` passed during `QueueNotify` processing; I-1 is now consistent with the design |
| Review | R-003 | Accepted | Added `MachineConfig` contract; RAM size, device list, DT path, and boot addresses all derived from one struct |
| Review | R-004 | Accepted | Per-target DTS files: `xemu.dts` (initramfs), `xemu-debian.dts` (disk root); each Makefile target compiles its own DTB |
| Review | R-005 | Accepted | Disk is snapshot-only (in-memory `Vec<u8>`, no writeback). I-3 and V-IT-2 updated accordingly |
| Review | TR-1 | Adopted | Bus-mediated DMA via `DmaCtx`; see Architecture section |
| Review | TR-2 | Adopted | Pre-built Debian image only; `mmdebstrap` deferred to follow-up |

---

## Spec

[**Goals**]
- G-1: Implement a VirtIO MMIO legacy (v1) block device that Linux's `virtio_mmio` + `virtio_blk` drivers can probe and use for root filesystem.
- G-2: Boot a minimal Debian riscv64 root filesystem to an interactive login shell with basic offline userspace (coreutils, dpkg, systemd-less init).
- G-3: Provide a `make debian` target in `resource/Makefile` that downloads a pre-built Debian image and boots it.
- G-4: Introduce a machine configuration layer so RAM size, devices, DT, and boot addresses are driven from one source of truth.

- NG-1: No virtio-net (networking) in this iteration.
- NG-2: No multi-queue or advanced virtio features.
- NG-3: No packed virtqueue — split virtqueue only.
- NG-4: No disk persistence/writeback — snapshot-only semantics.
- NG-5: No in-tree Debian image builder (mmdebstrap) — download pre-built only.

[**Architecture**]

```
┌──────────────────────────────────────────────────┐
│  Guest Linux Kernel                               │
│  ┌──────────┐  ┌──────────────────────┐          │
│  │virtio_mmio│→│virtio_blk driver     │          │
│  │(DT probe)│  │(read/write sectors)  │          │
│  └────┬─────┘  └──────────┬───────────┘          │
│       │ MMIO R/W          │ virtqueue             │
├───────┼───────────────────┼──────────────────────┤
│  Bus  │                   │                       │
│  ┌────▼────────────────────────────────────┐     │
│  │  Bus::write(QueueNotify)                │     │
│  │    │                                    │     │
│  │    ├─ creates DmaCtx { ram: &mut Ram }  │     │
│  │    ├─ calls virtio_blk.process(&dma)    │     │
│  │    └─ DmaCtx dropped (borrow ends)      │     │
│  └─────────────────────────────────────────┘     │
│                                                   │
│  VirtioBlk @ 0x10001000, IRQ 1                   │
│  ┌─────────────────────────────────────┐         │
│  │  MMIO registers (legacy v1)        │         │
│  │  Config: capacity in 512B sectors  │         │
│  │  Queue: 1 virtqueue, max size 128  │         │
│  │  Backend: Vec<u8> (snapshot)       │         │
│  │                                     │         │
│  │  process(dma: &mut DmaCtx):        │         │
│  │    read avail ring via dma          │         │
│  │    walk descriptor chains           │         │
│  │    T_IN:  disk→guest via dma.write  │         │
│  │    T_OUT: guest→disk via dma.read   │         │
│  │    update used ring via dma.write   │         │
│  │    set interrupt_status             │         │
│  └─────────────────────────────────────┘         │
│                                                   │
│  Host: debian.img loaded as Vec<u8>              │
└──────────────────────────────────────────────────┘
```

Memory map (Debian target, 256MB RAM):
```
0x00100000  SiFive Test Finisher (0x1000)
0x02000000  ACLINT (0x10000)
0x0C000000  PLIC (0x4000000)
0x10000000  UART0 (0x100, PLIC IRQ 10)
0x10001000  VirtIO Block (0x1000, PLIC IRQ 1)
0x80000000  DRAM base (256 MB = 0x10000000)
0x80000000  OpenSBI fw_jump.bin
0x80200000  Kernel (FW_JUMP_ADDR)
0x84000000  Initrd (if present, not used for Debian disk-root boot)
0x8FF00000  FDT (near top of 256MB window)
```

[**Invariants**]
- I-1: VirtioBlk accesses guest RAM exclusively through the `DmaCtx` interface provided by Bus during queue notification — no direct host-memory aliasing or shared pointers.
- I-2: Descriptor chain walks are bounded by queue size to prevent infinite loops on malformed descriptors.
- I-3: Disk image is loaded as a snapshot (`Vec<u8>`). Writes modify the in-memory copy only; the host file is never written back.
- I-4: Interrupt assertion follows VirtIO protocol: set `InterruptStatus` bit 0 after completing used ring entries. PLIC slow-tick routing delivers it.
- I-5: Device state has no side effects on register reads.
- I-6: `MachineConfig` is the single source of truth for RAM size, device set, DT path, and FDT load address. Bus construction, boot loading, and DT content must all agree.

[**Data Structure**]

```rust
/// Safe guest-memory accessor for DMA-capable devices.
/// Created by Bus, lives only for the duration of a queue notification.
pub struct DmaCtx<'a> {
    ram: &'a mut Ram,
}

impl<'a> DmaCtx<'a> {
    /// Read `size` bytes from guest physical address.
    pub fn read(&self, paddr: usize, size: usize) -> XResult<Word>;
    /// Read a contiguous byte slice from guest physical address.
    pub fn read_bytes(&self, paddr: usize, buf: &mut [u8]) -> XResult;
    /// Write `size` bytes to guest physical address.
    pub fn write(&mut self, paddr: usize, size: usize, value: Word) -> XResult;
    /// Write a contiguous byte slice to guest physical address.
    pub fn write_bytes(&mut self, paddr: usize, data: &[u8]) -> XResult;
}
```

```rust
/// VirtIO MMIO legacy (v1) transport + block device.
pub struct VirtioBlk {
    // Transport registers
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
    // Interrupt
    interrupt_status: u32,
    // Block config
    capacity: u64,           // in 512-byte sectors
    // Backend (snapshot)
    disk: Vec<u8>,
}
```

```rust
/// Machine configuration — single source of truth for platform topology.
pub struct MachineConfig {
    pub ram_size: usize,
    pub disk: Option<Vec<u8>>,
    pub fdt_load_addr: usize,
}
```

[**API Surface**]

```rust
// --- DmaCtx (bus.rs) ---
impl Bus {
    /// Process a DMA-capable device notification.
    /// Temporarily borrows RAM to create a DmaCtx, calls the device's
    /// process method, then returns.
    pub fn notify_with_dma(&mut self, device_idx: usize);
}

// --- VirtioBlk (device/virtio_blk.rs) ---
impl VirtioBlk {
    pub fn new(disk: Vec<u8>) -> Self;
    /// Process pending virtqueue requests using DMA context.
    pub fn process(&mut self, dma: &mut DmaCtx);
}

impl Device for VirtioBlk {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn irq_line(&self) -> bool;
    fn reset(&mut self);
}

// --- MachineConfig (cpu/riscv/mod.rs) ---
impl RVCore {
    pub fn with_config(config: MachineConfig) -> Self;
}
```

Key design: when `Bus::write()` dispatches to VirtioBlk and the written offset is `QueueNotify` (0x050), the VirtioBlk stores a "notify pending" flag. After the `Device::write()` returns, Bus checks the flag and calls `notify_with_dma()`, which:
1. Temporarily takes `&mut self.ram` out of `Bus` (via `std::mem::take` or split borrow).
2. Creates `DmaCtx { ram: &mut ram }`.
3. Calls `virtio_blk.process(&mut dma)`.
4. Puts RAM back.

This preserves Bus's single-owner model and requires no unsafe code.

[**Constraints**]
- C-1: Legacy (v1) transport only. Version register returns 1.
- C-2: Single virtqueue (queue 0). `QueueNumMax` = 128.
- C-3: Disk image loaded into `Vec<u8>` as snapshot. Host file not modified.
- C-4: Synchronous request processing on `QueueNotify` — no async I/O.
- C-5: Supported request types: `T_IN` (0), `T_OUT` (1). Others return `S_UNSUPP` (2).
- C-6: Existing `make opensbi`, `make xv6`, `make linux` must work unchanged.
- C-7: RAM size driven by `MachineConfig`. Default remains 128MB for existing targets; Debian target uses 256MB.
- C-8: FDT load address computed as `ram_base + ram_size - 0x100000` (1MB below top).

---

## Implement

### Execution Flow

[**Main Flow**]
1. `MachineConfig` constructed from environment variables (`X_DISK`, `X_MSIZE`).
2. `RVCore::with_config(config)` creates Bus with configured RAM size. If `config.disk` is `Some`, registers VirtioBlk at `0x10001000` with IRQ source 1.
3. Boot loads OpenSBI + kernel + FDT. FDT contains `virtio,mmio` node (Debian target only).
4. Guest kernel probes DT, finds `virtio,mmio`, reads MagicValue (0x74726976), Version (1), DeviceID (2).
5. Guest driver: Reset → Acknowledge → Driver → read features → accept features → set GuestPageSize → configure queue (QueueSel, QueueNum, QueuePFN) → DriverOK.
6. For each I/O:
   a. Guest writes descriptor chain to guest memory (header + data + status).
   b. Guest writes head index to available ring, increments `avail->idx`.
   c. Guest writes 0 to QueueNotify.
   d. `Bus::write()` dispatches to VirtioBlk, which sets `notify_pending = true`.
   e. Bus calls `notify_with_dma()`:
      - Split-borrows RAM from Bus.
      - Creates `DmaCtx`.
      - Calls `virtio_blk.process(&mut dma)`.
   f. `process()`: reads avail ring → walks descriptors → reads header → performs I/O → writes status → updates used ring → sets `interrupt_status |= 1`.
   g. Next slow-tick: PLIC sees IRQ 1 asserted → SEIP → guest handles completion.

[**Failure Flow**]
1. Unknown request type → write `S_UNSUPP` (2) to status byte, complete used ring entry normally.
2. Sector out of range → write `S_IOERR` (1) to status byte.
3. Malformed descriptor chain → log warning, write `S_IOERR`, stop processing that chain.
4. QueueNotify with unconfigured queue (PFN=0) → ignore.
5. Descriptor chain longer than queue size → break, write `S_IOERR`.

[**State Transition**]

- Reset (status written 0) → resets all transport state, clears queue, clears interrupt_status
- status |= ACKNOWLEDGE (1) → device acknowledged
- status |= DRIVER (2) → feature negotiation phase
- status |= DRIVER_OK (4) → device fully operational, I/O permitted
- status |= FAILED (128) → device error, no further I/O

### Implementation Plan

[**Phase 1: DMA Interface & Bus Plumbing**]

File: `xemu/xcore/src/device/bus.rs` + `xemu/xcore/src/device/ram.rs`

1. Add `DmaCtx<'a>` struct in `bus.rs` wrapping `&'a mut Ram`.
2. Implement `DmaCtx::read`, `read_bytes`, `write`, `write_bytes` — delegates to `Ram::get`/`load` with offset calculation (subtract ram base).
3. Add `Ram::read_bytes(&self, offset, buf)` and `Ram::write_bytes(&mut self, offset, data)` for bulk access (the existing `get`/`load` handle this, but we add convenience wrappers that work with physical addresses).
4. Add a `notify_pending` flag protocol in `Device` trait (new optional method `fn take_notify(&mut self) -> bool { false }`).
5. In `Bus::dispatch` for writes: after `dev.write()`, if `dev.take_notify()` returns true, call `self.process_dma(device_idx)`.
6. `Bus::process_dma()`: split-borrows `self.ram` and `self.mmio[idx].dev`, creates `DmaCtx`, calls device's `process_dma` method.
7. Add `Device::process_dma(&mut self, dma: &mut DmaCtx)` default no-op.

[**Phase 2: VirtIO Block Device**]

File: `xemu/xcore/src/device/virtio_blk.rs` (~350 lines)

1. Define MMIO register offset constants.
2. Define VirtIO constants (MAGIC, device status bits, descriptor flags, block request types/status).
3. Implement `VirtioBlk` struct.
4. `Device::read()`: dispatch on offset to return register values. Config space (0x100+) returns capacity.
5. `Device::write()`: dispatch on offset. QueueNotify sets `notify_pending`. Status write of 0 triggers reset.
6. `Device::take_notify()`: return and clear `notify_pending`.
7. `Device::process_dma()`: the core virtqueue processing:
   - Compute vring addresses from `queue_pfn * guest_page_size` and `queue_align`.
   - Loop: read `avail->idx` from guest, process new entries since `last_avail_idx`.
   - For each: read descriptor chain, parse `virtio_blk_outhdr`, do I/O, write status, update used ring.
8. `Device::irq_line()`: `interrupt_status & 1 != 0`.
9. `Device::reset()`: clear all mutable state.
10. Register module in `device/mod.rs`.

[**Phase 3: Machine Configuration & Integration**]

Files: `xemu/xcore/src/config/mod.rs`, `xemu/xcore/src/cpu/riscv/mod.rs`, `xemu/xcore/src/cpu/mod.rs`

1. Add `MachineConfig` to `config/mod.rs` with `ram_size`, `disk`, `fdt_load_addr`.
2. `MachineConfig::fdt_addr(&self) -> usize` computes `CONFIG_MBASE + self.ram_size - 0x100000`.
3. `RVCore::with_config(config)` replaces hardcoded `CONFIG_MSIZE` in `Bus::new()` with `config.ram_size`. Conditionally adds VirtioBlk if `config.disk.is_some()`.
4. Update `CPU` boot code: use `MachineConfig::fdt_addr()` for FDT load address instead of hardcoded `FDT_LOAD_ADDR`.
5. Parse `X_MSIZE` env var (defaults to 128MB). Parse `X_DISK` env var (optional disk image path).
6. Existing `RVCore::new()` remains unchanged (128MB, no disk) for backward compatibility.

[**Phase 4: Device Tree & Boot Infrastructure**]

Files: `resource/xemu-debian.dts`, `resource/debian.mk`, `resource/Makefile`

1. Create `xemu-debian.dts`: copy of `xemu.dts` with:
   - `memory@80000000` reg = 256MB (`0x10000000`).
   - `chosen.bootargs` = `"earlycon=sbi console=ttyS0 root=/dev/vda rw"`.
   - Remove `linux,initrd-start/end` from chosen.
   - Add `virtio_block@10001000` node with `compatible = "virtio,mmio"`, PLIC IRQ 1.
2. Create `resource/debian.mk`:
   - Download target: fetch a known-good minimal Debian riscv64 ext4 image.
   - `run-debian` target: compile DTB, build OpenSBI, boot with `X_DISK`, `X_MSIZE=256M`, appropriate kernel, and `xemu-debian.dtb`.
3. Update `resource/Makefile` to include `debian.mk`.
4. Keep `xemu.dts` unchanged — `make linux` continues to work as before.

[**Phase 5: Testing & Verification**]

1. Unit tests for VirtioBlk register reads (magic, version, device ID, features, config).
2. Unit tests for VirtioBlk register writes (status transitions, queue configuration).
3. Unit tests for DmaCtx read/write operations.
4. Integration: `make debian` boots to Debian login shell.
5. Regression: `make linux` still boots buildroot initramfs.

## Trade-offs

- T-1: **Legacy v1 vs Modern v2 transport**
  - v1: Simpler (single QueuePFN register, no FEATURES_OK). Linux supports it fully including modern kernels.
  - v2: Spec-correct for modern virtio, separate queue address registers.
  - Decision: v1. Upgrade in a follow-up iteration if a specific kernel version requires it.

- T-2: **Disk backend: in-memory Vec<u8> snapshot**
  - Pros: Simple, fast, no file I/O syscalls, no writeback complexity.
  - Cons: Uses host memory equal to image size (~512MB). No persistence across runs.
  - Decision: Snapshot-only. Persistence (writeback to file on shutdown) deferred.

- T-3: **RAM sharing: bus-mediated DmaCtx** (adopted per TR-1)
  - Pros: Preserves Bus's single-owner model, no unsafe code, clear borrow lifetime.
  - Cons: Requires split-borrow plumbing in Bus and new Device trait methods.
  - Decision: DmaCtx. The plumbing cost is modest and the safety benefit is significant. The split-borrow pattern (temporarily moving RAM out of Bus, then putting it back) is idiomatic Rust for this scenario.

- T-4: **Debian image: pre-built download only** (adopted per TR-2)
  - Pros: Minimal host dependencies, fast CI, tight first signal on virtio-blk correctness.
  - Cons: Less customizable, depends on external image availability.
  - Decision: Download only. In-tree mmdebstrap builder in a follow-up round.

## Validation

[**Unit Tests**]
- V-UT-1: MagicValue (0x74726976), Version (1), DeviceID (2) read correctly.
- V-UT-2: Feature negotiation: DeviceFeaturesSel selects pages, DriverFeatures write accepted.
- V-UT-3: Queue configuration: QueueSel, QueueNumMax (128), QueueNum, QueuePFN.
- V-UT-4: Status transitions: 0→1→3→7, write 0 resets.
- V-UT-5: InterruptStatus read / InterruptACK write clears bits.
- V-UT-6: Config space (offset 0x100) returns capacity as little-endian u64.
- V-UT-7: DmaCtx read/write round-trips through Ram.

[**Integration Tests**]
- V-IT-1: `make debian` — kernel probes virtio-blk, mounts `/dev/vda`, reaches login shell.
- V-IT-2: Write a file in Debian shell, read it back in same session (snapshot consistency within a run).
- V-IT-3: `make linux` (initramfs) still boots to buildroot shell unchanged (regression check).

[**Failure / Robustness Validation**]
- V-F-1: QueueNotify with PFN=0 → no crash, no processing.
- V-F-2: Sector beyond capacity → status byte = `S_IOERR` (1).
- V-F-3: Unknown request type → status byte = `S_UNSUPP` (2).
- V-F-4: Circular descriptor chain (next points to self) → bounded by queue size, returns `S_IOERR`.

[**Edge Case Validation**]
- V-E-1: Zero-length data buffer in descriptor chain.
- V-E-2: Request touching the last sector of the disk.
- V-E-3: Batch of multiple requests (avail_idx jumps by >1 between notifications).

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (VirtIO works with Linux) | V-IT-1 |
| G-2 (Debian boots to shell) | V-IT-1, V-IT-2 |
| G-3 (make debian target) | V-IT-1 |
| G-4 (MachineConfig coherence) | V-IT-1 (256MB), V-IT-3 (128MB default) |
| C-1 (Legacy v1) | V-UT-1 |
| C-2 (Single queue, 128) | V-UT-3 |
| C-5 (T_IN, T_OUT only) | V-F-3 |
| C-6 (No regression) | V-IT-3 |
| C-7 (Configurable RAM) | V-IT-1, V-IT-3 |
| I-1 (DmaCtx, no aliasing) | V-UT-7, code review |
| I-3 (Snapshot-only) | V-IT-2 |
