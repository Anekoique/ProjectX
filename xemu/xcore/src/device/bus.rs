//! Memory bus: RAM backing store + MMIO region dispatch with split-tick
//! optimization (MTIMER every step, UART/PLIC every 64 steps).
//!
//! The bus owns per-hart state shared across all harts in the system:
//! LR/SC reservations and the SSWI edge flag. Every physical store should
//! route through [`Bus::store`], which performs the write and invalidates
//! peer reservations covering the same 8-byte granule (RISC-V A-extension
//! §8.2 cross-hart invalidation rule).

use std::ops::Range;
#[cfg(feature = "difftest")]
use std::sync::atomic::{AtomicBool, Ordering::Relaxed};

use super::{Device, ram::Ram};
use crate::{
    config::Word,
    cpu::core::HartId,
    error::{XError, XResult},
};

/// LR/SC reservation granule: peer-hart stores within this many bytes of an
/// active reservation invalidate it. RISC-V A-extension §8.2 mandates a
/// natural-aligned granule of 2..=4096 bytes; 8 is a conservative minimum
/// that covers all normal store widths up through `sd`.
const RESERVATION_GRANULE: usize = 8;

fn overlaps(a: &Range<usize>, b: &Range<usize>) -> bool {
    a.start < b.end && b.start < a.end
}

/// True if the granule-aligned range of reservation `r` overlaps the store
/// range `[addr, end)`.
#[inline]
fn granule_overlaps(r: usize, addr: usize, end: usize) -> bool {
    let base = r & !(RESERVATION_GRANULE - 1);
    base < end && base + RESERVATION_GRANULE > addr
}

pub(crate) struct MmioRegion {
    pub name: &'static str,
    pub range: Range<usize>,
    pub dev: Box<dyn Device>,
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
    mtimer_idx: Option<usize>,
    plic_idx: Option<usize>,
    tick_count: u64,
    num_harts: usize,
    /// Per-hart LR/SC reservation: physical address of the reserved
    /// 8-byte granule, or `None` if the hart has no live reservation.
    reservations: Vec<Option<usize>>,
    #[cfg(feature = "difftest")]
    mmio_accessed: AtomicBool,
}

impl Bus {
    /// Create a bus with RAM at the given base address and size, sized for
    /// `num_harts` harts.
    pub fn new(ram_base: usize, ram_size: usize, num_harts: usize) -> Self {
        debug_assert!((1..=16).contains(&num_harts), "num_harts must be in 1..=16");
        Self {
            ram: Ram::new(ram_base, ram_size),
            mmio: Vec::new(),
            mtimer_idx: None,
            plic_idx: None,
            tick_count: 0,
            num_harts,
            reservations: vec![None; num_harts],
            #[cfg(feature = "difftest")]
            mmio_accessed: AtomicBool::new(false),
        }
    }

    /// Number of harts this bus was sized for.
    pub fn num_harts(&self) -> usize {
        self.num_harts
    }

    /// Record an LR reservation at `addr` for `hart`.
    pub fn reserve(&mut self, hart: HartId, addr: usize) {
        self.reservations[hart.0 as usize] = Some(addr);
    }

    /// Read `hart`'s current reservation (if any).
    pub fn reservation(&self, hart: HartId) -> Option<usize> {
        self.reservations[hart.0 as usize]
    }

    /// Clear `hart`'s reservation.
    pub fn clear_reservation(&mut self, hart: HartId) {
        self.reservations[hart.0 as usize] = None;
    }

    /// Clear every hart's reservation (used on CPU reset).
    pub fn clear_reservations(&mut self) {
        self.reservations.fill(None);
    }

    /// Physical-store chokepoint: write to RAM/MMIO and invalidate any
    /// peer-hart LR/SC reservation that overlaps the touched bytes within
    /// the reservation granule (RISC-V A-extension §8.2).
    pub fn store(&mut self, hart: HartId, addr: usize, size: usize, val: Word) -> XResult {
        self.write(addr, size, val)?;
        self.invalidate_peer_reservations(hart, addr, size);
        Ok(())
    }

    fn invalidate_peer_reservations(&mut self, src: HartId, addr: usize, size: usize) {
        let end = addr.saturating_add(size);
        let src_idx = src.0 as usize;
        for (i, slot) in self.reservations.iter_mut().enumerate() {
            if i == src_idx {
                continue;
            }
            if let Some(a) = *slot
                && granule_overlaps(a, addr, end)
            {
                *slot = None;
            }
        }
    }

    /// Register an MMIO device at the given address range. Returns the
    /// slot index — useful for pinning the region as timer or IRQ sink.
    pub fn add_mmio(
        &mut self,
        name: &'static str,
        base: usize,
        size: usize,
        dev: Box<dyn Device>,
    ) -> usize {
        assert!(size > 0, "region size must be non-zero");
        let range = base..base.checked_add(size).expect("address overflow");
        assert!(
            !overlaps(&range, self.ram.range()),
            "MMIO '{name}' overlaps RAM"
        );
        if let Some(r) = self.mmio.iter().find(|r| overlaps(&range, &r.range)) {
            panic!("MMIO '{name}' overlaps '{}'", r.name);
        }
        info!("bus: add_mmio '{}' base={:#x} size={:#x}", name, base, size);
        let idx = self.mmio.len();
        self.mmio.push(MmioRegion { name, range, dev });
        idx
    }

    /// Swap the device backing a named MMIO region.
    pub fn replace_device(&mut self, name: &str, dev: Box<dyn Device>) {
        self.mmio
            .iter_mut()
            .find(|r| r.name == name)
            .unwrap_or_else(|| panic!("bus: no device named '{name}'"))
            .dev = dev;
    }

    /// Designate a device as the timer source (MTIMER).
    pub fn set_timer_source(&mut self, idx: usize) {
        self.mtimer_idx = Some(idx);
    }

    /// Designate a device as the interrupt controller (PLIC).
    pub fn set_irq_sink(&mut self, idx: usize) {
        self.plic_idx = Some(idx);
    }

    /// Read mtime directly from MTIMER (avoids MMIO dispatch).
    #[inline]
    pub fn mtime(&self) -> u64 {
        self.mtimer_idx
            .and_then(|i| self.mmio[i].dev.mtime())
            .unwrap_or(0)
    }

    /// Tick devices. MTIMER ticks every step; slow devices (UART, PLIC)
    /// tick every `SLOW_TICK_DIVISOR` steps to reduce overhead.
    ///
    /// PLIC-last ordering (directIrq I-D16): every non-PLIC device ticks
    /// before the PLIC so a raise produced inside a device's own `tick` is
    /// observed in the same slow-tick drain.
    pub fn tick(&mut self) {
        if let Some(i) = self.mtimer_idx {
            self.mmio[i].dev.tick();
        }
        self.tick_count += 1;
        if !self.tick_count.is_multiple_of(SLOW_TICK_DIVISOR) {
            return;
        }
        for (idx, r) in self.mmio.iter_mut().enumerate() {
            if Some(idx) != self.mtimer_idx && Some(idx) != self.plic_idx {
                r.dev.tick();
            }
        }
        if let Some(i) = self.plic_idx {
            self.mmio[i].dev.tick();
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
        Bus::new(CONFIG_MBASE, CONFIG_MSIZE, 1)
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

    #[test]
    fn irq_sink_set_explicitly() {
        let mut bus = new_bus();
        assert!(bus.plic_idx.is_none());
        bus.add_mmio("plic", MMIO_BASE, MMIO_SIZE, stub());
        bus.set_irq_sink(0);
        assert_eq!(bus.plic_idx, Some(0));
    }

    #[test]
    fn replace_device_swaps_named_region() {
        let mut bus = new_bus();
        bus.add_mmio("stub", MMIO_BASE, MMIO_SIZE, stub());
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

    // ---- Multi-hart Bus state (PR1) ----

    fn new_bus_two_harts() -> Bus {
        Bus::new(CONFIG_MBASE, CONFIG_MSIZE, 2)
    }

    #[test]
    fn bus_new_two_harts_vec_lengths() {
        let bus = new_bus_two_harts();
        assert_eq!(bus.num_harts(), 2);
        assert_eq!(bus.reservations.len(), 2);
    }

    #[test]
    fn bus_store_invalidates_peer_reservation_in_granule() {
        let mut bus = new_bus_two_harts();
        bus.reserve(HartId(0), CONFIG_MBASE + 0x1000);
        // Hart 1 stores within the same 8-byte granule.
        bus.store(HartId(1), CONFIG_MBASE + 0x1004, 4, 0xDEAD)
            .unwrap();
        assert_eq!(bus.reservation(HartId(0)), None);
    }

    #[test]
    fn bus_store_preserves_peer_reservation_outside_granule() {
        let mut bus = new_bus_two_harts();
        bus.reserve(HartId(0), CONFIG_MBASE + 0x1000);
        // Hart 1 stores past the 8-byte granule.
        bus.store(HartId(1), CONFIG_MBASE + 0x1010, 4, 0xBEEF)
            .unwrap();
        assert_eq!(bus.reservation(HartId(0)), Some(CONFIG_MBASE + 0x1000));
    }

    #[test]
    fn bus_store_preserves_same_hart_reservation() {
        let mut bus = new_bus_two_harts();
        bus.reserve(HartId(0), CONFIG_MBASE + 0x1000);
        // Same hart store: reservation untouched by the chokepoint
        // (sc/lr/amo logic clears it explicitly when needed).
        bus.store(HartId(0), CONFIG_MBASE + 0x1000, 4, 0xCAFE)
            .unwrap();
        assert_eq!(bus.reservation(HartId(0)), Some(CONFIG_MBASE + 0x1000));
    }
}
