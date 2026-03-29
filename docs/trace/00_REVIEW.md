# `trace` REVIEW `00`

> Status: Open
> Feature: `trace`
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

- Decision: Rejected
- Blocking Issues: `4`
- Non-Blocking Issues: `1`

## Summary

The draft is directionally aligned with `docs/DEV.md` Phase 5 and covers the right capability set, but it is not yet implementable as a coherent first-round plan. The main blockers are architectural, not cosmetic: the plan currently places expression evaluation in `xdb` while requiring `xcore::CPU::step()` to evaluate watchpoints, relies on ELF symbol tables at runtime even though the shipped `xemu` workflow loads stripped raw binaries, and models breakpoint/watchpoint hits as generic `XError`s without defining a non-fatal debugger stop path.

Until those ownership and execution-flow issues are resolved, implementation would either violate the current crate layering (`xdb -> xcore` only), silently narrow the approved scope, or ship debugger stops that are misclassified as execution failures. The next PLAN should tighten the debugger/xcore boundary, define the runtime artifact model for symbols, and make memory/disassembly semantics explicit under MMU-enabled execution.

---

## Findings

### R-001 `Watchpoint evaluation is assigned to the wrong crate`

- Severity: CRITICAL
- Section: `Architecture / Invariants / API Surface / Execution Flow`
- Type: Invariant
- Problem:
  The plan puts the expression parser/evaluator in `xdb/src/expr.rs`, but it also says watchpoints are owned by `xcore` `DebugState` and are evaluated after every `CPU::step()`. That is not implementable with the current workspace layering: `xdb` depends on `xcore`, not the reverse. As written, `xcore` would need to call back into `xdb` to evaluate watchpoint expressions, which breaks the crate boundary the project already uses.
- Why it matters:
  This is a hard architectural contradiction inside the plan itself. If not fixed up front, the implementation will either duplicate expression logic, leak internal CPU types into `xdb`, or silently move behavior away from the approved design.
- Recommendation:
  Pick one ownership model and make it explicit in the next PLAN:
  either move expression parsing/evaluation into `xcore` as a debugger-support module, or keep expressions in `xdb` and redefine watchpoint checks as an `xdb`-driven stepping loop with a stable read-only introspection API from `xcore`.

### R-002 `ftrace symbol resolution does not match the shipped load artifact`

- Severity: HIGH
- Section: `Summary / Architecture / Constraints / Trade-offs / Validation`
- Type: Spec Alignment
- Problem:
  The plan chooses runtime ELF `.symtab` parsing on `load`, but the current build/run flow does not hand `xdb` an ELF. `xam/scripts/kernel.mk` generates `$(OUT_BIN)` via `objcopy --strip-all -O binary`, and `xemu` is run with `FILE=$(OUT_BIN)`. By the time `xcore::CPU::load()` sees the input, it is a stripped raw binary with no symbol table.
- Why it matters:
  `G-6` cannot be delivered as written. A plan that assumes symbols are available at runtime, when the default workflow removes them before launch, is approving functionality the current artifact pipeline cannot provide.
- Recommendation:
  Update the next PLAN to define the runtime symbol source explicitly. Viable options include:
  switching the debugger/runtime to load ELF directly,
  passing a sidecar `.elf` or map file together with the `.bin`,
  or scoping round `00` ftrace to raw addresses only and opening a follow-up round for symbolization.

### R-003 `Breakpoint hits are modeled as fatal errors instead of debugger stops`

- Severity: HIGH
- Section: `Data Structure / API Surface / Execution Flow`
- Type: Flow
- Problem:
  The plan adds `XError::BreakpointHit` / `XError::WatchpointHit` and says `xdb` will catch them and return to the prompt. But the current `xdb` command flow treats any command error as fatal through `terminate!`, which marks the CPU `ABORT`. There is no defined non-terminal stop/event channel in the plan, and no state model for "paused because the debugger requested a stop."
- Why it matters:
  Without an explicit stop model, the first breakpoint/watchpoint hit will be indistinguishable from a real execution failure. That breaks the interactive debugger flow and corrupts the CPU termination state.
- Recommendation:
  Replace generic error-based stops with an explicit run outcome model in the next PLAN, for example a `StopReason` / `RunOutcome` returned from `step()` / `run()`, or a dedicated paused state that `xdb` handles without calling `terminate!`.

### R-004 `Debugger memory and disassembly semantics are undefined under MMU translation`

- Severity: HIGH
- Section: `Spec / API Surface / Constraints`
- Type: API
- Problem:
  The plan introduces `info m`, expression dereference (`*addr`), watchpoint evaluation, and `x <addr>` disassembly, but it never defines whether those addresses are virtual or physical, or which translation/privilege context they use. That omission is material because `xcore` already performs MMU translation and access-fault handling in the normal fetch/load/store path.
- Why it matters:
  In a privileged emulator, debugger reads that bypass translation will disagree with what the CPU is actually executing. If the semantics stay implicit, different commands can end up observing different address spaces and produce misleading debug output.
- Recommendation:
  Make the address-space contract explicit in the next PLAN. At minimum, specify whether debugger memory/disassembly uses current virtual-address translation by default, whether there is also a physical-memory mode, and add validation for translated success and unmapped-fault cases.

### R-005 `ftrace detection scope ignores compressed control-flow forms`

- Severity: MEDIUM
- Section: `Spec / Implementation Plan / Validation`
- Type: Correctness
- Problem:
  The plan describes ftrace as detecting `JAL` / `JALR` calls and `RET` returns, but the emulator already supports compressed instructions and targets `rv64gc`. The plan does not say whether `c.jalr` / `c.jr`-style return paths, or alternate link-register conventions, are intentionally unsupported or simply overlooked.
- Why it matters:
  If the plan stays silent here, the implementation may ship a function trace that looks correct on hand-written uncompressed cases but silently misses common control-flow forms in optimized guest binaries.
- Recommendation:
  Define the supported call/return patterns explicitly in the next PLAN and cover them in validation. If round `00` intentionally limits ftrace to uncompressed standard-call-convention paths, say so as a scoped constraint instead of leaving it implicit.

---

## Trade-off Advice

### TR-1 `Prefer a read-only xcore debug facade over exposing CPU internals to xdb`

- Related Plan Item: `G-2 / G-3 / G-7`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option B
- Advice:
  Keep execution-synchronous debugger logic close to `xcore`, then expose a narrow read-only facade for `xdb` commands such as register snapshots, translated memory reads, and disassembly support.
- Rationale:
  `xdb` already depends on `xcore`; extending that boundary with a small, stable debugger API is much cheaper than exporting `CPU<Core>` internals, ISA internals, and MMU details into the frontend crate.
- Required Action:
  The next PLAN should define the minimal public debugger-facing surface from `xcore` and avoid APIs that require `xdb` to take direct ownership of core internals.

### TR-2 `Prefer sidecar-symbol support over forcing an ELF-only runtime immediately`

- Related Plan Item: `G-6 / T-4`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option B
- Advice:
  If the current `.bin`-based bring-up flow is important to preserve, introduce an explicit sidecar symbol input (`.elf` or map file) rather than silently assuming symbolized ELF loading will replace the current runtime path in one round.
- Rationale:
  This keeps Phase 5 compatible with the existing `xam -> out.bin -> xemu` pipeline while still making symbolization achievable. It also makes the symbol dependency visible instead of burying it inside `load()`.
- Required Action:
  The next PLAN should compare "ELF-only load" versus "raw binary + sidecar symbols" and state the chosen runtime contract explicitly.

---

## Positive Notes

- The draft picks the right Phase 5 capability set and stays aligned with `docs/DEV.md` instead of introducing unrelated debugger scope.
- The plan already tries to separate frontend concerns (`xdb`) from execution-time concerns (`xcore`), which is the right direction even though the current boundary needs correction.
- The ring-buffer choice for trace storage is sound and matches the long-run observability goal better than an unbounded `Vec`.

---

## Approval Conditions

### Must Fix

- R-001
- R-002
- R-003
- R-004

### Should Improve

- R-005

### Trade-off Responses Required

- TR-1
- TR-2

### Ready for Implementation

- No
- Reason: The plan still has blocking ownership, runtime-artifact, and stop-semantics gaps that would force architectural changes during implementation instead of before it.
