//! Per-hart decoded-instruction cache (Phase P4 of `docs/archived/perf/perfHotPath/`).
//!
//! Memoises the pest-based decoder walk; the post-P1 profile in
//! `docs/perf/baselines/2026-04-15/REPORT.md` pinned `xdb::main` as 40–47 % of
//! self-time, with decode as its biggest sub-cost.
//!
//! # Invariants
//!
//! - **I-11.** Geometry is per-hart, direct-mapped, 4096 lines.
//! - **I-12.** A line is a hit iff `line.pc == pc && line.raw == raw`; any
//!   mismatch re-decodes. The key is `(pc, raw)` only — decode is a pure
//!   function of `raw` and static tables, so the raw word alone carries all
//!   information decode needs.
//!
//! # Self-modifying code
//!
//! Self-healing: a guest store that rewrites an instruction changes
//! `raw` at the target PC, the cache comparison misses, and the line is
//! overwritten with the freshly decoded new instruction. `fence.i` is a
//! NOP in the emulator; the `(pc, raw)` key is the only coherence
//! mechanism needed.
//!
//! # Size
//!
//! 4096 lines mirrors QEMU's per-CPU jump cache (`TB_JMP_CACHE_BITS =
//! 12`). Dhrystone/CoreMark/microbench working sets are <= 1.5 K static
//! instructions, so aliasing is negligible.

use memory_addr::VirtAddr;

use crate::isa::{DecodedInst, InstKind};

/// Log2 of the number of cache lines. 12 bits → 4096 lines.
pub const ICACHE_BITS: usize = 12;
/// Number of cache lines in one [`ICache`] (direct-mapped).
pub const ICACHE_LINES: usize = 1 << ICACHE_BITS;
/// Bit mask for the index function (`pc >> 1`).
pub const ICACHE_MASK: usize = ICACHE_LINES - 1;

/// One decoded-instruction cache slot.
#[derive(Clone, Copy)]
pub struct ICacheLine {
    /// Guest virtual address the line decodes. Part of the lookup key.
    pub pc: VirtAddr,
    /// Raw instruction word captured at fill time. The second half of
    /// the lookup key — a mismatch means SMC or aliasing, either way a
    /// miss.
    pub raw: u32,
    /// Decoded form of `raw`. Valid only when the `(pc, raw)` pair
    /// matches the fetch.
    pub decoded: DecodedInst,
}

impl ICacheLine {
    /// Sentinel line for an empty cache. `pc = 0` can never match a
    /// real fetch (the reset vector is `0x8000_0000`), so every real
    /// lookup misses on a fresh cache.
    pub const INVALID: Self = Self {
        pc: VirtAddr::from_usize(0),
        raw: 0,
        decoded: DecodedInst::C {
            kind: InstKind::c_nop,
            inst: 0,
        },
    };
}

/// Per-hart decoded-instruction cache.
///
/// The inner `Box` owns the 256 KB line array on the heap; the outer
/// `Box<Self>` in `RVCore` keeps the struct itself out of the core
/// state. Avoids the stack-sized `[T; 4096]` literal that `Box::new(Self {
/// lines: [...] })` would construct on stable rustc.
pub struct ICache {
    pub lines: Box<[ICacheLine; ICACHE_LINES]>,
}

impl ICache {
    /// Build a fresh cache with every line invalid.
    pub fn new() -> Box<Self> {
        Box::new(Self {
            lines: Box::new([ICacheLine::INVALID; ICACHE_LINES]),
        })
    }

    /// Direct-mapped index function. `pc >> 1` so 2-byte (compressed)
    /// and 4-byte alignments both spread across the table.
    #[inline]
    pub fn index(pc: VirtAddr) -> usize {
        (pc.as_usize() >> 1) & ICACHE_MASK
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// V-UT-1 — `DecodedInst: Copy` is load-bearing for memcpy-safe
    /// cache fills; regressions stop compiling here.
    #[test]
    fn decoded_inst_is_copy() {
        fn _assert_copy<T: Copy>() {}
        _assert_copy::<DecodedInst>();
        _assert_copy::<ICacheLine>();
    }

    /// V-UT-2 — a fresh cache always misses at real guest PCs.
    #[test]
    fn invalid_line_never_matches_real_pc() {
        let cache = ICache::new();
        let real_pc = VirtAddr::from_usize(0x8000_0000);
        let line = &cache.lines[ICache::index(real_pc)];
        assert_ne!(line.pc, real_pc);
    }

    /// PCs separated by exactly `ICACHE_LINES << 1` alias into the
    /// same slot; the `pc` half of the key is what distinguishes them
    /// in the call-site compare.
    #[test]
    fn index_masks_low_bits_above_12() {
        let a = VirtAddr::from_usize(0x8000_0000);
        let b = VirtAddr::from_usize(0x8000_0000 + (ICACHE_LINES << 1));
        assert_eq!(ICache::index(a), ICache::index(b));
        assert_ne!(a, b);
    }
}
