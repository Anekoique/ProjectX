use super::{Device, ram::Ram};
use crate::{
    config::Word,
    error::{XError, XResult},
};

struct MmioRegion {
    name: &'static str,
    base: usize,
    size: usize,
    dev: Box<dyn Device>,
}

pub struct Bus {
    ram: Ram,
    ram_base: usize,
    mmio: Vec<MmioRegion>,
}

impl Bus {
    pub fn new(ram_base: usize, ram_size: usize) -> Self {
        Self {
            ram: Ram::new(ram_size),
            ram_base,
            mmio: Vec::new(),
        }
    }

    /// Register an MMIO device. Panics on overlap with RAM or existing regions.
    pub fn add_mmio(&mut self, name: &'static str, base: usize, size: usize, dev: Box<dyn Device>) {
        assert!(size > 0, "region size must be non-zero");
        let end = base
            .checked_add(size)
            .expect("region overflows address space");
        let overlaps = |lo: usize, hi: usize| base < hi && lo < end;
        assert!(
            !overlaps(self.ram_base, self.ram_base + self.ram.len()),
            "MMIO '{name}' [{base:#x}..{end:#x}) overlaps RAM"
        );
        for r in &self.mmio {
            assert!(
                !overlaps(r.base, r.base + r.size),
                "MMIO '{name}' [{base:#x}..{end:#x}) overlaps '{}'",
                r.name
            );
        }
        self.mmio.push(MmioRegion {
            name,
            base,
            size,
            dev,
        });
    }

    pub fn read(&mut self, addr: usize, size: usize) -> XResult<Word> {
        if let Some(off) = self.ram_offset(addr, size) {
            return self.ram.read(off, size);
        }
        let (dev, off) = self.find_mmio(addr, size)?;
        dev.read(off, size)
    }

    pub fn write(&mut self, addr: usize, size: usize, value: Word) -> XResult {
        if let Some(off) = self.ram_offset(addr, size) {
            return self.ram.write(off, size, value);
        }
        let (dev, off) = self.find_mmio(addr, size)?;
        dev.write(off, size, value)
    }

    /// Read from RAM only. Used by page table walks.
    pub fn read_ram(&self, addr: usize, size: usize) -> XResult<Word> {
        let off = self.ram_offset(addr, size).ok_or(XError::BadAddress)?;
        self.ram.read(off, size)
    }

    /// Bulk load bytes directly into RAM (for image/ELF loading).
    pub fn load_ram(&mut self, addr: usize, data: &[u8]) -> XResult {
        let off = self
            .ram_offset(addr, data.len())
            .ok_or(XError::BadAddress)?;
        self.ram.load(off, data)
    }

    fn ram_offset(&self, addr: usize, size: usize) -> Option<usize> {
        let off = addr.checked_sub(self.ram_base)?;
        if off + size <= self.ram.len() {
            Some(off)
        } else {
            None
        }
    }

    fn find_mmio(&mut self, addr: usize, size: usize) -> XResult<(&mut dyn Device, usize)> {
        for r in &mut self.mmio {
            if addr >= r.base && addr + size <= r.base + r.size {
                return Ok((r.dev.as_mut(), addr - r.base));
            }
        }
        Err(XError::BadAddress)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{CONFIG_MBASE, CONFIG_MSIZE};

    fn new_bus() -> Bus {
        Bus::new(CONFIG_MBASE, CONFIG_MSIZE)
    }

    #[test]
    fn ram_read_write() {
        let mut bus = new_bus();
        for (addr, sz, val) in [
            (CONFIG_MBASE, 1, 0xFF),
            (CONFIG_MBASE + 2, 2, 0xBEEF),
            (CONFIG_MBASE + 4, 4, 0xDEADBEEF),
        ] {
            bus.write(addr, sz, val).unwrap();
            assert_eq!(bus.read(addr, sz).unwrap(), val, "size={sz}");
        }
    }

    #[test]
    fn unmapped_returns_bad_address() {
        let mut bus = new_bus();
        assert!(matches!(bus.read(0, 4), Err(XError::BadAddress)));
        assert!(matches!(bus.write(0, 4, 0), Err(XError::BadAddress)));
    }

    #[test]
    fn read_ram_only() {
        let mut bus = new_bus();
        bus.write(CONFIG_MBASE, 4, 0x12345678).unwrap();
        assert_eq!(bus.read_ram(CONFIG_MBASE, 4).unwrap() as u32, 0x12345678);
        assert!(matches!(bus.read_ram(0, 4), Err(XError::BadAddress)));
    }

    #[test]
    fn load_ram_bulk() {
        let mut bus = new_bus();
        bus.load_ram(CONFIG_MBASE, &[0x11, 0x22, 0x33, 0x44])
            .unwrap();
        assert_eq!(bus.read(CONFIG_MBASE, 4).unwrap() as u32, 0x44332211);
    }

    #[test]
    fn load_ram_bounds() {
        let mut bus = new_bus();
        assert!(
            bus.load_ram(CONFIG_MBASE + CONFIG_MSIZE - 2, &[0u8; 4])
                .is_err()
        );
        assert!(bus.load_ram(CONFIG_MBASE - 4, &[0u8; 4]).is_err());
    }

    // --- MMIO ---

    struct StubDevice(Word);
    impl StubDevice {
        fn new() -> Self {
            Self(0)
        }
    }
    impl Device for StubDevice {
        fn read(&mut self, _: usize, _: usize) -> XResult<Word> {
            Ok(self.0)
        }
        fn write(&mut self, _: usize, _: usize, v: Word) -> XResult {
            self.0 = v;
            Ok(())
        }
    }
    fn stub() -> Box<dyn Device> {
        Box::new(StubDevice::new())
    }

    const MMIO_BASE: usize = 0xA000_0000;
    const MMIO_SIZE: usize = 0x100;

    #[test]
    fn mmio_read_write() {
        let mut bus = new_bus();
        bus.add_mmio("stub", MMIO_BASE, MMIO_SIZE, stub());
        bus.write(MMIO_BASE, 4, 0x42).unwrap();
        assert_eq!(bus.read(MMIO_BASE, 4).unwrap(), 0x42);
    }

    #[test]
    fn read_ram_rejects_mmio() {
        let mut bus = new_bus();
        bus.add_mmio("stub", MMIO_BASE, MMIO_SIZE, stub());
        assert!(matches!(
            bus.read_ram(MMIO_BASE, 4),
            Err(XError::BadAddress)
        ));
    }

    #[test]
    #[should_panic(expected = "overlaps RAM")]
    fn mmio_overlaps_ram() {
        new_bus().add_mmio("bad", CONFIG_MBASE, 0x100, stub());
    }

    #[test]
    #[should_panic(expected = "overlaps")]
    fn mmio_overlaps_mmio() {
        let mut bus = new_bus();
        bus.add_mmio("a", MMIO_BASE, MMIO_SIZE, stub());
        bus.add_mmio("b", MMIO_BASE + MMIO_SIZE / 2, MMIO_SIZE, stub());
    }

    #[test]
    #[should_panic(expected = "non-zero")]
    fn mmio_zero_size() {
        new_bus().add_mmio("empty", MMIO_BASE, 0, stub());
    }
}
