# `archModule` REVIEW `02`

> Status: Open
> Feature: `archModule`
> Iteration: `02`
> Owner: Reviewer
> Target Plan: `02_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking Issues: `3`
- Non-Blocking Issues: `4`



## Summary

Round 02 makes substantial progress: the flat topic layout is now unambiguous,
the `selected` name is gone, `compile_error!` canaries are gone, and the
Response Matrix covers every round-00 and round-01 CRITICAL/HIGH finding and
every MASTER directive. The plan's Architecture, Data Structure, API Surface,
and Execution Flow all consistently reference `arch/riscv/<topic>/…`. Numbering
convention: IDs from rounds 00 and 01 are re-used where the concern
demonstrably persists (audited below); new concerns begin at **R-016** / **TR-6**.

Resolution audit:

- **Round-00 CRITICAL (R-001, R-002):** Both remain resolved. R-001 (device/intc
  leak) is addressed via the `IntcSet` trait + Phase 4 `git mv` of
  `aclint.rs`/`plic.rs` into `arch/riscv/device/intc/`. R-002 (`pub(in …)` paths)
  is enumerated in the Response Matrix with exact line numbers that match
  the codebase (11 sites, cross-checked below).
- **Round-00 HIGH (R-003):** Resolved — all validation now uses
  `X_ARCH=<value> cargo check -p xcore`, which matches
  `xemu/xcore/build.rs:19-33`.
- **Round-01 CRITICAL (R-010):** Resolved. Flat layout is the *sole* canonical
  layout: `arch/riscv/{cpu, csr, mm, trap, inst, isa, device}` with
  `arch/riscv/cpu/` reserved for `{context.rs, debug.rs, mod.rs}` only. No
  residual `arch/riscv/cpu/{csr,mm,trap,inst}` phrasing anywhere.
- **Round-01 HIGH (R-011):** *Partially* resolved at the layout level — the
  `pub use cpu::{csr,inst,mm,trap}` hoist is gone because topics now live
  directly under `arch/riscv/mod.rs` carrying their original visibility.
  However, the `pub(in …)` token rewrite strategy in Phase 2c introduces a
  **new** correctness bug of the same family: it scopes cross-topic helpers
  too narrowly (see R-016 below, blocking).
- **Round-01 HIGH (R-012):** Resolved — mip-bit paste target is uniformly
  `arch/riscv/trap/interrupt.rs` in Summary, G-3, Data Structure, API Surface,
  Execution Flow, and the Response Matrix.
- **Round-00 MASTER (M-001, M-002):** Both applied. No global `trait Arch` is
  introduced; per-concern traits only. No `irq_bits.rs` anywhere; mip-bit
  constants land in `arch/riscv/trap/interrupt.rs`.
- **Round-01 MASTER (M-001, M-002, M-003, M-004):** M-001 (no `selected`),
  M-002 (tight plan), and M-003 (no `compile_error!` / `RUSTFLAGS` canary)
  are cleanly applied. M-004 is the central addition and is structurally
  faithful — traits live in the upper layer, impls live under `arch/` — but
  the *coverage* of the trait surface is incomplete against the concrete arch
  types the upper layer currently names (`CoreContext` via `lib.rs`
  re-export, `PendingTrap` via `crate::error::XError::Trap`). See R-017
  below (blocking).

Remaining blocking concerns collapse to three: **R-016** (Phase 2c `pub(in
crate::arch::riscv::<topic>)` visibility narrowing breaks `test_helpers`,
`find_desc`, `csr_read`/`csr_write`, and `Tlb`/`TlbEntry` cross-topic
callers — the plan would literally not compile as written), **R-017**
(`cpu/mod.rs` seam budget of "exactly one `#[cfg]` line" collides with
`lib.rs` re-exporting `CoreContext` and `error.rs` using `PendingTrap` — the
plan omits aliases for arch types other than `Core`, breaking I-4), and
**R-018** (the `TrapIntake` / `IntcSet` traits are narrower than the actual
cross-boundary surface and risk being ceremonial — their design is not
substantively different from the current direct method calls and does not
abstract the parts M-004 actually targets). Round-02 is *close*; one more
iteration tightens the visibility plan and the trait surface.

Non-blocking: R-019 (V-UT-1 allow-list is text-level, not AST-level — a
leaky but acceptable approximation), R-020 (Phase 2a is explicitly non-green
but lands on-branch; document or merge-gate), R-021 (Phase 4 mutates
`sync_interrupts` under C-2's "no semantic edits" rule — needs a named
exception), R-022 (dev-dep addition of `aho-corasick` conflicts with NG-7
unless explicitly waived).

---

## Findings

### R-016 `Phase 2c pub(in crate::arch::riscv::<topic>) narrows cross-topic visibility and breaks compilation`

- Severity: CRITICAL
- Section: Implementation Plan / Phase 2c, Response Matrix / R-002
- Type: Correctness
- Problem:
  Phase 2c (`02_PLAN.md:556-564`) and the Response Matrix for R-002 (line
  209) commit to a **topic-specific** rewrite:
  - `arch/riscv/trap.rs:21,26,31,35,55 → pub(in crate::arch::riscv::trap)`
  - `arch/riscv/mm/tlb.rs:10,58 → pub(in crate::arch::riscv::mm)`
  - `arch/riscv/mm/mmu.rs:20 → pub(in crate::arch::riscv::mm)`
  - `arch/riscv/csr.rs:95 → pub(in crate::arch::riscv::csr)`
  - `arch/riscv/csr/ops.rs:7,16 → pub(in crate::arch::riscv::csr)`

  Cross-checked against the codebase:
  - `cpu/riscv/trap.rs:55` declares `pub(in crate::cpu::riscv) mod test_helpers`.
    Under the current nested layout, this module is visible anywhere under
    `cpu::riscv`. Under the plan's flat layout + topic-narrow rewrite, it
    would become `pub(in crate::arch::riscv::trap)`, yet it is *actually*
    consumed from six cross-topic sites:
      - `cpu/riscv/mm.rs:340` (→ `arch/riscv/mm.rs`)
      - `cpu/riscv/csr/ops.rs:138` (→ `arch/riscv/csr/ops.rs`)
      - `cpu/riscv/inst/privileged.rs:102` (→ `arch/riscv/inst/…`)
      - `cpu/riscv/inst/zicsr.rs:106` (→ `arch/riscv/inst/…`)
      - `cpu/riscv/inst/atomic.rs:172` (→ `arch/riscv/inst/…`)
      - `cpu/riscv/inst/compressed.rs:362` (→ `arch/riscv/inst/…`)
    All six sites sit **outside** `arch::riscv::trap`. `cargo test -p xcore`
    will fail with `E0603: module test_helpers is private`.
  - `cpu/riscv/csr/ops.rs:7,16` declare `pub(in crate::cpu::riscv) fn
    csr_read / csr_write`. These methods are called from
    `cpu/riscv/inst/zicsr.rs:33,35,37` (→ `arch/riscv/inst/zicsr.rs`),
    which is outside `arch::riscv::csr`. Narrowing to
    `pub(in crate::arch::riscv::csr)` breaks the `inst` topic's access to
    `self.csr_write(…)` / `self.csr_read(…)`. Same failure.
  - `cpu/riscv/csr.rs:95` declares `pub(in crate::cpu::riscv) fn find_desc`,
    called from `cpu/riscv/debug.rs:6,87` (→ `arch/riscv/cpu/debug.rs`),
    which is under `arch::riscv::cpu`, **not** under `arch::riscv::csr`.
    Narrowing to `pub(in crate::arch::riscv::csr)` breaks debug.
  - Only `Tlb` / `TlbEntry` (`mm/tlb.rs:10,58`) and `Mmu::tlb` field
    (`mm/mmu.rs:20`) are genuinely `mm`-local — `pub(in crate::arch::riscv::mm)`
    is fine for those four sites.

  So of the 11 sites, **7 must be scoped at the arch level, not the topic
  level**: the five in `arch/riscv/trap.rs` (because `test_helpers` is used
  from `mm`, `csr`, and `inst`), and the two in `arch/riscv/csr/ops.rs`
  (because `csr_read` / `csr_write` are called from `inst`). The eighth
  (`csr.rs:95 find_desc`) must also be arch-level, reached from
  `arch/riscv/cpu/debug.rs`.
- Why it matters:
  Phase 2c is the green-bar-restore phase. Under the current wording, the
  branch from PR 2c will not compile; the executor will have to improvise
  the visibility fix on-PR, which either (a) forces an unplanned re-spin of
  the `pub(in …)` mapping, invalidating the Response Matrix's exact line
  list, or (b) introduces an ad-hoc visibility edit that the validation
  gate (`rg 'pub\(in crate::(cpu|isa)::(riscv|loongarch)' … = 0 hits`)
  passes but that fails `cargo test`. This is exactly the class of failure
  round-01 R-011 warned about, moved from "hoist syntax" to "visibility
  scope."
- Recommendation:
  In the Response Matrix and Phase 2c, distinguish "truly topic-local"
  from "cross-topic helpers." Concretely:
  - `arch/riscv/trap.rs:21,26,31,35,55 → pub(in crate::arch::riscv)` (not
    `…::trap`), because `test_helpers` and the `trap_*` methods are used
    from `mm`, `csr`, and `inst`.
  - `arch/riscv/csr.rs:95 → pub(in crate::arch::riscv)` (not `…::csr`),
    because `find_desc` is used from `cpu/debug.rs`.
  - `arch/riscv/csr/ops.rs:7,16 → pub(in crate::arch::riscv)` (not
    `…::csr`), because `csr_read`/`csr_write` are called from `inst`.
  - `arch/riscv/mm/tlb.rs:10,58 → pub(in crate::arch::riscv::mm)` — fine.
  - `arch/riscv/mm/mmu.rs:20 → pub(in crate::arch::riscv::mm)` — fine.

  Net: 8 sites become `pub(in crate::arch::riscv)`, 3 stay topic-local.
  The grep gate `rg 'pub\(in crate::(cpu|isa)::(riscv|loongarch)' = 0` is
  still satisfied. Mention this explicitly in Phase 2c so that the executor
  does not mechanically apply the tighter scope from the Response Matrix.



### R-017 `cpu/mod.rs one-line #[cfg] seam is insufficient: CoreContext and PendingTrap are arch types that lib.rs and error.rs still name`

- Severity: CRITICAL
- Section: Architecture / Seam shape, API Surface, Constraints / C-5,
  Invariants / I-4, Invariants / I-6
- Type: Correctness / Spec Alignment
- Problem:
  G-2, C-5, and I-6 all commit to **exactly one** `#[cfg]`-aware line per
  upper-layer seam file: `cpu/mod.rs` has one (`pub type Core = …`),
  `isa/mod.rs` has one (`pub use crate::arch::riscv::isa::IMG;`),
  `device/intc/mod.rs` has one (`pub type Intc = …`). But the public API
  surface the plan promises to preserve (I-4) reaches deeper than `Core`
  and `IMG`:

  1. `xemu/xcore/src/lib.rs:35-39` re-exports
     `pub use cpu::{BootConfig, CoreContext, RESET_VECTOR, State, XCPU,
     debug::{Breakpoint, DebugOps}, with_xcpu};`. `CoreContext` is **not**
     a neutral type — it resolves today to
     `cpu::riscv::context::RVCoreContext` via
     `cpu/riscv/mod.rs:15`'s `pub use self::{…, context::RVCoreContext as
     CoreContext, trap::PendingTrap};` and is consumed by field access in
     `xdb/src/difftest/{mod.rs,qemu.rs,spike.rs}` (e.g. `dut.pc`,
     `dut.gprs`, `dut.privilege`, `dut.csrs`). It **must** continue to
     resolve to the same concrete struct post-refactor for `xdb` to
     compile unchanged (I-4, V-E-3).
  2. `xemu/xcore/src/error.rs:5` imports `use crate::cpu::PendingTrap;`
     and `XError::Trap(PendingTrap)` uses it by value. `PendingTrap` is
     fundamentally arch-specific (it contains `TrapCause` → `Exception` |
     `Interrupt`, all RISC-V enums). Today this import works because
     `cpu/riscv/mod.rs:15` re-exports it at `cpu::PendingTrap`. Under the
     plan's seam rewrite (replace `cfg_if!` + `pub use self::riscv::*` by
     a single-line `pub type Core = …`), `cpu::PendingTrap` no longer
     resolves; `error.rs` fails to compile.

  The plan does not address (1) or (2). The `Data Structure` section
  (`02_PLAN.md:409-425`) shows only the `Core`, `IMG`, and `Intc` aliases.
  No alias for `CoreContext`, `PendingTrap`, or any of the other items
  `cpu/riscv/mod.rs:15` presently re-exports. I-4's "public API unchanged"
  promise and C-5's "exactly one `#[cfg]`-aware line" constraint are in
  direct conflict under M-004's "trait dispatch for arch behaviour"
  framing: neither trait dispatch nor a single `pub type Core` covers
  `CoreContext` (a data carrier consumed by field name) or `PendingTrap`
  (a value type embedded in a neutral error enum).
- Why it matters:
  This is a hard build break in Phase 2c, the phase that is supposed to
  restore the green bar and in which xdb must compile unchanged (V-E-3).
  It also casts doubt on C-5 / I-6 as written — if additional seam lines
  are required to paper over (1) and (2), the "exactly one" claim is
  false and the I-6 structural check needs relaxation. Worst case:
  executor improvises a wildcard re-export (`#[cfg(riscv)] pub use
  crate::arch::riscv::cpu::{CoreContext, PendingTrap, …};`), which brings
  back the round-01 "hoist" shape that R-011 tried to get rid of.
- Recommendation:
  Pick one of the following and record it explicitly in the next PLAN:
  1. **Relax C-5 / I-6 honestly.** State that `cpu/mod.rs` contains
     `#[cfg(riscv)] pub type Core = crate::arch::riscv::cpu::RVCore;` plus
     a small, enumerated set of additional type aliases for the data
     types `lib.rs` / `error.rs` currently name —
     `CoreContext = crate::arch::riscv::cpu::context::RVCoreContext`,
     `PendingTrap = crate::arch::riscv::trap::PendingTrap`, and any other
     types `lib.rs:34-41` re-exports that resolve inside `arch/` today.
     Update I-6 to bound the seam at "≤ N `#[cfg]`-aware lines per upper
     module" with N enumerated per module. This is the most honest landing
     of M-001 (drop `selected`) plus I-4 (unchanged public API).
  2. **Make `CoreContext` / `PendingTrap` neutral.** Move the struct
     definitions into `cpu/` (`CoreContext` is just
     `{pc: u64, gprs: Vec<(&'static str, u64)>, privilege: u64, csrs:
     Vec<(u16, &'static str, u64, u64)>, word_size, isa}`, all neutral).
     `PendingTrap` is harder — it wraps `TrapCause` (arch enum) — but
     could be abstracted as a trait object or opaque handle if really
     needed. This is a larger change and likely out of scope for the
     archModule iteration; flag as follow-up if rejected.
  3. **Hide via trait-associated types.** Add
     `type Context: Clone + Send` and `type Trap: …` on `CoreOps`; have
     `CPU<C: CoreOps>` expose `fn context(&self) -> C::Context`. This
     pushes the arch type behind a generic, so `lib.rs` re-exports
     `<Core as CoreOps>::Context` rather than a concrete `CoreContext`.
     This is the cleanest M-004-shaped answer but requires a difftest
     re-plumb because `xdb` reads `CoreContext` by field name; that
     plumbing is a second, separate iteration.

  Option 1 is the lightest compliant answer and still honors M-004's
  intent (upper layer owns *trait* surface; a type alias to a concrete
  arch data carrier is not dispatch). Whichever option is chosen, update
  G-2, C-5, I-6, and the `Data Structure` seam-alias block to name every
  re-exported arch type (`CoreContext`, `PendingTrap`, …), and either
  raise the I-6 budget or defend why the current budget still holds.



### R-018 `TrapIntake and IntcSet are ceremonial — the upper layer still owns arch-specific dispatch through Bus::aclint_idx / Bus::plic_idx / Bus::mtime / Bus::take_ssip`

- Severity: HIGH
- Section: Data Structure / trait surface, Invariants / NG-5, M-004
  compliance
- Type: Maintainability / Spec Alignment
- Problem:
  M-004 (round 01 MASTER, CRITICAL) requires that `cpu/`, `device/`, and
  `isa/` contain only trait APIs plus a tiny `#[cfg]` patch, with all
  arch-specific behaviour dispatched through traits and implemented
  inside `arch/`. The plan adds three new traits (`TrapIntake`,
  `InterruptSink`, `IntcSet`) but leaves the actual arch-coupling surface
  in `device/bus.rs` untouched (NG-5):
  `Bus::aclint_idx`, `Bus::plic_idx`, `Bus::mtime()`,
  `Bus::set_timer_source()`, `Bus::set_irq_sink()`, `Bus::ssip_flag()`,
  `Bus::take_ssip()`.

  Walking the new traits against what they actually abstract:

  - **`TrapIntake::sync(&mut self)`** is a single-method trait on `RVCore`
    itself. `RVCore::sync_interrupts` already exists as an inherent
    method (`cpu/riscv/mod.rs:132`). Wrapping it in a trait that only
    the owning type implements, called from the owning type's own
    `step()`, abstracts nothing — the trait sits between `RVCore` and
    itself. No upper-layer code invokes `TrapIntake::sync`; the plan's
    prose confirms this ("`RVCore::sync_interrupts` […] consumes
    `TrapIntake`" — but in fact `sync_interrupts` *is* the sync method).
    M-004 asks for dispatch across the `cpu/` seam, not a trait that
    only exists in `arch/`.
  - **`InterruptSink: Send + Sync + Clone { type Bits; set/clear/load/reset }`**
    duplicates exactly the inherent API of `IrqState`
    (`device/mod.rs:84-98`). The blanket `impl InterruptSink for IrqState`
    in `arch/riscv/trap/interrupt.rs` (with `type Bits = u64`) calls
    `IrqState::set` / `IrqState::clear` / `IrqState::load` /
    `IrqState::reset` one-to-one. No caller in the upper layer uses the
    trait — the arch-side `Aclint` / `Plic` call `IrqState::set(MSIP)`
    directly (aclint.rs:68, plic.rs via MEIP/SEIP). The trait is not
    observed by the seam.
  - **`IntcSet { tick_fast(); tick_slow() }`** does not actually supplant
    `Bus::aclint_idx` / `Bus::plic_idx`-driven tick in
    `bus.rs:133-160`, which remains untouched (NG-5). The plan introduces
    an `Intc` bundle struct under `arch/riscv/device/intc/` that
    *conceptually* bundles ACLINT + PLIC, but that bundle is not used by
    `Bus::tick` — `Bus::tick` still reaches into `self.mmio[idx].dev`
    directly. So `IntcSet` is defined but never called.

  Meanwhile, the genuinely arch-leaky surface remains in the upper layer:
  `Bus::mtime()` returns a RISC-V machine-timer value consumed by
  `arch/riscv/cpu/mod.rs` (`self.csr.set(CsrAddr::time, self.bus.mtime()
  as Word)`); `Bus::take_ssip()` exposes a RISC-V SSWI edge to the arch
  step loop; `Bus::set_timer_source()` / `Bus::set_irq_sink()` are
  RISC-V intc-role methods. These are not abstracted through any of the
  three new traits.

  NG-5 quarantines these as follow-up, which is legitimate *scoping* —
  but the plan simultaneously claims M-004 is "Applied" in the Response
  Matrix row for M-004 (01 CRITICAL). Partial compliance with M-004 is
  not marked as partial.
- Why it matters:
  M-004 is a MASTER MUST (CRITICAL). The new traits absorb review effort
  and review approval but do not, in their current shape, reduce the
  arch-coupling that M-004 targets. The Bus-level residuals are where
  the real leakage sits; deferring them to `aclintSplit` / `plicGateway`
  / `directIrq` is reasonable, but then the Response Matrix for M-004
  should read "Partial — trait surface introduced at CPU / interrupt
  boundary; Bus-level residuals (NG-5) explicitly deferred to named
  follow-ups" rather than "Applied."
- Recommendation:
  Do **one** of the following in the next PLAN:
  1. **Make the traits earn their keep.** `TrapIntake` should be the
     contract `CPU<Core>::step` calls — e.g. move `sync_interrupts`
     invocation from inside `RVCore::step` to `CPU::step` via
     `CoreOps::sync_interrupts` (or a `TrapIntake`-shaped separate
     trait). That way the trait sits *across* the seam and the upper
     layer truly dispatches into arch. If that is too disruptive, drop
     `TrapIntake` as an arch-local trait does not satisfy M-004.
  2. **Demote `InterruptSink` to "vocabulary trait" and make it observe.**
     Have the upper layer call `InterruptSink::load` via `dyn
     InterruptSink` somewhere that meaningfully crosses the seam (today
     nothing does). Otherwise, `IrqState` is already arch-neutral storage
     and the trait is a naming exercise — admit that.
  3. **Mark M-004 as "Partial" in the Response Matrix.** Explicitly list
     which parts of M-004 are in scope (the `CoreOps` / `DebugOps` seam
     is trait-dispatched; arch types + impls live in `arch/`) and which
     are out of scope (Bus↔intc contract redesign queued under NG-5).
     That is honest and defensible. Status quo — claiming "Applied" when
     `Bus::aclint_idx` / `Bus::mtime` still leak — under-reports the
     residual M-004 work.

  Option 3 is the cheapest landing and aligns the claim with NG-5's
  reality. Options 1 and 2 are the full solution but may belong in the
  NG-5 follow-up plans.



### R-019 `V-UT-1 allow-list is text-level, cannot distinguish token from comment / string / macro`

- Severity: MEDIUM
- Section: Validation / V-UT-1
- Type: Validation
- Problem:
  V-UT-1 (`02_PLAN.md:725-751`) walks `.rs` files as **text** and asserts
  per-file-per-literal allow-list entries from `vfz-allow.txt`. Using
  `regex` or `aho-corasick` on raw file bytes, it cannot distinguish:
  - a real identifier `MSIP` (token),
  - a comment `// MSIP is the …`,
  - a doc-comment mentioning SSIP / MTIP for rustdoc,
  - a string literal like `info!("bus: msip={}", …)`,
  - a macro-expanded identifier that only appears after `cargo expand`.

  Today `device/bus.rs` contains `info!("bus: add_mmio '{}' …")` debug
  strings that mention "plic" and "aclint" in `bus.rs:95`, and similar
  in `cpu/riscv/mod.rs`. The vocabulary list contains `aclint`, `plic`.
  These will flag spuriously unless the allow-list explicitly pins each
  string literal occurrence. That is tolerable but brittle — any future
  `info!("plic: …")` in an otherwise neutral module will fail V-UT-1
  without being a real leak.
- Why it matters:
  V-UT-1 is the I-1 / I-2 enforcement mechanism. If it flags false
  positives, contributors will add entries to `vfz-allow.txt` to silence
  it, which erodes the guarantee. If it misses real leaks (e.g. a
  macro-generated `Mip::SSIP` reference), contributors will believe the
  guarantee holds when it does not.
- Recommendation:
  Either (a) accept the limitation and name it explicitly in the plan
  ("V-UT-1 is a best-effort text-level gate; it may require allow-list
  entries for debug strings; it does not inspect macro expansions"), or
  (b) use `syn` to parse each file and key the vocabulary check on
  `Ident` tokens only. Option (a) is consistent with C-4 (no prod-dep
  change) and NG-7; option (b) adds `syn` as a dev-dependency. Pick one
  and document. Also: add an explicit initial allow-list entry for the
  `info!("bus: add_mmio …")` debug strings that mention device names so
  the first CI run of V-UT-1 passes.



### R-020 `Phase 2a is explicitly non-green; merge-gating rule is prose-only`

- Severity: MEDIUM
- Section: Implementation Plan / Phase 2a
- Type: Validation / Flow
- Problem:
  Phase 2a (`02_PLAN.md:667-674`) openly states the build does not
  compile after the two `git mv` calls because `cpu/mod.rs`'s `cfg_if!
  mod riscv;` stops resolving. The plan recommends "land PR 2a as 2a
  commit only on a branch; merge to trunk only once PR 2c lands."
  This is a sensible developer-process rule but it is not enforced
  anywhere — a reviewer approving PR 2a alone could merge it and break
  trunk. The rule lives only in prose in Phase 2a.
- Why it matters:
  I-5 ("behaviour unchanged at each phase") and V-IT-1 ("`cargo test -p
  xcore` passes at each phase end") are claimed for "2c, 3, 4, 5" but
  the narrative elsewhere implies per-phase green bar. A reader skimming
  I-5 without reading the Phase 2a caveat will believe Phase 2a is
  mergeable standalone. Git bisect through a broken 2a commit silently
  regresses `git bisect run`.
- Recommendation:
  Either (a) collapse 2a+2b+2c into a single PR (PR 2 with three commits
  inside), advertised as a single reviewable PR with 2a/2b/2c as
  bisectable history; or (b) state explicitly in the Implementation
  Plan that PR 2a and PR 2b are *stacked* behind PR 2c and none merges
  to trunk until 2c lands. In either case, update V-IT-1 to name which
  phase boundaries carry the green-bar claim (the current "each listed
  phase boundary" is ambiguous for 2a/2b).



### R-021 `Phase 4 rewires sync_interrupts; C-2 forbids semantic edits`

- Severity: MEDIUM
- Section: Constraints / C-2, Implementation Plan / Phase 4
- Type: Correctness / Spec Alignment
- Problem:
  C-2 says "No semantic edits inside moved files beyond `use`-path and
  `pub(in …)` token rewrites required to compile. Byte-identical
  behaviour." Phase 4 (step 6, `02_PLAN.md:592-594`) says:
  > Rewire `sync_interrupts` to call through `TrapIntake` locally rather
  > than import raw mip bits from `crate::device::`. The bit constants
  > stay arch-local; the upper layer never names them.

  Rewiring `sync_interrupts` from calling `self.csr.set(CsrAddr::mip,
  … | (hw & HW_IP_MASK) | stip)` with `HW_IP_MASK` imported from
  `crate::device::` to calling a `TrapIntake::sync`-shaped method with
  `HW_IP_MASK` imported from `super::trap::interrupt` is an *import*
  edit at the call site. But the Main Flow phrasing "call through
  `TrapIntake` locally" reads as a *restructuring* edit — moving the
  method out of `RVCore` into a trait impl. That is not an import-path
  edit; it is a semantic move.
- Why it matters:
  If Phase 4 changes the method structure (not just imports),
  V-F-4 (difftest vs QEMU / Spike zero divergence) becomes the only
  correctness guard. Difftest is strong, but C-2 exists precisely so
  that reviewers can skim phase-diffs with confidence that no semantics
  moved. A silent semantic edit inside a relocation phase undermines
  that.
- Recommendation:
  Pick one of:
  1. **Keep `sync_interrupts` inherent, only rewrite imports.** Drop
     `TrapIntake` (see R-018 option 3), replace `use crate::device::
     HW_IP_MASK` with `use super::trap::interrupt::HW_IP_MASK` inside
     `arch/riscv/cpu/mod.rs`. This is byte-identical behaviour modulo
     imports, satisfies C-2, and keeps M-004's residual at "Partial" as
     R-018 recommends.
  2. **Name the C-2 exception explicitly.** Add to C-2: "Exception:
     `sync_interrupts`'s call path from inherent method to
     `TrapIntake::sync` is a semantic edit justified by M-004
     (round-01 CRITICAL). Coverage: V-F-4 difftest." This makes the
     edit legible to reviewers.

  Either is acceptable; (1) is cleaner and composes with R-018.



### R-022 `Dev-dep addition of aho-corasick is not NG-7-compatible without explicit waiver`

- Severity: LOW
- Section: Validation / V-UT-1, Constraints / C-4, NG-7
- Type: Spec Alignment
- Problem:
  V-UT-1 (`02_PLAN.md:746-751`) proposes `aho-corasick` as a dev-dep and
  argues "already transitively present through `pest` in xcore."
  Cross-checked: pest does pull in `aho-corasick`, but as a *transitive*
  dep of a prod-dep. Adding `aho-corasick` as a direct `[dev-dependencies]`
  entry is still a `Cargo.toml` edit, which conflicts with C-4 ("no
  Cargo / MSRV / edition / dep changes") and NG-7 ("No MSRV / edition /
  Cargo dependency changes") — both are absolute, with no "transitive
  presence" escape clause in their current wording.
- Why it matters:
  NG-7 exists to keep the refactor strictly structural. Even a
  dev-dependency edit must be gated. The plan's "noted and acceptable"
  language in V-F-5 is the author waiving their own constraint.
- Recommendation:
  Either (a) state V-UT-1 in dep-free form (`str::find` over byte
  slices, one file at a time — acceptable for ~100 files), or (b)
  update NG-7 / C-4 to read "no prod-dep change; one dev-dep addition
  (`aho-corasick`) permitted for V-UT-1, bound by V-F-5." Pick one.
  Option (a) is trivially achievable and avoids the waiver.



---

## Trade-off Advice

### TR-2 `Back-compat re-exports vs hard cut` (carried forward, keep)

- Related Plan Item: `T-2`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A (keep back-compat re-exports)
- Advice:
  Keep the back-compat `cpu::Core` / `isa::IMG` / `device::intc::Intc`
  aliases. Downstream (`xdb`, `xam`, `xemu` binary) stays untouched per
  V-E-3.
- Rationale:
  Unchanged from rounds 00 / 01. I-4 guarantees this; V-E-3 verifies it.
- Required Action:
  Keep as is. The executor needs only to also expose `CoreContext` /
  `PendingTrap` under back-compat names to honor I-4 (see R-017).



### TR-3 `IrqState location` (carried forward, keep)

- Related Plan Item: `T-3`
- Topic: Premature Generalisation vs Now-ness
- Reviewer Position: Prefer Option A (IrqState stays in `device/mod.rs`)
- Advice:
  Storage stays in `device/mod.rs`; the `impl InterruptSink for IrqState`
  lives in `arch/riscv/trap/interrupt.rs` beside the bit constants.
- Rationale:
  Unchanged from rounds 00 / 01.
- Required Action:
  Keep as is.



### TR-4 `Phased PRs` (carried forward, tighten merge-gating)

- Related Plan Item: `T-4`
- Topic: Reviewability vs Merge Overhead
- Reviewer Position: Prefer Option B (phased), with merge-gating rule
- Advice:
  Seven PRs (1, 2a, 2b, 2c, 3, 4, 5) is tractable. Per R-020, either
  collapse 2a+2b+2c into one PR with three bisectable commits or
  explicitly stack them so trunk never sees a non-green Phase 2a.
- Rationale:
  Bisectability through 2a is the whole reason to split 2a from 2c; but
  merging 2a alone to trunk is a git-bisect regression hazard. The rule
  "land 2a only behind 2c" has to be explicit, not implicit.
- Required Action:
  Update Phase 2a prose to name the merge-gating rule, or collapse
  2a/2b/2c into a single PR.



### TR-6 `Introduce TrapIntake / IntcSet now vs defer to aclintSplit / directIrq`

- Related Plan Item: Data Structure (new traits)
- Topic: M-004 Compliance Framing vs Premature Trait Surface
- Reviewer Position: Need More Justification
- Advice:
  Three new traits (`TrapIntake`, `InterruptSink`, `IntcSet`) exist in
  the plan but none of them mediates a real upper-layer → arch dispatch
  today (see R-018). If they are added as M-004 scaffolding for the
  follow-up plans (`aclintSplit`, `plicGateway`, `directIrq`), say so
  explicitly and defer their wiring into the seam until those plans
  deliver. If they are meant to provide seam dispatch *now*, the call
  sites in `cpu/` / `device/` must actually invoke them (currently
  nothing does).
- Rationale:
  Adding three traits now with no upper-layer caller creates dead API
  surface that future plans must either adopt or delete. Rewriting the
  trait signature is cheaper before than after.
- Required Action:
  Either (a) name the follow-up plan that *will* call each trait and
  defer wiring; or (b) wire at least one trait (preferably `TrapIntake`
  via `CoreOps`) through the seam in Phase 4 so M-004's "Applied" claim
  is substantive.



---

## Positive Notes

- **Flat layout is now unambiguous.** Every paragraph of the plan
  (Summary, Architecture, Data Structure, API Surface, Execution Flow,
  Phase tables, Response Matrix) resolves to the same
  `arch/riscv/{cpu, csr, mm, trap, inst, isa, device}/…` layout. R-010,
  R-011 (layout half), R-012, TR-5 are cleanly closed.
- **`selected` removal is thorough.** No `selected` identifier remains
  in the plan text, and the direct-path `#[cfg(riscv)] pub type = …`
  alias is a strictly cleaner mechanism than the round-01 `cfg_if!` +
  `use self::selected` tree. M-001 (round 01) faithfully applied.
- **`compile_error!` canaries removed.** `build.rs` is the single gate
  for `X_ARCH` validity, matching `xemu/xcore/build.rs:19-33`. M-003
  (round 01) faithfully applied; V-E-1 removed.
- **Response Matrix is complete and explicit.** Every CRITICAL / HIGH
  finding from rounds 00/01 and every MASTER directive from rounds
  00/01 appears with a specific resolution, not a stub. Rejections
  are absent ("No rejections in this round") — honest given the
  content.
- **Phase 2 split (2a/2b/2c).** The three-commit split gives clean
  bisection surfaces, subject to R-020's merge-gating caveat.
- **V-UT-1 is the right *shape*** even if its allow-list granularity
  needs tightening (R-019). Vocabulary-level grep catches the class of
  leak a path-only grep misses.
- **Per-phase named validation targets.** `make cpu-tests-rs`,
  `make am-tests`, `make linux`, `make debian`, `cargo test -p xcore`
  — S2 is no longer "green bar" hand-waving. R-015 resolved.
- **Honest NG-5 scoping.** The plan clearly states which Bus-level
  residuals are deferred and to which named follow-up plans. Combined
  with the V-UT-1 allow-list, that is a defensible partial landing of
  MANUAL_REVIEW #3.



---

## Approval Conditions

### Must Fix
- R-016 (topic-narrow `pub(in crate::arch::riscv::<topic>)` breaks
  compilation for 8 of 11 rewrite sites; re-scope to
  `pub(in crate::arch::riscv)` for `trap.rs` and `csr/ops.rs` /
  `csr.rs` sites)
- R-017 (cpu/mod.rs seam budget insufficient — `CoreContext` and
  `PendingTrap` must be aliased for I-4 / `lib.rs` / `error.rs` to
  keep compiling; either relax C-5 / I-6 honestly or abstract via
  trait-associated types)
- R-018 (mark M-004 as "Partial" in the Response Matrix and either
  wire `TrapIntake` across the seam so it is not arch-local-only,
  or demote it and `InterruptSink` / `IntcSet` to named follow-up
  plan scaffolding)

### Should Improve
- R-019 (V-UT-1 text-level allow-list limitation — document
  explicitly or switch to `syn`-based token parsing)
- R-020 (Phase 2a merge-gating — make the "land only behind 2c" rule
  explicit, or collapse 2a/2b/2c into one PR)
- R-021 (C-2 exception for `sync_interrupts` rewiring — name the
  exception or drop the rewiring)
- R-022 (NG-7 waiver for `aho-corasick` dev-dep, or switch V-UT-1
  to `str::find` / no-dep form)

### Trade-off Responses Required
- TR-6 (justify the three new traits: either name their caller /
  wire one across the seam in Phase 4, or tag them explicitly as
  scaffolding for `aclintSplit` / `plicGateway` / `directIrq`)

### Ready for Implementation
- No
- Reason: R-016 and R-017 are CRITICAL and both cause literal `cargo
  build -p xcore` failures at the Phase 2c boundary as the plan is
  currently written. R-016 is a narrow scoping fix (8 of 11 sites
  move from `pub(in …::topic)` to `pub(in …::riscv)`); R-017 is a
  plan-level decision about the seam budget plus one or two extra
  aliases in `cpu/mod.rs`. R-018 is CRITICAL because it mis-claims
  M-004 status in the Response Matrix; a rewrite of that row to
  "Partial" plus one follow-up-plan sentence resolves it. None of
  the three blocks require architectural reconsideration. Round 03
  should be a tightening pass, not a redesign.
