# `trace` PLAN `04`

> Status: Revised
> Feature: `trace`
> Iteration: `04`
> Owner: Executor
> Depends on:
> - Previous Plan: `03_PLAN.md`
> - Review: `03_REVIEW.md`
> - Master Directive: `03_MASTER.md`

---

## Summary

Resolves all 4 blockers from round 03 by committing to a single coherent model: **CPU owns a `DebugContext`** with cfg-gated fields, xdb configures it via setter methods, and both bp/wp checks and trace capture happen **inside `CoreOps::step()`** — the only place with per-instruction granularity. No `debug_step`/`debug_run`. No thread-locals. No dyn downcasting. Debugger reads are RAM-only (side-effect-free). Breakpoint-after-hit uses `skip_bp_once` flag.

## Log

[**Feature Introduce**]

- `DebugContext` struct owned by CPU, configured by xdb via setter APIs
- bp/wp checks + trace capture all inside `CoreOps::step()` — works for `continue`, `step N`, single step
- Watchpoints are **address-based** (watch a memory location), not expression-based in xcore. xdb converts expressions to addresses.
- Debugger memory reads: `Bus::read_ram(&self)` — RAM only, `&self`, zero side effects. MMIO inspection out of scope (documented).
- `skip_bp_once: bool` flag — after `DebugBreak`, next step skips bp check once to advance past breakpointed PC
- `Trace` trait with `TraceRing<T>` — revert to trait-based design per M-002(03), but with `Any` via supertrait
- No thread-locals, no `MTraceEntry` sink — mtrace captured directly inside `step()` via `DebugContext` reference
- `c.jal` removed from ftrace (RV64 only)

[**Review Adjustments**]

- R-001 (03): CPU owns `DebugContext`. xdb calls `cpu.debug_ctx_mut().add_breakpoint(addr)`. No shared references, no ownership ambiguity. Watchpoints: xdb evaluates expression → gets address → calls `cpu.debug_ctx_mut().add_watchpoint(addr, size)`. xcore monitors the address.
- R-002 (03): `Bus::read_ram(&self)` — immutable, RAM only. MMIO reads are explicitly out of scope: documented as "MMIO device reads have side effects (UART RX pop, PLIC claim); debugger must not invoke them. Use `trace mtrace` to observe MMIO activity."
- R-003 (03): Trace capture inside `CoreOps::step()` via `self.debug_ctx`. Works during `continue` because `CPU::run()` calls `step()` per iteration.
- R-004 (03): `skip_bp_once` flag set when `DebugBreak` is returned. Next `step()` call skips the bp check once, then clears the flag.

[**Master Compliance**]

- M-001 (03): Code quality improved — single ownership model, no thread-locals, no dyn Any hacks
- M-002 (03): Reverted to `Trace` trait design. `Trace: Any` supertrait for typed access. `TraceRing<T>` wraps `HeapRb<T>` + implements `Trace`.
- M-003 (03): Every R-001~R-005 scrutinized and resolved with concrete mechanism
- M-004 (03): Thread-local `MTRACE_SINK` removed. mtrace captured directly in `step()` via `DebugContext`.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review 03 | R-001 | Accepted | CPU-owned `DebugContext` with setter APIs |
| Review 03 | R-002 | Accepted | RAM-only reads, MMIO out of scope |
| Review 03 | R-003 | Accepted | Trace capture inside `step()` |
| Review 03 | R-004 | Accepted | `skip_bp_once` flag |
| Review 03 | R-005 | Accepted | Tightened helper refs |
| Master 03 | M-001 | Applied | Single model, no thread-locals |
| Master 03 | M-002 | Applied | Reverted `Trace` trait |
| Master 03 | M-003 | Applied | PUA-level scrutiny on each blocker |
| Master 03 | M-004 | Applied | Thread-local removed |

---

## Spec

[**Goals**]
- G-1: Breakpoints — address-based, checked in `step()`, `skip_bp_once` for step-after-hit
- G-2: Watchpoints — address+size monitoring in `step()`, xdb converts expressions to addresses
- G-3: Expression evaluator (chumsky) in xdb — maps `$reg`/`*addr` to values
- G-4: Instruction trace (itrace) — captured in `step()` after decode
- G-5: Memory trace (mtrace) — captured in `step()` via load/store hooks
- G-6: Function trace (ftrace) — captured in `step()` after decode, detects call/return
- G-7: Disassembly — xdb reads RAM via `Bus::read_ram()`, decodes with `DECODER`

- NG-1: No ELF symbols
- NG-2: No MMIO debugger reads (documented)
- NG-3: No GDB remote protocol

[**Architecture**]

```
xdb (command frontend, expression evaluator)
 ├─ cli.rs          — preprocess_line() + clap
 ├─ cmd.rs          — command handlers call cpu.debug_ctx_mut().*
 ├─ expr.rs         — chumsky parser, evaluates $reg and *addr (NEW)
 ├─ fmt.rs          — format_mnemonic(), register/memory display (NEW)
 └─ main.rs         — respond() catches XError::DebugBreak/DebugWatch

xcore (execution + debug context, all cfg-gated)
 ├─ cpu/mod.rs      — CPU holds Option<DebugContext>, debug_ctx()/debug_ctx_mut()
 │                    run() unchanged, step() has cfg-gated hooks
 ├─ cpu/debug.rs    — DebugContext, DebugOps trait, Trace trait, TraceRing<T> (NEW)
 ├─ cpu/riscv/debug.rs — impl DebugOps for RVCore (NEW)
 ├─ cpu/riscv/mod.rs — step() gains cfg(feature="debug") hooks
 └─ error.rs        — XError::DebugBreak, XError::DebugWatch
```

**Execution flow (step with debug hooks):**

```rust
// RVCore::step() — existing method, with cfg-gated additions

fn step(&mut self) -> XResult {
    // ── existing: bus.tick(), sync_interrupts, check_pending_interrupts ──
    { /* ... unchanged ... */ }

    // ── cfg(feature="debug"): breakpoint check ──
    #[cfg(feature = "debug")]
    if let Some(ctx) = &mut self.debug_ctx {
        if !ctx.skip_bp_once && ctx.breakpoints.contains(&self.pc.as_usize()) {
            return Err(XError::DebugBreak(self.pc.as_usize()));
        }
        ctx.skip_bp_once = false;
    }

    // ── existing: fetch → decode → execute ──
    self.trap_on_err(|core| {
        let raw = core.fetch()?;
        let inst = core.decode(raw)?;

        // ── cfg(feature="debug"): itrace + ftrace capture ──
        #[cfg(feature = "debug")]
        if let Some(ctx) = &mut core.debug_ctx {
            ctx.record_itrace(core.pc.as_usize(), raw, &inst);
            ctx.record_ftrace(core.pc.as_usize(), &inst);
        }

        core.execute(inst)
    })?;

    // ── cfg(feature="debug"): watchpoint check (after execute, before retire) ──
    #[cfg(feature = "debug")]
    if let Some(ctx) = &self.debug_ctx {
        for wp in &ctx.watchpoints {
            let cur = self.bus.lock().unwrap().read_ram(wp.addr, wp.size)
                .unwrap_or(0);
            if cur != wp.prev_value {
                let old = wp.prev_value;
                // Update prev_value for next check
                // (need &mut self — done after this block)
                return Err(XError::DebugWatch {
                    id: wp.id, addr: wp.addr, old, new: cur,
                });
            }
        }
    }

    // ── existing: retire ──
    self.retire();
    Ok(())
}
```

**CPU::run() — unchanged**, calls `step()` which handles everything:

```rust
pub fn run(&mut self, count: u64) -> XResult {
    // ... existing guard ...
    for _ in 0..count {
        self.step()?;             // bp/wp/trace all happen inside
        if self.state.is_terminated() { break; }
    }
    Ok(())
}
```

**xdb respond() — catches debug stops without terminate!:**

```rust
pub fn respond(line: &str) -> Result<bool, String> {
    let line = preprocess_line(line);
    // ... clap parse ...
    match cli.command { /* dispatch */ }
    .map(|_| true)
    .or_else(|e| match e {
        #[cfg(feature = "debug")]
        XError::DebugBreak(pc) => {
            // Set skip_bp_once so next step advances past this PC
            with_xcpu(|cpu| {
                if let Some(ctx) = cpu.debug_ctx_mut() {
                    ctx.skip_bp_once = true;
                }
            });
            println!("Breakpoint at {:#x}", pc);
            Ok(true) // return to prompt
        }
        #[cfg(feature = "debug")]
        XError::DebugWatch { id, addr, old, new } => {
            // Update prev_value
            with_xcpu(|cpu| {
                if let Some(ctx) = cpu.debug_ctx_mut() {
                    if let Some(wp) = ctx.watchpoints.iter_mut().find(|w| w.id == id) {
                        wp.prev_value = new;
                    }
                }
            });
            println!("Watchpoint {}: *{:#x} changed {:#x} → {:#x}", id, addr, old, new);
            Ok(true)
        }
        _ => {
            terminate!(e);
            Ok(true)
        }
    })
}
```

[**Invariants**]
- I-1: CPU owns `DebugContext`. xdb configures via setter APIs. No shared references.
- I-2: All debug hooks inside `CoreOps::step()`. `CPU::run()` unchanged.
- I-3: Debugger memory reads: `Bus::read_ram(&self)` only. No MMIO. No side effects.
- I-4: `skip_bp_once` prevents re-triggering after breakpoint hit.
- I-5: Traces captured per-instruction inside `step()` — works for `continue`, `step N`, `step 1`.
- I-6: All debug code behind `cfg(feature = "debug")`. Zero cost when compiled out.
- I-7: Watchpoints are address+size based. xdb maps expressions to addresses externally.

[**Data Structure**]

```rust
// ── xcore/src/cpu/debug.rs ── cfg(feature = "debug")

use std::any::Any;
use std::collections::BTreeSet;
use ringbuf::HeapRb;
use crate::error::XResult;

/// CPU-owned debug context. Configured by xdb, checked in step().
pub struct DebugContext {
    pub breakpoints: BTreeSet<usize>,
    pub watchpoints: Vec<WatchEntry>,
    pub skip_bp_once: bool,
    next_wp_id: u32,
    // Traces
    pub itrace: Option<Box<dyn Trace>>,
    pub ftrace: Option<Box<dyn Trace>>,
    pub mtrace: Option<Box<dyn Trace>>,
}

pub struct WatchEntry {
    pub id: u32,
    pub addr: usize,
    pub size: usize,
    pub prev_value: u64,
}

// ── Trace trait ──

pub trait Trace: Any + Send {
    fn name(&self) -> &'static str;
    fn is_enabled(&self) -> bool;
    fn clear(&mut self);
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

pub struct TraceRing<T: 'static + Send> {
    name: &'static str,
    buf: HeapRb<T>,
}

impl<T: 'static + Send> TraceRing<T> {
    pub fn new(name: &'static str, cap: usize) -> Self {
        Self { name, buf: HeapRb::new(cap) }
    }

    pub fn push(&mut self, entry: T) {
        use ringbuf::traits::Producer;
        self.buf.try_push(entry).ok(); // overwrite oldest on full
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        use ringbuf::traits::Consumer;
        self.buf.iter()
    }
}

impl<T: 'static + Send> Trace for TraceRing<T> {
    fn name(&self) -> &'static str { self.name }
    fn is_enabled(&self) -> bool { true }
    fn clear(&mut self) { /* drain all */ }
    fn as_any(&self) -> &dyn Any { self }
    fn as_any_mut(&mut self) -> &mut dyn Any { self }
}

// ── Trace entry types ──

pub struct ITraceEntry {
    pub pc: usize,
    pub raw: u32,
    pub mnemonic: String,
}

pub struct MTraceEntry {
    pub pc: usize,
    pub addr: usize,
    pub size: usize,
    pub op: u8,     // b'R' or b'W'
    pub value: u64,
}

pub struct FTraceEntry {
    pub pc: usize,
    pub target: usize,
    pub kind: u8,   // b'C' or b'R'
    pub depth: usize,
}

// ── DebugOps trait (arch-agnostic read-only facade) ──

pub trait DebugOps {
    fn read_register(&self, name: &str) -> Option<u64>;
    fn dump_registers(&self) -> Vec<(&'static str, u64)>;
    fn disasm_raw(&self, raw: u32) -> String;
}

// ── DebugContext API ──

impl DebugContext {
    pub fn new() -> Self { /* ... */ }

    pub fn add_breakpoint(&mut self, addr: usize) { self.breakpoints.insert(addr); }
    pub fn remove_breakpoint(&mut self, addr: usize) -> bool { self.breakpoints.remove(&addr) }

    pub fn add_watchpoint(&mut self, addr: usize, size: usize, init: u64) -> u32 {
        let id = self.next_wp_id;
        self.next_wp_id += 1;
        self.watchpoints.push(WatchEntry { id, addr, size, prev_value: init });
        id
    }
    pub fn remove_watchpoint(&mut self, id: u32) -> bool { /* ... */ }

    pub fn enable_itrace(&mut self, cap: usize) {
        self.itrace = Some(Box::new(TraceRing::<ITraceEntry>::new("itrace", cap)));
    }
    pub fn enable_ftrace(&mut self, cap: usize) {
        self.ftrace = Some(Box::new(TraceRing::<FTraceEntry>::new("ftrace", cap)));
    }
    pub fn enable_mtrace(&mut self, cap: usize) {
        self.mtrace = Some(Box::new(TraceRing::<MTraceEntry>::new("mtrace", cap)));
    }
    pub fn disable_all_traces(&mut self) {
        self.itrace = None;
        self.ftrace = None;
        self.mtrace = None;
    }

    pub fn record_itrace(&mut self, pc: usize, raw: u32, inst: &DecodedInst) { /* ... */ }
    pub fn record_ftrace(&mut self, pc: usize, inst: &DecodedInst) { /* ... */ }
}
```

**CPU accessor (cfg-gated):**

```rust
// In CPU<Core>
#[cfg(feature = "debug")]
debug_ctx: Option<DebugContext>,

#[cfg(feature = "debug")]
pub fn debug_ctx(&self) -> Option<&DebugContext> { self.debug_ctx.as_ref() }

#[cfg(feature = "debug")]
pub fn debug_ctx_mut(&mut self) -> Option<&mut DebugContext> { self.debug_ctx.as_mut() }

#[cfg(feature = "debug")]
pub fn enable_debug(&mut self) { self.debug_ctx = Some(DebugContext::new()); }
```

**RVCore DebugOps impl:**

```rust
impl DebugOps for RVCore {
    fn read_register(&self, name: &str) -> Option<u64> {
        match name {
            "pc" => Some(self.pc.as_usize() as u64),
            "privilege" => Some(self.privilege as u64),
            _ => gpr_name_to_idx(name)
                    .map(|i| self.gpr[i])
                .or_else(|| csr_name_to_addr(name)
                    .map(|a| self.csr.get_by_addr(a)))
        }
    }
    // ... dump_registers, disasm_raw ...
}
```

[**API Surface**]

Pre-parser (valid `Regex::new`):

```rust
use std::sync::OnceLock;
use regex::Regex;

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

Clap commands (all valid clap shapes):

```rust
#[derive(Debug, Subcommand)]
enum Commands {
    #[command(alias = "s")]  Step { #[arg(default_value_t = 1)] count: u64 },
    #[command(alias = "c")]  Continue,
    #[command(alias = "x")]  Examine { #[arg(short='f', default_value="i")] format: char,
                                        #[arg(short='n', default_value_t=1)] count: usize,
                                        addr: Option<String> },
    #[command(alias = "b")]  Break { addr: String },
    #[command(name = "bd")]  BreakDelete { index: usize },
    #[command(name = "bl")]  BreakList,
    #[command(alias = "w")]  Watch { expr: Vec<String> },
    #[command(name = "wd")]  WatchDelete { id: u32 },
    #[command(name = "wl")]  WatchList,
    #[command(alias = "p")]  Print { expr: Vec<String> },
    Info { what: String, name: Option<String> },
    Trace { action: String, arg: Option<String> },
    #[command(alias = "l")]  Load { file: String },
    #[command(alias = "r")]  Reset,
    #[command(aliases = ["quit", "q"])] Exit,
}
```

[**Constraints**]
- C-1: `chumsky` for expression parser in xdb
- C-2: `ringbuf` for `TraceRing<T>` in xcore
- C-3: Debugger reads: `Bus::read_ram(&self)` only. MMIO out of scope.
- C-4: All debug code: `cfg(feature = "debug")`
- C-5: No `debug_step`/`debug_run`. Hooks inside existing `step()`/`run()`.
- C-6: Watchpoints: address+size based in xcore. Expression→address conversion in xdb.
- C-7: `skip_bp_once` for step-after-breakpoint.
- C-8: Pre-parser: `Regex::new()` + `OnceLock`.
- C-9: ftrace: no `c.jal` (RV64 only). Patterns: `jal rd=x1`, `jalr rd=x1`, `c.jalr`, `ret` (`jalr x0,x1,0`), `c.jr x1`.

---

## Implement

### Implementation Plan

[**Phase 1: xcore debug infrastructure**]
- `xcore/Cargo.toml` — `debug` and `trace` features, `ringbuf` dep
- `xcore/src/cpu/debug.rs` — `DebugContext`, `DebugOps`, `Trace`, `TraceRing<T>`, entry types, `WatchEntry`
- `xcore/src/cpu/riscv/debug.rs` — `impl DebugOps for RVCore`, `gpr_name_to_idx`, `csr_name_to_addr`
- `xcore/src/error.rs` — `XError::DebugBreak(usize)`, `XError::DebugWatch { id, addr, old, new }`
- `xcore/src/cpu/mod.rs` — `CPU.debug_ctx`, `enable_debug()`, `debug_ctx()`, `debug_ctx_mut()`
- `xcore/src/cpu/riscv/mod.rs` — cfg-gated hooks in `step()`: bp check, itrace, ftrace, wp check

[**Phase 2: xdb breakpoints + examine + info**]
- `xdb/Cargo.toml` — `regex`, enable xcore `debug`
- `xdb/src/cli.rs` — `preprocess_line()`, new `Commands`
- `xdb/src/cmd.rs` — `cmd_break`, `cmd_examine`, `cmd_info`
- `xdb/src/fmt.rs` — `format_mnemonic()`, register display
- `xdb/src/main.rs` — `respond()` catches `DebugBreak`, sets `skip_bp_once`
- Call `cpu.enable_debug()` on xdb init

[**Phase 3: Expression evaluator + watchpoints**]
- `xdb/src/expr.rs` — chumsky parser
- `cmd_print`, `cmd_watch` (evaluates expr → addr → `cpu.debug_ctx_mut().add_watchpoint()`)
- `respond()` catches `DebugWatch`, updates `prev_value`

[**Phase 4: Traces**]
- itrace/ftrace capture in `step()` via `DebugContext::record_itrace/record_ftrace`
- mtrace: add cfg-gated hooks in `RVCore`'s `load()`/`store()` methods, push to `DebugContext.mtrace`
- `trace itrace/ftrace/mtrace/show/off` commands
- Display formatting for trace ring buffers

---

## Trade-offs

- T-1: **RAM-only vs RAM+MMIO debugger reads** — RAM-only is side-effect-free. MMIO reads mutate device state (UART RX pop, PLIC claim). Chosen: RAM-only. Use `trace mtrace` to observe MMIO activity. Future: add `Device::peek()` for non-invasive MMIO reads.
- T-2: **Address-based vs expression-based watchpoints in xcore** — Address-based is simpler, no expression parser in xcore. xdb converts `$a0` → current a0 value → watch that addr. Tradeoff: `w $a0` monitors a memory address, not the register itself. For register watches, xdb can poll after each step. Chosen: address-based for memory, polling for registers.
- T-3: **`Trace` trait vs explicit typed fields** — Trait reverted per M-002(03). `TraceRing<T>` wraps HeapRb and implements `Trace: Any`. `DebugContext` stores `Option<Box<dyn Trace>>` per trace type. Typed access via `as_any().downcast_ref()`. 3 trace types is small enough that `as_any()` overhead is negligible.

---

## Validation

[**Unit Tests**]
- V-UT-1: Expression parser (literals, registers, deref, arithmetic, precedence)
- V-UT-2: Mnemonic formatter (all InstKind → string)
- V-UT-3: DebugContext bp add/remove/contains
- V-UT-4: WatchEntry value-change detection
- V-UT-5: TraceRing push/overflow/iter
- V-UT-6: Pre-parser transforms
- V-UT-7: DebugOps::read_register — GPR/CSR/PC names

[**Integration Tests**]
- V-IT-1: Breakpoint stops execution, step advances past it (skip_bp_once)
- V-IT-2: Watchpoint on memory address triggers on store
- V-IT-3: itrace captures during `continue` (not just single-step)
- V-IT-4: ftrace detects jal/ret and c.jr ra
- V-IT-5: `x/5i 0x80000000` disassembles 5 instructions

[**Edge Cases**]
- V-E-1: Step at breakpoint — advances one instruction, then re-checks
- V-E-2: Watchpoint on unmapped address — `Bus::read_ram` returns error, wp skipped
- V-E-3: `x/4x 0x80000000` — reads RAM as hex words
- V-E-4: `x/4x 0x02000000` — fails with "address not in RAM" (MMIO, no side effects)
- V-E-5: Compressed instruction disassembly
- V-E-6: Traces disabled by default — zero overhead

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
| I-1 (CPU owns debug) | DebugContext in CPU |
| I-2 (hooks in step) | cfg blocks in RVCore::step() |
| I-3 (RAM only) | V-E-4 |
| I-4 (skip_bp_once) | V-E-1 |
| I-5 (per-inst trace) | V-IT-3 |
