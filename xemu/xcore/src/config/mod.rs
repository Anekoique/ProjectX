//! Platform configuration: memory layout, word-width types, and shift helpers.

/// Physical RAM base address.
pub const CONFIG_MBASE: usize = 0x8000_0000;
/// Default physical RAM size (128 MB).
pub const CONFIG_MSIZE: usize = 0x0800_0000;

/// Machine configuration — independent inputs driving bus, devices, and boot.
pub struct MachineConfig {
    /// Physical RAM size, in bytes.
    pub ram_size: usize,
    /// Optional pre-loaded VirtIO block disk image.
    pub disk: Option<Vec<u8>>,
    /// Number of harts to instantiate. Default 1; valid range `1..=16`.
    pub num_harts: usize,
}

impl Default for MachineConfig {
    fn default() -> Self {
        Self {
            ram_size: CONFIG_MSIZE,
            disk: None,
            num_harts: 1,
        }
    }
}

impl MachineConfig {
    /// Disk profile: 1 GB RAM + disk image.
    pub fn with_disk(disk: Vec<u8>) -> Self {
        Self {
            ram_size: 0x4000_0000,
            disk: Some(disk),
            num_harts: 1,
        }
    }

    /// Override `num_harts` (builder-style). Must be in `1..=16`.
    pub fn with_harts(mut self, n: usize) -> Self {
        debug_assert!((1..=16).contains(&n), "num_harts must be in [1, 16]");
        self.num_harts = n;
        self
    }

    /// FDT load address: 1 MB below top of RAM.
    pub fn fdt_addr(&self) -> usize {
        CONFIG_MBASE + self.ram_size - 0x10_0000
    }
}

#[cfg(test)]
mod machine_config_tests {
    use super::*;

    #[test]
    fn machine_config_default_num_harts_is_one() {
        assert_eq!(MachineConfig::default().num_harts, 1);
    }
}

/// Boot layout persisted in CPU across resets.
#[derive(Clone, Copy, Debug)]
pub struct BootLayout {
    pub fdt_addr: usize,
}

/// Unsigned machine word (u64 on RV64, u32 on RV32).
#[cfg(isa64)]
pub type Word = u64;
/// Unsigned machine word (u64 on RV64, u32 on RV32).
#[cfg(isa32)]
pub type Word = u32;

/// Signed machine word (i64 on RV64, i32 on RV32).
#[cfg(isa64)]
pub type SWord = i64;
/// Signed machine word (i64 on RV64, i32 on RV32).
#[cfg(isa32)]
pub type SWord = i32;

/// Mask for valid shift amounts (Word::BITS - 1).
pub const SHAMT_MASK: Word = (Word::BITS as Word) - 1;

/// Truncate a [`Word`] to `u32`.
#[inline(always)]
#[allow(clippy::unnecessary_cast, dead_code)]
pub fn word_to_u32(value: Word) -> u32 {
    value as u32
}

/// Extract the shift amount from a [`Word`], masked to valid range.
#[inline(always)]
#[allow(clippy::unnecessary_cast)]
pub fn word_to_shamt(value: Word) -> u32 {
    (value & SHAMT_MASK) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn word_to_u32_truncates() {
        assert_eq!(word_to_u32(0), 0u32);
        assert_eq!(word_to_u32(0xDEADBEEF as Word), 0xDEADBEEF_u32);
        #[cfg(isa64)]
        assert_eq!(word_to_u32(0x1234_5678_DEAD_BEEF_u64 as Word), 0xDEAD_BEEF);
    }

    #[test]
    fn word_to_shamt_masks_shift_amount() {
        let cases = [(0, 0), (3, 3), ((Word::BITS as Word) + 5, 5)];
        for (input, expected) in cases {
            assert_eq!(word_to_shamt(input), expected);
        }
    }
}
