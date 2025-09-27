use std::sync::{LazyLock, RwLock};

use memory_addr::{MemoryAddr, PhysAddr};

use crate::{
    config::Word,
    ensure,
    error::{XError, XResult},
};

static MEMORY: LazyLock<RwLock<Memory>> = LazyLock::new(|| RwLock::new(Memory::new()));

pub struct Memory {
    data: Vec<u8>,
}

impl Memory {
    pub fn new() -> Self {
        Self {
            data: vec![0; crate::config::CONFIG_MSIZE],
        }
    }

    fn access(&self, addr: PhysAddr, size: usize) -> XResult<usize> {
        let offset = addr.as_usize() - crate::config::CONFIG_MBASE;
        ensure!(
            offset + size <= crate::config::CONFIG_MSIZE
                && [1, 2, 4, 8].contains(&size)
                && addr.is_aligned(size),
            Err(XError::BadAddress)
        );
        Ok(offset)
    }

    pub fn read(&self, addr: PhysAddr, size: usize) -> XResult<Word> {
        self.access(addr, size).map(|offset| unsafe {
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
}

pub fn pmem_read(addr: PhysAddr, size: usize) -> XResult<Word> {
    MEMORY
        .read()
        .map_err(|_| panic!("Failed to acquire lock"))?
        .read(addr, size)
}

pub fn pmem_write(addr: PhysAddr, size: usize, value: Word) -> XResult {
    MEMORY
        .write()
        .map_err(|_| panic!("Failed to acquire lock"))?
        .write(addr, size, value)
}

#[cfg(test)]
mod tests {
    use memory_addr::pa;

    use super::*;

    #[test]
    fn test_memory_basic() {
        let base = pa!(crate::config::CONFIG_MBASE);
        let mut mem = MEMORY.write().unwrap();
        assert!(mem.write(base, 1, 0xFF).is_ok());
        assert!(
            mem.write(pa!(crate::config::CONFIG_MBASE + 2), 2, 0xBEEF)
                .is_ok()
        );
        assert!(
            mem.write(pa!(crate::config::CONFIG_MBASE + 4), 4, 0xDEADBEEF)
                .is_ok()
        );
        assert!(
            mem.write(pa!(crate::config::CONFIG_MBASE + 8), 8, 0xCAFEBABEDEADBEEF)
                .is_ok()
        );
        assert_eq!(mem.read(base, 1).unwrap(), 0xFF);
        assert_eq!(
            mem.read(pa!(crate::config::CONFIG_MBASE + 2), 2).unwrap(),
            0xBEEF
        );
        assert_eq!(
            mem.read(pa!(crate::config::CONFIG_MBASE + 4), 4).unwrap(),
            0xDEADBEEF
        );
        assert_eq!(
            mem.read(pa!(crate::config::CONFIG_MBASE + 8), 8).unwrap(),
            0xCAFEBABEDEADBEEF
        );
        drop(mem);

        let addr = pa!(crate::config::CONFIG_MBASE + 0x100);
        assert!(pmem_write(addr, 4, 0xABCDEF00).is_ok());
        assert_eq!(pmem_read(addr, 4).unwrap(), 0xABCDEF00);
    }

    #[test]
    fn test_memory_bounds() {
        let mem = MEMORY.read().unwrap();

        let out_of_bounds = pa!(crate::config::CONFIG_MBASE + crate::config::CONFIG_MSIZE);
        assert!(matches!(
            mem.read(out_of_bounds, 1),
            Err(XError::BadAddress)
        ));

        let near_end = pa!(crate::config::CONFIG_MBASE + crate::config::CONFIG_MSIZE - 4);
        assert!(matches!(mem.read(near_end, 8), Err(XError::BadAddress)));

        let addr = pa!(crate::config::CONFIG_MBASE);
        assert!(matches!(mem.read(addr, 3), Err(XError::BadAddress)));
        assert!(matches!(mem.read(addr, 16), Err(XError::BadAddress)));
    }
}
