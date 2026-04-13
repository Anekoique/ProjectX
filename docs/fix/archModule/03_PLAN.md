# `archModule` PLAN `03`

> Status: Draft
> Feature: `archModule`
> Iteration: `03`
> Owner: Executor
> Depends on:
> - Previous Plan: `02_PLAN.md`
> - Review: `02_REVIEW.md`
> - Master Directive: `02_MASTER.md` (blank — skipped by user)

---

## Summary

Round 03 tightens round 02 into an implementable form without scope expansion.
Three deltas: (1) the Phase 2c `pub(in …)` rewrite maps each of the 11 sites
individually — 8 widen to `pub(in crate::arch::riscv)`, 3 stay
`pub(in crate::arch::riscv::mm)` — so the tree compiles after 2c; (2) the
upper-layer seam modules re-export a small, enumerated set of concrete arch
types (`CoreContext`, `PendingTrap`, …) alongside `pub type Core`, each one a
cfg-gated alias, because `lib.rs` and `error.rs` consume them by name and
`xdb` reads `CoreContext` by field (I-4); (3) the ceremonial traits
`TrapIntake` / `IntcSet` are dropped — only `CoreOps` / `DebugOps` remain as
the cross-seam dispatch surface, and M-004 is recorded as **Partial** in the
Response Matrix with the bus-level residuals (NG-5) explicitly deferred to
`aclintSplit` / `plicGateway` / `directIrq`. The V-UT-1 allow-list drops
`aho-corasick` for a `std`-only `arch_isolation.rs` integration test
(R-022). Phase 4's "rewire `sync_interrupts`" language is removed —
`sync_interrupts` stays an inherent `RVCore` method; Phase 4 changes only
`use` paths (R-021).

Residual risks (3 lines): (1) `arch_isolation.rs` is text-level and must
pin debug-string occurrences (`info!("plic: …")`, `info!("aclint: …")`) in
its allow-list; (2) `arch/loongarch/` remains a stub — this refactor neither
populates nor exercises the second backend; (3) **test/boot preservation
commitment**: every phase boundary must keep `cargo test --workspace` (336
tests), `cpu-tests-rs` (31), `am-tests` (8), `make linux`, `make debian`
green, and difftest vs QEMU/Spike at zero divergence — these five items are
gating in the Acceptance Mapping.

## Log

[**Feature Introduce**]

- **Per-site `pub(in …)` map (R-016).** Phase 2c lists each of the 11 sites
  individually. 8 sites widen to `pub(in crate::arch::riscv)` (arch-local);
  3 sites stay narrower (`pub(in crate::arch::riscv::mm)`). The tree
  compiles after 2c. The validation gate (`rg` returns 0 hits for
  `pub(in crate::(cpu|isa)::(riscv|loongarch))`) is unchanged.
- **Seam module = cfg-gated type-alias block (R-017).** `cpu/mod.rs` exposes
  `pub type Core`, `pub type CoreContext`, `pub type PendingTrap`, and any
  other concrete arch type currently named by `lib.rs` or `error.rs`. Each
  is one `#[cfg(riscv)]` line. I-6 is relaxed from "exactly one `#[cfg]`
  line per seam file" to "each seam file contains only cfg-gated type
  aliases / re-exports and no other arch-aware code." `xdb`'s
  field-access on `CoreContext` keeps working unchanged (I-4).
- **Drop ceremonial traits (R-018).** `TrapIntake` and `IntcSet` are
  removed. `InterruptSink` is removed too — `IrqState`'s inherent API is
  already arch-neutral storage, wrapping it in a trait only rename it.
  Only `CoreOps` / `DebugOps` remain (they predate this plan). M-004 is
  recorded as **Partial**: upper layer owns trait surface at the `CPU` /
  `debug` seam; bus-level residuals (`Bus::aclint_idx`, `Bus::plic_idx`,
  `Bus::mtime`, `Bus::take_ssip`, `Bus::set_timer_source`,
  `Bus::set_irq_sink`) are deferred to the named follow-up tasks
  `aclintSplit` / `plicGateway` / `directIrq` already queued in
  `docs/DEV.md`.
- **V-UT-1 as `std`-only `arch_isolation.rs` (R-019 / R-022).** The
  integration test at `xemu/xcore/tests/arch_isolation.rs` walks `src/`
  using `std::fs` + `str::find`; no dev-dep addition. The allow-list is a
  vocabulary of exact symbol names (`Core`, `CoreContext`, `PendingTrap`,
  …) re-exported by seam modules, applied at module-graph level (not
  whole-file exceptions). Debug-string occurrences in `device/bus.rs`
  (`info!("bus: …")` with `"aclint"` / `"plic"` substrings) are pinned as
  explicit allow entries.
- **Concrete merge-gate commands (R-020).** 2a/2b are stacked commits
  inside a single "PR 2"; only PR 2 as a whole merges to trunk. Each
  phase's green-bar is a named command list with expected exit codes.
- **C-2 narrowed (R-021).** C-2 reads "no semantic edits *inside moved
  files* beyond `use`-path and `pub(in …)` token rewrites." Phase 4's
  only source-level edit is rewriting the import lines of the relocated
  `aclint.rs` / `plic.rs` (and one import line of `arch/riscv/cpu/mod.rs`
  from `crate::device::HW_IP_MASK` to `super::trap::interrupt::HW_IP_MASK`).
  `sync_interrupts` is untouched.

[**Review Adjustments**]

- **R-016 (02 CRITICAL)** accepted in full. Phase 2c carries a per-site
  table (see Implementation Plan). 8 sites → `pub(in crate::arch::riscv)`;
  3 sites → `pub(in crate::arch::riscv::mm)`. Reason: `trap::test_helpers`
  is consumed from `mm`, `csr`, `inst`; `csr::find_desc` is consumed from
  `cpu/debug`; `csr::ops::{csr_read, csr_write}` are consumed from `inst`.
  Only `mm::tlb::{Tlb, TlbEntry}` and `mm::mmu::Mmu::tlb` are truly
  topic-local. Gate: `rg 'pub\(in crate::(cpu|isa)::(riscv|loongarch)'
  xemu/xcore/src` = 0 hits.
- **R-017 (02 CRITICAL)** accepted with option (b) of the reviewer's list.
  The seam module re-exports the concrete arch types `lib.rs` and
  `error.rs` name today, one cfg-gated alias per type. Trade-off TR-6
  documents the choice between (a) associated-types-on-`CoreOps` and
  (b) concrete-alias-re-export; (b) wins for this refactor because it
  keeps `xdb`'s field-access on `CoreContext` unchanged (I-4) and avoids
  a large difftest replumbing. I-6 relaxes from "exactly one `#[cfg]`
  line" to "seam file contains only cfg-gated type aliases and nothing
  else." C-5 relaxes identically.
- **R-018 (02 HIGH)** accepted with option (a) (drop ceremonial traits,
  record M-004 as Partial). Reason: the three traits don't mediate
  real cross-seam dispatch in the current codebase; the genuine leakage
  is `Bus`-level and is out of scope (NG-5). Round 03 does **not**
  introduce a real trait in place (that would be scope expansion — ruled
  out). M-004's Response Matrix row is rewritten to "Partial — `CPU` /
  `DebugOps` seam is trait-dispatched; bus-level residuals deferred to
  `aclintSplit` / `plicGateway` / `directIrq`."
- **R-019 (02 MEDIUM)** accepted. `arch_isolation.rs` is explicitly
  text-level; the plan names the limitation and pins the known
  debug-string occurrences in the initial allow-list.
- **R-020 (02 MEDIUM)** accepted. 2a and 2b are commit-only inside a
  single PR 2; PR 2 lands 2a+2b+2c atomically to trunk. No standalone
  broken commit lands on trunk; `git bisect` is safe.
- **R-021 (02 MEDIUM)** accepted. C-2 is narrowed to "no semantic edits
  inside moved files." Phase 4's `sync_interrupts` language is removed;
  Phase 4 only rewrites imports.
- **R-022 (02 LOW)** accepted. `arch_isolation.rs` uses `std` only;
  `aho-corasick` is not added. NG-7 holds as-is.

[**Master Compliance**]

- **00-M-001 (no global `trait Arch`).** Applied. `CoreOps` / `DebugOps`
  remain the fine-grained contracts; no coarse `Arch` trait.
- **00-M-002 (flat topic-organised `arch/<name>/`).** Applied. Carried
  from round 02; no change.
- **01-M-001 (no `selected`).** Applied. Seam is direct-path cfg-gated
  `pub type` / `pub use`. No identifier `selected` anywhere.
- **01-M-002 (clean, concise, elegant).** Applied. This plan is
  materially shorter than round 02: the ceremonial-trait discussion is
  deleted; Phase 2c gains a concrete table but loses prose; Trade-offs
  shrink (TR-2..TR-5 are closed; TR-6 is the only new open item).
- **01-M-003 (no in-source arch-validity check).** Applied. `build.rs` is
  authoritative.
- **01-M-004 (trait-dispatch + tiny `cfg(arch)` seam).** **Partial.**
  `CoreOps` / `DebugOps` cross the seam and dispatch arch behaviour;
  `CPU<Core>` is generic over `CoreOps`. Concrete arch *data types*
  (`CoreContext`, `PendingTrap`) cross the seam as cfg-gated aliases
  because they are consumed by name and by field (not by behaviour).
  Bus-level residuals (`Bus::aclint_idx`, …) remain; these are the real
  residual M-004 work and are queued under NG-5. This is recorded as
  "Partial — seam dispatch at CPU / DebugOps boundary; bus-level
  residuals deferred" in the Response Matrix.
- **02_MASTER** blank; no new directives.

### Changes from Previous Round

[**Added**]

- Per-site `pub(in …)` rewrite table in Phase 2c (11 rows).
- `CoreContext` / `PendingTrap` (and any other `lib.rs` / `error.rs`-named
  concrete arch types) as cfg-gated seam aliases in `cpu/mod.rs`.
- TR-6 (concrete-alias vs associated-type-on-`CoreOps`).
- `arch_isolation.rs` written in `std`-only form; explicit text-level
  caveat; debug-string allow entries pinned.
- Acceptance Mapping now lists the five gating items as named rows
  (336 unit tests, 31 cpu-tests-rs, 8 am-tests, `make linux`,
  `make debian`) plus difftest zero-divergence.

[**Changed**]

- I-6 / C-5: "exactly one `#[cfg]` line per seam file" → "seam file
  contains only cfg-gated type aliases / re-exports and nothing else."
- C-2: "no semantic edits beyond imports" → "no semantic edits **inside
  moved files** beyond imports and `pub(in …)` tokens."
- Phase 4's step-6 bullet "rewire `sync_interrupts` to call through
  `TrapIntake`" → "replace `use crate::device::HW_IP_MASK;` with
  `use super::trap::interrupt::HW_IP_MASK;` in `arch/riscv/cpu/mod.rs`;
  `sync_interrupts` method body is byte-identical."
- M-004 row in Response Matrix: "Applied" → "Partial".
- V-UT-1: `aho-corasick` dropped; `std`-only.

[**Removed**]

- Traits `TrapIntake`, `InterruptSink`, `IntcSet` (R-018 option a).
- `impl InterruptSink for IrqState` block (no longer needed).
- The `Intc` bundle struct's `impl IntcSet` (the `Intc` bundle still
  relocates to `arch/riscv/device/intc/mod.rs` as a plain struct —
  `device/intc/mod.rs` continues to expose it via a cfg-gated
  `pub type Intc` alias, same shape as `Core`).
- Round 02's "exactly one `#[cfg]` line" I-6 / C-5 wording.
- V-F-5's dev-dep discussion (no dep change).

[**Unresolved**]

- Bus-level residuals (`Bus::{aclint_idx, plic_idx, mtime,
  set_timer_source, set_irq_sink, ssip_flag, take_ssip}`). Queued under
  `aclintSplit` / `plicGateway` / `directIrq`; pinned in
  `arch_isolation.rs` allow-list (NG-5).
- External-device → PLIC direct-IRQ delivery (MANUAL_REVIEW #5/#6):
  unchanged.
- `arch/loongarch/` coverage: stub only; second backend neither
  populated nor exercised by this refactor.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 (00 CRIT) | Accepted | `device/intc/{aclint,plic}.rs` relocate to `arch/riscv/device/intc/` under Phase 4. `device/intc/mod.rs` exposes `#[cfg(riscv)] pub type Intc = crate::arch::riscv::device::intc::Intc;` — same shape as `Core`. No trait indirection (R-018). |
| Review | R-002 (00 CRIT) | Accepted | Per-site `pub(in …)` rewrite table in Phase 2c; 8 sites → `pub(in crate::arch::riscv)`, 3 sites → `pub(in crate::arch::riscv::mm)` (R-016). Gate: `rg 'pub\(in crate::(cpu\|isa)::(riscv\|loongarch)' xemu/xcore/src` = 0 hits. |
| Review | R-003 (00 HIGH) | Accepted | All validation uses `X_ARCH=<value> cargo check -p xcore`; no `--features loongarch`. |
| Review | R-010 (01 CRIT) | Accepted | Flat layout carried from round 02. `arch/riscv/{csr,mm,trap,inst,isa,cpu,device}` are direct children. `arch/riscv/cpu/` holds only `{context.rs, debug.rs, mod.rs}`. |
| Review | R-011 (01 HIGH) | Resolved by R-010 + R-016 | Flat layout removes the hoist. Cross-topic visibility addressed per-site in Phase 2c (R-016), not via a blanket re-export. |
| Review | R-012 (01 HIGH) | Resolved by R-010 | Canonical mip-bit paste target is `arch/riscv/trap/interrupt.rs`. |
| Review | R-013 (01 MED) | Accepted | V-UT-2 remains removed. |
| Review | R-014 (01 MED) | Accepted | `arch_isolation.rs` carries symbol-level allow-list at module-graph level. |
| Review | R-015 (01 LOW) | Accepted | Green-bar phrasing uses named targets (see Implementation Plan). |
| Review | R-016 (02 CRIT) | Accepted | Per-site `pub(in …)` table in Phase 2c: 8 arch-wide, 3 mm-local. |
| Review | R-017 (02 CRIT) | Accepted via option (b) | Seam module re-exports `CoreContext`, `PendingTrap`, and any other `lib.rs`/`error.rs`-named concrete arch type as cfg-gated aliases. I-6 / C-5 relax from "exactly one `#[cfg]` line" to "seam file contains only cfg-gated aliases / re-exports." TR-6 documents the choice. |
| Review | R-018 (02 HIGH) | Accepted via option (a) | `TrapIntake`, `InterruptSink`, `IntcSet` dropped. Only `CoreOps` / `DebugOps` cross the seam. M-004 row marked **Partial**; bus residuals deferred to NG-5 follow-ups. |
| Review | R-019 (02 MED) | Accepted | `arch_isolation.rs` is text-level; limitation named in plan; debug-string occurrences pinned in initial allow-list. |
| Review | R-020 (02 MED) | Accepted | 2a/2b are commit-only inside a single PR 2; PR 2 lands 2a+2b+2c atomically. No broken commit on trunk. Merge-gate commands enumerated per phase. |
| Review | R-021 (02 MED) | Accepted | C-2 narrowed to "no semantic edits inside moved files." Phase 4 only rewrites imports; `sync_interrupts` method body is byte-identical. |
| Review | R-022 (02 LOW) | Accepted | `arch_isolation.rs` uses `std` only. No `aho-corasick` dev-dep. NG-7 holds. |
| Review | TR-2 (01) | Closed — Accepted in round 02 | Back-compat aliases kept (`cpu::Core`, `isa::IMG`, `device::intc::Intc`, plus now `cpu::CoreContext`, `cpu::PendingTrap`). |
| Review | TR-3 (01) | Closed — Accepted in round 02 | `IrqState` stays arch-neutral in `device/mod.rs`. |
| Review | TR-4 (01) | Closed — Accepted in round 02 | Phased PRs. |
| Review | TR-1, TR-5 (01) | Closed | TR-1 by 00-M-001; TR-5 by R-010 / 00-M-002. |
| Master | 00-M-001 | Applied | No global `trait Arch`. `CoreOps` / `DebugOps` are the fine-grained contracts. |
| Master | 00-M-002 | Applied | `arch/riscv/` flat at topic level. mip bits in `arch/riscv/trap/interrupt.rs`. No `irq_bits.rs`. |
| Master | 01-M-001 | Applied | No identifier `selected`. Direct-path cfg-gated `pub type` / `pub use` at each seam. |
| Master | 01-M-002 | Applied | Plan shortened; ceremonial-trait material removed; Trade-offs shrink to TR-6 only. |
| Master | 01-M-003 | Applied | No `compile_error!` in source; `build.rs` authoritative. |
| Master | 01-M-004 | **Partial** | `CoreOps` / `DebugOps` cross the seam and dispatch arch behaviour. Concrete arch data types (`CoreContext`, `PendingTrap`) cross as cfg-gated aliases (consumed by name / field). Bus-level residuals (`Bus::aclint_idx`, etc.) remain — deferred to `aclintSplit` / `plicGateway` / `directIrq`. Honest scoping per R-018. |
| Master | 02_MASTER | — | Blank; no new directives. |

> Rules:
> - Every prior CRITICAL / HIGH finding (R-001, R-002, R-003, R-010, R-011, R-012, R-016, R-017, R-018) appears above.
> - Every MASTER directive (00-M-001, 00-M-002, 01-M-001..M-004; 02 blank) appears above.
> - Rejections: none. Deviations from round-02 text are per-finding, with explicit review citation.

---

## Spec

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

## Implement

### Execution Flow

[**Main Flow**]

1. **Phase 1 — arch/ skeleton.** Create `arch/{mod.rs, riscv/mod.rs,
   loongarch/mod.rs}`. `arch/mod.rs` is the two-line `#[cfg]` switch.
   Add `mod arch;` to `lib.rs`. Existing `cfg_if!` blocks in
   `cpu/mod.rs`, `isa/mod.rs`, `device/intc/mod.rs` remain untouched
   — seam switch happens in Phase 2c.
2. **Phase 2a — relocate.** `git mv cpu/riscv arch/riscv/cpu` and
   `git mv isa/riscv arch/riscv/isa`. **Non-green commit; internal to
   PR 2.**
3. **Phase 2b — flat hoist.** `git mv` each nested topic out of
   `arch/riscv/cpu/` to `arch/riscv/<topic>/`:
   `csr`, `mm`, `trap`, `inst` (and their top-level `.rs` files).
   After hoist, `arch/riscv/cpu/` contains only
   `{context.rs, debug.rs, mod.rs}`. Populate `arch/riscv/mod.rs` per
   Data Structure. **Non-green commit; internal to PR 2.**
4. **Phase 2c — seam switch + per-site `pub(in …)` rewrite.**
   - `cpu/mod.rs`: replace the `cfg_if!` block with the cfg-gated
     alias block (`Core`, `CoreContext`, `PendingTrap`). Remove
     `mod riscv; pub use self::riscv::*;`.
   - `isa/mod.rs`: replace the `cfg_if!` block with `#[cfg(riscv)] pub
     use crate::arch::riscv::isa::{IMG, DECODER, DecodedInst, RVReg};`.
   - Apply the 11-site `pub(in …)` rewrite table (below).
   - Fix `super::` / `crate::` hops inside `arch/riscv/` that broke on
     relocation — mechanical.
   - **Green-bar at PR 2 boundary** (after 2a+2b+2c land together).
5. **Phase 3 — LoongArch.** `git mv cpu/loongarch arch/loongarch/cpu`;
   same for `isa/loongarch`. Add `#[cfg(loongarch)]` arms to the seam
   aliases (only `Core` and `IMG` exist for LoongArch today; add
   others when that backend materialises).
6. **Phase 4 — ACLINT / PLIC + mip bits + intc alias.**
   - `git mv device/intc/{aclint,plic}.rs arch/riscv/device/intc/`.
     Create `arch/riscv/device/mod.rs` (`pub mod intc;`) and
     `arch/riscv/device/intc/mod.rs` (`pub mod aclint; pub mod plic;`
     plus the `Intc` bundle struct).
   - Cut `SSIP/MSIP/STIP/MTIP/SEIP/MEIP/HW_IP_MASK` from
     `device/mod.rs:55-72`. Paste into `arch/riscv/trap/interrupt.rs`.
     Body byte-identical.
   - Replace `device/intc/mod.rs` body with
     `#[cfg(riscv)] pub type Intc = crate::arch::riscv::device::intc::Intc;`.
   - Rewrite imports in the relocated `aclint.rs` / `plic.rs`:
     `use crate::device::{…, MSIP, MTIP, …}` →
     `use crate::{arch::riscv::trap::interrupt::{MSIP, MTIP}, device::{Device, IrqState, mmio_regs}};`
     (analogous for PLIC / `MEIP`, `SEIP`).
   - In `arch/riscv/cpu/mod.rs`: replace
     `use crate::device::intc::{aclint::Aclint, plic::Plic};` with
     `use super::device::intc::{Intc, aclint::Aclint, plic::Plic};`,
     and `use crate::device::HW_IP_MASK;` with
     `use super::trap::interrupt::HW_IP_MASK;`. `sync_interrupts`
     method body is byte-identical.
7. **Phase 5 — arch_isolation test + docs.**
   - Land `xemu/xcore/tests/arch_isolation.rs` and its initial
     allow-list (symbol table inlined into the test source; no
     separate file → no NG-7 risk).
   - Update `lib.rs` rustdoc lead to reference `arch/<name>/`.
   - Update `docs/DEV.md` architecture note if it references old paths.
   - Run `make fmt && make clippy && make run && make test`.

**Phase 2c per-site `pub(in …)` rewrite table:**

| # | File (new path)                  | Line(s)   | Old scope            | New scope                        | Reason                                                                 |
|---|----------------------------------|-----------|----------------------|----------------------------------|------------------------------------------------------------------------|
| 1 | `arch/riscv/trap.rs`             | 21        | `cpu::riscv`         | `arch::riscv`                    | `trap()` called from `inst/*`                                          |
| 2 | `arch/riscv/trap.rs`             | 26        | `cpu::riscv`         | `arch::riscv`                    | `trap_exception()` called from `inst/*`, `mm`, `csr`                   |
| 3 | `arch/riscv/trap.rs`             | 31        | `cpu::riscv`         | `arch::riscv`                    | `illegal_inst()` called from `inst/*`                                  |
| 4 | `arch/riscv/trap.rs`             | 35        | `cpu::riscv`         | `arch::riscv`                    | `trap_on_err()` called from `inst/*`, `mm`                             |
| 5 | `arch/riscv/trap.rs`             | 55        | `cpu::riscv`         | `arch::riscv`                    | `mod test_helpers` used from `mm`, `csr/ops`, `inst/{privileged,zicsr,atomic,compressed}` |
| 6 | `arch/riscv/csr.rs`              | 95        | `cpu::riscv`         | `arch::riscv`                    | `find_desc()` called from `arch::riscv::cpu::debug`                    |
| 7 | `arch/riscv/csr/ops.rs`          | 7         | `cpu::riscv`         | `arch::riscv`                    | `csr_read()` called from `inst/zicsr`                                  |
| 8 | `arch/riscv/csr/ops.rs`          | 16        | `cpu::riscv`         | `arch::riscv`                    | `csr_write()` called from `inst/zicsr`                                 |
| 9 | `arch/riscv/mm/tlb.rs`           | 10        | `cpu::riscv`         | `arch::riscv::mm`                | `Tlb` / `TlbEntry` are genuinely mm-local                              |
| 10| `arch/riscv/mm/tlb.rs`           | 58        | `cpu::riscv`         | `arch::riscv::mm`                | ditto                                                                  |
| 11| `arch/riscv/mm/mmu.rs`           | 20        | `cpu::riscv`         | `arch::riscv::mm`                | `Mmu::tlb` field is mm-local                                           |

8 sites widen to `pub(in crate::arch::riscv)`; 3 stay
`pub(in crate::arch::riscv::mm)`. Gate:
`rg 'pub\(in crate::(cpu|isa)::(riscv|loongarch)' xemu/xcore/src` = 0
hits.

[**Failure Flow**]

1. **Phase 2c build fails with `E0603 module private`.** A `pub(in …)`
   scope is too narrow for an actual cross-topic consumer. Consult the
   table; if the consumer lives outside the declared topic, widen to
   `pub(in crate::arch::riscv)`. If the consumer is outside `arch/`,
   that is an I-1 violation — relocate the call-site into `arch/` or
   expose the item via `CoreOps` / `DebugOps`.
2. **`arch_isolation.rs` fails after Phase 4.** For each offending
   `(file, literal)` pair, either (a) the literal is a known NG-5
   residual or debug-string — add to the allow-list with a
   `// TODO(aclintSplit/plicGateway/directIrq):` anchor, or (b)
   relocate the code into `arch/`. New unreferenced occurrences are
   always (b).
3. **`X_ARCH=loongarch32 cargo check -p xcore` fails after Phase 3.**
   A RISC-V-only symbol leaked into the neutral layer. Relocate into
   `arch/riscv/` or gate with `#[cfg(riscv)]`. No placeholder
   LoongArch bindings.
4. **`git log --follow` shows no pre-refactor history.** The move was
   copy+delete. Redo as `git mv` and amend the phase commit.
5. **Downstream (`xdb` / `xam` / `xemu`) fails to compile against the
   refactored `xcore`.** Most likely an I-4 regression. Check whether
   any previously re-exported name from `cpu::riscv::mod.rs:15` is
   missing from the new `cpu/mod.rs` seam alias block. If yes, add
   the missing `pub type`.

[**State Transition**]

- **S0** pre-refactor: current tree.
- **S1** after PR 1 (Phase 1): empty `arch/` skeleton exists; `cfg_if!`
  blocks still mediate dispatch. **Green-bar: `cargo test --workspace`,
  `make linux`, `make debian` pass.**
- **S2** after PR 2 (Phases 2a+2b+2c, atomic): RISC-V physically flat
  under `arch/riscv/`; seam aliases are the only arch-aware lines in
  `cpu/mod.rs` / `isa/mod.rs`. **Green-bar: full matrix (G-6).**
- **S3** after PR 3 (Phase 3): LoongArch relocated.
  **Green-bar: `X_ARCH=loongarch{32,64} cargo check -p xcore` plus
  full matrix for default `X_ARCH=riscv32`.**
- **S4** after PR 4 (Phase 4): ACLINT / PLIC / mip bits / `Intc`
  bundle under `arch/riscv/`; `device/mod.rs` arch-neutral.
  **Green-bar: full matrix (G-6).**
- **S5** after PR 5 (Phase 5): `arch_isolation.rs` locks I-1 and I-2;
  docs updated. **Green-bar: full matrix (G-6) + `arch_isolation.rs`
  passes.**

### Implementation Plan

Phase-level PR titles:

- **PR 1** — `refactor(xcore): introduce arch/ skeleton`
- **PR 2** — `refactor(xcore): relocate riscv cpu/isa/topics to arch/riscv/` (commits: 2a, 2b, 2c)
- **PR 3** — `refactor(xcore): relocate loongarch stubs under arch/loongarch/`
- **PR 4** — `refactor(xcore): move aclint/plic + mip bits under arch/riscv/`
- **PR 5** — `test(xcore): arch_isolation structural test + docs`

Green-bar command set per PR (C-1 / G-6 / R-020):

```sh
# run after every PR boundary, from repo root
X_ARCH=riscv32 cargo test --workspace && \
X_ARCH=riscv64 cargo test --workspace && \
make cpu-tests-rs DEBUG=n && \
make am-tests    DEBUG=n && \
make linux       DEBUG=n && \
make debian      DEBUG=n
# expected: exit code 0 for each; zero difftest divergence.

# LoongArch compile gate (PR 3 onward)
X_ARCH=loongarch32 cargo check -p xcore && \
X_ARCH=loongarch64 cargo check -p xcore
# expected: exit code 0.

# arch-isolation gate (PR 5 onward)
cargo test -p xcore --test arch_isolation
# expected: exit code 0.
```

Phase bodies (intentionally terse — details captured in Main Flow):

- **PR 1.** Create 3 empty module files + `mod arch;` in `lib.rs`.
- **PR 2.** 6 `git mv` calls (2a: 2; 2b: 4) + the Phase 2c seam-switch
  edits (`cpu/mod.rs`, `isa/mod.rs`, populate `arch/riscv/mod.rs`) +
  the 11-site `pub(in …)` rewrite + intra-arch `super::` / `crate::`
  fixups. Three commits, one PR, atomic merge.
- **PR 3.** 2 `git mv` calls + two `#[cfg(loongarch)]` lines.
- **PR 4.** 2 `git mv` calls + one const block move (`device/mod.rs`
  → `arch/riscv/trap/interrupt.rs`) + one alias line in
  `device/intc/mod.rs` + 3 import-line rewrites (aclint, plic,
  arch/riscv/cpu/mod.rs).
- **PR 5.** 1 new test file + rustdoc / DEV.md path updates +
  `make fmt && make clippy && make run && make test`.

---

## Trade-offs

Only the round-03 new trade-off is open:

- **TR-6: `CoreContext` / `PendingTrap` crossing the seam — concrete
  alias vs associated-type on `CoreOps`.**
  - (a) *Associated types on `CoreOps`:* `trait CoreOps { type Context;
    type TrapPayload; … }`. `lib.rs` re-exports `<Core as
    CoreOps>::Context`. Pro: arch data type is fully behind a trait —
    the purest M-004 landing. Con: `xdb/src/difftest/{qemu,spike}.rs`
    reads `CoreContext` by **field** (`dut.pc`, `dut.gprs`,
    `dut.privilege`, `dut.csrs`); turning `CoreContext` into an
    associated type does not remove the struct — it forces each
    consumer to name `<Core as CoreOps>::Context` — still a concrete
    struct under the alias. Zero M-004 gain for real work; larger
    diff; risks I-4 regression.
  - (b) *Concrete cfg-gated alias (chosen):*
    `#[cfg(riscv)] pub type CoreContext = crate::arch::riscv::cpu::context::RVCoreContext;`.
    Pro: zero `xdb` source edits; trivial; one line per type. Con:
    the seam file now has N cfg-gated aliases instead of exactly one —
    I-6 / C-5 relax to "aliases only." Honest landing of M-004:
    behaviour (traits) crosses the seam; data (concrete structs) is
    named through aliases because it's consumed by name/field.

  **Choice: (b).** Reasons: preserves I-4 (no `xdb` edits); keeps
  diff minimal; does not pretend trait dispatch where there is none.
  The residual M-004 ambition (associated-type `Context` /
  `TrapPayload`) belongs in a later refactor that also replumbs
  `xdb`'s difftest field access — out of scope for round 03.

Closed from round 02: TR-1 (00-M-001), TR-2 (accepted), TR-3
(accepted), TR-4 (accepted), TR-5 (R-010).

---

## Validation

[**Unit Tests**]

- **V-UT-1: Arch-isolation structural test.**
  `xemu/xcore/tests/arch_isolation.rs` — a `std`-only integration test
  (no dev-deps; NG-7 / R-022). Walks every `.rs` file under
  `xemu/xcore/src/` with `std::fs::read_to_string`, iterates a
  hard-coded vocabulary slice of `&[&str]`, and checks each via
  `str::find` / `str::contains`. Assertions:
  1. **Path isolation (I-1).** For each file whose relative path does
     not start with `src/arch/`, `content.contains("crate::arch::riscv::")`
     and `content.contains("crate::arch::loongarch::")` are both
     false, unless the file is one of the seam files
     `src/arch/mod.rs`, `src/cpu/mod.rs`, `src/isa/mod.rs`,
     `src/device/intc/mod.rs`. Those files are inspected
     line-by-line: only lines matching the exact symbol allow-list
     (`Core`, `CoreContext`, `PendingTrap`, `IMG`, `DECODER`,
     `DecodedInst`, `RVReg`, `Intc`) are permitted to reference
     `crate::arch::riscv::`.
  2. **Vocabulary isolation (I-2).** For each file outside `src/arch/`
     and outside the seam files, none of the identifier literals
     (`MSIP`, `MTIP`, `MEIP`, `SEIP`, `SSIP`, `STIP`, `RVCore`,
     `Mstatus`, `Mip`, `Sv32`, `Sv39`) appear. Explicit per-file
     allow-list for NG-5 residuals (baked into the test source):
     - `src/device/bus.rs` — allow: `aclint_idx`, `plic_idx`,
       `set_timer_source`, `set_irq_sink`, `ssip_flag`, `take_ssip`,
       `mtime` (one occurrence each, counted); the two debug-log
       strings `"aclint"` and `"plic"` in `info!` call sites.
  3. **No `selected` (01-M-001).** `src/` tree contains zero
     occurrences of the bare identifier `\bselected\b`.
  4. **No in-source arch check (01-M-003).** `src/arch/` contains
     zero occurrences of `compile_error!`.
  **Limitation explicitly named (R-019):** the test is text-level and
  cannot distinguish identifiers from strings / comments / macro
  expansion. Known false-positive sources (`info!("plic: …")`,
  `info!("aclint: …")`) are pinned per-occurrence. Any new occurrence
  outside the allow-list fails CI; contributors must either widen the
  allow-list with justification or relocate the code.
- **V-UT-2:** (removed — R-013.)

[**Integration Tests**]

- **V-IT-1: `cargo test --workspace` (336 tests).** Passes at every
  trunk-bound PR boundary (PR 1, PR 2, PR 3, PR 4, PR 5). (Not at the
  internal commits 2a / 2b — those are non-green by design, R-020.)
- **V-IT-2: `make cpu-tests-rs` (31 tests).** Passes at every PR
  boundary.
- **V-IT-3: `make am-tests` (8 tests: UART, ACLINT, PLIC, CSR, trap,
  interrupts, float, keyboard).** Passes at every PR boundary; PR 4
  is the critical one (ACLINT / PLIC relocation).
- **V-IT-4: `make linux` + `make debian`.** `make linux` boots to
  interactive Buildroot BusyBox shell. `make debian` boots Debian 13
  Trixie (4 GB ext4 rootfs via VirtIO-blk) to shell and runs Python3.
  Both measured with `DEBUG=n`; wall-clock within ±5% of the
  pre-refactor baseline.

[**Failure / Robustness Validation**]

- **V-F-1: LoongArch compile gate.**
  `X_ARCH=loongarch32 cargo check -p xcore` and
  `X_ARCH=loongarch64 cargo check -p xcore` succeed from PR 3 onward
  and stay green through PRs 4–5.
- **V-F-2: Difftest vs QEMU / Spike.** Zero divergence on the
  `cpu-tests-rs` suite at every PR boundary (PR 2, PR 4 are the
  at-risk ones).
- **V-F-3: Git history.** `git log --follow` on
  `arch/riscv/cpu/mod.rs`, `arch/riscv/isa/decoder.rs`,
  `arch/riscv/trap/interrupt.rs`, `arch/riscv/csr/mip.rs`,
  `arch/riscv/mm/mmu.rs`, `arch/riscv/inst/base.rs`,
  `arch/riscv/device/intc/aclint.rs`,
  `arch/riscv/device/intc/plic.rs` each traces pre-refactor.
- **V-F-4: Dep diff.** `cargo tree -p xcore` output at HEAD matches
  pre-refactor. No dep change (prod or dev). NG-7 / C-4.
- **V-F-5: `pub(in …)` gate.**
  `rg 'pub\(in crate::(cpu|isa)::(riscv|loongarch)' xemu/xcore/src`
  returns 0 hits after PR 2.

[**Edge Case Validation**]

- **V-E-1: Re-export hygiene.** `cargo build -p xcore` emits no
  `ambiguous_glob_reexports` or `unused_imports` warnings under the
  new seam aliases.
- **V-E-2: Downstream source-compat.** `cargo build -p xdb`,
  `cargo build -p xam`, `cargo build -p xemu` succeed against the
  refactored `xcore` with zero source edits. Difftest's field access
  on `CoreContext` resolves. (Protects I-4.)

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (flat arch/ topic layout) | V-F-3, V-UT-1 |
| G-2 (seam file contains only cfg-gated aliases / re-exports) | V-UT-1, V-E-1 |
| G-3 (mip bits in `arch/riscv/trap/interrupt.rs`) | V-UT-1, V-IT-3 |
| G-4 (ACLINT / PLIC relocated; `Intc` alias in `device/intc/mod.rs`) | V-UT-1, V-F-3, V-IT-3 |
| G-5 (git history preserved) | V-F-3 |
| G-6 / 336 unit tests | V-IT-1 |
| G-6 / 31 cpu-tests-rs | V-IT-2 |
| G-6 / 8 am-tests | V-IT-3 |
| G-6 / `make linux` boots | V-IT-4 |
| G-6 / `make debian` boots to Python3 | V-IT-4 |
| G-6 / difftest vs QEMU & Spike zero divergence | V-F-2 |
| I-1 (no `crate::arch::…` outside arch/ or seams) | V-UT-1 |
| I-2 (no RISC-V vocabulary outside arch/ or allow-list) | V-UT-1 |
| I-3 (git history) | V-F-3 |
| I-4 (public API unchanged) | V-E-2 |
| I-5 (behaviour unchanged) | V-IT-1, V-IT-2, V-IT-3, V-IT-4, V-F-2 |
| I-6 (seam file shape: aliases only) | V-UT-1 |
| C-1 (PR boundaries green) | V-IT-1..V-IT-4 per PR |
| C-2 (no semantic edits inside moved files) | V-F-2, V-IT-2 |
| C-3 (`git mv` only) | V-F-3 |
| C-4 (no dep change) | V-F-4 |
| C-5 (seam shape) | V-UT-1 |
| NG-5 (Bus residuals pinned) | V-UT-1 allow-list |
| 00-M-001 (no global `trait Arch`) | Static review against `cpu/core.rs`, `cpu/debug.rs` |
| 00-M-002 (flat topic layout; no `irq_bits.rs`) | V-F-3; `ls arch/riscv/` post-condition |
| 01-M-001 (no `selected`) | V-UT-1 clause 3 |
| 01-M-002 (clean plan) | Plan length vs 02_PLAN |
| 01-M-003 (no in-source arch check) | V-UT-1 clause 4 |
| 01-M-004 (trait dispatch) — **Partial** | `CoreOps` / `DebugOps` cross the seam (unchanged); bus-level residuals queued (NG-5); honest scoping recorded in Response Matrix |
| V-F-5 / R-002 / R-016 (`pub(in …)` gate) | `rg 'pub\(in crate::(cpu\|isa)::(riscv\|loongarch)'` = 0 hits |
