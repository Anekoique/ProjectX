use std::{
    ops::Range,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
};

use super::{Device, ram::Ram};
use crate::{
    config::Word,
    error::{XError, XResult},
};

fn overlaps(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

pub(crate) struct MmioRegion {
    pub name: &'static str,
    pub range: Range<usize>,
    pub dev: Box<dyn Device>,
    pub irq_source: u32,
}

impl MmioRegion {
    #[inline]
    fn contains(&self, addr: usize, end: usize) -> bool {
        addr >= self.range.start && end <= self.range.end
    }
}

pub struct Bus {
    ram: Ram,
    pub(crate) mmio: Vec<MmioRegion>,
    plic_idx: Option<usize>,
    ssip_pending: Arc<AtomicBool>,
    #[cfg(feature = "difftest")]
    mmio_accessed: AtomicBool,
}

impl Bus {
    pub fn new(ram_base: usize, ram_size: usize) -> Self {
        Self {
            ram: Ram::new(ram_base, ram_size),
            mmio: Vec::new(),
            plic_idx: None,
            ssip_pending: Arc::new(AtomicBool::new(false)),
            #[cfg(feature = "difftest")]
            mmio_accessed: AtomicBool::new(false),
        }
    }

    /// Returns the shared SSIP pending flag for ACLINT SSWI edge delivery.
    pub fn ssip_flag(&self) -> Arc<AtomicBool> {
        self.ssip_pending.clone()
    }

    /// Consume and return the SSIP edge signal.
    pub fn take_ssip(&self) -> bool {
        self.ssip_pending.swap(false, Relaxed)
    }

    pub fn add_mmio(
        &mut self,
        name: &'static str,
        base: usize,
        size: usize,
        dev: Box<dyn Device>,
        irq_source: u32,
    ) {
        assert!(size > 0, "region size must be non-zero");
        assert!(irq_source < 32, "irq_source must be < 32");
        let range = base..base.checked_add(size).expect("address overflow");
        assert!(
            !overlaps(&range, self.ram.range()),
            "MMIO '{name}' overlaps RAM"
        );
        if let Some(r) = self.mmio.iter().find(|r| overlaps(&range, &r.range)) {
            panic!("MMIO '{name}' overlaps '{}'", r.name);
        }
        info!("bus: add_mmio '{}' base={:#x} size={:#x}", name, base, size);
        self.mmio.push(MmioRegion {
            name,
            range,
            dev,
            irq_source,
        });
    }

    /// Designate a device as the interrupt controller (receives `notify()` with
    /// irq_lines).
    pub fn set_irq_sink(&mut self, idx: usize) {
        self.plic_idx = Some(idx);
    }

    /// Tick all devices, collect IRQ lines, notify PLIC.
    pub fn tick(&mut self) {
        let mut irq_lines: u32 = 0;
        for r in &mut self.mmio {
            r.dev.tick();
            if r.irq_source > 0 && r.dev.irq_line() {
                irq_lines |= 1 << r.irq_source;
            }
        }
        if let Some(i) = self.plic_idx {
            self.mmio[i].dev.notify(irq_lines);
        }
    }

    /// Reset all MMIO devices to initial state.
    pub fn reset_devices(&mut self) {
        debug!("bus: resetting all devices");
        for r in &mut self.mmio {
            r.dev.reset();
        }
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

    fn dispatch<T>(
        &mut self,
        addr: usize,
        size: usize,
        f: impl FnOnce(&mut dyn Device, usize) -> XResult<T>,
    ) -> XResult<T> {
        if let Some(off) = self.ram_offset(addr, size) {
            return f(&mut self.ram, off);
        }
        #[cfg(feature = "difftest")]
        self.mmio_accessed.store(true, Relaxed);
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
            .and_then(|off| self.ram.get(off, size))
    }

    /// Returns and clears the MMIO-accessed flag (for difftest MMIO-skip).
    #[cfg(feature = "difftest")]
    pub fn take_mmio_flag(&self) -> bool {
        self.mmio_accessed.swap(false, Relaxed)
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
        Box::new(StubDevice(0))
    }

    const MMIO_BASE: usize = 0xA000_0000;
    const MMIO_SIZE: usize = 0x100;

    #[test]
    fn mmio_read_write() {
        let mut bus = new_bus();
        bus.add_mmio("stub", MMIO_BASE, MMIO_SIZE, stub(), 0);
        bus.write(MMIO_BASE, 4, 0x42).unwrap();
        assert_eq!(bus.read(MMIO_BASE, 4).unwrap(), 0x42);
    }

    #[test]
    fn read_ram_rejects_mmio() {
        let mut bus = new_bus();
        bus.add_mmio("stub", MMIO_BASE, MMIO_SIZE, stub(), 0);
        assert!(matches!(
            bus.read_ram(MMIO_BASE, 4),
            Err(XError::BadAddress)
        ));
    }

    #[test]
    #[should_panic(expected = "overlaps RAM")]
    fn mmio_overlaps_ram() {
        new_bus().add_mmio("bad", CONFIG_MBASE, 0x100, stub(), 0);
    }

    #[test]
    #[should_panic(expected = "overlaps")]
    fn mmio_overlaps_mmio() {
        let mut bus = new_bus();
        bus.add_mmio("a", MMIO_BASE, MMIO_SIZE, stub(), 0);
        bus.add_mmio("b", MMIO_BASE + MMIO_SIZE / 2, MMIO_SIZE, stub(), 0);
    }

    #[test]
    #[should_panic(expected = "non-zero")]
    fn mmio_zero_size() {
        new_bus().add_mmio("empty", MMIO_BASE, 0, stub(), 0);
    }

    #[test]
    fn irq_sink_set_explicitly() {
        let mut bus = new_bus();
        assert!(bus.plic_idx.is_none());
        bus.add_mmio("plic", MMIO_BASE, MMIO_SIZE, stub(), 0);
        bus.set_irq_sink(0);
        assert_eq!(bus.plic_idx, Some(0));
    }
}
