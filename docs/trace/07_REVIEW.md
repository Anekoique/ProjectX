# `trace` REVIEW `07`

> Status: Open
> Feature: `trace`
> Iteration: `07`
> Owner: Reviewer
> Target Plan: `07_PLAN.md`
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

Round 07 is directionally better than round 06 for the new scope. Removing the callback-based trace architecture is a real simplification, and focusing the phase on xdb basics matches the new master directives. The GDB-style command surface is also much clearer than the earlier mixed command designs.

The plan is still not ready for implementation because the concrete command semantics and the xcore debugger boundary are not fully settled. The "public debug API" in `CPU` still does not type-check against the current generic core abstraction, the new memory/disassembly commands are specified against physical-only reads while still defaulting to the virtual PC and expression dereference syntax, and the new `bd <n>` / `bl` breakpoint commands do not have a stable numbering model behind them. Those are all user-facing design issues under the new command requirements.

---

## Findings

### R-001 `The claimed CPU debug API is still not a real generic boundary`

- Severity: CRITICAL
- Section: `Architecture / Data Structure / Implementation Plan`
- Type: API
- Problem:
  The round says `R-003 (06)` is resolved by defining a concrete public API on `CPU`, but the actual sketch still puts debugger methods on `impl<Core: CoreOps> CPU<Core>` while calling methods that are not in `CoreOps` at all: `add_breakpoint`, `remove_breakpoint`, `list_breakpoints`, and `skip_bp_once`. In the current code, `CoreOps` still only exposes `pc`, `bus`, `reset`, `step`, `halted`, and `halt_ret`. So the supposed public debugger surface is still not a type-checked design; it is pseudocode that depends on an unstated trait extension.
- Why it matters:
  This is the main xcore/xdb integration boundary for the new command set. If the pass-through API is not actually defined on the generic core trait surface, xdb still has no stable way to implement breakpoint commands or skip-on-resume behavior.
- Recommendation:
  The next PLAN should define an explicit debug trait alongside `CoreOps` and bind the relevant `CPU` impls to it. The code sketches should use only that real trait surface, not implicit core-specific methods.

### R-002 `The new command set still has incoherent address-space semantics`

- Severity: HIGH
- Section: `Goals / API Surface / Constraints`
- Type: Spec Alignment
- Problem:
  The round now states all debugger reads are physical RAM only through `DebugOps::read_memory(paddr, size)`, but the command surface still uses debugger syntax that naturally follows the current execution context. In particular, `x/Ni [addr]` defaults to `pc`, while `CPU::pc()` returns the core's current PC value, which is virtual once paging is enabled. The same physical-only read path is also used for expression dereference in `p <expr>` and `w <expr>` through `*addr`. So commands like `x/5i`, `p *$sp`, or `w *$a0` can silently observe a different address space from the one the CPU actually fetches and loads through the MMU.
- Why it matters:
  These are now the core debugger commands of the phase. If the command syntax suggests current-machine-state inspection but the implementation reads raw physical RAM, debugger output will diverge from executed code and watched memory under MMU-enabled runs.
- Recommendation:
  The next PLAN should define one consistent address-space contract for the new commands. If reads are intentionally physical-only, the command docs and defaults must reflect that explicitly, and `x/Ni` cannot silently default to the virtual PC. If current-context debugging is the goal, xcore needs a translated read/fetch API instead of raw `read_ram`.

### R-003 `Breakpoint list/delete commands do not have a stable numbering model`

- Severity: HIGH
- Section: `API Surface / Data Structure`
- Type: Correctness
- Problem:
  The new command requirements introduce `bd <n>` and `bl`, which imply a user-visible breakpoint numbering scheme. But the plan stores breakpoints in a `BTreeSet<usize>`, exposes `list_breakpoints() -> Vec<usize>`, and forwards `remove_breakpoint(idx)` without defining how an index maps to a specific entry. With a sorted set, "index" is just the current nth address in sorted order, which changes as breakpoints are added or removed and is not a stable breakpoint identity.
- Why it matters:
  This directly affects the new breakpoint commands you asked to preserve. A debugger delete-by-number command needs stable, user-visible numbering; otherwise `bd 2` can refer to a different breakpoint after unrelated edits.
- Recommendation:
  The next PLAN should either switch the command model to address-based deletion, or define stable breakpoint IDs separately from address storage. `bl` output and `bd` semantics should then be specified in the same numbering model.

---

## Trade-off Advice

### TR-1 `Prefer an explicit debug trait over implicit core-specific methods`

- Related Plan Item: `R-001`
- Topic: Clean Abstraction vs Shorter Sketches
- Reviewer Position: Prefer Option A
- Advice:
  Introduce a real `CoreDebugOps` or similarly named trait and bind the `CPU` debugger methods to it explicitly.
- Rationale:
  The current plan keeps saying the API is concrete while still relying on methods that are not part of `CoreOps`. A dedicated trait is simpler than more comments and avoids repeating this boundary bug in later rounds.
- Required Action:
  The next PLAN should show the exact trait definition and the `CPU` impl bounds that xdb relies on.

### TR-2 `Prefer stable breakpoint IDs over sorted-set indices`

- Related Plan Item: `R-003`
- Topic: Simplicity vs Command Correctness
- Reviewer Position: Prefer Option B
- Advice:
  Keep sorted address storage if you want, but expose a separate stable breakpoint ID in the debugger interface.
- Rationale:
  That preserves efficient lookup by address while making `bl` / `bd <n>` behave like actual debugger commands instead of ephemeral list positions.
- Required Action:
  The next PLAN should compare "delete by address" against "stable numeric breakpoint IDs" and commit to one command contract.

---

## Positive Notes

- Removing the trace callback architecture is a real simplification and matches the latest master directive.
- The new round is much more focused on the xdb basics that still matter: breakpoints, watchpoints, expression evaluation, disassembly, and register/memory inspection.
- The pre-parser design for `x/Ni` / `x/Nx` remains credible and consistent with the current clap frontend.
- The non-fatal `DebugBreak` handling in `respond()` is the right direction for interactive breakpoint stops.

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
- Reason: Round 07 has the right reduced scope, but the concrete xcore debugger API, the address semantics of the new commands, and the breakpoint numbering model are still not specified coherently enough to implement safely.
