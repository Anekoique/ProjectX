# `multiHart` REVIEW `00`

> Status: Open
> Feature: `multiHart`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
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
- Non-Blocking Issues: `7`



## Summary

Round-00 lays out a coherent two-PR path for introducing `HartId` /
`Hart` into `arch/riscv/cpu/` with every per-hart field migrated off
`RVCore` into `Hart`, ACLINT sub-devices extended to per-hart state
arrays at spec-mandated strides, and round-robin single-threaded
execution at `num_harts ≥ 1`. The design honours every inherited
MASTER directive (00-M-001/002, 01-M-001..004): `Hart` and `HartId`
live arch-internal, `CoreOps` / `DebugOps` signatures are unchanged,
the top-level `cpu/` seam stays a trait-plus-cfg thin layer, and the
`#[cfg(riscv)]` gate at `cpu/mod.rs` is not touched. `I-4`
(byte-identical behaviour at `num_harts == 1`) is the correct
behaviour-preservation anchor and it is traceable end-to-end via the
PR1 gate matrix. Trade-off framing (T-1..T-5) is substantive: each
option tuple lists a concrete rejection rationale and the chosen path
matches the conservatism the current goals demand.

Three blocking issues must be addressed before PR2 execution (PR1
itself is mostly green-bar, but two of the three blockers already
affect PR1's API-surface precision). **R-001 (HIGH)**: `Plic::new`
today is `pub fn new(irq: IrqState)` with a compile-time
`const NUM_CTX: usize = 2` and `const CTX_IP: [u64; NUM_CTX]`; making
`NUM_CTX = 2 * num_harts` at runtime is **not** a one-line constant
change — it requires reshaping `enable` / `threshold` / `claimed` to
runtime lengths, promoting `CTX_IP` from a compile-time array to a
runtime-generated pattern (`ctx_id → hart_id, mode`), **and**
replacing the scalar `irq: IrqState` with `Vec<IrqState>` so per-ctx
`evaluate()` can set the correct hart's MEIP / SEIP (I-5 requirement).
None of this is in the plan's API Surface. **R-002 (HIGH)**:
cross-hart LR/SC reservation invalidation is a spec invariant
(RISC-V Unpriv §14.2: a store from any hart invalidates reservations
on every other hart covering the same address) that the plan's
per-hart `reservation: Option<usize>` silently omits; at `num_harts
== 1` this is dead, but PR2 activates `num_harts = 2` and the plan
ships a gate that must pass without this. **R-003 (HIGH)**: the plan
asserts PR1 adds **zero net tests** ("same count as post aclintSplit",
`00_PLAN.md:534`) while simultaneously listing V-UT-1..V-UT-7
(seven new PR1 unit tests) plus V-IT-3 (one new PR1 integration test)
under Validation — the arithmetic must reconcile.

Seven non-blocking items cover: scope creep in the PLIC "mechanical
extension" framing (R-004 — really a mini-redesign, should be
acknowledged), `RVCore::with_bus` seam churn for external callers
(R-005), missing `Plic::new` / `Bus::new` call-site audit in PR2 file
list (R-006), acceptance mapping gaps for I-1/I-2/I-3/C-7 (R-007),
OpenSBI HSM assumption unverified (R-008), a minor contradiction
between `ebreak_as_trap` being machine-scoped vs. actually being a
policy that applies to each hart's ebreak (R-009), and the hard-coded
`mhartid = 0` deferral leaving PR1 at `num_harts == 1` with correct
observable behaviour but a subtle coupling that PR2's hart-id
seeding must unwind carefully (R-010).

Trade-off advice: TR-1 concurs with (a) round-robin, TR-2 concurs
with (a) `Vec<Hart>`, TR-3 **prefers (b) 3-PR split** — the plan's
own PR2 bundles PLIC runtime-size conversion, DTS/OpenSBI
integration, and SMP Linux as one unit, which is too coarse given
R-001's discovery. TR-4 concurs with (a) scalar DebugOps. TR-5
concurs with (a) per-hart `take_ssip`.

---

## Findings

### R-001 `PLIC runtime-size conversion is not a mechanical extension`

- Severity: HIGH
- Section: Implementation Plan — Phase 2 / PR2
- Type: API / Correctness
- Problem:
  `00_PLAN.md:446-450` describes PR2 step 14 as raising `NUM_CTX`
  from 2 to `2 * num_harts` and wiring `CTX_IP[ctx] →
  harts[hart_id].irq`. Ground truth at
  `xemu/xcore/src/arch/riscv/device/intc/plic.rs:11-22` shows five
  compile-time-sized items: `const NUM_CTX: usize = 2;`,
  `const CTX_IP: [u64; NUM_CTX] = [MEIP, SEIP];`, and the fields
  `enable: Vec<u32>`, `threshold: Vec<u8>`, `claimed: Vec<u32>`
  (each initialised `vec![0; NUM_CTX]`). Plus `Plic::new(irq:
  IrqState)` at line 36 takes a **scalar** IrqState, used by
  `evaluate()` at line 114 to set MEIP/SEIP on that one IrqState for
  every ctx. To honour the plan's own Invariant I-5 ("PLIC M/S
  contexts wire to hart `floor(ctx/2)`") the PR2 delta must at
  minimum:
  (a) Drop `const NUM_CTX: usize = 2` and `const CTX_IP: [u64; 2]`
      in favour of a runtime-sized scheme (e.g. compute `ip =
      (ctx & 1 == 0) ? MEIP : SEIP` inline, pick
      `hart_id = ctx >> 1`).
  (b) Re-size `enable` / `threshold` / `claimed` to `2 * num_harts`.
  (c) Replace the scalar `irq: IrqState` field with
      `irq_per_hart: Vec<IrqState>` (or `Vec<Arc<IrqState>>` —
      whichever matches `Hart::irq` semantics at clone time), and
      change `Plic::new` to take `Vec<IrqState>`.
  (d) Rewrite `evaluate()` so the MEIP / SEIP set/clear targets
      `self.irq_per_hart[ctx >> 1]` using `ip` from (a).
  (e) Update the `Plic::new(irq.clone())` call site at
      `cpu/mod.rs:68` to pass a `Vec<IrqState>` sized to `num_harts`.
  None of (a)-(e) appears in the plan's API Surface, PR2 file list
  (`00_PLAN.md:549-555`), or Implementation Plan step 14.
- Why it matters:
  Calling the PLIC change "mechanical" undersells the work and
  conflates a compile-time constant with a device-API shape change.
  Without (c) and (d), MEIP on hart 1 cannot assert hart 1's
  `IrqState` and I-5 is violated the moment the first timer
  interrupt targets the non-boot hart. Without (e), PR2 does not
  compile. This is not a stylistic nit — it is the difference
  between a 10-line diff and a ~80-line PLIC rewrite.
- Recommendation:
  Add to PR2's API Surface:
  ```rust
  impl Plic {
      pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self;
  }
  ```
  Replace the `const NUM_CTX: usize = 2` line with a runtime field
  `num_ctx: usize = 2 * num_harts` on `Plic`. Delete `CTX_IP`;
  compute `ip` inline from `ctx`. List the five edits (a)-(e) in
  the Phase-2 file table and either (i) split PR2 into PR2a (PLIC
  runtime-size + `Plic::new(Vec<IrqState>)` at num_harts=1 — zero
  behavioural change because the Vec has length 1) and PR2b (DTS +
  CLI + SMP boot), per TR-3 (b), or (ii) keep PR2 monolithic but
  acknowledge the PLIC edit is a mini-redesign, not a mechanical
  extension. Also add a PR2 unit test mirroring `meip_seip_set_and_
  clear` at `num_harts = 2` proving MEIP on ctx 2 lands on
  `irqs[1]` and not `irqs[0]`. (V-IT-2 covers this at integration
  level; a unit-level twin strengthens the gate.)



### R-002 `Cross-hart LR/SC reservation invalidation is unaddressed`

- Severity: HIGH
- Section: Architecture / Invariants
- Type: Spec Alignment / Correctness
- Problem:
  RISC-V Unprivileged ISA §14.2 (Zalrsc) mandates that a store from
  **any** hart to an address covered by another hart's load-reserved
  reservation must invalidate that reservation. Today (`num_harts ==
  1`) this invariant is trivially satisfied because a hart cannot
  race against itself — every store between LR and SC goes through
  the same fetch/execute path and clears `self.reservation` if it
  overlaps. At `num_harts = 2` the invariant becomes load-bearing:
  hart 0's `sw` to `0x80001000` must invalidate hart 1's
  `reservation = Some(0x80001000)`. The plan's Data Structure at
  `00_PLAN.md:282` puts `reservation: Option<usize>` **per-hart** —
  correct placement — but the Main Flow at `00_PLAN.md:224-235` and
  the `Hart::step_one` signature at line 318 say nothing about
  cross-hart invalidation. Spike's reference implementation walks
  every other hart's reservation on every store; QEMU uses a global
  exclusive monitor. Neither path is acknowledged.
- Why it matters:
  SMP Linux's `atomic_t`, `spinlock_t`, and `qspinlock` all depend on
  LR/SC correctness across harts. A missing invalidation is a silent
  data-race bug that will not crash boot — it will corrupt kernel
  locks at unpredictable intervals. V-IT-5 (`smp_linux_smoke`)
  reaching `buildroot login:` does not exercise this path with
  sufficient density to catch it; it may boot green while being
  functionally unsound. Worse, since difftest is pinned at
  `num_harts = 1` (NG-3), the normal divergence canary is disabled
  at exactly the configuration where the bug manifests.
- Recommendation:
  Add an Invariant (call it I-8):
  > **I-8** A guest store from hart `h_src` to physical address
  > `addr` invalidates `harts[h].reservation` for every `h ≠ h_src`
  > whose reservation overlaps the store's reservation granule.
  > Implementation: `Hart::store` (or `Bus::write` called from a
  > hart) posts the address through a hook that walks `harts[0..N]`
  > and clears matching reservations; single-hart collapses to the
  > existing self-clear.
  Pick one of two mechanisms and name it in API Surface:
  (a) `RVCore` owns the scheduler and can walk `harts` directly —
      add `RVCore::invalidate_reservations_except(src: HartId,
      addr: usize)` called from the post-store path, passing through
      `Hart::step_one` via a callback or by routing stores through
      `RVCore` instead of `Hart`.
  (b) Bus-side broadcast — `Bus::invalidate_reservations(addr:
      usize)` called by every `Bus::write`, with a registered
      per-hart reservation table (`Vec<Arc<Mutex<Option<usize>>>>`).
      Heavier, but decouples hart borrowing.
  Option (a) is cheaper at `num_harts = 1` (compile-time skip) and
  matches NG-2 (no `parking_lot` / no multi-thread). Add a PR2 unit
  test: two harts; hart 0 LRs `0x80001000`; hart 1 stores 1 byte at
  `0x80001003`; hart 0's SC to `0x80001000` fails.



### R-003 `PR1 "same test count" claim contradicts listed new tests`

- Severity: HIGH
- Section: Validation / Phase 1 gate matrix
- Type: Validation / Correctness
- Problem:
  `00_PLAN.md:534-535` asserts PR1's gate matrix expects
  "354 lib + 1 `arch_isolation` + 6 `xdb` = all pass; **same count**
  as post aclintSplit (no net add / remove in PR1)". Validation
  section at `00_PLAN.md:616-634` enumerates new PR1-scope unit
  tests: V-UT-1 (`Hart::new`), V-UT-2 (`Hart::reset`), V-UT-3
  (`Mswi` 4-hart fan-out), V-UT-4 (`Mtimer` 2-hart fan-out), V-UT-5
  (`Sswi` 3-hart fan-out), V-UT-6 (`Bus::new(_, _, 4)`), V-UT-7
  (`MachineConfig::default().num_harts == 1`). That's **seven new
  PR1 lib tests**. V-IT-3 (`round_robin_fairness_single_hart`) is
  also tagged `(PR1)` at `00_PLAN.md:644-647` — an eighth. Plus
  V-UT-8 restates an existing test pass-through, so not additive.
  PR1's post-merge count by this inventory is 354 + 8 = 362 lib
  tests, not 354.
- Why it matters:
  Gate-matrix arithmetic is the single hardest requirement on every
  PR in this project (archLayout 04's C-6, aclintSplit 01's V-IT-1).
  A contradiction between the "no net add" claim and a
  ≥8-test addition makes the PR1 exit check ambiguous: reviewers
  won't know whether 354 or 362 is the ceiling, and a post-merge CI
  run will have to pick one or fail. The plan must state the post-PR1
  count explicitly, or drop the "same count" assertion.
- Recommendation:
  Replace `00_PLAN.md:534-535` with:
  > `X_ARCH=riscv64 cargo test --workspace` — 354 pre-existing lib
  > + 8 new multiHart PR1 lib (V-UT-1/2/3/4/5/6/7, V-IT-3) + 1
  > `arch_isolation` + 6 `xdb` = **369** tests pass.
  And update C-6 / G-7 to match. Alternative: mark V-UT-3..7 and
  V-IT-3 as PR2 tests (they do validate multi-hart paths, which is
  PR2's purpose), move V-UT-1 / V-UT-2 into PR1 as the only two new
  PR1 lib tests (354 → 356), and reconcile. Either way, the
  arithmetic must add up.



### R-004 `"Mechanical" framing understates PLIC scope`

- Severity: MEDIUM
- Section: Summary / Non-Goals / Trade-off T-3
- Type: Maintainability / Framing
- Problem:
  Summary line at `00_PLAN.md:29-31` and NG-1 at `00_PLAN.md:150-152`
  both describe PR2's PLIC change as "a mechanical extension that
  preserves single-hart offsets". R-001 shows it's a constructor
  signature change, a field-type change (`irq → irq_per_hart`), a
  rewrite of `evaluate()` to index per-hart, and the deletion of a
  compile-time CTX_IP table. This is materially more than
  "mechanical". TR-3's option (b) — splitting PR2 into a
  PR2a-PLIC-runtime-size commit and PR2b-DTS+SMP commit — exists
  exactly to bisect risk when the "mechanical" work turns out non-
  mechanical, but the plan rejects (b) in favour of (a) citing
  "smallest landable unit" at line 593.
- Why it matters:
  If PR2 flakes on SMP Linux boot (V-IT-5), bisecting whether the
  bug is in PLIC re-routing, DTS wiring, OpenSBI HSM handoff, or
  Linux's own SMP bring-up is materially harder with everything in
  one commit. archLayout-04 and aclintSplit-01 both chose narrow
  PRs specifically for this reason.
- Recommendation:
  Either (i) reframe NG-1 and the Summary to say "PR2 performs a
  targeted PLIC runtime-size conversion (see R-001 for delta);
  full PLIC gateway redesign is deferred to `plicGateway`"; or
  (ii) adopt TR-3 (b): PR2a = PLIC runtime-size + `Plic::new(Vec)`
  at num_harts=1 (byte-identical, since Vec-of-1 collapses), PR2b =
  DTS + CLI `--harts` + SMP gate. (ii) is the reviewer's preferred
  path (see TR-3 advice below).



### R-005 `RVCore::with_bus signature change impacts external seams`

- Severity: MEDIUM
- Section: API Surface / Implementation Plan
- Type: API / Seam
- Problem:
  Plan at `00_PLAN.md:326` specifies
  `pub fn with_bus(bus: Bus, irqs: Vec<IrqState>) -> Self;`, a
  breaking change from today's
  `pub fn with_bus(bus: Bus, irq: IrqState) -> Self`
  (`cpu/mod.rs:93`). `with_bus` is a **`pub`** constructor used by
  any downstream consumer that wants to hand-roll a bus (tests,
  difftest harnesses, out-of-tree driver experiments). The plan
  doesn't audit call sites outside `cpu/mod.rs` or name the blast
  radius. It also doesn't explain why `with_bus` takes
  `Vec<IrqState>` instead of computing the per-hart Vec from
  `num_harts` internally (the default `new` / `with_config` paths
  already do; only external callers construct a `Bus` manually).
- Why it matters:
  At `num_harts = 1` the natural shape is
  `with_bus(bus: Bus, irq: IrqState)` exactly as today — callers
  hand one IrqState, the core wraps it in a `vec![irq]`
  internally. Taking a `Vec<IrqState>` makes every external caller
  responsible for vec-construction for no gain at `num_harts = 1`.
  If downstream crates depend on this signature, the break is
  source-incompatible with no behavioural benefit in PR1.
- Recommendation:
  Pick one of:
  (a) Keep `with_bus(bus, irq: IrqState)` unchanged; internally
      build `vec![irq; num_harts]`. Requires the caller to know
      `num_harts` from `bus` — add `Bus::num_harts()` accessor if
      needed.
  (b) Rename the new-shape constructor `with_bus_and_irqs` /
      `with_bus_multihart`, keep `with_bus(bus, irq)` as a
      single-hart delegator that forwards `vec![irq]`. Zero source
      break at num_harts=1.
  (c) Keep the plan's `Vec<IrqState>` if the reviewer is wrong about
      downstream callers — but justify by grepping the workspace
      and any public consumers.
  Prefer (b); it's the cleanest single-hart-compatible shape.



### R-006 `PR2 file list omits Plic::new call site`

- Severity: MEDIUM
- Section: Implementation Plan / Phase 2 file list
- Type: Completeness
- Problem:
  `00_PLAN.md:549-555` lists PR2-touched files:
  `arch/riscv/cpu/csr.rs`, `arch/riscv/cpu/mod.rs`,
  `arch/riscv/device/intc/plic.rs`, `config/mod.rs`, CLI main, and
  `resource/Makefile`. If R-001 is adopted, `arch/riscv/cpu/mod.rs`
  is already listed — but the `Plic::new(irq.clone())` call at
  `cpu/mod.rs:68` must be rewritten to pass `Vec<IrqState>`, and
  this is not called out specifically. Also `Aclint::install`
  signature changes already in PR1 (`00_PLAN.md:346-349`) mean the
  call at `cpu/mod.rs:61` gets a new argument list — PR1 file list
  mentions `mod.rs` generically but doesn't name the argument
  rewrite.
- Why it matters:
  File lists are the contract between the plan and the code review
  reader. If the reviewer expects only `NUM_CTX` to change in
  `plic.rs` and the actual diff also rewrites `Plic::new`,
  `evaluate`, `claim`, `complete`, the review gets surprised.
  archLayout-04 set the precedent of enumerating "exact function or
  constant" deltas.
- Recommendation:
  Expand Phase-2 step 14 to:
  > 14. PLIC runtime-size: delete `const NUM_CTX`, `const CTX_IP`;
  > add `num_ctx: usize`, `irqs: Vec<IrqState>` fields; rewrite
  > `Plic::new(num_harts: usize, irqs: Vec<IrqState>) -> Self`;
  > rewrite `evaluate()` to index `self.irqs[ctx >> 1]` with
  > `ip = if ctx & 1 == 0 { MEIP } else { SEIP }`; resize
  > `enable` / `threshold` / `claimed` to `2 * num_harts`; update
  > the `Plic::new` call at `cpu/mod.rs:68` to pass the per-hart
  > IrqState vector built in `with_config`.
  And add an equivalent bullet to Phase 1 for `Aclint::install`'s
  new argument tuple and `cpu/mod.rs:61` rewrite.



### R-007 `Acceptance Mapping omits I-1/I-2/I-3/C-7`

- Severity: LOW
- Section: Validation / Acceptance Mapping
- Type: Validation
- Problem:
  The Acceptance Mapping table at `00_PLAN.md:687-707` maps 15 of
  18 specification items (G-1..G-8, some C-*, some I-*) to their
  validations but omits:
  - I-1 (`harts.len() == num_harts` invariant) — implied by
    V-UT-7 indirectly, not explicitly mapped.
  - I-2 (`harts[i].id == HartId(i)`) — no validation.
  - I-3 (per-hart stride decode; `hart_id = offset / stride`) —
    partially covered by V-UT-3/4/5/V-E-2 but not named.
  - C-7 (≤ 360-line body budget) — self-witnessing, but should be
    noted as "checked at plan-review time" for consistency with
    archLayout-04 / aclintSplit-01.
- Why it matters:
  The Acceptance Mapping is the single traceability gate the
  executor uses to check "can I ship PR1 / PR2". Gaps in it create
  invariants that land without tests, which is exactly how silent
  assumptions become bugs.
- Recommendation:
  Add four rows to the table:
  | I-1 | V-UT-7 (default=1) + V-UT-6 (explicit 4-hart length) |
  | I-2 | new V-UT-9 `HartId ordering preserved`: after
        `with_config(num_harts=3)`, `harts[i].id == HartId(i as u32)`
        for i in 0..3 |
  | I-3 | V-UT-3/4/5 + V-E-2 |
  | C-7 | Checked at plan-review time; body line count ≤ 360 |



### R-008 `OpenSBI HSM assumption is unverified`

- Severity: LOW
- Section: Non-Goals / PR2 Boot path
- Type: Spec Alignment
- Problem:
  NG-8 at `00_PLAN.md:172-175` claims "OpenSBI builds from
  `resource/opensbi/` already support SMP via `platform-override`
  if the DTB declares multiple harts — no `.mk` edit needed". The
  plan does not cite an SBI version, does not name the HSM
  (`SBI_HSM`) extension requirement (SBI v0.2+ for `sbi_hsm_hart_
  start`), and does not verify the existing OpenSBI build turns on
  HSM. `resource/opensbi.mk` is not excerpted. At PR2 step 16,
  `OpenSBI reads DTB and starts both harts; hart 0 is boot hart,
  hart 1 enters SBI wait-for-IPI; SBI `sbi_hsm_hart_start` resumes
  hart 1 on Linux request` is a precondition, not a validation.
- Why it matters:
  If OpenSBI in this tree lacks HSM, Linux boots and pins only hart
  0 online (the `dmesg` `Brought up 1 node, 1 CPU` case, which the
  plan itself marks as a PR2 gate failure at V-F-5). Discovering
  this after PR2 is written wastes the PR.
- Recommendation:
  Before PR2 starts, either (a) confirm via `rg -l "sbi_hsm"
  resource/opensbi/` or `make -C resource/opensbi print-platform`
  that HSM is in the build; or (b) add an NG acknowledgement: "If
  the current OpenSBI lacks HSM, a `resource/opensbi.mk`
  `PLATFORM_FEATURES += hsm` flag will be required; this is PR2
  scope." No design change, but the conditional must be pre-flagged.



### R-009 `ebreak_as_trap placement vs. per-hart semantics`

- Severity: LOW
- Section: Data Structure
- Type: Design
- Problem:
  `00_PLAN.md:199-211` puts `ebreak_as_trap: bool` on `RVCore`
  (machine-scoped), alongside `bus`. Semantically, `ebreak_as_trap`
  controls what `Hart::step_one` does when it hits an `ebreak`
  instruction — today it's checked at retire time inside the step
  loop. That's per-hart execution state. Putting it on `RVCore`
  means every `Hart::step_one` call needs either to receive
  `ebreak_as_trap` as a parameter (which the plan's signature at
  line 318 does: `fn step_one(&mut self, bus: &mut Bus,
  ebreak_as_trap: bool) -> XResult`) or to read it through a
  back-reference to `RVCore` (which creates a cycle). The parameter
  approach works but threads machine-scoped policy through every
  hart call. At `num_harts = 1` this is fine; at `num_harts = N`
  it's a pattern: every other machine-scoped policy will need the
  same plumbing. Consider whether `ebreak_as_trap` should be on
  `Hart` for locality (all harts share the same value, but each
  reads its own copy — mirrors how every hart has its own `halted`
  flag).
- Why it matters:
  Not a correctness issue — just a coupling smell. The plan's
  current shape works; the alternative (per-hart copy of
  `ebreak_as_trap`) removes the parameter from `step_one` at a
  small storage cost (1 byte × N harts).
- Recommendation:
  Reviewer accepts the plan's current shape (machine-scoped
  `RVCore.ebreak_as_trap` + pass-through parameter). If the
  executor finds the parameter threading too noisy during
  implementation (>5 call sites), mirror the flag onto `Hart` as
  a no-op storage duplication. Document the choice in a one-line
  note to the next plan round either way.



### R-010 `mhartid PR1/PR2 split is subtle`

- Severity: LOW
- Section: Implementation Plan / I-6
- Type: Correctness
- Problem:
  Plan at `00_PLAN.md:258-260` says "today `mhartid` is a hard-coded
  `0` in `xcore/src/arch/riscv/cpu/csr.rs:250`; PR1 keeps that
  behaviour at num_harts=1, PR2 routes it through `Hart::id`". At
  `num_harts = 1` both paths read 0; at num_harts=1 with the PR1
  refactor, `Hart[0].csr` is a freshly-constructed `CsrFile` where
  the hard-coded `mhartid = 0` lives. Good. But `CsrFile::new()` is
  shared across all harts in PR1 — if the executor writes
  `Hart::new(id, …)` to seed `csr.set(mhartid, id.0)` proactively
  in PR1, mhartid becomes per-hart in PR1 (safe since
  `num_harts=1`). If they defer it to PR2, hart 0's mhartid stays
  tied to the hard-coded CSR and PR2 needs to drop the
  hard-coding **and** seed in `Hart::new`. Both orders work; the
  plan picks the second but doesn't say why.
- Why it matters:
  Deferring mhartid seeding to PR2 means PR1 leaves a hard-coded
  value that must be deleted later. Doing it in PR1 is a one-line
  addition with zero observable effect. The plan's choice adds a
  small PR2 diff for no benefit.
- Recommendation:
  Move mhartid seeding from PR2 step 13 to PR1 step 10: in
  `RVCore::with_config`, after `Hart::new(HartId(i), irq)`, do
  `hart.csr.set(CsrAddr::mhartid, i as Word);`. Delete the
  hard-coded `mhartid = 0` at `csr.rs:250` in PR1 — this becomes
  a dynamic CSR (still `[RO]` from the guest's perspective; only
  host-side `Hart::new` writes it). PR2 gets no mhartid-specific
  diff, only the `num_harts > 1` activation. Validated by V-E-4
  unchanged.



---

## Trade-off Advice

### TR-1 `Scheduling model — round-robin vs. N-instruction burst vs. work-stealing`

- Related Plan Item: `T-1`
- Topic: Performance vs Simplicity
- Reviewer Position: Concur with chosen option (a)
- Advice:
  Keep one-instruction round-robin. No change.
- Rationale:
  (a) is deterministic, matches Spike's default scheduling, and
  degenerates to today's exact step loop at `num_harts == 1`
  (satisfying I-4 trivially). (b) risks starving interrupt
  delivery to the non-current hart when that hart has a
  time-critical MTIP — not a theoretical concern; Linux's
  `__arch_get_hw_counter` polls frequently and a burst scheduler
  can delay `time` CSR sync by thousands of instructions. (c)
  (work-stealing / skip-halted) is correct behaviour that belongs
  in a later perf task; skipping halted harts in PR2 would mean
  hart 1 (starting halted per the boot path) would never execute
  until MSIP releases it, which ironically needs hart 0 to tick —
  so the naive "skip halted" breaks the SBI HSM handshake.
  Deferring to a perf task lets that task design skip-logic
  alongside an interrupt-prompt cross-hart wakeup.
- Required Action:
  Keep as is. Consider adding a one-line note to T-1 explaining
  why (c) would break the SBI HSM handshake if naively
  implemented — future-proofs the rationale.



### TR-2 `Hart as struct vs. SoA on RVCore`

- Related Plan Item: `T-2`
- Topic: Flexibility vs Performance
- Reviewer Position: Concur with chosen option (a)
- Advice:
  Keep `Vec<Hart>`. No change.
- Rationale:
  (a) matches the existing field layout (every per-hart field is
  already a contiguous region of `RVCore`, so moving them as a
  block is the natural refactor); (b) SoA would require splitting
  every instruction dispatcher to index by hart and fighting the
  borrow checker on partial borrows like
  `(&mut harts[i].csr, &bus)`. archLayout-04's
  "clean, concise, elegant" directive aligns with (a).
- Required Action:
  None.



### TR-3 `PR count — 2 PRs vs. 3 PRs`

- Related Plan Item: `T-3`
- Topic: Maintainability vs Velocity
- Reviewer Position: **Prefer option (b)** — 3 PRs
- Advice:
  Adopt (b): PR1 (refactor at num_harts=1), PR2a (PLIC runtime-size
  conversion + `Plic::new(Vec<IrqState>)` still at num_harts=1 —
  byte-identical because Vec-of-1 collapses), PR2b (DTS + `--harts
  N` CLI + SMP Linux boot gate).
- Rationale:
  R-001 shows PR2 as currently scoped bundles three orthogonal
  risks: PLIC runtime-size conversion (pure refactor, testable at
  num_harts=1), DTS / OpenSBI HSM (external contract with the
  firmware), and SMP Linux boot (end-to-end integration). A flake
  in any one of these blocks the PR. Splitting PR2 → PR2a + PR2b
  lets PR2a land behind a green bar **without requiring an SMP
  Linux build to exist**, and PR2b reduces to DTS + CLI + boot
  gate — a much tighter bisection target if SMP boot flakes.
  archLayout-04 and aclintSplit-01 both consistently chose narrow
  PRs for exactly this reason. The plan's own rationale for
  picking (a) — "smallest landable unit that makes multi-hart
  real" — is belied by R-001: PR2 as drafted is **not** small.
- Required Action:
  Restructure Phase 2 as two sub-phases: **Phase 2a (PR2a)** PLIC
  runtime-size conversion with `num_harts = 1` still the only
  value exercised (zero guest-observable change; V-IT-1 +
  existing PLIC tests unchanged); **Phase 2b (PR2b)** DTS,
  `--harts` CLI flag, `mhartid` per-hart activation (if not moved
  to PR1 per R-010), V-IT-2 / V-IT-4 / V-IT-5 / V-E-4 / V-F-5.
  Gate matrix: PR2a must pass all of Phase-1's gates unchanged;
  PR2b adds the SMP-linux gate on top.



### TR-4 `DebugOps signatures — scalar vs. per-hart`

- Related Plan Item: `T-4`
- Topic: API Stability vs Future Flexibility
- Reviewer Position: Concur with chosen option (a)
- Advice:
  Keep scalar DebugOps. No change.
- Rationale:
  (a) routes through `self.current` (PR2) / hart[0] (PR1) and
  preserves the six xdb tests byte-identical (C-8). (b) would
  churn six xdb tests for zero user-visible gain until the xdb
  hart-selection UX task (NG-6) lands, at which point the right
  API shape will be clear from the UX requirements — premature
  parameterisation would likely pick the wrong shape.
- Required Action:
  None.



### TR-5 `SSIP fan-out — per-hart vs. bitmap`

- Related Plan Item: `T-5`
- Topic: API Spec-alignment vs Micro-optimisation
- Reviewer Position: Concur with chosen option (a)
- Advice:
  Keep per-hart `take_ssip(HartId) -> bool`. No change.
- Rationale:
  (a) mirrors the ACLINT SSWI spec (SSIP is per-hart, not a
  bitmap) and the existing single-hart API shape. (b) caps
  `num_harts` at 64 (or requires u128), and the `Relaxed` atomic
  load is cheap enough that saving N-1 loads per step is
  micro-optimisation unwarranted by any profile.
- Required Action:
  None.



---

## Positive Notes

- **Hart placement at `arch/riscv/cpu/hart.rs` and `pub(in
  crate::arch::riscv)` visibility is exactly right** — honours
  01-M-004 (`cpu/` top-level stays trait-plus-cfg only), 00-M-002
  (topic-organised arch/), and keeps `Hart` / `HartId` out of the
  seam (I-7). The plan's decision to NOT add a top-level
  `crate::cpu::HartId` is correct single-arch-today and trivially
  generalisable later by promoting the newtype if a second arch
  grows its own Hart concept.
- **Per-hart state inventory is thorough.** GPRs, FPRs, PC/NPC,
  CsrFile, MMU, Pmp, IrqState, PrivilegeMode, PendingTrap, LR/SC
  reservation, `halted`, breakpoints/next_bp_id/skip_bp_once are
  all correctly moved (breakpoints-per-hart is the quietly right
  call per NG-9; keeping them on `RVCore` would silently collapse
  xdb's breakpoint semantics at num_harts>1).
- **Round-robin loop at `num_harts=1` is byte-identical.** The
  proposed step body at `00_PLAN.md:226-235` collapses to today's
  exact behaviour when `num_harts == 1` — `current` stays at
  `HartId(0)`, the modular increment is a no-op. I-4 is
  structurally satisfied, not just asserted.
- **Trade-off framing is substantive.** T-1..T-5 each list
  concrete options with rejection rationale; T-3 even pre-offers
  the 3-PR split that the reviewer prefers (see TR-3) — the plan
  anticipates the bisection-clarity argument without fully
  adopting it.
- **ACLINT per-hart extension respects spec-exact strides.** MSWI
  4-byte, MTIMER `mtimecmp` 8-byte, SSWI 4-byte, with `mtime`
  kept scalar (per ACLINT spec §4: mtime is cluster-scoped, not
  hart-scoped) — consistent with the just-landed aclintSplit.
- **NG list is disciplined.** NG-1 (PLIC deferral — caveat per
  R-001), NG-2 (no multi-threading), NG-3 (difftest at
  num_harts=1), NG-6 (xdb UX), NG-9 (per-hart breakpoints) each
  scope out a legitimate follow-up without leaking into PR1's
  surface. This is the right discipline for a multi-PR refactor.
- **The `Bus::mtime` shared invariant is explicitly stated** (NG-5
  at `00_PLAN.md:161-165`) — prevents future "one mtime per hart"
  drift that would contradict ACLINT §4.



---

## Approval Conditions

### Must Fix
- R-001 (HIGH — PLIC runtime-size conversion spec + API delta)
- R-002 (HIGH — cross-hart LR/SC reservation invalidation
  invariant + implementation hook)
- R-003 (HIGH — PR1 test-count arithmetic must reconcile)

### Should Improve
- R-004 (reframe "mechanical" — or adopt TR-3 (b))
- R-005 (`RVCore::with_bus` signature — prefer single-hart-
  compatible shape)
- R-006 (PR2 file list — enumerate PLIC deltas and `cpu/mod.rs`
  call-site rewrite)
- R-007 (Acceptance Mapping — add I-1/I-2/I-3/C-7 rows)
- R-008 (OpenSBI HSM — verify before PR2 starts)
- R-009 (`ebreak_as_trap` coupling — accept as-is with a note)
- R-010 (move mhartid seeding to PR1)

### Trade-off Responses Required
- TR-1 — concurred, optional one-line note on HSM handshake
- TR-2 — concurred, no action
- TR-3 — **prefer (b) 3-PR split**, executor should adopt or
  explicitly justify rejection
- TR-4 — concurred, no action
- TR-5 — concurred, no action

### Ready for Implementation
- No
- Reason: R-001, R-002, R-003 are blocking. R-001 changes the
  PLIC constructor signature and field layout (not captured in
  API Surface); without it PR2 cannot compile. R-002 omits a
  RISC-V ISA invariant (cross-hart LR/SC reservation
  invalidation) that becomes load-bearing at `num_harts = 2` and
  will silently corrupt SMP Linux locks without detection since
  difftest is pinned at num_harts=1. R-003 contradicts its own
  gate-matrix test-count. All three are mechanical fixes in the
  next plan round — no design overhaul required. Recommend
  folding R-001..R-010 + TR-3(b) adoption into 01_PLAN.md and
  re-reviewing.
