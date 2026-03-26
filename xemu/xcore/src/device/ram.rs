use std::ops::Range;

use super::Device;
use crate::{config::Word, error::XResult};

pub struct Ram {
    range: Range<usize>,
    data: Vec<u8>,
}

impl Device for Ram {
    /// Raw little-endian read. No alignment checks.
    fn read(&self, offset: usize, size: usize) -> XResult<Word> {
        let end = offset
            .checked_add(size)
            .filter(|&e| e <= self.data.len() && size <= std::mem::size_of::<Word>())
            .ok_or(crate::error::XError::BadAddress)?;
        let mut buf = [0u8; std::mem::size_of::<Word>()];
        buf[..size].copy_from_slice(&self.data[offset..end]);
        Ok(Word::from_le_bytes(buf))
    }

    /// Raw little-endian write. No alignment checks.
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult {
        let end = offset
            .checked_add(size)
            .filter(|&e| e <= self.data.len() && size <= std::mem::size_of::<Word>())
            .ok_or(crate::error::XError::BadAddress)?;
        self.data[offset..end].copy_from_slice(&value.to_le_bytes()[..size]);
        Ok(())
    }
}

impl Ram {
    pub fn new(ram_base: usize, size: usize) -> Self {
        Self {
            range: ram_base..ram_base + size,
            data: vec![0; size],
        }
    }

    pub fn range(&self) -> &Range<usize> {
        &self.range
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Bulk byte copy for image loading.
    pub fn load(&mut self, offset: usize, data: &[u8]) -> XResult {
        let end = offset
            .checked_add(data.len())
            .filter(|&e| e <= self.data.len())
            .ok_or(crate::error::XError::BadAddress)?;
        self.data[offset..end].copy_from_slice(data);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_sizes() {
        let mut ram = Ram::new(0, 64);
        let cases: &[(usize, usize, Word)] = &[(0, 1, 0xAB), (2, 2, 0xBEEF), (4, 4, 0xDEADBEEF)];
        for &(off, sz, val) in cases {
            ram.write(off, sz, val).unwrap();
            assert_eq!(ram.read(off, sz).unwrap(), val);
        }
    }

    #[cfg(isa64)]
    #[test]
    fn roundtrip_8byte() {
        let mut ram = Ram::new(0, 64);
        ram.write(8, 8, 0xCAFEBABE_DEADBEEF).unwrap();
        assert_eq!(ram.read(8, 8).unwrap(), 0xCAFEBABE_DEADBEEF);
    }

    #[test]
    fn load_bulk_data() {
        let mut ram = Ram::new(0, 64);
        ram.load(0, &[0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77, 0x88])
            .unwrap();
        assert_eq!(ram.read(0, 1).unwrap() as u8, 0x11);
        assert_eq!(ram.read(2, 2).unwrap() as u16, 0x4433);
        assert_eq!(ram.read(4, 4).unwrap() as u32, 0x88776655);
    }

    #[test]
    fn little_endian_layout() {
        let mut ram = Ram::new(0, 64);
        ram.write(0, 4, 0x04030201).unwrap();
        for (i, expected) in [0x01, 0x02, 0x03, 0x04].iter().enumerate() {
            assert_eq!(ram.read(i, 1).unwrap(), *expected);
        }
    }
}
