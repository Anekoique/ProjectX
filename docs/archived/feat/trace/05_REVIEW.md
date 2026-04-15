# `trace` REVIEW `05`

> Status: Open
> Feature: `trace`
> Iteration: `05`
> Owner: Reviewer
> Target Plan: `05_PLAN.md`
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

Round 05 fixes two real round-04 problems. Moving `DebugContext` onto `RVCore` finally aligns ownership with the hook site, and restoring expression-based watchpoints matches the Phase 5 requirement again. The file split requested by `04_MASTER` is also clearer than the previous monolithic shape.

The plan is still not ready for implementation because the new split introduces a different set of execution and boundary problems. xdb-owned watchpoints are defined as "check after each `CPU::step()`", but the execution model still centers on `CPU::run()` and does not give xdb per-instruction control during `continue` or `step N`. The trace capture path also now calls xdb-side helpers from xcore, which breaks the crate layering the round claims to improve. Finally, the xcore debugger facade is still not concrete enough for xdb to implement `x/Nx` and `x/Ni` against the current public boundary.

---

## Findings

### R-001 `xdb-owned watchpoints still do not work during continue or step-N execution`

- Severity: CRITICAL
- Section: `Summary / Invariants / API Surface / Implementation Plan`
- Type: Flow
- Problem:
  The round says watchpoints live in xdb and are evaluated "after each `CPU::step()`", while also keeping `step()/run()` unchanged and explicitly avoiding a separate debug run path. That conflicts with the actual frontend execution model: current `cmd_continue()` delegates to `cmd_step(u64::MAX)`, `cmd_step()` delegates to `CPU::run(count)`, and batch mode also calls `run(u64::MAX)` directly. Under that flow, xdb does not regain control between retired instructions, so it cannot re-evaluate software watchpoints during `continue`, `step N`, or batch execution.
- Why it matters:
  This breaks the main watchpoint stop path for the most important execution modes. As written, an expression watchpoint would only be checked after `run()` returns for some other reason, which defeats the purpose of a watchpoint.
- Recommendation:
  The next PLAN should define one concrete stop model for software watchpoints. Either xdb must own the stepping loop when watchpoints are active, or xcore must expose a per-step callback/event surface that lets xdb re-evaluate expressions before `run()` continues. The plan should also state the intended behavior for batch mode explicitly.

### R-002 `Trace capture in xcore now depends on xdb-side helpers`

- Severity: HIGH
- Section: `Architecture / Data Structure / Implementation Plan`
- Type: API
- Problem:
  The architecture places `format_mnemonic()` in `xdb/src/fmt.rs` and `detect_call_return()` in `xdb/src/trace/ftrace.rs`, but the xcore execution sketches call both from inside `RVCore::step()`. That is a reverse dependency: xdb already depends on xcore, and xcore cannot call helpers implemented in xdb without breaking the crate graph and the claimed layering.
- Why it matters:
  The round's core trace-capture path is not implementable as written. It also muddies ownership of trace logic, because capture-time decoding and call/return classification belong to the execution layer, while display formatting belongs to the debugger frontend.
- Recommendation:
  Move all helper logic needed during capture into xcore, and keep xdb responsible only for command parsing and presentation. The next PLAN should make the xcore/xdb dependency direction explicit in the code sketches.

### R-003 `The public debugger facade is still not concrete enough for examine/disassembly`

- Severity: HIGH
- Section: `Architecture / Data Structure / API Surface`
- Type: API
- Problem:
  `DebugOps` still exposes only register/dump/disasm helpers, but the plan requires xdb to implement `x/Nx` and `x/Ni` via RAM reads. No public xcore API is defined for those memory reads, and the current `Bus::read_ram()` path sits behind crate-private device internals. At the same time, the CPU pass-through sketch is written on `impl<Core: CoreOps>` even though it calls methods that are not in `CoreOps` (`debug_ctx`, `debug_ctx_mut`, `enable_debug`) and returns `&dyn DebugOps` without tightening the generic bound. The inline comment "requires CoreOps extension or direct access" is effectively an admission that this boundary is still unresolved.
- Why it matters:
  xdb cannot implement memory examine/disassembly against the current public surface, and the proposed CPU facade still does not type-check as written. The debugger boundary remains under-designed even after the ownership change.
- Recommendation:
  The next PLAN should define a single debugger-facing xcore trait surface that the CPU generic impl actually requires, including a concrete RAM-read API for debugger commands. It should also state whether debugger addresses are interpreted as virtual or physical once the MMU is active.

---

## Trade-off Advice

### TR-1 `Prefer an explicit step loop when software watchpoints live in xdb`

- Related Plan Item: `R-001`
- Topic: Simplicity vs Correctness
- Reviewer Position: Prefer Option A
- Advice:
  If expression watchpoints remain owned by xdb, let xdb explicitly drive single-instruction stepping while watchpoints are enabled instead of trying to preserve a fully opaque `run()` loop.
- Rationale:
  That keeps the evaluation logic where the plan wants it, but also gives xdb the per-instruction observation point that software watchpoints require. It is simpler and more honest than claiming `run()` stays unchanged while xdb somehow still sees every step.
- Required Action:
  The next PLAN should compare "xdb-driven step loop when watchpoints exist" against "xcore callback/event hook", then commit to one behavior for `step`, `continue`, and batch mode.

### TR-2 `Keep capture logic in xcore and display logic in xdb`

- Related Plan Item: `R-002`
- Topic: Clean Layering vs Convenience
- Reviewer Position: Prefer Option A
- Advice:
  Functions that are required during execution-time trace capture should live in xcore, even if xdb also needs presentation helpers for the same data.
- Rationale:
  The current split is backwards: xcore is the only layer with access to the execution hook sites, so it must own any helper logic used there. xdb can still format or filter the captured records afterwards without creating a reverse dependency.
- Required Action:
  The next PLAN should redraw the helper placement so xcore owns capture-time decoding/classification helpers and xdb owns only rendering and commands.

---

## Positive Notes

- Moving `DebugContext` onto `RVCore` closes the round-04 ownership contradiction cleanly.
- Restoring expression-based watchpoints aligns the plan with the Phase 5 requirement in `docs/DEV.md`.
- The file split requested by `04_MASTER` is clearer than the previous all-in-one design.
- Switching to an always-overwriting circular buffer is the right direction for post-mortem traces.

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
- Reason: Round 05 improves ownership and watchpoint semantics, but the execution flow for software watchpoints, the crate layering for trace capture, and the public xcore debugger boundary are still not specified coherently enough to implement safely.
