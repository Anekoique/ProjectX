use std::ops::Range;

use super::{Device, ram::Ram};
use crate::{
    config::Word,
    error::{XError, XResult},
};

fn overlaps(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

struct MmioRegion {
    name: &'static str,
    range: Range<usize>,
    dev: Box<dyn Device>,
}

impl MmioRegion {
    #[inline]
    fn contains(&self, addr: usize, end: usize) -> bool {
        addr >= self.range.start && end <= self.range.end
    }
}

pub struct Bus {
    ram: Ram,
    mmio: Vec<MmioRegion>,
}

impl Bus {
    pub fn new(ram_base: usize, ram_size: usize) -> Self {
        Self {
            ram: Ram::new(ram_base, ram_size),
            mmio: Vec::new(),
        }
    }

    pub fn add_mmio(&mut self, name: &'static str, base: usize, size: usize, dev: Box<dyn Device>) {
        assert!(size > 0, "region size must be non-zero");
        let range = base..base.checked_add(size).expect("address overflow");
        assert!(
            !overlaps(&range, self.ram.range()),
            "MMIO '{name}' overlaps RAM"
        );

        if let Some(r) = self.mmio.iter().find(|r| overlaps(&range, &r.range)) {
            panic!("MMIO '{name}' overlaps '{}'", r.name);
        }

        self.mmio.push(MmioRegion { name, range, dev });
    }

    fn find_mmio(&mut self, addr: usize, size: usize) -> XResult<(&mut dyn Device, usize)> {
        let end = addr.checked_add(size).ok_or(XError::BadAddress)?;

        let r = self
            .mmio
            .iter_mut()
            .find(|r| r.contains(addr, end))
            .ok_or(XError::BadAddress)?;

        Ok((r.dev.as_mut(), addr - r.range.start))
    }

    fn dispatch<T, F>(&mut self, addr: usize, size: usize, f: F) -> XResult<T>
    where
        F: FnOnce(&mut dyn Device, usize) -> XResult<T>,
    {
        if let Some(off) = self.ram_offset(addr, size) {
            return f(&mut self.ram, off);
        }

        let (dev, off) = self.find_mmio(addr, size)?;
        f(dev, off)
    }

    pub fn read(&mut self, addr: usize, size: usize) -> XResult<Word> {
        self.dispatch(addr, size, |dev, off| dev.read(off, size))
    }

    pub fn write(&mut self, addr: usize, size: usize, value: Word) -> XResult {
        self.dispatch(addr, size, |dev, off| dev.write(off, size, value))
    }

    pub fn read_ram(&self, addr: usize, size: usize) -> XResult<Word> {
        self.ram_offset(addr, size)
            .ok_or(XError::BadAddress)
            .and_then(|off| self.ram.read(off, size))
    }

    pub fn load_ram(&mut self, addr: usize, data: &[u8]) -> XResult {
        self.ram_offset(addr, data.len())
            .ok_or(XError::BadAddress)
            .and_then(|off| self.ram.load(off, data))
    }

    fn ram_offset(&self, addr: usize, size: usize) -> Option<usize> {
        let off = addr.checked_sub(self.ram.range().start)?;
        let end = off.checked_add(size)?;
        (end <= self.ram.len()).then_some(off)
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

    struct StubDevice(Word);
    impl StubDevice {
        fn new() -> Self {
            Self(0)
        }
    }
    impl Device for StubDevice {
        fn read(&self, _: usize, _: usize) -> XResult<Word> {
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
