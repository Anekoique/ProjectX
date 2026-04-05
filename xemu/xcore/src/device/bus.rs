//! Memory bus: RAM backing store + MMIO region dispatch with split-tick
//! optimization (ACLINT every step, UART/PLIC every 64 steps).

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

/// Slow-tick divisor: UART/PLIC are ticked every N instructions.
const SLOW_TICK_DIVISOR: u64 = 64;

/// Memory bus: dispatches reads/writes to RAM or MMIO devices.
pub struct Bus {
    ram: Ram,
    pub(crate) mmio: Vec<MmioRegion>,
    aclint_idx: Option<usize>,
    plic_idx: Option<usize>,
    tick_count: u64,
    ssip_pending: Arc<AtomicBool>,
    #[cfg(feature = "difftest")]
    mmio_accessed: AtomicBool,
}

impl Bus {
    /// Create a bus with RAM at the given base address and size.
    pub fn new(ram_base: usize, ram_size: usize) -> Self {
        Self {
            ram: Ram::new(ram_base, ram_size),
            mmio: Vec::new(),
            aclint_idx: None,
            plic_idx: None,
            tick_count: 0,
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

    /// Register an MMIO device at the given address range.
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

    /// Swap the device backing a named MMIO region.
    pub fn replace_device(&mut self, name: &str, dev: Box<dyn Device>) {
        self.mmio
            .iter_mut()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("bus: no device named '{name}'"))
            .dev = dev;
    }

    /// Designate a device as the timer source (ACLINT).
    pub fn set_timer_source(&mut self, idx: usize) {
        self.aclint_idx = Some(idx);
    }

    /// Designate a device as the interrupt controller (PLIC).
    pub fn set_irq_sink(&mut self, idx: usize) {
        self.plic_idx = Some(idx);
    }

    /// Read mtime directly from ACLINT (avoids MMIO dispatch).
    #[inline]
    pub fn mtime(&self) -> u64 {
        self.aclint_idx
            .and_then(|i| self.mmio[i].dev.mtime())
            .unwrap_or(0)
    }

    /// Tick devices. ACLINT ticks every step; slow devices (UART, PLIC)
    /// tick every `SLOW_TICK_DIVISOR` steps to reduce overhead.
    pub fn tick(&mut self) {
        // Fast path: always tick ACLINT (timer source)
        if let Some(i) = self.aclint_idx {
            self.mmio[i].dev.tick();
        }
        self.tick_count += 1;
        if !self.tick_count.is_multiple_of(SLOW_TICK_DIVISOR) {
            return;
        }
        // Slow path: tick remaining devices, collect IRQ lines, notify PLIC
        let irq_lines = self
            .mmio
            .iter_mut()
            .enumerate()
            .fold(0u32, |lines, (idx, r)| {
                if Some(idx) != self.aclint_idx {
                    r.dev.tick();
                }
                if r.irq_source > 0 && r.dev.irq_line() {
                    lines | (1 << r.irq_source)
                } else {
                    lines
                }
            });
        if let Some(i) = self.plic_idx {
            self.mmio[i].dev.notify(irq_lines);
        }
    }

    /// Hard-reset all MMIO devices to power-on state.
    pub fn reset_devices(&mut self) {
        debug!("bus: resetting all devices");
        for r in &mut self.mmio {
            r.dev.hard_reset();
        }
    }

    fn find_mmio_idx(&self, addr: usize, size: usize) -> XResult<usize> {
        let end = addr.checked_add(size).ok_or(XError::BadAddress)?;
        self.mmio
            .iter()
            .position(|r| r.contains(addr, end))
            .ok_or(XError::BadAddress)
    }

    /// Read from RAM or MMIO device at physical address.
    pub fn read(&mut self, addr: usize, size: usize) -> XResult<Word> {
        if let Some(off) = self.ram_offset(addr, size) {
            return self.ram.read(off, size);
        }
        #[cfg(feature = "difftest")]
        self.mmio_accessed.store(true, Relaxed);
        let idx = self.find_mmio_idx(addr, size)?;
        let off = addr - self.mmio[idx].range.start;
        self.mmio[idx].dev.read(off, size)
    }

    /// Write to RAM or MMIO device at physical address.
    pub fn write(&mut self, addr: usize, size: usize, value: Word) -> XResult {
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

    /// If a device flagged a pending DMA notification, process it now.
    fn maybe_process_dma(&mut self, idx: usize) {
        if !self.mmio[idx].dev.take_notify() {
            return;
        }
        let ram_base = self.ram.range().start;
        let mut dma = DmaCtx {
            ram: &mut self.ram,
            ram_base,
        };
        self.mmio[idx].dev.process_dma(&mut dma);
    }

    /// Direct RAM read (no MMIO dispatch, no side effects).
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

    /// Bulk-load bytes into RAM at physical address.
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

// ---------------------------------------------------------------------------
// DMA context — safe guest-memory accessor for DMA-capable devices
// ---------------------------------------------------------------------------

/// Safe guest-memory accessor created by Bus during DMA processing.
pub struct DmaCtx<'a> {
    ram: &'a mut Ram,
    ram_base: usize,
}

impl<'a> DmaCtx<'a> {
    /// Create a DmaCtx for unit tests (not used in production).
    #[cfg(test)]
    pub fn test_new(ram: &'a mut Ram, ram_base: usize) -> Self {
        Self { ram, ram_base }
    }

    fn offset(&self, paddr: usize, len: usize) -> XResult<usize> {
        paddr
            .checked_sub(self.ram_base)
            .filter(|&off| {
                off.checked_add(len)
                    .is_some_and(|end| end <= self.ram.len())
            })
            .ok_or(XError::BadAddress)
    }

    /// Read a contiguous byte slice from guest physical address.
    pub fn read_bytes(&self, paddr: usize, buf: &mut [u8]) -> XResult {
        let off = self.offset(paddr, buf.len())?;
        self.ram.read_bytes(off, buf)
    }

    /// Write a contiguous byte slice to guest physical address.
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

/// Little-endian byte conversion for DMA primitive reads/writes.
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

    #[test]
    fn replace_device_swaps_named_region() {
        let mut bus = new_bus();
        bus.add_mmio("stub", MMIO_BASE, MMIO_SIZE, stub(), 0);
        bus.write(MMIO_BASE, 4, 0x42).unwrap();
        assert_eq!(bus.read(MMIO_BASE, 4).unwrap(), 0x42);

        bus.replace_device("stub", Box::new(StubDevice(0x99)));
        assert_eq!(bus.read(MMIO_BASE, 4).unwrap(), 0x99);
    }

    #[test]
    #[should_panic(expected = "no device named")]
    fn replace_device_panics_on_unknown_name() {
        let mut bus = new_bus();
        bus.replace_device("nonexistent", stub());
    }
}
