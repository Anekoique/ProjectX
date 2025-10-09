use crate::config::{SWord, Word};

#[inline(always)]
pub fn bit_slice_u32(value: u32, hi: u8, lo: u8) -> u32 {
    debug_assert!(hi < 32 && lo <= hi);
    (value >> lo) & ((1u32 << (hi - lo + 1)) - 1)
}

#[inline(always)]
pub fn sign_extend_u32(value: u32, bits: u8) -> i32 {
    let shift = 32 - bits;
    ((value << shift) as i32) >> shift
}

#[inline(always)]
pub fn sign_extend_word(value: Word, bits: u32) -> Word {
    let shift = Word::BITS - bits;
    (((value << shift) as SWord) >> shift) as Word
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bit_slice_extracts_expected_range() {
        let value = 0b1011_1100u32;
        assert_eq!(bit_slice_u32(value, 5, 2), 0b1111);
    }

    #[test]
    fn sign_extend_helpers_match_twos_complement_rules() {
        assert_eq!(sign_extend_u32(0x7F, 8), 0x7F);
        assert_eq!(sign_extend_u32(0x80, 8), -128);

        let negative = sign_extend_word(0x80, 8);
        assert_eq!(negative, (-128 as SWord) as Word);
        assert_eq!(sign_extend_word(0x7F, 8), 0x7F);
    }
}
