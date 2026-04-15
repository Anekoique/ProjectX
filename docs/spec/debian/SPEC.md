# `debian` SPEC

> Source: [`/docs/archived/feat/debian/04_PLAN.md`](/docs/archived/feat/debian/04_PLAN.md).
> Iteration history, trade-off analysis, and implementation
> plan live under `docs/archived/feat/debian/`.

---


[**Goals**]
- G-1: VirtIO MMIO legacy (v1) block device usable by Linux.
- G-2: Boot minimal Debian riscv64 to interactive login shell (offline).
- G-3: `make debian` downloads pre-built image and boots it.
- G-4: `MachineConfig` wired into real `XCPU` construction; `BootLayout` persists FDT address.

- NG-1: No virtio-net.
- NG-2: No multi-queue, packed virtqueue, advanced features.
- NG-3: No disk writeback.
- NG-4: No in-tree image builder.
- NG-5: No runtime RAM override.

[**Architecture**]

```
Module Structure:

  device/
    mod.rs          — Device trait + hard_reset/take_notify/process_dma
    virtio_blk.rs   — VirtioBlk (MMIO registers + block I/O)
    virtio/
      queue.rs      — Virtqueue (ring addresses, chain walking, completion)
      defs.rs       — Enums: VirtioStatus, BlkReqType, BlkStatus, DescFlags
```

```
Data Flow:

  Bus::write(0x10001050)
    → VirtioBlk::write(0x50) → notify_pending = true
    → Bus::maybe_process_dma()
        → DmaCtx { &mut ram }
        → VirtioBlk::process_dma(&mut dma)
            → queue.poll(dma)        // yields (head, [Desc...])
            → handle_chain(dma, ..)  // parse header, dispatch I/O
            → queue.complete(dma, ..)// write used ring entry
            → interrupt_status |= 1  // only if completed > 0
```

```
Boot Layout Persistence:

  init_xcore(MachineConfig)
    → Core::with_config(config)
        → Bus::new(MBASE, config.ram_size)
        → conditionally add VirtioBlk
    → CPU::new(core, BootLayout { fdt_addr, ram_size })
        → stores boot_layout in CPU

  CPU::load_firmware()
    → uses self.boot_layout.fdt_addr for FDT placement
```

Memory maps unchanged from round 02:
- Default: 128MB, FDT @ `0x87F0_0000`
- Debian: 256MB + virtio-blk @ `0x1000_1000` IRQ 1, FDT @ `0x8FF0_0000`

[**Invariants**]
- I-1: VirtioBlk accesses guest RAM exclusively through `DmaCtx`.
- I-2: Descriptor chain walk bounded by `queue_num`.
- I-3: Disk snapshot. VirtIO reset preserves disk. Emulator `hard_reset()` restores original.
- I-4: Interrupt only raised when ≥1 chain completed in a `process_dma` call.
- I-5: No side effects on register reads.
- I-6: Fixed profiles — DTS and `MachineConfig` match exactly.
- I-7: `dev_features_sel` and `drv_features_sel` are independent registers.

[**Data Structure**]

```rust
// device/virtio/defs.rs

/// VirtIO device status (accumulative bitmask).
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum VirtioStatus {
    Reset       = 0,
    Acknowledge = 1,
    Driver      = 2,
    DriverOk    = 4,
    Failed      = 128,
}

/// Block request types.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u32)]
pub enum BlkReqType {
    In  = 0,
    Out = 1,
}

impl BlkReqType {
    fn from_u32(v: u32) -> Option<Self> {
        match v { 0 => Some(Self::In), 1 => Some(Self::Out), _ => None }
    }
}

/// Block request status (device → guest).
#[derive(Clone, Copy)]
#[repr(u8)]
pub enum BlkStatus {
    Ok     = 0,
    IoErr  = 1,
    Unsupp = 2,
}

/// Descriptor flags.
pub struct DescFlags(u16);

impl DescFlags {
    pub const NEXT: u16  = 1;
    pub const WRITE: u16 = 2;
    pub fn has_next(self)  -> bool { self.0 & Self::NEXT != 0 }
    pub fn writable(self) -> bool { self.0 & Self::WRITE != 0 }
}
```

```rust
// device/virtio/queue.rs

/// Parsed descriptor from guest memory.
pub struct Desc {
    pub addr: u64,
    pub len: u32,
    pub flags: DescFlags,
    pub next: u16,
}

/// Split virtqueue state for legacy (v1) transport.
pub struct Virtqueue {
    num: u16,
    pfn: u32,
    align: u32,
    guest_page_size: u32,
    last_avail_idx: u16,
}

impl Virtqueue {
    pub fn new() -> Self { ... }
    pub fn configured(&self) -> bool { self.pfn != 0 && self.guest_page_size != 0 }
    pub fn set_num(&mut self, n: u16) { self.num = n; }
    pub fn set_pfn(&mut self, pfn: u32) { self.pfn = pfn; }
    pub fn set_align(&mut self, align: u32) { self.align = align.max(1); }
    pub fn set_page_size(&mut self, ps: u32) { self.guest_page_size = ps; }

    /// Base addresses for descriptor table, available ring, used ring.
    fn ring_addrs(&self) -> (usize, usize, usize) {
        let base = self.pfn as usize * self.guest_page_size as usize;
        let n = self.num as usize;
        let align = self.align as usize;
        let desc = base;
        let avail = base + 16 * n;
        let used = (avail + 6 + 2 * n + align - 1) & !(align - 1);
        (desc, avail, used)
    }

    /// Read a single descriptor from guest memory.
    fn read_desc(&self, dma: &DmaCtx, desc_base: usize, idx: u16) -> XResult<Desc> {
        let off = desc_base + idx as usize * 16;
        let mut buf = [0u8; 16];
        dma.read_bytes(off, &mut buf)?;
        Ok(Desc {
            addr:  u64::from_le_bytes(buf[0..8].try_into().unwrap()),
            len:   u32::from_le_bytes(buf[8..12].try_into().unwrap()),
            flags: DescFlags(u16::from_le_bytes(buf[12..14].try_into().unwrap())),
            next:  u16::from_le_bytes(buf[14..16].try_into().unwrap()),
        })
    }

    /// Collect descriptors from a chain starting at `head`, bounded by queue size.
    pub fn collect_chain(&self, dma: &DmaCtx, desc_base: usize, head: u16) -> XResult<Vec<Desc>> {
        let mut chain = Vec::new();
        let mut idx = head;
        loop {
            if chain.len() >= self.num as usize { break; } // I-2: bounded
            let desc = self.read_desc(dma, desc_base, idx)?;
            let has_next = desc.flags.has_next();
            let next = desc.next;
            chain.push(desc);
            if !has_next { break; }
            idx = next;
        }
        Ok(chain)
    }

    /// Process all pending entries. Returns number of completed chains.
    /// Calls `handler` for each chain, which returns (status, bytes_written).
    pub fn poll(
        &mut self,
        dma: &mut DmaCtx,
        mut handler: impl FnMut(&mut DmaCtx, &[Desc]) -> (BlkStatus, u32),
    ) -> XResult<u32> {
        if !self.configured() { return Ok(0); }
        let (desc_base, avail_base, used_base) = self.ring_addrs();
        let avail_idx = dma.read_val::<u16>(avail_base + 2)?;

        let mut completed = 0u32;
        while self.last_avail_idx != avail_idx {
            let ring_off = (self.last_avail_idx % self.num) as usize;
            let head = dma.read_val::<u16>(avail_base + 4 + ring_off * 2)?;
            let chain = self.collect_chain(dma, desc_base, head)?;

            let (status, written) = handler(dma, &chain);
            // Write status byte (last descriptor's address)
            if let Some(last) = chain.last() {
                let _ = dma.write_val(last.addr as usize, status as u8);
            }

            // Update used ring
            let used_idx = dma.read_val::<u16>(used_base + 2)?;
            let entry = used_base + 4 + (used_idx % self.num) as usize * 8;
            dma.write_val::<u32>(entry, head as u32)?;
            dma.write_val::<u32>(entry + 4, written)?;
            dma.write_val::<u16>(used_base + 2, used_idx.wrapping_add(1))?;

            self.last_avail_idx = self.last_avail_idx.wrapping_add(1);
            completed += 1;
        }
        Ok(completed)
    }

    /// Clear all queue state. Called on VirtIO transport reset.
    pub fn reset(&mut self) {
        *self = Self::new();
    }
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
    fn offset(&self, paddr: usize, len: usize) -> XResult<usize> {
        paddr.checked_sub(self.ram_base)
            .filter(|&off| off.checked_add(len).is_some_and(|end| end <= self.ram.len()))
            .ok_or(XError::BadAddress)
    }

    pub fn read_bytes(&self, paddr: usize, buf: &mut [u8]) -> XResult {
        let off = self.offset(paddr, buf.len())?;
        self.ram.read_bytes(off, buf)
    }

    pub fn write_bytes(&mut self, paddr: usize, data: &[u8]) -> XResult {
        let off = self.offset(paddr, data.len())?;
        self.ram.write_bytes(off, data)
    }

    /// Read a little-endian value of any primitive type.
    pub fn read_val<T: LeBytes>(&self, paddr: usize) -> XResult<T> {
        let mut buf = T::Buf::default();
        self.read_bytes(paddr, buf.as_mut())?;
        Ok(T::from_le(buf))
    }

    /// Write a little-endian value of any primitive type.
    pub fn write_val<T: LeBytes>(&mut self, paddr: usize, val: T) -> XResult {
        self.write_bytes(paddr, val.to_le().as_ref())
    }
}

/// Trait for little-endian byte conversion.
pub trait LeBytes: Sized {
    type Buf: Default + AsMut<[u8]> + AsRef<[u8]>;
    fn from_le(buf: Self::Buf) -> Self;
    fn to_le(self) -> Self::Buf;
}

macro_rules! impl_le_bytes {
    ($($ty:ty),*) => { $(
        impl LeBytes for $ty {
            type Buf = [u8; std::mem::size_of::<$ty>()];
            fn from_le(buf: Self::Buf) -> Self { <$ty>::from_le_bytes(buf) }
            fn to_le(self) -> Self::Buf { self.to_le_bytes() }
        }
    )* };
}
impl_le_bytes!(u8, u16, u32, u64);
```

```rust
// device/virtio_blk.rs

use super::{Device, bus::DmaCtx, virtio::{defs::*, queue::Virtqueue}};

const VIRTIO_MAGIC: u32   = 0x7472_6976;
const VIRTIO_VERSION: u32 = 1;
const VIRTIO_BLK_ID: u32  = 2;
const VIRTIO_VENDOR: u32  = 0x554d_4551;
const QUEUE_NUM_MAX: u16  = 128;
const SECTOR_SIZE: u64    = 512;

mmio_regs! {
    enum Reg {
        Magic         = 0x000,
        Version       = 0x004,
        DeviceId      = 0x008,
        VendorId      = 0x00c,
        DevFeatures   = 0x010,
        DevFeaturesSel = 0x014,
        DrvFeatures   = 0x020,
        DrvFeaturesSel = 0x024,
        GuestPageSize = 0x028,
        QueueSel      = 0x030,
        QueueNumMax   = 0x034,
        QueueNum      = 0x038,
        QueueAlign    = 0x03c,
        QueuePfn      = 0x040,
        QueueNotify   = 0x050,
        InterruptStatus = 0x060,
        InterruptAck  = 0x064,
        Status        = 0x070,
    }
}

/// VirtIO MMIO legacy (v1) block device.
pub struct VirtioBlk {
    status: u32,
    dev_features_sel: u32,
    drv_features_sel: u32,
    drv_features: [u32; 2],
    queue: Virtqueue,
    interrupt_status: u32,
    notify_pending: bool,
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
            drv_features_sel: 0,
            drv_features: [0; 2],
            queue: Virtqueue::new(),
            interrupt_status: 0,
            notify_pending: false,
            capacity,
            disk,
            original,
        }
    }

    /// Read config space (offset relative to 0x100). Only `capacity` (u64 LE).
    fn read_config(&self, offset: usize, size: usize) -> Word {
        let bytes = self.capacity.to_le_bytes();
        (offset < 8).then(|| {
            let mut buf = [0u8; 4];
            let end = (offset + size).min(8);
            buf[..end - offset].copy_from_slice(&bytes[offset..end]);
            u32::from_le_bytes(buf) as Word
        })
        .unwrap_or(0)
    }

    /// Handle a single block request descriptor chain.
    fn handle_chain(&mut self, dma: &mut DmaCtx, descs: &[Desc]) -> (BlkStatus, u32) {
        // Need at least 3 descriptors: header + data + status
        if descs.len() < 3 {
            return (BlkStatus::IoErr, 0);
        }
        let (header, data, _status) = (&descs[0], &descs[1], &descs[2]);

        // Parse request header (16 bytes)
        let mut hdr = [0u8; 16];
        if dma.read_bytes(header.addr as usize, &mut hdr).is_err() {
            return (BlkStatus::IoErr, 0);
        }
        let req_type = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
        let sector = u64::from_le_bytes(hdr[8..16].try_into().unwrap());

        match BlkReqType::from_u32(req_type) {
            Some(BlkReqType::In) =>
                self.blk_read(dma, sector, data.addr as usize, data.len as usize),
            Some(BlkReqType::Out) =>
                self.blk_write(dma, sector, data.addr as usize, data.len as usize),
            None => (BlkStatus::Unsupp, 0),
        }
    }

    fn blk_read(&self, dma: &mut DmaCtx, sector: u64, addr: usize, len: usize) -> (BlkStatus, u32) {
        let off = sector * SECTOR_SIZE;
        if off + len as u64 > self.capacity * SECTOR_SIZE {
            return (BlkStatus::IoErr, 0);
        }
        let start = off as usize;
        dma.write_bytes(addr, &self.disk[start..start + len])
            .map_or((BlkStatus::IoErr, 0), |()| (BlkStatus::Ok, len as u32))
    }

    fn blk_write(&mut self, dma: &DmaCtx, sector: u64, addr: usize, len: usize) -> (BlkStatus, u32) {
        let off = sector * SECTOR_SIZE;
        if off + len as u64 > self.capacity * SECTOR_SIZE {
            return (BlkStatus::IoErr, 0);
        }
        let start = off as usize;
        let mut buf = vec![0u8; len];
        if dma.read_bytes(addr, &mut buf).is_err() {
            return (BlkStatus::IoErr, 0);
        }
        self.disk[start..start + len].copy_from_slice(&buf);
        (BlkStatus::Ok, 0)
    }

    fn soft_reset(&mut self) {
        self.status = 0;
        self.dev_features_sel = 0;
        self.drv_features_sel = 0;
        self.drv_features = [0; 2];
        self.queue.reset();
        self.interrupt_status = 0;
        self.notify_pending = false;
    }
}

impl Device for VirtioBlk {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word> {
        if offset >= 0x100 {
            return Ok(self.read_config(offset - 0x100, size));
        }
        Ok(match Reg::decode(offset) {
            Some(Reg::Magic)           => VIRTIO_MAGIC as Word,
            Some(Reg::Version)         => VIRTIO_VERSION as Word,
            Some(Reg::DeviceId)        => VIRTIO_BLK_ID as Word,
            Some(Reg::VendorId)        => VIRTIO_VENDOR as Word,
            Some(Reg::DevFeatures)     => 0, // no optional features
            Some(Reg::QueueNumMax)     => QUEUE_NUM_MAX as Word,
            Some(Reg::QueuePfn)        => self.queue.pfn() as Word,
            Some(Reg::InterruptStatus) => self.interrupt_status as Word,
            Some(Reg::Status)          => self.status as Word,
            _ => 0,
        })
    }

    fn write(&mut self, offset: usize, _size: usize, value: Word) -> XResult {
        let val = value as u32;
        match Reg::decode(offset) {
            Some(Reg::DevFeaturesSel) => self.dev_features_sel = val,
            Some(Reg::DrvFeatures) => {
                if let Some(slot) = self.drv_features.get_mut(self.drv_features_sel as usize) {
                    *slot = val;
                }
            }
            Some(Reg::DrvFeaturesSel) => self.drv_features_sel = val,
            Some(Reg::GuestPageSize)  => self.queue.set_page_size(val),
            Some(Reg::QueueSel)       => {} // single queue, ignore non-zero
            Some(Reg::QueueNum)       => self.queue.set_num(val as u16),
            Some(Reg::QueueAlign)     => self.queue.set_align(val),
            Some(Reg::QueuePfn)       => self.queue.set_pfn(val),
            Some(Reg::QueueNotify)    => self.notify_pending = true,
            Some(Reg::InterruptAck)   => self.interrupt_status &= !val,
            Some(Reg::Status) => {
                if val == 0 { self.soft_reset(); }
                else { self.status = val; }
            }
            _ => {}
        }
        Ok(())
    }

    fn irq_line(&self) -> bool { self.interrupt_status != 0 }

    fn reset(&mut self) { self.soft_reset(); }

    fn hard_reset(&mut self) {
        self.soft_reset();
        self.disk = self.original.clone();
    }

    fn take_notify(&mut self) -> bool { std::mem::take(&mut self.notify_pending) }

    fn process_dma(&mut self, dma: &mut DmaCtx) {
        let completed = self.queue.poll(dma, |dma, descs| self.handle_chain(dma, descs))
            .unwrap_or(0);
        if completed > 0 {
            self.interrupt_status |= 1;
        }
    }
}
```

[**API Surface**]

```rust
// --- xcore public API ---
pub fn init_xcore(config: MachineConfig) -> XResult;

// --- MachineConfig ---
pub struct MachineConfig { pub ram_size: usize, pub disk: Option<Vec<u8>> }
impl Default for MachineConfig { ... }  // 128MB, no disk
impl MachineConfig {
    pub fn with_disk(disk: Vec<u8>) -> Self;  // 256MB
    pub fn fdt_addr(&self) -> usize;          // MBASE + ram_size - 0x10_0000
}

// --- BootLayout (stored in CPU) ---
pub struct BootLayout { pub fdt_addr: usize }

// --- CPU ---
pub struct CPU<Core> { ..., boot_layout: BootLayout }
impl CPU<Core> {
    pub fn new(core: Core, layout: BootLayout) -> Self;
}

// --- Device trait ---
pub trait Device: Send {
    fn read/write/tick/irq_line/notify/reset/mtime ...
    fn hard_reset(&mut self) { self.reset(); }
    fn take_notify(&mut self) -> bool { false }
    fn process_dma(&mut self, _dma: &mut DmaCtx) {}
}

// --- DmaCtx ---
pub struct DmaCtx<'a> { ... }
impl DmaCtx { read_bytes, write_bytes, read_val::<T>, write_val::<T> }

// --- Virtqueue ---
pub struct Virtqueue { ... }
impl Virtqueue { poll(), collect_chain(), reset() }

// --- VirtioBlk ---
pub struct VirtioBlk { ... }
impl VirtioBlk { new(disk), handle_chain(), blk_read(), blk_write() }
```

[**Constraints**]
- C-1: Legacy (v1) transport. Version = 1.
- C-2: Single virtqueue (queue 0). `QueueNumMax` = 128.
- C-3: Disk snapshot. VirtIO reset preserves. Emulator `hard_reset()` restores.
- C-4: Synchronous on `QueueNotify`.
- C-5: `T_IN` / `T_OUT` only. Others → `Unsupp`.
- C-6: Existing targets unchanged.
- C-7: Fixed 256MB Debian profile.
- C-8: FDT: `CONFIG_MBASE + ram_size - 0x10_0000`, stored in `BootLayout`.
- C-9: `init_xcore` called once → `AlreadyInitialized` on second call.
- C-10: `dev_features_sel` and `drv_features_sel` independent.

---
