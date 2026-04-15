# `trace` REVIEW `01`

> Status: Open
> Feature: `trace`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
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

Round 01 fixes the core ownership mistake from round 00: moving breakpoint/watchpoint state out of `xcore`, deferring symbolized ftrace, and introducing an explicit debugger stop model are the right corrections. The response matrix is complete, and the revised direction is materially closer to an implementable Phase 5 plan.

The remaining problems are now lower-level but still blocking. The plan still assumes a GDB-style `x/Ni` command surface that does not fit the current `shlex + clap` parser architecture, it leaves the `xcore` debug facade under-specified for the current generic `CPU<Core>` boundary, it defines virtual debugger reads as read-only even though the existing MMU path is stateful, and it still does not specify a concrete ownership/registration model for xcore-side mtrace hooks feeding xdb-owned buffers. Those gaps need to be resolved in the next PLAN before implementation starts.

---

## Findings

### R-001 `GDB-style x/Ni syntax does not fit the planned clap parsing model`

- Severity: HIGH
- Section: `Architecture / API Surface / Execution Flow`
- Type: Flow
- Problem:
  The plan adopts GDB-style commands such as `x/Ni addr` and still says the REPL flow is "User types command -> clap parses -> dispatch to handler". In the live frontend, input is tokenized with `shlex::split()` and then parsed by clap multicall subcommands. Under that model, `x/5i 0x80000000` arrives as a first token `x/5i`, not as command `x` plus arguments, so clap cannot parse the dynamic `/Ni` suffix without an explicit pre-parser.
- Why it matters:
  This is not a cosmetic CLI detail. `x` is one of the main Phase 5 commands, and as written the approved syntax cannot be recognized by the parser architecture the plan keeps.
- Recommendation:
  The next PLAN should explicitly choose one of these paths:
  add a custom command pre-parser before clap for GDB-style slash syntax,
  or change the command syntax to something clap can parse directly (for example `x /5i addr` or `x --fmt i --count 5 addr`).

### R-002 `The xcore debug facade is still underspecified for the current generic CPU boundary`

- Severity: HIGH
- Section: `Architecture / API Surface`
- Type: API
- Problem:
  The plan adds RV-specific methods such as `read_gpr`, `read_csr`, `read_privilege`, `read_mem`, `fetch_raw_at`, and `decode_raw -> DecodedInst` onto generic `CPU<Core>`, but the current `CoreOps` boundary only exposes `pc`, `bus`, `reset`, `step`, `halted`, and `halt_ret`. The plan does not say whether it will:
  extend `CoreOps`,
  add an RV-only specialization,
  or publicly re-export the ISA types needed by xdb. It also introduces `StopReason::ProgramExit(u32)` without defining how xdb will read the termination state and exit code through the proposed facade.
- Why it matters:
  Breakpoints, register display, expression evaluation, disassembly, and clean stop reporting all depend on this facade. Without a concrete contract, the plan still leaves the main xdb<->xcore boundary unresolved.
- Recommendation:
  The next PLAN should define the exact facade shape, including whether it is RV-specific or generic, which new public exports are required from `xcore`, and which termination/exit-code accessors xdb will use after `cpu.step()`.

### R-003 `Virtual debugger reads are specified as side-effect-free read-only calls, but the existing MMU path is stateful`

- Severity: HIGH
- Section: `Invariants / API Surface / Constraints`
- Type: Correctness
- Problem:
  The plan says debugger memory/disassembly uses virtual-address translation "same as CPU" and exposes `read_mem(&self, ...)` / `fetch_raw_at(&self, ...)` as read-only facade methods. But the current RISC-V MMU translation path is `&mut self` and updates the TLB on successful translation. The plan does not define whether debugger reads are allowed to mutate MMU/TLB state, or whether they must use a separate no-side-effects translation helper.
- Why it matters:
  This is a real semantic choice, not an implementation footnote. If debugger reads mutate TLB state, they can perturb later execution. If they bypass that state, they are no longer "same as CPU". The plan currently promises both.
- Recommendation:
  The next PLAN should make debugger virtual-read semantics explicit:
  either allow debugger reads to use the normal mutating translation path and document that effect,
  or add a separate translation/read helper and document how it differs from normal execution.

### R-004 `mtrace still lacks a concrete bridge between xcore hooks and xdb-owned trace state`

- Severity: HIGH
- Section: `Architecture / Invariants / Implementation Plan`
- Type: Maintainability
- Problem:
  The plan says all debug/trace state lives in xdb, but precise mtrace requires xcore-side load/store hooks. The proposed bridge is only described as "thread-local or AtomicPtr-based log sink" and "xdb collects mtrace entries via facade". There is no defined registration API, ownership model, reset lifecycle, or entry type contract between the two crates.
- Why it matters:
  `G-5` depends on this bridge. Without a concrete contract, the implementation will be forced to invent a cross-crate callback/channel design mid-flight, which is exactly the kind of architectural drift the PLAN stage is supposed to prevent.
- Recommendation:
  The next PLAN should define the bridge explicitly:
  where the sink lives,
  how xdb enables/disables it,
  what xcore emits,
  how reset/load clears it,
  and how the feature behaves when tracing is compiled out.

### R-005 `ftrace storage no longer matches the ring-buffered tracing goal`

- Severity: MEDIUM
- Section: `Summary / Data Structure / Trade-offs`
- Type: Spec Alignment
- Problem:
  The revised plan frames the feature as part of Phase 5 tracing and describes `trace.rs` as owning trace ring buffers, but the data structure uses `Option<Vec<FTraceEntry>>` for ftrace with the comment "unbounded". `docs/DEV.md` also explicitly frames the comparison target as ring-buffered traces.
- Why it matters:
  An unbounded ftrace log can grow without limit on long `continue` runs, which cuts against the plan's own bounded-observability direction and makes ftrace behavior inconsistent with itrace/mtrace.
- Recommendation:
  Either make ftrace bounded as well, or explicitly scope it to a different artifact such as "current call stack only" and separate that from the event log shown by `trace show`.

---

## Trade-off Advice

### TR-1 `Prefer an explicit pre-parser if GDB slash syntax is non-negotiable`

- Related Plan Item: `M-003 / G-7`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A
- Advice:
  If GDB-style `x/nfu` syntax is a hard requirement, introduce a small REPL-specific pre-parser before clap instead of trying to force the syntax through clap subcommands.
- Rationale:
  GDB's `x/nfu addr` syntax is designed as a command-language form, not a normal argv-style CLI form. A thin pre-parser keeps that syntax honest and avoids twisting clap into handling tokens it is not structured to model.
- Required Action:
  The next PLAN should explicitly state whether the frontend keeps pure clap parsing or adds a command pre-parser for GDB-style syntax.

### TR-2 `Prefer an RV-specific debugger facade first over a prematurely generic one`

- Related Plan Item: `R-002`
- Topic: Flexibility vs Safety
- Reviewer Position: Prefer Option B
- Advice:
  Define the first debugger facade against the shipped RISC-V core, then generalize later if/when LoongArch grows a real backend.
- Rationale:
  The current codebase has a real RISC-V backend and only a stub LoongArch backend. Forcing the full debugger surface through a generic trait layer now adds abstraction cost exactly where the implementation still needs the most ISA-specific detail.
- Required Action:
  The next PLAN should compare "RV-specific facade now" versus "generic facade now" and justify the chosen boundary.

---

## Positive Notes

- The round correctly resolves the round-00 ownership mistake by moving breakpoint/watchpoint logic out of `xcore`.
- Deferring symbolized ftrace instead of silently assuming ELF input is the right response to the current `.bin`-based run pipeline.
- The plan now states virtual-versus-physical debugger access explicitly, which is materially better than the round-00 ambiguity.

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
- Reason: The high-level direction is now sound, but the parser boundary, facade contract, MMU-read semantics, and mtrace bridge are still not specified tightly enough to implement without design drift.
