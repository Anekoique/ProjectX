# `Debian Boot` PLAN `03`

> Status: Revised
> Feature: `debian`
> Iteration: `03`
> Owner: Executor
> Depends on:
> - Previous Plan: `02_PLAN.md`
> - Review: `02_REVIEW.md`
> - Master Directive: `02_MASTER.md`

---

## Summary

Implement a VirtIO MMIO legacy block device for booting Debian on xemu. This revision wires `X_DISK` through the full Makefile→xdb→xcore launcher chain, separates VirtIO transport reset from emulator-level snapshot restore, defines the `init_xcore` repeated-call contract, and provides more detailed code per master directives.

## Log

[**Feature Introduce**]

- Two-tier reset model: VirtIO `Status=0` clears transport/queue/interrupt state only (disk preserved). Emulator-level `CPU::reset()` also restores original disk snapshot via a new `Device::hard_reset()` method.
- Full launcher wiring: `xemu/Makefile` gains `DISK` variable → exports `X_DISK`. `resource/debian.mk` passes `DISK=$(DEBIAN_IMG)` to xemu's make. xdb reads `X_DISK` → `MachineConfig`.
- `init_xcore` contract: `OnceLock::set()` returns `Err` on second call → `init_xcore` propagates that as `XError`. Tests use a separate construction path.

[**Review Adjustments**]

- R-001: Added `DISK` variable and `export X_DISK` to `xemu/Makefile`. `resource/debian.mk`'s `run-debian` passes `DISK=...`. Full chain documented.
- R-002: Split reset into soft (VirtIO transport) and hard (emulator snapshot). `Device::reset()` = soft. `Device::hard_reset()` = full restore. `Bus::reset_devices()` calls `hard_reset()`.
- R-003: `init_xcore(config)` returns `Err(XError::AlreadyInitialized)` on second call. Tests bypass `XCPU` singleton.

[**Master Compliance**]

- M-001: Added detailed code blocks for all key types, register dispatch, virtqueue processing, DMA interface, and launcher integration.
- M-002: Simplified `MachineConfig` to two constructors. Unified reset via trait method. Eliminated redundant state by computing all derived values inline.

### Changes from Previous Round

[**Added**]
- `DISK` / `X_DISK` in `xemu/Makefile`.
- `Device::hard_reset()` trait method for emulator-level restore.
- `XError::AlreadyInitialized` variant.
- Detailed code for register dispatch, virtqueue walk, DMA, launcher.

[**Changed**]
- VirtIO `Status=0` no longer restores disk snapshot — clears transport only.
- `Bus::reset_devices()` calls `hard_reset()` instead of `reset()`.
- `init_xcore` second call → explicit error.

[**Removed**]
- Conflated "reset restores disk" in VirtIO transport path.

[**Unresolved**]
- None.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | `xemu/Makefile` adds `DISK ?=` and `export X_DISK = $(DISK)`. `debian.mk` passes `DISK=$(DEBIAN_IMG)`. xdb reads `X_DISK`. Full chain specified. |
| Review | R-002 | Accepted | VirtIO `Status=0` = soft reset (transport only, disk preserved). `Device::hard_reset()` = emulator restore (original snapshot). `Bus::reset_devices()` calls `hard_reset()`. |
| Review | R-003 | Accepted | `init_xcore` returns `Err(XError::AlreadyInitialized)` on second call. Tests construct `CPU<Core>` directly. |
| Master | M-001 | Applied | Added detailed code blocks throughout: register dispatch tables, virtqueue processing loop, DMA methods, Makefile snippets, DTS node. |
| Master | M-002 | Applied | Simplified MachineConfig API. Unified reset semantics via single trait extension. Removed redundant derived state. |

---

## Spec

[**Goals**]
- G-1: VirtIO MMIO legacy (v1) block device usable by Linux's `virtio_mmio` + `virtio_blk`.
- G-2: Boot minimal Debian riscv64 to interactive login shell (offline userspace).
- G-3: `make debian` target downloads pre-built image and boots it.
- G-4: `MachineConfig` wired into real `XCPU` construction via `init_xcore(config)`.

- NG-1: No virtio-net.
- NG-2: No multi-queue, packed virtqueue, or advanced features.
- NG-3: No disk writeback — snapshot only.
- NG-4: No in-tree image builder.
- NG-5: No runtime RAM override.

[**Architecture**]

```
Launcher Chain:

  resource/debian.mk                    xemu/Makefile
  ┌─────────────────┐                  ┌──────────────────────┐
  │ run-debian:      │                  │ DISK ?=              │
  │   $(MAKE) -C     │───DISK=path───→ │ export X_DISK=$(DISK)│
  │     $(XEMU_HOME) │                  │                      │
  │     run DISK=... │                  │ run:                 │
  │     FW=... FDT=..│                  │   cargo run ...      │
  └─────────────────┘                  └──────────┬───────────┘
                                                   │
                                       ┌───────────▼───────────┐
                                       │ xdb::main()           │
                                       │  1. machine_config()  │
                                       │     X_DISK → load     │
                                       │     → MachineConfig   │
                                       │  2. init_xcore(config)│
                                       │     → OnceLock<XCPU>  │
                                       │  3. boot_config()     │
                                       │     X_FW/KERNEL/FDT   │
                                       │  4. run(boot)         │
                                       └───────────────────────┘
```

```
Reset Model:

  Guest writes Status=0          CPU::reset() / xdb "reset"
  (VirtIO transport reset)       (Emulator-level reset)
         │                              │
         ▼                              ▼
  Device::reset()               Device::hard_reset()
  ┌───────────────┐             ┌───────────────────────┐
  │ Clear status   │             │ Device::reset()       │
  │ Clear queue    │             │ + restore disk from   │
  │ Clear interrupt│             │   original snapshot   │
  │ Disk PRESERVED │             └───────────────────────┘
  └───────────────┘
```

```
DMA Flow:

  Bus::dispatch(write, 0x10001050, ...)
    │
    ├─ dev.write(0x50, ...) → VirtioBlk sets notify_pending
    │
    └─ dev.take_notify() == true
         │
         ▼
       Bus::process_dma(idx)
         │
         ├─ split borrow: &mut self.ram + &mut self.mmio[idx].dev
         ├─ DmaCtx { ram: &mut ram, ram_base }
         ├─ dev.process_dma(&mut dma)
         │    ├─ read avail ring
         │    ├─ walk descriptors
         │    ├─ sector I/O ↔ disk Vec<u8>
         │    ├─ write used ring + status
         │    └─ interrupt_status |= 1
         └─ borrows end
```

Memory maps (unchanged from round 02):
- Default: 128MB, no virtio, FDT @ `0x87F0_0000`
- Debian: 256MB, virtio-blk @ `0x1000_1000` IRQ 1, FDT @ `0x8FF0_0000`

[**Invariants**]
- I-1: VirtioBlk accesses guest RAM exclusively through `DmaCtx`.
- I-2: Descriptor chain walk bounded by `queue_num`.
- I-3: Disk snapshot — writes modify in-memory copy. VirtIO transport reset preserves disk. Emulator `hard_reset()` restores original.
- I-4: Interrupt: `interrupt_status` bit 0 set after used ring update.
- I-5: No side effects on register reads.
- I-6: Fixed profiles — each target's DTS and `MachineConfig` match exactly.

[**Data Structure**]

```rust
// config/mod.rs

/// Machine configuration — independent inputs only.
pub struct MachineConfig {
    pub ram_size: usize,
    pub disk: Option<Vec<u8>>,
}

impl Default for MachineConfig {
    fn default() -> Self {
        Self { ram_size: CONFIG_MSIZE, disk: None }
    }
}

impl MachineConfig {
    pub fn with_disk(disk: Vec<u8>) -> Self {
        Self {
            ram_size: 0x1000_0000, // 256MB
            disk: Some(disk),
        }
    }

    /// FDT load address: 1MB below top of RAM.
    pub fn fdt_addr(&self) -> usize {
        CONFIG_MBASE + self.ram_size - 0x10_0000
    }
}
```

```rust
// device/mod.rs — trait extension

pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
    fn irq_line(&self) -> bool { false }
    fn notify(&mut self, _irq_lines: u32) {}
    /// Soft reset: clear device-level state.
    /// VirtIO: clears transport/queue/interrupt. Disk preserved.
    fn reset(&mut self) {}
    /// Hard reset: full restore to power-on state.
    /// Default: delegates to reset(). VirtioBlk: also restores disk snapshot.
    fn hard_reset(&mut self) { self.reset(); }
    fn mtime(&self) -> Option<u64> { None }
    /// Return true if device needs DMA processing after a write.
    fn take_notify(&mut self) -> bool { false }
    /// Process pending DMA operations with guest memory access.
    fn process_dma(&mut self, _dma: &mut DmaCtx) {}
}
```

```rust
// device/bus.rs — DMA context

/// Safe guest-memory accessor for DMA-capable devices.
pub struct DmaCtx<'a> {
    ram: &'a mut Ram,
    ram_base: usize,
}

impl<'a> DmaCtx<'a> {
    /// Read a contiguous byte slice from guest physical address.
    pub fn read_bytes(&self, paddr: usize, buf: &mut [u8]) -> XResult {
        let offset = paddr.checked_sub(self.ram_base).ok_or(XError::BadAddress)?;
        self.ram.read_bytes(offset, buf)
    }

    /// Write a contiguous byte slice to guest physical address.
    pub fn write_bytes(&mut self, paddr: usize, data: &[u8]) -> XResult {
        let offset = paddr.checked_sub(self.ram_base).ok_or(XError::BadAddress)?;
        self.ram.write_bytes(offset, data)
    }

    /// Read a little-endian u16 from guest physical address.
    pub fn read_u16(&self, paddr: usize) -> XResult<u16> {
        let mut buf = [0u8; 2];
        self.read_bytes(paddr, &mut buf)?;
        Ok(u16::from_le_bytes(buf))
    }

    /// Read a little-endian u32 from guest physical address.
    pub fn read_u32(&self, paddr: usize) -> XResult<u32> {
        let mut buf = [0u8; 4];
        self.read_bytes(paddr, &mut buf)?;
        Ok(u32::from_le_bytes(buf))
    }

    /// Read a little-endian u64 from guest physical address.
    pub fn read_u64(&self, paddr: usize) -> XResult<u64> {
        let mut buf = [0u8; 8];
        self.read_bytes(paddr, &mut buf)?;
        Ok(u64::from_le_bytes(buf))
    }

    /// Write a little-endian u16 to guest physical address.
    pub fn write_u16(&mut self, paddr: usize, val: u16) -> XResult {
        self.write_bytes(paddr, &val.to_le_bytes())
    }

    /// Write a little-endian u32 to guest physical address.
    pub fn write_u32(&mut self, paddr: usize, val: u32) -> XResult {
        self.write_bytes(paddr, &val.to_le_bytes())
    }
}
```

```rust
// device/virtio_blk.rs

// --- Constants ---
const VIRTIO_MAGIC: u32 = 0x7472_6976;     // "virt"
const VIRTIO_VERSION: u32 = 1;              // legacy
const VIRTIO_DEVICE_BLK: u32 = 2;
const VIRTIO_VENDOR: u32 = 0x554d_4551;    // "QEMU"
const QUEUE_NUM_MAX: u32 = 128;
const SECTOR_SIZE: u64 = 512;

// Status register bits
const STATUS_ACKNOWLEDGE: u32 = 1;
const STATUS_DRIVER: u32 = 2;
const STATUS_DRIVER_OK: u32 = 4;
const STATUS_FAILED: u32 = 128;

// Descriptor flags
const VRING_DESC_F_NEXT: u16 = 1;
const VRING_DESC_F_WRITE: u16 = 2;

// Block request types
const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_T_OUT: u32 = 1;

// Block request status
const VIRTIO_BLK_S_OK: u8 = 0;
const VIRTIO_BLK_S_IOERR: u8 = 1;
const VIRTIO_BLK_S_UNSUPP: u8 = 2;

// MMIO register offsets
const REG_MAGIC: usize          = 0x000;
const REG_VERSION: usize        = 0x004;
const REG_DEVICE_ID: usize      = 0x008;
const REG_VENDOR_ID: usize      = 0x00c;
const REG_DEV_FEATURES: usize   = 0x010;
const REG_DEV_FEATURES_SEL: usize = 0x014;
const REG_DRV_FEATURES: usize   = 0x020;
const REG_DRV_FEATURES_SEL: usize = 0x024;
const REG_GUEST_PAGE_SIZE: usize = 0x028;
const REG_QUEUE_SEL: usize      = 0x030;
const REG_QUEUE_NUM_MAX: usize  = 0x034;
const REG_QUEUE_NUM: usize      = 0x038;
const REG_QUEUE_ALIGN: usize    = 0x03c;
const REG_QUEUE_PFN: usize      = 0x040;
const REG_QUEUE_NOTIFY: usize   = 0x050;
const REG_INTERRUPT_STATUS: usize = 0x060;
const REG_INTERRUPT_ACK: usize  = 0x064;
const REG_STATUS: usize         = 0x070;
const REG_CONFIG: usize         = 0x100;

/// VirtIO MMIO legacy (v1) block device.
pub struct VirtioBlk {
    // Transport
    status: u32,
    dev_features_sel: u32,
    drv_features: [u32; 2],
    guest_page_size: u32,
    // Queue (single queue, index 0)
    queue_sel: u32,
    queue_num: u32,
    queue_pfn: u32,
    queue_align: u32,
    last_avail_idx: u16,
    // Interrupt
    interrupt_status: u32,
    notify_pending: bool,
    // Block
    capacity: u64,
    disk: Vec<u8>,
    original: Vec<u8>,
}

impl VirtioBlk {
    pub fn new(disk: Vec<u8>) -> Self {
        let capacity = disk.len() as u64 / SECTOR_SIZE;
        let original = disk.clone();
        Self {
            status: 0,
            dev_features_sel: 0,
            drv_features: [0; 2],
            guest_page_size: 0,
            queue_sel: 0,
            queue_num: 0,
            queue_pfn: 0,
            queue_align: 4096,
            last_avail_idx: 0,
            interrupt_status: 0,
            notify_pending: false,
            capacity,
            disk,
            original,
        }
    }

    /// Compute vring component addresses from legacy layout.
    fn vring_addrs(&self) -> (usize, usize, usize) {
        let base = self.queue_pfn as usize * self.guest_page_size as usize;
        let num = self.queue_num as usize;
        let align = self.queue_align as usize;
        let desc = base;
        let avail = base + 16 * num;
        let used_unaligned = avail + 6 + 2 * num;
        let used = (used_unaligned + align - 1) & !(align - 1);
        (desc, avail, used)
    }

    /// Clear transport/queue/interrupt state. Disk preserved.
    fn soft_reset(&mut self) {
        self.status = 0;
        self.dev_features_sel = 0;
        self.drv_features = [0; 2];
        self.guest_page_size = 0;
        self.queue_sel = 0;
        self.queue_num = 0;
        self.queue_pfn = 0;
        self.queue_align = 4096;
        self.last_avail_idx = 0;
        self.interrupt_status = 0;
        self.notify_pending = false;
    }
}
```

```rust
// device/virtio_blk.rs — Device trait impl

impl Device for VirtioBlk {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word> {
        // All registers are 32-bit except config space
        let val: u32 = match offset {
            REG_MAGIC           => VIRTIO_MAGIC,
            REG_VERSION         => VIRTIO_VERSION,
            REG_DEVICE_ID       => VIRTIO_DEVICE_BLK,
            REG_VENDOR_ID       => VIRTIO_VENDOR,
            REG_DEV_FEATURES    => {
                if self.dev_features_sel == 0 { 0 } else { 0 }
                // No optional features advertised
            }
            REG_QUEUE_NUM_MAX   => {
                if self.queue_sel == 0 { QUEUE_NUM_MAX } else { 0 }
            }
            REG_QUEUE_PFN       => self.queue_pfn,
            REG_INTERRUPT_STATUS => self.interrupt_status,
            REG_STATUS          => self.status,
            // Config space: capacity (u64 LE at offset 0x100)
            o if o >= REG_CONFIG && o + size <= REG_CONFIG + 8 => {
                let config_off = o - REG_CONFIG;
                let cap_bytes = self.capacity.to_le_bytes();
                let mut buf = [0u8; 4];
                let end = (config_off + size).min(8);
                buf[..end - config_off].copy_from_slice(&cap_bytes[config_off..end]);
                return Ok(u32::from_le_bytes(buf) as Word);
            }
            _ => {
                warn!("virtio-blk: unhandled read offset={:#x}", offset);
                0
            }
        };
        Ok(val as Word)
    }

    fn write(&mut self, offset: usize, _size: usize, value: Word) -> XResult {
        let val = value as u32;
        match offset {
            REG_DEV_FEATURES_SEL => self.dev_features_sel = val,
            REG_DRV_FEATURES     => {
                let sel = self.dev_features_sel as usize;
                if sel < 2 { self.drv_features[sel] = val; }
            }
            REG_DRV_FEATURES_SEL => self.dev_features_sel = val,
            REG_GUEST_PAGE_SIZE  => self.guest_page_size = val,
            REG_QUEUE_SEL        => self.queue_sel = val,
            REG_QUEUE_NUM        => self.queue_num = val,
            REG_QUEUE_ALIGN      => self.queue_align = val.max(1),
            REG_QUEUE_PFN        => self.queue_pfn = val,
            REG_QUEUE_NOTIFY     => self.notify_pending = true,
            REG_INTERRUPT_ACK    => self.interrupt_status &= !val,
            REG_STATUS           => {
                if val == 0 { self.soft_reset(); }
                else { self.status = val; }
            }
            _ => warn!("virtio-blk: unhandled write offset={:#x} val={:#x}", offset, val),
        }
        Ok(())
    }

    fn irq_line(&self) -> bool {
        self.interrupt_status != 0
    }

    fn reset(&mut self) {
        self.soft_reset();
        // Disk preserved — VirtIO transport reset only.
    }

    fn hard_reset(&mut self) {
        self.soft_reset();
        self.disk = self.original.clone();
    }

    fn take_notify(&mut self) -> bool {
        std::mem::take(&mut self.notify_pending)
    }

    fn process_dma(&mut self, dma: &mut DmaCtx) {
        if self.queue_pfn == 0 || self.guest_page_size == 0 { return; }

        let (desc_base, avail_base, used_base) = self.vring_addrs();
        let num = self.queue_num as u16;

        // Read avail->idx
        let avail_idx = match dma.read_u16(avail_base + 2) {
            Ok(v) => v,
            Err(_) => return,
        };

        while self.last_avail_idx != avail_idx {
            let ring_idx = (self.last_avail_idx % num) as usize;
            let head = match dma.read_u16(avail_base + 4 + ring_idx * 2) {
                Ok(v) => v,
                Err(_) => break,
            };

            // Walk descriptor chain
            let mut idx = head;
            let mut req_type: u32 = 0;
            let mut sector: u64 = 0;
            let mut data_addr: usize = 0;
            let mut data_len: usize = 0;
            let mut data_writable = false;
            let mut status_addr: usize = 0;
            let mut desc_count: u16 = 0;
            let mut phase = 0; // 0=header, 1=data, 2=status

            loop {
                if desc_count >= num { break; } // bounded walk (I-2)
                desc_count += 1;

                let d = desc_base + idx as usize * 16;
                let addr = match dma.read_u64(d) { Ok(v) => v as usize, Err(_) => break };
                let len  = match dma.read_u32(d + 8) { Ok(v) => v, Err(_) => break };
                let flags = match dma.read_u16(d + 12) { Ok(v) => v, Err(_) => break };
                let next = match dma.read_u16(d + 14) { Ok(v) => v, Err(_) => break };

                match phase {
                    0 => { // Header: virtio_blk_outhdr (16 bytes)
                        let mut hdr = [0u8; 16];
                        if dma.read_bytes(addr, &mut hdr).is_err() { break; }
                        req_type = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
                        sector = u64::from_le_bytes(hdr[8..16].try_into().unwrap());
                        phase = 1;
                    }
                    1 => { // Data buffer
                        data_addr = addr;
                        data_len = len as usize;
                        data_writable = flags & VRING_DESC_F_WRITE != 0;
                        phase = 2;
                    }
                    2 => { // Status byte
                        status_addr = addr;
                    }
                    _ => {}
                }

                if flags & VRING_DESC_F_NEXT == 0 { break; }
                idx = next;
            }

            // Process I/O
            let status = if status_addr == 0 {
                VIRTIO_BLK_S_IOERR
            } else {
                self.process_request(dma, req_type, sector, data_addr, data_len, data_writable)
            };

            // Write status byte
            let _ = dma.write_bytes(status_addr, &[status]);

            // Update used ring
            let used_idx = match dma.read_u16(used_base + 2) { Ok(v) => v, Err(_) => break };
            let used_entry = used_base + 4 + (used_idx % num) as usize * 8;
            let _ = dma.write_u32(used_entry, head as u32);
            let _ = dma.write_u32(used_entry + 4, data_len as u32);
            let _ = dma.write_u16(used_base + 2, used_idx.wrapping_add(1));

            self.last_avail_idx = self.last_avail_idx.wrapping_add(1);
        }

        // Assert interrupt if we processed anything
        if self.last_avail_idx != avail_idx || self.interrupt_status & 1 == 0 {
            self.interrupt_status |= 1;
        }
    }
}

impl VirtioBlk {
    fn process_request(
        &mut self,
        dma: &mut DmaCtx,
        req_type: u32,
        sector: u64,
        data_addr: usize,
        data_len: usize,
        data_writable: bool,
    ) -> u8 {
        let offset = sector * SECTOR_SIZE;
        if offset + data_len as u64 > self.capacity * SECTOR_SIZE {
            return VIRTIO_BLK_S_IOERR;
        }
        let off = offset as usize;

        match req_type {
            VIRTIO_BLK_T_IN if data_writable => {
                match dma.write_bytes(data_addr, &self.disk[off..off + data_len]) {
                    Ok(()) => VIRTIO_BLK_S_OK,
                    Err(_) => VIRTIO_BLK_S_IOERR,
                }
            }
            VIRTIO_BLK_T_OUT if !data_writable => {
                let mut buf = vec![0u8; data_len];
                if dma.read_bytes(data_addr, &mut buf).is_err() {
                    return VIRTIO_BLK_S_IOERR;
                }
                self.disk[off..off + data_len].copy_from_slice(&buf);
                VIRTIO_BLK_S_OK
            }
            VIRTIO_BLK_T_IN | VIRTIO_BLK_T_OUT => VIRTIO_BLK_S_IOERR,
            _ => VIRTIO_BLK_S_UNSUPP,
        }
    }
}
```

```rust
// device/bus.rs — DMA dispatch

impl Bus {
    /// After a device write, check for pending DMA and process it.
    fn maybe_process_dma(&mut self, idx: usize) {
        if !self.mmio[idx].dev.take_notify() { return; }
        // Split borrow: ram vs mmio[idx].dev
        let dev = &mut self.mmio[idx].dev;
        let ram_base = self.ram.range().start;
        let dma = &mut DmaCtx { ram: &mut self.ram, ram_base };
        dev.process_dma(dma);
    }
}

// Updated dispatch for writes:
fn dispatch_write(&mut self, addr: usize, size: usize, value: Word) -> XResult {
    if let Some(off) = self.ram_offset(addr, size) {
        return self.ram.write(off, size, value);
    }
    #[cfg(feature = "difftest")]
    self.mmio_accessed.store(true, Relaxed);
    let idx = self.find_mmio_idx(addr, size)?;
    let off = addr - self.mmio[idx].range.start;
    self.mmio[idx].dev.write(off, size, value)?;
    self.maybe_process_dma(idx);
    Ok(())
}
```

```rust
// device/ram.rs — bulk byte access

impl Ram {
    pub fn read_bytes(&self, offset: usize, buf: &mut [u8]) -> XResult {
        let end = offset.checked_add(buf.len())
            .filter(|&e| e <= self.data.len())
            .ok_or(XError::BadAddress)?;
        buf.copy_from_slice(&self.data[offset..end]);
        Ok(())
    }

    pub fn write_bytes(&mut self, offset: usize, data: &[u8]) -> XResult {
        let end = offset.checked_add(data.len())
            .filter(|&e| e <= self.data.len())
            .ok_or(XError::BadAddress)?;
        self.data[offset..end].copy_from_slice(data);
        Ok(())
    }
}
```

```rust
// cpu/mod.rs — XCPU + init

pub static XCPU: OnceLock<Mutex<CPU<Core>>> = OnceLock::new();

pub fn init_xcore(config: MachineConfig) -> XResult {
    info!("Hello xcore!");
    let core = Core::with_config(config);
    let cpu = CPU::new(core);
    XCPU.set(Mutex::new(cpu))
        .map_err(|_| XError::AlreadyInitialized)?;
    with_xcpu!(reset())
}

pub fn with_xcpu<R>(f: impl FnOnce(&mut CPU<Core>) -> R) -> R {
    let mut guard = XCPU.get().expect("XCPU not initialized — call init_xcore() first").lock()
        .expect("Poisoned lock on CPU mutex");
    f(&mut guard)
}
```

```rust
// xdb/src/main.rs — machine config

fn machine_config() -> anyhow::Result<xcore::MachineConfig> {
    let env = |n: &str| std::env::var(n).ok().filter(|s| !s.is_empty());

    match env("X_DISK") {
        Some(path) => {
            let disk = std::fs::read(&path)
                .map_err(|e| anyhow!("Failed to read disk image {path}: {e}"))?;
            info!("Loaded disk image: {} ({} bytes)", path, disk.len());
            Ok(xcore::MachineConfig::with_disk(disk))
        }
        None => Ok(xcore::MachineConfig::default()),
    }
}

pub fn main() -> anyhow::Result<()> {
    init_xdb();
    let config = machine_config()?;
    xcore::init_xcore(config).map_err(|e| anyhow!("XCore Error: {e}"))?;
    run(boot_config()).map_err(|e| anyhow!("XDB Error: {e}"))?;
    if !xcore::with_xcpu(|cpu| cpu.is_exit_normal()) {
        std::process::exit(1);
    }
    Ok(())
}
```

[**API Surface**]

(See code blocks above — all public APIs are shown inline.)

[**Constraints**]
- C-1: Legacy (v1) transport only. Version = 1.
- C-2: Single virtqueue (queue 0). `QueueNumMax` = 128.
- C-3: Disk snapshot — host file never modified. VirtIO reset preserves disk. Emulator reset restores original.
- C-4: Synchronous on `QueueNotify`.
- C-5: `T_IN` (0), `T_OUT` (1) only. Others → `S_UNSUPP`.
- C-6: Existing targets unchanged.
- C-7: Fixed 256MB Debian profile.
- C-8: FDT: `CONFIG_MBASE + ram_size - 0x10_0000`.
- C-9: `init_xcore(config)` called once. Second call → `Err(XError::AlreadyInitialized)`.

---

## Implement

### Execution Flow

[**Main Flow**]
1. `xdb::machine_config()` reads `X_DISK`. If set → `MachineConfig::with_disk(disk)`. Else → `MachineConfig::default()`.
2. `init_xcore(config)` → `Core::with_config(config)` → `Bus::new(CONFIG_MBASE, config.ram_size)` → add devices (+ VirtioBlk if disk) → `CPU::new(core)` → store in `OnceLock`.
3. `boot_config()` → `BootConfig::Firmware { fw, kernel, fdt, .. }`.
4. `cpu.boot(config)` → `reset()` → `load_firmware()` → starts execution.
5. Guest probes DT → VirtIO init handshake → queue setup → DRIVER_OK.
6. Guest I/O: write descriptors → QueueNotify → Bus DMA → sector I/O → used ring → interrupt.
7. Guest mounts `/dev/vda` → Debian shell.

[**Failure Flow**]
1. `X_DISK` file not found → `machine_config()` returns error → process exits.
2. Unknown request type → `S_UNSUPP`.
3. Sector out of range → `S_IOERR`.
4. Malformed descriptors → bounded walk → `S_IOERR`.
5. QueueNotify + PFN=0 → ignored.
6. `init_xcore` called twice → `Err(AlreadyInitialized)`.

[**State Transition**]

VirtIO transport:
```
Status=0  → soft_reset(): transport cleared, disk preserved
Status|=1 → ACKNOWLEDGE
Status|=3 → DRIVER
Status|=7 → DRIVER_OK (I/O enabled)
Status|=128 → FAILED
```

Emulator-level:
```
CPU::reset() → Bus::reset_devices() → hard_reset() on all devices
  VirtioBlk::hard_reset() = soft_reset() + disk restored from original
```

### Implementation Plan

[**Phase 1: Core Infrastructure**]
1. `config/mod.rs`: add `MachineConfig`.
2. `error.rs`: add `XError::AlreadyInitialized`.
3. `device/ram.rs`: add `read_bytes`, `write_bytes`.
4. `device/mod.rs`: add `hard_reset`, `take_notify`, `process_dma` to `Device` trait.
5. `device/bus.rs`: add `DmaCtx`, `find_mmio_idx`, `maybe_process_dma`, split write dispatch.
6. `cpu/mod.rs`: `XCPU` → `OnceLock`, `init_xcore(config)`, update `with_xcpu`.

[**Phase 2: VirtIO Block Device**]
1. `device/virtio_blk.rs`: full implementation (constants, struct, `Device` impl, virtqueue processing).
2. `device/mod.rs`: register `pub mod virtio_blk`.

[**Phase 3: Machine Integration**]
1. `cpu/riscv/mod.rs`: `RVCore::with_config(config)` — conditional VirtioBlk registration.
2. `cpu/riscv/mod.rs`: `RVCore::new()` delegates to `with_config(Default::default())`.
3. `cpu/mod.rs`: `load_firmware` uses `MachineConfig::fdt_addr()`.
4. `xdb/src/main.rs`: `machine_config()`, updated `main()`.
5. `xemu/Makefile`: add `DISK ?=`, `export X_DISK = $(DISK)`.

[**Phase 4: Device Tree & Build**]
1. `resource/xemu-debian.dts`: 256MB + virtio,mmio node + `root=/dev/vda rw`.
2. `resource/debian.mk`: download target + `run-debian`.
3. `resource/Makefile`: include `debian.mk`.

[**Phase 5: Testing**]
1. Unit: VirtioBlk registers, status, config, reset.
2. Unit: DmaCtx round-trip.
3. Unit: hard_reset restores original disk.
4. Integration: `make debian` boots.
5. Regression: `make linux` unchanged.

## Trade-offs

- T-1: **Two-tier reset** — soft (transport) vs hard (emulator). Slightly more API surface than single reset, but spec-correct and avoids surprising data loss during guest driver resets.
- T-2: **`OnceLock` vs `LazyLock`** — `OnceLock` requires explicit init order but enables config-aware construction. Worth the ceremony.

## Validation

[**Unit Tests**]
- V-UT-1: Magic/Version/DeviceID/VendorID constants.
- V-UT-2: Feature negotiation (sel + read).
- V-UT-3: Queue config (sel, num_max, num, pfn).
- V-UT-4: Status transitions (0→1→3→7, write 0 = soft reset).
- V-UT-5: InterruptStatus/InterruptACK.
- V-UT-6: Config space capacity (LE u64 at 0x100).
- V-UT-7: DmaCtx read_bytes/write_bytes round-trip.
- V-UT-8: MachineConfig defaults and with_disk.
- V-UT-9: Soft reset preserves disk, hard_reset restores original.

[**Integration Tests**]
- V-IT-1: `make debian` boots to login shell.
- V-IT-2: File write+read in same Debian session.
- V-IT-3: `make linux` unchanged.

[**Failure / Robustness Validation**]
- V-F-1: QueueNotify + PFN=0 → no crash.
- V-F-2: Sector beyond capacity → `S_IOERR`.
- V-F-3: Unknown request type → `S_UNSUPP`.
- V-F-4: Circular descriptor → bounded → `S_IOERR`.
- V-F-5: hard_reset restores disk after writes.
- V-F-6: Second `init_xcore` call → `AlreadyInitialized` error.

[**Edge Case Validation**]
- V-E-1: Zero-length data buffer.
- V-E-2: Last sector of disk.
- V-E-3: Batch requests (avail_idx > last_avail_idx + 1).

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-1 |
| G-2 | V-IT-1, V-IT-2 |
| G-3 | V-IT-1 |
| G-4 | V-UT-8, V-IT-1, V-IT-3 |
| C-1 | V-UT-1 |
| C-2 | V-UT-3 |
| C-3 | V-UT-9, V-F-5 |
| C-5 | V-F-3 |
| C-6 | V-IT-3 |
| C-9 | V-F-6 |
| I-1 | V-UT-7 |
| I-2 | V-F-4 |
| I-3 | V-UT-9, V-F-5 |
