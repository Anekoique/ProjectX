# `trace` PLAN `07`

> Status: Revised
> Feature: `trace`
> Iteration: `07`
> Owner: Executor
> Depends on:
> - Previous Plan: `06_PLAN.md`
> - Review: `06_REVIEW.md`
> - Master Directive: `06_MASTER.md`

---

## Summary

**Breaking scope change per M-001/M-002**: Remove all trace infrastructure (itrace/ftrace/mtrace ring buffers, DebugHook callbacks, TraceManager). Replace with proper `log!()` at different levels in xcore's execution path. Focus entirely on **improving xdb**: breakpoints, watchpoints, expression evaluator, disassembly, register/memory inspection.

## Log

[**Feature Introduce**]

- **Traces replaced by log levels**: `trace!()` for instruction execution, `debug!()` for memory access, `info!()` for control flow events (call/return). Users control visibility via `LOG=trace/debug/info`.
- **Focused xdb scope**: breakpoints, watchpoints (expression-based), `p expr`, `x/Ni`, `x/Nx`, `info reg`
- **Clean xcore surface**: `DebugOps` trait for read-only inspection. Breakpoints on RVCore with `cfg(feature = "debug")`. No hook/callback/adapter complexity.
- **xdb-driven step loop**: watchpoints checked per-step in xdb

[**Review Adjustments**]

- R-001 (06): Hook ownership problem eliminated — no hooks. Traces are plain `log!()` calls.
- R-002 (06): ftrace payload problem eliminated — control flow logged directly in `execute()` with resolved target.
- R-003 (06): Concrete public API defined — `CPU` exposes debug methods through `with_xcpu!`.

[**Master Compliance**]

- M-001 (06): All traces removed. Replaced by `log!()` at `trace`/`debug` levels in xcore execution path.
- M-002 (06): Focus on improving xdb with breakpoints, watchpoints, expression eval, disasm, info commands.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review 06 | R-001 | N/A | Hook system removed |
| Review 06 | R-002 | N/A | ftrace removed, replaced by log |
| Review 06 | R-003 | Accepted | Concrete public API defined below |
| Master 06 | M-001 | Applied | Traces → log levels |
| Master 06 | M-002 | Applied | Focus on xdb improvements |

---

## Spec

[**Goals**]
- G-1: Breakpoints — address-based, checked in `step()`, skip-on-resume
- G-2: Watchpoints — expression-based, xdb step loop
- G-3: Expression evaluator (chumsky) — `$reg`, `*addr`, arithmetic
- G-4: Disassembly — `x/Ni addr` using `DebugOps`
- G-5: Memory examine — `x/Nx addr` using `DebugOps`
- G-6: Register inspect — `info reg [name]`
- G-7: Execution logging — `trace!()` per instruction, `debug!()` per memory access

- NG-1: No ring buffer traces (replaced by log)
- NG-2: No DebugHook callbacks
- NG-3: No ELF symbols

[**Architecture**]

```
xdb/src/
 ├─ cli.rs           — preprocess_line() + clap Commands
 ├─ cmd.rs           — command dispatch + handlers
 ├─ expr.rs          — chumsky expression parser (NEW)
 ├─ watchpoint.rs    — WatchManager (NEW)
 └─ main.rs          — respond() catches DebugBreak, wp step loop

xcore/src/
 ├─ cpu/debug.rs     — DebugOps trait, format_mnemonic() (NEW, cfg=debug)
 ├─ cpu/riscv/debug.rs — impl DebugOps for RVCore (NEW, cfg=debug)
 ├─ cpu/riscv/mod.rs — breakpoints field + check in step(), execution log
 ├─ cpu/mod.rs       — CPU debug pass-through methods
 └─ error.rs         — XError::DebugBreak
```

**Layering (simple, no callbacks):**
```
┌─────────────────────────────────────────────┐
│ xdb (command frontend)                      │
│  expr.rs       — chumsky parser             │
│  watchpoint.rs — expression wp + step loop  │
│  cmd.rs        — break/examine/info/print   │
├─────────────────────────────────────────────┤
│ xcore::CPU<Core> (public debug API)         │
│  add_breakpoint(), remove_breakpoint()      │
│  skip_bp_once(), debug_ops()                │
│  state(), pc() (exposed for xdb queries)    │
├─────────────────────────────────────────────┤
│ xcore::RVCore (execution + log)             │
│  step(): bp check, trace!() per inst        │
│  load()/store(): debug!() per access        │
│  impl DebugOps: read_register, read_memory  │
└─────────────────────────────────────────────┘
```

[**Invariants**]
- I-1: No trace ring buffers, no DebugHook, no TraceManager.
- I-2: Execution observability via `log!()` at `trace`/`debug`/`info` levels.
- I-3: Breakpoints on RVCore, checked in `step()`, behind `cfg(feature = "debug")`.
- I-4: Watchpoints in xdb. xdb-driven step loop when wp active.
- I-5: `skip_bp_once` prevents re-trigger after breakpoint hit.
- I-6: Debugger reads: `Bus::read_ram(&self)`, physical RAM only.
- I-7: `DebugOps` trait behind `cfg(feature = "debug")`.

[**Data Structure**]

```rust
// ═══ xcore/src/cpu/debug.rs ═══ cfg(feature = "debug")

use crate::{config::Word, error::XResult, isa::DecodedInst};

/// Arch-agnostic read-only debug facade.
pub trait DebugOps {
    fn read_register(&self, name: &str) -> Option<u64>;
    fn dump_registers(&self) -> Vec<(&'static str, u64)>;
    fn read_memory(&self, paddr: usize, size: usize) -> XResult<u64>;
    fn fetch_inst(&self, paddr: usize) -> XResult<u32>;
    fn disasm_raw(&self, raw: u32) -> String;
}

/// Format a decoded instruction into a human-readable mnemonic.
pub fn format_mnemonic(inst: &DecodedInst) -> String {
    // e.g., "addi sp, sp, -16", "jal ra, 0x80001000"
    // Matches all InstKind variants for R/I/S/B/U/J/C formats
    todo!()
}

// ═══ xcore/src/cpu/riscv/mod.rs ═══ additions

use std::collections::BTreeSet;

pub struct RVCore {
    // ... existing fields ...
    #[cfg(feature = "debug")]
    breakpoints: BTreeSet<usize>,
    #[cfg(feature = "debug")]
    skip_bp_once: bool,
}

impl CoreOps for RVCore {
    fn step(&mut self) -> XResult {
        // existing: bus.tick(), sync_interrupts, check_pending_interrupts

        // ── breakpoint check ──
        #[cfg(feature = "debug")]
        {
            if !self.skip_bp_once
                && self.breakpoints.contains(&self.pc.as_usize())
            {
                return Err(XError::DebugBreak(self.pc.as_usize()));
            }
            self.skip_bp_once = false;
        }

        // existing: fetch → decode → execute
        self.trap_on_err(|core| {
            let raw = core.fetch()?;
            let inst = core.decode(raw)?;

            // ── execution trace log ──
            trace!(
                "  {:#010x}: {:08x}  {}",
                core.pc.as_usize(),
                raw,
                format_mnemonic(&inst),
            );

            core.execute(inst)
        })?;

        self.retire();
        Ok(())
    }
}

// ── memory access logging in mm.rs ──
// After successful load:
debug!("  R [{:#x}+{}] = {:#x}", addr, size, val);
// After successful store:
debug!("  W [{:#x}+{}] = {:#x}", addr, size, val);

// ═══ xcore/src/cpu/riscv/debug.rs ═══ cfg(feature = "debug")

impl DebugOps for RVCore {
    fn read_register(&self, name: &str) -> Option<u64> {
        match name {
            "pc" => Some(self.pc.as_usize() as u64),
            "privilege" => Some(self.privilege as u64),
            _ => gpr_name_to_idx(name).map(|i| self.gpr[i])
                .or_else(|| csr_name_to_addr(name).map(|a| self.csr.get_by_addr(a)))
        }
    }

    fn dump_registers(&self) -> Vec<(&'static str, u64)> {
        let mut out = vec![("pc", self.pc.as_usize() as u64)];
        for i in 0u8..32 {
            let r = RVReg::try_from(i).unwrap();
            out.push((r.name(), self.gpr[i as usize]));
        }
        out
    }

    fn read_memory(&self, paddr: usize, size: usize) -> XResult<u64> {
        self.bus.lock().unwrap().read_ram(paddr, size).map(|v| v as u64)
    }

    fn fetch_inst(&self, paddr: usize) -> XResult<u32> {
        let bus = self.bus.lock().unwrap();
        let lo = bus.read_ram(paddr, 2)? as u32;
        if lo & 0x3 != 0x3 { return Ok(lo & 0xFFFF); }
        let hi = bus.read_ram(paddr + 2, 2)? as u32;
        Ok(lo | (hi << 16))
    }

    fn disasm_raw(&self, raw: u32) -> String {
        match DECODER.decode(raw) {
            Ok(inst) => format_mnemonic(&inst),
            Err(_) => format!("???  ({:#010x})", raw),
        }
    }
}

fn gpr_name_to_idx(name: &str) -> Option<usize> { /* ... */ }
fn csr_name_to_addr(name: &str) -> Option<u16> { /* ... */ }

// ═══ xcore/src/cpu/mod.rs ═══ public debug API

impl<Core: CoreOps> CPU<Core> {
    pub fn pc(&self) -> usize { self.core.pc().as_usize() }
    pub fn is_terminated(&self) -> bool { self.state.is_terminated() }

    #[cfg(feature = "debug")]
    pub fn add_breakpoint(&mut self, addr: usize) {
        self.core.add_breakpoint(addr);
    }

    #[cfg(feature = "debug")]
    pub fn remove_breakpoint(&mut self, idx: usize) -> bool {
        self.core.remove_breakpoint(idx)
    }

    #[cfg(feature = "debug")]
    pub fn list_breakpoints(&self) -> Vec<usize> {
        self.core.list_breakpoints()
    }

    #[cfg(feature = "debug")]
    pub fn skip_bp_once(&mut self) {
        self.core.skip_bp_once();
    }
}

// For DebugOps, bound on Core:
impl<Core: CoreOps + DebugOps> CPU<Core> {
    pub fn debug_ops(&self) -> &dyn DebugOps {
        &self.core
    }
}

// ═══ xcore/src/error.rs ═══

#[cfg(feature = "debug")]
DebugBreak(usize),

// ═══ xdb/src/watchpoint.rs ═══

pub struct Watchpoint {
    pub id: u32,
    pub expr_text: String,
    pub prev_value: Option<u64>,
}

pub struct WatchManager {
    wps: Vec<Watchpoint>,
    next_id: u32,
}

impl WatchManager {
    pub fn new() -> Self { Self { wps: Vec::new(), next_id: 1 } }
    pub fn is_empty(&self) -> bool { self.wps.is_empty() }

    pub fn add(&mut self, expr: String, init_val: Option<u64>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.wps.push(Watchpoint { id, expr_text: expr, prev_value: init_val });
        id
    }

    pub fn remove(&mut self, id: u32) -> bool {
        if let Some(pos) = self.wps.iter().position(|w| w.id == id) {
            self.wps.remove(pos);
            true
        } else { false }
    }

    pub fn list(&self) -> &[Watchpoint] { &self.wps }

    /// Check all watchpoints. Returns first triggered (id, expr, old, new).
    pub fn check(&mut self, eval: impl Fn(&str) -> Option<u64>)
        -> Option<(u32, String, u64, u64)>
    {
        for wp in &mut self.wps {
            let new_val = eval(&wp.expr_text);
            if wp.prev_value != new_val {
                let old = wp.prev_value.unwrap_or(0);
                let new = new_val.unwrap_or(0);
                let expr = wp.expr_text.clone();
                wp.prev_value = new_val;
                return Some((wp.id, expr, old, new));
            }
        }
        None
    }
}

// ═══ xdb/src/expr.rs ═══ (chumsky)

// Grammar:
//   expr    = compare
//   compare = arith (("==" | "!=") arith)?
//   arith   = term (('+' | '-') term)*
//   term    = unary (('*' | '/') unary)*
//   unary   = '*' unary | '-' unary | atom
//   atom    = '$' REG | "0x" HEX | DECIMAL | '(' expr ')'

/// Evaluate expression given a register/memory read function.
pub fn eval_expr(
    expr: &str,
    read_reg: &impl Fn(&str) -> Option<u64>,
    read_mem: &impl Fn(usize, usize) -> Option<u64>,
) -> Result<u64, String> {
    let ast = parse(expr)?;
    evaluate(&ast, read_reg, read_mem)
}
```

[**API Surface**]

xdb commands (GDB-style):
```
s [N]           — step N instructions (default 1)
c               — continue until bp/wp/exit
x/Ni [addr]     — disassemble N instructions at addr (default: pc)
x/Nx [addr]     — examine N hex words at addr
x/Nb [addr]     — examine N bytes at addr
b <addr>        — set breakpoint
bd <n>          — delete breakpoint by index
bl              — list breakpoints
w <expr>        — watch expression for value change
wd <n>          — delete watchpoint
wl              — list watchpoints
p <expr>        — evaluate and print expression
info reg [name] — register dump (all or one)
l <file>        — load binary
r               — reset
q               — quit
```

Pre-parser:
```rust
fn preprocess_line(line: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^x/(\d+)?([ixbd])\s*(.*)").unwrap());
    if let Some(caps) = re.captures(line.trim()) {
        let n = caps.get(1).map_or("1", |m| m.as_str());
        let f = &caps[2];
        let rest = caps.get(3).map_or("", |m| m.as_str()).trim();
        format!("x -f {f} -n {n} {rest}")
    } else {
        line.to_string()
    }
}
```

Logging levels (in xcore execution path):
```
LOG=trace   → per-instruction: "  0x80000000: 00000297  auipc t0, 0"
LOG=debug   → per-memory-access: "  R [0x80001000+4] = 0xdeadbeef"
LOG=info    → lifecycle events (load, reset, trap, halt)
LOG=warn    → warnings only
LOG=off     → silent (batch mode default)
```

xdb command flow:
```rust
fn cmd_continue(watch_mgr: &mut WatchManager) -> XResult {
    with_xcpu(|cpu| {
        if watch_mgr.is_empty() {
            // Fast path: no watchpoints
            return cpu.run(u64::MAX);
        }
        // Slow path: step loop for watchpoint eval
        loop {
            cpu.step()?;
            if cpu.is_terminated() { break; }
            let eval = |expr: &str| {
                let read_reg = |name: &str| cpu.debug_ops().read_register(name);
                let read_mem = |addr, sz| cpu.debug_ops().read_memory(addr, sz).ok();
                eval_expr(expr, &read_reg, &read_mem).ok()
            };
            if let Some((id, expr, old, new)) = watch_mgr.check(eval) {
                println!("Watchpoint {id}: {expr} changed {old:#x} → {new:#x}");
                return Ok(());
            }
        }
        Ok(())
    })
}

fn respond(line: &str, watch_mgr: &mut WatchManager) -> Result<bool, String> {
    let line = preprocess_line(line);
    // ... clap parse + dispatch ...
    .or_else(|e| match e {
        #[cfg(feature = "debug")]
        XError::DebugBreak(pc) => {
            with_xcpu(|cpu| cpu.skip_bp_once());
            println!("Breakpoint at {pc:#x}");
            Ok(true)
        }
        _ => { terminate!(e); Ok(true) }
    })
}
```

[**Constraints**]
- C-1: `chumsky` for expression parsing
- C-2: `regex` for pre-parser
- C-3: Debugger reads: `Bus::read_ram(&self)`, physical RAM only
- C-4: `cfg(feature = "debug")` for breakpoints, DebugOps
- C-5: Watchpoints: expression-based in xdb, step loop when active
- C-6: Traces: `log!()` at trace/debug levels, no ring buffers
- C-7: `skip_bp_once` for step-after-breakpoint
- C-8: Pre-parser: `Regex::new()` + `OnceLock`

---

## Implement

### Implementation Plan

[**Phase 1: xcore debug surface**]
- `xcore/Cargo.toml` — `debug` feature
- `xcore/src/cpu/debug.rs` — `DebugOps` trait, `format_mnemonic()`
- `xcore/src/cpu/riscv/debug.rs` — `impl DebugOps`, helper fns
- `xcore/src/cpu/riscv/mod.rs` — `breakpoints`/`skip_bp_once` fields, bp check in `step()`, `trace!()` per instruction
- `xcore/src/cpu/riscv/mm.rs` — `debug!()` per memory access
- `xcore/src/cpu/mod.rs` — public debug methods: `add_breakpoint()`, `remove_breakpoint()`, `list_breakpoints()`, `skip_bp_once()`, `debug_ops()`, `pc()`, `is_terminated()`
- `xcore/src/error.rs` — `XError::DebugBreak(usize)`

[**Phase 2: xdb commands**]
- `xdb/Cargo.toml` — `regex`, `chumsky`; enable xcore `debug`
- `xdb/src/cli.rs` — `preprocess_line()`, expanded `Commands` enum
- `xdb/src/cmd.rs` — `cmd_break*`, `cmd_examine`, `cmd_info`, `cmd_print`
- `xdb/src/main.rs` — `respond()` catches `DebugBreak`, creates `WatchManager`

[**Phase 3: Expression evaluator + watchpoints**]
- `xdb/src/expr.rs` — chumsky parser + evaluator
- `xdb/src/watchpoint.rs` — `WatchManager`
- Modified `cmd_continue`/`cmd_step` with wp step loop
- `cmd_watch*`, `cmd_print`

---

## Validation

[**Unit Tests**]
- V-UT-1: Expression parser (literals, `$a0`, `*0x80000000`, `$sp + 4 * 2`, precedence, errors)
- V-UT-2: `format_mnemonic()` — all InstKind variants → valid string
- V-UT-3: Breakpoint add/remove/list
- V-UT-4: WatchManager value-change detection
- V-UT-5: Pre-parser transforms (`x/5i addr` → `x -f i -n 5 addr`)
- V-UT-6: `DebugOps::read_register` — GPR/CSR/PC names
- V-UT-7: `DebugOps::read_memory` — valid RAM / invalid → error

[**Integration Tests**]
- V-IT-1: Breakpoint stops at address, step advances past (skip_bp_once)
- V-IT-2: Watchpoint `w $a0` triggers on value change during continue
- V-IT-3: `x/5i 0x80000000` disassembles 5 instructions
- V-IT-4: `x/4x 0x80000000` shows 4 hex words
- V-IT-5: `LOG=trace make run` shows per-instruction output

[**Edge Cases**]
- V-E-1: Step at breakpoint — advances one instruction
- V-E-2: Watchpoint on invalid expression → parse error
- V-E-3: `x/4x 0x02000000` → "not in RAM"
- V-E-4: Continue without watchpoints → fast `run()` path
- V-E-5: `LOG=off` → no execution logging overhead

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (breakpoints) | V-IT-1 |
| G-2 (watchpoints) | V-IT-2 |
| G-3 (expr eval) | V-UT-1 |
| G-4 (disasm) | V-IT-3 |
| G-5 (memory) | V-IT-4 |
| G-6 (registers) | V-UT-6 |
| G-7 (logging) | V-IT-5 |
| I-1 (no ring buffers) | No trace infrastructure |
| I-4 (wp step loop) | V-E-4 |
| I-6 (RAM only) | V-E-3 |
