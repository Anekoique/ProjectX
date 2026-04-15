# `archLayout` SPEC

> Source: base spec from [`/docs/archived/refactor/archLayout/00_PLAN.md`](/docs/archived/refactor/archLayout/00_PLAN.md),
> with subsequent delta amendments in rounds up to [`/docs/archived/refactor/archLayout/04_PLAN.md`](/docs/archived/refactor/archLayout/04_PLAN.md).
> Iteration history and trade-off analysis live under `docs/archived/refactor/archLayout/`.

---


[**Goals**]

- G-1: Nest the five CPU-internal topic modules (`csr`, `mm`, `trap`, `inst`,
  `isa`) under `arch/riscv/cpu/`, preserving git history via `git mv`.
- G-2: Keep `arch/riscv/device/` as a sibling of `cpu/`; devices are not
  CPU-internal and `device/intc/` already lives behind its own seam
  (`crate::device::intc`).
- G-3: Resolve the `isa/` (encoding) vs `inst/` (execution) naming collision by
  renaming `arch/riscv/inst/` → `arch/riscv/cpu/executor/` (see TR-2).
- G-4: Leave `xcore/src/isa/` at the crate root — it holds the neutral `pub use`
  seam plus the `instpat/` pest grammar (ISA-defining data). The existing
  `arch/riscv/isa/decoder.rs` → `../../../isa/instpat/riscv.instpat`
  `include_str!` path becomes a four-hop path after the nest.
- G-5: Behaviour is byte-identical. All 344 tests (343 unit/integration from
  archModule-03 + 1 `arch_isolation` integration test) must pass at every phase
  boundary; `make linux` boots to shell; `make debian` boots Debian 13 Trixie
  to shell; difftest vs QEMU/Spike shows zero divergence.

- NG-1: Do NOT move `xcore/src/isa/instpat/` (the pest grammar is neutral data
  that belongs at the crate root, not inside an arch backend).
- NG-2: Do NOT fold `arch/riscv/device/` into `arch/riscv/cpu/`.
- NG-3: Do NOT touch `arch/loongarch/` (the current stub is already minimal;
  when loongarch implementation lands it can adopt the same nested shape
  naturally).
- NG-4: Do NOT modify landed plan/review/master documents in `docs/archived/refactor/archModule/`.
- NG-5: Do NOT change any public API visible outside `xcore` (the xemu CLI sees
  `xcore::CPU`, `xcore::cpu::BootConfig`, etc. — none of those paths change).
- NG-6: Do NOT introduce new dependencies or new `cfg` flags.
- NG-7: Do NOT reintroduce the `arch/riscv/isa/` → `xcore/src/isa/riscv/`
  parallel-tree pattern (Direction B, rejected in TR-1).

[**Architecture**]

Before (archModule-03 landed):

```
xemu/xcore/src/
├── arch/
│   ├── mod.rs
│   └── riscv/
│       ├── mod.rs          (pub mod cpu; pub mod csr; pub mod device;
│       │                    mod inst; pub mod isa;
│       │                    pub(crate) mod mm; pub mod trap;)
│       ├── cpu/            (context.rs, debug.rs, mod.rs — RVCore)
│       ├── csr/            (mip.rs, mstatus.rs, ops.rs, privilege.rs) + csr.rs
│       ├── device/         (intc/{aclint, plic}/, mod.rs)
│       ├── inst/           (atomic.rs, base.rs, compressed.rs, float.rs,
│       │                    mul.rs, privileged.rs, zicsr.rs) + inst.rs
│       ├── isa/            (decoder.rs, inst.rs, reg.rs, mod.rs)
│       ├── mm/             (mmu.rs, pmp.rs, tlb.rs) + mm.rs
│       └── trap/           (cause.rs, exception.rs, handler.rs, interrupt.rs)
│                            + trap.rs
├── cpu/                    (seam: pub type Core = riscv::cpu::RVCore;)
├── device/                 (seam: intc/mod.rs re-exports Aclint, Plic;
│                            mod.rs re-exports SSIP/MSIP/…)
└── isa/                    (seam: pub use riscv::isa::{…}; + instpat/ pest)
```

After (target):

```
xemu/xcore/src/
├── arch/
│   ├── mod.rs              (unchanged)
│   └── riscv/
│       ├── mod.rs          (pub mod cpu; pub mod device;)
│       ├── cpu/
│       │   ├── mod.rs      (RVCore — unchanged code; super-path imports rewritten)
│       │   ├── context.rs  (unchanged)
│       │   ├── debug.rs    (use super::super::… → use super::… adjustments)
│       │   ├── csr/        (mip.rs, mstatus.rs, ops.rs, privilege.rs)
│       │   ├── csr.rs
│       │   ├── executor/   (atomic.rs, base.rs, compressed.rs, float.rs,
│       │   │                mul.rs, privileged.rs, zicsr.rs)  [renamed from inst/]
│       │   ├── executor.rs [renamed from inst.rs]
│       │   ├── isa/        (decoder.rs, inst.rs, reg.rs)
│       │   ├── isa.rs      [renamed from isa/mod.rs into the mod-in-file style
│       │   │                already used by siblings: csr.rs / mm.rs / trap.rs /
│       │   │                inst.rs — see TR-3]
│       │   ├── mm/         (mmu.rs, pmp.rs, tlb.rs)
│       │   ├── mm.rs
│       │   ├── trap/       (cause.rs, exception.rs, handler.rs, interrupt.rs)
│       │   └── trap.rs
│       └── device/         (unchanged — still a sibling of cpu/)
├── cpu/                    (seam: pub type Core = riscv::cpu::RVCore;
│                            pub type CoreContext = riscv::cpu::context::RVCoreContext;
│                            pub type PendingTrap = riscv::cpu::trap::PendingTrap;)
├── device/                 (seam: intc/mod.rs re-exports riscv::device::intc::…;
│                            mod.rs re-exports riscv::cpu::trap::interrupt::…)
└── isa/                    (seam: pub use riscv::cpu::isa::{…}; + instpat/ pest)
```

[**Invariants**]

- I-1: Outside `arch/` and outside the five seam files in `arch_isolation.rs`
  `SEAM_FILES`, **zero** source lines reference `crate::arch::riscv::` or
  `crate::arch::loongarch::`. Inherited verbatim from archModule-03.
- I-2: After the nest, the **only** arch subpaths the seam files name are under
  `crate::arch::riscv::cpu::*` (for CPU/ISA/trap vocabulary) and
  `crate::arch::riscv::device::*` (for interrupt controllers). No seam file
  references the old flat paths `crate::arch::riscv::{csr, mm, trap, inst, isa}::`.
- I-3: The `arch_isolation` integration test's `SEAM_ALLOWED_SYMBOLS` remains a
  name-level allow-list. Symbol names (`Core`, `Aclint`, `DECODER`, etc.) do
  not change; only the arch paths behind them shift.
- I-4: `git log --follow` from each moved file reaches its pre-nest ancestor
  (because `git mv` was used).
- I-5: The `build.rs`-emitted `cfg(riscv)` / `cfg(loongarch)` / `cfg(isa32)` /
  `cfg(isa64)` flags remain the sole source of truth for arch selection (01-M-003).
- I-6: No new `trait Arch` is introduced (00-M-001).

[**Data Structure**]

No structural changes. Every struct, enum, trait, and macro keeps its current
shape and visibility, with **one** visibility relaxation:

```rust
// Before (arch/riscv/mod.rs):
pub(crate) mod mm;

// After (arch/riscv/cpu/mod.rs adds mm in its own nest):
pub(in crate::arch::riscv) mod mm;
```

Rationale: the only non-arch caller of `mm::MemOp` is `inst/atomic.rs`, which
is being renamed to `cpu/executor/atomic.rs` and thus lives **inside** the same
`arch::riscv` subtree — so `pub(in crate::arch::riscv)` is sufficient. This
matches `trap`'s existing `pub(in crate::arch::riscv)` pattern.

[**API Surface**]

Seam file diffs. Only path roots change; symbol names are identical.

```rust
// xcore/src/cpu/mod.rs — type aliases
// Before:
#[cfg(riscv)] pub type Core        = crate::arch::riscv::cpu::RVCore;
#[cfg(riscv)] pub type CoreContext = crate::arch::riscv::cpu::context::RVCoreContext;
#[cfg(riscv)] pub type PendingTrap = crate::arch::riscv::trap::PendingTrap;
// After (only PendingTrap path changes):
#[cfg(riscv)] pub type Core        = crate::arch::riscv::cpu::RVCore;
#[cfg(riscv)] pub type CoreContext = crate::arch::riscv::cpu::context::RVCoreContext;
#[cfg(riscv)] pub type PendingTrap = crate::arch::riscv::cpu::trap::PendingTrap;
```

```rust
// xcore/src/isa/mod.rs — re-exports
// Before:
#[cfg(riscv)] pub use crate::arch::riscv::isa::{DECODER, DecodedInst, IMG, InstFormat, InstKind, RVReg};
// After:
#[cfg(riscv)] pub use crate::arch::riscv::cpu::isa::{DECODER, DecodedInst, IMG, InstFormat, InstKind, RVReg};
```

```rust
// xcore/src/device/mod.rs — mip bit re-export
// Before:
#[cfg(riscv)] pub use crate::arch::riscv::trap::interrupt::{HW_IP_MASK, MEIP, MSIP, MTIP, SEIP, SSIP, STIP};
// After:
#[cfg(riscv)] pub use crate::arch::riscv::cpu::trap::interrupt::{HW_IP_MASK, MEIP, MSIP, MTIP, SEIP, SSIP, STIP};
```

```rust
// xcore/src/device/intc/mod.rs — ACLINT / PLIC re-export
// Before:
#[cfg(riscv)] pub use crate::arch::riscv::device::intc::{Aclint, Plic};
// After: UNCHANGED (device/ did not move).
```

No other public surface changes.

[**Constraints**]

- C-1: Every phase must leave the tree `cargo build`-, `cargo test`-, and
  `cargo clippy`-clean. No intermediate "wip" commit may break `make test`.
- C-2: `git mv` is the only mechanism used to relocate files — no copy-and-delete.
- C-3: Behaviour is byte-identical. Difftest vs QEMU/Spike must show zero new
  divergences. `make linux` and `make debian` boots must reach the same shell
  prompt as pre-refactor.
- C-4: No new dependencies (NG-6). The integration test `arch_isolation` stays
  on its current `std::fs`-only text-level check.
- C-5: The `include_str!("../../../isa/instpat/riscv.instpat")` path in
  `isa/decoder.rs` must be updated to `"../../../../isa/instpat/riscv.instpat"`
  after decoder.rs moves from `arch/riscv/isa/decoder.rs` to
  `arch/riscv/cpu/isa/decoder.rs` (one additional `..` hop).
- C-6: `pest_derive::Parser` `#[grammar = "src/isa/instpat/riscv.pest"]` is
  `CARGO_MANIFEST_DIR`-relative, not file-relative; it does **not** need
  adjustment (verified against the current annotation in `isa/decoder.rs`).
- C-7: The `arch_isolation` integration test must pass after every phase. Its
  allow-list and pinned debug-string counts are updated in the same commit that
  changes the underlying paths.

---
