# `archModule` SPEC

> Source: [`/docs/archived/refactor/archModule/03_PLAN.md`](/docs/archived/refactor/archModule/03_PLAN.md).
> Iteration history, trade-off analysis, and implementation
> plan live under `docs/archived/refactor/archModule/`.

---


[**Goals**]

- G-1: Every arch-specific file lives under `xcore/src/arch/<name>/` in a
  flat topic layout (`arch/riscv/{cpu, csr, mm, trap, inst, isa, device}`;
  `arch/riscv/cpu/` holds only `{context.rs, debug.rs, mod.rs}`).
- G-2: Upper-layer modules (`cpu/`, `isa/`, `device/`) contain only
  arch-neutral items — trait definitions (`CoreOps`, `DebugOps`), neutral
  storage (`IrqState`), generic types (`CPU<Core>`) — plus cfg-gated type
  aliases / re-exports that name the concrete arch implementors. No
  `use crate::arch::<name>::…` path appears outside `arch/` and the seam
  alias sites.
- G-3: The RISC-V `mip` bit vocabulary (`SSIP` / `MSIP` / `STIP` / `MTIP` /
  `SEIP` / `MEIP` / `HW_IP_MASK`) lives in
  `arch/riscv/trap/interrupt.rs`.
- G-4: `device/intc/aclint.rs` and `device/intc/plic.rs` relocate to
  `arch/riscv/device/intc/{aclint,plic}.rs`. `device/intc/mod.rs` exposes
  `#[cfg(riscv)] pub type Intc = crate::arch::riscv::device::intc::Intc;`.
- G-5: Git history is preserved for every moved file (`git mv` only).
- G-6: **Test/boot preservation.** After every phase boundary (post-PR):
  `cargo test --workspace` passes (336 tests); `cpu-tests-rs` passes
  (31 tests); `am-tests` passes (8 tests); `make linux` boots to
  interactive shell; `make debian` boots Debian 13 Trixie to shell and
  runs Python3; difftest vs QEMU and Spike shows zero divergence.

- NG-1: No change to `BootConfig`, `BootLayout`, `MachineConfig`,
  `XError`, `XResult`, `CoreContext`, `PendingTrap`, `RESET_VECTOR`,
  `State`, `XCPU`, `with_xcpu`, `Breakpoint`. Downstream crates compile
  unchanged.
- NG-2: No global `trait Arch { type Word; … }` (00-M-001).
- NG-3: No `xdb` / `xlogger` / `xam` / `xlib` / `difftest` / `am-tests`
  source edits.
- NG-4: No `Makefile`, DTS, boot-config, or env-default changes.
- NG-5: `Bus::{aclint_idx, plic_idx, mtime, set_timer_source,
  set_irq_sink, ssip_flag, take_ssip}` remain in place. Bus ↔ intc
  contract redesign is out of scope; queued under `aclintSplit` /
  `plicGateway` / `directIrq`; pinned in `arch_isolation.rs` allow-list.
- NG-6: No semantic edits to `ACLINT` / `PLIC` / `UART` / `VirtioBlk` /
  `TestFinisher` / `RVCore` / `CSR` / `MMU` / `TLB` / trap logic. Only
  `use`-paths and `pub(in …)` tokens are edited inside moved files.
- NG-7: No MSRV / edition / Cargo dependency changes (prod or dev).

[**Architecture**]

```
xcore/src/
├── arch/
│   ├── mod.rs                (two-line #[cfg] switch: `pub mod riscv;` / `pub mod loongarch;`)
│   ├── riscv/
│   │   ├── mod.rs            (declares topic submodules — flat)
│   │   ├── cpu/              (RVCore, RVCoreContext, debug impl)
│   │   ├── csr/              (CsrFile, Mip, MStatus, PrivilegeMode, ops)
│   │   ├── mm/               (Mmu, Pmp, Tlb)
│   │   ├── trap/             (cause, exception, handler, interrupt — interrupt.rs hosts mip bits)
│   │   ├── inst/             (per-instruction handlers)
│   │   ├── isa/              (RvDecoder, DecodedInst, RVReg)
│   │   └── device/intc/      (Aclint, Plic, Intc bundle struct)
│   └── loongarch/{cpu/, isa/}   (stub)
├── cpu/
│   ├── mod.rs                (CPU<Core>; cfg-gated aliases: Core, CoreContext, PendingTrap)
│   ├── core.rs               (CoreOps, BootMode)
│   └── debug.rs              (DebugOps, Breakpoint)
├── isa/
│   ├── mod.rs                (cfg-gated: IMG, Decoder, DecodedInst, RVReg — whatever upper layer names)
│   └── instpat/              (unchanged)
├── device/
│   ├── mod.rs                (Device, IrqState; no mip bits; no arch-aware cfg lines)
│   ├── bus.rs                (NG-5 — unchanged)
│   ├── intc/mod.rs           (cfg-gated: pub type Intc)
│   └── …                     (ram, uart, test_finisher, virtio, virtio_blk — neutral)
├── config/   utils/   error.rs   lib.rs
```

**Seam shape.** Each upper-layer seam file contains only cfg-gated
aliases / re-exports that name arch types. Trait dispatch across the
seam uses `CoreOps` / `DebugOps`, which already exist. Concrete arch
data types consumed by name (`CoreContext`) or by value (`PendingTrap`)
cross the seam as cfg-gated `pub type` aliases; this is the honest
landing of R-017.

[**Invariants**]

- I-1: **Arch-path isolation.** No file under `xemu/xcore/src/` outside
  `arch/` references `crate::arch::riscv::` or `crate::arch::loongarch::`
  by name, **except** at the seam-alias sites enumerated in
  `xemu/xcore/tests/arch_isolation.rs`'s allow-list.
- I-2: **Vocabulary isolation.** No file under `xemu/xcore/src/` outside
  `arch/` contains the RISC-V vocabulary literals (`MSIP`, `MTIP`,
  `MEIP`, `SEIP`, `SSIP`, `STIP`, `mtime`, `mtimecmp`, `RVCore`,
  `Mstatus`, `Mip`, `Sv32`, `Sv39`) as identifiers, **except** the
  allow-list entries pinned per-file per-literal in
  `arch_isolation.rs` — which are exactly the `Bus` NG-5 residuals
  plus `device/bus.rs`'s debug-log strings `"aclint"` / `"plic"`.
- I-3: **Git history preserved.** `git log --follow` traces every
  moved file to its pre-refactor location. `git mv` only.
- I-4: **Public API unchanged.** `lib.rs` re-exports (`BootConfig`,
  `CoreContext`, `RESET_VECTOR`, `State`, `XCPU`, `DebugOps`,
  `Breakpoint`, `with_xcpu`, `Uart`, `XError`, `XResult`, `BootLayout`,
  `MachineConfig`) are identical in name, type, and path. `error.rs`'s
  `use crate::cpu::PendingTrap;` continues to resolve.
- I-5: **Behaviour unchanged.** `cargo test --workspace`,
  `cpu-tests-rs`, `am-tests`, `make linux`, `make debian` produce the
  same pass/fail and boot artefacts; difftest vs QEMU / Spike has zero
  divergence on the default cpu-tests-rs set.
- I-6: **Seam shape.** Each upper-layer seam file (`arch/mod.rs`,
  `cpu/mod.rs`, `isa/mod.rs`, `device/intc/mod.rs`) contains only
  cfg-gated type aliases / `pub use` re-exports in its arch-aware
  section — no `cfg_if!`, no `#[cfg]`-gated `mod`, no inline
  arch-specific logic. `device/mod.rs` contains zero `#[cfg]`-aware
  lines.

[**Data Structure**]

Upper-layer trait surface (arch-agnostic, unchanged from today):

```rust
// xcore/src/cpu/core.rs
pub trait CoreOps { /* unchanged — see cpu/core.rs:20-37 */ }

// xcore/src/cpu/debug.rs
pub trait DebugOps { /* unchanged */ }
```

Seam aliases (only cfg-aware lines in the upper layer):

```rust
// xcore/src/arch/mod.rs
#[cfg(riscv)]     pub mod riscv;
#[cfg(loongarch)] pub mod loongarch;

// xcore/src/cpu/mod.rs  (+ existing neutral items: CPU<Core>, BootConfig, BootMode, State, XCPU, with_xcpu, RESET_VECTOR)
#[cfg(riscv)]     pub type Core        = crate::arch::riscv::cpu::RVCore;
#[cfg(riscv)]     pub type CoreContext = crate::arch::riscv::cpu::context::RVCoreContext;
#[cfg(riscv)]     pub type PendingTrap = crate::arch::riscv::trap::PendingTrap;
#[cfg(loongarch)] pub type Core        = crate::arch::loongarch::cpu::LaCore;
// (CoreContext / PendingTrap for LoongArch added when that backend materialises)

// xcore/src/isa/mod.rs
#[cfg(riscv)]     pub use crate::arch::riscv::isa::{IMG, DECODER, DecodedInst, RVReg};
#[cfg(loongarch)] pub use crate::arch::loongarch::isa::IMG;

// xcore/src/device/intc/mod.rs
#[cfg(riscv)]     pub type Intc = crate::arch::riscv::device::intc::Intc;
```

`arch/riscv/mod.rs` (flat topic layout, visibility preserved from
`cpu/riscv/mod.rs:6-11`):

```rust
// xcore/src/arch/riscv/mod.rs
pub mod cpu;
pub mod csr;
pub(crate) mod mm;  // preserved from cpu/riscv/mod.rs:10
mod inst;           // preserved from cpu/riscv/mod.rs:9
pub mod trap;
pub mod isa;
pub mod device;
```

Arch-side mip bits (moved from `device/mod.rs:55-72` to
`arch/riscv/trap/interrupt.rs`; body byte-identical):

```rust
// xcore/src/arch/riscv/trap/interrupt.rs
pub const SSIP:       u64 = 1 << 1;
pub const MSIP:       u64 = 1 << 3;
pub const STIP:       u64 = 1 << 5;
pub const MTIP:       u64 = 1 << 7;
pub const SEIP:       u64 = 1 << 9;
pub const MEIP:       u64 = 1 << 11;
pub const HW_IP_MASK: crate::config::Word = (MSIP | MTIP | SEIP | MEIP) as _;
// + Interrupt enum (pre-existing content of this file)
```

Arch-side intc bundle (no trait; concrete struct consumed directly):

```rust
// xcore/src/arch/riscv/device/intc/mod.rs
pub mod aclint;
pub mod plic;

pub struct Intc {
    pub aclint: aclint::Aclint,
    pub plic:   plic::Plic,
}
```

[**API Surface**]

Public crate API (unchanged — I-4):

```rust
// xcore/src/lib.rs (unchanged)
pub use config::{BootLayout, MachineConfig};
pub use cpu::{
    BootConfig, CoreContext, RESET_VECTOR, State, XCPU,
    debug::{Breakpoint, DebugOps},
    with_xcpu,
};
pub use device::uart::Uart;
pub use error::{XError, XResult};
```

Intra-arch imports (referenced only inside `arch/riscv/`):

```rust
// xcore/src/arch/riscv/cpu/mod.rs  (after Phase 4)
use crate::{
    cpu::core::CoreOps,
    device::{Device, IrqState, bus::Bus,
             test_finisher::TestFinisher, uart::Uart, virtio_blk::VirtioBlk},
};
use super::{
    csr::{CsrAddr, CsrFile, MStatus, Mip, PrivilegeMode},
    device::intc::{Intc, aclint::Aclint, plic::Plic},
    mm::{Mmu, Pmp},
    trap::{PendingTrap, TrapCause, interrupt::HW_IP_MASK},
};
```

[**Constraints**]

- C-1: Landable as phased PRs. Green-bar (G-6) holds at every **PR
  boundary** that lands on trunk: PR 1, PR 2 (= 2a+2b+2c atomic),
  PR 3, PR 4, PR 5. No standalone non-green commit lands on trunk.
- C-2: **No semantic edits inside moved files** beyond `use`-path and
  `pub(in …)` token rewrites required to compile. Byte-identical
  behaviour. (R-021 narrowing from round 02's broader "no semantic
  edits" wording.)
- C-3: **`git mv` only** for every moved file. Verified by
  `git log --follow`.
- C-4: **No Cargo / MSRV / edition / dep changes** (prod or dev). NG-7.
- C-5: **Seam shape** — each upper-layer seam file contains only
  cfg-gated type aliases / `pub use` re-exports plus doc comments; no
  `cfg_if!`, no `#[cfg]`-gated `mod`, no inline arch-specific logic.
  `device/mod.rs` contains **zero** cfg-aware lines.

---
