# Supported ISA

xemu implements the RISC-V unprivileged ISA plus the privileged
model, across both RV32 and RV64 via `cfg_if`.

## Base + standard extensions

| Ext | Description | RV32 | RV64 |
|-----|-------------|:----:|:----:|
| `I` | Base integer | ✅ | ✅ |
| `M` | Multiply / divide | ✅ | ✅ |
| `A` | Atomic (LR/SC + 9 AMO ops, `.w` and `.d`) | ✅ | ✅ |
| `F` | Single-precision float | ✅ | ✅ |
| `D` | Double-precision float | ✅ | ✅ |
| `C` | Compressed | ✅ | ✅ |
| `Zicsr` | CSR access | ✅ | ✅ |
| `Zifencei` | `fence.i` | ✅ | ✅ |

**DTS advertisement:** `riscv,isa = "rv64imafdcsu_sstc"`.

## Privileged ISA

- **M / S / U** modes with full trap delegation (`medeleg` /
  `mideleg`).
- **Vectored** and **direct** `mtvec` / `stvec`.
- **`mret` / `sret`** with MPRV handling.
- **Sstc** — S-mode direct `stimecmp`.

## MMU

| Mode | Support |
|------|---------|
| Bare (identity) | ✅ |
| Sv32 (RV32) | ✅ |
| Sv39 (RV64) | ✅ — hardware A/D bit update |
| Sv48 | Descriptor exists; write to `satp` masks it off |
| Sv57 | Not wired |

- **TLB**: 64-entry direct-mapped, ASID-tagged, global-page aware.
- **PMP**: 16 entries, TOR / NA4 / NAPOT, lock semantics, partial-overlap
  detection.

## Float details

- `softfloat_pure` — pure Rust Berkeley softfloat-3.
- NaN-boxing for F operands when D is also active.
- `fcsr` / `fflags` / `frm` are shifted subfield aliases of one
  canonical `fcsr` (see [CSR subsystem](../internals/csr.md)).
- `mstatus.FS` tracked as `Off` / `Initial` / `Clean` / `Dirty`, with
  `SD` recomputed on every `mstatus` / `fcsr` write.

## What's not implemented

- **V** (vector) — RVV is not supported.
- **H** (hypervisor) — no HS-mode.
- **Zba / Zbb / Zbc / Zbs** (bit-manipulation) — deferred.
- **Zicbom / Zicboz** (cache management) — no caches modelled.
- **Svnapot / Svpbmt** — not wired.

## Instruction table

For the full per-mnemonic implementation status, see
[`../../spec/inst/SPEC.md`](../../spec/inst/SPEC.md).
