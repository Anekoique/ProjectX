# `trace` REVIEW `03`

> Status: Open
> Feature: `trace`
> Iteration: `03`
> Owner: Reviewer
> Target Plan: `03_PLAN.md`
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

Round 03 resolves several concrete defects from round 02. The parser sketch is now substantially more credible, the trace registry is simpler, and removing `c.jal` from the RV64 ftrace matrix is the right correction. The document is also more concrete about where the new hooks would land.

The remaining blockers come from the new design shift itself. By moving breakpoint/watchpoint handling into `CPU::run()`, the plan now leaves debugger-state ownership and watchpoint evaluation incoherent, makes debugger MMIO reads observably side-effectful, and still does not provide a path to collect itrace/ftrace/mtrace during `continue` runs. Those are design-level correctness issues, not implementation details, so the plan is still not ready for implementation.

---

## Findings

### R-001 `Breakpoint/watchpoint ownership is still incoherent under CPU-run hooks`

- Severity: CRITICAL
- Section: `Architecture / Invariants / Data Structure`
- Type: Invariant
- Problem:
  The round says `xdb` owns debug state, but it also moves breakpoint/watchpoint checks into `CPU::run()` and sketches `self.breakpoints` / `self.watchpoints` as CPU fields "set by xdb before run". It simultaneously says xdb reaches xcore only through `CPU::debug_ops()` plus `with_xcpu!`. That does not define a real ownership model or a lifetime-safe synchronization path. The watchpoint path is even less coherent: the expression parser remains in xdb, but `CPU::run()` calls `wp.check_changed()` inside xcore without defining how current values are evaluated there.
- Why it matters:
  This is the central execution model for Phase 5. If ownership of breakpoint/watchpoint state is unclear, and watchpoint evaluation still has no implementable home, the debugger stop path is not actually designed yet.
- Recommendation:
  The next PLAN should pick one coherent model and make it explicit:
  either xcore owns a debug context with concrete setter APIs from xdb,
  or xdb owns evaluation and CPU exposes a per-step callback/event surface.
  The plan must also define exactly where watchpoint expressions are evaluated and by which crate.

### R-002 `Debugger memory reads through Bus::read() are not observationally safe`

- Severity: HIGH
- Section: `Invariants / API Surface / Validation`
- Type: Correctness
- Problem:
  `DebugOps::read_memory()` is defined in terms of `Bus::read()`, and the plan treats that as a read-only observation path for RAM and MMIO. But the current MMIO interface is `Device::read(&mut self, ...)`, and several device reads are stateful: UART reads pop the RX FIFO, and PLIC claim-register reads call `claim()`. So an `x/4x` debugger read against MMIO can mutate guest-visible device state.
- Why it matters:
  A debugger memory-examine command must not silently consume input bytes or claim interrupts just because the user inspected device state. That would make debugging itself perturb the machine being debugged.
- Recommendation:
  Replace the blanket `Bus::read()` design with an explicit non-invasive debug-read contract. If some devices cannot support side-effect-free reads, the next PLAN should state that clearly and restrict or split MMIO inspection semantics instead of calling them ordinary debugger reads.

### R-003 `Trace capture no longer works during continue or multi-step execution`

- Severity: HIGH
- Section: `Architecture / Implementation Plan / Spec Alignment`
- Type: Flow
- Problem:
  The round removes `debug_step()` / `debug_continue()`, but itrace/ftrace/mtrace capture is still described as happening "after each `cmd_step`" or "after step" in xdb. `cmd_continue()` still delegates directly to `CPU::run(u64::MAX)`, which can execute many instructions internally before returning. Under that design, `continue` and `step N` do not provide per-instruction trace capture in xdb, so the ring buffers miss the very execution history they are supposed to preserve.
- Why it matters:
  `docs/DEV.md` defines itrace as ring-buffered post-mortem analysis. If traces are only captured for single-step command boundaries, they are not useful for the main `continue` workflow where post-mortem history matters most.
- Recommendation:
  The next PLAN should define a per-instruction trace event path that still works while keeping debug hooks inside existing CPU execution flow. That can be a CPU-owned trace sink, a feature-gated callback/context, or another concrete mechanism, but it must capture each retired instruction/access during `run()`, not only after xdb regains control.

### R-004 `Step semantics are broken once execution stops at a breakpoint`

- Severity: HIGH
- Section: `Execution Flow / API Surface`
- Type: Correctness
- Problem:
  `cmd_step(count)` and `cmd_continue()` both route through the same `CPU::run()` logic, and the breakpoint check is performed before `self.step()` on each loop iteration. After execution stops at a breakpointed PC, a subsequent `step` command re-enters `run()`, hits the same breakpoint check immediately, and returns `DebugBreak` again without executing the instruction.
- Why it matters:
  That traps the debugger on the breakpointed instruction unless the user first deletes the breakpoint. A stepping command that cannot advance past the current PC after a breakpoint hit is not workable debugger behavior.
- Recommendation:
  The next PLAN should define distinct stop-handling semantics for `continue` versus `step`, or a one-shot breakpoint-suppression rule for the current PC after a breakpoint stop. The exact behavior needs to be explicit before implementation.

### R-005 `The concrete code sketches still overstate available helper APIs`

- Severity: MEDIUM
- Section: `Data Structure / Implementation Detail`
- Type: Maintainability
- Problem:
  The round presents concrete code-level detail, but some helper calls in the sketches still do not correspond to current exported helpers, for example `RVReg::as_str()` in the register-dump sketch. These are individually easy to add, but they show that some of the "drop-in" code examples are still illustrative rather than implementation-ready.
- Why it matters:
  This is not blocking on its own, but the plan now positions itself as code-level concrete. Small helper mismatches reduce confidence that the design has been fully walked through.
- Recommendation:
  Tighten the next PLAN’s code snippets so referenced helper methods either already exist or are explicitly listed as part of the planned API changes.

---

## Trade-off Advice

### TR-1 `Prefer an explicit CPU-owned debug context if hooks must live inside run()`

- Related Plan Item: `R-001 / M-002`
- Topic: Clean Design vs Execution Simplicity
- Reviewer Position: Prefer Option B
- Advice:
  If breakpoint/watchpoint checks must be stubbed into `CPU::run()`, make the debug context an explicit CPU-owned structure with feature-gated setter methods from xdb instead of trying to preserve xdb ownership through shared references.
- Rationale:
  The current hybrid model is where most of the remaining ambiguity comes from. A clearly CPU-owned context is more honest about where stop decisions are made and avoids hidden lifetime/synchronization problems.
- Required Action:
  The next PLAN should compare "CPU-owned debug context" versus "xdb-owned state plus callback/event bridge" and commit to one.

### TR-2 `Prefer non-invasive MMIO inspection over full guest-visible read reuse`

- Related Plan Item: `R-002 / T-2`
- Topic: Compatibility vs Safety
- Reviewer Position: Prefer Option B
- Advice:
  Do not reuse the guest-visible `Device::read` path as the debugger's generic MMIO inspection primitive unless the plan explicitly accepts and documents the side effects.
- Rationale:
  Reusing the normal device read path is simpler, but it makes debugger inspection mutate machine state. For observability features, safety of observation is usually the more important property.
- Required Action:
  The next PLAN should either define a side-effect-free debug-read facility or explicitly limit which MMIO regions can be examined safely.

---

## Positive Notes

- The parser issue from round 02 is resolved in the right direction.
- Replacing the dynamic trace registry with explicit typed fields is a real improvement.
- Removing `c.jal` from the RV64 ftrace matrix closes the prior ISA mismatch cleanly.

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
- Reason: The plan is closer, but debugger-state ownership, MMIO-read semantics, per-instruction trace capture during `run()`, and step-after-breakpoint behavior are still not specified coherently enough to implement safely.
