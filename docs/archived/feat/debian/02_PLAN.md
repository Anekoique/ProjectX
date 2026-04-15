# `Debian Boot` PLAN `02`

> Status: Revised
> Feature: `debian`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: none

---

## Summary

Implement a VirtIO MMIO legacy block device for booting Debian on xemu. This revision resolves the two remaining blocking issues by (a) wiring `MachineConfig` into the actual `XCPU` construction path through `init_xcore()`, and (b) adopting a fixed Debian machine profile with its own static DTS instead of claiming runtime-configurable RAM.

## Log

[**Feature Introduce**]

- Fixed machine profiles: instead of a general-purpose `MachineConfig` with runtime RAM override, define two concrete profiles — `default` (128MB, no disk) and `debian` (256MB, virtio-blk). Each profile has a matching static DTS. No runtime/DT coherence drift.
- `init_xcore(config)` replaces `init_xcore()`: the `XCPU` singleton is constructed with a `MachineConfig` at init time, not from a hardcoded `Core::new()`. This makes the config reach the real runtime path.
- Disk reset semantics: `Device::reset()` on VirtioBlk restores the original snapshot (preserves a clone of the initial image).

[**Review Adjustments**]

- R-001: `XCPU` construction now takes `MachineConfig`. `init_xcore(config)` builds the core from it. The `LazyLock` is replaced with `OnceLock` initialized by `init_xcore()`.
- R-002: Dropped runtime `X_MSIZE` override. Debian target is a fixed 256MB profile with a static DTS that matches. No DT/RAM drift possible.
- R-003: `MachineConfig` normalized — only independent inputs (`ram_size`, `disk`), derived values computed. `fdt_load_addr` removed from struct.
- R-004: Disk reset restores original snapshot. Validation added (V-F-5).

[**Master Compliance**]

N/A

### Changes from Previous Round

[**Added**]
- `init_xcore(config)` signature change — config-aware core construction.
- `OnceLock` replaces `LazyLock` for `XCPU` — initialized at startup rather than on first access.
- Disk reset-to-original semantics with validation.
- Fixed machine profiles concept.

[**Changed**]
- `MachineConfig` simplified: no `fdt_load_addr` field, no DT selection — just `ram_size` and `disk`.
- Removed `X_MSIZE` env var — Debian Makefile target hardcodes 256MB via `MachineConfig`.
- `init_xcore()` → `init_xcore(config: MachineConfig)`.

[**Removed**]
- Runtime RAM size override (`X_MSIZE` env var).
- `fdt_load_addr` from `MachineConfig` struct.
- General-purpose "single source of truth" rhetoric — replaced with concrete fixed profiles.

[**Unresolved**]
- None.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | `XCPU` constructed from `MachineConfig` via `init_xcore(config)`. `LazyLock` → `OnceLock` initialized at startup. `boot_config()` in xdb parses `X_DISK` to choose profile. |
| Review | R-002 | Accepted | Dropped runtime `X_MSIZE`. Debian = fixed 256MB profile + matching static DTS. No DT/RAM drift. Adopted TR-1. |
| Review | R-003 | Accepted | `MachineConfig` holds only `ram_size` and `disk`. FDT address computed by caller. |
| Review | R-004 | Accepted | `VirtioBlk::reset()` restores original snapshot from a stored clone. Added V-F-5. |
| Review | TR-1 | Adopted | Fixed Debian machine profile, no general runtime configurability this round. |

---

## Spec

[**Goals**]
- G-1: Implement a VirtIO MMIO legacy (v1) block device that Linux's `virtio_mmio` + `virtio_blk` drivers can probe and use as root filesystem.
- G-2: Boot a minimal Debian riscv64 root filesystem to an interactive login shell with basic offline userspace (coreutils, dpkg, shell).
- G-3: Provide a `make debian` target that downloads a pre-built Debian image and boots it.
- G-4: Wire machine configuration into the real `XCPU` construction path so RAM size and device set are determined at init time.

- NG-1: No virtio-net in this iteration.
- NG-2: No multi-queue or advanced virtio features.
- NG-3: No packed virtqueue.
- NG-4: No disk persistence/writeback — snapshot-only.
- NG-5: No in-tree Debian image builder.
- NG-6: No runtime RAM size override — each target has a fixed profile.

[**Architecture**]

```
Startup Flow:
  xdb main()
    → init_xdb()
    → machine_config()      // parse X_DISK env → choose profile
    → init_xcore(config)    // construct XCPU with correct RAM + devices
    → boot_config()         // parse X_FW/X_KERNEL/X_FDT env
    → run(boot_config)      // boot + execute

Machine Profiles:
  Default:  { ram_size: 128MB, disk: None  }  ← make opensbi/xv6/linux
  Debian:   { ram_size: 256MB, disk: Some } ← make debian

XCPU Construction (init_xcore):
  OnceLock<Mutex<CPU<Core>>>
    ↓ init_xcore(config)
  Core::with_config(config)
    → Bus::new(CONFIG_MBASE, config.ram_size)
    → add ACLINT, PLIC, UART, Finisher (always)
    → if config.disk.is_some(): add VirtioBlk @ 0x10001000, IRQ 1
    → CPU::new(core)
```

```
DMA Flow (on QueueNotify write):
  Bus::write(0x10001000 + 0x50, ...)
    → VirtioBlk::write() sets notify_pending
    → Bus detects notify_pending
    → Bus::process_dma():
        split-borrow: take &mut ram, take &mut mmio[virtio_idx].dev
        create DmaCtx { ram }
        call virtio_blk.process_dma(&mut dma)
        (borrow ends, ram returned)
```

Memory map — Default profile (128MB, existing targets):
```
0x00100000  Finisher
0x02000000  ACLINT
0x0C000000  PLIC
0x10000000  UART0 (IRQ 10)
0x80000000  DRAM (128MB = 0x08000000)
0x87F00000  FDT
```

Memory map — Debian profile (256MB, virtio-blk):
```
0x00100000  Finisher
0x02000000  ACLINT
0x0C000000  PLIC
0x10000000  UART0 (IRQ 10)
0x10001000  VirtIO Block (IRQ 1)
0x80000000  DRAM (256MB = 0x10000000)
0x8FF00000  FDT
```

[**Invariants**]
- I-1: VirtioBlk accesses guest RAM exclusively through `DmaCtx` — no raw aliasing.
- I-2: Descriptor chain walks bounded by queue size.
- I-3: Disk is a snapshot. Writes modify in-memory copy only. `reset()` restores original.
- I-4: Interrupt: `InterruptStatus` bit 0 set after used ring update. PLIC slow-tick delivers.
- I-5: No side effects on register reads.
- I-6: Each machine profile is a fixed (RAM size, device set) pair with a matching static DTS. No runtime override can cause DT/RAM mismatch.

[**Data Structure**]

```rust
/// Machine configuration — independent inputs only.
pub struct MachineConfig {
    pub ram_size: usize,
    pub disk: Option<Vec<u8>>,
}

impl MachineConfig {
    /// Default profile: 128MB, no disk.
    pub fn default() -> Self {
        Self { ram_size: CONFIG_MSIZE, disk: None }
    }

    /// Debian profile: 256MB, disk from file.
    pub fn with_disk(disk: Vec<u8>) -> Self {
        Self { ram_size: 256 * 1024 * 1024, disk: Some(disk) }
    }
}
```

```rust
/// Safe guest-memory accessor for DMA-capable devices.
pub struct DmaCtx<'a> {
    ram: &'a mut Ram,
}

impl<'a> DmaCtx<'a> {
    pub fn read_bytes(&self, paddr: usize, buf: &mut [u8]) -> XResult;
    pub fn write_bytes(&mut self, paddr: usize, data: &[u8]) -> XResult;
}
```

```rust
/// VirtIO MMIO legacy (v1) transport + block device.
pub struct VirtioBlk {
    // Transport
    status: u32,
    device_features: u64,
    driver_features: u64,
    guest_page_size: u32,
    // Queue
    queue_sel: u32,
    queue_num: u32,
    queue_pfn: u32,
    queue_align: u32,
    last_avail_idx: u16,
    // Interrupt
    interrupt_status: u32,
    // Block
    capacity: u64,
    disk: Vec<u8>,
    original: Vec<u8>,   // snapshot baseline for reset
}
```

[**API Surface**]

```rust
// --- xcore public API ---
pub fn init_xcore(config: MachineConfig) -> XResult;

// --- Bus (bus.rs) ---
impl Bus {
    pub fn new(ram_base: usize, ram_size: usize) -> Self;
    // After Device::write, if notify pending, process DMA:
    fn process_dma(&mut self, device_idx: usize);
}

// --- Device trait extension ---
pub trait Device: Send {
    // ... existing methods ...
    /// Return true if device needs DMA processing after a write.
    fn take_notify(&mut self) -> bool { false }
    /// Process DMA-capable operations with guest memory access.
    fn process_dma(&mut self, _dma: &mut DmaCtx) {}
}

// --- VirtioBlk ---
impl VirtioBlk {
    pub fn new(disk: Vec<u8>) -> Self;
}
// Device trait implementation handles read/write/irq_line/reset/take_notify/process_dma

// --- RVCore ---
impl RVCore {
    pub fn new() -> Self;                        // default profile (backward compat)
    pub fn with_config(config: MachineConfig) -> Self; // config-aware
}
```

[**Constraints**]
- C-1: Legacy (v1) transport only. Version register returns 1.
- C-2: Single virtqueue (queue 0). `QueueNumMax` = 128.
- C-3: Disk snapshot — host file never modified.
- C-4: Synchronous request processing on `QueueNotify`.
- C-5: `T_IN` (0) and `T_OUT` (1) only. Others → `S_UNSUPP`.
- C-6: Existing `make opensbi`, `make xv6`, `make linux` unchanged.
- C-7: Debian profile fixed at 256MB. No runtime RAM override.
- C-8: FDT load address: computed per-profile in Makefile and `load_firmware()`. Default = `0x87F00000`, Debian = `0x8FF00000`.

---

## Implement

### Execution Flow

[**Main Flow**]
1. `xdb::main()` calls `machine_config()` — reads `X_DISK` env var. If set, loads disk file into `Vec<u8>` and returns `MachineConfig::with_disk(disk)`. Otherwise returns `MachineConfig::default()`.
2. `init_xcore(config)` builds `Core::with_config(config)`, wraps in `CPU::new(core)`, stores in `OnceLock<Mutex<CPU<Core>>>`.
3. `boot_config()` reads `X_FW`/`X_KERNEL`/`X_FDT` env vars as before.
4. `run(boot_config)` calls `cpu.boot(config)` → `reset()` → `load_firmware()` → kernel starts.
5. Guest probes `virtio,mmio` DT node → reads Magic/Version/DeviceID → negotiates → configures queue → DRIVER_OK.
6. Guest writes sector requests to virtqueue → writes QueueNotify → Bus processes DMA → I/O completes → interrupt asserted.
7. Guest mounts `/dev/vda` as root → boots to Debian login shell.

[**Failure Flow**]
1. `X_DISK` points to nonexistent file → `machine_config()` returns error → process exits with message.
2. Unknown request type → `S_UNSUPP` status byte.
3. Out-of-range sector → `S_IOERR` status byte.
4. Malformed descriptor chain → bounded walk, `S_IOERR`.
5. QueueNotify with PFN=0 → silently ignored.

[**State Transition**]
- VirtIO status: Reset(0) → Acknowledge(1) → Driver(3) → DriverOK(7)
- Write 0 to status → full reset (clears queue, interrupt, restores original disk)
- `Device::reset()` (from `cpu.reset()`) → same as status=0 reset

### Implementation Plan

[**Phase 1: XCPU Construction Refactor**]

Files: `xemu/xcore/src/lib.rs`, `xemu/xcore/src/cpu/mod.rs`, `xemu/xcore/src/config/mod.rs`, `xemu/xdb/src/main.rs`

1. Add `MachineConfig` to `config/mod.rs`.
2. Change `XCPU` from `LazyLock<Mutex<CPU<Core>>>` to `OnceLock<Mutex<CPU<Core>>>`.
3. Change `init_xcore()` → `init_xcore(config: MachineConfig)`. Inside: construct `Core::with_config(config)`, wrap in `CPU::new(core)`, store in `OnceLock`.
4. Add `with_xcpu` wrapper that panics with clear message if `OnceLock` not yet initialized.
5. In `xdb/src/main.rs`: add `machine_config()` function that reads `X_DISK`, constructs `MachineConfig`. Call `init_xcore(machine_config())` before `boot_config()`.
6. Add `FDT_LOAD_ADDR` computation based on `MachineConfig::ram_size` in `load_firmware()`: `CONFIG_MBASE + ram_size - 0x10_0000`.
7. `RVCore::new()` kept as `with_config(MachineConfig::default())` for backward compat (tests).

[**Phase 2: DMA Interface**]

Files: `xemu/xcore/src/device/mod.rs`, `xemu/xcore/src/device/bus.rs`, `xemu/xcore/src/device/ram.rs`

1. Add `DmaCtx<'a>` struct in `bus.rs`.
2. Add `Ram::read_bytes(&self, offset, buf)` and `Ram::write_bytes(&mut self, offset, data)` methods.
3. Implement `DmaCtx::read_bytes` and `write_bytes` — offset-adjusts paddr by RAM base, delegates to Ram.
4. Add `take_notify()` and `process_dma()` to `Device` trait with default no-ops.
5. Modify `Bus::dispatch` for writes: after `dev.write()`, check `dev.take_notify()`. If true, call `self.process_dma(idx)`.
6. `Bus::process_dma(idx)`: use index-based split borrow — `self.mmio` is a Vec, borrow `self.ram` mutably and `self.mmio[idx].dev` mutably via safe index manipulation. Create `DmaCtx`, call `device.process_dma(&mut dma)`.

[**Phase 3: VirtIO Block Device**]

File: `xemu/xcore/src/device/virtio_blk.rs` (~350 lines)

1. MMIO register constants, VirtIO constants (magic, status bits, descriptor flags, blk request types).
2. `VirtioBlk::new(disk)`: compute capacity, clone disk to `original`.
3. `Device::read()`: match offset → return register/config values.
4. `Device::write()`: match offset → update registers. QueueNotify → set `notify_pending`.
5. `Device::take_notify()`: return+clear flag.
6. `Device::process_dma(dma)`:
   - If queue not configured (PFN=0), return.
   - Compute vring base: `queue_pfn * guest_page_size`.
   - Descriptor table at base, avail ring at `base + 16*queue_num`, used ring at `align_up(base + 16*queue_num + 6 + 2*queue_num, queue_align)`.
   - Read `avail->idx` from guest. Loop from `last_avail_idx` to `avail_idx`:
     - Read `avail->ring[i % queue_num]` → head descriptor index.
     - Walk chain (max `queue_num` steps): read descriptors, identify header/data/status.
     - Parse header: type (u32), reserved (u32), sector (u64).
     - T_IN: read from `disk[sector*512..]`, write to guest via `dma.write_bytes`.
     - T_OUT: read from guest via `dma.read_bytes`, write to `disk[sector*512..]`.
     - Write status byte (0=OK, 1=IOERR, 2=UNSUPP) to guest.
     - Write used ring entry: `{ id: head, len: bytes_written }`.
     - Increment used->idx.
   - Set `interrupt_status |= 1`.
   - Update `last_avail_idx`.
7. `Device::irq_line()`: `interrupt_status & 1 != 0`.
8. `Device::reset()`: clear transport state, restore `disk` from `original.clone()`.
9. Register `pub mod virtio_blk;` in `device/mod.rs`.

[**Phase 4: Integration — RVCore + Bus Wiring**]

File: `xemu/xcore/src/cpu/riscv/mod.rs`

1. `RVCore::with_config(config)`:
   - `Bus::new(CONFIG_MBASE, config.ram_size)`.
   - Add ACLINT, PLIC, UART, Finisher (same as `new()`).
   - If `config.disk` is `Some(disk)`: add VirtioBlk at `0x1000_1000`, size `0x1000`, IRQ source 1.
   - Return `Self::with_bus(bus, irq)`.
2. `RVCore::new()` → delegates to `with_config(MachineConfig::default())`.

[**Phase 5: Device Tree & Build Infrastructure**]

Files: `resource/xemu-debian.dts`, `resource/debian.mk`, `resource/Makefile`

1. `xemu-debian.dts`:
   ```dts
   chosen { bootargs = "earlycon=sbi console=ttyS0 root=/dev/vda rw"; };
   memory@80000000 { reg = <0x0 0x80000000 0x0 0x10000000>; }; // 256MB
   soc {
     // ... existing ACLINT, PLIC, UART, Finisher ...
     virtio_block@10001000 {
       compatible = "virtio,mmio";
       reg = <0x0 0x10001000 0x0 0x1000>;
       interrupts = <1>;
       interrupt-parent = <&plic>;
     };
   };
   ```
2. `debian.mk`:
   - `DEBIAN_IMG_URL` pointing to a known-good riscv64 Debian minimal ext4 image.
   - `fetch-debian`: download image.
   - `run-debian`: compile `xemu-debian.dtb`, build OpenSBI, run xemu with `DISK=$(DEBIAN_IMG)`, `FDT=$(DEBIAN_DTB)`, appropriate kernel.
3. `resource/Makefile`: add `include debian.mk`, add `debian` phony target.
4. `xemu.dts` unchanged.

[**Phase 6: Testing**]

1. Unit tests: VirtioBlk register read/write, status transitions, config space.
2. Unit tests: DmaCtx read_bytes/write_bytes through Ram.
3. Unit tests: VirtioBlk reset restores original disk snapshot.
4. Integration: `make debian` boots to login shell.
5. Regression: `make linux` unchanged.

## Trade-offs

- T-1: **Fixed profiles vs runtime configurability**
  - Decision: Fixed profiles (adopted per TR-1). Debian = 256MB with matching static DTS. No drift possible. Runtime configurability deferred.

- T-2: **Disk reset semantics: restore-original vs preserve-within-process**
  - Restore-original: `reset()` clones `original` back to `disk`. Clean state for debugging. Costs one `Vec::clone()` per reset.
  - Preserve: `reset()` keeps modified `disk`. Cheaper but less predictable.
  - Decision: Restore-original. Debugging workflows expect clean resets, and the clone cost is negligible compared to re-reading the file.

- T-3: **`LazyLock` vs `OnceLock` for XCPU**
  - `LazyLock`: init on first access, closure captures no config → stuck with `Core::new()`.
  - `OnceLock`: explicit init at startup, caller provides config → `Core::with_config(config)`.
  - Decision: `OnceLock`. Slightly more ceremony in `init_xcore`, but required for config-aware construction.

## Validation

[**Unit Tests**]
- V-UT-1: MagicValue (0x74726976), Version (1), DeviceID (2).
- V-UT-2: Feature negotiation (sel pages, read/write features).
- V-UT-3: Queue config (sel, num_max=128, num, pfn).
- V-UT-4: Status transitions (0→1→3→7; write 0 resets).
- V-UT-5: InterruptStatus/InterruptACK.
- V-UT-6: Config space capacity (little-endian u64 at offset 0x100).
- V-UT-7: DmaCtx read_bytes/write_bytes round-trip.
- V-UT-8: `MachineConfig::default()` → 128MB, no disk. `MachineConfig::with_disk()` → 256MB.

[**Integration Tests**]
- V-IT-1: `make debian` → kernel probes virtio-blk, mounts `/dev/vda`, reaches login shell.
- V-IT-2: File write+read within same Debian session (snapshot consistency).
- V-IT-3: `make linux` still boots buildroot initramfs (regression).

[**Failure / Robustness Validation**]
- V-F-1: QueueNotify with PFN=0 → no crash.
- V-F-2: Sector beyond capacity → `S_IOERR`.
- V-F-3: Unknown request type → `S_UNSUPP`.
- V-F-4: Circular descriptor chain → bounded walk, `S_IOERR`.
- V-F-5: `Device::reset()` restores original disk snapshot — write sectors, reset, verify original bytes.

[**Edge Case Validation**]
- V-E-1: Zero-length data buffer.
- V-E-2: Last sector of disk.
- V-E-3: Batch of multiple requests between notifications.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (VirtIO works with Linux) | V-IT-1 |
| G-2 (Debian boots to shell) | V-IT-1, V-IT-2 |
| G-3 (make debian target) | V-IT-1 |
| G-4 (XCPU config-aware) | V-UT-8, V-IT-1 (256MB), V-IT-3 (128MB) |
| C-1 (Legacy v1) | V-UT-1 |
| C-2 (Single queue, 128) | V-UT-3 |
| C-5 (T_IN, T_OUT only) | V-F-3 |
| C-6 (No regression) | V-IT-3 |
| C-7 (Fixed 256MB Debian) | V-IT-1 |
| I-1 (DmaCtx only) | V-UT-7, code review |
| I-3 (Snapshot + reset) | V-F-5 |
