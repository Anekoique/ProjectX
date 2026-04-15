# CSR subsystem

## Layering

Two layers with a clean split:

```
CsrFile    ← storage + descriptor-driven mask / shadow dispatch
RVCore     ← privilege checks, dynamic rules, side effects, trap generation
```

Key principle: `CsrFile` knows **what** a CSR is (address, width,
WARL mask, shadow); `RVCore` knows **when** a CSR access is allowed
and **what happens** after.

## Storage

Flat 4096-entry array indexed by the 12-bit CSR address:

```rust
pub struct CsrFile {
    regs: [Word; 4096],
}
```

Shadow registers (`sstatus`, `sip`, `sie`) don't occupy their own
slot — they redirect to the M-mode slot with a mask.

## `csr_table!` macro

`xcore/src/arch/riscv/cpu/csr/table.rs` declares every CSR in a
single macro invocation. The macro generates:

- The `CsrAddr` enum.
- The `CSR_DESCS` descriptor table (address, WARL mask, shadow
  target, side effects, difftest whitelist membership).
- The O(1) dispatch `match` for read / write.

One source of truth prevents the enum and descriptor table from
drifting apart.

## WARL model

Writes go through the descriptor's write mask:

```
new = (old & !mask) | (incoming & mask)
```

Read-only-zero bits are enforced by `mask = 0` on those fields.
Shadow registers wrap the M-mode register's read/write with an extra
mask (e.g. `sstatus` only exposes the S-mode subset of `mstatus`).

## Side effects

Some writes trigger xemu-internal reconfiguration:

| CSR | Side effect |
|-----|-------------|
| `satp` | `mmu.update_satp` → reconfigure SvMode + flush TLB + bump icache `ctx_tag` |
| `mstatus` | Recompute SUM / MXR flags |
| `pmpcfg* / pmpaddr*` | Rebuild PMP entries with lock semantics |
| `mtimecmp` | Recompute `next_fire_mtime` in ACLINT (Phase P3) |
| `fcsr / fflags / frm` | Route through shifted-subfield alias to the canonical `fcsr` |

## Traps on CSR access

CSR privilege violations raise architectural traps, **not** emulator
errors. Use:

```rust
self.raise_trap(TrapCause::IllegalInst, /*tval=*/ instruction_word);
return Ok(());
```

Never return `Err(XError)` from a trap — reserve `Err` for host
failures (I/O error) and emulator invariant violations. This is the
"`err2trap`" refactor pattern; see
[`../spec/err2trap/SPEC.md`](../../spec/err2trap/SPEC.md).

## Difftest whitelist

The `csr_table!` `@ difftest` annotation marks CSRs whose value is
checked against the reference every step. Currently 14 CSRs are on
the whitelist — architectural state that both DUT and REF model the
same way. CSRs that depend on xemu-specific timing (`time`,
`mcycle`) are excluded.
