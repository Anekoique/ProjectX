# Memory: MMU, TLB, PMP

## Access pipeline

```
vaddr → align check → MMU.translate(vaddr, op, priv) → paddr → PMP.check(paddr, op, priv) → Bus.access
                              │                                   ▲
                              └── page walk (pte_paddr checks PMP)┘
```

Four layers, each with a narrow responsibility. See
[`../spec/mm/SPEC.md`](../../spec/mm/SPEC.md) for the full design.

## MMU

- **Sv32** (RV32) and **Sv39** (RV64) — multi-level page walk with
  hardware A / D bit update.
- **`SvMode` descriptor** — runtime switch between Sv32 / 39 / 48 / 57;
  PTE format-dependent methods take `&SvMode`.
- **`satp` WARL** — RV64 currently restricts to Sv39; writes to
  unsupported modes are masked.

The MMU caches the effective privilege, SUM / MXR bits, and the
current `SvMode` — avoiding a CSR read on every translate.

## TLB

- **64 entries**, direct-mapped.
- **ASID-tagged**. Global pages (`PTE.G = 1`) match any ASID.
- **Flushed on `sfence.vma`** with the standard ASID / vaddr operand
  semantics.

## PMP

- **16 entries**, matching modes: `OFF`, `TOR`, `NA4`, `NAPOT`.
- **Partial-overlap detection** — a paddr straddling two entries raises
  the appropriate fault.
- **Lock bit** — once set, the entry is immutable until the next reset.
- **M-mode fast path** — when no entries are locked, M-mode bypasses
  the 16-entry linear scan entirely.

## Trap generation

MMU and PMP produce `Err(XError::PageFault)` or `Err(XError::BadAddress)`
up the stack. `RVCore` maps these to the correct `TrapCause`:

| XError | Trap cause |
|--------|------------|
| `PageFault { access: Load }` | `LoadPageFault` |
| `PageFault { access: Store }` | `StorePageFault` |
| `PageFault { access: Fetch }` | `InstPageFault` |
| `BadAddress { access: Load }` | `LoadAccessFault` |
| `BadAddress { access: Store }` | `StoreAccessFault` |

This translation happens once in the `step` → `commit_trap` path;
instruction handlers just propagate `?`.

## MPRV

When `mstatus.MPRV` is set, loads / stores use `mstatus.MPP` as the
effective privilege. xemu routes this through
`Mmu::effective_privilege(&mstatus, op)` — not by clamping the actual
`privilege` field.

## Typed RAM access (Phase P6)

Hot-path loads / stores of 1 / 2 / 4 / 8 bytes bypass the generic
`memmove` shim:

```rust
// Pseudocode
if op.aligned() && size ∈ {1,2,4,8} {
    direct_u{size}_read(ram_slice)
} else {
    bytemuck::copy_within(...)  // slow path
}
```

Drops the `_platform_memmove` + `Bus::{read,write}` combined bucket
from ~18 % to sub-2 % on the dhrystone / coremark / microbench
profile. See
[`../spec/perfHotPath/SPEC.md`](../../spec/perfHotPath/SPEC.md).
