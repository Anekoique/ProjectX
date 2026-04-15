# `directIrq` PLAN `02`

> Status: Revised
> Feature: `directIrq`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `01_MASTER.md`

---

## Summary

Round 02 is a tightening pass on `01_PLAN.md`. No new architecture;
no new API surface. Deltas only:

- R-011 (HIGH, 01_REVIEW) — pin a PLIC-last slow-tick order so a
  same-cycle raise from an adopter's own `Device::tick` (UART rx
  pump, virtio completion) is drained in the same slow tick. New
  invariant **I-D16**, new validation **V-IT-8**.
- R-012 (MEDIUM) — add `xemu/xcore/src/cpu/mod.rs:357` row to the
  Phase-3 call-site table.
- R-013 (MEDIUM) — restructure V-IT-1 so raise and tick actually
  interleave (no join barrier); retain a second "smoke" variant.
- R-014 (MEDIUM) — pin the `reset`-vs-concurrent-raise outcome to
  match real hardware (raise-during-reset is delivered, matching the
  PLIC Gateway's behaviour where assertion that outlives a state
  change still surfaces on the next cycle). Sharpen **V-F-1**.
- M-001/M-002/M-003 (MUST, 01_MASTER) — M-001/M-002 were applied in
  round 01; round 02 adds the M-003 hardware-semantic grounding
  block with manual citations and ties every atomic ordering /
  signalling choice back to the RISC-V PLIC spec and the Rust
  atomics reference.

Inherited directives (archModule, archLayout, plicGateway I-9) are
still honoured, cited-by-reference into 01_PLAN.

## Log

[**Feature Introduce**]

N/A. Round 02 is a revision of 01_PLAN; the three-phase rollout, the
`IrqLine`/`PlicSignals` substrate, the event-driven posture P-A, and
the atomic ordering table are all unchanged relative to round 01.

[**Review Adjustments**]

- R-011 (HIGH) — resolved via I-D16 + V-IT-8 (see §Tick-Order
  Resolution).
- R-012 (MEDIUM) — resolved by adding the `cpu/mod.rs:357` row to
  the Phase-3 call-site table (see §Phase-3 Call-Site Table Delta).
- R-013 (MEDIUM) — resolved by rewriting V-IT-1 into two variants:
  V-IT-1a interleaving (exercises HB), V-IT-1b smoke (the old join
  form, explicitly demoted).
- R-014 (MEDIUM) — resolved by pinning the reset outcome to match
  real-silicon behaviour: bits set concurrently with `reset` may
  persist and surface as a post-reset interrupt assertion. V-F-1
  now asserts this as a positive property, not a vague "some raise
  is observed."
- TR-4, TR-6 (trade-off advice) — closed per 01_REVIEW; TR-6 closure
  text absorbed into §Open Questions OQ-6.

[**Master Compliance**]

- M-001 (MUST, Applied in round 01; re-confirmed) — async posture
  signalling direction is correct. NG-2 not violated because the
  atomic level/edge bitmap is host-side device bookkeeping, not
  guest architectural state. §Async Posture in 01_PLAN:346-454
  stands unchanged.
- M-002 (MUST, Applied) — async semantics must match real silicon.
  Round 02 adds the explicit §Hardware-Semantic Grounding block
  that ties each design choice to a manual citation, not to
  implementation convenience.
- M-003 (MUST, Applied) — manuals consulted and cited inline; see
  §Hardware-Semantic Grounding.

### Changes from Previous Round

[**Added**]

- §Hardware-Semantic Grounding (new section, per M-002 / M-003) —
  citations from RISC-V PLIC v1.0.0, Rust Nomicon, Rust Reference
  atomic ordering docs, ARM GIC v2/v3 manuals, mapped one-to-one
  onto the design choices in 01_PLAN.
- §Tick-Order Resolution — pinned PLIC-last mechanism.
- §Phase-3 Call-Site Table Delta — adds `cpu/mod.rs:357` row.
- §V-IT-1 Restructure — V-IT-1a interleaving variant + V-IT-1b
  demoted smoke.
- §Reset-Race Outcome — V-F-1 sharpened.
- Invariant I-D16 (PLIC-last slow-tick ordering).
- Validation V-IT-8 (same-tick adopter raise observed within one
  slow tick).

[**Changed**]

- V-IT-1 replaced by V-IT-1a + V-IT-1b (01_PLAN:1102-1105).
- V-F-1 tightened (01_PLAN:1130-1132).
- Phase-3 call-site table at 01_PLAN:999-1010 gains one row.
- Acceptance Mapping gains I-D16 row; G-2 row gains V-IT-8.

[**Removed**]

- None. No artifacts from 01_PLAN are retracted.

[**Unresolved**]

- OQ-5 (`loom` exhaustive check) — still deferred. Position:
  hardware-semantic grounding in this round makes `loom` a nice-to-
  have, not a correctness requirement; std::sync::atomic
  Release/Acquire semantics are load-bearing and cited. Re-open
  only if V-E-4 flakes under stress.
- OQ-6 (AtomicBool vs AtomicU32 epoch) — closed per TR-6 (01_REVIEW)
  as AtomicBool; revisit only under OQ-5 loom follow-up.

### Response Matrix

| Source | ID | Severity | Decision | Resolution |
|--------|----|----------|----------|------------|
| Master | M-001 | MUST | Applied (carry) | Async posture P-A unchanged; §Async Posture in 01_PLAN:346-454 satisfies direction. Re-confirmed via §Hardware-Semantic Grounding tie-back. |
| Master | M-002 | MUST | Applied | §Hardware-Semantic Grounding justifies each atomic-ordering choice against manuals. Implementation-convenience reasoning removed; every design decision is traced to a cited spec. |
| Master | M-003 | MUST | Applied | Manuals cited inline: RISC-V PLIC v1.0.0 §2 (Gateway), §3 (Notification), §7-§8 (Claim/Complete); Rust Nomicon §8 (Release/Acquire); `std::sync::atomic::Ordering` API docs; ARM GIC Architecture Spec (level-sensitive semantics). URLs in-line. |
| Master (inh.) | archModule 00-M-002 | — | Honored | `src/device/irq.rs` arch-neutral (01_PLAN:824-829). |
| Master (inh.) | archLayout 01-M-004 | — | Honored | 01_PLAN:824-829. |
| Master (inh.) | plicGateway I-9 | — | Honored | Superseded by I-D9-revised + I-D14 (01_PLAN:480-494, 514-519). |
| Review | R-011 | HIGH | Accepted | I-D16 + V-IT-8 + explicit bus-side mechanism (§Tick-Order Resolution). Option (b) of 01_REVIEW's two: PLIC's `Device::tick` is promoted out of the fold and runs after all other device ticks. |
| Review | R-012 | MEDIUM | Accepted | `cpu/mod.rs:357` added to Phase-3 call-site table (§Phase-3 Call-Site Table Delta). |
| Review | R-013 | MEDIUM | Accepted | V-IT-1 split into V-IT-1a (interleaving, exercises HB) + V-IT-1b (smoke, former form, demoted). |
| Review | R-014 | MEDIUM | Accepted | Reset outcome pinned to match real hardware — concurrent raise-during-reset is delivered (PLIC Gateway semantics §2). V-F-1 sharpened. |
| Review (TR-6) | OQ-6 | — | Closed | AtomicBool adopted; revisit only under OQ-5. |
| Review (TR-4) | T-4 | — | Closed | Accepted per 01_REVIEW TR-4; sticky-level hazard already documented via I-D4 + NG-3 + V-IT-6 / V-E-6. |

> All prior CRITICAL/HIGH findings and all MASTER directives
> resolved or explicitly honored. No rejections this round.

---

## Hardware-Semantic Grounding

(New section per M-002 / M-003.) Every design choice in 01_PLAN that
was previously justified on internal-consistency grounds is here
cross-referenced with an authoritative manual. Where the manual
specifies semantics, this plan matches it; where the manual leaves
behaviour implementation-defined, the plan explicitly notes it.

**Real-hardware premise (M-002):** a rising edge on an IRQ line
reaches the interrupt controller essentially instantaneously
(propagation delay is ns scale on-chip). In an emulator, the
64-bus-tick "delay" between a raise and the next PLIC drain is a
*scheduling artefact* — it reflects how often we run the slow-tick
fold, not a semantic property of the line. The plan therefore
targets the real-hardware semantics: "the PLIC Gateway sees the
raise *as soon as it can*, which in our scheduler means on the next
slow tick." I-D16 (below) is what makes "next slow tick" actually
true for same-cycle raises that originate from an adopter's own
`Device::tick` — not a tick later.

### H-1 RISC-V PLIC Gateway — level vs edge

Source: RISC-V PLIC Specification v1.0.0, §2 "Interrupt Gateways".
URL: https://github.com/riscv/riscv-plic-spec/blob/master/riscv-plic.adoc

Manual passages (verified via WebFetch, round 02):

- "Gateways convert interrupt signals into a standardized format
  for the PLIC core. At most one interrupt request per interrupt
  source can be pending in the PLIC core at any time, indicated by
  setting the source's IP bit."
- Level-sensitive: "the gateway converts the first assertion of the
  interrupt level into an interrupt request" then waits for
  completion. "If the level remains asserted after completion, a
  new interrupt request will be forwarded to the PLIC core."
- Edge-triggered: "the gateway converts the first matching signal
  edge into an interrupt request."

Design tie-back:

- `PlicSignals.level: AtomicU32` models the post-gateway level
  register: the bit stays asserted as long as the source holds it,
  independent of claim/complete. This matches the PLIC spec's
  "if the level remains asserted after completion, a new interrupt
  request will be forwarded." This is the same invariant
  `plicGateway` I-8 pins.
- `PlicSignals.edge_latch: AtomicU32` models the gateway's
  edge-latch — the "first matching signal edge" is captured and
  held until drained, so that a pulse whose level bit is lowered
  before the drainer reads cannot be lost (01_PLAN I-D4).
- `pulse()` writes level first then edge; the drain reads edge
  first then level (01_PLAN I-D15). Rationale: the edge is the
  commit-point ("an event happened") and the level is the
  current-state snapshot. Reversing would let a drainer observe
  `edge=1, level=0` for a pulse whose level was already lowered by
  a subsequent `lower`, misrepresenting the observed sequence.

### H-2 PLIC claim/complete — post-claim-clear vs real-silicon SiFive variant

Source: RISC-V PLIC Specification v1.0.0, §7 "Interrupt Claim
Process" + §8 "Interrupt Completion".
URL: https://github.com/riscv/riscv-plic-spec/blob/master/riscv-plic.adoc

Manual passages:

- §7: "A target reads the claim register, returning the highest-
  priority pending interrupt ID. Reading claim atomically clears
  the corresponding IP bit."
- §8: "After servicing, the target writes the same ID to the
  completion register. The gateway then permits the next request
  from that source."

Design tie-back:

- `plicGateway` I-8 ("level bit stays asserted until complete")
  matches the RISC-V PLIC spec's description of the level-sensitive
  gateway's post-completion behaviour. SiFive's U54/FU540
  implements this pre-claim-clear variant directly; `xemu` already
  matches per `plicGateway/02_PLAN.md`.
- `needs_reevaluate` flag (01_PLAN:642) encodes "the PLIC state
  changed and the next tick must re-run `core.evaluate`, even in
  the absence of a raise." This is exactly §8's "next request from
  that source" requirement translated into an event-driven
  scheduler: complete can drop a MEIP, so the next tick must
  re-run the gateway FSM for that source even if `pending_raises`
  says no new raise happened.

### H-3 Cross-ISA validation: ARM GIC

Sources:

- ARM GIC Architecture Specification (GICv3) —
  https://www.scs.stanford.edu/~zyedidia/docs/arm/gic_v3.pdf
- ARM GIC level-sensitive behaviour secondary note:
  https://www.systemonchips.com/arm-gic-interrupt-handling-edge-vs-level-trigger-mismatch-issues/

Manual passages (WebSearch verified):

- "For level-sensitive interrupts, the peripheral must de-assert
  the signal to clear the Pending state. If the signal remains
  asserted, the interrupt will re-enter the Pending state."
- "When a CPU acknowledges an IRQ by reading from GICC_IAR ... the
  Distributor changes the status of the IRQ from active to pending
  if it is a level-triggered IRQ and the device has deasserted the
  level on the line."

Relevance:

- The ARM GIC's level-sensitive contract is shape-identical to the
  RISC-V PLIC's: the controller keeps the "pending" state tied to
  the source's line and re-asserts if the line is still high after
  EOI / completion. This is the *universal* IRQ-controller
  contract, not a RISC-V idiosyncrasy. The `level` bit in
  `PlicSignals` matching this contract is therefore
  cross-architecture correct, not "happens to work for PLIC."

### H-4 Rust atomics: Release / Acquire pair

Sources:

- Rust Nomicon §8 "Atomics" —
  https://doc.rust-lang.org/nomicon/atomics.html
- `std::sync::atomic::Ordering` docs —
  https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html

Manual passages (WebFetch verified):

- Nomicon: "When thread A releases a location in memory and then
  thread B subsequently acquires the same location in memory,
  causality is established. Every write (including non-atomic and
  relaxed atomic writes) that happened before A's release will be
  observed by B after its acquisition."
- `Ordering::Release`: "all previous operations become ordered
  before any load of this value with Acquire (or stronger)
  ordering. In particular, all previous writes become visible to
  all threads that perform an Acquire (or stronger) load of this
  value."
- `Ordering::Acquire`: "if the loaded value was written by a store
  operation with Release (or stronger) ordering, then all
  subsequent operations become ordered after that store. In
  particular, all subsequent loads will see data written before
  the store."
- `Ordering::AcqRel`: "Has the effects of both Acquire and Release
  together."

Design tie-back to 01_PLAN §Concurrency Posture (01_PLAN:738-757):

- `raise`/`lower`/`pulse` release `pending_raises` as the final
  store (Release). `Plic::tick` acquires `pending_raises` via a
  `swap` (Acquire). The Nomicon contract guarantees: any prior
  `level.fetch_or`, `edge_latch.fetch_or`, or `level.fetch_and` on
  the raise side is observed by the drain side.
- `edge_latch.swap(0, AcqRel)` on the drain has the AcqRel
  semantics spelled out by the Ordering docs. The Acquire half
  synchronizes with prior `pulse`-side Release stores on
  `edge_latch`; the Release half publishes the zero to any
  subsequent observer. The `level.load(Acquire)` that follows
  synchronizes with prior Release stores on `level`.
- Consequence: the HB chain
  `raise -> pending_raises(Release) ~~> pending_raises(Acquire-swap)
  -> drain reads level/edge_latch` is valid per the Nomicon.
  I-D9-revised is a direct application of the documented
  Release/Acquire pair contract, not a Rust-specific invention.

### H-5 Emulator prior art: QEMU `qemu_set_irq`

Sources:

- QEMU `include/hw/irq.h` —
  https://github.com/qemu/qemu/blob/master/include/hw/irq.h
- airbus-seclab QEMU-internals blog —
  https://airbus-seclab.github.io/qemu_blog/interrupts.html

Relevance:

- QEMU's `qemu_set_irq(irq, level)` is a synchronous function call
  from the device model into the interrupt-controller handler, on
  the same thread. It matches the real-hardware "instant
  propagation" contract. Our posture P-A is the threaded-emulator
  analogue: the raise is a cross-thread atomic store; the next
  tick's Acquire swap is the synchronization point. We do not
  replicate QEMU's callback model because our bus tick is the
  scheduler; we want the PLIC to pull, not push, so that the drain
  cost is zero when nothing happened (I-D14, G-7).

### Summary of the tie-back

Every design choice in 01_PLAN that touches timing or ordering has a
manual citation above. No architectural decision rests on
"implementation convenience" alone; each one matches either the
RISC-V PLIC spec, the Rust atomics contract, or universal industry
practice (ARM GIC). M-002 and M-003 are satisfied.

---

## Tick-Order Resolution (R-011)

### I-D16 (NEW, R-011) — PLIC-last slow-tick ordering

Statement:

> Within any single slow-tick pass of `Bus::tick`, `Plic`'s
> `Device::tick` runs *strictly after* every other device's
> `Device::tick` in that pass. `Bus::tick` achieves this by
> excluding the PLIC slot from the slow-tick iteration and
> explicitly invoking `self.mmio[plic_idx].dev.tick()` after the
> iteration completes.

Mechanism (option (b) of 01_REVIEW R-011's two):

- `Bus::tick` is restructured at 01_PLAN's Phase 1 step 7 (replaces
  01_PLAN:944-949 "Bus::tick unchanged" note):

  ```rust
  pub fn tick(&mut self) {
      if let Some(i) = self.mtimer_idx {
          self.mmio[i].dev.tick();
      }
      self.tick_count += 1;
      if !self.tick_count.is_multiple_of(SLOW_TICK_DIVISOR) {
          return;
      }
      // Slow pass 1: every non-MTIMER, non-PLIC device ticks.
      // [Phase 1 only] simultaneously collect the legacy bitmap
      // from non-adopter devices (adopter devices return false).
      let irq_lines = self.mmio.iter_mut().enumerate().fold(
          0u32,
          |lines, (idx, r)| {
              if Some(idx) == self.mtimer_idx || Some(idx) == self.plic_idx {
                  return lines;
              }
              r.dev.tick();
              if r.irq_source > 0 && r.dev.irq_line() {
                  lines | (1u32 << r.irq_source)
              } else {
                  lines
              }
          },
      );
      // Slow pass 2: PLIC — consumes legacy bitmap via notify,
      //              then drains its signal plane via tick.
      if let Some(i) = self.plic_idx {
          // Phase 1 only:
          self.mmio[i].dev.notify(irq_lines);
          // Always:
          self.mmio[i].dev.tick();
      }
  }
  ```

- `Plic::notify(bitmap)` still sets `needs_reevaluate = true` as
  in 01_PLAN:703-710; the subsequent explicit `tick()` call
  passes the `!needs_reevaluate` guard and drains, evaluating
  both the gateway decisions `notify` staged and the signal-plane
  drain in one pass.
- Phase 2 deletes the `notify` line; the explicit `plic.tick()`
  remains.
- Phase 3 deletes `notify` entirely; the explicit `plic.tick()`
  and `plic_idx` tracking remain.

Why option (b) over option (a):

- Option (a) required splitting `Bus::tick` into "device tick" and
  "plic tick" phases with an explicit plic_idx exclusion from the
  first fold. Option (b) achieves the same effect with a single
  fold that excludes plic_idx + mtimer_idx, then an explicit plic
  call. Code delta is one extra exclusion check in the fold
  closure and one line outside it. This matches the existing
  mtimer_idx exclusion pattern at `bus.rs:219-221`, so Bus code
  stays symmetric.
- Eliminates registration-order dependence: the guest-observable
  ordering is now independent of the order in which devices were
  added via `add_mmio`.

Cost analysis:

- One extra `Option` comparison per slow-tick fold iteration:
  negligible, branch-predictable.
- One extra virtual call per slow tick (the explicit
  `plic.tick()`). In Phase 1/2 this is zero net — `notify` was
  already called explicitly and `plic.tick()` replaces the
  implicit fold-path `tick`. In Phase 3, one explicit call
  instead of one fold-path call — net zero.

### V-IT-8 (NEW, R-011) — same-tick adopter raise

Test structure:

```rust
struct RaisingDevice { line: IrqLine }
impl Device for RaisingDevice {
    fn tick(&mut self) { self.line.raise(); }
    fn irq_line(&self) -> bool { false }   // I-D11-rev
    // ... other Device default impls
}

#[test]
fn plic_drains_same_cycle_adopter_raise() {
    let mut bus = Bus::new(...);
    let plic = Plic::new(2, irqs.clone());
    let line = plic.with_irq_line(3);
    let plic_idx = bus.add_mmio("plic", ..., Box::new(plic), 0);
    bus.set_irq_sink(plic_idx);
    bus.add_mmio("raiser", ..., Box::new(RaisingDevice { line }), 0);

    // Configure PLIC so source 3 passes the gateway.
    // ... enable + priority + threshold writes ...

    // Cross SLOW_TICK_DIVISOR boundary with exactly one slow pass.
    for _ in 0..SLOW_TICK_DIVISOR { bus.tick(); }

    // R-011 claim: MEIP asserted after exactly one slow pass,
    // not after a second one.
    assert!(bus.irq_state().meip(ctx0));
}
```

Negative control: a test variant that registers the raiser *before*
PLIC (`add_mmio` order reversed) asserts the same outcome — proves
the fix is not registration-order-dependent.

### Tie-back to G-2 / G-7

- G-2 "any-thread raise observed at the next bus-tick boundary" —
  I-D16 ensures "next bus-tick boundary" is literal for same-cycle
  raises; no hidden `+1 slow tick` from registration order.
- G-7 "event-driven PLIC" — unaffected. The no-raise fast path is
  still one `Acquire` swap + early return.

---

## Phase-3 Call-Site Table Delta (R-012)

Augments the table in 01_PLAN:999-1010. All other rows are
unchanged.

| File | Line | Call |
|---|---|---|
| `xemu/xcore/src/cpu/mod.rs` | 357 | `let plic_idx = bus.add_mmio("plic", 0x0C00_0000, 0x400_0000, Box::new(Plic::new(num_harts, irqs.clone())), 0);` |

Verification: after the Phase-3 diff, `set_irq_sink(plic_idx)` at
`cpu/mod.rs:364` remains unchanged — `set_irq_sink` takes only the
mmio index, not a source-id arg. This closes the review loop on
R-012.

UART site at `cpu/mod.rs:365` (`bus.add_mmio("uart0", 0x1000_0000,
0x100, Box::new(Uart::new()), 10)`) is already covered by the
`xemu/src/machine/*.rs` wildcard row in 01_PLAN:1009. Noted here
explicitly so the Phase-3 implementer does not miss it: the `10`
literal source id is the argument being dropped in Phase 3. In
Phase 1, UART's constructor changes to
`Uart::new_stdio_with_irq(line)` (01_PLAN step 5), and the `10`
literal is retained through Phase 2, then dropped with the
parameter in Phase 3.

---

## V-IT-1 Restructure (R-013)

Replaces 01_PLAN:1102-1105. Split into two variants.

### V-IT-1a — cross-thread interleaving (exercises HB)

Purpose: exercise the `Release`/`Acquire` pair on `pending_raises`
directly, without a join barrier that would trivially make the
test pass under Relaxed.

```rust
#[test]
fn v_it_1a_cross_thread_interleaved_raise_and_tick() {
    const N: usize = 10_000;
    let plic = Arc::new(std::sync::Mutex::new(
        Plic::new(1, irqs_of_1())));
    let line = plic.lock().unwrap().with_irq_line(2);
    enable_ctx_for_src_2(&mut *plic.lock().unwrap());

    let raiser = {
        let line = line.clone();
        std::thread::spawn(move || {
            for _ in 0..N {
                line.raise();
                line.lower();
            }
        })
    };

    // Main thread ticks concurrently.
    let mut ticks_observing_meip = 0u32;
    for _ in 0..N {
        let mut p = plic.lock().unwrap();
        <Plic as Device>::tick(&mut *p);
        if p.irq_state_for(ctx0).meip_snapshot() {
            ticks_observing_meip += 1;
        }
    }
    raiser.join().unwrap();
    // Final tick after raiser terminated drains any remainder.
    {
        let mut p = plic.lock().unwrap();
        <Plic as Device>::tick(&mut *p);
    }

    // HB property: at least one tick observed MEIP concurrently.
    // (If Release/Acquire were Relaxed, this could fail on a
    //  weakly-ordered target such as aarch64.)
    assert!(ticks_observing_meip >= 1);
    // Final-state property: no bit stuck.
    assert_eq!(plic.lock().unwrap().signals_level_for_test(), 0);
}
```

Notes:

- `std::sync::Mutex` is used to serialize *access to the Plic
  struct* (the drain mutates `&mut Plic`). The atomics inside
  `PlicSignals` are still lock-free; the lock is a pragmatic
  choice because `Plic::tick` takes `&mut self`. Neither
  `std::sync::Mutex` nor `parking_lot::Mutex` violate C-13 — C-13
  forbids async-runtime / channel crates, not sync primitives.
- On x86 TSO, a buggy `Relaxed` ordering may be invisible; on
  aarch64 CI a buggy ordering would manifest as a missed MEIP
  observation or a nonzero final `level` (because a `lower` was
  not ordered after its matching `raise`). This is the HB witness
  R-013 asked for.

### V-IT-1b — smoke test (former V-IT-1, demoted)

```rust
#[test]
fn v_it_1b_cross_thread_raise_then_join_then_tick() {
    // Demoted: full join barrier makes this pass under any ordering.
    // Kept as a compile/link smoke test only.
    let plic = Plic::new(1, irqs_of_1());
    let line = plic.with_irq_line(2);
    let t = std::thread::spawn(move || line.raise());
    t.join().unwrap();
    let mut plic = plic;   // move back
    <Plic as Device>::tick(&mut plic);
    assert!(/* MEIP asserted per normal path */);
}
```

Classification: V-IT-1b no longer appears in the Acceptance
Mapping's "I-D9-revised" row. I-D9-revised is validated by
V-IT-1a + V-E-4 + §Concurrency Posture (01_PLAN:738-757) +
deferred OQ-5 (loom).

---

## Reset-Race Outcome (R-014)

Pins the race semantics to match real hardware (M-002).

### Choice: raise-during-reset is delivered

Rationale (cross-referenced to H-1 / H-3):

- Real PLIC gateways continue to observe the source line during any
  software state change. The RISC-V PLIC spec §2 describes the
  gateway as transforming the *source's* assertion state; there
  is no "reset ignores concurrent sources" clause. If the source
  drives the line high during a reset, the gateway sees that line
  high.
- ARM GIC v3 follows the same pattern: peripheral-driven
  level-sensitive signals re-enter Pending after EOI if still
  asserted. The controller's reset handlers clear *controller*
  state, not *source* state.
- Implementation convenience was the argument *against* this
  posture in 01_PLAN F-6. The hardware-grounded argument flips the
  conclusion: preferring the "raise-during-reset is delivered"
  outcome matches silicon, so this is the correct semantics, not
  a concession.

Behavioural statement (appended to 01_PLAN F-6):

> F-6 (REFINED) — `PlicSignals::reset` serializes with the bus-tick
> thread only. It clears `level` and `edge_latch` in place, then
> stores `pending_raises = true` (forcing the next tick to drain).
> Raises from other threads interleaved with `reset` may set
> `level`/`edge_latch` bits after the reset's Release stores. Those
> bits are preserved and observed by the next drain. Guest-visible
> outcome: a raise that happens concurrently with reset surfaces
> as an interrupt assertion on the next bus-tick boundary after
> reset. This matches real PLIC / GIC silicon where the controller
> does not suppress peripheral signalling during its own state
> changes.

Consequence:

- For the in-tree adopter set (UART, VirtioBlk), the outcome is
  harmless: the guest re-enables interrupts and re-arms the
  handler after the reset-inducing MMIO write, so the delivered
  post-reset raise is serviced normally.
- For a hypothetical future device that treats "raise = consume
  one queued event" with side effects, the semantics are
  documented — the device will need to coordinate event-consumption
  with its own reset path, not rely on the PLIC to drop the event.
  Noted for any future edge adopter under T-4 / NG-3.

### V-F-1 (SHARPENED, R-014)

Replaces 01_PLAN:1130-1132.

```rust
#[test]
fn v_f_1_raise_during_reset_is_delivered() {
    let plic = Arc::new(std::sync::Mutex::new(Plic::new(1, ...)));
    let line = plic.lock().unwrap().with_irq_line(2);
    enable_src_2(&mut *plic.lock().unwrap());

    // Thread B: spin-raising.
    let stop = Arc::new(AtomicBool::new(false));
    let raiser = {
        let line = line.clone();
        let stop = stop.clone();
        std::thread::spawn(move || {
            while !stop.load(Ordering::Relaxed) {
                line.raise();
            }
        })
    };

    // Thread A: reset while raiser is active.
    std::thread::sleep(Duration::from_millis(1));   // let raiser spin
    {
        let mut p = plic.lock().unwrap();
        <Plic as Device>::reset(&mut *p);
    }

    // Keep raising for a moment post-reset.
    std::thread::sleep(Duration::from_millis(1));
    stop.store(true, Ordering::Relaxed);
    raiser.join().unwrap();

    // Drain.
    {
        let mut p = plic.lock().unwrap();
        <Plic as Device>::tick(&mut *p);
        // Positive property: a post-reset raise is delivered.
        assert!(p.irq_state_for(ctx0).meip_snapshot(),
                "post-reset raise must surface");
    }

    // Arc-identity property (I-D8a).
    assert!(line_still_routes_to(&*plic.lock().unwrap(), &line));
}
```

Two assertions:

1. **Delivery**: post-reset MEIP is asserted. This is the new
   positive property (replacing the former vague "some raise is
   observed").
2. **Identity** (already in I-D8a / V-UT-10): `line` still routes
   to `plic.signals`. Reset did not swap the Arc.

---

## Invariants Delta

Delta only; I-D1..I-D15 unchanged (01_PLAN:458-529).

- **I-D16 (NEW)** — PLIC-last slow-tick ordering. See §Tick-Order
  Resolution. Statement: "Within any single slow-tick pass of
  `Bus::tick`, `Plic::tick` runs strictly after every other device's
  `Device::tick`. `Bus::tick` achieves this by excluding `plic_idx`
  from the slow-tick iteration and explicitly invoking
  `self.mmio[plic_idx].dev.tick()` after the iteration."
  Enforced by V-IT-8 plus the bus-side code diff in Phase 1 step 7.

No other invariants are added, changed, or removed.

---

## Validation Delta

Delta only; all other validation items in 01_PLAN:1058-1168 stand
unchanged.

[**Unit Tests**] — unchanged.

[**Integration Tests**]

- V-IT-1 replaced by V-IT-1a + V-IT-1b (§V-IT-1 Restructure).
- **V-IT-8 (NEW, R-011)** — same-tick adopter raise: a device
  whose `Device::tick` calls `line.raise()` has MEIP asserted
  within one slow-pass. See §Tick-Order Resolution for structure.
  Covers I-D16 and the G-2 "one slow tick" latency bound for
  adopter raises.

[**Failure / Robustness Validation**]

- V-F-1 sharpened (§Reset-Race Outcome). Replaces 01_PLAN:1130-1132.

[**Edge Case Validation**] — unchanged.

### Acceptance Mapping Delta

Augments 01_PLAN:1172-1204.

| Goal / Constraint | Validation |
|---|---|
| G-2 (any-thread raise) | V-IT-1a, V-IT-3, V-F-1, V-IT-6, **V-IT-8** (adopter-originated raise within one tick) |
| I-D9-revised (orderings) | V-IT-1a, V-E-4, §Concurrency Posture |
| **I-D16 (PLIC-last order)** | **V-IT-8** + bus-side code inspection |

V-IT-1b is not listed — it is the demoted smoke variant and does
not carry any acceptance claim.

---

## Gates

Test count delta relative to 01_PLAN Gates:

- Phase 1 gate: `cargo test -p xcore` >= baseline 374 + **12 new**
  (was +11 in 01_PLAN; +1 for V-IT-8). V-IT-1a replaces V-IT-1
  one-for-one; V-IT-1b is a demoted smoke variant not counted
  against the +12 budget but still compiles and runs. Boot trio
  (xv6, linux-2hart, debian-2hart) green with `DEBUG=n`. C-2 diff
  empty. C-13 `Cargo.toml` diff empty.
- Phase 2 gate: Phase-1 gate still passes; `cargo test -p xcore`
  >= Phase-1 + 4. Bitmap fold deleted from `Bus::tick` but the
  explicit `plic.tick()` call (per I-D16) remains. Boot trio green.
- Phase 3 gate: Phase-2 gate still passes; `fn irq_line` and
  `fn notify` gone from `Device` trait; `MmioRegion::irq_source`
  and `Bus::add_mmio`'s `irq_source` parameter gone; `cargo clippy`
  / `cargo fmt --all` clean; boot trio green; `arch_isolation`
  pins unchanged. The explicit `plic.tick()` call-site (§I-D16)
  survives Phase 3 and becomes the sole PLIC entry.
- At every gate: user commits (project memory
  `feedback_user_commits`).

---

## Risks (Delta)

Adds to 01_PLAN:1210-1236.

- **Risk 8 (NEW)** — a future refactor of `Bus::tick` re-introduces
  the PLIC into the slow-tick fold, silently reverting I-D16.
  Mitigation: V-IT-8 includes the registration-order-reversed
  negative control; any such refactor would fail that test.
- **Risk 9 (NEW)** — V-IT-1a passes on x86 CI (TSO hides Relaxed
  bugs) but should also run on aarch64. Mitigation: if xemu's CI
  adds aarch64 runners, V-IT-1a should be exercised there. For
  now, the test exercises the path; OQ-5 (`loom`) remains the
  model-checker escalation.

---

## Open Questions

Unchanged from 01_PLAN with closures:

- OQ-1 — yes, `IrqLine: Clone`.
- OQ-2 — superseded.
- OQ-3 — superseded.
- OQ-4 — resolved.
- OQ-5 — `loom` deferred; not required for correctness in light of
  §Hardware-Semantic Grounding H-4. Re-open only if V-E-4 or
  V-IT-1a flakes under stress on weakly-ordered targets.
- OQ-6 — **CLOSED** per TR-6 (01_REVIEW). `AtomicBool` adopted;
  promote to `AtomicU32` only if OQ-5 `loom` validation is ever
  taken up and raise-count diagnostics become necessary.

---

## Citations (URLs, one place)

- RISC-V PLIC Specification v1.0.0 §2, §7, §8 —
  https://github.com/riscv/riscv-plic-spec/blob/master/riscv-plic.adoc
- Rust Nomicon §8 Atomics —
  https://doc.rust-lang.org/nomicon/atomics.html
- Rust `std::sync::atomic::Ordering` —
  https://doc.rust-lang.org/std/sync/atomic/enum.Ordering.html
- ARM GIC Architecture Specification (GICv3) —
  https://www.scs.stanford.edu/~zyedidia/docs/arm/gic_v3.pdf
- ARM GIC level-sensitive behaviour (secondary) —
  https://www.systemonchips.com/arm-gic-interrupt-handling-edge-vs-level-trigger-mismatch-issues/
- QEMU `include/hw/irq.h` —
  https://github.com/qemu/qemu/blob/master/include/hw/irq.h
- QEMU-internals interrupt post (airbus-seclab) —
  https://airbus-seclab.github.io/qemu_blog/interrupts.html
- Rust async-book §2 (carried from 01_PLAN) —
  https://rust-lang.github.io/async-book/02_execution/04_executor.html
- phil-opp "Async/Await" (carried from 01_PLAN) —
  https://os.phil-opp.com/async-await/
- without.boats "The Waker API I" (carried from 01_PLAN) —
  https://without.boats/blog/wakers-i/
