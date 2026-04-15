# `trace` PLAN `01`

> Status: Revised
> Feature: `trace`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `00_MASTER.md`

---

## Summary

Resolves all 4 blocking issues + 1 medium from round 00. Redesigns the architecture: all debug/trace logic lives in `xdb` (not `xcore`), xcore exposes a narrow read-only facade, debugger stops use `StopReason` instead of `XError`, memory/disasm addresses are explicitly virtual with physical fallback, and ftrace ships with raw addresses (symbol support deferred). Commands follow GDB conventions. External crates used where they reduce complexity.

## Log

[**Feature Introduce**]

- `StopReason` enum replaces `XError::BreakpointHit` — clean non-fatal stop model
- All debug state (bp, wp, traces) owned by xdb, not xcore
- xcore exposes `DebugFacade` trait: register snapshot, memory read, instruction fetch/decode
- GDB-style commands: `x/Ni addr` for disasm, `x/Nx addr` for hex memory, `p expr`, `b`, `w`
- `ringbuf` crate for trace ring buffers (per M-002)
- `chumsky` crate for expression parser (per M-002)
- ftrace uses raw addresses only (symbol resolution deferred per R-002)
- Compressed instruction forms (c.jalr, c.jr) covered in ftrace (per R-005)

[**Review Adjustments**]

- R-001: Expression evaluator + watchpoint logic moved entirely to xdb. xcore provides read-only accessors only.
- R-002: ftrace ships with raw addresses. ELF symbol loading deferred to follow-up. `kernel.mk` not changed.
- R-003: New `StopReason` enum returned from xdb's stepping loop. `terminate!` only called on real errors, never on debugger stops.
- R-004: All debugger memory/disasm use virtual addresses via `CPU::load()` path. `x/p` flag for physical-mode bypass.
- R-005: ftrace detects `jal`, `jalr`, `c.jal`, `c.jalr`, `c.jr` with link-register heuristic.

[**Master Compliance**]

- M-001: Debug/trace in xdb, not xcore. xcore trace hooks gated by `cfg(feature = "trace")`.
- M-002: `ringbuf` for ring buffers, `chumsky` for expression parsing. Avoids reinventing.
- M-003: GDB-compatible commands: `x` for examine (memory + disasm), `p` for print, `info reg`.

### Changes from Previous Round

[**Added**]
- `StopReason` enum (Breakpoint, Watchpoint, Stepped)
- `DebugFacade` read-only trait on CPU
- GDB `x` command with format specifiers
- `cfg(feature = "trace")` gating
- `chumsky` expression parser
- Compressed instruction ftrace patterns

[**Changed**]
- All debug/trace state: xcore → xdb
- Watchpoint eval: xcore callback → xdb stepping loop
- `info m` → merged into `x` command (GDB style)
- ftrace: ELF symbols → raw addresses (deferred)

[**Removed**]
- `XError::BreakpointHit` / `XError::WatchpointHit`
- `xcore/src/debug/` module (moved to xdb)
- `xcore/src/trace/` module (moved to xdb)

[**Unresolved**]
- ELF symbol resolution for ftrace (deferred to follow-up round)

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Expr eval + wp logic in xdb; xcore exposes read-only facade |
| Review | R-002 | Accepted | ftrace raw addresses only; symbols deferred |
| Review | R-003 | Accepted | `StopReason` enum, no `terminate!` on debugger stops |
| Review | R-004 | Accepted | Virtual addresses by default, `/p` for physical |
| Review | R-005 | Accepted | Compressed call/return patterns in ftrace |
| Review | TR-1 | Accepted | Read-only facade from xcore |
| Review | TR-2 | Accepted | Sidecar symbols deferred |
| Master | M-001 | Applied | Debug/trace in xdb, trace hooks behind `cfg(feature)` |
| Master | M-002 | Applied | `ringbuf` + `chumsky` crates |
| Master | M-003 | Applied | GDB-style `x`, `p`, `info reg` commands |

---

## Spec

[**Goals**]
- G-1: Breakpoints — address-based, managed by xdb, checked in xdb stepping loop
- G-2: Watchpoints — expression-based value-change detection, evaluated by xdb after each step
- G-3: Expression evaluator — `chumsky`-based parser, register/memory/arithmetic
- G-4: Instruction trace (itrace) — `ringbuf` ring buffer in xdb, captured per step
- G-5: Memory trace (mtrace) — log memory accesses via xcore hook (cfg-gated)
- G-6: Function trace (ftrace) — call/return detection from decoded instructions, raw addresses
- G-7: Disassembly — `x/Ni addr` using xcore's DECODER + mnemonic formatter

- NG-1: No ELF symbol resolution this round
- NG-2: No difftest
- NG-3: No GDB remote protocol

[**Architecture**]

```
xdb (debugger frontend — owns all debug state)
 ├─ cli.rs          — clap commands (GDB-style)
 ├─ cmd.rs          — command dispatch
 ├─ expr.rs         — chumsky expression parser + evaluator (NEW)
 ├─ state.rs        — DebugState: breakpoints, watchpoints (NEW)
 ├─ trace.rs        — TraceState: itrace, mtrace, ftrace ring buffers (NEW)
 ├─ fmt.rs          — mnemonic formatter, register/memory display (NEW)
 └─ run.rs          — debug-aware stepping loop with StopReason (NEW)

xcore (emulator engine — exposes read-only facade)
 ├─ cpu/mod.rs      — StopReason enum, step() unchanged
 ├─ cpu/riscv/mod.rs — pub accessors: read_gpr(), read_csr(), read_mem(), fetch_raw()
 └─ (no debug/ or trace/ modules)
```

Execution flow:
```
xdb::run::debug_step(cpu)
  │
  ├─ check breakpoint: if cpu.pc() in bp_set → return StopReason::Breakpoint
  │
  ├─ snapshot watchpoint old values
  │
  ├─ cpu.step()  ← xcore unchanged, returns XResult
  │   ├─ fetch → decode → execute → retire
  │   └─ (optional) mtrace hook: cfg(feature="trace") logs to thread-local
  │
  ├─ capture itrace: (pc, raw, mnemonic) into ring buffer
  │
  ├─ capture ftrace: inspect decoded inst for call/return patterns
  │
  ├─ evaluate watchpoints: compare old vs new values
  │   └─ if changed → return StopReason::Watchpoint(id, old, new)
  │
  └─ return StopReason::Stepped

xdb::run::debug_continue(cpu)
  └─ loop { match debug_step(cpu) {
       Stepped => continue,
       Breakpoint | Watchpoint => break (print diagnostic, return to prompt),
       Error(e) => terminate!(e),
     }}
```

[**Invariants**]
- I-1: xcore has zero knowledge of breakpoints/watchpoints. All debug logic in xdb.
- I-2: `StopReason` is defined in xdb, not xcore. xcore's `step()` returns `XResult` as before.
- I-3: Ring buffers use `ringbuf::HeapRb` with configurable capacity.
- I-4: Trace capture happens in xdb's stepping loop, not inside xcore's step().
- I-5: mtrace is the only xcore-side hook, gated by `cfg(feature = "trace")`.
- I-6: All debugger memory reads go through virtual address translation (same as CPU). Physical mode via `/p` flag.
- I-7: Expression evaluator reads CPU state but never writes.

[**Data Structure**]

```rust
// xdb/src/run.rs
pub enum StopReason {
    Stepped,
    Breakpoint(usize),                    // pc that matched
    Watchpoint { id: u32, old: u64, new: u64, expr: String },
    ProgramExit(u32),                     // normal termination
    Error(String),                        // real error
}

// xdb/src/state.rs
pub struct DebugState {
    pub breakpoints: BTreeSet<usize>,     // sorted addresses
    pub watchpoints: Vec<Watchpoint>,
    pub next_wp_id: u32,
}

pub struct Watchpoint {
    pub id: u32,
    pub expr_text: String,
    pub prev_value: Option<u64>,
}

// xdb/src/trace.rs
use ringbuf::HeapRb;

pub struct TraceState {
    pub itrace: Option<HeapRb<ITraceEntry>>,   // None = disabled
    pub mtrace: Option<HeapRb<MTraceEntry>>,
    pub ftrace: Option<Vec<FTraceEntry>>,      // unbounded (call stack)
    pub ftrace_depth: usize,
}

pub struct ITraceEntry {
    pub pc: usize,
    pub raw: u32,
    pub mnemonic: String,
}

pub struct MTraceEntry {
    pub pc: usize,
    pub addr: usize,
    pub size: usize,
    pub op: char,          // 'R' or 'W'
    pub value: u64,
}

pub struct FTraceEntry {
    pub pc: usize,
    pub target: usize,
    pub kind: char,        // 'C' (call) or 'R' (return)
    pub depth: usize,
}
```

[**API Surface**]

xcore read-only facade (new pub methods on `CPU<Core>`):

```rust
// Register access
pub fn read_pc(&self) -> usize
pub fn read_gpr(&self, idx: u8) -> XResult<u64>
pub fn read_gpr_by_name(&self, name: &str) -> XResult<u64>
pub fn read_csr(&self, addr: u16) -> XResult<u64>
pub fn read_privilege(&self) -> u8

// Memory access (virtual, uses current MMU context)
pub fn read_mem(&self, addr: usize, size: usize) -> XResult<u64>

// Instruction access
pub fn fetch_raw_at(&self, addr: usize) -> XResult<u32>
pub fn decode_raw(&self, raw: u32) -> XResult<DecodedInst>
```

xdb expression grammar (`chumsky`):
```
expr     = logic
logic    = compare (("&&" | "||") compare)*
compare  = arith (("==" | "!=" | "<" | ">" | "<=" | ">=") arith)*
arith    = term (('+' | '-') term)*
term     = unary (('*' | '/' | '%') unary)*
unary    = '*' unary              // memory deref (8-byte read)
         | '-' unary              // negation
         | atom
atom     = '$' REGISTER           // $a0, $sp, $pc, $mstatus
         | "0x" HEX              // hex literal
         | DECIMAL               // decimal literal
         | '(' expr ')'
```

xdb GDB-style commands:

```
# Execution
s [N]                — step N instructions (default 1)
c                    — continue until bp/wp/exit
si [N]               — alias for step

# Breakpoints
b <addr>             — set breakpoint (hex addr)
b d <n>              — delete breakpoint by index
b l                  — list breakpoints
# or: break, delete, info break (GDB aliases)

# Watchpoints
w <expr>             — watch expression for value change
w d <n>              — delete watchpoint by id
w l                  — list watchpoints

# Examine / Disassemble (GDB x command)
x/Ni [addr]          — disassemble N instructions at addr (default: pc)
x/Nx [addr]          — examine N words of memory as hex
x/Nb [addr]          — examine N bytes
x [addr]             — default: x/1i addr (disassemble 1 instruction)

# Print
p <expr>             — evaluate expression and print result

# Info
info reg [name]      — print registers (all or one)

# Trace control
trace itrace [N]     — enable itrace ring buffer (default 16 entries)
trace mtrace [N]     — enable mtrace ring buffer (default 64 entries)
trace ftrace         — enable function trace
trace off            — disable all traces
trace show           — dump trace buffers

# Existing
l <file>             — load binary
r                    — reset
q                    — quit
```

[**Constraints**]
- C-1: `chumsky` parser is the expression evaluator. No custom recursive descent.
- C-2: `ringbuf` crate for ring buffers. No custom implementation.
- C-3: ftrace: raw addresses only. No ELF symbol table this round.
- C-4: mtrace hooks in xcore gated by `cfg(feature = "trace")`. Zero overhead when disabled.
- C-5: Debugger memory reads use virtual address translation. Physical mode via `x/Np addr`.
- C-6: Disassembly uses `DECODER.decode()` + custom mnemonic formatter in xdb. No LLVM/capstone.
- C-7: All addresses in commands are hex by default (like GDB). `0x` prefix optional.

---

## Implement

### Execution Flow

[**Main Flow**]
1. User types command → clap parses → dispatch to handler
2. `b addr` → add to `DebugState.breakpoints`
3. `c` → enter `debug_continue()` loop → calls `debug_step()` per iteration
4. `debug_step()`: bp check → snapshot wp → `cpu.step()` → itrace/ftrace capture → wp check → return `StopReason`
5. On `StopReason::Breakpoint` → print "Breakpoint at 0x...", return to prompt
6. On `StopReason::Watchpoint` → print "Watchpoint N: expr changed 0x.. → 0x..", return to prompt
7. `x/5i` → call `cpu.fetch_raw_at()` + `cpu.decode_raw()` for 5 instructions, format mnemonics

[**Failure Flow**]
1. Expression parse error → print error, return to prompt (no state change)
2. Memory read at unmapped addr → print "cannot access memory at 0x...", return to prompt
3. `cpu.step()` returns `XError` → `terminate!` for real errors, `StopReason::ProgramExit` for clean exit

### Implementation Plan

[**Phase 1: xcore facade + StopReason + xdb scaffolding**]

New files:
- `xdb/src/run.rs` — `StopReason`, `debug_step()`, `debug_continue()`
- `xdb/src/state.rs` — `DebugState` (empty bp/wp containers)
- `xdb/src/fmt.rs` — register display, mnemonic formatter

xcore changes:
- Add pub read-only methods to `CPU`: `read_pc()`, `read_gpr()`, `read_mem()`, `fetch_raw_at()`, `decode_raw()`
- No new modules in xcore

Wire xdb commands to use `debug_step()` / `debug_continue()` instead of raw `with_xcpu!(step())`.

[**Phase 2: Breakpoints + Info + Examine**]

- `b` / `b d` / `b l` commands
- Breakpoint check in `debug_step()` before `cpu.step()`
- `info reg` — formatted register dump (colored, ABI names)
- `x/Ni addr` — disassemble N instructions using facade
- `x/Nx addr` — examine memory as hex words
- Mnemonic formatter: `DecodedInst` → `"addi sp, sp, -16"` string

[**Phase 3: Expression evaluator + Print + Watchpoints**]

- `chumsky` parser in `xdb/src/expr.rs`
- `p <expr>` command
- `w <expr>` / `w d` / `w l` commands
- Watchpoint evaluation in `debug_step()` after `cpu.step()`

[**Phase 4: Instruction trace + Function trace**]

- `ringbuf::HeapRb<ITraceEntry>` in `TraceState`
- Capture in `debug_step()` after successful step
- ftrace: detect call/return from decoded instruction:
  - Call: `jal rd` where rd == ra (x1), `jalr rd, rs1, imm` where rd == ra, `c.jal`, `c.jalr`
  - Return: `jalr x0, ra, 0` (ret), `c.jr ra`
- `trace itrace/ftrace/show/off` commands

[**Phase 5: Memory trace (cfg-gated)**]

- Add `cfg(feature = "trace")` to xcore's Cargo.toml
- Optional mtrace hook in xcore's load/store path
- Thread-local or AtomicPtr-based log sink
- `xdb` collects mtrace entries via facade
- `trace mtrace` command

---

## Trade-offs

- T-1: **`chumsky` vs custom recursive descent** — `chumsky` is battle-tested, composable, produces good error messages. ~50 lines vs ~150 lines custom. Trade-off: adds a dependency. **Prefer chumsky** (per M-002).

- T-2: **Ring buffer: `ringbuf` vs `circular-buffer`** — `ringbuf` is most popular (1.69M downloads), supports `HeapRb` and `StaticRb`. `circular-buffer` is simpler but less flexible. **Prefer `ringbuf`** (per M-002).

- T-3: **ftrace symbols: this round vs deferred** — Current build pipeline strips ELF to raw binary. Adding symbol support requires either changing the pipeline or sidecar files. **Defer to follow-up** (per R-002).

- T-4: **mtrace: xcore hook vs xdb memory diffing** — xcore hook is precise (captures every access), xdb diffing is imprecise (misses reads). Hook needs `cfg` gating. **Prefer cfg-gated xcore hook** for accuracy.

- T-5: **Virtual vs physical memory commands** — GDB defaults to virtual. Physical access is rare but needed for MMIO debugging. **Default virtual, `/p` flag for physical** (per R-004).

---

## Validation

[**Unit Tests**]
- V-UT-1: Expression parser — literals, registers, arithmetic, memory deref, precedence, nested parens, error cases
- V-UT-2: Mnemonic formatter — all `InstKind` variants produce valid strings (R/I/S/B/U/J/C formats)
- V-UT-3: Breakpoint add/remove/contains
- V-UT-4: Watchpoint value-change detection

[**Integration Tests**]
- V-IT-1: Breakpoint stops execution at target address, then continue resumes
- V-IT-2: Watchpoint on `$a0` triggers when function writes a0
- V-IT-3: itrace captures correct instruction sequence for a known 5-instruction program
- V-IT-4: ftrace detects `jal ra` as call and `ret` as return
- V-IT-5: `x/5i 0x80000000` produces 5 valid disassembled lines

[**Edge Case Validation**]
- V-E-1: Breakpoint at current PC — triggers immediately on next continue
- V-E-2: Watchpoint on `*0xDEAD0000` — graceful error (unmapped memory)
- V-E-3: Expression `$pc + 4 * 2` — correct precedence (multiply before add)
- V-E-4: Compressed instruction disassembly — `c.addi sp, -16` correctly formatted
- V-E-5: ftrace with `c.jr ra` — correctly detected as return

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (breakpoints) | V-IT-1 |
| G-2 (watchpoints) | V-IT-2 |
| G-3 (expr eval) | V-UT-1 |
| G-4 (itrace) | V-IT-3 |
| G-5 (mtrace) | Manual: `trace mtrace` + step shows memory accesses |
| G-6 (ftrace) | V-IT-4 |
| G-7 (disasm) | V-IT-5, V-UT-2 |
| C-1 (chumsky) | Expression parser uses chumsky |
| C-2 (ringbuf) | itrace/mtrace use HeapRb |
| C-5 (virtual addr) | Memory reads translate via MMU |
| C-6 (DECODER) | Disassembly reuses DECODER |
| I-1 (xdb owns debug) | No debug state in xcore |
