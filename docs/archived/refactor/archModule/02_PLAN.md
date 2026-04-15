# `archModule` PLAN `02`

> Status: Draft
> Feature: `archModule`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `01_MASTER.md`

---

## Summary

Round 02 replaces round-01's `cfg_if` re-export seam with a **trait-dispatch
contract**: the upper layer (`cpu/`, `isa/`, `device/`) owns fine-grained
per-concern traits; every arch-specific behaviour lives as an `impl` inside
`arch/<name>/` under a **flat topic layout** (`arch/riscv/{cpu,csr,trap,mm,
inst,isa,device}`); and each upper-layer module picks its concrete backend
via a single-line `#[cfg(riscv)] pub type … = crate::arch::riscv::…;` alias
— no `arch::selected`, no `cfg_if` trees, no re-exported types. `build.rs`
remains the sole source of `cfg(riscv)` / `cfg(loongarch)` / `cfg(isa32)` /
`cfg(isa64)` emission and the sole gate against unknown `X_ARCH`; the plan
adds no `compile_error!` canaries. MANUAL_REVIEW #4 is fully addressed;
MANUAL_REVIEW #3 is partially addressed (bus-level field residuals queued
under `aclintSplit` / `plicGateway` / `directIrq`).

Residual risks (3 lines): (1) the trait-surface additions beyond
`CoreOps` / `DebugOps` (i.e. `TrapIntake`, `InterruptSink`, `IntcSet`)
are minimal but non-zero — a reviewer may judge them over-abstract;
(2) Phase 2 gains a 2a/2b split for the flat hoist, slightly growing PR
count; (3) LoongArch coverage remains a stub and this refactor neither
populates nor exercises the second backend.

## Log

[**Feature Introduce**]

- **Trait-dispatch contract (M-004).** `cpu/core.rs` keeps `CoreOps` /
  `DebugOps`. `device/mod.rs` gains `InterruptSink` (neutral trait the
  existing `IrqState` implements — callers assert arch-opaque `u64` bit
  values obtained from an associated `IntcBits` const set owned by the
  arch impl). `device/intc/mod.rs` gains `IntcSet` (neutral trait for a
  platform's interrupt-controller bundle — returns `&mut dyn Device`
  handles by role, hiding ACLINT/PLIC concreteness). `cpu/core.rs`
  gains `TrapIntake` (neutral edge/level-input contract that
  `sync_interrupts` consumes, so the upper layer no longer imports mip
  bit constants). `isa/mod.rs` keeps `IMG` and adds nothing — the
  decoder stays arch-private behind the type alias.
- **Flat arch layout (R-010, 00-M-002).** `arch/riscv/` contains
  topic-level directories at the root: `cpu/`, `csr/`, `mm/`, `trap/`,
  `inst/`, `isa/`, `device/`. The only files under `arch/riscv/cpu/`
  are those that belong to the "CPU core" topic itself (`context.rs`,
  `debug.rs`, the `RVCore` `mod.rs`). All other topics are siblings.
- **Direct-path seam (M-001).** Upper-layer modules use a single
  `#[cfg(riscv)] pub type Core = crate::arch::riscv::cpu::RVCore;`-style
  alias to name the concrete implementor. No module named `selected`
  anywhere. Callers outside `cpu/` / `isa/` / `device/` continue to
  write `crate::cpu::Core`, `crate::isa::IMG`, `crate::device::intc::Intc`
  — unchanged public surface.
- **Build-script duty (M-003).** `build.rs` is authoritative. The plan
  adds no `compile_error!(all(riscv, loongarch))`, no `RUSTFLAGS`
  canary, and no arch-validity assertion in-source. Validation reduces
  to `X_ARCH={riscv32|riscv64|loongarch32|loongarch64} cargo check -p
  xcore`.
- **Visibility lift is explicit (R-011).** Flat layout removes the
  round-01 `pub use cpu::{csr,inst,mm,trap}` hoist that collided with
  `mod inst;` (private) and `pub(crate) mod mm;`. Post-hoist, each topic
  lives as `pub mod <topic>;` directly under `arch/riscv/mod.rs`.

[**Review Adjustments**]

- **R-010 (CRITICAL)** adopted: **flat topic layout** is the sole
  canonical layout. Every Phase, Data Structure, API Surface, and
  Validation reference uses paths of the form `arch/riscv/<topic>/…`.
- **R-011 (HIGH)** resolved by R-010. No top-level `pub use` hoist is
  needed; each topic is declared directly under `arch/riscv/mod.rs`
  with its original visibility carried over (`pub mod csr;`, `pub(crate)
  mod mm;`, `mod inst;`). The mechanical phase relocates topic
  directories with `git mv` and changes no visibility.
- **R-012 (HIGH)** resolved by R-010: the canonical mip-bit paste target
  is `arch/riscv/trap/interrupt.rs`, referenced identically in Summary,
  Data Structure, API Surface, Execution Flow, and Phase 4.
- **R-013 (MEDIUM)** adopted: V-UT-2 is removed. Seam boot behaviour is
  already covered by `cpu::tests::cpu_reset_sets_pc_to_reset_vector`;
  no duplicate runtime test is added. The remaining compile-time seam
  check collapses into V-UT-1's "every existing test still compiles
  and passes" property.
- **R-014 (MEDIUM)** adopted: V-F-2 becomes a **structural invariant**
  I-1 expressed as a `cargo test` that walks `xemu/xcore/src/` outside
  `arch/` and asserts the vocabulary allow-list at **symbol granularity
  per allow-listed file** (not whole-file allow). The `device/bus.rs`
  NG-5 residuals are pinned to a fixed set of identifier literals;
  any new occurrence fails.
- **R-015 (LOW)** adopted: S2's green-bar phrasing is made concrete —
  "`cargo test -p xcore` and `make linux` pass; `make debian` boots to
  Python3; the intc seam is unexercised until Phase 4" — rather than a
  blanket "green bar" claim.

[**Master Compliance**]

- **M-001 (01_MASTER — drop `selected`).** Applied. No identifier named
  `selected` appears anywhere. Each upper-layer module names its arch
  implementor through a direct `#[cfg]`-gated `pub type` alias:
  `crate::cpu::Core = crate::arch::riscv::cpu::RVCore`,
  `crate::isa::Decoder = crate::arch::riscv::isa::RvDecoder`, etc.
  The exact alias list is in API Surface.
- **M-002 (01_MASTER — clean, concise, elegant).** Applied. This plan
  is shorter than `01_PLAN.md` (single canonical layout; no
  `compile_error!` paragraph; no duplicate paste-target language; T-1
  and T-5 removed; validation consolidated). Code snippets show the
  minimal seam shape only.
- **M-003 (01_MASTER — no redundant arch check).** Applied. No
  `compile_error!(all(riscv, loongarch))`, no `compile_error!(not
  any(riscv, loongarch))`, no `RUSTFLAGS` manual-injection scenario.
  `build.rs` is authoritative; round-01 V-E-1 is removed.
- **M-004 CRITICAL (01_MASTER — trait-dispatch contract).** Applied.
  `cpu/core.rs` owns `CoreOps`, `DebugOps`, `TrapIntake`.
  `device/mod.rs` owns `InterruptSink` (with associated `type Bits`
  and `fn set(&self, b: Self::Bits)` / `fn clear(&self, b: Self::Bits)` /
  `fn load(&self) -> u64`); `device/intc/mod.rs` owns `IntcSet`.
  `isa/mod.rs` keeps `IMG`. Implementations (`RVCore`, `Aclint`, `Plic`,
  the RISC-V `Intc` bundle, the RISC-V `Decoder`) live under
  `arch/riscv/`. Upper-layer modules contain exactly one `#[cfg]`
  line each — the type alias — and no `use crate::arch::…` paths leak
  outside the seam.
- **M-001 (00_MASTER — keep cfg-if-style seam, no global `trait Arch`).**
  Applied and reconciled with 01-M-004. **No global** `trait Arch {
  type Word; … }` is introduced. The seam mechanism shifts from a
  `cfg_if!` re-export block to a single `#[cfg]`-gated `pub type`
  alias per upper-layer module — functionally identical (one-line
  per-seam cfg switch), conceptually cleaner (direct path, per
  01-M-001). Fine-grained per-concern traits (`CoreOps`,
  `TrapIntake`, `InterruptSink`, `IntcSet`) live in the upper layer
  per 01-M-004; no coarse `Arch` trait crosses boundaries.
- **M-002 (00_MASTER — topic-organised arch/ ; no `irq_bits.rs`).**
  Applied strictly. `arch/riscv/` is physically flat at topic level.
  No file named `irq_bits.rs` exists; mip bit constants live in
  `arch/riscv/trap/interrupt.rs` alongside the `Interrupt` enum.

### Changes from Previous Round

[**Added**]

- **Fine-grained upper-layer traits**: `TrapIntake` (in `cpu/core.rs`),
  `InterruptSink` (in `device/mod.rs`), `IntcSet` (in `device/intc/mod.rs`).
  They are the per-concern dispatch surface required by M-004.
- **Direct `pub type` aliases** at each upper-layer seam (`cpu/mod.rs`,
  `isa/mod.rs`, `device/intc/mod.rs`, `device/mod.rs` mip-bits import
  site). Replaces the round-01 `cfg_if!` blocks and the `selected`
  alias.
- **Flat-layout Phase 2b** (`git mv` hoist of `csr`, `mm`, `trap`,
  `inst` to `arch/riscv/<topic>/`) and a Phase 2c cross-topic import
  fixup step.
- **I-1 structural test** (`tests/arch_isolation.rs`) — see Validation
  V-UT-1 — that encodes R-014's line-level allow-list in code so any
  future `MSIP` / `aclint` / `Mstatus` reference outside the seam
  fails CI.

[**Changed**]

- Seam mechanism: `cfg_if!` block → single `#[cfg(riscv)] pub type …`.
  One `#[cfg]` line per seam instead of a four-line `cfg_if!` tree.
- `device/mod.rs` loses its RISC-V mip bit constants and its re-export;
  instead it exposes the neutral `InterruptSink` trait. Arch-side
  `impl InterruptSink for IrqState` (gated `#[cfg(riscv)]` and defined
  inside `arch/riscv/trap/interrupt.rs`) supplies the bit vocabulary.
- `arch/riscv/cpu/mod.rs`'s `sync_interrupts` stops importing raw mip
  constants from `crate::device::…`; it calls through the new
  `TrapIntake` contract on `self` (arch-local), keeping the bit layout
  encapsulated.
- Phase 2 splits into 2a (relocate), 2b (flat hoist), 2c (cross-topic
  imports + visibility-preserving declarations).
- Validation dropped duplicate V-UT-2 (R-013). V-F-2 replaced by a
  compile-time / `cargo test`-time structural check (R-014). V-E-1
  removed (R-015 / M-003).

[**Removed**]

- `selected` alias / identifier (M-001).
- `cfg_if!` blocks at `arch/mod.rs`, `cpu/mod.rs`, `isa/mod.rs`,
  `device/intc/mod.rs` (replaced by single `#[cfg]`-gated `pub type`).
- Nested `arch/riscv/cpu/{csr,mm,trap,inst}` layout (R-010).
- `pub use cpu::{csr,inst,mm,trap}` hoist at `arch/riscv/mod.rs`
  (R-011).
- `compile_error!(all(riscv, loongarch))` and `compile_error!(not
  any(riscv, loongarch))` arms (M-003).
- `RUSTFLAGS='--cfg riscv --cfg loongarch'` validation scenario (V-E-1).
- V-UT-2 runtime boot test (R-013).
- T-1 (closed in 00-MASTER) and T-5 (closed by R-010 / 00-M-002).

[**Unresolved**]

- `Bus::aclint_idx`, `Bus::plic_idx`, `Bus::mtime`,
  `Bus::set_timer_source`, `Bus::set_irq_sink`, `Bus::ssip_flag`,
  `Bus::take_ssip` remain. Redesigning the bus ↔ intc contract is
  explicit scope of `aclintSplit` / `plicGateway` / `directIrq` and
  is tracked by the NG-5 allow-list pinned in V-UT-1.
- External-device → PLIC direct IRQ delivery (MANUAL_REVIEW #5/#6)
  is unchanged.
- `arch/loongarch/` remains a stub under both widths.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 (00) | Accepted | Still accepted. `device/intc/aclint.rs` / `plic.rs` move to `arch/riscv/device/intc/{aclint,plic}.rs` (Phase 4). Now addressed through `IntcSet` trait dispatch (M-004) rather than a `cfg_if` re-export. |
| Review | R-002 (00) | Accepted | 11 `pub(in crate::cpu::riscv)` call sites (`cpu/riscv/trap.rs:21,26,31,35,55`, `cpu/riscv/mm/tlb.rs:10,58`, `cpu/riscv/mm/mmu.rs:20`, `cpu/riscv/csr.rs:95`, `cpu/riscv/csr/ops.rs:7,16`) are rewritten to `pub(in crate::arch::riscv::<topic>)` — target crate path is topic-specific under the flat layout (`…::arch::riscv::trap` / `::csr` / `::mm`). Gate: `rg 'pub\(in crate::(cpu\|isa)::(riscv\|loongarch)' xemu/xcore/src` = 0 hits. |
| Review | R-003 (00) | Accepted | All validation uses `X_ARCH=<value> cargo check -p xcore`, per `xemu/xcore/build.rs:19-33`. No `--features loongarch`. |
| Review | R-010 (01 CRITICAL) | Accepted | Flat layout adopted. `arch/riscv/{csr,mm,trap,inst,isa,cpu,device}` are direct children. `arch/riscv/cpu/` holds only `{context.rs, debug.rs, mod.rs}`. The mip-bit module is `arch/riscv/trap/interrupt.rs` everywhere. |
| Review | R-011 (01 HIGH) | Resolved by R-010 | No top-level `pub use` hoist is needed; flat layout declares each topic directly under `arch/riscv/mod.rs` carrying original visibility (`pub mod csr;`, `pub(crate) mod mm;`, `mod inst;`). No C-2 visibility edit is required. |
| Review | R-012 (01 HIGH) | Resolved by R-010 | Canonical path for mip-bit module is `arch/riscv/trap/interrupt.rs`. Summary, Data Structure, API Surface, Execution Flow, Phase 4, and the Response Matrix all reference the same path; the `arch/riscv/cpu/trap/…` phrasing is deleted. |
| Review | R-013 (01 MEDIUM) | Accepted | V-UT-2 removed (was redundant with `cpu::tests::cpu_reset_sets_pc_to_reset_vector`). |
| Review | R-014 (01 MEDIUM) | Accepted | V-F-2 whole-file allow-list replaced by a per-file, per-literal allow-list (`docs/fix/archModule/vfz-allow.txt`) enforced by a `cargo test` integration test. New vocabulary anywhere outside the allow-list fails CI. |
| Review | R-015 (01 LOW) | Accepted | S2 green-bar phrasing pinned to named targets: `cargo test -p xcore`, `make cpu-tests-rs`, `make am-tests`, `make linux`, `make debian` pass; the intc seam is not exercised (Phase 4 land). |
| Review | TR-2 | Accepted | Back-compat re-exports kept at `cpu::*` / `isa::*` / `device::*` — implemented as trait aliases + `pub type` aliases, which preserve name, type, and module path for downstream (I-4). |
| Review | TR-3 | Accepted | `IrqState`'s storage (`Arc<AtomicU64>`) stays neutral in `device/mod.rs`; the `impl InterruptSink for IrqState` block lives in `arch/riscv/trap/interrupt.rs`, which is where bit semantics live. |
| Review | TR-4 | Accepted | Phased PRs, with Phase 2 split 2a/2b/2c. |
| Review | TR-1, TR-5 | Closed | TR-1 closed by 00-M-001 (no global `trait Arch`). TR-5 closed by R-010 / 00-M-002 (flat layout). Listed here for traceability; not open. |
| Master | M-001 (00) | Applied | No global `trait Arch` is introduced. Per-concern traits (`CoreOps`, `DebugOps`, `TrapIntake`, `InterruptSink`, `IntcSet`) are the dispatch surface — compatible with the "fine-grained CoreOps-like" mandate. |
| Master | M-002 (00) | Applied | `arch/riscv/` is physically flat at topic level. No `irq_bits.rs`. mip bits are in `arch/riscv/trap/interrupt.rs`. |
| Master | M-001 (01) | Applied | No identifier `selected` anywhere. Each upper-layer module names its arch implementor through a direct `#[cfg(riscv)] pub type …` alias: `cpu::Core = crate::arch::riscv::cpu::RVCore`, etc. |
| Master | M-002 (01) | Applied | Plan tightened — removed repeated layout diagrams, consolidated the two paste-target phrasings into one, dropped the compile_error! paragraph, removed T-1 / T-5 from open trade-offs. |
| Master | M-003 (01) | Applied | No `compile_error!` canary; no `RUSTFLAGS` scenario; no arch-validity assertion in-source. Build-script is authoritative. |
| Master | M-004 (01 CRITICAL) | Applied | Upper layer owns trait surface (`CoreOps`, `DebugOps`, `TrapIntake`, `InterruptSink`, `IntcSet`); `arch/<name>/` owns impls. Each upper-layer module contains exactly one `#[cfg]`-gated `pub type` alias naming the arch implementor. No `use crate::arch::<name>::…` leaks outside `arch/` or the seam alias sites. |

> Rules:
> - Every prior CRITICAL / HIGH finding (R-001, R-002, R-003, R-010, R-011, R-012) appears above.
> - Every MASTER directive (00-M-001, 00-M-002, 01-M-001..M-004) appears above.
> - No rejections in this round.

---

## Spec

[**Goals**]

- G-1: Every arch-specific file lives under `xcore/src/arch/<name>/` in
  a flat topic layout: `arch/riscv/{cpu, csr, mm, trap, inst, isa,
  device}`. `arch/riscv/cpu/` holds only the "CPU core" topic
  (`context.rs`, `debug.rs`, `mod.rs` with `RVCore`); all other topics
  are siblings.
- G-2: Upper-layer modules (`cpu/`, `isa/`, `device/`) contain only
  arch-neutral items: trait definitions, trait-using generic types,
  and **exactly one** `#[cfg]`-gated `pub type` alias per seam that
  names the concrete arch implementor. No `use crate::arch::<name>::…`
  path appears outside `arch/` and the alias sites.
- G-3: The RISC-V `mip` bit vocabulary (SSIP / MSIP / STIP / MTIP / SEIP
  / MEIP / HW_IP_MASK) lives in `arch/riscv/trap/interrupt.rs`. No
  RISC-V constants remain in `device/mod.rs`; the upper layer
  interacts with interrupt pending state through the neutral
  `InterruptSink` trait.
- G-4: `device/intc/aclint.rs` and `device/intc/plic.rs` move to
  `arch/riscv/device/intc/{aclint,plic}.rs`. A neutral `Intc` bundle
  type (arch-implemented) satisfies the upper-layer `IntcSet` trait
  so the generic `CPU<Core>` construction path does not name ACLINT
  / PLIC concretely.
- G-5: Git history for every moved file is preserved via `git mv`.

- NG-1: No change to `BootConfig`, `BootLayout`, `MachineConfig`,
  `XError`, `XResult`, `CoreContext`, `RESET_VECTOR`, `State`, `XCPU`,
  `with_xcpu`, `Breakpoint`. Downstream crates compile unchanged.
- NG-2: No global `trait Arch { type Word; … }` (00-M-001).
- NG-3: No xdb / xlogger / xam / xlib / difftest / am-tests source edits.
- NG-4: No `Makefile`, DTS, boot-config, or env-default changes.
- NG-5: `Bus::{aclint_idx, plic_idx, mtime, set_timer_source,
  set_irq_sink, ssip_flag, take_ssip}` remain in place. Bus-level ↔
  intc contract redesign is queued under `aclintSplit` / `plicGateway`
  / `directIrq` and tracked by the V-UT-1 allow-list.
- NG-6: No semantic edits to ACLINT / PLIC / UART / VirtioBlk /
  TestFinisher / RVCore / CSR / MMU / TLB / trap logic. `use` paths
  and visibility-path tokens are the only edits permitted.
- NG-7: No MSRV / edition / Cargo dependency changes.

[**Architecture**]

Before (today):

```
xcore/src/
├── cpu/{mod.rs, core.rs, debug.rs, riscv/, loongarch/}
├── isa/{mod.rs, instpat/, riscv/, loongarch/}
└── device/{mod.rs (holds RISC-V mip bits), bus.rs, intc/{aclint,plic}.rs, …}
```

After (this plan):

```
xcore/src/
├── arch/
│   ├── mod.rs                (single-line `#[cfg]` `pub mod riscv;` / `pub mod loongarch;`)
│   ├── riscv/
│   │   ├── mod.rs            (declares topic submodules — flat)
│   │   ├── cpu/              (RVCore, RVCoreContext, debug impl)
│   │   ├── csr/              (CsrFile, Mip, MStatus, PrivilegeMode, ops)
│   │   ├── mm/               (Mmu, Pmp, Tlb)
│   │   ├── trap/             (cause, exception, handler, interrupt)
│   │   │                       interrupt.rs hosts mip bits + InterruptSink impl
│   │   ├── inst/             (per-instruction handlers)
│   │   ├── isa/              (RvDecoder, DecodedInst, RVReg)
│   │   └── device/intc/      (Aclint, Plic, Intc bundle + IntcSet impl)
│   └── loongarch/{mod.rs, cpu/, isa/}   (stub)
├── cpu/
│   ├── mod.rs                (CPU<Core> generic; `#[cfg(riscv)] pub type Core = …;`)
│   ├── core.rs               (CoreOps, BootMode, TrapIntake)
│   └── debug.rs              (DebugOps, Breakpoint)
├── isa/
│   ├── mod.rs                (`#[cfg(riscv)] pub type Decoder = …;` + IMG passthrough)
│   └── instpat/              (unchanged)
├── device/
│   ├── mod.rs                (Device, IrqState, InterruptSink; no mip bits)
│   ├── bus.rs                (unchanged in this plan; NG-5)
│   ├── intc/mod.rs           (IntcSet trait; `#[cfg(riscv)] pub type Intc = …;`)
│   └── …                     (ram, uart, test_finisher, virtio, virtio_blk — neutral)
├── config/   utils/   error.rs   lib.rs
```

**Seam shape.** Every upper-layer module has at most one line of
arch-aware code: `#[cfg(riscv)] pub type <Name> = crate::arch::riscv::<path>;`.
No `cfg_if!`, no `pub use` glob across seam, no `selected`. Trait
definitions — `CoreOps` / `DebugOps` / `TrapIntake` / `InterruptSink`
/ `IntcSet` — are arch-agnostic and defined once in the upper layer.
Arch-specific `impl` blocks for these traits live in `arch/<name>/`
and are the only place the concrete types are named.

`arch/mod.rs` is also a single-line switch:

```rust
// xcore/src/arch/mod.rs
#[cfg(riscv)]     pub mod riscv;
#[cfg(loongarch)] pub mod loongarch;
```

No `cfg_if!`, no `compile_error!` — `build.rs` guarantees exactly one
of `riscv` / `loongarch` is set from a known `X_ARCH`.

[**Invariants**]

- I-1: **Arch-path isolation.** No file under `xemu/xcore/src/` outside
  `arch/` may reference `crate::arch::riscv::` or `crate::arch::loongarch::`
  by name, **except** at a known seam-alias site listed in
  `docs/fix/archModule/vfz-allow.txt`. Enforced structurally by the
  V-UT-1 isolation test.
- I-2: **Vocabulary isolation.** No file under `xemu/xcore/src/` outside
  `arch/` and outside the allow-list may contain the RISC-V vocabulary
  literals (`MSIP`, `MTIP`, `MEIP`, `SEIP`, `SSIP`, `STIP`, `mtime`,
  `mtimecmp`, `aclint`, `plic`, `hart`, `RVCore`, `Mstatus`, `Mip`,
  `Sv32`, `Sv39`). The only knowingly-allowed occurrences are pinned
  per-file, per-literal in `vfz-allow.txt` (NG-5).
- I-3: **Git history preserved** (`git log --follow` traces every
  moved file back pre-refactor). `git mv` only.
- I-4: **Public API unchanged.** `lib.rs` re-exports (`BootConfig`,
  `CoreContext`, `RESET_VECTOR`, `State`, `XCPU`, `DebugOps`,
  `Breakpoint`, `with_xcpu`, `Uart`, `XError`, `XResult`, `BootLayout`,
  `MachineConfig`) are identical in name, type, and path. Downstream
  compiles with zero edits.
- I-5: **Behaviour unchanged.** `cargo test -p xcore`, `make
  cpu-tests-rs`, `make am-tests`, `make linux`, `make debian` produce
  the same pass/fail and boot artefacts; difftest vs QEMU / Spike has
  zero divergence on the default cpu-tests-rs set.
- I-6: **Seam bound.** The set of `#[cfg]`-aware lines outside `arch/`
  is exactly: `arch/mod.rs` (2 lines), `cpu/mod.rs` (1 line alias),
  `isa/mod.rs` (1 line alias), `device/intc/mod.rs` (1 line alias),
  and the `impl InterruptSink for IrqState` location is arch-local —
  `device/mod.rs` contains zero `#[cfg]` lines.

[**Data Structure**]

Upper-layer trait surface (arch-agnostic, owned by `cpu/`, `isa/`,
`device/`). Only the traits the refactor needs are shown; existing
traits (`CoreOps`, `DebugOps`, `Device`) are kept verbatim.

```rust
// xcore/src/cpu/core.rs
pub trait CoreOps { /* unchanged — see cpu/core.rs:20-37 */ }

/// Neutral edge/level-input contract consumed by `RVCore::sync_interrupts`
/// and by any future arch that needs to fold interrupt-line state into
/// a privileged pending register.
pub trait TrapIntake {
    /// Refresh arch pending state from the shared `InterruptSink`
    /// snapshot. Arch decides which bits map where.
    fn sync(&mut self);
}

// xcore/src/device/mod.rs
/// Arch-neutral pending-interrupt sink. `IrqState` is the canonical
/// implementor; `type Bits` is defined by the arch via a blanket impl.
pub trait InterruptSink: Send + Sync + Clone {
    type Bits: Copy;
    fn set(&self, b: Self::Bits);
    fn clear(&self, b: Self::Bits);
    fn load(&self) -> u64;
    fn reset(&self);
}

// xcore/src/device/intc/mod.rs
/// Arch-neutral "platform interrupt controller bundle" — hides the
/// concrete (ACLINT, PLIC, …) tuple behind a handle-by-role API.
pub trait IntcSet {
    fn tick_fast(&mut self);    // ACLINT-class fast tick
    fn tick_slow(&mut self);    // PLIC-class slow tick
}
```

Seam aliases (the only `#[cfg]`-aware lines in the upper layer):

```rust
// xcore/src/arch/mod.rs
#[cfg(riscv)]     pub mod riscv;
#[cfg(loongarch)] pub mod loongarch;

// xcore/src/cpu/mod.rs
#[cfg(riscv)]     pub type Core = crate::arch::riscv::cpu::RVCore;
#[cfg(loongarch)] pub type Core = crate::arch::loongarch::cpu::LaCore;

// xcore/src/isa/mod.rs
#[cfg(riscv)]     pub use crate::arch::riscv::isa::IMG;
#[cfg(loongarch)] pub use crate::arch::loongarch::isa::IMG;

// xcore/src/device/intc/mod.rs
#[cfg(riscv)]     pub type Intc = crate::arch::riscv::device::intc::Intc;
// LoongArch: no intc — absence is the correct enforcement.
```

`arch/riscv/mod.rs` (flat topic layout):

```rust
// xcore/src/arch/riscv/mod.rs
pub mod cpu;
pub mod csr;
pub(crate) mod mm;  // visibility preserved from cpu/riscv/mod.rs:10
mod inst;           // visibility preserved from cpu/riscv/mod.rs:9
pub mod trap;
pub mod isa;
pub mod device;
```

Arch-side impls (shown in outline — full bodies come from relocation):

```rust
// xcore/src/arch/riscv/trap/interrupt.rs
pub const SSIP: u64 = 1 << 1;
pub const MSIP: u64 = 1 << 3;
pub const STIP: u64 = 1 << 5;
pub const MTIP: u64 = 1 << 7;
pub const SEIP: u64 = 1 << 9;
pub const MEIP: u64 = 1 << 11;
pub const HW_IP_MASK: crate::config::Word = (MSIP | MTIP | SEIP | MEIP) as _;

impl crate::device::InterruptSink for crate::device::IrqState {
    type Bits = u64;
    fn set(&self, b: u64)   { Self::set(self, b) }
    fn clear(&self, b: u64) { Self::clear(self, b) }
    fn load(&self)  -> u64  { Self::load(self) }
    fn reset(&self)         { Self::reset(self) }
}

// xcore/src/arch/riscv/device/intc/mod.rs
pub mod aclint;
pub mod plic;
pub struct Intc { pub aclint: aclint::Aclint, pub plic: plic::Plic }
impl crate::device::intc::IntcSet for Intc {
    fn tick_fast(&mut self) { self.aclint.tick() }
    fn tick_slow(&mut self) { self.plic.tick() }
}
```

[**API Surface**]

Public crate API (unchanged — I-4):

```rust
// xcore/src/lib.rs
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
// xcore/src/arch/riscv/cpu/mod.rs (after)
use crate::{
    cpu::core::{CoreOps, TrapIntake},
    device::{Device, InterruptSink, IrqState, bus::Bus,
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

- C-1: Landable as phased PRs; each phase boundary keeps `cargo test
  -p xcore`, `make linux`, `make debian` green where applicable
  (Phase 3 has only `cargo check` for LoongArch).
- C-2: **No semantic edits** inside moved files beyond `use`-path and
  `pub(in …)` token rewrites required to compile. Byte-identical
  behaviour.
- C-3: **`git mv` only** for every moved file. Verified by
  `git log --follow`.
- C-4: **No Cargo / MSRV / edition / dep changes.**
- C-5: **Exactly one `#[cfg]`-aware line** per upper-layer seam file
  (`cpu/mod.rs`, `isa/mod.rs`, `device/intc/mod.rs`) plus `arch/mod.rs`
  (two `#[cfg]` lines). `device/mod.rs` contains zero.

---

## Implement

### Execution Flow

[**Main Flow**]

1. **Phase 1 — Upper-layer trait surface + arch skeleton.** Add
   `TrapIntake` to `cpu/core.rs`, `InterruptSink` to `device/mod.rs`,
   `IntcSet` to `device/intc/mod.rs`. No bodies change yet. Create
   empty `arch/{mod.rs, riscv/mod.rs, loongarch/mod.rs}`; `arch/mod.rs`
   is the two-line `#[cfg]` switch. Add `mod arch;` to `lib.rs`.
   Existing `cfg_if!` blocks in `cpu/mod.rs`, `isa/mod.rs`, and
   `device/intc/mod.rs` remain untouched at this phase — the seam
   switch happens in Phase 2c once the arch tree is populated.
2. **Phase 2a — Relocate.** `git mv cpu/riscv arch/riscv/cpu` and
   `git mv isa/riscv arch/riscv/isa`.
3. **Phase 2b — Flat hoist.** `git mv` of each nested topic out of
   `arch/riscv/cpu/` to `arch/riscv/<topic>/`:
   - `arch/riscv/cpu/csr`  → `arch/riscv/csr`   (+ `csr.rs`)
   - `arch/riscv/cpu/mm`   → `arch/riscv/mm`    (+ `mm.rs`)
   - `arch/riscv/cpu/trap` → `arch/riscv/trap`  (+ `trap.rs`)
   - `arch/riscv/cpu/inst` → `arch/riscv/inst`  (+ `inst.rs`)
   After hoist, `arch/riscv/cpu/` contains only `{context.rs,
   debug.rs, mod.rs}`. Populate `arch/riscv/mod.rs` per Data Structure
   (preserves `pub mod csr`, `pub(crate) mod mm`, `mod inst`,
   `pub mod trap`, `pub mod cpu`, `pub mod isa` — each carrying its
   original visibility from `cpu/riscv/mod.rs:6-11`, so no C-2
   visibility edit).
4. **Phase 2c — Seam switch + import rewrites.**
   - Replace `cpu/mod.rs`'s `cfg_if!` block with the single-line
     `#[cfg(riscv)] pub type Core = crate::arch::riscv::cpu::RVCore;`.
     Remove `mod riscv;` and `pub use self::riscv::*;`.
   - Replace `isa/mod.rs`'s `cfg_if!` block with `#[cfg(riscv)] pub
     use crate::arch::riscv::isa::IMG;` (and `RVReg`/`Decoder` aliases
     if the upper layer currently names them — verify by grepping
     `isa::` users).
   - Rewrite all 11 `pub(in crate::cpu::riscv)` tokens to
     `pub(in crate::arch::riscv::<topic>)`, where `<topic>` is the
     topic under which the token's file now resides after Phase 2b:
     - `arch/riscv/trap.rs:21,26,31,35,55` → `pub(in crate::arch::riscv::trap)`
     - `arch/riscv/mm/tlb.rs:10,58` → `pub(in crate::arch::riscv::mm)`
     - `arch/riscv/mm/mmu.rs:20` → `pub(in crate::arch::riscv::mm)`
     - `arch/riscv/csr.rs:95` → `pub(in crate::arch::riscv::csr)`
     - `arch/riscv/csr/ops.rs:7,16` → `pub(in crate::arch::riscv::csr)`
   - Gate: `rg 'pub\(in crate::(cpu|isa)::(riscv|loongarch)' xemu/xcore/src` = 0.
   - Fix `super::`/`crate::` hops inside `arch/riscv/` that broke on
     relocation (mechanical).
5. **Phase 3 — LoongArch relocation.** `git mv` the two LoongArch
   stubs into `arch/loongarch/{cpu,isa}/`. Add the
   `#[cfg(loongarch)]` arm to each upper-layer seam alias.
   Validation gate: `X_ARCH=loongarch32 cargo check -p xcore` and
   `X_ARCH=loongarch64 cargo check -p xcore`.
6. **Phase 4 — Device / intc + mip bits + trait wiring.**
   - `git mv device/intc/{aclint,plic}.rs arch/riscv/device/intc/`.
     Create `arch/riscv/device/mod.rs` (`pub mod intc;`) and
     `arch/riscv/device/intc/mod.rs` (`pub mod aclint; pub mod plic;`
     plus the `Intc` bundle struct and its `impl IntcSet`).
   - Cut `SSIP/MSIP/STIP/MTIP/SEIP/MEIP/HW_IP_MASK` from
     `device/mod.rs:55-72`. Paste into `arch/riscv/trap/interrupt.rs`.
     Add `impl InterruptSink for IrqState` in that same file.
   - Replace `device/intc/mod.rs` body with the single-line
     `#[cfg(riscv)] pub type Intc = crate::arch::riscv::device::intc::Intc;`.
   - Rewrite imports in the relocated `aclint.rs` / `plic.rs`:
     `use crate::device::{…, MSIP, MTIP, …}` →
     `use crate::{arch::riscv::trap::interrupt::{MSIP, MTIP},
                 device::{Device, IrqState, mmio_regs}};`
     (analogous for PLIC / `MEIP, SEIP`).
   - Rewrite `arch/riscv/cpu/mod.rs` device imports:
     `use crate::device::intc::{aclint::Aclint, plic::Plic};` →
     `use super::device::intc::{Intc, aclint::Aclint, plic::Plic};`
     and `use crate::device::HW_IP_MASK;` →
     `use super::trap::interrupt::HW_IP_MASK;`.
   - Rewire `sync_interrupts` to call through `TrapIntake` locally
     rather than import raw mip bits from `crate::device::`. The bit
     constants stay arch-local; the upper layer never names them.
   - Validation gate: `cargo test -p xcore`, `make cpu-tests-rs`,
     `make am-tests`, `make linux`, `make debian` pass. V-UT-1
     structural test passes. V-F-3 shows history on the moved files.
7. **Phase 5 — Docs + structural test.**
   - Land `tests/arch_isolation.rs` (V-UT-1 body) and
     `docs/fix/archModule/vfz-allow.txt` (per-file per-literal
     allow-list pinning NG-5 residuals).
   - Update `lib.rs` rustdoc lead to reference `arch/<name>/`.
   - Update `docs/DEV.md` architecture note if it references old
     paths (`cpu/riscv/…`, `isa/riscv/…`, `device/intc/…`).
   - Run `make fmt && make clippy && make run && make test`.

[**Failure Flow**]

1. **Phase 2c import cascade fails.** Walk rustc errors top-to-bottom;
   apply the `pub(in …)` token rewrite and `super::`/`crate::arch::`
   path fixups. If any file *outside* `arch/` needs
   `use crate::arch::…`, that is an I-1 violation — relocate the
   call-site into `arch/` or expose the item via an upper-layer trait.
2. **V-UT-1 fails after Phase 4.** For each offending file/line, either
   (a) the literal belongs in `vfz-allow.txt` (a tracked NG-5 residual
   with `// TODO: archBus follow-up`), or (b) relocate the code into
   `arch/`. New unreferenced occurrences are always (b).
3. **`X_ARCH=loongarch32 cargo check -p xcore` fails after Phase 3.**
   A RISC-V-only symbol leaked into the neutral layer. Relocate into
   `arch/riscv/` or gate with `#[cfg(riscv)]`. Do not fake LoongArch
   bindings (no placeholder `arch/loongarch/trap/interrupt.rs`).
4. **`git log --follow` shows no pre-refactor history.** The move was
   copy+delete. Redo as `git mv` and amend the phase commit.

[**State Transition**]

- **S0** pre-refactor: current tree.
- **S1** after Phase 1: upper-layer traits and empty `arch/` skeleton
  exist; old `cfg_if!` blocks still mediate dispatch. `cargo test -p
  xcore` and `make linux` / `make debian` pass; intc seam unchanged.
- **S2** after Phase 2 (2a+2b+2c): RISC-V cpu/isa/topics physically
  flat under `arch/riscv/`; upper-layer seams are `#[cfg(riscv)] pub
  type …` aliases. Named gates: `cargo test -p xcore`,
  `make cpu-tests-rs`, `make am-tests`, `make linux`, `make debian`
  pass. The intc seam is unexercised (Phase 4 lands it).
- **S3** after Phase 3: LoongArch relocated to `arch/loongarch/{cpu,
  isa}`. Gate: `X_ARCH=loongarch{32,64} cargo check -p xcore`;
  default `X_ARCH=riscv32 cargo test -p xcore` remains green.
- **S4** after Phase 4: `Aclint`, `Plic`, `Intc`, mip bits, and
  `impl InterruptSink for IrqState` all under `arch/riscv/`.
  `device/mod.rs` is arch-neutral. Full matrix green.
- **S5** after Phase 5: `tests/arch_isolation.rs` locks I-1 and I-2;
  docs reflect new layout; ready to merge.

### Implementation Plan

Phase-level PR titles (per TR-4 / R-014):

- **PR 1 — `refactor(xcore): introduce arch/ skeleton + upper-layer traits`**
- **PR 2a — `refactor(xcore): relocate riscv cpu/isa under arch/riscv/ (step 1/2)`**
- **PR 2b — `refactor(xcore): flatten arch/riscv topic directories (step 2/2)`**
- **PR 2c — `refactor(xcore): switch cpu/isa seams to direct type alias`**
- **PR 3 — `refactor(xcore): relocate loongarch stubs under arch/loongarch/`**
- **PR 4 — `refactor(xcore): move aclint/plic + mip bits + wire InterruptSink/IntcSet`**
- **PR 5 — `test(xcore): arch_isolation structural test + docs`**

Phase bodies:

- **Phase 1 — skeleton + traits.** Edit `cpu/core.rs` (add
  `TrapIntake`), `device/mod.rs` (add `InterruptSink`),
  `device/intc/mod.rs` (add `IntcSet`). Create `arch/{mod.rs,
  riscv/mod.rs, loongarch/mod.rs}` — `mod.rs` bodies are a doc
  comment + (for `arch/mod.rs`) two `#[cfg]` lines. Add `mod arch;`
  to `lib.rs`. `cargo build -p xcore` and `cargo test -p xcore`
  green under default `X_ARCH=riscv32`.
- **Phase 2a — relocate.** Two `git mv` calls. Build is still green
  only because `cpu/mod.rs`'s `cfg_if!` still says `mod riscv; pub
  use self::riscv::*;` — but that `mod riscv;` no longer resolves,
  so Phase 2a alone does **not** compile. 2a is a commit-only step
  within PR 2a; PR 2a lands 2a+2b+2c together or 2a+2b alone with
  a temporary `#[path]` attribute. Recommended: land PR 2a as 2a
  commit only on a branch; merge to trunk only once PR 2c lands.
  (This is the only non-green phase boundary; S2's named-target
  green-bar refers to post-2c.)
- **Phase 2b — flat hoist.** Four `git mv` calls. Populate
  `arch/riscv/mod.rs` per Data Structure. `arch/riscv/cpu/mod.rs`
  drops its now-empty `pub mod csr; pub mod trap; pub(crate) mod mm;
  mod inst;` lines.
- **Phase 2c — seam switch.** `cpu/mod.rs` `cfg_if!` → single-line
  `#[cfg(riscv)] pub type Core = …`. Same for `isa/mod.rs`. Apply
  the `pub(in …)` token rewrite (11 sites, topic-specific under the
  flat layout). Fix intra-arch `use super::` paths. **Green bar:**
  `cargo test -p xcore`, `make cpu-tests-rs`, `make am-tests`,
  `make linux`, `make debian` pass. The intc seam is unexercised
  here — `device/intc/` is untouched until Phase 4.
- **Phase 3 — LoongArch.** `git mv cpu/loongarch
  arch/loongarch/cpu`; same for `isa/loongarch`. Add the
  `#[cfg(loongarch)]` arm of each alias in `cpu/mod.rs` and
  `isa/mod.rs`. The `pub(in …)` grep still returns 0 hits.
  **Green bar:** `X_ARCH=loongarch{32,64} cargo check -p xcore` plus
  default `cargo test -p xcore`.
- **Phase 4 — ACLINT / PLIC + mip bits + traits wiring.** Per Main
  Flow step 6. **Green bar:** full test + boot matrix; V-UT-1
  passes against the initial `vfz-allow.txt`.
- **Phase 5 — tests + docs.** Add `tests/arch_isolation.rs`, land
  `vfz-allow.txt`, update `lib.rs` rustdoc and `docs/DEV.md` path
  references, run `make fmt && make clippy && make run && make test`.

---

## Trade-offs

Open trade-offs in round 02:

- **T-2: Back-compat re-exports.** Keep (TR-2). `cpu::Core`,
  `isa::IMG`, `device::intc::Intc` are the public names; they continue
  to exist via the `pub type` / `pub use` aliases, so downstream
  compiles unchanged. Hard-cut rename is out of scope.
- **T-3: `IrqState` location.** Keep `IrqState` arch-neutral in
  `device/mod.rs`; move only the `impl InterruptSink` and the bit
  literals to `arch/riscv/trap/interrupt.rs` (TR-3). The MANUAL_REVIEW
  #5/#6 follow-up reshapes `IrqState`'s storage; moving it now would
  collide.
- **T-4: Phased PRs (TR-4).** Seven phased PRs (1, 2a, 2b, 2c, 3, 4,
  5). Per-phase review is tractable and `git bisect`-friendly.

Closed: T-1 (00-M-001), T-5 (R-010 / 00-M-002).

---

## Validation

[**Unit Tests**]

- **V-UT-1: Arch-isolation structural test.** `xemu/xcore/tests/arch_isolation.rs`
  walks every `.rs` file under `xemu/xcore/src/` that is **not** inside
  `src/arch/`. For each file, it tokenises with `regex` (or `aho-corasick`
  — dev-only dep, see C-4 note below) and asserts:
  1. No substring `crate::arch::riscv::` or `crate::arch::loongarch::`
     appears outside the seam-alias allow-list entries (I-1).
  2. None of the RISC-V vocabulary literals (MSIP, MTIP, MEIP, SEIP,
     SSIP, STIP, mtime, mtimecmp, aclint, plic, hart, RVCore, Mstatus,
     Mip, Sv32, Sv39) appears outside the per-file-per-literal allow-list
     entries pinned in `docs/fix/archModule/vfz-allow.txt` (I-2, R-014).
  Allow-list initial contents:
  - `src/cpu/mod.rs` — literal `crate::arch::riscv::cpu::RVCore`
    (and, once Phase 3 lands, `crate::arch::loongarch::cpu::LaCore`).
  - `src/isa/mod.rs` — `crate::arch::riscv::isa` (and LoongArch
    counterpart post-Phase 3).
  - `src/device/intc/mod.rs` — `crate::arch::riscv::device::intc::Intc`.
  - `src/device/bus.rs` — NG-5 residuals, pinned literals:
    `aclint_idx`, `plic_idx`, `set_timer_source`, `set_irq_sink`,
    `ssip_flag`, `take_ssip`, `mtime` (one occurrence each; duplicates
    fail).
  The test is run by `cargo test -p xcore --test arch_isolation`.
  **Note on C-4 / deps:** if adding a dev-dep is undesirable, the test
  can be written with `std` alone (byte-level `memchr` via `str::find`).
  The plan commits to zero production-dep change; a `dev-dependencies`
  addition of `aho-corasick` (already transitively present through
  `pest` in xcore) is acceptable and verifiable by `cargo tree -p
  xcore --edges normal` diff-empty.
- **V-UT-2: removed** (R-013 — redundant with existing
  `cpu::tests::cpu_reset_sets_pc_to_reset_vector`).

[**Integration Tests**]

- **V-IT-1:** `cargo test -p xcore` passes at the end of each phase
  (2c, 3, 4, 5). Phase 2a/2b are commit-only internal steps whose
  green-bar is covered by the 2c boundary.
- **V-IT-2:** `make cpu-tests-rs` passes at Phase 2c and again at
  Phase 4.
- **V-IT-3:** `make am-tests` (UART, ACLINT, PLIC, CSR, trap,
  interrupts, float, keyboard) passes at Phase 4 — ACLINT/PLIC
  relocation must not perturb am-tests.
- **V-IT-4:** `make linux` boots to interactive shell; `make debian`
  boots to Python3 (per `docs/DEV.md`). Wall-clock within ±5% of
  pre-refactor baseline, measured with `DEBUG=n`.

[**Failure / Robustness Validation**]

- **V-F-1: LoongArch check.** `X_ARCH=loongarch32 cargo check -p
  xcore` and `X_ARCH=loongarch64 cargo check -p xcore` succeed at
  Phase 3 and stay green through Phases 4–5. (Uses the real
  `build.rs` mechanism.)
- **V-F-3: Git history.** `git log --follow` on
  `arch/riscv/cpu/mod.rs`, `arch/riscv/isa/decoder.rs`,
  `arch/riscv/trap/interrupt.rs`, `arch/riscv/csr/mip.rs`,
  `arch/riscv/mm/mmu.rs`, `arch/riscv/inst/base.rs`,
  `arch/riscv/device/intc/aclint.rs`,
  `arch/riscv/device/intc/plic.rs` each traces pre-refactor.
- **V-F-4: Difftest.** Difftest vs QEMU and Spike — zero divergence
  on the default cpu-tests-rs set after Phases 2c and 4.
- **V-F-5: Dep diff.** `cargo tree -p xcore --edges normal` diff is
  empty (C-4 / NG-7). Dev-deps may grow by `aho-corasick` iff V-UT-1
  adopts it; noted and acceptable.

[**Edge Case Validation**]

- **V-E-1: removed** (M-003 — `build.rs` is authoritative).
- **V-E-2: Re-export hygiene.** `cargo build -p xcore` emits no
  `ambiguous_glob_reexports` or `unused_imports` warnings under the
  new seam aliases.
- **V-E-3: Downstream source-compat.** `cargo build -p xdb`,
  `cargo build -p xam`, and `cargo build -p xemu` succeed against the
  refactored `xcore` with zero source edits. (Protects I-4.)

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (flat arch/ topic layout) | V-F-3, V-UT-1 |
| G-2 (upper-layer owns traits; one `#[cfg]` alias per seam) | V-UT-1, V-E-2 |
| G-3 (mip bits in arch/riscv/trap/interrupt.rs) | V-UT-1, V-IT-3 |
| G-4 (ACLINT/PLIC relocated; IntcSet bundle) | V-UT-1, V-F-3, V-IT-3 |
| G-5 (git history preserved) | V-F-3 |
| I-1 (no `crate::arch::…` outside arch/ or seams) | V-UT-1 |
| I-2 (no RISC-V vocabulary outside arch/ or seams or NG-5) | V-UT-1 |
| I-3 (git history) | V-F-3 |
| I-4 (public API unchanged; downstream compiles unchanged) | V-E-3 |
| I-5 (behaviour unchanged) | V-IT-1, V-IT-2, V-IT-3, V-IT-4, V-F-4 |
| I-6 (seam bound: ≤1 `#[cfg]` line per upper-layer seam file) | V-UT-1 (structural grep), V-E-2 |
| C-1 (phased PRs, green at each listed phase boundary) | V-IT-1..V-IT-4 per phase |
| C-2 (no semantic edits inside moved files) | V-UT-1 (no test source edits), V-IT-2, V-IT-4, V-F-4 |
| C-3 (git mv only) | V-F-3 |
| C-4 (no prod-dep / MSRV change) | V-F-5 |
| C-5 (seam-line bound) | V-UT-1 grep + phase 2c/3 checklist |
| M-001 (00) / M-004 (01) (trait dispatch, no global `trait Arch`) | V-UT-1 (no `arch::…` path leaks); static review against `cpu/core.rs`, `device/mod.rs`, `device/intc/mod.rs` trait definitions |
| M-001 (01) (no `selected`) | grep `\bselected\b` under `xemu/xcore/src` = 0 hits (added to V-UT-1) |
| M-002 (00) (flat topic layout, no `irq_bits.rs`) | V-F-3 + Phase 2b `ls arch/riscv/` post-condition |
| M-003 (01) (no in-source arch-validity check) | grep `compile_error!` under `xemu/xcore/src/arch/` = 0 hits |
