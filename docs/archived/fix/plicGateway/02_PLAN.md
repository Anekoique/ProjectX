# `plicGateway` PLAN `02`

> Status: Revised
> Feature: `plicGateway`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `none`

---

## Summary

Round 02 is a **tightening pass** on `01_PLAN.md` in response to
`01_REVIEW.md`. Verdict was Approved with Revisions, 0 blocking, 4
non-blocking findings (R-012..R-015). All remaining content of
`01_PLAN.md` — Spec, Goals, Non-Goals, Architecture, Invariants
I-1..I-11, Data Structure, API Surface, Constraints C-1..C-11,
Implementation Plan phase scope, Gates, Trade-offs T-1..T-5, Risks — is
unchanged and cited by reference rather than restated. This round
disambiguates the Edge FSM transition table, pins the Core↔Gateway
wiring to one concrete design, adds a Plic-boundary edge integration
test V-E-7, and operationalises the V-IT-1 byte-identical allowlist
gate into a `git diff` assertion. No scope change.

## Log

[**Feature Introduce**]

No new feature work. This round only refines specification text in
`01_PLAN.md` so the executor can implement without reconstructing
intent from multiple artifacts.

[**Review Adjustments**]

- R-012 (MEDIUM) — Edge FSM wildcard `sample(false)` row replaced with
  explicit row that preserves `armed`. See §Revised State Transition.
- R-013 (MEDIUM) — Core↔Gateway callback wiring pinned to design (a)
  from `01_REVIEW.md:137-144`: `Core` is gateway-agnostic; `Plic::read`
  and `Plic::write` orchestrate the claim/complete callbacks. See
  §Revised Core↔Gateway Wiring.
- R-014 (LOW) — New validation item V-E-7 drives the edge path through
  the `Plic` MMIO boundary (not just the `Gateway` struct). See
  §Revised Validation.
- R-015 (LOW) — V-IT-1 now requires
  `git diff main -- xemu/xcore/tests/arch_isolation.rs` empty at each
  phase gate. See §Revised Validation.

[**Master Compliance**]

No new MASTER directives in this round. Inherited directives from
`archModule 00-M-002` and `archLayout 01-M-004` remain Honored per the
Response Matrix of `01_PLAN.md` (unchanged).

### Changes from Previous Round

[**Added**]
- V-E-7 — Plic-boundary edge integration test (Phase 2).
- `git diff` operational mechanism for V-IT-1.
- Pinned Core↔Gateway wiring paragraph under Execution Flow.
- Explicit Edge FSM row covering `sample(false)` with `armed`
  preservation.

[**Changed**]
- Edge FSM table: wildcard `sample(false)` row (previously
  `01_PLAN.md:595`) replaced with an explicit row stating `armed=a`
  preserved. Why: R-012 — an ambiguous spec forces the executor to
  re-derive intent from test names, violating AGENTS.md self-contained
  PLAN requirement.
- V-IT-1 text: adds a diff gate on
  `xemu/xcore/tests/arch_isolation.rs`. Why: R-015 — C-3 demands
  byte-identical, but `cargo test arch_isolation` passes trivially if
  the executor adds a new allowed symbol alongside a new leaking
  symbol.
- Phase 1 step 4 / Execution Flow §5-6: the wiring paragraph is
  promoted from implicit to explicit. Why: R-013 — three wiring
  designs were compatible with the existing API surface.
- Phase 2 Gate arithmetic: "≥ 5 new Phase-2 tests" becomes "≥ 6" and
  total new becomes ≥ 20 to account for V-E-7.

[**Removed**]
- Nothing removed. This is strictly a tightening pass.

[**Unresolved**]
- Open Q 2 from `01_PLAN.md:914-918` (whether `directIrq` should land
  before any in-tree device adopts edge) stays open. Same rationale:
  out of scope for this feature.

### Response Matrix

| Source | ID    | Decision | Resolution |
|--------|-------|----------|------------|
| Review | R-012 | Accepted | Edge FSM row replaced; explicit `armed` preservation on `sample(false)`. See §Revised State Transition. |
| Review | R-013 | Accepted | Wiring pinned to `Core`-agnostic design (a). See §Revised Core↔Gateway Wiring. |
| Review | R-014 | Accepted | V-E-7 added. See §Revised Validation. |
| Review | R-015 | Accepted | V-IT-1 tightened with `git diff` gate. See §Revised Validation. |
| Review | TR-1..TR-5 | Concurred | No action required; concurrence already recorded in `01_PLAN.md` Trade-off section. |

> Rules:
> - R-012..R-015 were MEDIUM/LOW, not HIGH/CRITICAL; listed here for
>   traceability.
> - No prior HIGH/CRITICAL findings outstanding (round 00 findings
>   R-001..R-004 were resolved in `01_PLAN.md`).
> - No MASTER directives introduced in this round.

---

## Spec

All Spec content — Goals G-1..G-4, Non-Goals NG-1..NG-6, Architecture,
Invariants I-1..I-11, Data Structure, API Surface, Constraints
C-1..C-11 — is unchanged from `01_PLAN.md:190-478`. Reference those
sections directly; re-stating them here would add surface for drift.

One citation convenience:
- I-8 (SiFive variant clear-on-low pre-claim), I-9 (tick-thread-only),
  I-10 (evaluation-in-notify), I-11 (reset preserves `kind`) are the
  invariants most relevant to this round's changes. None moves.

---

## Implement

### Revised Execution Flow

Main Flow and Failure Flow in `01_PLAN.md:523-574` are retained
verbatim. Only §5-6 of Main Flow gains the wiring paragraph below.

#### Revised Core↔Gateway Wiring (addresses R-013)

Pin one concrete design. Rationale under Trade-offs below.

**Design (a) — `Core` is gateway-agnostic; `Plic` orchestrates.**

- `Core::claim(&mut self, ctx) -> u32` takes `ctx` only. It selects
  the highest-priority enabled source above threshold, clears its
  pending bit, and records `claimed[ctx]`. It does **not** touch any
  gateway field.
- `Core::complete(&mut self, ctx, src) -> bool` returns `true` iff
  `claimed[ctx] == src`, clearing `claimed[ctx]` on success. It does
  **not** touch any gateway field.
- `Core::set_pending(&mut self, s)` and `Core::evaluate(&mut self)`
  are the only mutators gateways can poke indirectly — and only
  through `Plic`, never directly from within `Gateway`.
- `Plic::read` on the claim register:
  1. `let s = self.core.claim(ctx);`
  2. `if s != 0 { self.gateways[s as usize].on_claim(); }`
  3. return `s`.
- `Plic::write` on the complete register:
  1. `if !self.core.complete(ctx, src) { return; }` — bail early on
     mismatch, preserving the current
     `complete_wrong_source_no_change` contract.
  2. `match self.gateways[src as usize].on_complete() { Pend => { self.core.set_pending(src); self.core.evaluate(); } NoChange => {} }`
  3. The re-pend-on-complete path runs to completion **inside the
     same MMIO `write` stack frame**, so the existing test
     `source_repended_after_complete` (`plic.rs:269-278` pre-split)
     observes identical timing.

**Why (a) and not (b) or (c)** (from `01_REVIEW.md:137-144`):

- (b) — `Core::claim(&mut self, ctx, &mut [Gateway; NUM_SRC])` —
  couples `Core` to gateway array width and gateway type, defeating
  the single-responsibility premise of the split.
- (c) — `Core` owns the gateway array — makes `Plic` a thin MMIO
  facade but puts FSM concerns in the arbitrator module. The C-11
  250-line soft cap would push back on this immediately.
- (a) keeps `Core` testable standalone with pure integer inputs, and
  keeps `Gateway` testable standalone via V-UT-G1 and V-UT-G2.

This design is already implied by the Data Structure diagram at
`01_PLAN.md:376-447` where `Plic { gateways: [Gateway; NUM_SRC], core: Core }`
and `Core::claim(&mut self, ctx) -> u32`. Pinning it as design (a)
closes R-013.

#### Revised State Transition (addresses R-012)

The Level FSM at `01_PLAN.md:580-588` is unchanged and re-listed here
only so both tables show the `armed` column explicitly side-by-side:

**Level FSM (unchanged, re-listed for parity)**

| From                                 | Event         | To                                                     | Emit                                        |
|--------------------------------------|---------------|--------------------------------------------------------|---------------------------------------------|
| `(armed=false, in_flight=false)`     | `sample(true)` | `(armed=true, in_flight=false)`                        | `Pend`                                      |
| `(armed=true, in_flight=false)`      | `sample(false)` | `(armed=false, in_flight=false)`                      | `Clear` (I-8 SiFive variant)                |
| `(armed=true, in_flight=false)`      | `sample(true)` | unchanged                                              | `NoChange`                                  |
| `(armed=a, in_flight=false)`         | `on_claim`    | `(armed=a, in_flight=true)`                            | —                                           |
| `(armed=a, in_flight=true)`          | `sample(level)` | `(armed=(a \| level), in_flight=true)`               | `NoChange` (held during in-flight)          |
| `(armed=true, in_flight=true)`       | `on_complete` | `(armed=armed', in_flight=false)` where `armed'=latest level` | `Pend` if `armed'` else `NoChange`   |
| `(armed=false, in_flight=true)`      | `on_complete` | `(armed=false, in_flight=false)`                       | `NoChange`                                  |

**Revised Edge FSM for `SourceKind::Edge`** — row 2 is now explicit
about `armed` preservation (delta vs `01_PLAN.md:592-600` marked with
`←`):

| From                                               | Event          | To                                                 | Emit                  |
|----------------------------------------------------|----------------|----------------------------------------------------|-----------------------|
| `(armed=false, in_flight=false, prev_level=false)` | `sample(true)` | `(armed=true, in_flight=false, prev_level=true)`   | `Pend`                |
| `(armed=a, in_flight=f, prev_level=*)`             | `sample(false)` | `(armed=a, in_flight=f, prev_level=false)` ←     | `NoChange`            |
| `(armed=true, in_flight=false, prev_level=true)`   | `sample(true)` | unchanged                                          | `NoChange` (coalesce) |
| `(armed=false, in_flight=true, prev_level=false)`  | `sample(true)` | `(armed=true, in_flight=true, prev_level=true)`    | `NoChange` (latch)    |
| `(armed=a, in_flight=false, prev_level=p)`         | `on_claim`     | `(armed=false, in_flight=true, prev_level=p)`      | —                     |
| `(armed=true, in_flight=true, prev_level=p)`       | `on_complete`  | `(armed=true, in_flight=false, prev_level=p)`      | `Pend`                |
| `(armed=false, in_flight=true, prev_level=p)`      | `on_complete`  | `(armed=false, in_flight=false, prev_level=p)`     | `NoChange`            |

Prose clarifier (normative): **`sample(false)` never clears `armed`.
It only lowers `prev_level`.** A rising edge latched during in-flight
therefore survives subsequent level drops, so `on_complete` still
emits `Pend` even if the line has since fallen. This is the exact
coalesce semantics I-3 promises and V-UT-7 / V-UT-12 exercise.

Implementation note for the executor: the Level FSM's
`(armed=a, in_flight=true)` row already preserves `armed` across level
drops during in-flight (Level latches on sample). The Edge FSM
equivalent had been written as a wildcard row and is now pinned
explicit. No Level-FSM change.

### Revised Implementation Plan

Phase structure, step order, and phase gate arithmetic from
`01_PLAN.md:602-693` are unchanged **except** for the two items
called out below. Phase 1 scope, Phase 1 gate counts, and Phase 2
scope are otherwise identical.

**Phase 1 step 4 (`01_PLAN.md:626-642`) — addition:** the wiring
paragraph above is the authoritative callsite description. The
placeholder pseudo-code for `Plic::notify` in that step stays; add a
second pseudo-code block for `Plic::read` / `Plic::write` that mirrors
the "Design (a)" numbered steps above.

**Phase 2 Gate (`01_PLAN.md:683-692`) — arithmetic delta:**

- `make test`: Phase-1 gate set + **new Phase-2 tests ≥ 6**
  (was ≥ 5): V-UT-6, V-UT-7, V-UT-12, V-UT-G2, V-E-6, **V-E-7**.
  Total new tests at end of Phase 2 ≥ **20** (was ≥ 19).
- All other Phase 2 gate items unchanged.

---

## Trade-offs

All trade-offs T-1..T-5 from `01_PLAN.md:696-758` stand as-is.
`01_REVIEW.md` concurred with the plan's position on each (TR-1..TR-5);
no re-discussion needed.

One new implicit trade-off recorded for completeness:

- **T-6 (new, resolved): Core↔Gateway wiring.** Options (a)/(b)/(c)
  listed in `01_REVIEW.md:137-144`. **Chosen: (a)** — `Core`
  gateway-agnostic; `Plic::read`/`Plic::write` orchestrate callbacks.
  (b) rejected: couples `Core` to `Gateway` type and array width.
  (c) rejected: bloats `core.rs` past the C-11 soft cap and mixes
  FSM concerns into the arbitrator. (a) keeps each module
  independently testable and matches the Data Structure at
  `01_PLAN.md:376-447`.

---

## Validation

All validation items V-UT-1..V-UT-12, V-UT-G1, V-UT-G2, V-IT-2..V-IT-7,
V-F-1..V-F-5, V-E-1..V-E-6 from `01_PLAN.md:730-841` stand unchanged.
Only V-IT-1 is tightened, and V-E-7 is added.

### Revised V-IT-1 (addresses R-015)

**V-IT-1** `arch_isolation` allowlist byte-identical across the
feature branch. Two operational gates, both must pass:

1. `cargo test -p xcore --test arch_isolation` green (same as before).
2. `git diff main -- xemu/xcore/tests/arch_isolation.rs` produces
   **empty output** at both Phase-1 completion and Phase-2
   completion. This catches the "add a leaking symbol + add it to
   the allowlist" loophole flagged in R-015 / aclintSplit R-002.

Rationale: C-3 demands byte-identical. The isolation test alone
passes trivially under that loophole. The `git diff` gate is
zero-code and CI-checkable.

### New V-E-7 (addresses R-014)

**V-E-7** `plic_with_config_edge_source_mmio_roundtrip` — Phase 2
Plic-boundary integration test.

- Construct `Plic::with_config` on `num_harts=1`, with source 5
  configured as `SourceKind::Edge` and all other sources
  `SourceKind::Level`. Wire context 0 to source 5 via the existing
  MMIO register setters: `priority[5]=1`, `enable[0,5]=true`,
  `threshold[0]=0`.
- Drive the following sequence at the `Plic::notify` / MMIO
  boundary, with claim-reads and complete-writes interleaved per
  the pinned arrival order below (this makes the test
  deterministic):
  1. `plic.notify(0x20)` — source 5 rising edge. Edge FSM row 1
     fires: `(armed=true, prev_level=true)`, emit `Pend`.
     `Core::evaluate` raises MEIP for ctx 0.
  2. Guest reads claim register for ctx 0 → expect `5`.
     `Plic::read` drives `Core::claim(0)` then
     `gateways[5].on_claim()`: `(armed=false, in_flight=true)`.
  3. `plic.notify(0x00)` — Edge FSM row 2 fires:
     `(armed=false, in_flight=true, prev_level=false)`,
     `NoChange`.
  4. `plic.notify(0x20)` — Edge FSM row 4 fires:
     `(armed=true, in_flight=true, prev_level=true)`,
     `NoChange` (latch).
  5. Guest writes `5` to complete for ctx 0 → `Plic::write`
     drives `Core::complete(0, 5)=true`, then
     `gateways[5].on_complete() = Pend` (Edge FSM row 6). Inside
     the same write call: `Core::set_pending(5)` and
     `Core::evaluate()` re-raise MEIP.
  6. Guest reads claim register for ctx 0 → expect `5` again.
  7. Guest writes `5` to complete. No further pending.
  8. Guest reads claim register → expect `0` (no pending).
- Assertions: two distinct claim-complete cycles land on source 5
  via the MMIO boundary end-to-end; no spurious claims on other
  sources; `gateways[5].in_flight == false` after step 7.
- The test lives in `plic/mod.rs#[cfg(test)]` alongside the other
  `Plic`-level tests, not in `plic/gateway.rs` unit-tests, because
  it exercises `Plic::with_config` + `Core` + `Gateway`
  integration.

Why this closes R-014: every other Phase-2 test exercises only the
`Gateway` struct. V-E-6 exercises `with_config` construction but does
not drive a claim/complete cycle. V-E-7 is the first in-tree witness
that `Plic::with_config → notify(edge_bit) → read(claim) → write(complete)`
wiring is correct end-to-end, without needing `directIrq` to land.

### Acceptance Mapping (delta)

| Goal / Constraint | Validation |
|-------------------|------------|
| G-2 (edge config) | V-UT-6, V-UT-7, V-UT-12, V-UT-G2, V-E-6, **V-E-7**, V-F-5 |
| C-3               | V-IT-1 (incl. `git diff` gate) |

All other acceptance rows from `01_PLAN.md:845-872` remain as-is.

---

## Gates

Gate contents inherited from `01_PLAN.md:876-890`. Only the Phase 2
test count in item 3 changes: Phase-2 end total ≥ **20** new tests
(was 19).

## Risks and Open Questions

Unchanged from `01_PLAN.md:892-918`.
