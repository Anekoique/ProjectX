use std::ops::Range;

use super::Device;
use crate::{
    config::Word,
    error::{XError, XResult},
};

pub struct Ram {
    range: Range<usize>,
    data: Vec<u8>,
}

impl Ram {
    pub fn new(base: usize, size: usize) -> Self {
        Self {
            range: base..base.checked_add(size).expect("RAM range overflow"),
            data: vec![0; size],
        }
    }

    pub fn range(&self) -> &Range<usize> {
        &self.range
    }
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Little-endian read. Pure `&self` — used by both `Device::read` and
    /// `Bus::read_ram`.
    pub(crate) fn get(&self, offset: usize, size: usize) -> XResult<Word> {
        let end = self.check_bounds(offset, size)?;
        let mut buf = [0u8; std::mem::size_of::<Word>()];
        buf[..size].copy_from_slice(&self.data[offset..end]);
        Ok(Word::from_le_bytes(buf))
    }

    pub fn load(&mut self, offset: usize, data: &[u8]) -> XResult {
        let end = offset
            .checked_add(data.len())
            .filter(|&e| e <= self.data.len())
            .ok_or(XError::BadAddress)?;
        self.data[offset..end].copy_from_slice(data);
        Ok(())
    }

    fn check_bounds(&self, offset: usize, size: usize) -> XResult<usize> {
        offset
            .checked_add(size)
            .filter(|&e| e <= self.data.len() && size <= std::mem::size_of::<Word>())
            .ok_or(XError::BadAddress)
    }
}

impl Device for Ram {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word> {
        self.get(offset, size)
    }

    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult {
        let end = self.check_bounds(offset, size)?;
        self.data[offset..end].copy_from_slice(&value.to_le_bytes()[..size]);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_all_sizes() {
        let mut ram = Ram::new(0, 64);
        for &(off, sz, val) in &[(0, 1, 0xAB), (2, 2, 0xBEEF), (4, 4, 0xDEADBEEF)] {
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
