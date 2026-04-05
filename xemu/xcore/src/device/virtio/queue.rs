//! Split virtqueue for VirtIO legacy (v1) transport.
//!
//! Manages ring addresses, descriptor chain walking, and used-ring completion.

use super::defs::{BlkStatus, DescFlags};
use crate::{device::bus::DmaCtx, error::XResult};

/// Parsed descriptor read from guest memory (16 bytes).
pub struct Desc {
    pub addr: u64,
    pub len: u32,
    pub flags: DescFlags,
    pub next: u16,
}

/// Split virtqueue state for legacy (v1) MMIO transport.
pub struct Virtqueue {
    num: u16,
    pfn: u32,
    align: u32,
    /// Guest page size — set by transport, survives device reset.
    page_size: u32,
    last_avail_idx: u16,
}

impl Default for Virtqueue {
    fn default() -> Self {
        Self {
            num: 0,
            pfn: 0,
            align: 4096,
            page_size: 0,
            last_avail_idx: 0,
        }
    }
}

impl Virtqueue {
    pub fn pfn(&self) -> u32 {
        self.pfn
    }
    pub fn set_num(&mut self, n: u16) {
        self.num = n;
    }
    pub fn set_pfn(&mut self, pfn: u32) {
        self.pfn = pfn;
    }
    pub fn set_align(&mut self, align: u32) {
        self.align = align.max(1);
    }
    pub fn set_page_size(&mut self, ps: u32) {
        self.page_size = ps;
    }

    pub fn configured(&self) -> bool {
        self.pfn != 0 && self.page_size != 0
    }

    /// Compute (desc_base, avail_base, used_base) from legacy layout.
    fn ring_addrs(&self) -> (usize, usize, usize) {
        let base = self.pfn as usize * self.page_size as usize;
        let n = self.num as usize;
        let align = self.align as usize;
        let desc = base;
        let avail = base + 16 * n;
        let used = (avail + 6 + 2 * n + align - 1) & !(align - 1);
        (desc, avail, used)
    }

    /// Read one descriptor from guest memory.
    fn read_desc(dma: &DmaCtx, desc_base: usize, idx: u16) -> XResult<Desc> {
        let mut buf = [0u8; 16];
        dma.read_bytes(desc_base + idx as usize * 16, &mut buf)?;
        Ok(Desc {
            addr: u64::from_le_bytes(buf[0..8].try_into().unwrap()),
            len: u32::from_le_bytes(buf[8..12].try_into().unwrap()),
            flags: DescFlags(u16::from_le_bytes(buf[12..14].try_into().unwrap())),
            next: u16::from_le_bytes(buf[14..16].try_into().unwrap()),
        })
    }

    /// Collect all descriptors in a chain, bounded by queue size (I-2).
    fn collect_chain(&self, dma: &DmaCtx, desc_base: usize, head: u16) -> XResult<Vec<Desc>> {
        let mut chain = Vec::new();
        let mut idx = head;
        loop {
            if chain.len() >= self.num as usize {
                break;
            }
            let desc = Self::read_desc(dma, desc_base, idx)?;
            let cont = desc.flags.has_next();
            let next = desc.next;
            chain.push(desc);
            if !cont {
                break;
            }
            idx = next;
        }
        Ok(chain)
    }

    /// Process all pending available entries.
    ///
    /// Calls `handler` for each descriptor chain; the handler returns
    /// `(status, bytes_written)`. Returns the number of completed chains.
    pub fn poll(
        &mut self,
        dma: &mut DmaCtx,
        mut handler: impl FnMut(&mut DmaCtx, &[Desc]) -> (BlkStatus, u32),
    ) -> XResult<u32> {
        if !self.configured() {
            return Ok(0);
        }
        let (desc_base, avail_base, used_base) = self.ring_addrs();
        let avail_idx = dma.read_val::<u16>(avail_base + 2)?;
        let mut completed = 0u32;

        while self.last_avail_idx != avail_idx {
            let ring_off = (self.last_avail_idx % self.num) as usize;
            let head = dma.read_val::<u16>(avail_base + 4 + ring_off * 2)?;
            let chain = self.collect_chain(dma, desc_base, head)?;

            let (status, written) = handler(dma, &chain);

            // Write status byte to last descriptor's buffer
            if let Some(last) = chain.last() {
                let _ = dma.write_val(last.addr as usize, status as u8);
            }

            // Append used ring entry
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

    /// Clear queue state (VirtIO transport reset).
    /// Preserves `page_size` — it's a transport-level register set once by the
    /// driver and must survive device resets.
    pub fn reset(&mut self) {
        self.num = 0;
        self.pfn = 0;
        self.align = 4096;
        self.last_avail_idx = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_addrs_legacy_layout() {
        let mut q = Virtqueue::default();
        q.set_page_size(4096);
        q.set_num(128);
        q.set_pfn(1);
        let (desc, avail, used) = q.ring_addrs();
        // base = 4096, desc = 4096, avail = 4096 + 16*128 = 6144
        assert_eq!(desc, 4096);
        assert_eq!(avail, 4096 + 2048);
        // used = align_up(6144 + 6 + 256, 4096) = align_up(6406, 4096) = 8192
        assert_eq!(used, 8192);
    }

    #[test]
    fn unconfigured_poll_returns_zero() {
        let mut q = Virtqueue::default();
        let mut ram = crate::device::ram::Ram::new(0x8000_0000, 0x1000);
        let mut dma = DmaCtx::test_new(&mut ram, 0x8000_0000);
        let count = q.poll(&mut dma, |_, _| (BlkStatus::Ok, 0)).unwrap();
        assert_eq!(count, 0);
    }
}
