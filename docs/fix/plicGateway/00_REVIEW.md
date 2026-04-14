# `plicGateway` REVIEW `00`

> Status: Open
> Feature: `plicGateway`
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
- Blocking Issues: `4`
- Non-Blocking Issues: `7`



## Summary

Round-00 stakes out a coherent three-phase refactor of the monolithic
`Plic` (`xemu/xcore/src/arch/riscv/device/intc/plic.rs`) into the
canonical RISC-V PLIC `source → gateway → core → context` split, plus a
direct device-to-PLIC `PlicIrqLine` signaling handle that retires the
`Bus::tick` bitmap pump at `src/device/bus.rs:227-242`. The architecture
sketch, data-structure shapes, gateway FSMs for both `Level` and `Edge`
sources, MMIO-layout preservation, seam-isolation posture, and
phase-gated 375-test baseline are all named explicitly, and the
acceptance-mapping table ties every Goal and Invariant to at least one
test. T-1..T-5 trade-offs are substantive and each rejected option
carries a concrete rationale. Phase 1 (internal split, behaviour
preserved) is genuinely a no-op semantically if carried out as
described, and Phase 3 (per-source edge config) is appropriately scoped.

Four issues block approval.

First and most serious, the plan's **scope drift** silently folds Task 5
(`directIrq`, MANUAL_REVIEW #5) into Task 4 (`plicGateway`, #6 + #7).
The saved project memory at
`project_manual_review_progress.md:14-16` is explicit: Task 4
plicGateway covers items #6 + #7 (gateway/core/context redesign with
level + edge trigger support), and Task 5 directIrq is a **separate
queued task** covering item #5 (external devices → PLIC direct
signaling). This plan's Summary and Goals G-2/G-3, Phase 2, `PlicIrqLine`
API, and Bus::tick surgery all belong to Task 5 directIrq. Merging the
two tasks into one 8-round iteration contradicts the inherited task
decomposition and widens the blast radius of a single review cycle
past the repo's 5-round convention (memory line 20). See R-001.

Second, the Gateway's `SourceKind::Level` FSM emits a `Clear` decision
from `sample_level(false)` while `in_flight=false` (`00_PLAN.md:471-474`
and state-transition line 4). This matches the current `update()`
behaviour at `plic.rs:56-68` (pending de-asserts when the level line
goes low pre-claim), but **diverges from RISC-V PLIC v1.0.0 §5**, which
specifies that once the gateway forwards a request to the core, the
core's IP bit stays set until claim + complete. SiFive-style
implementations do permit pre-claim de-assertion, so the plan is
consistent with *current* behaviour (Invariant I-1 preservation), but
spec alignment is not actually achieved for pure level semantics —
contradicting the plan's framing. The plan must either name the
deviation as an invariant (paralleling how aclintSplit pinned SiFive
register offsets) or change the Level FSM to latch pending until
complete. See R-002.

Third, **`I-6` and the `AtomicU32`-based `LineSignal` design are
over-engineered for NG-2 single-threaded execution**, with one load-
bearing exception the plan misses: UART's reader thread
(`uart.rs:94-154`, `spawn_reader` → `Arc<Mutex<VecDeque<u8>>>`). The
UART reader thread already exists, and once `PlicIrqLine` replaces
`Device::irq_line`, the reader thread reaching `line.raise()` on new
RX bytes is the natural integration point (otherwise RX IRQs still
wait for a bus tick). The plan handwaves this ("`Send+Sync` is not
load-bearing at runtime", line 217-218) but the reader thread makes
`Send+Sync` load-bearing at runtime the moment Phase 2 lands unless
the plan explicitly forbids the reader thread from touching the line.
Either commit to making the reader thread the eager `raise()` caller
(and reconcile with T-5 Option A "evaluate only at tick boundary") or
commit to the slower "line is set by UART `tick()`" path and document
why the reader thread is disallowed from signaling. See R-003.

Fourth, the plan proposes `Device::tick(&mut self)` with a
default-no-op and PLIC overriding it (T-4 Option A, Phase 2 step 4),
but `Device::tick` **already exists** on the trait at
`src/device/mod.rs:28` and every device (UART, VirtioBlk, MTIMER) is
already `tick`ed on the slow path at `bus.rs:233`. The plan's
"new `Device::tick`" framing misrepresents current code: the change is
not "add a trait method" but "repurpose the existing method from per-
device self-advancement to the PLIC-evaluation rendezvous". This
collides with UART's current `tick()` usage (THRE promotion + rx_buf
drain at `uart.rs:315-324`): if PLIC's `on_signal` runs in the same bus
loop as UART's `tick`, ordering matters — UART's `tick()` updates
`thre_ip`/`rx_fifo` which the new post-Phase-2 UART will translate to
`line.raise()`. The plan never spells out tick-ordering requirements,
and today's MMIO insertion order puts PLIC (index 1) before UART
(index 2) at `cpu/mod.rs:357-365`, so the naive iteration would ship a
one-tick-late IRQ regression. See R-004.

Seven non-blocking items cover: missing Response-Matrix entries for
inherited binding directives (R-005); T-5 conflating "where raise
comes from" with "when evaluation runs" (R-006); `SourceConfig` vs
`Plic::reset` preservation gap (R-007); ambiguous fate of the legacy
`Plic::notify` shim (R-008); edge-latch read-and-clear ordering
ambiguity (R-009); seam-diff validation weaker than aclintSplit's
(R-010); and Phase-2 gate test-count arithmetic that under-counts by
~5× (R-011).

Trade-off advice: TR-1 (module split) concurs with Option B; TR-2
(atomics) prefers Option A but **conditional on R-003**; TR-3
(`Device::irq_line` fate) concurs with Option B (deprecate); TR-4
(`Device::tick` dispatch) flags that Option A is not novel and the
plan must be rewritten to acknowledge the pre-existing trait method;
TR-5 (eager vs deferred) recommends orthogonal-axis framing per R-006
and commits to (A2, B1) conditional on R-003.

---

## Findings

### R-001 `Silent scope merge of directIrq (T5) into plicGateway (T4)`

- Severity: CRITICAL
- Section: Summary / Goals / Phase 2 / Response Matrix
- Type: Spec Alignment
- Problem:
  The saved project-memory note at
  `/Users/anekoique/.claude/projects/-Users-anekoique-ProjectX/memory/project_manual_review_progress.md:14-16`
  decomposes the MANUAL_REVIEW remediation into:
  > Task 4 `plicGateway` (MANUAL_REVIEW #6, #7) — Queued. Gateway +
  > Core + Context redesign with level + edge support.
  > Task 5 `directIrq` (MANUAL_REVIEW #5, #6) — Queued. External
  > devices signal PLIC directly via `PlicIrqLine` instead of through
  > `Bus::write`; async decoupling via `IrqState`.
  The plan's Summary (`00_PLAN.md:16-27`), Goals G-2 (line 112), Phase 2
  (lines 528-556), `PlicIrqLine` API (lines 273-285, 333-345), Bus::tick
  surgery (C-5, line 392-395), and UART/virtio-blk migration (Phase 2
  step 1-2) **belong to Task 5 directIrq**, not Task 4 plicGateway.
  The Response Matrix at lines 37-41 does not acknowledge the task
  boundary and does not justify the merge.
- Why it matters:
  (a) The saved memory at line 20 pins "Plan/Review/Master loop: capped
  at 5 rounds"; this plan opens with an 8-round budget for a merged
  T4+T5 scope. (b) Coupling gateway/core FSM invariants (reviewable in
  isolation) to the cross-file seam changes of `PlicIrqLine` means one
  finding in either half blocks both halves. (c) Implementation review
  becomes non-bisectable — a broken 375 baseline after Phase 2 cannot
  distinguish gateway-FSM bugs from signaling-wire-up bugs across one
  commit window. (d) AGENTS.md §3 Response Rules require every
  inherited binding expectation to be recorded with a decision +
  resolution in the Response Matrix; the task-boundary entry is
  absent.
- Recommendation:
  Pick one of two resolutions.
  (a) **Narrow this plan to plicGateway proper** — items #6 + #7 only
      — keep Phase 1 (internal split) and Phase 3 (edge config), drop
      Phase 2, drop G-2, drop `PlicIrqLine`, drop the Bus::tick change.
      Item #6 ("async interrupt handling") collapses to "PLIC latches
      device state at tick boundary and drives MEIP/SEIP before the
      hart next steps", which is already current behaviour. Defer
      `PlicIrqLine` + direct signaling + Bus::tick surgery to a
      separate `directIrq` iteration as the saved memory lists.
  (b) **Explicitly merge T4 + T5 into one task** with reviewer-visible
      reasoning in the Response Matrix: name the memory entry being
      superseded, list the combined 8-round budget, and split the
      PLAN's Validation section so Phase 1-3 gating is independently
      verifiable. This is acceptable only if the Executor can argue
      that the two tasks are not separable — they are: Phase 1 is a
      legal plicGateway-only refactor with no directIrq intersection.
  Either way, add a Response-Matrix row:
  ```
  | project-memory | T4/T5 boundary | Resolve to (a) or (b) | <reasoning> |
  ```
  Reviewer prefers (a).



### R-002 `Level gateway Clear on pre-claim level-low diverges from PLIC §5`

- Severity: HIGH
- Section: Architecture / State Transition / Invariants / Spec References
- Type: Spec Alignment / Correctness
- Problem:
  The Level FSM at `00_PLAN.md:471-474`:
  > `(armed=true, in_flight=false)` → `(armed=false)` when
  > `sample_level(false)`; emit `Clear`.
  emits `Clear` to the Core when the device drops its line **before**
  the source is claimed. This matches the current monolithic `update()`
  at `plic.rs:64-65` (`self.pending &= !bit` when the line is low), so
  Invariant I-1 (arbitration equivalence with current PLIC) is
  preserved. However, RISC-V PLIC v1.0.0 §5 (Interrupt Gateways)
  specifies that once the global interrupt is forwarded to the PLIC
  core, the gateway will not forward another request for that source
  until it receives a completion message — the spec treats the core's
  IP bit as sticky regardless of subsequent level fluctuations. The
  plan's Spec References at lines 87-100 claim the refactor aligns
  with §5, but the FSM in fact preserves the SiFive-style
  pre-claim-clearable behaviour. Not flagged as deliberate.
- Why it matters:
  (a) A reviewer reading Spec Alignment alongside the FSM will
  conclude the plan is spec-faithful when it is in fact behaviour-
  preserving against a non-spec-pure base. A future kernel driver
  that assumes pure §5 semantics and does not re-raise after a
  spurious line-low would silently regress. (b) The I-1 framing
  "arbitration equivalence … provided all sources are configured as
  `SourceKind::Level` (the default)" is correct for *today's* PLIC
  but misleading re: the spec. (c) aclintSplit set the precedent for
  pinning SiFive-variant choices as explicit invariants; plicGateway
  should follow.
- Recommendation:
  Add Invariant I-8:
  > **I-8** (Level-trigger pre-claim de-assertion — SiFive variant)
  > For `SourceKind::Level`, if the device line goes low before the
  > source is claimed, the gateway clears its `armed` bit and the
  > core's pending bit is cleared. This diverges from RISC-V PLIC
  > v1.0.0 §5 (which treats IP as sticky post-forward) but preserves
  > the behaviour of the current monolithic `Plic::update` and
  > therefore of all existing guest workloads (xv6, linux,
  > linux-2hart, am-tests, cpu-tests). A spec-pure variant is a
  > follow-up feature.
  Add a validation item `V-UT-11 level_pre_claim_dropline_clears`
  asserting this deviation explicitly (rather than tacitly carrying
  it from the old tests). Update Spec References (lines 87-100) to
  call out this choice.



### R-003 `Atomics in LineSignal are load-bearing under UART reader thread`

- Severity: HIGH
- Section: Data Structure / Invariants / Execution Flow
- Type: Correctness / API
- Problem:
  Invariant I-6 (lines 214-218) states:
  > `PlicIrqLine` is `Clone + Send + Sync` … Under NG-2
  > single-threaded execution `Send+Sync` is not load-bearing at
  > runtime, but the trait bounds keep the API forward-compatible
  > with future threaded work.
  and the Data Structure notes at lines 295-298 repeat "under NG-2 we
  do not strictly need atomics". This is factually wrong in the
  presence of UART's reader thread. Ground truth at
  `xemu/xcore/src/device/uart.rs:94-154,179,206,216,321` shows UART
  already spawns a `std::thread` (`spawn_reader`) that pushes RX
  bytes into `Arc<Mutex<VecDeque<u8>>>`; `Uart::tick` drains that
  into `rx_fifo` (line 321); `irq_line()` derives the RX-interrupt
  signal from `rx_fifo`. Once Phase 2 lands, the most responsive
  wiring is for the reader thread itself (or for `tick`) to call
  `PlicIrqLine::raise()` whenever `!rx_fifo.is_empty() && IER.rxne`
  transitions 0→1. If the reader thread calls `raise()`, `Send+Sync`
  is load-bearing *today*, not "forward-compatible".
- Why it matters:
  (a) The plan's rationale for `AtomicU32` is under-justified — it
  casts the atomics as future-looking when they are in fact
  current-release requirements. A future reviewer may propose
  downgrading to `Cell<u32>` / `!Sync` and regress UART.
  (b) Conversely, if the plan intends the reader thread NOT to call
  `raise()` (i.e., `raise()` happens only inside `Uart::tick` on the
  bus-tick thread), the plan must say so. This affects IRQ latency:
  today the reader thread pushes bytes, the guest waits until the
  next bus tick (SLOW_TICK_DIVISOR=64 bus cycles, `bus.rs:58`).
  Phase 2 as written does not change that latency unless the reader
  thread signals directly.
  (c) The edge path (`edge_latch: AtomicU32`) has the same question:
  who calls `pulse()`? If only tick-thread devices, `AtomicU32` is
  pure tax under NG-2; if reader threads or future worker threads,
  `AtomicU32` is necessary.
- Recommendation:
  Pick one posture and state it as an invariant.
  (a) **Cross-thread signaling allowed**:
      > **I-9** `PlicIrqLine::{raise,lower,pulse}` may be called from
      > any thread. Atomic operations on `LineSignal` use
      > `Ordering::Release` for writes and `Ordering::Acquire` for
      > the gateway read in `on_signal`, establishing a happens-
      > before edge such that a `raise()` returning before tick N is
      > visible to tick N.
      Add a failure-flow entry for UART reader thread raising
      `PlicIrqLine` while `Plic::reset` is mid-flight (the raise must
      either land or be dropped; no split-visibility).
  (b) **Tick-thread only**:
      > **I-9** `PlicIrqLine` methods are called only from the bus
      > tick thread. The UART reader thread continues to push into
      > `rx_buf`; `Uart::tick` computes the new line state and calls
      > `raise()/lower()`. `LineSignal` uses `Relaxed` ordering.
      Then downgrade to `Cell<u32>` + `!Sync`, matching NG-2.
  Reviewer recommends (a) — removes the SLOW_TICK_DIVISOR=64 latency
  floor for RX and matches real-hardware UART behaviour. Either way,
  add a V-IT case exercising the chosen posture.



### R-004 `Device::tick repurposing collides with existing per-device tick`

- Severity: HIGH
- Section: Execution Flow / Implementation Plan / Phase 2
- Type: API / Flow
- Problem:
  Phase 2 step 4 at lines 546-548:
  > Modify `Bus::tick`: remove the `irq_lines` bitmap collection loop
  > (`src/device/bus.rs:227-242`). Replace with per-device `Device::tick()`
  > (default no-op, overridden by `Plic` to call `on_signal`).
  and T-4 Option A at lines 612-615:
  > new `Device::tick(&mut self)` default-no-op, PLIC overrides
  > (arch-neutral; adds one trait method).
  misrepresent the current state of `trait Device`. Ground truth at
  `xemu/xcore/src/device/mod.rs:28`:
  > `fn tick(&mut self) {}`
  already exists, already has a default no-op, and is already called
  for every MMIO device on the slow path at `bus.rs:233`. UART
  overrides it at `uart.rs:315-324`; MTIMER overrides it to advance
  mtime. The actual change is:
  1. Add `Plic::tick` override calling `on_signal`.
  2. Delete the bitmap-collection loop at `bus.rs:227-242`.
  3. Ensure tick ordering: every non-PLIC device's `tick()` runs
     before PLIC's `tick()` so that level signals computed during
     each device's own `tick` are visible to PLIC's evaluation in
     the same tick boundary.
  The plan does not address (3). With the current `mmio.iter_mut()`
  traversal order at `bus.rs:229`, PLIC is installed at index 1 and
  UART at index 2 (`cpu/mod.rs:357-365`), so PLIC's `tick` would run
  *before* UART's `tick` in a naive iteration, and UART's new
  `raise()`-on-`tick` pattern would be one tick late.
- Why it matters:
  One-tick-late IRQ delivery is a functional regression, not just a
  latency tweak. A guest that expects MEIP to be pending by the time
  `mret` returns from a prior trap may deadlock, loop, or miss bytes
  (UART RX drops under fast input). V-IT-5 (xv6 boot) is too coarse
  to catch a 64-cycle regression reliably. The plan as written would
  ship this bug on Phase 2 merge.
- Recommendation:
  Rewrite Phase 2 step 4 to:
  (a) Acknowledge `Device::tick` is pre-existing.
  (b) State a bus-tick ordering invariant:
      > **I-10** In `Bus::tick` slow path, `Plic::tick` runs **after**
      > every other device's `tick`. Implementation: two-pass loop —
      > pass 1 ticks every device except `plic_idx`; pass 2 ticks
      > `plic_idx`. (Alternative: sort tick order at `set_irq_sink`
      > time.)
  (c) Delete the `r.irq_source > 0 && r.dev.irq_line()` bitmap
      collection (lines 235-236). The `irq_source` field on
      `MmioRegion` becomes dead weight; decide explicitly to remove
      or repurpose it.
  (d) Decide `Plic::notify` override fate (see R-008).
  Add a unit test `bus_tick_ordering_plic_last` that asserts the
  pass order. Add a V-IT case exercising the one-tick-delay
  regression: UART raises in its own `tick`, PLIC observes MEIP in
  the same bus tick, not the next.



### R-005 `Response Matrix missing inherited-directive acknowledgement`

- Severity: MEDIUM
- Section: Log / Response Matrix
- Type: Spec Alignment
- Problem:
  The Response Matrix at lines 37-41 contains only:
  > | — | — | — | No prior REVIEW or MASTER for this feature. |
  The saved project memory at
  `project_manual_review_progress.md:21-27` records five inherited
  MASTER directives from archModule (00-M-001/002, 01-M-001..004)
  that are binding on every remaining task. The plan interacts with
  at least 00-M-002 (topic-organised `arch/<name>/` layout — the
  `plic/` subdir is compliant) and 01-M-004 (top-level
  `cpu/`/`device/`/`isa/` = trait APIs + tiny `#[cfg(arch)]` patches
  only — `PlicIrqLine` is the new seam crossing, C-3). Neither is
  named in the Response Matrix. AGENTS.md §3 "Response Rules"
  requires every MASTER directive that bears on the current plan to
  appear there.
- Why it matters:
  Inherited directives that are never named become implicit; drift
  accrues silently across iterations. aclintSplit round-00 R-001
  flagged the same class of issue.
- Recommendation:
  Add rows for every inherited binding directive the plan interacts
  with, at minimum:
  | Source | ID | Decision | Resolution |
  |--------|----|----------|------------|
  | archModule | 00-M-002 | Honored | `plic/` subdirectory under `arch/riscv/device/intc/`. |
  | archLayout | 01-M-004 | Honored | `PlicIrqLine` is the only seam addition; `Device::tick` override lives in arch/. |
  | project-memory | T4/T5 boundary | See R-001 | … |



### R-006 `T-5 conflates "where raise comes from" with "when evaluation runs"`

- Severity: MEDIUM
- Section: Trade-offs
- Type: Flow
- Problem:
  T-5 (lines 620-627) frames eager vs deferred evaluation as a
  choice between "evaluate at tick boundary" (A) and "evaluate on
  every raise/lower" (B). Option B's rejection rationale — "risks
  re-entrancy because raise may happen while Bus holds `Bus` lock in
  multiHart" — is accurate but incomplete. The real question is
  orthogonal: where does `raise()` happen? If the UART reader thread
  calls `raise()` (R-003 option a), Option A still evaluates at tick
  boundary, but the *visibility* of the raise is immediate (atomic
  store), not deferred. The plan conflates "when evaluation runs"
  with "when level is observed".
- Why it matters:
  T-5's framing as written may be read as "raise() only happens
  inside a device's tick()", which would preclude R-003 option (a)
  and cement the SLOW_TICK_DIVISOR=64 latency floor for RX.
- Recommendation:
  Rewrite T-5 options on two orthogonal axes:
  - Raise caller: bus-thread tick (A1) vs any-thread including UART
    reader (A2).
  - Evaluation site: tick boundary (B1) vs on every raise (B2).
  Proposed combination: (A2, B1) — cross-thread raise visible as
  atomic store; PLIC evaluates at the immediately next tick boundary
  the bus reaches. Keeps determinism (B1), removes RX latency (A2).
  Reject (B2) unconditionally as today.



### R-007 `SourceConfig vs Plic::reset preservation is unspecified`

- Severity: MEDIUM
- Section: API Surface / Failure Flow / Phase 3
- Type: Correctness
- Problem:
  The plan proposes `Plic::with_config(num_harts, irqs, [SourceConfig;
  NUM_SRC])` (lines 308-312) to configure per-source Level/Edge at
  construction, and Phase 3 step 2 chooses "Option (a) a board-level
  constant table baked at construction" (lines 568-572). But
  `Plic::reset` at `plic.rs:174-180` and Phase-1/2 preservation
  thereof must not clobber source kinds — a reset is not a
  reconfiguration. The Failure Flow §5 (lines 462-465) says reset
  clears gateway, core, and `LineSignal`, but does not say
  "`Gateway::kind` is preserved across reset". If reset zeroes out
  the gateway state struct verbatim (which includes `kind:
  SourceKind` per Data Structure line 240), all sources revert to
  whatever the default enum variant is, silently breaking the
  board's edge configuration after a guest-triggered reset.
- Why it matters:
  Guest-triggered reset (VirtIO-blk soft reset at `virtio_blk.rs:220`,
  OpenSBI system reset) would silently reset every source to Level,
  producing mysterious IRQ-delivery behaviour only after a reset.
  Difftest does not currently reset.
- Recommendation:
  Add Invariant I-11:
  > **I-11** `Plic::reset` preserves `Gateway::kind` and
  > `SourceConfig`. Reset clears `armed`, `in_flight`, `prev_level`,
  > `pending`, `enable`, `threshold`, `claimed`, `LineSignal.level`,
  > and `LineSignal.edge_latch` — not per-source kind.
  Add V-F-5: construct with one source configured as Edge, reset,
  assert edge config survives.



### R-008 `Plic::notify legacy shim fate is ambiguous`

- Severity: MEDIUM
- Section: Implementation Plan / Open Questions
- Type: API
- Problem:
  Phase 2 step 3 at lines 541-543:
  > Change `Plic::notify` to a no-op deprecated shim (still there to
  > keep the trait signature happy for other devices that haven't been
  > migrated; the body does nothing).
  but `Device::notify` is a trait *method* with a default no-op at
  `device/mod.rs:34`. PLIC is the *only* device that overrides it
  (grep: `rg 'fn notify\(' xemu/xcore` shows PLIC only). "Keeping
  the trait signature happy for other devices that haven't been
  migrated" describes a non-issue. Meanwhile, Open Q 3 (lines
  763-765) asks whether to remove `Plic::notify` in Phase 2 or keep
  it. The two sections contradict.
- Why it matters:
  Confusion between "trait method" (can always have a default) and
  "PLIC's override" (dead code post-Phase 2) invites half-migration
  bugs where a developer assumes `notify` is the canonical interrupt
  path and adds a new device with a `notify()` call that never fires.
- Recommendation:
  Decide explicitly. Reviewer recommends: remove both. In Phase 2,
  delete PLIC's `notify` override; in Phase 3 (or as Phase-2
  cleanup) remove `Device::notify` from the trait. If `Device::notify`
  is kept for future devices, add a comment pinning intent.



### R-009 `Edge-latch read-and-clear ordering is under-specified`

- Severity: MEDIUM
- Section: Data Structure / State Transition / Invariants
- Type: Correctness
- Problem:
  `LineSignal.edge_latch: AtomicU32` (line 283) stores one bit per
  source, rising-edge latched. Phase 3 step 3 wires
  `PlicIrqLine::pulse()` to set the bit; `on_signal` reads-and-clears
  the bit for edge-configured sources. The FSM at lines 482-488
  coalesces at the Gateway level (multiple `on_edge()` while
  `armed=true` collapse to one pending). That is correct for rapid
  edges. But if `pulse()` fires between `on_signal`'s read-and-clear
  and its `gateway.on_edge()` call, whether `on_signal` observes it
  in the current tick or the next depends on internal ordering. The
  plan does not pin this.
- Why it matters:
  For a device that fires tight-back-to-back edges (not in tree
  today; future use case) the boundary case where a pulse straddles
  the read-and-clear is under-specified. Under-specification at
  Phase-3 sign-off risks a correctness regression when edge sources
  are actually adopted.
- Recommendation:
  Add a sequencing note to the Execution Flow:
  > `on_signal` processes all sources in one pass: for each edge
  > source, `fetch_and(!bit, AcqRel)` the edge_latch to obtain the
  > pre-clear value, then call `gateway[s].on_edge()` if that bit
  > was set. Pulses that arrive between the `fetch_and` and the call
  > are observed on the next `on_signal` pass (acceptable per spec
  > coalesce semantics if the gateway is still `armed`).
  Document as part of I-3 and add V-UT-12
  `gateway_edge_pulse_after_fetch_and_lands_next_tick`.



### R-010 `Seam-diff validation weaker than aclintSplit precedent`

- Severity: LOW
- Section: Validation / Constraints
- Type: Validation
- Problem:
  C-3 (lines 385-388) correctly calls for adding `PlicIrqLine` to
  `SEAM_ALLOWED_SYMBOLS` at
  `xemu/xcore/tests/arch_isolation.rs:42-67`. V-IT-2 (lines
  661-662) asserts `cargo test arch_isolation` passes, which is
  necessary but not sufficient — it does not assert that *only*
  `PlicIrqLine` was added (not, e.g., `SourceKind` or `LineSignal`
  by mistake). aclintSplit round-00 R-002 flagged the same gap
  class.
- Why it matters:
  If the executor accidentally marks `SourceKind` as `pub` and
  re-exports it through `device/intc/mod.rs`, adding `"SourceKind"`
  to the allow-list silences the test but violates architectural
  intent (the plan pins SourceKind as `pub(super)` crate-internal).
- Recommendation:
  Add V-IT-8: pin the post-plicGateway allow-list as exactly
  `[…existing symbols…, "PlicIrqLine"]` — any drift fails.
  Alternatively, a Phase-2 `git diff` gate that shows the seam file
  diff adds exactly one line.



### R-011 `Phase-2 test-count arithmetic under-counts by ~5×`

- Severity: LOW
- Section: Validation / Implementation Plan
- Type: Validation
- Problem:
  Phase 2 gate at lines 554-556 reads:
  > 375 + N new tests pass (N ≥ 3 for gateway level, gateway edge,
  > and direct-signaling integration)
  but the Validation section enumerates 10 UT + 7 IT + 4 F + 6 E.
  Net new tests by Phase 2 boundary (excluding Phase-3-reserved
  V-UT-6/7): V-UT-2..5 (4) + V-UT-8..10 (3) + V-IT-1 (1) + V-F-1/2/4
  (3) + V-E-1..4 (4) ≈ 15, not 3.
- Why it matters:
  Under-counting the new-test budget makes the gate trivial —
  "add 3 tests, ship" — instead of surfacing the intended coverage
  breadth.
- Recommendation:
  Restate Phase 2 gate as:
  > Baseline 375 + new UT ≥ 7 (V-UT-2/3/4/5/8/9/10; V-UT-6/7 reserved
  > Phase 3) + new IT ≥ 1 (V-IT-1) + new F ≥ 3 (V-F-1/2/4) + new E ≥
  > 4 (V-E-1/2/3/4) = ≥ 15 new tests post Phase 2.
  Reconcile with the Validation-section enumeration so the Response
  Matrix arithmetic is unambiguous.



---

## Trade-off Advice

### TR-1 `Module split granularity`

- Related Plan Item: `T-1`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option B
- Advice:
  Adopt the proposed `plic/{mod,source,gateway,core,line}.rs` split.
- Rationale:
  The feature's explicit goal is separating responsibilities the
  current code entangles; keeping the file monolithic undermines the
  premise. Each file is projected at 80-200 lines (common/coding-
  style: 200-400 typical, 800 max). aclintSplit set the precedent
  (3-file split). Phase 1 is strictly additive on tests so initial
  churn is bounded.
- Required Action:
  Keep Option B. Document file-line budgets as a soft cap in the
  plan (e.g. "each file ≤ 250 lines in Phase-1 completion").



### TR-2 `Concurrency primitives in LineSignal`

- Related Plan Item: `T-2`
- Topic: Flexibility vs Safety
- Reviewer Position: Prefer Option A — conditional on R-003 resolution
- Advice:
  Keep `AtomicU32` **and** pin the justification on the UART-reader-
  thread cross-thread raise path (R-003 option a).
- Rationale:
  Option A's cost is negligible on hot platforms. Option B
  (`Cell<u32>`) forecloses the UART-reader-thread optimisation
  forever and pushes RX IRQ latency to SLOW_TICK_DIVISOR=64 cycles
  unconditionally. Option C (`Mutex<u32>`) introduces a bus-side
  lock in the signaling hot path.
- Required Action:
  Adopt Option A. Rewrite the "not strictly needed" prose at lines
  217-218 and 295-298 to "required because UART reader thread calls
  raise() cross-thread". Tie atomic ordering choice to I-9 from
  R-003.



### TR-3 `Fate of Device::irq_line`

- Related Plan Item: `T-3`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option B
- Advice:
  Deprecate in Phase 2, remove in a later iteration.
- Rationale:
  `Device::irq_line` is called at `bus.rs:235` by the bitmap
  collection loop Phase 2 deletes. After deletion there is no
  in-tree caller (tests only: `uart.rs:408/410/412`,
  `virtio_blk.rs:310/312`). Immediate removal is low-risk
  technically but leaves a gap in review; `#[deprecated]` lets the
  tests migrate incrementally.
- Required Action:
  Adopt Option B. Add a Phase-2 action listing the 5 test call
  sites and the concrete migration pattern (construct a
  `PlicIrqLine`, assert via the new API, or assert the internal
  flag directly).



### TR-4 `Where on_signal is called from`

- Related Plan Item: `T-4`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A — with R-004 correction
- Advice:
  PLIC overrides `Device::tick` (the existing trait method), with
  explicit tick-ordering invariant (R-004 I-10).
- Rationale:
  Options B/C break arch isolation or require back-pointers;
  Option A is the minimum-surface change. However, the plan
  misrepresents Option A as "adds one trait method" — it doesn't,
  because `Device::tick` already exists. The correction is editorial
  but load-bearing for reviewability.
- Required Action:
  Rewrite T-4 prose to acknowledge `Device::tick` is pre-existing
  and the change is (a) override it in PLIC, (b) guarantee PLIC
  ticks last.



### TR-5 `Eager vs deferred evaluation`

- Related Plan Item: `T-5`
- Topic: Performance vs Simplicity
- Reviewer Position: Prefer Option A — with R-006 orthogonal axis
- Advice:
  Evaluate at tick boundary (B1); allow cross-thread raise (A2).
- Rationale:
  (B1) keeps determinism and avoids re-entrancy into Core mid-raise.
  (A2) keeps RX latency responsive. The plan's T-5 conflates the two
  axes; clarifying them dissolves most of the apparent tradeoff.
- Required Action:
  Rewrite T-5 options per R-006. Commit to (A2, B1) if R-003
  option (a) is adopted; commit to (A1, B1) if R-003 option (b) is
  adopted.



---

## Positive Notes

- Phase 1 is genuinely a no-op semantically if each step preserves
  the existing tests verbatim; the three-step decomposition
  (core → gateway → line) is well-factored.
- Invariant I-1 (arbitration equivalence) and I-5 (M/S context
  routing) anchor the behaviour-preservation gate and map one-to-one
  to V-UT-10 and V-E-6.
- C-4 (no CSR vocabulary leakage through `PlicIrqLine`) explicitly
  pins MEIP/SEIP/Mip to arch/riscv/ — matches 01-M-004 without
  needing the directive to be named (though R-005 still asks for
  the explicit row).
- Edge-trigger FSM (I-3, lines 206-209) correctly captures coalesce
  semantics per PLIC §5.
- Failure-flow coverage (V-F-1..4) addresses reset-mid-claim and
  raise-after-drop, which prior reviewers commonly flag as omitted.
- Acceptance Mapping table (lines 706-728) traces every G/I/C to a
  test — a habit several earlier plans skipped.



---

## Approval Conditions

### Must Fix
- R-001 — task-boundary narrowing (prefer option a) or explicit
  multi-task acknowledgement in the Response Matrix
- R-002 — name the SiFive-variant pre-claim-clear deviation as
  Invariant I-8
- R-003 — pick and state the UART reader-thread posture; tie
  atomic ordering to it
- R-004 — `Device::tick` is pre-existing; add tick-ordering
  invariant I-10; rewrite Phase 2 step 4

### Should Improve
- R-005 — Response-Matrix inheritance rows
- R-006 — T-5 orthogonal axes
- R-007 — Plic::reset vs SourceConfig preservation (I-11)
- R-008 — `Plic::notify` fate decision
- R-009 — edge-latch read-and-clear ordering note (I-3 refinement)
- R-010 — seam-diff pin beyond `cargo test arch_isolation`
- R-011 — Phase-2 test-count arithmetic

### Trade-off Responses Required
- T-1 — confirm Option B; add file-line soft cap
- T-2 — confirm Option A; rewrite justification per R-003
- T-3 — confirm Option B; add test-migration plan for 5 call sites
- T-4 — rewrite acknowledging pre-existing `Device::tick`
- T-5 — split into orthogonal axes per R-006

### Ready for Implementation
- No
- Reason: Four blocking issues remain. R-001 (scope boundary) is the
  load-bearing one: until it is resolved, a large fraction of Phase 2
  may be out of scope for this iteration entirely. R-002/R-003/R-004
  are correctness/spec-alignment flaws that would ship concrete
  regressions if implemented as-planned. Round 01 should narrow scope
  (R-001), name deviations explicitly (R-002), and pin cross-thread
  semantics and tick ordering (R-003, R-004); after that, Phase 1 is
  approvable for implementation on its own.
