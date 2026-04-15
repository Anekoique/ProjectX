# `trace` PLAN `05`

> Status: Revised
> Feature: `trace`
> Iteration: `05`
> Owner: Executor
> Depends on:
> - Previous Plan: `04_PLAN.md`
> - Review: `04_REVIEW.md`
> - Master Directive: `04_MASTER.md`

---

## Summary

Resolves all 3 blockers from round 04, all 3 master directives. Clean layered design: debug context lives on `RVCore` (where hooks execute), CPU provides pass-through accessors. Watchpoints restored to expression-based evaluation in xdb. `circular-buffer` crate for auto-overwrite ring buffers. Traces split into separate files per M-002. `register_trace!` macro per M-003.

## Log

[**Feature Introduce**]

- `DebugContext` lives on `RVCore`, CPU provides `debug_ctx()`/`debug_ctx_mut()` pass-through
- Expression-based watchpoints: xdb evaluates expression after each step, compares old/new
- `circular-buffer` crate: `push_back()` auto-overwrites oldest entry
- File layout per M-002: `xdb/src/trace/{mod.rs, itrace.rs, ftrace.rs, mtrace.rs}`, `xdb/src/watchpoint.rs`
- `Trace` trait in `xdb/src/trace/mod.rs`, `register_trace!` macro per M-003
- Feature hierarchy: `debug` enables all, `itrace`/`ftrace`/`mtrace` as sub-features

[**Review Adjustments**]

- R-001 (04): `DebugContext` moved to `RVCore`. Hooks in `step()`/`load()`/`store()` access `self.debug_ctx` directly. CPU delegates via `self.core.debug_ctx()`. No cross-boundary access.
- R-002 (04): Expression-based watchpoints restored. xdb owns expression parser + evaluation. After each `CPU::step()`, xdb re-evaluates watched expressions and compares values. No address-only reduction.
- R-003 (04): `circular-buffer::CircularBuffer` with `push_back()` auto-overwrites oldest. Verified: `buf.push_back(x)` on full buffer drops oldest automatically.

[**Master Compliance**]

- M-001 (04): `DebugContext` split — breakpoints in core, traces dispatched to separate files, watchpoints in xdb. Clear layering.
- M-002 (04): `xdb/src/trace/{mod.rs, itrace.rs, ftrace.rs, mtrace.rs}`, `xdb/src/watchpoint.rs`
- M-003 (04): `debug` feature enables all. `itrace`/`ftrace`/`mtrace` as sub-features. `register_trace!` macro.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review 04 | R-001 | Accepted | DebugContext on RVCore, CPU pass-through |
| Review 04 | R-002 | Accepted | Expression-based watchpoints in xdb |
| Review 04 | R-003 | Accepted | `circular-buffer` auto-overwrite |
| Review 04 | TR-1 | Accepted | Core-owned debug state |
| Review 04 | TR-2 | Accepted | Preserve expression watchpoints |
| Master 04 | M-001 | Applied | Split DebugContext, clear layering |
| Master 04 | M-002 | Applied | File layout per directive |
| Master 04 | M-003 | Applied | Feature hierarchy + register_trace! |

---

## Spec

[**Goals**]
- G-1: Breakpoints — address-based, checked in `RVCore::step()`
- G-2: Watchpoints — expression-based, evaluated in xdb after each step
- G-3: Expression evaluator (chumsky) in xdb
- G-4: Instruction trace (itrace) — captured in `step()`, auto-overwrite ring buffer
- G-5: Memory trace (mtrace) — captured in `load()`/`store()`, auto-overwrite ring buffer
- G-6: Function trace (ftrace) — captured in `step()` after decode
- G-7: Disassembly — `Bus::read_ram(&self)` + `DECODER`

- NG-1: No ELF symbols
- NG-2: No MMIO debugger reads (documented: use mtrace)

[**Architecture**]

```
xdb/src/
 ├─ cli.rs                 — preprocess_line() + clap Commands
 ├─ cmd.rs                 — command dispatch
 ├─ expr.rs                — chumsky expression parser + evaluator (NEW)
 ├─ watchpoint.rs          — Watchpoint struct, expr eval + compare (NEW)
 ├─ fmt.rs                 — format_mnemonic(), register/memory display (NEW)
 ├─ trace/
 │   ├─ mod.rs             — Trace trait, TraceManager, register_trace! (NEW)
 │   ├─ itrace.rs          — ITrace: instruction trace (NEW)
 │   ├─ ftrace.rs          — FTrace: function trace (NEW)
 │   └─ mtrace.rs          — MTrace: memory trace (NEW)
 └─ main.rs                — respond() catches DebugBreak

xcore/src/
 ├─ cpu/debug.rs           — DebugContext (bp + trace rings), DebugOps trait (NEW)
 ├─ cpu/riscv/debug.rs     — impl DebugOps for RVCore (NEW)
 ├─ cpu/riscv/mod.rs       — RVCore gains debug_ctx field, step() hooks
 ├─ cpu/mod.rs             — CPU pass-through: debug_ctx(), debug_ctx_mut()
 └─ error.rs               — XError::DebugBreak(usize)
```

**Layering:**
```
┌─────────────────────────────────────────────┐
│ xdb                                         │
│  watchpoint.rs  — expression wp (owns eval) │
│  trace/         — TraceManager, display     │
│  expr.rs        — chumsky parser            │
├─────────────────────────────────────────────┤
│ xcore::CPU<Core>                            │
│  debug_ctx()    — pass-through to core      │
│  step()/run()   — unchanged                 │
├─────────────────────────────────────────────┤
│ xcore::RVCore                               │
│  debug_ctx      — DebugContext (bp + traces) │
│  step()         — bp check + trace capture  │
│  load()/store() — mtrace capture            │
└─────────────────────────────────────────────┘
```

**Key insight**: Breakpoints and trace capture live in xcore (where per-instruction hooks execute). Watchpoints and expression evaluation live in xdb (where user-facing logic belongs). Clean split.

[**Invariants**]
- I-1: `DebugContext` lives on `RVCore`. CPU delegates via `self.core`.
- I-2: Breakpoints checked in `RVCore::step()` before execute.
- I-3: Traces captured in `RVCore::step()` (itrace/ftrace) and `load()`/`store()` (mtrace).
- I-4: Watchpoints evaluated in xdb after `CPU::step()` returns. Not in xcore.
- I-5: `skip_bp_once` prevents re-trigger after breakpoint hit.
- I-6: `circular-buffer::push_back()` auto-overwrites oldest on full.
- I-7: All debug code behind `cfg(feature = "debug")`.
- I-8: Debugger memory reads: `Bus::read_ram(&self)`, RAM only.

[**Data Structure**]

```rust
// ═══ xcore/src/cpu/debug.rs ═══ cfg(feature = "debug")

use circular_buffer::CircularBuffer;
use crate::error::XResult;

/// Arch-agnostic debug facade.
pub trait DebugOps {
    fn read_register(&self, name: &str) -> Option<u64>;
    fn dump_registers(&self) -> Vec<(&'static str, u64)>;
    fn disasm_raw(&self, raw: u32) -> String;
}

/// Trace entry types
pub struct ITraceEntry {
    pub pc: usize,
    pub raw: u32,
    pub mnemonic: String,
}

pub struct MTraceEntry {
    pub pc: usize,
    pub addr: usize,
    pub size: usize,
    pub op: u8,      // b'R' or b'W'
    pub value: u64,
}

pub struct FTraceEntry {
    pub pc: usize,
    pub target: usize,
    pub kind: u8,    // b'C' or b'R'
    pub depth: usize,
}

/// Core-owned debug context. Holds breakpoints + trace ring buffers.
/// Configured by xdb via CPU pass-through accessors.
pub struct DebugContext {
    // Breakpoints
    pub breakpoints: std::collections::BTreeSet<usize>,
    pub skip_bp_once: bool,

    // Trace ring buffers (boxed for heap allocation)
    pub itrace: Option<Box<CircularBuffer<128, ITraceEntry>>>,
    pub ftrace: Option<Box<CircularBuffer<128, FTraceEntry>>>,
    pub mtrace: Option<Box<CircularBuffer<256, MTraceEntry>>>,
    pub ftrace_depth: usize,
}

impl DebugContext {
    pub fn new() -> Self {
        Self {
            breakpoints: BTreeSet::new(),
            skip_bp_once: false,
            itrace: None,
            ftrace: None,
            mtrace: None,
            ftrace_depth: 0,
        }
    }

    // Breakpoint API
    pub fn add_breakpoint(&mut self, addr: usize) { self.breakpoints.insert(addr); }
    pub fn remove_breakpoint(&mut self, addr: usize) -> bool { self.breakpoints.remove(&addr) }

    // Trace enable/disable
    pub fn enable_itrace(&mut self) {
        self.itrace = Some(Box::new(CircularBuffer::new()));
    }
    pub fn enable_ftrace(&mut self) {
        self.ftrace = Some(Box::new(CircularBuffer::new()));
    }
    pub fn enable_mtrace(&mut self) {
        self.mtrace = Some(Box::new(CircularBuffer::new()));
    }
    pub fn disable_all_traces(&mut self) {
        self.itrace = None;
        self.ftrace = None;
        self.mtrace = None;
    }
}

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
        let mut regs = vec![("pc", self.pc.as_usize() as u64)];
        for i in 0u8..32 {
            let reg = RVReg::try_from(i).unwrap();
            regs.push((reg.name(), self.gpr[i as usize]));
        }
        regs
    }

    fn disasm_raw(&self, raw: u32) -> String {
        match DECODER.decode(raw) {
            Ok(inst) => format_mnemonic(&inst),
            Err(_) => format!("???  ({:#010x})", raw),
        }
    }
}

/// Map GPR ABI name to index. Returns None for unknown names.
fn gpr_name_to_idx(name: &str) -> Option<usize> {
    RVReg::try_from_name(name).map(|r| r as usize)
}

/// Map CSR name to address. Returns None for unknown names.
fn csr_name_to_addr(name: &str) -> Option<u16> {
    CsrAddr::from_name(name).map(|a| a as u16)
}

// ═══ xcore/src/cpu/riscv/mod.rs ═══ step() hooks

impl CoreOps for RVCore {
    fn step(&mut self) -> XResult {
        // ... existing bus.tick(), sync_interrupts, check_pending_interrupts ...

        // ── breakpoint check ──
        #[cfg(feature = "debug")]
        if let Some(ref mut ctx) = self.debug_ctx {
            if !ctx.skip_bp_once && ctx.breakpoints.contains(&self.pc.as_usize()) {
                return Err(XError::DebugBreak(self.pc.as_usize()));
            }
            ctx.skip_bp_once = false;
        }

        // ── existing fetch → decode → execute ──
        self.trap_on_err(|core| {
            let raw = core.fetch()?;
            let inst = core.decode(raw)?;

            // ── itrace capture ──
            #[cfg(feature = "debug")]
            if let Some(ref mut ctx) = core.debug_ctx {
                if let Some(ref mut it) = ctx.itrace {
                    it.push_back(ITraceEntry {
                        pc: core.pc.as_usize(),
                        raw,
                        mnemonic: format_mnemonic(&inst),
                    });
                }
                // ── ftrace capture ──
                if let Some(ref mut ft) = ctx.ftrace {
                    detect_call_return(core.pc.as_usize(), &inst, &mut ctx.ftrace_depth, ft);
                }
            }

            core.execute(inst)
        })?;

        self.retire();
        Ok(())
    }
}

// ── mtrace hooks in load/store (inside mm.rs) ──
// pub(super) fn load(...) → after successful bus read:
#[cfg(feature = "debug")]
if let Some(ref mut ctx) = self.debug_ctx {
    if let Some(ref mut mt) = ctx.mtrace {
        mt.push_back(MTraceEntry {
            pc: self.pc.as_usize(), addr, size, op: b'R', value: val as u64,
        });
    }
}

// ═══ xcore/src/cpu/mod.rs ═══ CPU pass-through

impl<Core: CoreOps> CPU<Core> {
    #[cfg(feature = "debug")]
    pub fn debug_ctx(&self) -> Option<&DebugContext> {
        self.core.debug_ctx()  // requires CoreOps extension or direct access
    }

    #[cfg(feature = "debug")]
    pub fn debug_ctx_mut(&mut self) -> Option<&mut DebugContext> {
        self.core.debug_ctx_mut()
    }

    #[cfg(feature = "debug")]
    pub fn debug_ops(&self) -> &dyn DebugOps {
        &self.core  // Core: DebugOps
    }

    #[cfg(feature = "debug")]
    pub fn enable_debug(&mut self) {
        self.core.enable_debug();
    }
}

// CoreOps extension for debug access:
#[cfg(feature = "debug")]
pub trait CoreDebugOps: CoreOps {
    fn debug_ctx(&self) -> Option<&DebugContext>;
    fn debug_ctx_mut(&mut self) -> Option<&mut DebugContext>;
    fn enable_debug(&mut self);
}

// ═══ xdb/src/watchpoint.rs ═══

pub struct Watchpoint {
    pub id: u32,
    pub expr_text: String,
    pub prev_value: Option<u64>,
}

pub struct WatchManager {
    watchpoints: Vec<Watchpoint>,
    next_id: u32,
}

impl WatchManager {
    pub fn add(&mut self, expr: String) -> u32 { /* ... */ }
    pub fn remove(&mut self, id: u32) -> bool { /* ... */ }

    /// Evaluate all watchpoints. Returns first triggered (id, old, new).
    pub fn check<F>(&mut self, eval: F) -> Option<(u32, String, u64, u64)>
    where F: Fn(&str) -> Option<u64>
    {
        for wp in &mut self.watchpoints {
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

// ═══ xdb/src/trace/mod.rs ═══

pub mod itrace;
pub mod ftrace;
pub mod mtrace;

/// Trait abstracting trace display behavior.
pub trait Trace {
    fn name(&self) -> &'static str;
    fn display(&self);
    fn clear(&mut self);
}

/// Register a trace type by enabling it in DebugContext.
macro_rules! register_trace {
    ($ctx:expr, itrace) => { $ctx.enable_itrace() };
    ($ctx:expr, ftrace) => { $ctx.enable_ftrace() };
    ($ctx:expr, mtrace) => { $ctx.enable_mtrace() };
}
pub(crate) use register_trace;

/// Display all enabled traces.
pub fn show_traces(ctx: &DebugContext) {
    if let Some(ref it) = ctx.itrace {
        itrace::display(it);
    }
    if let Some(ref ft) = ctx.ftrace {
        ftrace::display(ft);
    }
    if let Some(ref mt) = ctx.mtrace {
        mtrace::display(mt);
    }
}

// ═══ xdb/src/trace/itrace.rs ═══
pub fn display(buf: &CircularBuffer<128, ITraceEntry>) {
    println!("=== Instruction Trace ({} entries) ===", buf.len());
    for entry in buf.iter() {
        println!("  {:#010x}: {:08x}  {}", entry.pc, entry.raw, entry.mnemonic);
    }
}

// ═══ xdb/src/trace/ftrace.rs ═══
pub fn display(buf: &CircularBuffer<128, FTraceEntry>) {
    println!("=== Function Trace ({} entries) ===", buf.len());
    for entry in buf.iter() {
        let indent = "  ".repeat(entry.depth);
        let arrow = if entry.kind == b'C' { "call" } else { "ret " };
        println!("  {:#010x}: {}{} → {:#010x}", entry.pc, indent, arrow, entry.target);
    }
}

// ═══ xdb/src/trace/mtrace.rs ═══
pub fn display(buf: &CircularBuffer<256, MTraceEntry>) {
    println!("=== Memory Trace ({} entries) ===", buf.len());
    for entry in buf.iter() {
        let op = if entry.op == b'R' { "R" } else { "W" };
        println!("  {:#010x}: {} [{:#x}+{}] = {:#x}",
            entry.pc, op, entry.addr, entry.size, entry.value);
    }
}
```

[**API Surface**]

xdb commands (same as round 04, all valid clap shapes):

```
s [N]           — step N instructions
c               — continue until bp/wp/exit
x/Ni [addr]     — disassemble (pre-parser → x -f i -n N addr)
x/Nx [addr]     — examine memory as hex
b <addr>        — set breakpoint
bd <n>          — delete breakpoint
bl              — list breakpoints
w <expr>        — watch expression for value change
wd <n>          — delete watchpoint
wl              — list watchpoints
p <expr>        — evaluate and print
info reg [name] — register dump
trace itrace    — enable itrace
trace ftrace    — enable ftrace
trace mtrace    — enable mtrace
trace show      — display all traces
trace off       — disable all
```

xdb expression grammar (chumsky):
```
expr     = compare
compare  = arith (("==" | "!=") arith)?
arith    = term (('+' | '-') term)*
term     = unary (('*' | '/') unary)*
unary    = '*' unary | '-' unary | atom
atom     = '$' REGISTER | "0x" HEX | DECIMAL | '(' expr ')'
```

**Watchpoint flow (expression-based, evaluated in xdb):**
```
1. User: w $a0 + 4
2. xdb: parse expr, evaluate to get initial value, store Watchpoint { expr, prev_value }
3. After each CPU::step():
   xdb calls watch_mgr.check(|expr| eval_expr(expr, cpu))
   If value changed → print diagnostic, return to prompt
```

**Feature hierarchy (Cargo.toml):**
```toml
[features]
debug = ["itrace", "ftrace", "mtrace"]  # enable all debug features
itrace = ["circular-buffer"]
ftrace = ["circular-buffer"]
mtrace = ["circular-buffer"]
```

[**Constraints**]
- C-1: `chumsky` for expression parsing in xdb
- C-2: `circular-buffer` for trace ring buffers (auto-overwrite on `push_back()`)
- C-3: Debugger reads: `Bus::read_ram(&self)`, RAM only
- C-4: `cfg(feature = "debug")` for all debug code
- C-5: No `debug_step`/`debug_run`. bp check in `step()`, wp check in xdb
- C-6: Watchpoints: expression-based in xdb, not address-based in xcore
- C-7: `skip_bp_once` for step-after-breakpoint
- C-8: Pre-parser: `Regex::new()` + `OnceLock`
- C-9: ftrace patterns: `jal rd=x1`, `jalr rd=x1`, `c.jalr`, `ret`, `c.jr x1` (no `c.jal`)

---

## Implement

### Implementation Plan

[**Phase 1: xcore debug infrastructure**]
- `xcore/Cargo.toml` — features: `debug`, `itrace`, `ftrace`, `mtrace`; dep `circular-buffer`
- `xcore/src/cpu/debug.rs` — `DebugContext`, `DebugOps` trait, `CoreDebugOps` trait, entry types
- `xcore/src/cpu/riscv/debug.rs` — `impl DebugOps for RVCore`, `impl CoreDebugOps for RVCore`, helpers
- `xcore/src/error.rs` — `XError::DebugBreak(usize)`
- `xcore/src/cpu/riscv/mod.rs` — `debug_ctx: Option<DebugContext>` field, cfg hooks in `step()`
- `xcore/src/cpu/mod.rs` — CPU pass-through accessors

[**Phase 2: xdb breakpoints + examine + info**]
- `xdb/Cargo.toml` — deps: `regex`, `chumsky`; enable xcore `debug`
- `xdb/src/cli.rs` — `preprocess_line()`, new Commands enum
- `xdb/src/cmd.rs` — `cmd_break*`, `cmd_examine`, `cmd_info`
- `xdb/src/fmt.rs` — `format_mnemonic()`, register/memory display
- `xdb/src/main.rs` — `respond()` catches `DebugBreak`, sets `skip_bp_once`, calls `enable_debug()`

[**Phase 3: Expression evaluator + watchpoints**]
- `xdb/src/expr.rs` — chumsky parser + evaluator
- `xdb/src/watchpoint.rs` — `WatchManager`, `cmd_watch*`
- Watchpoint check after each step in `respond()` / command handlers

[**Phase 4: Traces**]
- `xdb/src/trace/mod.rs` — `Trace` trait, `register_trace!`, `show_traces()`
- `xdb/src/trace/itrace.rs` — display formatting
- `xdb/src/trace/ftrace.rs` — display + `detect_call_return()` helper
- `xdb/src/trace/mtrace.rs` — display
- cfg-gated mtrace hooks in `xcore/src/cpu/riscv/mm.rs`
- `trace` commands in xdb

---

## Validation

[**Unit Tests**]
- V-UT-1: Expression parser (literals, `$a0`, `*0x80000000`, `$sp + 4 * 2`, errors)
- V-UT-2: Mnemonic formatter (R/I/S/B/U/J/C format types)
- V-UT-3: DebugContext bp add/remove/contains
- V-UT-4: WatchManager value-change detection
- V-UT-5: CircularBuffer push_back overwrites oldest on full
- V-UT-6: Pre-parser transforms
- V-UT-7: `DebugOps::read_register` — all GPR/CSR/PC names
- V-UT-8: `detect_call_return` — jal/ret/c.jalr/c.jr patterns

[**Integration Tests**]
- V-IT-1: Breakpoint stops, step advances past (skip_bp_once)
- V-IT-2: Watchpoint `w $a0` triggers when a0 changes
- V-IT-3: itrace captures during `continue`
- V-IT-4: ftrace detects jal/ret
- V-IT-5: `x/5i 0x80000000` disassembles correctly

[**Edge Cases**]
- V-E-1: Step at breakpoint advances one instruction
- V-E-2: Watchpoint on invalid expression → parse error, no crash
- V-E-3: `x/4x 0x02000000` → "address not in RAM"
- V-E-4: Compressed instruction disassembly
- V-E-5: Traces disabled → zero overhead (Option::None, no allocation)

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (breakpoints) | V-IT-1 |
| G-2 (watchpoints) | V-IT-2 |
| G-3 (expr eval) | V-UT-1 |
| G-4 (itrace) | V-IT-3 |
| G-5 (mtrace) | Phase 4 manual |
| G-6 (ftrace) | V-IT-4 |
| G-7 (disasm) | V-IT-5, V-UT-2 |
| I-1 (core owns debug) | DebugContext on RVCore |
| I-4 (xdb owns wp) | WatchManager in xdb |
| I-6 (auto-overwrite) | V-UT-5 |
| M-002 (file layout) | trace/ dir with itrace/ftrace/mtrace.rs |
| M-003 (features) | debug → itrace + ftrace + mtrace |
