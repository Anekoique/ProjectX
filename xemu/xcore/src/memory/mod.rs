use std::sync::{LazyLock, Mutex};

use memory_addr::{MemoryAddr, PhysAddr};

use crate::{
    config::{CONFIG_MBASE, CONFIG_MSIZE, Word},
    ensure,
    error::{XError, XResult},
};

pub static MEMORY: LazyLock<Mutex<Memory>> = LazyLock::new(|| Mutex::new(Memory::new()));

macro_rules! with_mem {
    ($method:ident($($arg:expr),* $(,)?)) => {{
        $crate::MEMORY.lock()
            .expect("Poisoned lock on MEMORY mutex")
            .$method($($arg),*)
    }};
}
pub(crate) use with_mem;

#[derive(Debug, Default)]
pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            data: vec![0; CONFIG_MSIZE],
        }
    }

    fn access(&self, addr: PhysAddr, size: usize) -> XResult<usize> {
        let addr = addr.as_usize();
        ensure!(addr >= CONFIG_MBASE, Err(XError::BadAddress));
        let offset = addr - CONFIG_MBASE;
        ensure!(offset + size <= CONFIG_MSIZE, Err(XError::BadAddress));
        Ok(offset)
    }

    fn read_at(&self, offset: usize, size: usize) -> Word {
        let mut buf = [0u8; std::mem::size_of::<Word>()];
        buf[..size].copy_from_slice(&self.data[offset..offset + size]);
        Word::from_le_bytes(buf)
    }

    pub fn read(&self, addr: PhysAddr, size: usize) -> XResult<Word> {
        ensure!(
            [1, 2, 4, 8].contains(&size) && addr.is_aligned(size),
            Err(XError::AddrNotAligned)
        );
        self.access(addr, size).map(|off| self.read_at(off, size))
    }

    /// Read with relaxed alignment (IALIGN=16: 2-byte aligned).
    /// Used for instruction fetch where 32-bit instructions may start at
    /// non-4-aligned addresses.
    pub fn fetch_u32(&self, addr: PhysAddr, size: usize) -> XResult<Word> {
        ensure!(addr.is_aligned(2_usize), Err(XError::AddrNotAligned));
        self.access(addr, size).map(|off| self.read_at(off, size))
    }

    pub fn write(&mut self, addr: PhysAddr, size: usize, value: Word) -> XResult {
        ensure!(
            [1, 2, 4, 8].contains(&size) && addr.is_aligned(size),
            Err(XError::AddrNotAligned)
        );
        self.access(addr, size).map(|offset| {
            let bytes = value.to_le_bytes();
            self.data[offset..offset + size].copy_from_slice(&bytes[..size]);
        })
    }

    pub fn load(&mut self, addr: PhysAddr, data: &[u8]) -> XResult {
        let size = data.len();
        self.access(addr, size).map(|offset| {
            self.data[offset..offset + size].copy_from_slice(data);
        })
    }
}

#[cfg(test)]
mod tests {
    use memory_addr::pa;

    use super::*;

    fn mbase() -> PhysAddr {
        pa!(crate::config::CONFIG_MBASE)
    }

    #[test]
    #[cfg(any(isa32, isa64))]
    fn test_memory_read_write_common() {
        let base = mbase();
        let mut mem = Memory::new();
        let cases = [(0usize, 1usize, 0xFF), (2, 2, 0xBEEF), (4, 4, 0xDEADBEEF)];

        for (offset, size, value) in cases {
            assert!(
                mem.write(base + offset, size, value).is_ok(),
                "write failed for size {size}"
            );
            assert_eq!(
                mem.read(base + offset, size).unwrap(),
                value,
                "readback mismatch for size {size}"
            );
        }
    }

    #[test]
    #[cfg(isa64)]
    fn test_memory_read_write_64bit() {
        let base = mbase();
        let mut mem = Memory::new();

        assert!(mem.write(base + 8, 8, 0xCAFEBABEDEADBEEF).is_ok());
        assert_eq!(mem.read(base + 8, 8).unwrap(), 0xCAFEBABEDEADBEEF);
    }

    #[test]
    fn test_memory_rejects_invalid_read_size_and_alignment() {
        let mem = Memory::new();
        let base = mbase();

        for size in [3, 16] {
            assert!(matches!(mem.read(base, size), Err(XError::AddrNotAligned)));
        }

        assert!(matches!(mem.read(base + 1, 2), Err(XError::AddrNotAligned)));
        assert!(matches!(
            mem.fetch_u32(base + 1, 4),
            Err(XError::AddrNotAligned)
        ));
    }

    #[test]
    fn test_memory_rejects_invalid_write_size_and_alignment() {
        let mut mem = Memory::new();
        let base = mbase();

        for size in [3, 16] {
            assert!(matches!(
                mem.write(base, size, 0),
                Err(XError::AddrNotAligned)
            ));
        }

        assert!(matches!(
            mem.write(base + 1, 2, 0xBEEF),
            Err(XError::AddrNotAligned)
        ));
    }

    #[test]
    fn test_memory_rejects_out_of_bounds_accesses() {
        let mut mem = Memory::new();
        let base = mbase();
        let msize = crate::config::CONFIG_MSIZE;
        let out_of_bounds = base + msize;
        // 4-byte aligned but straddles the end (offset + 4 > MSIZE)
        let near_end_aligned = base + msize - 4 + 4; // == out_of_bounds
        // 2-byte aligned for fetch_u32 (offset + 4 > MSIZE)
        let near_end_half = base + msize - 2;

        assert!(matches!(
            mem.read(out_of_bounds, 1),
            Err(XError::BadAddress)
        ));
        assert!(matches!(
            mem.write(near_end_aligned, 4, 0xAABBCCDD),
            Err(XError::BadAddress)
        ));
        assert!(matches!(
            mem.fetch_u32(near_end_half, 4),
            Err(XError::BadAddress)
        ));
        assert!(matches!(
            mem.load(near_end_half, &[1, 2, 3, 4]),
            Err(XError::BadAddress)
        ));
    }

    #[test]
    fn test_fetch_u32_relaxed_alignment() {
        let mut mem = Memory::new();
        let base = mbase();

        // Write at 4-aligned address, fetch from 2-aligned offset
        mem.write(base, 4, 0xDEADBEEF).unwrap();
        // fetch_u32 from base (4-aligned) should work
        assert_eq!(mem.fetch_u32(base, 4).unwrap() as u32, 0xDEADBEEF_u32);

        // Write data and fetch from 2-byte-aligned but not 4-byte-aligned address
        mem.write(base + 4, 4, 0xCAFEBABE).unwrap();
        // fetch_u32 at base+2 should succeed (2-byte aligned)
        let val = mem.fetch_u32(base + 2, 4).unwrap();
        // Bytes at offset 2..6: [0xBE, 0xDE, 0xBE, 0xBA] (little-endian cross-word
        // read)
        assert_eq!(val as u32, 0xBABEDEAD_u32);
    }

    #[test]
    fn test_load_bulk_data() {
        let mut mem = Memory::new();
        let base = mbase();
        let data = [0x11u8, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88];
        mem.load(base, &data).unwrap();

        assert_eq!(mem.read(base, 1).unwrap() as u8, 0x11);
        assert_eq!(mem.read(base + 2, 2).unwrap() as u16, 0x4433);
        assert_eq!(mem.read(base + 4, 4).unwrap() as u32, 0x88776655);
    }

    #[test]
    fn test_load_out_of_bounds() {
        let mut mem = Memory::new();
        let addr = mbase() + crate::config::CONFIG_MSIZE - 2;
        let data = [0u8; 4];
        assert!(matches!(mem.load(addr, &data), Err(XError::BadAddress)));
    }

    #[test]
    fn test_write_read_preserves_little_endian() {
        let mut mem = Memory::new();
        let base = mbase();
        mem.write(base, 4, 0x04030201).unwrap();
        // Read individual bytes to verify LE layout
        assert_eq!(mem.read(base, 1).unwrap(), 0x01);
        assert_eq!(mem.read(base + 1, 1).unwrap(), 0x02);
        assert_eq!(mem.read(base + 2, 1).unwrap(), 0x03);
        assert_eq!(mem.read(base + 3, 1).unwrap(), 0x04);
    }
}
