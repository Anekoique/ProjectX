# `trace` REVIEW `06`

> Status: Open
> Feature: `trace`
> Iteration: `06`
> Owner: Reviewer
> Target Plan: `06_PLAN.md`
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

Round 06 fixes the most obvious round-05 design problem. Moving capture-time helpers back into xcore removes the reverse dependency, and the xdb-driven watchpoint loop is the right direction for software watchpoints. The plan is also clearer about physical-memory debugger reads.

The remaining blockers are around ownership and interface shape. The callback object that holds xdb’s trace state is moved into xcore with `Box<dyn DebugHook>`, but the rest of the design still treats xdb as if it can read and reconfigure that same object later. The hook payload is also too weak to implement correct ftrace for indirect calls and returns, and the public xcore/xdb boundary is still left as comments and direct field access instead of a real API. Those are still design-level issues, so the plan is not ready for implementation.

---

## Findings

### R-001 `Hook ownership is still incoherent for trace control and display`

- Severity: CRITICAL
- Section: `Architecture / Data Structure / Implementation Plan`
- Type: Invariant
- Problem:
  The round says xdb implements `DebugHook` and owns trace storage, but the same hook object is moved into xcore via `set_debug_hook() -> Option<Box<dyn DebugHook>>` and then stored on `RVCore` as `hook: Option<Box<dyn DebugHook>>`. At that point xcore owns the only handle to `XdbHook`, yet the plan still expects xdb to run `trace show/off` commands and to wire `register_trace!` into `XdbHook` enable flags. No shared ownership, getter, downcast path, or separate trace-state object is defined.
- Why it matters:
  This makes the main trace control path unimplementable as written. xdb cannot both transfer ownership of `XdbHook` into xcore and still inspect its buffers or mutate its enable flags later without introducing hidden globals or another unstated indirection layer.
- Recommendation:
  The next PLAN should separate the callback adapter from the debugger-owned state explicitly. For example, xdb can own shared hook state and pass xcore a lightweight hook object that writes into that shared state, or xcore can expose a well-defined accessor for a shared/debugger-owned hook handle. The ownership model needs to be concrete.

### R-002 `The hook payload is too weak for correct ftrace of indirect control flow`

- Severity: HIGH
- Section: `Goals / Invariants / Data Structure`
- Type: Correctness
- Problem:
  `DebugHook::on_execute()` only receives `(pc, raw, mnemonic)`, and ftrace is delegated to xdb-side `detect_and_record(pc, raw, mnemonic, ...)`. That is enough for direct `jal`, but it is not enough to compute resolved targets for `jalr`, `c.jalr`, or `c.jr x1`. In the current core, those targets are computed from live register values at execution time, not from opcode bits alone.
- Why it matters:
  Round 06 explicitly keeps `jalr x1`, `c.jalr`, and `c.jr x1` in scope for ftrace. Without the resolved target or the relevant register snapshot from xcore, xdb cannot produce correct target addresses or reliable call/return nesting for indirect control flow.
- Recommendation:
  The next PLAN should enrich the hook contract for control-flow tracing. Options include a dedicated `on_call` / `on_return` callback with resolved targets, or extending `on_execute` with decoded control-flow metadata that xcore computes before execution.

### R-003 `The xcore/xdb debugger API boundary is still left unresolved`

- Severity: HIGH
- Section: `Architecture / API Surface / Implementation Plan`
- Type: API
- Problem:
  The round still relies on xdb reaching across xcore internals instead of defining a real public debugger surface. The plan’s own sketch says breakpoint and hook management would use `with_xcpu(|cpu| cpu.core.add_breakpoint(addr))` and comments that this "requires making `core` pub(crate) or adding pass-through methods". The watchpoint loop sketch also reads `cpu.state.is_terminated()` directly. In the current code, `core` and `state` are private CPU fields, and `pub(crate)` would still not expose them to the external `xdb` crate. `xcore` also currently hides the `cpu` module entirely from public exports.
- Why it matters:
  Even if the hook design were fixed, xdb still could not compile against the proposed surface. The plan leaves the essential breakpoint/hook/termination API as an unresolved note instead of specifying the actual exported methods xdb will use.
- Recommendation:
  The next PLAN should define a complete public debugger API exported from xcore: pass-through methods for breakpoint management, hook installation, `skip_bp_once`, and execution-state queries, plus the public exports for `DebugHook` and `DebugOps`. Direct field access from xdb should be removed from the plan entirely.

---

## Trade-off Advice

### TR-1 `Prefer a shared-state hook adapter over moving the whole debugger object into xcore`

- Related Plan Item: `R-001`
- Topic: Clean Layering vs Ownership Clarity
- Reviewer Position: Prefer Option A
- Advice:
  Keep trace buffers and enable flags in xdb-owned shared state, and let xcore hold only a small callback adapter that writes into that state.
- Rationale:
  That preserves the callback architecture the round is aiming for, but it removes the current contradiction where xdb both "owns" the trace state and gives away the only handle to it.
- Required Action:
  The next PLAN should compare "shared-state adapter" against "xcore-owned hook with explicit inspection API", then commit to one ownership model for `trace show/off` and breakpoint reporting.

### TR-2 `Prefer richer control-flow events over raw-instruction reconstruction in xdb`

- Related Plan Item: `R-002`
- Topic: Minimal Interface vs Correctness
- Reviewer Position: Prefer Option B
- Advice:
  Let xcore provide resolved control-flow information for ftrace-critical instructions instead of asking xdb to reconstruct everything from `raw` and a mnemonic string.
- Rationale:
  That keeps the decode/execution truth in one place and avoids duplicating partial instruction semantics in xdb. It is especially important for `jalr` and compressed indirect jumps, whose targets depend on live register values.
- Required Action:
  The next PLAN should either extend the hook payload with resolved targets and event kind, or justify a raw-only reconstruction scheme that is correct for every ftrace pattern still listed in scope.

---

## Positive Notes

- Moving capture-time helpers like `format_mnemonic()` back into xcore fixes the round-05 reverse-dependency problem.
- The xdb-driven step loop is the right direction for expression watchpoints.
- Physical RAM-only debugger reads are now stated explicitly instead of being left implicit.
- The callback-interface idea is a cleaner starting point than the earlier trace-entry-in-xcore designs.

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
- Reason: Round 06 has the right high-level direction, but hook ownership, ftrace event data, and the exported xcore/xdb debugger API are still not specified concretely enough to implement safely.
