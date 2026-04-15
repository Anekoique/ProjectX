# `trace` REVIEW `04`

> Status: Open
> Feature: `trace`
> Iteration: `04`
> Owner: Reviewer
> Target Plan: `04_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Rejected
- Blocking Issues: `3`
- Non-Blocking Issues: `0`

## Summary

Round 04 is materially better than round 03. The parser design is now implementable with the current clap frontend, the MMIO side-effect problem is handled correctly by restricting debugger reads to `Bus::read_ram(&self)`, and moving trace capture into the per-instruction path is the right direction for `continue`.

The plan is still not ready for implementation because two of the claimed round-03 resolutions are not actually coherent in the concrete design. The document says CPU owns `DebugContext`, but the hook site is still `RVCore::step()` and the code sketches access `self.debug_ctx` directly from `RVCore`. In parallel, the round silently narrows watchpoints from the Phase 5 requirement of expression-based value-change stops to address-based memory watches only. The trace ring sketch also still does not implement the overwrite-on-full behavior it claims. Those are still blocking design issues.

---

## Findings

### R-001 `DebugContext ownership still contradicts the chosen hook site`

- Severity: CRITICAL
- Section: `Summary / Architecture / Execution Flow / Data Structure`
- Type: Invariant
- Problem:
  The round says the CPU owns `DebugContext` and exposes it via `debug_ctx()` / `debug_ctx_mut()`, but the concrete `RVCore::step()` sketch still reads `self.debug_ctx` and `core.debug_ctx` directly. The same problem also applies to the planned mtrace hooks in `RVCore`'s `load()` / `store()` path. In the current code, `RVCore` has no `debug_ctx` field at all, and the plan does not add any `CoreOps` or callback bridge that would make a CPU-owned context reachable from those core-level hooks.
- Why it matters:
  This is the central execution model for the whole phase. If the debug state lives on `CPU`, but the only per-instruction/per-memory hook point is inside `RVCore`, the design is still not implementable as written.
- Recommendation:
  The next PLAN should make the ownership model and hook site match exactly. Either move the debug context into `RVCore` (with CPU pass-through accessors if desired), or keep it on `CPU` and redesign the hook path so `step()` / `load()` / `store()` can access it explicitly through a real API instead of direct field access.

### R-002 `Round 04 changes watchpoints from expression-based to memory-location-only`

- Severity: HIGH
- Section: `Summary / Goals / Constraints / Trade-offs`
- Type: Spec Alignment
- Problem:
  Phase 5 in `docs/DEV.md` requires watchpoints to be "expression-based pause on value change". Round 04 changes that contract to address+size monitoring in xcore, with xdb "converts expressions to addresses". The trade-off section is explicit that `w $a0` would watch the memory address currently held in `a0`, not the value of the register expression itself. That is a semantic reduction, not just an internal implementation detail. It also leaves no defined mapping for arbitrary expressions, and still does not define whether those watched addresses are virtual or physical once the MMU is active.
- Why it matters:
  This breaks a stated Phase 5 requirement and changes the meaning of common debugger commands. Under the current memory system, guest fetch/load/store go through virtual-address translation, while the proposed debugger read path uses raw `Bus::read_ram()`; without a defined address-space contract, expression watch behavior is still ambiguous under paging.
- Recommendation:
  The next PLAN should either preserve expression watch semantics, or explicitly rescope the Phase 5 requirement and user-facing command semantics to memory-location watchpoints only. If the memory-watch design is kept, it must define how arbitrary expressions are rejected or lowered, how watch size is determined, and whether watched addresses are virtual or physical.

### R-003 `TraceRing still does not implement the overwrite semantics it claims`

- Severity: HIGH
- Section: `Data Structure / Validation`
- Type: Correctness
- Problem:
  The concrete `TraceRing<T>::push()` sketch calls `self.buf.try_push(entry).ok();` and comments "overwrite oldest on full". That is not what the selected `ringbuf` API does: `try_push` fails when the buffer is full, while overwrite mode is a separate operation. So the plan's code-level sketch still does not match the behavior it claims to provide.
- Why it matters:
  The whole point of itrace/ftrace/mtrace is recent post-mortem history. If the buffer keeps the oldest entries and silently drops the newest ones once full, the traces stop reflecting the execution that just happened.
- Recommendation:
  The next PLAN should specify real overwrite-on-full behavior using the chosen crate's overwrite API or an explicit eviction step, and the validation matrix should test for retention of the most recent entries rather than generic "overflow".

---

## Trade-off Advice

### TR-1 `Keep debug state where the execution hooks can actually reach it`

- Related Plan Item: `Architecture / R-001`
- Topic: Clean Design vs Implementability
- Reviewer Position: Prefer Option A
- Advice:
  If breakpoint/watchpoint/trace hooks remain inside `RVCore::step()` and `RVCore` memory helpers, the debug context should live with the core or be passed into those hooks through an explicit API.
- Rationale:
  A CPU-owned context can still be a valid public-facing model, but only if the plan also introduces a concrete bridge from CPU to the core hook site. The current hybrid model is where the round is still contradictory.
- Required Action:
  The next PLAN should compare "CPU-owned public accessors plus core-owned hook state" versus "fully core-owned debug context" versus "CPU-owned context with explicit hook API", then commit to one design that is reflected consistently in the code sketches.

### TR-2 `Do not silently redefine Phase 5 watchpoints`

- Related Plan Item: `T-2`
- Topic: Spec Fidelity vs Simplicity
- Reviewer Position: Prefer Option A
- Advice:
  Preserve expression-based watchpoint semantics even if that means evaluating some watch expressions outside xcore or at lower performance.
- Rationale:
  The project spec explicitly calls for expression watchpoints. A reduced memory-watch feature can be a subset or fallback mode, but it should not quietly replace the Phase 5 contract.
- Required Action:
  The next PLAN should either restore expression watch semantics, or explicitly change the authoritative spec and command semantics to a narrower memory-watch feature with a clear rationale.

---

## Positive Notes

- The parser design is now consistent with the current `shlex` + clap frontend and resolves the round-02/03 parser drift cleanly.
- Restricting debugger reads to `Bus::read_ram(&self)` correctly avoids the earlier MMIO side-effect problem.
- Moving trace capture into the per-instruction execution path is the right direction for `continue` and `step N`.
- Removing the thread-local mtrace sink is an improvement in design clarity.

---

## Approval Conditions

### Must Fix

- R-001
- R-002
- R-003

### Should Improve

- None

### Trade-off Responses Required

- TR-1
- TR-2

### Ready for Implementation

- No
- Reason: Round 04 fixes several real problems, but the chosen debug-context placement is still internally contradictory, watchpoints no longer match the stated Phase 5 contract, and the trace ring still does not implement the bounded overwrite behavior required for useful post-mortem tracing.
