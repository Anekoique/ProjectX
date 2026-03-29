# `trace` PLAN `06`

> Status: Revised
> Feature: `trace`
> Iteration: `06`
> Owner: Executor
> Depends on:
> - Previous Plan: `05_PLAN.md`
> - Review: `05_REVIEW.md`
> - Master Directive: `05_MASTER.md`

---

## Summary

Resolves all 3 blockers + 2 master directives. The core design shift: xcore defines a **`DebugHook` callback trait** (the interface), xdb implements it (the storage + logic). xcore calls hooks during `step()`, xdb controls what happens. No trace entries in xcore. No reverse dependencies. Watchpoints use xdb-driven step loop when active.

## Log

[**Feature Introduce**]

- **`DebugHook` trait** in xcore — callback interface with `on_execute()`, `on_mem_access()`, `on_breakpoint()`. xcore defines the shape, xdb implements the behavior.
- **No TraceEntry types in xcore** — xcore passes raw values (`pc: usize, raw: u32, mnemonic: &str`) through the hook. xdb decides how to store them.
- **xdb-driven step loop** when watchpoints active — `cmd_continue` calls `step()` in a loop, checks watchpoints per iteration. When no watchpoints, delegates to `CPU::run()` as before.
- **`format_mnemonic()`** in xcore (capture-time helper) — called during `step()` before hook invocation.
- **`DebugOps` trait** expanded with `read_memory()` — single debugger facade for registers + memory + disasm.

[**Review Adjustments**]

- R-001 (05): xdb-driven step loop when watchpoints active. `cmd_continue()` checks `watch_mgr.is_empty()`: if empty → `CPU::run()` (fast path), if non-empty → loop `CPU::step()` + check watchpoints. Batch mode: no watchpoints, uses `run()`.
- R-002 (05): `format_mnemonic()` and `detect_call_return()` moved to xcore. xdb owns only display/rendering. No reverse dependency.
- R-003 (05): `DebugOps::read_memory(paddr, size) -> XResult<u64>` added. Implemented by `Bus::read_ram()` for RAM. Addresses are physical (documented).

[**Master Compliance**]

- M-001 (05): No TraceEntry in xcore. xcore defines `DebugHook` trait with `on_execute(pc, raw, &str)`, `on_mem_access(pc, addr, size, op, value)`. xdb implements the trait and stores entries in its own types.
- M-002 (05): Clean layering. Improved code quality.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review 05 | R-001 | Accepted | xdb step loop when watchpoints active |
| Review 05 | R-002 | Accepted | Capture helpers in xcore, display in xdb |
| Review 05 | R-003 | Accepted | `DebugOps::read_memory()` via `Bus::read_ram()` |
| Review 05 | TR-1 | Accepted | xdb-driven step loop |
| Review 05 | TR-2 | Accepted | Capture in xcore, display in xdb |
| Master 05 | M-001 | Applied | DebugHook trait — no entries in xcore |
| Master 05 | M-002 | Applied | Clean code |

---

## Spec

[**Goals**]
- G-1: Breakpoints — checked in `step()`, reported via `DebugHook::on_breakpoint()`
- G-2: Watchpoints — expression-based, xdb step loop
- G-3: Expression evaluator (chumsky)
- G-4: Instruction trace (itrace) — via `DebugHook::on_execute()`
- G-5: Memory trace (mtrace) — via `DebugHook::on_mem_access()`
- G-6: Function trace (ftrace) — via `DebugHook::on_execute()` (xdb detects call/return from decoded info)
- G-7: Disassembly — `DebugOps::read_memory()` + `DebugOps::disasm_raw()`

[**Architecture**]

```
xcore defines interfaces:
  DebugOps   — read-only facade (registers, memory, disasm)
  DebugHook  — callback interface (on_execute, on_mem_access, on_breakpoint)

xdb implements:
  DebugHook  → stores trace entries in circular buffers
  WatchManager → expression evaluation + step loop
  TraceManager → display formatting
```

```
┌──────────────────────────────────────────────────────────┐
│ xdb (implements DebugHook, owns storage + watchpoints)   │
│                                                          │
│  impl DebugHook for XdbHook {                           │
│    on_execute(pc,raw,mn)  → itrace_buf.push_back(...)   │
│                           → detect call/ret → ftrace    │
│    on_mem_access(...)     → mtrace_buf.push_back(...)   │
│    on_breakpoint(pc)      → set stop flag               │
│  }                                                       │
│                                                          │
│  WatchManager  — expr eval after each step               │
│  TraceManager  — display + trace on/off                  │
├──────────────────────────────────────────────────────────┤
│ xcore::CPU<Core>                                         │
│  run(count)         — unchanged (fast path, no wp)       │
│  step()             → calls core.step()                  │
│  debug_ops()        → &dyn DebugOps                      │
│  set_debug_hook()   → Option<Box<dyn DebugHook>>         │
├──────────────────────────────────────────────────────────┤
│ xcore::RVCore                                            │
│  step():                                                 │
│    fetch → decode → format_mnemonic()                    │
│    hook.on_execute(pc, raw, &mnemonic)                   │
│    if bp_hit → hook.on_breakpoint(pc) → return DebugBreak│
│    execute                                               │
│  load()/store():                                         │
│    hook.on_mem_access(pc, addr, size, op, value)         │
│                                                          │
│  impl DebugOps:                                          │
│    read_register(), dump_registers()                     │
│    read_memory() → Bus::read_ram(&self)                  │
│    disasm_raw() → DECODER + format_mnemonic()            │
└──────────────────────────────────────────────────────────┘
```

**Dependency direction**: xdb → xcore (only). xcore defines traits. xdb implements them.

[**Invariants**]
- I-1: xcore has zero trace entry types. Hook passes raw values.
- I-2: `DebugHook` trait defined in xcore, implemented in xdb.
- I-3: `DebugOps` trait defined in xcore, implemented per-arch.
- I-4: Watchpoints: xdb step loop when active, `CPU::run()` when inactive.
- I-5: `format_mnemonic()` in xcore (used during capture + disasm).
- I-6: `detect_call_return()` in xdb (interprets hook data for ftrace).
- I-7: Debugger reads: physical RAM via `Bus::read_ram(&self)`.
- I-8: `skip_bp_once` on RVCore for step-after-breakpoint.
- I-9: All debug code behind `cfg(feature = "debug")`.

[**Data Structure**]

```rust
// ═══ xcore/src/cpu/debug.rs ═══ cfg(feature = "debug")

/// Callback interface. xcore calls these, xdb implements them.
pub trait DebugHook: Send {
    /// Called after fetch+decode, before execute.
    fn on_execute(&mut self, pc: usize, raw: u32, mnemonic: &str);
    /// Called on load/store.
    fn on_mem_access(&mut self, pc: usize, addr: usize, size: usize, write: bool, value: u64);
    /// Called when breakpoint address matches current PC.
    /// Return true to stop execution (causes DebugBreak).
    fn on_breakpoint(&mut self, pc: usize) -> bool;
}

/// Read-only debug facade. Arch-specific impl.
pub trait DebugOps {
    fn read_register(&self, name: &str) -> Option<u64>;
    fn dump_registers(&self) -> Vec<(&'static str, u64)>;
    fn read_memory(&self, paddr: usize, size: usize) -> XResult<u64>;
    fn disasm_raw(&self, raw: u32) -> String;
    fn fetch_inst(&self, paddr: usize) -> XResult<u32>;
}

/// Mnemonic formatter (used by step() hooks and disasm).
pub fn format_mnemonic(inst: &DecodedInst) -> String { /* ... */ }

// ═══ xcore/src/cpu/riscv/mod.rs ═══

pub struct RVCore {
    // ... existing fields ...
    #[cfg(feature = "debug")]
    breakpoints: BTreeSet<usize>,
    #[cfg(feature = "debug")]
    skip_bp_once: bool,
    #[cfg(feature = "debug")]
    hook: Option<Box<dyn DebugHook>>,
}

impl CoreOps for RVCore {
    fn step(&mut self) -> XResult {
        // ... existing: bus.tick(), sync_interrupts, check_pending_interrupts ...

        #[cfg(feature = "debug")]
        {
            // Breakpoint check
            if !self.skip_bp_once && self.breakpoints.contains(&self.pc.as_usize()) {
                if let Some(ref mut hook) = self.hook {
                    if hook.on_breakpoint(self.pc.as_usize()) {
                        return Err(XError::DebugBreak(self.pc.as_usize()));
                    }
                }
            }
            self.skip_bp_once = false;
        }

        self.trap_on_err(|core| {
            let raw = core.fetch()?;
            let inst = core.decode(raw)?;

            #[cfg(feature = "debug")]
            if let Some(ref mut hook) = core.hook {
                let mn = format_mnemonic(&inst);
                hook.on_execute(core.pc.as_usize(), raw, &mn);
            }

            core.execute(inst)
        })?;

        self.retire();
        Ok(())
    }
}

// mtrace hooks in mm.rs load()/store():
#[cfg(feature = "debug")]
if let Some(ref mut hook) = self.hook {
    hook.on_mem_access(self.pc.as_usize(), addr, size, false, val as u64); // load
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
        // pc + 32 GPRs
        let mut out = vec![("pc", self.pc.as_usize() as u64)];
        for i in 0u8..32 { out.push((RVReg::try_from(i).unwrap().name(), self.gpr[i as usize])); }
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
        match DECODER.decode(raw) { Ok(i) => format_mnemonic(&i), Err(_) => format!("???") }
    }
}

impl RVCore {
    #[cfg(feature = "debug")]
    pub fn set_hook(&mut self, hook: Box<dyn DebugHook>) { self.hook = Some(hook); }
    #[cfg(feature = "debug")]
    pub fn add_breakpoint(&mut self, addr: usize) { self.breakpoints.insert(addr); }
    #[cfg(feature = "debug")]
    pub fn remove_breakpoint(&mut self, addr: usize) -> bool { self.breakpoints.remove(&addr) }
    #[cfg(feature = "debug")]
    pub fn set_skip_bp_once(&mut self) { self.skip_bp_once = true; }
}

// ═══ CPU pass-through ═══ cfg(feature = "debug")

impl<Core: CoreOps + DebugOps> CPU<Core> {
    pub fn debug_ops(&self) -> &dyn DebugOps { &self.core }
}

// For breakpoint/hook management, xdb uses with_xcpu to access core directly:
// with_xcpu(|cpu| cpu.core.add_breakpoint(addr))
// This requires making `core` pub(crate) or adding pass-through methods.

// ═══ xdb/src/trace/mod.rs ═══

use circular_buffer::CircularBuffer;

pub mod itrace;
pub mod ftrace;
pub mod mtrace;

pub trait Trace {
    fn name(&self) -> &'static str;
    fn display(&self);
}

macro_rules! register_trace {
    ($mgr:expr, itrace) => { $mgr.itrace_enabled = true };
    ($mgr:expr, ftrace) => { $mgr.ftrace_enabled = true };
    ($mgr:expr, mtrace) => { $mgr.mtrace_enabled = true };
}
pub(crate) use register_trace;

// ═══ xdb DebugHook implementation ═══

pub struct XdbHook {
    // itrace ring buffer
    pub itrace_buf: CircularBuffer<128, itrace::Entry>,
    pub itrace_enabled: bool,
    // ftrace ring buffer
    pub ftrace_buf: CircularBuffer<128, ftrace::Entry>,
    pub ftrace_enabled: bool,
    pub ftrace_depth: usize,
    // mtrace ring buffer
    pub mtrace_buf: CircularBuffer<256, mtrace::Entry>,
    pub mtrace_enabled: bool,
    // breakpoint stop flag
    pub bp_hit: Option<usize>,
}

impl xcore::cpu::debug::DebugHook for XdbHook {
    fn on_execute(&mut self, pc: usize, raw: u32, mnemonic: &str) {
        if self.itrace_enabled {
            self.itrace_buf.push_back(itrace::Entry {
                pc, raw, mnemonic: mnemonic.to_string(),
            });
        }
        if self.ftrace_enabled {
            ftrace::detect_and_record(pc, raw, mnemonic,
                &mut self.ftrace_buf, &mut self.ftrace_depth);
        }
    }

    fn on_mem_access(&mut self, pc: usize, addr: usize, size: usize, write: bool, value: u64) {
        if self.mtrace_enabled {
            self.mtrace_buf.push_back(mtrace::Entry {
                pc, addr, size,
                op: if write { b'W' } else { b'R' },
                value,
            });
        }
    }

    fn on_breakpoint(&mut self, pc: usize) -> bool {
        self.bp_hit = Some(pc);
        true // stop execution
    }
}

// ═══ xdb/src/trace/itrace.rs ═══
pub struct Entry { pub pc: usize, pub raw: u32, pub mnemonic: String }

pub fn display(buf: &CircularBuffer<128, Entry>) {
    println!("=== Instruction Trace ({} entries) ===", buf.len());
    for e in buf.iter() {
        println!("  {:#010x}: {:08x}  {}", e.pc, e.raw, e.mnemonic);
    }
}

// ═══ xdb/src/trace/ftrace.rs ═══
pub struct Entry { pub pc: usize, pub target: usize, pub kind: u8, pub depth: usize }

/// Detect call/return from mnemonic string or raw instruction.
pub fn detect_and_record(pc: usize, raw: u32, _mn: &str,
    buf: &mut CircularBuffer<128, Entry>, depth: &mut usize)
{
    // Decode raw to check:
    // Call: jal x1,* / jalr x1,* / c.jalr
    // Return: jalr x0,x1,0 / c.jr x1
    // (pattern detection from raw bits, not mnemonic string)
}

pub fn display(buf: &CircularBuffer<128, Entry>) { /* ... */ }

// ═══ xdb/src/trace/mtrace.rs ═══
pub struct Entry { pub pc: usize, pub addr: usize, pub size: usize, pub op: u8, pub value: u64 }
pub fn display(buf: &CircularBuffer<256, Entry>) { /* ... */ }

// ═══ xdb/src/watchpoint.rs ═══

pub struct Watchpoint { pub id: u32, pub expr_text: String, pub prev_value: Option<u64> }

pub struct WatchManager {
    watchpoints: Vec<Watchpoint>,
    next_id: u32,
}

impl WatchManager {
    pub fn is_empty(&self) -> bool { self.watchpoints.is_empty() }
    pub fn add(&mut self, expr: String, init: Option<u64>) -> u32 { /* ... */ }
    pub fn remove(&mut self, id: u32) -> bool { /* ... */ }

    /// Check all watchpoints. Returns first triggered.
    pub fn check<F>(&mut self, eval: F) -> Option<(u32, String, u64, u64)>
    where F: Fn(&str) -> Option<u64> { /* ... */ }
}

// ═══ xdb command flow ═══

fn cmd_continue() -> XResult {
    with_xcpu(|cpu| {
        // Fast path: no watchpoints → use run()
        if !has_watchpoints() {
            return cpu.run(u64::MAX);
        }
        // Slow path: step loop for watchpoint evaluation
        loop {
            cpu.step()?;
            if cpu.state.is_terminated() { break; }
            // Check watchpoints
            let eval = |expr: &str| eval_expr(expr, cpu).ok();
            if let Some((id, expr, old, new)) = WATCH_MGR.check(eval) {
                println!("Watchpoint {id}: {expr} changed {old:#x} → {new:#x}");
                return Ok(()); // return to prompt
            }
        }
        Ok(())
    })
}

fn cmd_step(count: u64) -> XResult {
    with_xcpu(|cpu| {
        for _ in 0..count {
            cpu.step()?;
            if cpu.state.is_terminated() { break; }
            // Check watchpoints after each step
            if !watch_mgr_is_empty() {
                let eval = |expr: &str| eval_expr(expr, cpu).ok();
                if let Some((id, expr, old, new)) = WATCH_MGR.check(eval) {
                    println!("Watchpoint {id}: {expr} changed {old:#x} → {new:#x}");
                    return Ok(());
                }
            }
        }
        Ok(())
    })
}
```

[**Constraints**]
- C-1: `chumsky` for expression parsing
- C-2: `circular-buffer` for trace ring buffers
- C-3: Debugger reads: `Bus::read_ram(&self)`, physical RAM only
- C-4: `cfg(feature = "debug")` for all debug code
- C-5: xdb step loop when watchpoints active, `run()` when inactive
- C-6: `format_mnemonic()` in xcore (capture-time), display in xdb
- C-7: `DebugHook` trait in xcore, impl in xdb
- C-8: `skip_bp_once` for step-after-breakpoint
- C-9: Pre-parser: `Regex::new()` + `OnceLock`
- C-10: ftrace: `jal x1`, `jalr x1`, `c.jalr`, `ret`, `c.jr x1` (no `c.jal`)

---

## Implement

### Implementation Plan

[**Phase 1: xcore debug traits + hooks**]
- `xcore/Cargo.toml` — `debug` feature, `circular-buffer` dep
- `xcore/src/cpu/debug.rs` — `DebugHook` trait, `DebugOps` trait, `format_mnemonic()`
- `xcore/src/cpu/riscv/debug.rs` — `impl DebugOps for RVCore`, `gpr_name_to_idx`, `csr_name_to_addr`
- `xcore/src/cpu/riscv/mod.rs` — `hook`, `breakpoints`, `skip_bp_once` fields; cfg hooks in `step()`
- `xcore/src/cpu/riscv/mm.rs` — mtrace hooks in `load()`/`store()`
- `xcore/src/error.rs` — `XError::DebugBreak(usize)`
- `xcore/src/cpu/mod.rs` — `CPU::debug_ops()` pass-through

[**Phase 2: xdb breakpoints + examine + info**]
- `xdb/Cargo.toml` — `regex`, `circular-buffer`, `chumsky`; enable xcore `debug`
- `xdb/src/trace/mod.rs` — `XdbHook` impl `DebugHook`, `Trace` trait, `register_trace!`
- `xdb/src/trace/itrace.rs` — `Entry`, `display()`
- `xdb/src/trace/ftrace.rs` — `Entry`, `detect_and_record()`, `display()`
- `xdb/src/trace/mtrace.rs` — `Entry`, `display()`
- `xdb/src/cli.rs` — `preprocess_line()`, new Commands
- `xdb/src/cmd.rs` — `cmd_break*`, `cmd_examine`, `cmd_info`
- `xdb/src/fmt.rs` — register/memory display
- `xdb/src/main.rs` — `respond()` catches `DebugBreak`, sets `skip_bp_once`, installs hook

[**Phase 3: Expression evaluator + watchpoints**]
- `xdb/src/expr.rs` — chumsky parser
- `xdb/src/watchpoint.rs` — `WatchManager`
- `cmd_print`, `cmd_watch*`
- Modified `cmd_continue`/`cmd_step` with wp step loop

[**Phase 4: Trace commands**]
- `trace itrace/ftrace/mtrace/show/off` commands
- Wire `register_trace!` calls to `XdbHook` enable flags

---

## Validation

[**Unit Tests**]
- V-UT-1: Expression parser
- V-UT-2: format_mnemonic (all InstKind)
- V-UT-3: Breakpoint add/remove
- V-UT-4: WatchManager value-change detection
- V-UT-5: CircularBuffer push_back overwrites oldest
- V-UT-6: Pre-parser transforms
- V-UT-7: DebugOps::read_register
- V-UT-8: detect_and_record call/return patterns

[**Integration Tests**]
- V-IT-1: Breakpoint stops, step advances past (skip_bp_once)
- V-IT-2: Watchpoint `w $a0` triggers during continue (step loop)
- V-IT-3: itrace captures during continue (via hook)
- V-IT-4: ftrace detects jal/ret
- V-IT-5: x/5i disassembles correctly

[**Edge Cases**]
- V-E-1: Step at breakpoint — advances one instruction
- V-E-2: Watchpoint on invalid expression → parse error
- V-E-3: `x/4x 0x02000000` → "not in RAM"
- V-E-4: Continue without watchpoints → uses fast `run()` path
- V-E-5: Batch mode → no hook installed, no debug overhead

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (breakpoints) | V-IT-1 |
| G-2 (watchpoints) | V-IT-2 |
| G-3 (expr eval) | V-UT-1 |
| G-4 (itrace) | V-IT-3 |
| G-5 (mtrace) | Phase 4 manual |
| G-6 (ftrace) | V-IT-4 |
| G-7 (disasm) | V-IT-5 |
| I-1 (no entries in xcore) | DebugHook passes raw values |
| I-4 (wp step loop) | V-E-4 |
| I-7 (RAM only) | V-E-3 |
| M-001 (no entries in xcore) | Hook trait interface only |
