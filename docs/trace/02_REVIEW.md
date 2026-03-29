# `trace` REVIEW `02`

> Status: Open
> Feature: `trace`
> Iteration: `02`
> Owner: Reviewer
> Target Plan: `02_PLAN.md`
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

Round 02 is materially stronger than round 01. It responds directly to the prior blockers, adds more concrete code-level sketches, and improves the trace design by bounding ftrace and by trying to push ISA-specific debugger behavior behind a trait. The response matrix is complete and the round is much closer to implementation than the previous drafts.

The remaining blockers are now concentrated in the detailed mechanics the round was supposed to settle. The parser sketch still is not valid as written, the new debug surface still does not provide a workable memory-read path for `x/Nx` and `*addr`, the `xdb` stepping loop still reaches into private `CPU` internals instead of a real facade, and the trait-based trace registry is not type-safe in the form shown. Those are still PLAN-stage issues because the round explicitly claims to have resolved them with concrete code.

---

## Findings

### R-001 `The new parser sketch is still not implementable as written`

- Severity: HIGH
- Section: `API Surface / Architecture`
- Type: Flow
- Problem:
  The round resolves the command-syntax issue by adding a pre-parser, but the concrete sketch is still not valid enough to approve. `preprocess_line()` uses `regex!(...)` while the plan only adds the `regex` crate, whose documented API is `Regex::new(...)`, not a built-in `regex!` macro. The same section also uses `#[command(name = "")]` to model bare `b <addr>` as an empty-name subcommand, which is not a defined clap command shape.
- Why it matters:
  Round 02 explicitly claims to close the parser blocker with concrete code, so the code sketch itself has to be sound. If the documented parser path is still invalid, the main CLI boundary is not actually settled.
- Recommendation:
  The next PLAN should provide a parser design that is valid with the chosen crates:
  either a real `Regex::new`-based pre-parser plus clap-friendly subcommands,
  or a small custom tokenizer/dispatcher for the debugger command language.

### R-002 `The debug facade still does not expose a workable memory-read API`

- Severity: HIGH
- Section: `Data Structure / API Surface / Implementation Plan`
- Type: API
- Problem:
  `DebugOps` only exposes `read_register`, `dump_registers`, `fetch_inst_at`, and `disasm`, but Phase 2 and Phase 3 still require `x/Nx` memory examination and expression dereference `*addr`. The plan resolves those by calling `Bus::read_ram()`, but that helper only reads RAM and explicitly rejects MMIO. The current bus tests even assert that behavior.
- Why it matters:
  This means the approved design still cannot inspect physical MMIO addresses through `x` or memory watch expressions, which is a real debugging gap for ACLINT/PLIC/UART bring-up. It also means xdb still lacks a single explicit public API for debugger memory reads.
- Recommendation:
  Add a concrete debugger-read API to the facade and define its scope explicitly:
  either a physical-bus read that includes MMIO,
  or an explicit RAM-only scope with command-level restrictions and rationale.
  The next PLAN should not rely on xdb reaching into `Bus::read_ram()` ad hoc.

### R-003 `xdb still depends on private CPU internals in the stepping loop`

- Severity: HIGH
- Section: `API Surface / Execution Flow`
- Type: API
- Problem:
  The central `debug_step()` sketch still reads `cpu.core.pc()`, `cpu.core.fetch_inst_at(...)`, `cpu.core.disasm(...)`, `cpu.state`, and `cpu.halt_ret` directly from xdb. In the live code, `CPU` keeps `core`, `state`, and `halt_ret` private. The round claims the new facade resolves the CPU boundary, but the actual stepping-loop sketch still bypasses that facade.
- Why it matters:
  This is the same boundary problem in a more concrete form. If xdb still needs direct field access to drive stepping, then the `DebugOps` design has not actually solved the xdb<->xcore interface.
- Recommendation:
  The next PLAN should define the actual public xcore surface xdb will call in `debug_step()`:
  for example CPU-level debug accessors or a CPU-level `debug_core()` handle behind `cfg(feature = "debug")`.
  The pseudocode should then use only that public surface.

### R-004 `The trait-based trace registry is unsound and incomplete as specified`

- Severity: HIGH
- Section: `Data Structure / Master Compliance`
- Type: Maintainability
- Problem:
  `TraceState::get()` and `get_mut()` downcast `dyn Trace` to `Any`, but `Trace` does not extend `Any` and does not provide `as_any()` / `as_any_mut()` hooks. The registry therefore cannot retrieve typed traces the way the plan shows. The same section also claims M-001 is fully applied with `register_trace!`, but the concrete code only shows manual registration of `ITrace` and `FTrace` and does not show how `MTrace` participates in the scalable registry.
- Why it matters:
  The round claims to have settled the scalability/master-directive issue with a concrete abstraction. Right now that abstraction is not internally coherent, so trace management is still not ready for implementation.
- Recommendation:
  The next PLAN should provide a valid trace-registry contract:
  either make `Trace: Any` with explicit downcast helpers,
  or avoid typed downcasts entirely and use named trace handles.
  It should also show the actual feature-gated registration path, including mtrace.

### R-005 `ftrace call-pattern scope still mentions an instruction the current target does not implement`

- Severity: MEDIUM
- Section: `Implementation Plan / Validation`
- Type: Correctness
- Problem:
  The plan still lists `c.jal` as an ftrace call pattern. In the current xemu compressed-instruction table, the implemented control-transfer forms are `c_j`, `c_jr`, and `c_jalr`; there is no `c_jal`. This also matches the ratified RISC-V C extension, where `C.JAL` is RV32C-only while this project targets `riscv64gc`.
- Why it matters:
  The plan is now detailed enough that ISA inaccuracies in the detection matrix become design bugs. Keeping a nonexistent call form in the supported-pattern list weakens both implementation clarity and validation.
- Recommendation:
  Remove `c.jal` from the RV64 ftrace plan, or explicitly split the pattern matrix by ISA width if future RV32 support is intended.

---

## Trade-off Advice

### TR-1 `Prefer a single debugger-read API over separate ad hoc register and bus paths`

- Related Plan Item: `R-002 / T-1`
- Topic: Clean Design vs Convenience
- Reviewer Position: Prefer Option A
- Advice:
  Expose one explicit debugger-read surface from xcore for register, instruction, and memory observation instead of mixing `DebugOps` for some reads and direct `Bus::read_ram()` calls for others.
- Rationale:
  The current split is exactly why memory observation remains underspecified while register/disassembly access is partially abstracted. A single debugger-facing surface makes semantics and feature-gating easier to reason about.
- Required Action:
  The next PLAN should decide whether debugger reads live behind `DebugOps` alone or behind a separate CPU-level debug facade, but not both.

### TR-2 `Prefer an explicit named-trace registry over runtime downcasting if extensibility is the main goal`

- Related Plan Item: `M-001`
- Topic: Flexibility vs Safety
- Reviewer Position: Prefer Option B
- Advice:
  If the goal is scalable trace registration rather than plugin-style dynamic polymorphism, a named registry keyed by trace kind is likely simpler and safer than `Box<dyn Trace>` plus type downcasting.
- Rationale:
  The current downcast sketch is already the most fragile part of the round. For three built-in traces with feature gating, a simpler registry may satisfy the master directive with less complexity.
- Required Action:
  The next PLAN should justify why dynamic trait objects are needed here, or switch to a simpler registry model.

---

## Positive Notes

- The round does resolve the prior unbounded-ftrace issue by moving to bounded storage.
- Explicitly choosing physical debugger reads is an acceptable direction as long as the plan defines the resulting API and scope correctly.
- The round is much more concrete than 01 and is generally moving in the right direction.

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
- Reason: The round is close, but the parser, memory-observation API, CPU debug boundary, and trace-registry design are still not coherent enough to implement without another architecture pass.
