# `trace` REVIEW `08`

> Status: Open
> Feature: `trace`
> Iteration: `08`
> Owner: Reviewer
> Target Plan: `08_PLAN.md`
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
- Blocking Issues: `2`
- Non-Blocking Issues: `1`

## Summary

Round 08 is materially better than round 07. The explicit `CoreDebugOps` / `DebugOps` split is the right fix for the old generic-bound problem, and the stable breakpoint ID model is now coherent with `b` / `bd` / `bl`.

The plan is still not ready for implementation because the biggest round-07 semantic blocker is not actually resolved. The plan now says all debugger addresses are physical and claims that `cpu.pc()` is therefore a physical default for `x/Ni`, but the concrete sketch still forwards the current `CoreOps::pc()` surface, which in the live code is virtual. The watchpoint design also still collapses expression-evaluation failures into synthetic value changes, which will create false stops and misleading output. Those are user-visible debugger semantics, not minor implementation details.

---

## Findings

### R-001 `The address-space blocker from round 07 is still not actually resolved`

- Severity: HIGH
- Section: `Summary / Response Matrix / Architecture / API Surface / Validation`
- Type: Spec Alignment
- Problem:
  The round says `R-002 (07)` is fixed by making all debugger addresses physical and by defaulting `x/Ni` to `cpu.pc()`, which it claims is a physical address. But the concrete `CPU::pc()` sketch still just forwards `self.core.pc().as_usize()`, and the current `CoreOps::pc()` in `xcore` still returns `VirtAddr`. The same physical-only `DebugOps::read_memory(paddr, size)` path is then used for `x/Nx`, `x/Ni`, and expression dereference in `p <expr>` / `w <expr>`. So under MMU-enabled execution, commands like `x/5i`, `p *$sp`, or `w *$a0` can still inspect a different address space from the one the CPU is actually fetching and loading through.
- Why it matters:
  This was the main remaining user-facing blocker in round 07, and the response matrix now incorrectly marks it resolved. A debugger command contract is only coherent if its defaults, dereference semantics, and underlying read/fetch APIs all agree on the same address space.
- Recommendation:
  The next PLAN should pick one consistent contract and implement to it. If the debugger is intentionally physical-only, then `x/Ni` cannot silently default to the current `pc`, and `*expr` dereference should not pretend to be current-context memory inspection. If current-context debugging is the goal, `xcore` needs translated read/fetch APIs instead of raw `read_ram`, and the validation section must cover MMU-enabled cases directly.

### R-002 `Watchpoint evaluation still turns read failures into false trigger events`

- Severity: HIGH
- Section: `Data Structure / xdb command flow`
- Type: Correctness
- Problem:
  The round’s watchpoint loop calls `eval_expr(...).ok()` and feeds that into `WatchManager::check`, which compares `Option<u64>` values directly. Any parse failure, unknown register/CSR, or memory-read error becomes `None`, and `check()` treats `prev_value != new_val` as a watchpoint hit, then prints missing values as `0`. In other words, evaluation failure is currently encoded as “the watched value changed”.
- Why it matters:
  Software watchpoints are one of the core phase goals. Under this design, invalid or temporarily unreadable expressions will stop execution as if the guest changed data, and the reported old/new values will be misleading. That makes watchpoints noisy and unreliable exactly in the debugging scenarios where they are supposed to help.
- Recommendation:
  The next PLAN should carry a real error channel through watchpoint evaluation, such as `Result<Option<u64>, EvalError>` or an equivalent enum. It should then define explicit runtime policy for unreadable expressions: reject at creation, report a watch-eval error, or treat “not yet readable” as a non-trigger state. Do not collapse evaluation failure into `None` and then into `0`.

### R-003 `The code sketches still depend on name-resolution helpers that are not specified`

- Severity: MEDIUM
- Section: `Data Structure / Implementation Plan / Validation`
- Type: Completeness
- Problem:
  The `DebugOps` sketches rely on `RVReg::name()`, `RVReg::try_from_name()`, and `CsrAddr::from_name()`, but the current code does not provide those helpers and the implementation plan never lists their addition explicitly. That leaves part of `read_register`, `dump_registers`, and `info reg` as unstated pseudocode rather than an implementation-ready design.
- Why it matters:
  This is not a deep architecture flaw, but it is still part of the public debugger surface. Register/CSR name resolution is exactly the kind of detail that tends to drift unless the accepted aliases and API location are nailed down in the plan.
- Recommendation:
  The next PLAN should specify where these helpers live, their exact signatures, and which names/aliases are supported (`x5` and `t0`, `pc`, CSR mnemonics, case rules, and negative lookup behavior). Validation should also include failing-name cases, not only successful reads.

---

## Trade-off Advice

### TR-1 `Prefer one command family per address-space contract`

- Related Plan Item: `R-001`
- Topic: Simpler physical-RAM inspection vs semantically correct current-context debugging
- Reviewer Position: Prefer Option B
- Advice:
  Keep a physical-inspection path if you want it, but do not overload the same command defaults and dereference syntax with both physical and current-context meanings. Either make `x` genuinely current-context through translated reads/fetches, or keep it explicitly physical and introduce a separate virtual/current-context command later.
- Rationale:
  The confusing part is not “physical mode exists”; it is the current hybrid where command syntax looks context-relative while the underlying API is raw physical RAM only.
- Required Action:
  The next PLAN should choose one address-space contract for each command family and make the defaults consistent with that choice.

### TR-2 `Prefer explicit watchpoint evaluation states over Option-collapsing`

- Related Plan Item: `R-002`
- Topic: Shorter watchpoint loop vs reliable stop semantics
- Reviewer Position: Prefer Option B
- Advice:
  Represent watchpoint evaluation as distinct states such as `Value(u64)`, `Unreadable`, and `Error`, rather than compressing everything into `Option<u64>`.
- Rationale:
  `Option<u64>` is convenient for code shape, but it loses the distinction between “not yet readable”, “temporary read failure”, and “actual value changed”, which is exactly the distinction a debugger needs.
- Required Action:
  The next PLAN should define the evaluation-state model and the stop/reporting policy for each state transition.

---

## Positive Notes

- The explicit `CoreDebugOps` / `DebugOps` split is the right answer to round 07’s generic-bound problem.
- The stable breakpoint ID model is now coherent with the `b`, `bd`, and `bl` command set.
- The non-fatal `DebugBreak` handling plus `set_skip_bp()` is still the right direction for interactive breakpoint UX.

---

## Approval Conditions

### Must Fix

- R-001
- R-002

### Should Improve

- R-003

### Trade-off Responses Required

- TR-1
- TR-2

### Ready for Implementation
- No
- Reason: Round 08 fixes the old trait-bound and breakpoint-numbering issues, but the command address-space contract and watchpoint stop semantics are still not coherent enough to implement safely.
