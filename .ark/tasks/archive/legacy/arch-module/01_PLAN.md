# `archModule` PLAN `01`

> Status: Draft
> Feature: `archModule`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `00_MASTER.md`

---

## Summary

Consolidate every arch-specific file in `xcore/src/` into a single
**topic-organised** `arch/<name>/` tree. `cpu/`, `isa/`, and `device/` keep
**only arch-neutral interfaces**; each exposes its arch backend through a
single `cfg_if` seam that selects `arch::riscv::*` or `arch::loongarch::*`.

Scope (narrower and more precise than round 00):

1. Relocate the existing RISC-V and LoongArch sub-trees from
   `cpu/<arch>/` and `isa/<arch>/` into `arch/<arch>/cpu/` and
   `arch/<arch>/isa/` via `git mv`, with all `pub(in …)` visibility paths
   rewritten.
2. Relocate `device/intc/aclint.rs` and `device/intc/plic.rs` (end-to-end
   RISC-V artefacts — MSIP/MTIP/MEIP/SEIP, mtimecmp, hart contexts) into
   `arch/riscv/device/intc/` behind a neutral `device/intc/mod.rs`
   `cfg_if` seam.
3. Move the RISC-V mip bit constants (`SSIP`/`MSIP`/`STIP`/`MTIP`/`SEIP`/
   `MEIP`, `HW_IP_MASK`) into `arch/riscv/trap/interrupt.rs` (topic:
   interrupt vocabulary). `device/mod.rs` re-exports them only through a
   `cfg_if` seam that is active under `cfg(riscv)`.
4. Reorganise `arch/<arch>/` by **topic/theme** (`trap/interrupt`, `csr`,
   `mm`, `inst`, `device/intc`, …) rather than flat per-concern files.
   `irq_bits.rs` is renamed to `irq.rs` and folded under
   `arch/riscv/trap/interrupt.rs` per M-002.

This fully addresses MANUAL_REVIEW.md **#4** and addresses **#3** to the
extent that *bus addressing is arch-independent*. The residual MANUAL_REVIEW
#3 concern (bus still has `aclint_idx` / `plic_idx` / `mtime()` /
`set_timer_source` / `set_irq_sink` RISC-V vocabulary on arch-neutral
`Bus`) is deliberately **not** resolved here — it requires redesigning
the device/interrupt-line contract and is queued as follow-up plans
`aclintSplit`, `plicGateway`, `directIrq`. The Summary narrows the
MANUAL_REVIEW #3 claim accordingly (see R-001, TR-1 resolutions).

Out of scope, unchanged: `CoreOps`, `DebugOps`, `Device`, `BootConfig`,
`BootLayout`, `MachineConfig`; ISA semantics; boot flows; xdb / xam /
xemu binary; MSRV, edition, dependencies; Makefile targets.

## Log

[**Feature Introduce**]

- Introduces `xcore/src/arch/` as a **topic-organised** tree (per M-002):
  `arch/riscv/{cpu, isa, trap, csr, mm, inst, device}` etc. Files inside
  `arch/` are named by the *semantic concept* they encode (e.g.
  `trap/interrupt.rs`, `csr/mip.rs`), not by the upper-layer module they
  feed into. Upper-layer modules (`cpu/`, `isa/`, `device/`) see only the
  topical interface.
- Introduces a neutral `device/intc/mod.rs` `cfg_if` seam that picks an
  arch-specific interrupt-controller back-end. Today only RISC-V has real
  controllers; LoongArch exposes no controllers and the seam resolves to
  an empty module under LoongArch builds (no stub `MEIP` constants etc.).
- Keeps the `cfg_if` seam pattern and the fine-grained `CoreOps`-style
  abstractions that exist today (per M-001). No `trait Arch { type Word;
  … }` is introduced. `T-1` from round 00 is closed.
- Preserves every existing public API of `xcore` (`BootConfig`,
  `CoreContext`, `RESET_VECTOR`, `State`, `XCPU`, `Breakpoint`,
  `DebugOps`, `with_xcpu`, `Uart`, `XError`, `XResult`, `BootLayout`,
  `MachineConfig`). Downstream crates (`xdb`, `xam`, `xemu` binary)
  compile unchanged.

[**Review Adjustments**]

- R-001 (CRITICAL) accepted, option (a): `device/intc/aclint.rs` and
  `device/intc/plic.rs` move under `arch/riscv/device/intc/` with a
  `cfg_if` seam in `device/intc/mod.rs`. Bus field names
  (`aclint_idx` / `plic_idx` / `mtime` / `set_timer_source` /
  `set_irq_sink`) are out of scope here and documented as residual
  follow-ups, so the Summary narrows the MANUAL_REVIEW #3 claim.
- R-002 (CRITICAL) accepted: Phase 2/3 now include an explicit
  "rewrite `pub(in crate::cpu::riscv)` → `pub(in crate::arch::riscv::cpu)`
  and `pub(in crate::isa::riscv)` → `pub(in crate::arch::riscv::isa)`"
  substep, with an enumerated list of the 11 call sites observed in the
  current tree and a grep gate that must return 0 hits after each phase.
- R-003 (HIGH) accepted: all validation entries now invoke arch selection
  via `X_ARCH=<name> cargo check -p xcore` (the real build mechanism
  emitted by `xemu/xcore/build.rs`). `--features loongarch` removed.
  V-E-1 retargeted to assert a `compile_error!` fires when both
  `riscv` and `loongarch` cfgs are set via `RUSTFLAGS`.
- R-004 (HIGH) accepted: `V-F-2` is now a **vocabulary allow-list** grep
  (`MSIP`, `MTIP`, `MEIP`, `SEIP`, `SSIP`, `STIP`, `mtime`, `mtimecmp`,
  `aclint`, `plic`, `hart`, `RVCore`, `Mstatus`, `Mip`, `Sv32`, `Sv39`)
  outside the explicit allow-list of seam files. Catches the R-001 bus
  residuals too (those fail and are explicitly allow-listed with a
  follow-up reference, not silently ignored).
- R-005 (MEDIUM) accepted: `V-UT-2`'s brittle `type_name`-string canary
  is replaced by a behavioural seam-liveness check — see V-UT-2.
- R-006 (MEDIUM) accepted: `device/mod.rs` re-exports of mip bit
  constants are gated on `cfg(riscv)`; no placeholder
  `arch/loongarch/trap/interrupt.rs` file is invented; LoongArch
  consumers that ask for `MSIP`/`MEIP` fail to find them, which is the
  correct enforcement.
- R-007 (MEDIUM) accepted: Phase 4 includes an explicit step to rewrite
  `arch/riscv/cpu/mod.rs` (formerly `cpu/riscv/mod.rs`) device imports
  from `crate::device::intc::{aclint::Aclint, plic::Plic}` to
  `crate::arch::riscv::device::intc::{aclint::Aclint, plic::Plic}`
  (intra-arch path, does not violate I-1). Construction boundary is
  documented: MMIO addresses and wiring stay in `arch/riscv/cpu/mod.rs`
  because "which addresses the RISC-V platform exposes" is an arch
  policy. Devices themselves are arch-local under `arch/riscv/device/`.
- R-008 (LOW) accepted: Phase 3 acceptance bar downgraded explicitly to
  "`X_ARCH=loongarch32 cargo check -p xcore` and `X_ARCH=loongarch64
  cargo check -p xcore` succeed; LoongArch remains a ~5-line stub".
- R-009 (LOW) accepted: the incorrect "register `mod arch;` before `mod
  cpu;`" note is dropped. Module order is irrelevant.

[**Master Compliance**]

- **M-001 (keep cfg-if)** applied. The plan uses `cfg_if` at each seam
  (`cpu/mod.rs`, `isa/mod.rs`, `device/intc/mod.rs`, `device/mod.rs` mip
  bit re-export, `arch/mod.rs`) and keeps the existing `CoreOps` /
  `DebugOps` fine-grained traits. No `trait Arch` with associated types
  is introduced. T-1 from round 00 is formally closed in favour of
  Option A.
- **M-002 (rename `irq_bits.rs` → `irq.rs` AND reorganise `arch/` by
  topic)** applied. There is no flat `irq_bits.rs`. The mip bit
  constants move into `arch/riscv/trap/interrupt.rs` alongside the
  `Interrupt` enum that already lives in `cpu/riscv/trap/interrupt.rs`
  today — the two belong to the same topic ("interrupt vocabulary"). The
  rest of `arch/riscv/` mirrors this: CSR registers under
  `arch/riscv/csr/`, memory under `arch/riscv/mm/`, trap logic under
  `arch/riscv/trap/`, decoder/inst under `arch/riscv/isa/` and
  `arch/riscv/inst/`, devices under `arch/riscv/device/`. Every file in
  `arch/` is reachable through an upper-layer `cfg_if` seam by topic;
  the upper layers never name RISC-V files directly.

### Changes from Previous Round

[**Added**]

- G-5: relocate `device/intc/aclint.rs` and `device/intc/plic.rs` into
  `arch/riscv/device/intc/` behind a `device/intc/mod.rs` `cfg_if` seam
  (new scope per R-001).
- G-6: `arch/<arch>/` directory structure is topic-organised per M-002;
  plan enumerates the target layout.
- I-5: no file under `xcore/src/{cpu,device,isa,utils,config,error}.rs`
  or any non-`arch/` subdirectory may contain RISC-V **vocabulary**
  (`MSIP`/`MEIP`/…/`RVCore`/`Mstatus`/`Sv39`/…) outside the `cfg_if`
  seam files. Enforced by V-F-2 vocabulary grep.
- NG-5: no changes to `Bus`'s RISC-V-named fields (`aclint_idx`,
  `plic_idx`, `mtime`, `set_timer_source`, `set_irq_sink`) in this
  iteration. Tracked as residual MANUAL_REVIEW #3 follow-up
  (`aclintSplit`, `plicGateway`, `directIrq`).
- NG-6: no semantic edits to ACLINT/PLIC/Uart/VirtioBlk behaviour.
  Relocation only.
- Explicit enumeration of all 11 `pub(in crate::cpu::riscv)` /
  `pub(in crate::isa::riscv)` call sites in the Phase 2 substep
  (per R-002).
- Phase 4 substep to rewrite `arch/riscv/cpu/mod.rs` device imports
  to intra-arch paths (per R-007).
- Phase 5 documents a `compile_error!` in `arch/mod.rs` for the
  "both cfgs set" case (per R-003 V-E-1 retarget).
- TR-1 response acknowledges MANUAL_REVIEW #3 is only *partially*
  addressed and forward-references follow-up plans.

[**Changed**]

- Summary scope: now explicitly names the device/intc relocation; no
  longer claims full MANUAL_REVIEW #3 closure.
- G-3 (was "move mip bits to `arch/riscv/irq_bits.rs`"): now "move mip
  bits into `arch/riscv/trap/interrupt.rs` as part of the topical
  'interrupt' module". File name `irq_bits.rs` no longer exists.
- Validation: `--features loongarch` and `--features riscv,loongarch`
  references replaced with `X_ARCH=...`-based invocations.
- V-F-2: rewritten as a vocabulary allow-list grep instead of a
  `crate::arch::…` deny-list grep.
- V-UT-2: replaced `type_name`-string canary with a behavioural seam
  check (`Core::new().pc() == VirtAddr::from(RESET_VECTOR)`).
- Phase 2 substep-5 calls out `pub(in …)` rewrites explicitly.
- Phase 3 acceptance bar narrowed to "cargo check" only, with an
  explicit "LoongArch is a stub" disclosure.
- Phase 4 expanded to cover intc relocation **and** mip-bit relocation.
- Phase 5 renamed from "docs touch-up" to "seam hardening + docs";
  adds `compile_error!` gate, rustdoc touch-up of `lib.rs`.
- `device/mod.rs` mip bit re-exports now gated on `cfg(riscv)` (per
  R-006) instead of pulling from a LoongArch stub.
- Phased-PR naming added to Implementation Plan per TR-4.

[**Removed**]

- Flat file `arch/riscv/irq_bits.rs` and stub
  `arch/loongarch/irq_bits.rs` (per M-002 and R-006 respectively).
- T-1 as an open trade-off (closed by M-001).
- Phase 1's erroneous "before `mod cpu;`" note (per R-009).
- V-E-1's unrunnable `--features riscv,loongarch` invocation (replaced).

[**Unresolved**]

- Bus-level RISC-V vocabulary (`aclint_idx`, `plic_idx`, `mtime()`,
  `set_timer_source`, `set_irq_sink`) remains in `device/bus.rs`. This
  is the core of MANUAL_REVIEW #3's "bus design seems to target only
  RISC-V" and requires a redesign of the bus-to-interrupt-controller
  contract (source-ID allocation, generic "timer source" trait,
  per-device IRQ routing). Tracked as residual follow-up; explicitly
  excluded here to keep this refactor purely mechanical.
- External-device → PLIC direct-IRQ delivery (MANUAL_REVIEW #5/#6) is
  unchanged. This plan preserves today's Bus-mediated IRQ dispatch.
- `arch/loongarch/` remains a ~5-line stub under both ISA widths. This
  plan does not improve LoongArch coverage.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Option (a). `device/intc/aclint.rs` and `device/intc/plic.rs` move to `arch/riscv/device/intc/{aclint,plic}.rs` via `git mv`; `device/intc/mod.rs` becomes a `cfg_if` seam. Bus-level RISC-V vocabulary is explicitly queued as residual follow-up (NG-5) and the Summary narrows the MANUAL_REVIEW #3 claim to "bus addressing is arch-independent; bus wiring remains residual". |
| Review | R-002 | Accepted | Phase 2 substep-5 enumerates all 11 call sites (`cpu/riscv/csr.rs:95`, `cpu/riscv/csr/ops.rs:7,16`, `cpu/riscv/trap.rs:21,26,31,35,55`, `cpu/riscv/mm/tlb.rs:10,58`, `cpu/riscv/mm/mmu.rs:20`) and rewrites `pub(in crate::cpu::riscv)` → `pub(in crate::arch::riscv::cpu)`, `pub(in crate::isa::riscv)` → `pub(in crate::arch::riscv::isa)`. Gate: `rg 'pub\(in crate::(cpu\|isa)::(riscv\|loongarch)' xemu/xcore/src` = 0 hits. Same rule is written into Phase 3 even though LoongArch has none today. |
| Review | R-003 | Accepted | All validation entries use `X_ARCH=<riscv32\|riscv64\|loongarch32\|loongarch64> cargo check -p xcore` (see `xemu/xcore/build.rs:19–31`). V-E-1 retargeted: instead of invoking two features simultaneously (impossible given `build.rs` picks one from `X_ARCH`), the plan asserts that `arch/mod.rs` contains `#[cfg(all(riscv, loongarch))] compile_error!("...")` and that manually injecting both cfgs via `RUSTFLAGS='--cfg riscv --cfg loongarch'` produces the expected compile-time error. |
| Review | R-004 | Accepted | V-F-2 rewritten as a vocabulary allow-list grep (listed in Validation below). Explicit allow-list of seam files that may legitimately mention RISC-V vocabulary (and `device/bus.rs` with a tracked follow-up reference per NG-5). |
| Review | R-005 | Accepted | V-UT-2 replaced by two checks: a compile-time seam check (`const _: fn() = || { let _: arch::selected::cpu::Core; };`) and a behavioural check (under `cfg(riscv)`, `Core::new().pc() == VirtAddr::from(RESET_VECTOR)`). Neither relies on `type_name` spelling. |
| Review | R-006 | Accepted | `device/mod.rs` re-exports of mip bit constants are wrapped in `#[cfg(riscv)]`; LoongArch builds see no such constants. No `arch/loongarch/trap/interrupt.rs` placeholder with invented RISC-V bits is created. |
| Review | R-007 | Accepted | Phase 4 includes: update `arch/riscv/cpu/mod.rs` `use` list from `crate::device::intc::{aclint::Aclint, plic::Plic}` to `crate::arch::riscv::device::intc::{aclint::Aclint, plic::Plic}`. MMIO construction (addresses `0x0200_0000` for ACLINT, `0x0C00_0000` for PLIC, `0x1000_0000` for UART, `0x1000_1000` for virtio-blk, `0x10_0000` for TestFinisher) stays in `arch/riscv/cpu/mod.rs` — it is arch policy. Uart / TestFinisher / VirtioBlk stay in `device/` (they are arch-neutral devices used by the RISC-V platform). |
| Review | R-008 | Accepted | Phase 3 acceptance downgraded: "`X_ARCH=loongarch32 cargo check -p xcore` succeeds; LoongArch remains a stub, meaningful LoongArch coverage is out of scope." S3 state transition updated accordingly. |
| Review | R-009 | Accepted | The "before `mod cpu;`" parenthetical is dropped. Phase 1 step simply adds `mod arch;` in `xcore/src/lib.rs` alongside `mod cpu;` / `mod isa;`; order is immaterial. |
| Review | TR-1 | Accepted | Option A confirmed (keep `cfg_if`) per M-001. T-1 removed from open trade-offs. Summary now states MANUAL_REVIEW #3 is *partially* addressed; NG-5 names follow-up plans. |
| Review | TR-2 | Accepted | Option A: back-compat re-exports at `cpu::*` / `isa::*` / `device::*` (via `cfg_if` seam) preserved so xdb/xam/xemu-binary compile unchanged. |
| Review | TR-3 | Accepted | Option A: `IrqState` stays in `device/mod.rs`; bit-layout semantics (the mip bit constants) move to `arch/riscv/trap/interrupt.rs`. Wording tightened: "`IrqState` is arch-neutral storage for arch-specific bit positions; the bit positions live in `arch/<arch>/trap/interrupt.rs`". |
| Review | TR-4 | Accepted | Option B: phased PRs. Implementation Plan below lists proposed PR titles per phase. |
| Master | M-001 | Applied | `cfg_if` seam preserved at every boundary (`cpu/mod.rs`, `isa/mod.rs`, `device/intc/mod.rs`, `device/mod.rs` mip re-export, `arch/mod.rs`). No `trait Arch` introduced. Existing `CoreOps` / `DebugOps` fine-grained traits are unchanged — they remain the upper-layer contract. |
| Master | M-002 | Applied | No file named `irq_bits.rs` exists in the target layout. `arch/<arch>/` is reorganised by topic: `arch/riscv/{cpu,isa,csr,mm,trap,inst,device}/…`. The mip bit constants live in `arch/riscv/trap/interrupt.rs` (topic: interrupt vocabulary). Every file under `arch/` is reachable from the upper layer only through a `cfg_if` seam keyed on a topic (not on a concrete filename). |

> Rules:
> - Every prior HIGH / CRITICAL finding (R-001, R-002, R-003) appears
>   above.
> - Every Master directive (M-001, M-002) appears above.
> - Rejections require explicit reasoning. No rejections in this round.

---

## Spec

[**Goals**]

- G-1: Create `xcore/src/arch/` with `arch/riscv/` and `arch/loongarch/`.
  Every arch-specific file (cpu, isa, **and** arch-specific device logic)
  lives under exactly one `arch/<name>/` subtree.
- G-2: `cpu/mod.rs`, `isa/mod.rs`, `device/intc/mod.rs`, and the mip-bit
  re-export in `device/mod.rs` each contain **exactly one** `cfg_if`
  block that points into `arch::<arch>::…`. No other file under
  `xcore/src/` outside `arch/` may reference `arch::riscv::…` or
  `arch::loongarch::…` by concrete path, and no such file may contain
  RISC-V vocabulary (MSIP/MEIP/…/RVCore/Mstatus/Sv39).
- G-3: Move the RISC-V mip bit constants (SSIP/MSIP/STIP/MTIP/SEIP/MEIP,
  HW_IP_MASK) out of `device/mod.rs` into
  `arch/riscv/trap/interrupt.rs`. `device/mod.rs` re-exports them back
  only under `#[cfg(riscv)]`. `IrqState` itself stays in `device/`
  because its `Arc<AtomicU64>` storage is arch-neutral — only the *bit
  positions* are arch-specific.
- G-4: Move `device/intc/aclint.rs` and `device/intc/plic.rs` into
  `arch/riscv/device/intc/{aclint,plic}.rs`. `device/intc/mod.rs`
  becomes a `cfg_if` seam (`pub use crate::arch::selected::device::intc::*`
  on `cfg(riscv)`; empty on `cfg(loongarch)`).
- G-5: `arch/<arch>/` is topic-organised (per M-002). The target layout
  (RISC-V) is:
  ```
  arch/riscv/
  ├── mod.rs                (pub use re-exports keyed by topic)
  ├── cpu/                  (from cpu/riscv/{context,mod,debug})
  │   ├── mod.rs
  │   ├── context.rs
  │   └── debug.rs
  ├── isa/                  (from isa/riscv/ — decoder, reg, inst)
  ├── inst/                 (from cpu/riscv/inst.rs + cpu/riscv/inst/)
  ├── csr/                  (from cpu/riscv/csr.rs + cpu/riscv/csr/)
  ├── mm/                   (from cpu/riscv/mm.rs + cpu/riscv/mm/)
  ├── trap/                 (from cpu/riscv/trap.rs + cpu/riscv/trap/;
  │   │                      mip bit constants land in interrupt.rs)
  │   ├── cause.rs
  │   ├── exception.rs
  │   ├── handler.rs
  │   └── interrupt.rs     (Interrupt enum + SSIP/MSIP/STIP/MTIP/
  │                         SEIP/MEIP + HW_IP_MASK)
  └── device/
      └── intc/
          ├── aclint.rs
          └── plic.rs
  ```
  LoongArch mirrors the skeleton but only `cpu/mod.rs` and `isa/mod.rs`
  are non-empty stubs today.
- G-6: Preserve git history for every moved file via `git mv`.

- NG-1: No change to `CoreOps`, `DebugOps`, `Device`, `BootConfig`,
  `BootLayout`, `MachineConfig`, `XError`, `XResult`.
- NG-2: No new `Arch` trait with associated types. Per M-001 the seam
  stays as `cfg_if`. Forward reference: a future `archTrait` plan may
  revisit this if a second **live** arch backend appears.
- NG-3: No change to xdb, xlogger, xam, xlib, difftest, am-tests, or
  benchmarks. No source edits in downstream crates.
- NG-4: No change to boot configs, DTS files, `Makefile`, or `make`
  targets.
- NG-5: **Bus-level RISC-V vocabulary not addressed in this iteration.**
  `Bus::aclint_idx`, `Bus::plic_idx`, `Bus::mtime()`,
  `Bus::set_timer_source`, `Bus::set_irq_sink`, and
  `Bus::ssip_flag`/`take_ssip` remain in place. Redesigning the bus ↔
  intc contract is the explicit scope of follow-up plans
  `aclintSplit`, `plicGateway`, `directIrq`
  (MANUAL_REVIEW #2, #5, #6, #7). This is why the Summary says
  MANUAL_REVIEW #3 is only *partially* addressed.
- NG-6: No semantic edits to ACLINT, PLIC, UART, VirtioBlk,
  TestFinisher, RVCore, CSR, MMU, TLB, or any trap/interrupt logic.
  Files are moved and their `use` paths rewritten; behaviour is
  byte-identical.
- NG-7: No MSRV / edition / dependency changes.

[**Architecture**]

Before (today):

```
xcore/src/
├── cpu/
│   ├── mod.rs            (cfg_if → riscv | loongarch)
│   ├── core.rs           (arch-neutral CoreOps)
│   ├── debug.rs          (arch-neutral DebugOps)
│   ├── riscv/            ← arch-specific (flat per-concern)
│   └── loongarch/        ← arch-specific (stub)
├── isa/
│   ├── mod.rs            (cfg_if → riscv | loongarch)
│   ├── instpat/
│   ├── riscv/            ← arch-specific
│   └── loongarch/        ← arch-specific (stub)
└── device/
    ├── mod.rs            (holds RISC-V mip bits — leak)
    ├── bus.rs            (holds aclint_idx/plic_idx — leak)
    ├── intc/
    │   ├── aclint.rs     ← RISC-V end-to-end
    │   └── plic.rs       ← RISC-V end-to-end
    ├── ram.rs
    ├── uart.rs
    ├── test_finisher.rs
    ├── virtio.rs
    └── virtio_blk.rs
```

After (this plan):

```
xcore/src/
├── arch/
│   ├── mod.rs                   (cfg_if → riscv | loongarch;
│   │                             compile_error! if both set)
│   ├── riscv/
│   │   ├── mod.rs               (topic re-exports)
│   │   ├── cpu/                 (moved from cpu/riscv/ core files)
│   │   ├── isa/                 (moved from isa/riscv/)
│   │   ├── inst/                (moved from cpu/riscv/inst*)
│   │   ├── csr/                 (moved from cpu/riscv/csr*)
│   │   ├── mm/                  (moved from cpu/riscv/mm*)
│   │   ├── trap/                (moved from cpu/riscv/trap*;
│   │   │                         mip bits land in interrupt.rs)
│   │   └── device/
│   │       └── intc/
│   │           ├── aclint.rs    (moved from device/intc/aclint.rs)
│   │           └── plic.rs      (moved from device/intc/plic.rs)
│   └── loongarch/
│       ├── mod.rs
│       ├── cpu/                 (moved from cpu/loongarch/)
│       └── isa/                 (moved from isa/loongarch/)
├── cpu/
│   ├── mod.rs                   (cfg_if → arch::selected::cpu +
│   │                             topical re-exports for public API)
│   ├── core.rs                  (unchanged — CoreOps lives here)
│   └── debug.rs                 (unchanged — DebugOps lives here)
├── isa/
│   ├── mod.rs                   (cfg_if → arch::selected::isa)
│   └── instpat/                 (unchanged — pest glue is neutral)
├── device/
│   ├── mod.rs                   (IrqState + neutral mmio_regs!;
│   │                             mip bits re-exported from
│   │                             arch::selected under cfg(riscv))
│   ├── bus.rs                   (unchanged in THIS plan; see NG-5)
│   ├── intc/
│   │   └── mod.rs               (cfg_if → arch::selected::device::intc)
│   ├── ram.rs
│   ├── uart.rs
│   ├── test_finisher.rs
│   ├── virtio.rs
│   └── virtio_blk.rs
├── config/
├── utils/
├── error.rs
└── lib.rs                       (mod arch; + existing mods)
```

Upper-layer contract, unchanged: `cpu/core.rs::CoreOps` plus
`cpu/debug.rs::DebugOps` remain the fine-grained traits the generic
`CPU<Core>` wrapper uses. Per M-001 nothing more coarse-grained (no
`trait Arch`) is added.

Seam invariants:

- Exactly four `cfg_if` blocks span the seam after this plan:
  `arch/mod.rs`, `cpu/mod.rs`, `isa/mod.rs`, `device/intc/mod.rs`. A
  fifth **single-arm** `#[cfg(riscv)] pub use …` lives in
  `device/mod.rs` for mip bit re-export (it is one-armed because
  LoongArch has no mip bits).

[**Invariants**]

- I-1: **Arch-path isolation.** No file under `xcore/src/` outside
  `arch/` may write `use crate::arch::riscv::…` or `use
  crate::arch::loongarch::…` by concrete arch name. Arch access is
  **only** via `crate::arch::selected::…` or the `cfg_if` seams listed
  above.
- I-2: **Vocabulary isolation.** No file under `xcore/src/` outside
  `arch/` and outside the seam allow-list (see V-F-2) may contain
  RISC-V vocabulary strings (`MSIP`, `MTIP`, `MEIP`, `SEIP`, `SSIP`,
  `STIP`, `mtime`, `mtimecmp`, `aclint`, `plic`, `hart`, `RVCore`,
  `Mstatus`, `Mip`, `Sv32`, `Sv39`). The one knowingly-remaining
  `device/bus.rs` violation is tracked against NG-5 and included in an
  explicit allow-list with a follow-up reference; new violations are
  prohibited.
- I-3: **History preserved.** `git log --follow` on any moved file
  (cpu/riscv/trap/handler.rs, isa/riscv/decoder.rs,
  device/intc/aclint.rs, device/intc/plic.rs, etc.) shows its full
  pre-refactor history.
- I-4: **Public API unchanged.** Every item currently exported from
  `crate::cpu::*`, `crate::isa::*`, and `crate::device::*` remains
  exported with the same name, type, and external module path.
  Downstream (`xdb`, `xam`, `xemu` binary) compiles with **zero**
  source changes.
- I-5: **Behaviour unchanged.** `cargo test --workspace`,
  `make cpu-tests-rs`, `make am-tests`, `make linux`, and `make debian`
  produce the same pass/fail and boot artefacts as pre-refactor.
  Difftest vs QEMU and Spike produces zero divergence on the default
  cpu-tests-rs set.
- I-6: **Phase green-bar.** Each of Phases 1..5 ends with a green
  `cargo test --workspace` (Phase 3 also `X_ARCH=loongarch32 cargo
  check` and `X_ARCH=loongarch64 cargo check`).

[**Data Structure**]

No new runtime data types. Only module-layout types change. The only
new "type" is the module alias `arch::selected`:

```rust
// xcore/src/arch/mod.rs
cfg_if::cfg_if! {
    if #[cfg(all(riscv, loongarch))] {
        compile_error!(
            "xcore: both `riscv` and `loongarch` cfgs are set; \
             choose one via X_ARCH."
        );
    } else if #[cfg(riscv)] {
        pub mod riscv;
        pub use self::riscv as selected;
    } else if #[cfg(loongarch)] {
        pub mod loongarch;
        pub use self::loongarch as selected;
    } else {
        compile_error!(
            "xcore: neither `riscv` nor `loongarch` cfg is set; \
             build.rs emits one from X_ARCH."
        );
    }
}
```

`arch/riscv/mod.rs` exposes its topical submodules:

```rust
// xcore/src/arch/riscv/mod.rs
pub mod cpu;        // RVCore, RVCoreContext
pub mod csr;        // CsrFile, CsrAddr, Mip, MStatus, PrivilegeMode
pub mod device;     // device::intc::{aclint::Aclint, plic::Plic}
pub mod inst;       // per-instruction handlers
pub mod isa;        // DECODER, DecodedInst, RVReg
pub mod mm;         // Mmu, Pmp, Tlb, TlbEntry
pub mod trap;       // PendingTrap, TrapCause, Exception, Interrupt,
                    // SSIP/MSIP/.../HW_IP_MASK
```

`arch/riscv/trap/mod.rs` (topical bundling):

```rust
// xcore/src/arch/riscv/trap/mod.rs
pub mod cause;
pub mod exception;
pub mod handler;
pub mod interrupt;

pub use cause::{PendingTrap, TrapCause};
pub use exception::Exception;
pub use interrupt::{
    Interrupt,
    HW_IP_MASK, MEIP, MSIP, MTIP, SEIP, SSIP, STIP,
};
```

`arch/riscv/device/intc/mod.rs`:

```rust
// xcore/src/arch/riscv/device/intc/mod.rs
pub mod aclint;
pub mod plic;
```

`arch/loongarch/mod.rs` (stub skeleton, mirrors topic names but only
cpu/isa are non-empty):

```rust
// xcore/src/arch/loongarch/mod.rs
pub mod cpu;
pub mod isa;
// no trap / csr / mm / inst / device modules yet — LoongArch stub.
```

[**API Surface**]

Public crate API (unchanged — this is I-4):

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

Internal seam shapes (names unchanged, source paths updated):

```rust
// xcore/src/cpu/mod.rs  (after)
cfg_if::cfg_if! {
    if #[cfg(riscv)] {
        pub use crate::arch::riscv::cpu::*;
    } else if #[cfg(loongarch)] {
        pub use crate::arch::loongarch::cpu::*;
    }
}

// xcore/src/isa/mod.rs  (after)
cfg_if::cfg_if! {
    if #[cfg(riscv)] {
        pub use crate::arch::riscv::isa::*;
    } else if #[cfg(loongarch)] {
        pub use crate::arch::loongarch::isa::*;
    }
}

// xcore/src/device/intc/mod.rs  (after)
cfg_if::cfg_if! {
    if #[cfg(riscv)] {
        pub use crate::arch::riscv::device::intc::*;
    }
    // LoongArch: no intc exposed; device/ callers that need one
    // must go through arch::selected and will fail to compile,
    // which is the correct enforcement.
}

// xcore/src/device/mod.rs  (after — mip bit re-export)
#[cfg(riscv)]
pub use crate::arch::riscv::trap::interrupt::{
    HW_IP_MASK, MEIP, MSIP, MTIP, SEIP, SSIP, STIP,
};
```

Intra-arch paths (referenced only inside `arch/riscv/`):

```rust
// xcore/src/arch/riscv/cpu/mod.rs  (after — formerly cpu/riscv/mod.rs)
use crate::{
    config::{CONFIG_MBASE, MachineConfig, Word},
    device::{
        IrqState,                                // still neutral
        bus::Bus,                                // still neutral
        test_finisher::TestFinisher,             // neutral device
        uart::Uart,                              // neutral device
        virtio_blk::VirtioBlk,                   // neutral device
    },
    arch::riscv::{
        device::intc::{aclint::Aclint, plic::Plic},
        trap::interrupt::HW_IP_MASK,
    },
    // … existing imports …
};
```

[**Constraints**]

- C-1: Landable as a chain of five PRs (one per phase) that keeps
  `cargo test --workspace`, `make linux`, and `make debian` green at
  each phase boundary.
- C-2: **No semantic edits** inside moved files beyond import-path
  adjustments needed to compile after relocation. Byte-identical
  behaviour except for `use` statement reordering.
- C-3: **Git history preserved** for every moved file via `git mv`
  (not copy+delete). Confirmed by `git log --follow`.
- C-4: **No MSRV / edition / dependency changes.** `Cargo.toml` of
  `xcore` is unchanged (verified by `diff`).
- C-5: **Seam count bounded.** After the refactor there are exactly
  four `cfg_if` blocks plus one `#[cfg(riscv)]` single-arm re-export
  across `xcore/src/{cpu,isa,device}/` and `xcore/src/arch/mod.rs`.
- C-6: **Phase-local compile.** Phase 2 alone must compile and pass
  tests without Phase 4 having landed (and vice versa for Phase 3 vs
  Phase 4). `device/intc/` keeps its pre-refactor behaviour during
  Phases 1..3 because it is untouched until Phase 4.

---

## Implement

### Execution Flow

[**Main Flow**]

1. **Skeleton.** Create empty `xcore/src/arch/{mod.rs,riscv/,
   loongarch/}`. Add `mod arch;` in `xcore/src/lib.rs`. `arch/mod.rs`
   contains the `cfg_if` block with the two `compile_error!` arms.
   Nothing is re-exported from `selected` yet because the submodules
   are still empty — so at this phase `arch::selected` is valid but
   empty, and `cpu/mod.rs` / `isa/mod.rs` / `device/*` keep their
   current pre-refactor wiring. **Build green.**
2. **RISC-V cpu/isa relocation.**
   - `git mv xemu/xcore/src/cpu/riscv xemu/xcore/src/arch/riscv/cpu`
   - `git mv xemu/xcore/src/isa/riscv xemu/xcore/src/arch/riscv/isa`
   - Split the cpu subtree so topic directories
     (`arch/riscv/{csr,mm,trap,inst}`) are reachable from
     `arch/riscv/mod.rs`. Today these already live under
     `cpu/riscv/{csr,csr.rs,mm,mm.rs,trap,trap.rs,inst,inst.rs}`, so
     the post-move layout becomes `arch/riscv/cpu/{csr,csr.rs,mm,mm.rs,
     trap,trap.rs,inst,inst.rs,context.rs,debug.rs,mod.rs}`. **No file
     is further split within `cpu/`**; the topic directories are
     exposed at `arch/riscv/mod.rs` by re-exporting:
     ```rust
     // arch/riscv/mod.rs  (phase 2)
     pub mod cpu;
     pub mod isa;
     pub use cpu::{csr, inst, mm, trap};
     ```
     This satisfies "topic-organised" at the `arch/riscv/mod.rs`
     level without an extra mechanical move. If Round 01 review
     requires directory-level topic split (i.e. `arch/riscv/csr/`
     as a sibling of `arch/riscv/cpu/`), Phase 2 performs a second
     `git mv` wave to hoist them up; this is reviewer-decidable.
   - Rewire `cpu/mod.rs` and `isa/mod.rs` `cfg_if` blocks to the
     `arch::selected::cpu` / `arch::selected::isa` paths.
   - Fix broken imports inside relocated files:
     - Any `use super::super::CoreOps` / `use super::super::…` that
       crossed into the old `cpu::` root now crosses into `cpu::` still
       (path: `crate::cpu::core::CoreOps`). Unchanged.
     - Any `use crate::cpu::riscv::…` becomes `use
       crate::arch::riscv::cpu::…` **only when the referring file is
       itself inside `arch/riscv/`**. Outside `arch/` this path must
       not appear (I-1).
     - Every `pub(in crate::cpu::riscv)` / `pub(in crate::isa::riscv)`
       is rewritten per the enumerated list below.
   - **`pub(in …)` rewrite list (R-002) — 11 call sites:**
     - `arch/riscv/cpu/trap.rs:21,26,31,35,55`
       (formerly `cpu/riscv/trap.rs`): `pub(in crate::cpu::riscv)` →
       `pub(in crate::arch::riscv::cpu)`.
     - `arch/riscv/cpu/csr.rs:95`: same rewrite.
     - `arch/riscv/cpu/csr/ops.rs:7,16`: same rewrite.
     - `arch/riscv/cpu/mm/tlb.rs:10,58`: same rewrite.
     - `arch/riscv/cpu/mm/mmu.rs:20`: same rewrite.
     - Verification gate: `rg 'pub\(in crate::(cpu|isa)::(riscv|loongarch)' xemu/xcore/src`
       returns **0 hits**. No `pub(in crate::isa::riscv)` sites exist
       today (verified by `rg`), but the rule is still added to the
       Phase checklist for defence in depth.
   - **Build green:** `cargo test --workspace`, `make cpu-tests-rs`,
     `make am-tests`, `make linux`, `make debian`.
3. **LoongArch cpu/isa relocation.**
   - `git mv xemu/xcore/src/cpu/loongarch xemu/xcore/src/arch/loongarch/cpu`
   - `git mv xemu/xcore/src/isa/loongarch xemu/xcore/src/arch/loongarch/isa`
   - Populate `arch/loongarch/mod.rs` with `pub mod cpu; pub mod isa;`.
   - `cpu/mod.rs` / `isa/mod.rs` cfg_if blocks now cover both arms.
   - Apply the `pub(in …)` rewrite rule to any LoongArch site that
     appears in future (none today — verified).
   - **Build green:** both `X_ARCH=riscv32 cargo test -p xcore` and
     `X_ARCH=loongarch32 cargo check -p xcore`,
     `X_ARCH=loongarch64 cargo check -p xcore`.
4. **Device / intc + mip relocation.**
   - `git mv xemu/xcore/src/device/intc/aclint.rs
     xemu/xcore/src/arch/riscv/device/intc/aclint.rs`
   - `git mv xemu/xcore/src/device/intc/plic.rs
     xemu/xcore/src/arch/riscv/device/intc/plic.rs`
   - Create `xemu/xcore/src/arch/riscv/device/intc/mod.rs` with
     `pub mod aclint; pub mod plic;`.
   - Create `xemu/xcore/src/arch/riscv/device/mod.rs` with
     `pub mod intc;`.
   - Add `pub mod device;` to `xemu/xcore/src/arch/riscv/mod.rs`.
   - Rewrite `xemu/xcore/src/device/intc/mod.rs` to a `cfg_if` seam
     (see API Surface).
   - Cut mip bit constants `{SSIP, MSIP, STIP, MTIP, SEIP, MEIP,
     HW_IP_MASK}` from `xemu/xcore/src/device/mod.rs` (lines 55–72)
     and paste into `xemu/xcore/src/arch/riscv/cpu/trap/interrupt.rs`
     (append to the existing `Interrupt` enum module). `device/mod.rs`
     gets `#[cfg(riscv)] pub use crate::arch::riscv::trap::interrupt::{…};`.
     (Note: since in Phase 2 `arch/riscv/mod.rs` re-exports
     `cpu::trap` as `trap`, the canonical neutral path is
     `crate::arch::riscv::trap::interrupt::…`.)
   - Rewrite imports in the (now-relocated) intc files that referred
     to `device::{MSIP, MTIP, MEIP, SEIP}`:
     - `arch/riscv/device/intc/aclint.rs:13`:
       `use crate::device::{Device, IrqState, MSIP, MTIP, mmio_regs};`
       → `use crate::{
            arch::riscv::trap::interrupt::{MSIP, MTIP},
            device::{Device, IrqState, mmio_regs},
          };`
     - `arch/riscv/device/intc/plic.rs:4-8`: analogous rewrite for
       `MEIP, SEIP`.
   - Rewrite `arch/riscv/cpu/mod.rs` device imports (R-007):
     - `use crate::device::intc::{aclint::Aclint, plic::Plic};` →
       `use crate::arch::riscv::device::intc::{aclint::Aclint,
       plic::Plic};`.
     - Uart, TestFinisher, VirtioBlk imports stay at `crate::device::…`
       — they remain arch-neutral devices used by the RISC-V platform.
     - `HW_IP_MASK` reference in `sync_interrupts` switches from
       `crate::device::HW_IP_MASK` to
       `crate::arch::riscv::trap::interrupt::HW_IP_MASK`.
     - MMIO addresses (0x0200_0000, 0x0C00_0000, 0x1000_0000, …) stay
       here — they are RISC-V platform policy.
   - **Build green** across the full test + boot matrix. Vocabulary
     grep (V-F-2) passes.
5. **Seam hardening + docs.**
   - Add the `compile_error!` arms to `arch/mod.rs` (already present
     from Phase 1; confirm).
   - Update `lib.rs` rustdoc header (currently claims "cycle-accurate
     **RISC-V** emulator core library" — keep as-is since RISC-V is
     the only live backend, but replace "The crate is ISA-generic at
     compile time via `cfg(riscv)` / `cfg(loongarch)`" with "The crate
     is ISA-generic at compile time: each active arch lives in
     `arch/<name>/` and is selected via `X_ARCH`.").
   - Update `docs/DEV.md` architecture note if it references old paths
     (`cpu/riscv/…`, `isa/riscv/…`, `device/intc/…`).
   - No README change; it speaks at a higher level.
   - **Final matrix green:** `make fmt && make clippy && make run &&
     make test`, plus `X_ARCH=loongarch{32,64} cargo check -p xcore`,
     plus V-F-2 / V-F-3 / V-F-4 gates.

[**Failure Flow**]

1. **Import fixup cascade fails in Phase 2.** After `git mv`, rustc
   reports unresolved imports and/or the `pub(in crate::cpu::riscv)`
   E0742 errors. Fix by walking the compiler error list top-to-bottom
   and applying the rewrite rules in Main Flow step 2. If a file needs
   a concrete `crate::arch::riscv::…` import *outside* `arch/`, that
   is an I-1 violation: the fix is to push the call-site into
   `arch/riscv/` or re-expose through `cpu/mod.rs` / `isa/mod.rs`
   cfg_if — never paper over with a direct arch import outside the
   seam.
2. **V-F-2 vocabulary grep fails after Phase 4.** For each hit, decide:
   (a) does it belong in the allow-list (seam files + tracked NG-5
   residuals in `device/bus.rs`)? Add with an explicit in-file
   `// TODO: archBus follow-up` marker if so. (b) Otherwise relocate
   the offending code into `arch/riscv/`. New unreferenced hits are
   always (b).
3. **`cargo check` fails on `X_ARCH=loongarch32`.** A RISC-V-only
   symbol leaked into the neutral layer during Phase 2 or Phase 3.
   Relocate into `arch/riscv/` or gate with `#[cfg(riscv)]`. Do not
   fake LoongArch bindings (per R-006).
4. **`git log --follow` shows no pre-refactor history on a moved
   file.** The move was done as copy+delete instead of `git mv`. Redo
   the move via `git mv` and amend the phase commit.
5. **Phase 4 breaks `make linux` / `make debian`.** The most likely
   cause is a mistyped import in one of the relocated intc files or a
   missed mip-bit reference in `arch/riscv/cpu/mod.rs::sync_interrupts`.
   Bisect by running `cargo test --workspace -- riscv::trap` first
   (catches mip wiring), then `cargo test -p xcore intc` (catches
   intc wiring).

[**State Transition**]

- **S0** (pre-refactor): four sibling arch dirs
  (`cpu/riscv`, `cpu/loongarch`, `isa/riscv`, `isa/loongarch`);
  `device/intc/aclint.rs`, `device/intc/plic.rs`, and mip bits in
  `device/mod.rs`.
- **S1** (after Phase 1): `arch/` exists with `mod.rs` + two empty
  submodules. Nothing wired yet. `cargo test --workspace` green
  under the default (`X_ARCH=riscv32`) config.
- **S2** (after Phase 2): RISC-V cpu/isa subtree moved to
  `arch/riscv/{cpu,isa}/`; `cpu/mod.rs` / `isa/mod.rs` rewired;
  all 11 `pub(in …)` sites rewritten. `cargo test --workspace`,
  `make cpu-tests-rs`, `make am-tests`, `make linux`, `make debian`
  green under RISC-V.
- **S3** (after Phase 3): LoongArch cpu/isa subtree moved.
  `X_ARCH=loongarch32 cargo check -p xcore` **and**
  `X_ARCH=loongarch64 cargo check -p xcore` succeed. (LoongArch
  remains a stub; the acceptance bar is strictly "cargo check" per
  R-008.)
- **S4** (after Phase 4): ACLINT / PLIC relocated under
  `arch/riscv/device/intc/`; `device/intc/mod.rs` is the seam.
  mip bit constants live in `arch/riscv/trap/interrupt.rs`.
  `device/mod.rs` re-exports them under `#[cfg(riscv)]`.
  V-F-2 vocabulary grep passes (allow-list covers the tracked NG-5
  bus-field residuals with explicit `// TODO: archBus` markers).
  `arch/riscv/cpu/mod.rs` uses intra-arch `crate::arch::riscv::…`
  paths for intc + mip bits. Full matrix green.
- **S5** (after Phase 5): `compile_error!` gates confirmed; rustdoc
  updated; `grep -r 'cpu::riscv\|isa::riscv' xemu/xcore/src`
  returns only seam-file hits (`cpu/mod.rs`, `isa/mod.rs`). Ready
  to merge.

### Implementation Plan

Per TR-4 the refactor lands as a chain of five PRs, one per phase.
Proposed PR titles:

- **PR 1 — `refactor(xcore): introduce arch/ skeleton`** (Phase 1)
- **PR 2 — `refactor(xcore): relocate RISC-V cpu/isa under arch/riscv`**
  (Phase 2, includes the 11 `pub(in …)` rewrites)
- **PR 3 — `refactor(xcore): relocate LoongArch stubs under arch/loongarch`**
  (Phase 3)
- **PR 4 — `refactor(xcore): move ACLINT/PLIC and mip bits under arch/riscv`**
  (Phase 4)
- **PR 5 — `docs(xcore): arch/ module layout + compile_error seam gates`**
  (Phase 5)

[**Phase 1 — Skeleton**]

- Add `xemu/xcore/src/arch/mod.rs` with the `cfg_if` block (see
  Data Structure) including both `compile_error!` arms.
- Add empty `xemu/xcore/src/arch/riscv/mod.rs` and
  `xemu/xcore/src/arch/loongarch/mod.rs` (each is a `//!` rustdoc
  placeholder).
- Add `mod arch;` in `xemu/xcore/src/lib.rs` alongside the existing
  `mod cpu;` and `mod isa;`.
- Verify: `cargo build -p xcore` and `cargo test --workspace` green
  under default (`X_ARCH=riscv32`).

[**Phase 2 — RISC-V cpu + isa**]

- `git mv xemu/xcore/src/cpu/riscv xemu/xcore/src/arch/riscv/cpu`
- `git mv xemu/xcore/src/isa/riscv xemu/xcore/src/arch/riscv/isa`
- Populate `arch/riscv/mod.rs`: `pub mod cpu; pub mod isa; pub use
  cpu::{csr, inst, mm, trap};`. (The `pub use` hoists the existing
  nested directories so other arch-internal files can reach
  `crate::arch::riscv::trap::interrupt::…` — the path that Phase 4
  will rely on.)
- Rewrite `cpu/mod.rs` cfg_if `riscv` arm to `pub use
  crate::arch::riscv::cpu::*;`.
- Rewrite `isa/mod.rs` cfg_if `riscv` arm to `pub use
  crate::arch::riscv::isa::*;`.
- Fix intra-arch imports (`super::` hops that broke because of the
  depth change) — mechanical path adjustment only. Most imports
  continue to resolve because they go through the arch-neutral root
  (`crate::cpu::core::CoreOps`, `crate::device::…`, `crate::config::…`).
- **Rewrite all 11 `pub(in crate::cpu::riscv)` declarations** (listed
  above) to `pub(in crate::arch::riscv::cpu)`. Verify:
  `rg 'pub\(in crate::(cpu|isa)::(riscv|loongarch))' xemu/xcore/src`
  returns 0.
- Verify: `cargo test --workspace`, `make cpu-tests-rs`, `make
  am-tests`, `make linux`, `make debian` green. V-F-3 passes on at
  least one moved cpu file and one moved isa file.

[**Phase 3 — LoongArch cpu + isa**]

- `git mv xemu/xcore/src/cpu/loongarch xemu/xcore/src/arch/loongarch/cpu`
- `git mv xemu/xcore/src/isa/loongarch xemu/xcore/src/arch/loongarch/isa`
- Populate `arch/loongarch/mod.rs`: `pub mod cpu; pub mod isa;`.
- Add `else if #[cfg(loongarch)]` arms in `cpu/mod.rs` / `isa/mod.rs`
  pointing at `crate::arch::loongarch::{cpu,isa}::*`.
- Apply the `pub(in …)` rewrite rule — today no matches, but the
  phase checklist still runs the grep gate.
- Verify: `X_ARCH=loongarch32 cargo check -p xcore` **and**
  `X_ARCH=loongarch64 cargo check -p xcore` succeed. Default
  `X_ARCH=riscv32 cargo test --workspace` still green.

[**Phase 4 — ACLINT / PLIC + mip bits**]

- Create directory `xemu/xcore/src/arch/riscv/device/intc/`.
- `git mv xemu/xcore/src/device/intc/aclint.rs
  xemu/xcore/src/arch/riscv/device/intc/aclint.rs`
- `git mv xemu/xcore/src/device/intc/plic.rs
  xemu/xcore/src/arch/riscv/device/intc/plic.rs`
- Create `arch/riscv/device/intc/mod.rs` = `pub mod aclint; pub mod plic;`.
- Create `arch/riscv/device/mod.rs` = `pub mod intc;`.
- Extend `arch/riscv/mod.rs`: add `pub mod device;`.
- Rewrite `xemu/xcore/src/device/intc/mod.rs` to:
  ```rust
  //! Interrupt controllers (arch-specific).
  cfg_if::cfg_if! {
      if #[cfg(riscv)] {
          pub use crate::arch::riscv::device::intc::*;
      }
  }
  ```
- Cut mip bit constants `SSIP/MSIP/STIP/MTIP/SEIP/MEIP/HW_IP_MASK`
  from `device/mod.rs` (lines 55–72). Paste into the existing
  `arch/riscv/cpu/trap/interrupt.rs` module. Replace with:
  ```rust
  // xcore/src/device/mod.rs  (after)
  #[cfg(riscv)]
  pub use crate::arch::riscv::trap::interrupt::{
      HW_IP_MASK, MEIP, MSIP, MTIP, SEIP, SSIP, STIP,
  };
  ```
- Rewrite `arch/riscv/device/intc/aclint.rs` imports:
  `use crate::device::{Device, IrqState, MSIP, MTIP, mmio_regs};` →
  `use crate::{arch::riscv::trap::interrupt::{MSIP, MTIP},
            device::{Device, IrqState, mmio_regs}};`.
- Rewrite `arch/riscv/device/intc/plic.rs` imports analogously for
  `MEIP, SEIP`.
- Rewrite `arch/riscv/cpu/mod.rs` (formerly `cpu/riscv/mod.rs`):
  - `use crate::device::intc::{aclint::Aclint, plic::Plic};` →
    `use crate::arch::riscv::device::intc::{aclint::Aclint,
    plic::Plic};`.
  - `use crate::device::HW_IP_MASK;` → `use
    crate::arch::riscv::trap::interrupt::HW_IP_MASK;`.
- Add `mmio_regs` to `device::mod` re-export so the relocated
  `aclint.rs` still sees it (`pub(crate) use mmio_regs;` already
  exists; no change). Confirm.
- Verify: full test + boot matrix green. V-F-2 vocabulary grep
  passes with the `device/bus.rs` NG-5 allow-list entries.
- V-F-3 on `arch/riscv/device/intc/aclint.rs` and
  `arch/riscv/device/intc/plic.rs` shows pre-refactor history.

[**Phase 5 — Seam hardening + docs**]

- Confirm `arch/mod.rs` already contains both `compile_error!` arms
  from Phase 1.
- Update `xcore/src/lib.rs` rustdoc (line 14–18): replace the
  `cfg(riscv) / cfg(loongarch)` mention with a reference to the new
  `arch/` module.
- Update `docs/DEV.md` architecture note if it mentions old paths
  `cpu/riscv/…`, `isa/riscv/…`, `device/intc/…`. (Check first —
  only touch if stale.)
- Run `make fmt && make clippy && make run && make test` per
  `AGENTS.md` Development Standards.
- Append a `// TODO: archBus follow-up` marker at each NG-5
  residual site in `device/bus.rs` (fields `aclint_idx`, `plic_idx`,
  and methods `mtime`, `set_timer_source`, `set_irq_sink`,
  `ssip_flag`, `take_ssip`). This makes the V-F-2 allow-list
  self-documenting.
- Verify final full matrix green including the four
  `X_ARCH={riscv32,riscv64,loongarch32,loongarch64} cargo check -p
  xcore` invocations.

---

## Trade-offs

T-1 is **closed** by M-001 (Option A). Remaining open trade-offs:

- **T-2: Back-compat re-exports at `cpu::*` / `isa::*` / `device::*`
  vs hard cut.**
  - Option A (this plan): `cpu/mod.rs`, `isa/mod.rs`, and
    `device/intc/mod.rs` `pub use` verbatim what their old
    per-concrete-arch subtrees exposed. Downstream compiles unchanged.
    TR-2 accepted → Option A.
  - Option B: rename downstream imports to say `arch::riscv::…`
    directly.
  - Recommendation: **Option A**, confirmed by TR-2. This plan is
    scoped to `xcore/src/`; downstream cleanup is out of scope.

- **T-3: Where does `IrqState` live?**
  - Option A (this plan): `IrqState` stays in `device/mod.rs`; the
    bit positions (MSIP/MTIP/MEIP/SEIP/…) live in
    `arch/riscv/trap/interrupt.rs`. Storage is arch-neutral; bit
    positions are arch-specific.
  - Option B: move `IrqState` entirely into `arch/riscv/`.
  - Recommendation: **Option A**, confirmed by TR-3. The
    MANUAL_REVIEW #5/#6 follow-up (async IRQ + external-device ↔
    PLIC direct) will reshape `IrqState`; moving it now would
    collide.

- **T-4: Single PR vs phased PRs.**
  - Option A: one mega-PR.
  - Option B: five phased PRs (Phase 1..5).
  - Recommendation: **Option B**, per TR-4. Per-phase review is
    tractable; four `git mv` walls in one diff are not.

- **T-5 (new): directory-level topic split within `arch/riscv/`.**
  - Option A (this plan): nested topics inside `arch/riscv/cpu/`
    (`cpu/csr/`, `cpu/mm/`, `cpu/trap/`, `cpu/inst/`), re-exported at
    `arch/riscv/mod.rs` as `pub use cpu::{csr, inst, mm, trap};`.
    **Pro:** one `git mv` per subtree, minimal churn, history
    preserved at full depth. **Con:** physical layout is still
    "`cpu` is deeper than the topic names suggest". The upper-layer
    contract is unaffected because callers go through `arch/riscv/
    mod.rs`'s `pub use`.
  - Option B: a second wave of `git mv` that hoists each topic to
    the arch root (`arch/riscv/csr/`, `arch/riscv/mm/`,
    `arch/riscv/trap/`, `arch/riscv/inst/`), leaving `arch/riscv/
    cpu/` with just `{context.rs, debug.rs, mod.rs}`. **Pro:** flat
    topic layout matches M-002's letter more literally. **Con:**
    doubles the mechanical diff and introduces cross-topic
    `super::super::…` hops that aren't there today (e.g. `trap`
    currently uses `use crate::cpu::riscv::RVCore`, which would
    become `use crate::arch::riscv::cpu::RVCore`).
  - Recommendation: **Option A** for this iteration — M-002's intent
    ("every file in `arch/` abstracted by topic, upper layer sees
    only the topical interface") is satisfied by the `pub use` at
    `arch/riscv/mod.rs`, which is what the upper-layer seam actually
    imports. Reviewer may escalate to Option B if the physical
    layout is considered binding; that is a straightforward
    follow-up `git mv` wave that does not change behaviour.

---

## Validation

[**Unit Tests**]

- V-UT-1: All existing unit tests under `cpu/riscv/**`,
  `isa/riscv/**`, `device/intc/**`, `device/**` continue to pass at
  their new paths under `arch/riscv/**` and `arch/riscv/device/**`.
  **No test source edits.** Test count pre = test count post.
- V-UT-2: A behavioural seam-liveness test replaces the brittle
  round-00 `type_name` canary. In `arch/mod.rs`:
  ```rust
  // Compile-time seam-liveness: the selected arch must expose a
  // `cpu::Core` type that the upper layer can name.
  const _: fn() = || {
      #[cfg(riscv)]
      let _: fn() -> crate::arch::selected::cpu::Core =
          crate::arch::selected::cpu::Core::new;
  };

  #[cfg(test)]
  mod tests {
      #[test]
      #[cfg(riscv)]
      fn selected_core_boots_at_reset_vector() {
          use crate::cpu::RESET_VECTOR;
          use memory_addr::VirtAddr;
          let mut cpu = crate::cpu::CPU::new(
              crate::arch::selected::cpu::Core::new(),
              crate::config::BootLayout {
                  fdt_addr: crate::config::CONFIG_MBASE
                      + crate::config::CONFIG_MSIZE - 0x10_0000,
              },
          );
          cpu.reset().unwrap();
          assert_eq!(cpu.pc(), RESET_VECTOR);
      }
  }
  ```
  Binds the seam to an observable architectural default rather than
  a type-name spelling.

[**Integration Tests**]

- V-IT-1: `cargo test --workspace` at each of Phase 1..5 boundaries.
- V-IT-2: `make cpu-tests-rs` pass at Phase 2 and again at Phase 4.
- V-IT-3: `make am-tests` (UART, ACLINT, PLIC, CSR, trap,
  interrupts, float, keyboard) pass at Phase 4 (ACLINT/PLIC
  relocation must not perturb am-tests).
- V-IT-4: `make linux` boots to the interactive shell; `make debian`
  boots and runs Python3 (per `docs/DEV.md` acceptance). Wall-clock
  within ±5% of a pre-refactor baseline (measured with `DEBUG=n`).

[**Failure / Robustness Validation**]

- V-F-1: `X_ARCH=loongarch32 cargo check -p xcore` **and**
  `X_ARCH=loongarch64 cargo check -p xcore` succeed at Phase 3 and
  stay green through Phases 4–5. This flushes any RISC-V-only
  symbol that leaked into the neutral layer. (Corrects round-00
  R-003 — uses the `X_ARCH` env var per `xemu/xcore/build.rs:19–31`,
  not a non-existent `--features loongarch`.)
- V-F-2: **Vocabulary allow-list grep** (corrects round-00 R-004).
  ```
  rg -nP --type rust \
     '\b(MSIP|MTIP|MEIP|SEIP|SSIP|STIP|mtime|mtimecmp|aclint|plic|hart|RVCore|Mstatus|Mip|Sv32|Sv39)\b' \
     xemu/xcore/src
  ```
  Every hit must come from the allow-list:
    - `xemu/xcore/src/arch/**`
    - `xemu/xcore/src/cpu/mod.rs` (seam — the `cfg_if` block)
    - `xemu/xcore/src/isa/mod.rs` (seam)
    - `xemu/xcore/src/device/intc/mod.rs` (seam)
    - `xemu/xcore/src/device/mod.rs` (seam — one `#[cfg(riscv)]`
      `pub use` line for mip bits)
    - `xemu/xcore/src/device/bus.rs` — **NG-5 residual**.
      Tracked hits at the `aclint_idx`, `plic_idx`, `mtime`,
      `set_timer_source`, `set_irq_sink`, `ssip_flag`, `take_ssip`
      declarations; each decorated with `// TODO: archBus follow-up`
      in Phase 5.
  Any hit outside the allow-list is a V-F-2 failure.
- V-F-3: `git log --follow -- xemu/xcore/src/arch/riscv/cpu/mod.rs`
  shows history back through `cpu/riscv/mod.rs`. Same for
  `arch/riscv/isa/decoder.rs` (back to `isa/riscv/decoder.rs`),
  `arch/riscv/device/intc/aclint.rs` (back to
  `device/intc/aclint.rs`), and `arch/riscv/device/intc/plic.rs`
  (back to `device/intc/plic.rs`).
- V-F-4: Difftest vs QEMU and Spike on the default `cpu-tests-rs`
  set — zero divergence after Phases 2 and 4.
- V-F-5: `cargo tree -p xcore` diff is empty (C-4 / NG-7).

[**Edge Case Validation**]

- V-E-1 (retargeted per R-003): **both-cfgs-set gate.** Confirm
  `xcore/src/arch/mod.rs` contains the `compile_error!(all(riscv,
  loongarch))` arm. Manually run
  `RUSTFLAGS='--cfg riscv --cfg loongarch' cargo check -p xcore`
  and assert it fails with the expected `compile_error!` message.
  (`build.rs` cannot emit both cfgs from one `X_ARCH` value, so the
  runnable scenario is the `RUSTFLAGS` manual injection; the plan
  commits to the gate being in place, not to any Make target.)
- V-E-2: **Re-export cycle check.** `cargo build -p xcore` emits
  no `ambiguous_glob_reexports` or `unused_imports` warnings under
  the new seam wiring.
- V-E-3: **Downstream source-compat.** `xdb`, `xam`, and the
  `xemu` binary compile unchanged against the refactored `xcore`.
  (Protects I-4.)
- V-E-4: **Missing arch cfg.** Manually run
  `RUSTFLAGS='--cfg foo' X_ARCH= cargo check -p xcore`
  (i.e. neither cfg emitted). Expect a compile_error! from the
  `else` arm of `arch/mod.rs`. Sanity-check that `build.rs`'s
  default (`riscv32` when `X_ARCH` is unset) still kicks in on a
  clean invocation — `cargo check -p xcore` without env vars must
  succeed.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (arch/ exists; cpu/isa/arch-device under it) | V-UT-1, V-F-3 |
| G-2 (cfg_if seams only; no RISC-V paths outside arch/ except seams) | V-F-1, V-F-2 |
| G-3 (mip bits move to arch/riscv/trap/interrupt.rs; gated re-export) | V-F-2, V-IT-3 |
| G-4 (ACLINT/PLIC move under arch/riscv/device/intc/) | V-F-2, V-F-3, V-IT-3 |
| G-5 (arch/ is topic-organised per M-002) | V-UT-1 (no test edits — topic re-exports work), V-F-2 |
| G-6 (git history preserved) | V-F-3 |
| I-1 (no `crate::arch::riscv::…` outside arch/ or seams) | V-F-2 (vocabulary grep is stronger than a path grep) |
| I-2 (no RISC-V vocabulary outside arch/ or seams or NG-5) | V-F-2 |
| I-3 (git history) | V-F-3 |
| I-4 (public API unchanged; downstream compiles unchanged) | V-E-3 |
| I-5 (behaviour unchanged) | V-IT-1, V-IT-2, V-IT-3, V-IT-4, V-F-4 |
| I-6 (phase green-bar) | V-IT-1 at each phase; V-F-1 at Phase 3+ |
| C-1 (landable in five PRs; test + boot green per phase) | V-IT-1..V-IT-4 at each phase boundary |
| C-2 (no semantic edits in moved files) | V-UT-1 (no test edits); V-IT-2 / V-IT-4 (byte-identical behaviour) |
| C-3 (git mv not copy+delete) | V-F-3 |
| C-4 (no Cargo.toml / MSRV / dep changes) | V-F-5 (`cargo tree` diff empty); manual `git diff Cargo.toml` empty |
| C-5 (seam count bounded) | Phase 5 checklist + V-F-2 allow-list is exhaustive |
| C-6 (phase-local compile) | V-IT-1 per phase |
| M-001 (keep cfg-if) | V-F-1 + V-F-2 both reference cfg_if-only seams; no `trait Arch` introduced (plan constraint, verifiable by diff) |
| M-002 (rename irq_bits → irq + topic-organised arch/) | V-F-2 + file layout enumerated in G-5; no `irq_bits.rs` in final tree |
