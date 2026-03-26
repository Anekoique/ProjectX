# MEM Implementation Fix Report

> Fixes for issues identified in [MEM_IMPL_REVIEW_5_COMMITS_2026-03-26.md](./MEM_IMPL_REVIEW_5_COMMITS_2026-03-26.md)
> Verified against RISC-V Privileged Specification §3.1.6.3, §3.7.1, §4.3.1, §8.2.
>
> **Remaining known issues** (from review v5):
> - Svade is implemented but not formally exposed as a declared extension capability

## Fix Summary

| # | Severity | Issue | Fix | File(s) |
|---|----------|-------|-----|---------|
| 1 | HIGH | SUM allows S-mode fetch from U=1 pages | SUM only applies to data access, Fetch always denied on U=1 for S-mode | mmu.rs |
| 2 | HIGH | Final PMP check uses original privilege, ignoring MPRV | PMP check uses `priv_mode` (effective) for all accesses | mem.rs |
| 3 | HIGH | RV64 pmpcfg1/3 accepted, entry mapping wrong | cfg-gated: RV64 only processes pmpcfg0→entries 0..7, pmpcfg2→entries 8..15 | csr/ops.rs |
| 4 | HIGH | Locked PMP entries can be overwritten | `update_cfg`/`update_addr` check L bit; TOR locks pmpaddr[i-1] | pmp.rs |
| 5 | HIGH | LR translates as Amo, gets store-class faults | LR uses `MemOp::Load`, SC uses `MemOp::Store` | atomic.rs |
| 6 | MEDIUM | reset() doesn't clear MMU/PMP state | Reset reinitializes `mmu` and `pmp` to defaults | mod.rs |
| 7 | MEDIUM | PMP only checks start address, not full range | `Pmp::check` takes `size`, verifies `[paddr, paddr+size)` fully contained | pmp.rs |
| 8 | MEDIUM | Bus unchecked arithmetic overflow | `ram_offset`/`find_mmio` use `checked_add` | bus.rs |
| 9 | MEDIUM | Ram panics on out-of-bounds | `read`/`write`/`load` return `Err(BadAddress)` on overflow or OOB | ram.rs |
| 10 | HIGH | RV64 pmpcfg1/3 accepted as valid CSR | `is_illegal_csr` rejects odd pmpcfg on RV64 | csr/ops.rs |
| 11 | HIGH | Locked PMP write changes CSR readback | `pmp_writeback_csr` restores CSR from actual Pmp state after side effects | csr/ops.rs, pmp.rs |
| 12 | HIGH | Final PMP only checks 1 byte | `translate` takes `size`, passes to `pmp.check` | mem.rs, atomic.rs |
| 13 | HIGH | Page walk ignores reserved PTE bits | Check non-leaf D/A/U and high reserved bits | mmu.rs |
| 14 | HIGH | Fetch cross-page: 4-byte read misses second page translation | Split into two 2-byte parcel reads with independent translate | mem.rs |
| 15 | HIGH | RV64 PTE PPN width wrong (uses levels*vpn_bits+2 instead of 44) | `SvMode.ppn_bits` field, Sv39/48/57 all use 44-bit PPN | mmu.rs |
| 16 | HIGH | Svade not explicitly modeled | Documented as Svade policy with comment | mmu.rs |
| 17 | MEDIUM | Fix report doc stale | Updated reference and added remaining issues section | MEM_IMPL_FIX_REPORT.md |
| 18 | HIGH | PMP partial-overlap not handled | `overlap()` returns tri-state; partial → immediate fail | pmp.rs |
| 19 | MEDIUM | Clippy `as u32` warning on RV32 | Mask `& 0xFFFF` before cast in fetch | mem.rs |

## Fix Details

### Fix 1: SUM does not affect instruction fetch (spec §4.3.1)

**Spec**: "Irrespective of SUM, the supervisor may not execute code on pages with U=1."

**Change**: In both `TlbEntry::permits()` and `Mmu::check_perm()`, the U-bit privilege check
now explicitly excludes Fetch from SUM override:

```rust
let priv_ok = if f.contains(PteFlags::U) {
    match priv_mode {
        User => true,
        Supervisor => op != MemOp::Fetch && sum,  // SUM never applies to Fetch
        _ => false,
    }
} else {
    priv_mode != User
};
```

### Fix 2: MPRV-aware PMP check (spec §3.1.6.3, §3.7.1)

**Spec**: When MPRV=1 in M-mode, data accesses use MPP as effective privilege for both
address translation AND PMP checking.

**Change**: The final PMP check in `RVCore::translate()` now passes `priv_mode`
(which is already the effective privilege computed via `effective_priv()` for data access)
instead of `self.privilege`:

```rust
self.pmp.check(paddr, 1, op, priv_mode)  // was: self.privilege
```

### Fix 3: RV64 pmpcfg CSR layout (spec §3.7.1)

**Spec**: RV64 only has pmpcfg0 (entries 0..7) and pmpcfg2 (entries 8..15).
pmpcfg1 and pmpcfg3 are illegal on RV64.

**Change**: Side-effect handler uses cfg-gated mapping:
- RV64: pmpcfg0 → entries 0..7, pmpcfg2 → entries 8..15, pmpcfg1/3 ignored
- RV32: pmpcfg0..3 → entries 0..3, 4..7, 8..11, 12..15 (4 entries each)

### Fix 4: Locked PMP entry write-ignore (spec §3.7.1)

**Spec**: If L=1, writes to that entry's pmpcfg and pmpaddr are ignored.
If L=1 and A=TOR, writes to pmpaddr[i-1] are also ignored.

**Change**: `Pmp::update_cfg()` and `Pmp::update_addr()` now check:
- `self.entries[index].locked()` → skip write
- For addr: if next entry is locked+TOR → skip write on this addr

### Fix 5: LR uses Load path, SC uses Store path (spec §8.2)

**Spec**: LR loads a word (requires read permission). SC stores conditionally
(requires write permission). LR is load-like for fault classification.

**Change**:
- `lr_w`/`lr_d`: `self.translate(addr, MemOp::Load)` (was `MemOp::Amo`)
- `sc_w`/`sc_d`: `self.translate(addr, MemOp::Store)` (was `MemOp::Amo`)

### Fix 6: Reset clears MMU/PMP (spec reset behavior)

**Spec**: After reset, hart enters M-mode with address translation disabled.
CSRs reset to zero → MMU/PMP cached state must match.

**Change**: `RVCore::reset()` now includes:
```rust
self.mmu = Mmu::new();
self.pmp = Pmp::new();
```

### Fix 7: PMP checks full access range (spec §3.7.1)

**Spec**: "The matching PMP entry must match all bytes of an access."

**Change**: `Pmp::check()` signature changed from `(paddr, op, priv)` to
`(paddr, size, op, priv)`. Each entry's `contains()` method checks
`[paddr, paddr+size)` is fully within the entry's range.

### Fix 8: Bus overflow-safe arithmetic

**Issue**: `off + size` and `addr + size` in `ram_offset`/`find_mmio` could wrap on malicious input.

**Change**: All arithmetic uses `checked_add()`. Overflow returns `None`/`Err(BadAddress)`.

### Fix 9: Ram returns Err instead of panicking

**Issue**: `Ram::read`/`write`/`load` did unchecked slice indexing — panics on OOB.

**Change**: Each method validates `offset + size <= data.len()` (and `size <= size_of::<Word>()`
for read/write) via `checked_add().filter()`, returning `Err(BadAddress)` on failure.

## Test Coverage Added

New tests in pmp.rs:
- `locked_entry_ignores_writes` — L=1 prevents cfg/addr modification
- `tor_locks_prev_addr` — L=1+TOR prevents pmpaddr[i-1] modification
- `cross_boundary_access_denied` — access spanning region boundary is denied
