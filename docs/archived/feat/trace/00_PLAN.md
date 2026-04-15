# `trace` PLAN `00`

> Status: Draft
> Feature: `trace`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

Phase 5: Debugging & Observability for xemu. Extend xdb with breakpoints, watchpoints, expression evaluation, instruction/memory/function trace, and disassembly. This transforms xdb from a minimal step/continue debugger into a full-featured debugging environment.

## Log

[**Feature Introduce**]

Seven deliverables aligned with DEV.md Phase 5:

1. **Breakpoints** — address-based execution pause (`b 0x80000000`, `b delete 1`)
2. **Watchpoints** — value-change triggered pause (`w $a0`, `w *0x80001000`)
3. **Expression evaluator** — arithmetic, register refs (`$a0`), memory deref (`*addr`), hex/dec literals
4. **Instruction trace (itrace)** — ring-buffered decoded instruction log for post-mortem
5. **Memory trace (mtrace)** — log memory reads/writes with address, size, value
6. **Function trace (ftrace)** — call/return tracking with ELF symbol resolution
7. **Disassembly** — inline disasm of current/arbitrary instructions

[**Review Adjustments**]

N/A — first iteration.

[**Master Compliance**]

N/A — first iteration.

### Changes from Previous Round

N/A — first iteration.

### Response Matrix

N/A — first iteration.

---

## Spec

[**Goals**]
- G-1: Breakpoints — set/delete/list address breakpoints; `continue` stops at breakpoint
- G-2: Watchpoints — monitor expression value; pause when value changes after any step
- G-3: Expression evaluator — parse and evaluate expressions with registers, memory deref, arithmetic
- G-4: Instruction trace — configurable-size ring buffer of recent executed instructions (PC + raw + decoded mnemonic)
- G-5: Memory trace — optional log of memory accesses (addr, size, R/W, value)
- G-6: Function trace — track JAL/JALR calls and RET returns with ELF symbol names when available
- G-7: Disassembly — decode and print instruction at PC or arbitrary address

- NG-1: No difftest (Phase 6)
- NG-2: No instruction cache / performance optimization (Phase 6)
- NG-3: No GDB remote protocol (future)

[**Architecture**]

```
xdb (debugger frontend)
 ├─ cli.rs          — clap command definitions
 ├─ cmd.rs          — command implementations
 ├─ expr.rs         — expression parser + evaluator (NEW)
 └─ fmt.rs          — display formatting for traces/disasm (NEW)

xcore (emulator engine)
 ├─ trace/mod.rs    — TraceManager: itrace + mtrace ring buffers (NEW)
 ├─ trace/itrace.rs — InstructionTrace entry (NEW)
 ├─ trace/mtrace.rs — MemoryTrace entry (NEW)
 ├─ trace/ftrace.rs — FunctionTrace with ELF symbol table (NEW)
 ├─ debug/mod.rs    — DebugState: breakpoints + watchpoints (NEW)
 ├─ debug/bp.rs     — Breakpoint storage + matching (NEW)
 ├─ debug/wp.rs     — Watchpoint storage + evaluation (NEW)
 └─ cpu/riscv/      — Hook points in step() for trace capture + bp/wp check
```

Execution flow with debugging:
```
CPU::step()
  ├─ fetch instruction
  ├─ ─── itrace: record (pc, raw, decoded) ───
  ├─ ─── breakpoint check: if pc in bp_set → return BreakpointHit ───
  ├─ decode + execute
  ├─ ─── mtrace: recorded via load/store hooks ───
  ├─ ─── ftrace: detect JAL/JALR/RET → record call/return ───
  ├─ retire (update pc)
  └─ ─── watchpoint check: evaluate exprs, compare old/new values ───
```

[**Invariants**]
- I-1: Breakpoints only checked in step(), not in batch `run()` inner loop (performance). `run()` calls `step()` per iteration, so bp check happens naturally.
- I-2: Watchpoints evaluated after each step. Value comparison uses expression evaluator.
- I-3: Ring buffers have fixed capacity. Oldest entries evicted on overflow.
- I-4: Trace capture has zero overhead when disabled (compile-time `cfg` or runtime flag).
- I-5: All debug state is owned by `CPU`, not global. Clean separation from execution logic.
- I-6: Expression evaluator is pure (no side effects) — reads registers/memory but never writes.

[**Data Structure**]

```rust
// xcore/src/debug/mod.rs
pub struct DebugState {
    pub breakpoints: Vec<usize>,          // sorted addresses
    pub watchpoints: Vec<Watchpoint>,
}

pub struct Watchpoint {
    pub id: u32,
    pub expr: String,                     // raw expression text
    pub prev_value: Option<Word>,         // last evaluated value
}

// xcore/src/trace/itrace.rs
pub struct ITraceEntry {
    pub pc: usize,
    pub raw: u32,
    pub mnemonic: String,                 // "addi sp, sp, -16"
}

// xcore/src/trace/mtrace.rs
pub struct MTraceEntry {
    pub pc: usize,
    pub addr: usize,
    pub size: usize,
    pub op: MemOp,                        // Read / Write
    pub value: Word,
}

// xcore/src/trace/ftrace.rs
pub struct FTraceEntry {
    pub pc: usize,
    pub target: usize,
    pub kind: FTraceKind,                 // Call / Return
    pub symbol: Option<String>,           // ELF symbol name if available
    pub depth: usize,                     // call depth for indentation
}

// Ring buffer
pub struct RingBuf<T> {
    buf: Vec<T>,
    head: usize,
    len: usize,
}
```

[**API Surface**]

xcore debug API (accessible via `with_xcpu!`):

```rust
// Breakpoints
pub fn add_breakpoint(&mut self, addr: usize)
pub fn remove_breakpoint(&mut self, index: usize) -> bool
pub fn list_breakpoints(&self) -> &[usize]

// Watchpoints
pub fn add_watchpoint(&mut self, expr: String)
pub fn remove_watchpoint(&mut self, id: u32) -> bool
pub fn list_watchpoints(&self) -> &[Watchpoint]

// Trace control
pub fn set_itrace(&mut self, enabled: bool, capacity: usize)
pub fn set_mtrace(&mut self, enabled: bool, capacity: usize)
pub fn set_ftrace(&mut self, enabled: bool)
pub fn get_itrace(&self) -> &RingBuf<ITraceEntry>
pub fn get_mtrace(&self) -> &RingBuf<MTraceEntry>
pub fn get_ftrace(&self) -> &[FTraceEntry]

// Disassembly
pub fn disasm_at(&self, addr: usize, count: usize) -> Vec<(usize, u32, String)>
```

xdb expression evaluator:

```rust
// xdb/src/expr.rs
pub fn eval_expr(expr: &str, cpu: &CPU<Core>) -> Result<Word, String>
```

Expression grammar:
```
expr     = term (('+' | '-') term)*
term     = factor (('*' | '/') factor)*
factor   = unary | '(' expr ')'
unary    = '*' factor              // memory deref (4-byte read)
         | '-' factor              // negation
         | atom
atom     = '$' REGISTER            // register value ($a0, $sp, $pc)
         | '0x' HEX_DIGITS        // hex literal
         | DECIMAL_DIGITS          // decimal literal
```

xdb new commands:

```
# Breakpoints
b <addr>              — set breakpoint at address
b delete <n>          — delete breakpoint by index
b list                — list all breakpoints

# Watchpoints
w <expr>              — set watchpoint on expression
w delete <n>          — delete watchpoint by id
w list                — list all watchpoints

# Info
info r                — print all registers
info r <name>         — print one register
info m <addr> [len]   — examine memory

# Expression
p <expr>              — evaluate and print expression

# Trace
trace itrace [N]      — enable itrace with N entries (default 16)
trace mtrace [N]      — enable mtrace with N entries (default 64)
trace ftrace          — enable ftrace
trace off             — disable all traces
trace show            — print current trace buffers

# Disassembly
x <addr> [count]      — disassemble N instructions from addr
x                     — disassemble at current PC
```

[**Constraints**]
- C-1: Expression evaluator is recursive descent, no external parser dependency.
- C-2: Ring buffer capacity configurable, default 16 for itrace, 64 for mtrace.
- C-3: ftrace requires ELF symbol table — optional, degrades gracefully to raw addresses.
- C-4: mtrace hooks must not break existing Device trait. Use optional callback or flag.
- C-5: Breakpoint/watchpoint checks add per-step overhead. Acceptable for interactive debugging.
- C-6: Disassembly uses existing `DECODER.decode()` + format to mnemonic string. No LLVM dependency.

---

## Implement

### Execution Flow

[**Main Flow**]
1. User sets breakpoint/watchpoint via xdb command
2. `continue` or `step` calls `CPU::step()`
3. In step(): check breakpoints → fetch → itrace record → decode → execute → mtrace/ftrace record → retire → watchpoint check
4. If breakpoint hit or watchpoint triggered → return `XError::BreakpointHit` / `XError::WatchpointHit`
5. xdb catches these and drops to prompt with diagnostic message

[**Failure Flow**]
1. Invalid expression syntax → return parse error string, no state change
2. Memory deref in expression fails → return "cannot read memory at addr"
3. ELF not loaded for ftrace → degrade to raw addresses

### Implementation Plan

[**Phase 1: Foundation**]
- RingBuf<T> generic ring buffer
- DebugState struct (breakpoints + watchpoints)
- Wire DebugState into CPU
- `XError::BreakpointHit` / `XError::WatchpointHit` variants

[**Phase 2: Breakpoints + Info commands**]
- `b`/`b delete`/`b list` xdb commands
- Breakpoint check in step() before execute
- `info r` / `info m` commands for register and memory inspection
- Colored register/memory display

[**Phase 3: Expression evaluator + Watchpoints**]
- Recursive descent parser in `xdb/src/expr.rs`
- Register refs (`$a0`, `$pc`), memory deref (`*addr`), arithmetic
- `p <expr>` command
- `w <expr>` / `w delete` / `w list` commands
- Watchpoint evaluation after each step

[**Phase 4: Instruction trace + Disassembly**]
- ITraceEntry + ring buffer in xcore
- Capture hook in step() after fetch+decode
- Mnemonic formatter: `DecodedInst` → human-readable string
- `trace itrace` / `trace show` commands
- `x` disassembly command using decoder + formatter

[**Phase 5: Memory trace + Function trace**]
- MTraceEntry + ring buffer
- Hook in load/store paths (optional, flag-controlled)
- FTraceEntry with call depth tracking
- Detect JAL/JALR (call) and RET (jalr x0, ra, 0) patterns
- ELF symbol table loader (parse .symtab from ELF binary)
- `trace mtrace` / `trace ftrace` commands

---

## Trade-offs

- T-1: **Expression evaluator: custom vs parser-combinator crate** — Custom recursive descent is ~100 lines, zero dependencies. A crate (nom, pest) adds dependency for marginal benefit. **Prefer custom.**

- T-2: **Trace storage: ring buffer vs Vec** — Ring buffer bounds memory, Vec grows unbounded. For long runs, unbounded mtrace would consume GB. **Prefer ring buffer with configurable capacity.**

- T-3: **Breakpoint check location: before fetch vs after decode** — Before fetch is simpler (just compare PC). After decode allows instruction-type breakpoints but adds complexity. **Prefer before fetch (address-only breakpoints).**

- T-4: **ftrace symbol resolution: build-time ELF parse vs runtime** — Runtime ELF parse on `load` command keeps the binary generic. Build-time couples to ELF format. **Prefer runtime parse on load.**

- T-5: **Disassembly: reuse DECODER vs capstone/LLVM** — DECODER already parses all supported instructions. Building mnemonics from `DecodedInst` + `InstKind` is 200-300 lines. External disassemblers add heavy deps and may disagree with our decoder. **Prefer reuse DECODER.**

---

## Validation

[**Unit Tests**]
- V-UT-1: RingBuf — push, overflow, iterate, clear
- V-UT-2: Expression parser — literals, registers, arithmetic, memory deref, precedence, error cases
- V-UT-3: Breakpoint add/remove/match
- V-UT-4: Mnemonic formatter — all InstKind variants produce valid strings

[**Integration Tests**]
- V-IT-1: Breakpoint halts execution at target address
- V-IT-2: Watchpoint triggers on register value change
- V-IT-3: itrace captures correct instruction sequence for a known program
- V-IT-4: ftrace detects call/return for a function with known ELF symbols

[**Edge Case Validation**]
- V-E-1: Breakpoint at address 0 (should work, it's a valid address)
- V-E-2: Watchpoint on expression that reads unmapped memory (graceful error)
- V-E-3: Ring buffer wrap-around preserves most recent entries
- V-E-4: Expression with nested parentheses and operator precedence

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (breakpoints) | V-IT-1: step/continue stops at bp address |
| G-2 (watchpoints) | V-IT-2: value change triggers pause |
| G-3 (expr eval) | V-UT-2: parser correctness |
| G-4 (itrace) | V-IT-3: ring buffer captures instruction flow |
| G-5 (mtrace) | Manual: memory accesses logged correctly |
| G-6 (ftrace) | V-IT-4: call/return detection with symbols |
| G-7 (disasm) | V-UT-4: mnemonic formatting |
| C-1 (no deps) | Expression evaluator is self-contained |
| C-2 (ring buffer) | V-UT-1: bounded capacity |
| C-6 (reuse DECODER) | V-UT-4: all InstKind produce mnemonics |
