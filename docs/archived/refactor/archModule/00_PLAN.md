# `archModule` PLAN `00`

> Status: Draft
> Feature: `archModule`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`
> - Master Directive: `none`

---

## Summary

Consolidate the two parallel arch sub-trees (`cpu/riscv`, `cpu/loongarch`, `isa/riscv`,
`isa/loongarch`) into a single `xcore/src/arch/` module. After this change, **every
arch-specific file lives under exactly one folder per arch** (`arch/riscv/*`,
`arch/loongarch/*`), and the top-level `cpu/` and `isa/` modules contain only arch-neutral
interfaces plus a single `cfg_if` seam that selects which arch is active.

Addresses [MANUAL_REVIEW.md](../../MANUAL_REVIEW.md) items **#3** and **#4**. This plan is
scoped tightly: **structural relocation + isolation of one device-layer RISC-V leak**. It
does *not* change ISA semantics, boot flows, or the `CoreOps` / `Device` contracts.

## Log `{None in 00_PLAN}`

---

## Spec

### Goals

- G-1: Create `xcore/src/arch/` with two children: `arch/riscv/` and `arch/loongarch/`.
  Each child re-exports the code previously split across `cpu/<arch>/` and `isa/<arch>/`.
- G-2: `cpu/mod.rs` and `isa/mod.rs` each keep exactly one `cfg_if` block that maps to
  `arch::riscv` or `arch::loongarch`. All other files under `cpu/` and `isa/` top-level
  must be arch-neutral (no `riscv::` / `loongarch::` paths).
- G-3: Move the RISC-V mip bit constants (`SSIP`/`MSIP`/`STIP`/`MTIP`/`SEIP`/`MEIP`,
  `HW_IP_MASK`) out of `device/mod.rs` into `arch/riscv/`. `device/mod.rs` exposes them
  back only through a neutral re-export (e.g. `arch::selected::irq_bits`). `IrqState`
  itself stays in `device/` because its `AtomicU64` storage is arch-neutral.
- G-4: Preserve git history for moved files via `git mv` (no copy+delete).

- NG-1: No change to `CoreOps`, `Device`, `BootConfig`, `BootLayout`, `MachineConfig`.
- NG-2: No new `Arch` trait with associated types. The current `cfg_if` seam is what
  we unify on; introducing `trait Arch { type Word; … }` is deliberately deferred. This
  keeps the diff small and leaves room for a future `Arch`-trait plan if/when a second
  live arch backend is needed.
- NG-3: No change to xdb, xlogger, xam, xlib, difftest, am-tests, or benchmarks.
- NG-4: No change to boot configs, DTS files, or `make` targets.

### Architecture

Before:

```
xcore/src/
├── cpu/
│   ├── mod.rs        (cfg_if -> riscv | loongarch)
│   ├── core.rs       (arch-neutral CoreOps)
│   ├── debug.rs      (arch-neutral DebugOps)
│   ├── riscv/        ← arch-specific
│   └── loongarch/    ← arch-specific
├── isa/
│   ├── mod.rs        (cfg_if -> riscv | loongarch)
│   ├── instpat/      (arch-neutral pest glue)
│   ├── riscv/        ← arch-specific
│   └── loongarch/    ← arch-specific
└── device/
    ├── mod.rs        (contains RISC-V mip bits — leak)
    └── ...
```

After:

```
xcore/src/
├── arch/
│   ├── mod.rs            (cfg_if selects active arch; re-exports `selected`)
│   ├── riscv/
│   │   ├── mod.rs        (re-exports cpu::* and isa::*)
│   │   ├── cpu/          ← moved from cpu/riscv/
│   │   ├── isa/          ← moved from isa/riscv/
│   │   └── irq_bits.rs   ← SSIP/MSIP/STIP/MTIP/SEIP/MEIP + HW_IP_MASK
│   └── loongarch/
│       ├── mod.rs
│       ├── cpu/          ← moved from cpu/loongarch/
│       └── isa/          ← moved from isa/loongarch/
├── cpu/
│   ├── mod.rs            (cfg_if -> arch::selected::cpu; re-export identical to before)
│   ├── core.rs           (unchanged)
│   └── debug.rs          (unchanged)
├── isa/
│   ├── mod.rs            (cfg_if -> arch::selected::isa)
│   └── instpat/          (unchanged)
└── device/
    ├── mod.rs            (IrqState stays; mip bits now re-exported from arch::selected)
    └── ...
```

The `cfg_if` pattern is preserved, but now points into `arch/` instead of fanning out
across three top-level trees. Existing `pub use self::riscv::*` semantics are kept
verbatim so downstream crates (xdb, xam) compile unchanged.

### Invariants

- I-1: No file under `xcore/src/{cpu,device,isa,utils,config,error}.rs` or any
  non-`arch/` subdirectory may `use crate::arch::riscv::…` or
  `use crate::arch::loongarch::…` by concrete path. Arch access is **only** via
  `crate::arch::selected::…` or the `cfg_if` seam inside `cpu/mod.rs` / `isa/mod.rs`.
- I-2: `git log --follow` on any moved file must still show its full pre-refactor
  history.
- I-3: Every public item currently exported from `crate::cpu::*` and `crate::isa::*`
  remains exported with the same name and same module path. Downstream (`xdb`, `xam`,
  `xemu` binary) compiles with zero source changes.
- I-4: `cargo test --workspace`, `make cpu-tests-rs`, `make am-tests`, `make linux`, and
  `make debian` all produce the same pass/fail and boot artefacts as pre-refactor.

### Data Structures

No new data types. The only new module is `arch::selected`, which is a type alias
produced by `cfg_if`:

```rust
// arch/mod.rs
cfg_if::cfg_if! {
    if #[cfg(riscv)] {
        pub mod riscv;
        pub use self::riscv as selected;
    } else if #[cfg(loongarch)] {
        pub mod loongarch;
        pub use self::loongarch as selected;
    }
}

// arch/riscv/mod.rs
pub mod cpu;
pub mod isa;
pub mod irq_bits;

// Back-compat re-exports so existing `use crate::cpu::riscv::foo` paths keep working
// during the transition phase and can be dropped in a later cleanup.
```

### API Surface

Public API of `xcore` is unchanged:

```rust
// lib.rs (unchanged)
pub use cpu::{BootConfig, CoreContext, RESET_VECTOR, State, XCPU, debug::{...}, with_xcpu};
pub use device::uart::Uart;
pub use error::{XError, XResult};
```

Internal re-exports change shape but not names:

```rust
// cpu/mod.rs (after)
cfg_if::cfg_if! {
    if #[cfg(riscv)] {
        pub use crate::arch::riscv::cpu::*;
    } else if #[cfg(loongarch)] {
        pub use crate::arch::loongarch::cpu::*;
    }
}

// device/mod.rs (after)
pub use crate::arch::selected::irq_bits::{SSIP, MSIP, STIP, MTIP, SEIP, MEIP, HW_IP_MASK};
```

### Constraints

- C-1: Landable as a single PR that keeps `cargo test --workspace` and both Linux +
  Debian boots green.
- C-2: No semantic edits inside moved files beyond import-path adjustments needed to
  compile after relocation.
- C-3: Git history for every moved file preserved via `git mv` (CI can add a
  `git log --follow` smoke check if desired).
- C-4: No MSRV / edition / dependency changes.

---

## Implement

### Execution Flow

**Main Flow**

1. Create `xcore/src/arch/` with an empty `mod.rs` and empty `riscv/`, `loongarch/`
   subdirs. Register `mod arch;` in `lib.rs`.
2. `git mv xemu/xcore/src/cpu/riscv xemu/xcore/src/arch/riscv/cpu`
   (and analogous for `isa/riscv`, `cpu/loongarch`, `isa/loongarch`).
3. Wire `arch/riscv/mod.rs` and `arch/loongarch/mod.rs` to `pub mod cpu; pub mod isa;`.
4. Rewrite `cpu/mod.rs` and `isa/mod.rs` `cfg_if` blocks to point at `arch::selected::*`.
5. Rewrite the arch-internal imports that broke due to the move (`super::`, `crate::cpu::riscv::`,
   `crate::isa::riscv::`) — mechanical path fixup only.
6. Extract RISC-V mip bit constants from `device/mod.rs` into
   `arch/riscv/irq_bits.rs`; re-export them from `device/mod.rs` via
   `pub use crate::arch::selected::irq_bits::*`.
7. Run `cargo test --workspace`, `make cpu-tests-rs`, `make am-tests`, `make linux`,
   `make debian`. All must pass.

**Failure Flow**

1. If a top-level file still needs a concrete arch import after the move, that's an
   architecture smell (not a mechanical issue). Log the offender; either push the call
   down into `arch/<name>/` or add a neutral accessor on a shared trait — but do **not**
   paper over with a direct `use crate::arch::riscv::…`.
2. If `cargo build --no-default-features --features loongarch` fails due to RISC-V code
   leaking out of `arch/`, that is a G-2 violation; fix by relocating the offender into
   `arch/riscv/`.
3. If a move breaks `git log --follow` for a file, redo the move via `git mv` instead of
   copy+delete.

**State Transition**

- S0: pre-refactor — four sibling dirs (`cpu/riscv`, `cpu/loongarch`, `isa/riscv`,
  `isa/loongarch`); `device/mod.rs` holds RISC-V mip bits.
- S1: `arch/` exists but is empty; build still green (nothing wired yet).
- S2: RISC-V tree moved under `arch/riscv/`; `cpu/mod.rs` + `isa/mod.rs` rewired; build
  green.
- S3: LoongArch tree moved under `arch/loongarch/`; build green in both `riscv` and
  `loongarch` feature configurations.
- S4: RISC-V mip bits relocated to `arch/riscv/irq_bits.rs`; `device/mod.rs` re-exports;
  build green.
- S5: Grep-assertion passes: zero `use crate::arch::riscv::` or
  `use crate::arch::loongarch::` hits outside `arch/`.

### Implementation Plan

**Phase 1 — Introduce `arch/` skeleton (no moves yet)**

- Add `xemu/xcore/src/arch/mod.rs` with the `cfg_if` skeleton and empty `riscv`,
  `loongarch` submodules (each `mod.rs` is empty or a `//!` placeholder).
- Register `mod arch;` in `lib.rs` (before `mod cpu;` and `mod isa;` so they can refer to
  it).
- Verify: `cargo build` still green (nothing else changed).

**Phase 2 — Relocate RISC-V subtree**

- `git mv xemu/xcore/src/cpu/riscv xemu/xcore/src/arch/riscv/cpu`
- `git mv xemu/xcore/src/isa/riscv xemu/xcore/src/arch/riscv/isa`
- Populate `arch/riscv/mod.rs` with `pub mod cpu; pub mod isa;`.
- Rewrite `cpu/mod.rs` RISC-V arm to `pub use crate::arch::riscv::cpu::*;`.
- Rewrite `isa/mod.rs` RISC-V arm to `pub use crate::arch::riscv::isa::*;`.
- Fix up moved files' internal imports (`super::` hops that broke by the move). The
  pattern is: `super::super::CoreOps` → `crate::cpu::core::CoreOps`, etc.
- Verify: `cargo test --workspace`, `make cpu-tests-rs`, `make am-tests`, `make linux`,
  `make debian` all green under default (RISC-V) feature set.

**Phase 3 — Relocate LoongArch subtree**

- Same as Phase 2 for `cpu/loongarch` → `arch/loongarch/cpu` and `isa/loongarch` →
  `arch/loongarch/isa`.
- Verify: `cargo build --no-default-features --features loongarch` green. (LoongArch is a
  stub today so runtime tests may not exist; a clean `cargo check` is the acceptance
  bar.)

**Phase 4 — Move the `device/mod.rs` arch leak**

- Create `xemu/xcore/src/arch/riscv/irq_bits.rs` containing SSIP/MSIP/STIP/MTIP/
  SEIP/MEIP + `HW_IP_MASK` (cut from `device/mod.rs`).
- Replace their `device/mod.rs` definitions with
  `pub use crate::arch::selected::irq_bits::*;`.
- Add an empty (or feature-gated) `arch/loongarch/irq_bits.rs` stub so the `selected`
  alias resolves in LoongArch builds.
- Verify: full test + boot matrix green; grep assertion passes (see V-F-2).

**Phase 5 — Docs touch-up**

- Update `lib.rs` doc comment (currently mentions `cfg(riscv)` / `cfg(loongarch)`) to
  mention the new `arch/` module.
- Update `docs/DEV.md` "What Works" section's architecture note if it references old
  paths.
- No changes to README — it speaks at a higher level.

---

## Trade-offs

- T-1: **Keep `cfg_if` seam vs introduce a real `trait Arch` with associated types.**
  - Option A (this plan): keep `cfg_if`, only relocate. Minimal diff; no behaviour
    change; unblocks MANUAL_REVIEW #3/#4 immediately.
  - Option B: introduce `trait Arch { type Word; type PhysAddr; … }` and parameterise
    `Bus<A>`, `Cpu<A>`. Stronger enforcement of I-1, but a much larger diff, touches
    every generic signature in the workspace, and risks regressing OS boot timings.
  - Recommendation: Option A now. If a second live arch (not stub) lands later, open a
    follow-up `archTrait` plan to do Option B.

- T-2: **Keep back-compat re-exports at old paths vs hard cut.**
  - Option A: `cpu/mod.rs` re-exports verbatim what `cpu/riscv/` used to export, so
    downstream crates compile unchanged.
  - Option B: hard cut, force every import in xdb/xam to say `crate::arch::riscv::…`.
  - Recommendation: Option A. The goal here is enforcement at the `xcore/src/` top
    level, not a cosmetic rename for downstream crates.

- T-3: **Where does `IrqState` live?**
  - Option A (this plan): keep `IrqState` in `device/mod.rs` because its storage
    (`Arc<AtomicU64>`) is arch-neutral; only the **bit-layout semantics** (which bit is
    MSIP vs SEIP) are arch-specific, and those move to `arch/<name>/irq_bits.rs`.
  - Option B: move `IrqState` entirely into `arch/riscv/`.
  - Recommendation: Option A. The interrupt-lines refactor (MANUAL_REVIEW #5/#6) is a
    separate plan; moving `IrqState` prematurely conflicts with that design space.

- T-4: **Single PR vs phased PRs.**
  - Option A: one PR covering all five phases.
  - Option B: one PR per phase (Phase 1, Phase 2, Phase 3+4, Phase 5).
  - Recommendation: Option B. Each phase has an independent green-bar check, and per-
    phase review is far easier than reviewing four `git mv` walls simultaneously.

---

## Validation

**Unit Tests**

- V-UT-1: All existing unit tests under `cpu/riscv/**`, `isa/riscv/**`, `device/**`
  (336 total) continue to pass at their new paths. No test source edits.
- V-UT-2: A new tiny test in `arch/mod.rs` asserts that
  `core::any::type_name::<arch::selected::cpu::Core>()` contains `"riscv"` under the
  default feature set — a canary that the seam is wired correctly.

**Integration Tests**

- V-IT-1: `cargo test --workspace` at each phase boundary.
- V-IT-2: `cpu-tests-rs` (31) pass at Phase 2 and again at Phase 4.
- V-IT-3: `am-tests` (UART, ACLINT, PLIC, CSR, trap, interrupts, float, keyboard — 8)
  pass at Phase 4.
- V-IT-4: `make linux` boots to interactive shell; `make debian` boots and runs Python3
  (per DEV.md's acceptance). Wall-clock within ±5% of a pre-refactor baseline.

**Failure / Robustness Validation**

- V-F-1: `cargo build --no-default-features --features loongarch` compiles
  (phases 1, 3, 4 post-landing). Flushes hidden RISC-V deps out of top-level modules.
- V-F-2: `grep -R "crate::arch::riscv\|crate::arch::loongarch" xemu/xcore/src` excluding
  `xemu/xcore/src/arch/`, `xemu/xcore/src/cpu/mod.rs`, `xemu/xcore/src/isa/mod.rs`,
  `xemu/xcore/src/device/mod.rs` must return **zero** matches. (The three allow-listed
  files are the `cfg_if` seam.)
- V-F-3: `git log --follow -- xemu/xcore/src/arch/riscv/cpu/mod.rs` must show history
  back through the pre-refactor `cpu/riscv/mod.rs`. Same for one isa-side file and the
  new `irq_bits.rs`.
- V-F-4: Difftest vs QEMU and Spike on the default cpu-tests-rs set — zero divergence.

**Edge Case Validation**

- V-E-1: Dual feature flags: build `--features riscv,loongarch` must fail fast with a
  clear `cfg_if` error rather than silently picking one. (Current behaviour; we just
  want to confirm we haven't regressed it.)
- V-E-2: Re-export cycle check: `rustc` must not warn about ambiguous re-exports when
  `cpu/mod.rs` re-exports from `arch::riscv::cpu::*` and `arch::riscv::cpu::mod.rs` in
  turn re-exports its submodules.
- V-E-3: `xdb` crate continues to compile and link against `xcore` with zero source
  changes — protects downstream consumers (I-3).

**Acceptance Mapping**

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-UT-1, V-F-3 (files exist at new paths with preserved history) |
| G-2 | V-F-2 (grep assertion) |
| G-3 | V-F-2 on `device/mod.rs`; V-IT-3 (am-tests: ACLINT/PLIC still work) |
| G-4 | V-F-3 |
| C-1 | V-IT-1, V-IT-4 |
| C-2 | V-UT-1 (no test edits needed) |
| C-3 | V-F-3 |
| C-4 | `cargo tree` diff = empty |
