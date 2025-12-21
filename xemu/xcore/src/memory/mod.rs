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
        let offset = addr.as_usize() - CONFIG_MBASE;
        ensure!(offset + size <= CONFIG_MSIZE, Err(XError::BadAddress));
        Ok(offset)
    }

    fn check_aligned(&self, addr: PhysAddr, size: usize) -> XResult<()> {
        ensure!(
            [1, 2, 4, 8].contains(&size) && addr.is_aligned(size),
            Err(XError::AddrNotAligned)
        );
        Ok(())
    }

    pub fn read(&self, addr: PhysAddr, size: usize) -> XResult<Word> {
        self.check_aligned(addr, size)
            .and_then(|_| self.access(addr, size))
            .map(|offset| unsafe {
                let mut value: Word = 0;
                std::ptr::copy_nonoverlapping(
                    self.data.as_ptr().add(offset),
                    &mut value as *mut _ as *mut u8,
                    size,
                );
                value
            })
    }

    pub fn write(&mut self, addr: PhysAddr, size: usize, value: Word) -> XResult {
        self.access(addr, size).map(|offset| unsafe {
            std::ptr::copy_nonoverlapping(
                &value as *const _ as *const u8,
                self.data.as_mut_ptr().add(offset),
                size,
            )
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

    // Helper function to get the memory base address.
    fn mbase() -> PhysAddr {
        pa!(crate::config::CONFIG_MBASE)
    }

    #[test]
    #[cfg(any(isa32, isa64))]
    fn test_memory_read_write_common() {
        let base = mbase();
        let mut mem = Memory::new();

        // Test 1, 2, and 4-byte reads/writes which are common to both isa32 and isa64.
        assert!(mem.write(base, 1, 0xFF).is_ok());
        assert_eq!(mem.read(base, 1).unwrap(), 0xFF);

        assert!(mem.write(base + 2, 2, 0xBEEF).is_ok());
        assert_eq!(mem.read(base + 2, 2).unwrap(), 0xBEEF);

        assert!(mem.write(base + 4, 4, 0xDEADBEEF).is_ok());
        assert_eq!(mem.read(base + 4, 4).unwrap(), 0xDEADBEEF);
    }

    #[test]
    #[cfg(isa64)]
    fn test_memory_read_write_64bit() {
        let base = mbase();
        let mut mem = Memory::new();

        // Test 8-byte read/write specific to isa64.
        assert!(mem.write(base + 8, 8, 0xCAFEBABEDEADBEEF).is_ok());
        assert_eq!(mem.read(base + 8, 8).unwrap(), 0xCAFEBABEDEADBEEF);
    }

    #[test]
    fn test_memory_bounds() {
        let mem = Memory::new();

        // Test reading from an out-of-bounds address.
        let out_of_bounds = mbase() + crate::config::CONFIG_MSIZE;
        assert!(matches!(
            mem.read(out_of_bounds, 1),
            Err(XError::BadAddress)
        ));

        // Test reading where the access would go out of bounds.
        let near_end = mbase() + crate::config::CONFIG_MSIZE - 4;
        assert!(matches!(mem.read(near_end, 8), Err(XError::AddrNotAligned)));

        // Test reading with an unsupported access size.
        let addr = mbase();
        assert!(matches!(mem.read(addr, 3), Err(XError::AddrNotAligned)));
        assert!(matches!(mem.read(addr, 16), Err(XError::AddrNotAligned)));
    }
}
