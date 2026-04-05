//! VirtIO MMIO legacy (v1) block device.
//!
//! Implements the VirtIO transport registers and block-device I/O.
//! Request processing is synchronous: triggered on `QueueNotify` write,
//! executed via `DmaCtx` during the same bus dispatch.

use super::{
    Device,
    bus::DmaCtx,
    mmio_regs,
    virtio::{
        defs::{BlkReqType, BlkStatus},
        queue::{Desc, Virtqueue},
    },
};
use crate::{config::Word, error::XResult};

const VIRTIO_MAGIC: u32 = 0x7472_6976;
const VIRTIO_VERSION: u32 = 1;
const VIRTIO_BLK_ID: u32 = 2;
const VIRTIO_VENDOR: u32 = 0x554d_4551;
const QUEUE_NUM_MAX: u16 = 128;
const SECTOR_SIZE: u64 = 512;

mmio_regs! {
    enum Reg {
        Magic           = 0x000,
        Version         = 0x004,
        DeviceId        = 0x008,
        VendorId        = 0x00c,
        DevFeatures     = 0x010,
        DevFeaturesSel  = 0x014,
        DrvFeatures     = 0x020,
        DrvFeaturesSel  = 0x024,
        GuestPageSize   = 0x028,
        QueueSel        = 0x030,
        QueueNumMax     = 0x034,
        QueueNum        = 0x038,
        QueueAlign      = 0x03c,
        QueuePfn        = 0x040,
        QueueNotify     = 0x050,
        InterruptStatus = 0x060,
        InterruptAck    = 0x064,
        Status          = 0x070,
    }
}

/// Block-device backing store, separated from transport state to enable
/// safe split borrows between `Virtqueue::poll` and I/O handlers.
struct BlkStorage {
    capacity: u64,
    disk: Vec<u8>,
    original: Vec<u8>,
}

impl BlkStorage {
    fn new(disk: Vec<u8>) -> Self {
        let capacity = disk.len() as u64 / SECTOR_SIZE;
        let original = disk.clone();
        Self {
            capacity,
            disk,
            original,
        }
    }

    /// Sector byte offset with overflow check.
    fn sector_offset(&self, sector: u64, len: usize) -> Option<usize> {
        sector
            .checked_mul(SECTOR_SIZE)
            .and_then(|o| usize::try_from(o).ok())
            .filter(|&off| off + len <= self.disk.len())
    }

    /// Handle a descriptor chain: parse header (type/reserved/sector),
    /// dispatch.
    fn handle_chain(&mut self, dma: &mut DmaCtx, descs: &[Desc]) -> (BlkStatus, u32) {
        if descs.len() < 3 {
            return (BlkStatus::IoErr, 0);
        }
        let (header, data) = (&descs[0], &descs[1]);

        // Header layout: type(u32) + reserved(u32) + sector(u64) = 16 bytes
        let mut hdr = [0u8; 16];
        if dma.read_bytes(header.addr as usize, &mut hdr).is_err() {
            return (BlkStatus::IoErr, 0);
        }
        let req_type = u32::from_le_bytes(hdr[0..4].try_into().unwrap());
        let sector = u64::from_le_bytes(hdr[8..16].try_into().unwrap());
        let (addr, len) = (data.addr as usize, data.len as usize);

        match BlkReqType::from_u32(req_type) {
            Some(BlkReqType::In) if data.flags.is_write() => self.blk_read(dma, sector, addr, len),
            Some(BlkReqType::Out) if !data.flags.is_write() => {
                self.blk_write(dma, sector, addr, len)
            }
            Some(_) => (BlkStatus::IoErr, 0), // descriptor flag mismatch
            None => (BlkStatus::Unsupp, 0),
        }
    }

    fn blk_read(&self, dma: &mut DmaCtx, sector: u64, addr: usize, len: usize) -> (BlkStatus, u32) {
        let off = match self.sector_offset(sector, len) {
            Some(o) => o,
            None => return (BlkStatus::IoErr, 0),
        };
        dma.write_bytes(addr, &self.disk[off..off + len])
            .map_or((BlkStatus::IoErr, 0), |()| (BlkStatus::Ok, len as u32))
    }

    fn blk_write(
        &mut self,
        dma: &DmaCtx,
        sector: u64,
        addr: usize,
        len: usize,
    ) -> (BlkStatus, u32) {
        let off = match self.sector_offset(sector, len) {
            Some(o) => o,
            None => return (BlkStatus::IoErr, 0),
        };
        let mut buf = vec![0u8; len];
        if dma.read_bytes(addr, &mut buf).is_err() {
            return (BlkStatus::IoErr, 0);
        }
        self.disk[off..off + len].copy_from_slice(&buf);
        (BlkStatus::Ok, 0)
    }
}

/// VirtIO MMIO legacy (v1) block device.
pub struct VirtioBlk {
    // Transport
    status: u32,
    dev_features_sel: u32,
    drv_features_sel: u32,
    drv_features: [u32; 2],
    queue: Virtqueue,
    interrupt_status: u32,
    notify_pending: bool,
    // Block storage (split from transport for safe borrow in process_dma)
    storage: BlkStorage,
}

impl VirtioBlk {
    /// Create a block device backed by an in-memory disk snapshot.
    pub fn new(disk: Vec<u8>) -> Self {
        Self {
            status: 0,
            dev_features_sel: 0,
            drv_features_sel: 0,
            drv_features: [0; 2],
            queue: Virtqueue::default(),
            interrupt_status: 0,
            notify_pending: false,
            storage: BlkStorage::new(disk),
        }
    }

    /// Config space read (relative to 0x100). Only field: capacity (u64 LE).
    fn read_config(&self, offset: usize, size: usize) -> Word {
        if offset >= 8 {
            return 0;
        }
        let bytes = self.storage.capacity.to_le_bytes();
        let end = (offset + size).min(8);
        let mut buf = [0u8; 4];
        buf[..end - offset].copy_from_slice(&bytes[offset..end]);
        u32::from_le_bytes(buf) as Word
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
            Some(Reg::Magic) => VIRTIO_MAGIC as Word,
            Some(Reg::Version) => VIRTIO_VERSION as Word,
            Some(Reg::DeviceId) => VIRTIO_BLK_ID as Word,
            Some(Reg::VendorId) => VIRTIO_VENDOR as Word,
            Some(Reg::DevFeatures) => 0, // no optional features
            Some(Reg::QueueNumMax) => QUEUE_NUM_MAX as Word,
            Some(Reg::QueuePfn) => self.queue.pfn() as Word,
            Some(Reg::InterruptStatus) => self.interrupt_status as Word,
            Some(Reg::Status) => self.status as Word,
            _ => 0,
        })
    }

    fn write(&mut self, offset: usize, _size: usize, value: Word) -> XResult {
        #[allow(clippy::unnecessary_cast)]
        let val = value as u32;
        match Reg::decode(offset) {
            Some(Reg::DevFeaturesSel) => self.dev_features_sel = val,
            Some(Reg::DrvFeatures) => {
                if let Some(slot) = self.drv_features.get_mut(self.drv_features_sel as usize) {
                    *slot = val;
                }
            }
            Some(Reg::DrvFeaturesSel) => self.drv_features_sel = val,
            Some(Reg::GuestPageSize) => self.queue.set_page_size(val),
            Some(Reg::QueueSel) => {} // single queue — non-zero ignored
            Some(Reg::QueueNum) => self.queue.set_num(val as u16),
            Some(Reg::QueueAlign) => self.queue.set_align(val),
            Some(Reg::QueuePfn) => self.queue.set_pfn(val),
            Some(Reg::QueueNotify) => self.notify_pending = true,
            Some(Reg::InterruptAck) => self.interrupt_status &= !val,
            Some(Reg::Status) => {
                if val == 0 {
                    self.soft_reset();
                } else {
                    self.status = val;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn irq_line(&self) -> bool {
        self.interrupt_status != 0
    }

    fn reset(&mut self) {
        self.soft_reset();
    }

    fn hard_reset(&mut self) {
        self.soft_reset();
        self.storage.disk = self.storage.original.clone();
    }

    fn take_notify(&mut self) -> bool {
        std::mem::take(&mut self.notify_pending)
    }

    fn process_dma(&mut self, dma: &mut DmaCtx) {
        let storage = &mut self.storage;
        let completed = self
            .queue
            .poll(dma, |dma, descs| storage.handle_chain(dma, descs))
            .unwrap_or(0);
        if completed > 0 {
            self.interrupt_status |= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::device::{bus::DmaCtx, ram::Ram};

    fn new_blk() -> VirtioBlk {
        VirtioBlk::new(vec![0u8; 512 * 8])
    }

    #[test]
    fn magic_version_device_id() {
        let mut blk = new_blk();
        assert_eq!(blk.read(0x000, 4).unwrap() as u32, 0x7472_6976);
        assert_eq!(blk.read(0x004, 4).unwrap() as u32, 1);
        assert_eq!(blk.read(0x008, 4).unwrap() as u32, 2);
        assert_eq!(blk.read(0x00c, 4).unwrap() as u32, 0x554d_4551);
    }

    #[test]
    fn queue_num_max() {
        let mut blk = new_blk();
        assert_eq!(blk.read(0x034, 4).unwrap() as u32, 128);
    }

    #[test]
    fn feature_selectors_independent() {
        let mut blk = new_blk();
        blk.write(0x014, 4, 1).unwrap(); // DevFeaturesSel = 1
        blk.write(0x024, 4, 0).unwrap(); // DrvFeaturesSel = 0
        assert_eq!(blk.dev_features_sel, 1);
        assert_eq!(blk.drv_features_sel, 0);
    }

    #[test]
    fn status_transitions_and_reset() {
        let mut blk = new_blk();
        blk.write(0x070, 4, 1).unwrap(); // ACK
        blk.write(0x070, 4, 3).unwrap(); // DRIVER
        blk.write(0x070, 4, 7).unwrap(); // DRIVER_OK
        assert_eq!(blk.read(0x070, 4).unwrap() as u32, 7);

        blk.write(0x070, 4, 0).unwrap(); // reset
        assert_eq!(blk.read(0x070, 4).unwrap() as u32, 0);
    }

    #[test]
    fn interrupt_ack_clears() {
        let mut blk = new_blk();
        blk.interrupt_status = 1;
        assert!(blk.irq_line());
        blk.write(0x064, 4, 1).unwrap();
        assert!(!blk.irq_line());
    }

    #[test]
    fn config_capacity() {
        let blk = VirtioBlk::new(vec![0u8; 512 * 100]);
        assert_eq!(blk.storage.capacity, 100);
        let mut blk = blk;
        let lo = blk.read(0x100, 4).unwrap() as u32;
        let hi = blk.read(0x104, 4).unwrap() as u32;
        assert_eq!(lo, 100);
        assert_eq!(hi, 0);
    }

    #[test]
    fn soft_reset_preserves_disk() {
        let mut blk = VirtioBlk::new(vec![0u8; 512]);
        blk.storage.disk[0] = 0xAB;
        blk.soft_reset();
        assert_eq!(blk.storage.disk[0], 0xAB);
    }

    #[test]
    fn hard_reset_restores_disk() {
        let mut blk = VirtioBlk::new(vec![0u8; 512]);
        blk.storage.disk[0] = 0xAB;
        blk.hard_reset();
        assert_eq!(blk.storage.disk[0], 0x00);
    }

    #[test]
    fn sector_overflow_returns_ioerr() {
        let storage = BlkStorage::new(vec![0u8; 512]);
        assert!(storage.sector_offset(u64::MAX, 512).is_none());
        assert!(storage.sector_offset(u64::MAX / 512 + 1, 512).is_none());
    }

    #[test]
    fn dma_read_write_roundtrip() {
        let mut ram = Ram::new(0x8000_0000, 0x1000);
        let mut dma = DmaCtx::test_new(&mut ram, 0x8000_0000);
        dma.write_val::<u32>(0x8000_0000, 0xDEAD_BEEF).unwrap();
        assert_eq!(dma.read_val::<u32>(0x8000_0000).unwrap(), 0xDEAD_BEEF);
        dma.write_val::<u16>(0x8000_0010, 0xCAFE).unwrap();
        assert_eq!(dma.read_val::<u16>(0x8000_0010).unwrap(), 0xCAFE);
    }
}
