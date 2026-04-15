# `trace` PLAN `09` (FINAL)

> Status: Approved for Implementation
> Feature: `trace`
> Iteration: `09`
> Owner: Executor
> Depends on:
> - Previous Plan: `08_PLAN.md`
> - Review: `08_REVIEW.md`
> - Master Directive: `08_MASTER.md`

---

## Summary

Final implementation plan. Fixes R-001 (address semantics), R-002 (wp eval errors), R-003 (name helpers). Merges CoreDebugOps + DebugOps into single `DebugOps` trait per M-001.

### Key Decisions

**Address semantics**: All debugger addresses are **physical**. Current xemu runs bare-metal with identity mapping (VA == PA). `x/Ni` defaults to `cpu.pc()` which is a physical address in bare-metal. Command help documents this. When MMU paging is added in Phase 7, a future `xv` command will add virtual-address support.

**Watchpoint eval errors**: `WatchManager::check()` uses `Result<u64, EvalError>`. Eval failures are **non-triggers** — if evaluation fails, the watchpoint is skipped with a warning, not fired. `prev_value` uses `WatchValue` enum: `Ok(u64)` | `Err`.

**Single `DebugOps` trait**: Merges `CoreDebugOps` (bp management) + `DebugOps` (read-only inspection) into one trait per M-001.

**Name helpers**: `RVReg::name()` and `RVReg::from_name()` added to `reg.rs`. CSR name lookup via extending `csr_table!` macro to generate `CsrAddr::from_name()`.

### Response Matrix

| Source | ID | Resolution |
|--------|----|------------|
| R-001 | All physical. Bare-metal identity map. Documented. |
| R-002 | `WatchValue` enum. Eval errors = non-trigger + warning. |
| R-003 | `RVReg::name/from_name`, `CsrAddr::from_name` — concrete helpers. |
| M-001 | Single `DebugOps` trait. |
| M-002 | All findings fixed. |

---

## Implementation Scope

### Phase 1: xcore debug infrastructure

**New files:**
- `xcore/src/cpu/debug.rs` — `DebugOps` trait, `Breakpoint`, `format_mnemonic()`

**Modified files:**
- `xcore/Cargo.toml` — add `debug` feature
- `xcore/src/cpu/mod.rs` — CPU fields + pass-through methods (cfg-gated)
- `xcore/src/cpu/core.rs` — unchanged (CoreOps stays as-is)
- `xcore/src/cpu/riscv/mod.rs` — bp fields + bp check in step() + trace!() logging
- `xcore/src/cpu/riscv/debug.rs` — impl DebugOps for RVCore
- `xcore/src/cpu/riscv/mm.rs` — debug!() per memory access
- `xcore/src/isa/riscv/reg.rs` — add `name()`, `from_name()`
- `xcore/src/cpu/riscv/csr.rs` — extend csr_table! to generate `from_name()`
- `xcore/src/error.rs` — `XError::DebugBreak(usize)`
- `xcore/src/lib.rs` — re-export debug types when feature enabled

### Phase 2: xdb commands

**New files:**
- `xdb/src/expr.rs` — chumsky expression parser
- `xdb/src/watchpoint.rs` — WatchManager

**Modified files:**
- `xdb/Cargo.toml` — add regex, chumsky; enable xcore debug feature
- `xdb/src/cli.rs` — preprocess_line(), expanded Commands
- `xdb/src/cmd.rs` — all new command handlers
- `xdb/src/main.rs` — respond() with DebugBreak handling, WatchManager state
