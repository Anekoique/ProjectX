# `trace` PLAN `03`

> Status: Revised
> Feature: `trace`
> Iteration: `03`
> Owner: Executor
> Depends on:
> - Previous Plan: `02_PLAN.md`
> - Review: `02_REVIEW.md`
> - Master Directive: `02_MASTER.md`

---

## Summary

Resolves all 4 blockers + 1 medium from round 02 and both master directives. Key design shift: debug hooks are **cfg-stubbed into existing `CPU::step()` and `CPU::run()`**, not separate functions. Debugger facade is a single unified `DebugOps` trait (registers + memory + disasm). Trace registry uses named `EnumMap` instead of `dyn Any` downcasting. Pre-parser uses `Regex::new()`.

## Log

[**Feature Introduce**]

- Debug hooks stubbed into `CPU::step()`/`CPU::run()` via `cfg(feature = "debug")` — no new API surface
- `StopReason` variant added to `XError` — breakpoint/watchpoint are CPU-level events, not xdb-level
- Unified `DebugOps` trait: `read_register()`, `read_memory()`, `fetch_inst()`, `disasm()`
- `read_memory()` uses `Bus::read()` (includes MMIO) for full device visibility
- Named trace registry via `TraceManager` with explicit typed fields (no dyn downcasting)
- Pre-parser uses `Regex::new()` (lazy_static/OnceLock cached)
- Remove `c.jal` from ftrace patterns (RV64 only)

[**Review Adjustments**]

- R-001: Pre-parser uses `Regex::new()` with `OnceLock` caching. clap `Break` uses positional args, not empty-name subcommand.
- R-002: `DebugOps::read_memory(paddr, size)` uses `Bus::read()` — includes both RAM and MMIO. Single facade for all debugger reads.
- R-003: `CPU` exposes `debug_ops() -> &dyn DebugOps` behind `cfg(feature = "debug")`. xdb calls only this handle. No private field access.
- R-004: `TraceManager` with explicit typed fields: `itrace: Option<HeapRb<..>>`, `ftrace: Option<HeapRb<..>>`. No `dyn Trace` + Any downcasting.
- R-005: Removed `c.jal` from ftrace (RV64-only target).

[**Master Compliance**]

- M-001 (02): Renamed `TraceState` → `TraceManager`. Clearer semantics.
- M-002 (02): No `debug_step`/`debug_run`. Debug hooks cfg-stubbed into existing `CPU::step()`/`CPU::run()`. Breakpoint/watchpoint checks happen inside the existing execution loop via `cfg(feature = "debug")` blocks.

### Changes from Previous Round

[**Added**]
- `XError::DebugBreak(usize)` / `XError::DebugWatch { id, old, new }` — CPU-level stop reasons
- `CPU::debug_ops()` → `&dyn DebugOps` accessor
- `DebugOps::read_memory()` for RAM + MMIO reads
- `TraceManager` with typed fields

[**Changed**]
- Breakpoint/watchpoint checks: xdb stepping loop → cfg-stubbed in `CPU::run()`
- Memory reads: `Bus::read_ram()` only → `Bus::read()` (includes MMIO)
- Trace registry: `Vec<Box<dyn Trace>>` + downcasting → explicit typed fields
- Pre-parser: `regex!()` macro → `Regex::new()` + `OnceLock`
- Break command: empty-name subcommand → positional arg

[**Removed**]
- `debug_step()`, `debug_continue()` in xdb
- `dyn Trace` + `Any` downcasting
- `c.jal` ftrace pattern

[**Unresolved**]
- None

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review 02 | R-001 | Accepted | `Regex::new()` + `OnceLock`, valid clap subcommands |
| Review 02 | R-002 | Accepted | `DebugOps::read_memory()` → `Bus::read()` includes MMIO |
| Review 02 | R-003 | Accepted | `CPU::debug_ops()` is the only xdb→xcore entry point |
| Review 02 | R-004 | Accepted | Named typed fields in `TraceManager`, no downcasting |
| Review 02 | R-005 | Accepted | `c.jal` removed |
| Master 02 | M-001 | Applied | `TraceState` → `TraceManager` |
| Master 02 | M-002 | Applied | Debug hooks cfg-stubbed into `CPU::step()`/`CPU::run()` |

---

## Spec

[**Goals**]
- G-1: Breakpoints — checked inside `CPU::run()` via `cfg(feature = "debug")`
- G-2: Watchpoints — checked inside `CPU::run()` via `cfg(feature = "debug")`
- G-3: Expression evaluator (chumsky)
- G-4: Instruction trace (itrace)
- G-5: Memory trace (mtrace)
- G-6: Function trace (ftrace) — raw addresses, no `c.jal`
- G-7: Disassembly

[**Architecture**]

```
xdb (owns debug/trace state, drives commands)
 ├─ cli.rs          — pre-parser (Regex::new + OnceLock) + clap
 ├─ cmd.rs          — command handlers
 ├─ expr.rs         — chumsky expression parser (NEW)
 ├─ debug.rs        — breakpoint/watchpoint containers (NEW)
 ├─ trace.rs        — TraceManager: itrace/ftrace/mtrace (NEW)
 ├─ fmt.rs          — mnemonic formatter, register/memory display (NEW)
 └─ main.rs         — wire debug state into xdb mainloop

xcore (execution engine, debug surface behind cfg)
 ├─ cpu/mod.rs      — CPU::run() with cfg(feature="debug") bp/wp hooks
 │                    CPU::debug_ops() → &dyn DebugOps
 ├─ cpu/debug.rs    — DebugOps trait (NEW, cfg(feature="debug"))
 ├─ cpu/riscv/debug.rs — impl DebugOps for RVCore (NEW, cfg(feature="debug"))
 └─ trace.rs        — MTRACE_SINK thread-local (NEW, cfg(feature="trace"))
```

Key: xdb sets breakpoint/watchpoint state. xcore's `CPU` reads that state during `run()` via a shared reference. No new run/step functions.

[**Invariants**]
- I-1: No `debug_step()` or `debug_run()` functions. Debug hooks inside existing `CPU::step()`/`CPU::run()`.
- I-2: `DebugOps` behind `cfg(feature = "debug")`. Zero cost when disabled.
- I-3: mtrace behind `cfg(feature = "trace")`. Zero cost when disabled.
- I-4: `DebugOps::read_memory()` uses `Bus::read()` — covers RAM + MMIO.
- I-5: `TraceManager` has explicit typed fields. No dyn downcasting.
- I-6: xdb accesses xcore only via `CPU::debug_ops()` and existing `with_xcpu!` macro.
- I-7: Pre-parser uses `Regex::new()` with `OnceLock` caching.

[**Data Structure**]

```rust
// ── xcore/src/cpu/debug.rs ── cfg(feature = "debug")

use crate::error::XResult;

/// Arch-agnostic read-only debug facade.
pub trait DebugOps {
    /// Read named register: "pc", "a0", "sp", "mstatus", "privilege", etc.
    fn read_register(&self, name: &str) -> Option<u64>;

    /// All registers as (name, value) pairs.
    fn dump_registers(&self) -> Vec<(&'static str, u64)>;

    /// Read physical memory (RAM + MMIO). Uses Bus::read().
    fn read_memory(&self, paddr: usize, size: usize) -> XResult<u64>;

    /// Fetch raw instruction at physical address.
    fn fetch_inst(&self, paddr: usize) -> XResult<u32>;

    /// Decode raw instruction to mnemonic string.
    fn disasm(&self, raw: u32) -> String;
}

// ── xcore/src/cpu/mod.rs changes ──

impl<Core: CoreOps> CPU<Core> {
    // Existing step() unchanged.

    // Existing run() with cfg-stubbed debug hooks:
    pub fn run(&mut self, count: u64) -> XResult {
        if self.state.is_terminated() {
            info!("CPU is not running.");
            return Ok(());
        }
        for _ in 0..count {
            // ── breakpoint check (cfg-stubbed) ──
            #[cfg(feature = "debug")]
            if let Some(bp_set) = self.breakpoints.as_ref() {
                if bp_set.contains(&self.core.pc().as_usize()) {
                    return Err(XError::DebugBreak(self.core.pc().as_usize()));
                }
            }

            self.step()?;

            if self.state.is_terminated() {
                break;
            }

            // ── watchpoint check (cfg-stubbed) ──
            #[cfg(feature = "debug")]
            if let Some(ref mut wps) = self.watchpoints {
                for wp in wps.iter_mut() {
                    // xdb evaluates expression externally and stores
                    // snapshot values. CPU just compares old vs current.
                    if let Some((old, new)) = wp.check_changed() {
                        return Err(XError::DebugWatch {
                            id: wp.id, old, new
                        });
                    }
                }
            }
        }
        Ok(())
    }

    /// Debug facade accessor.
    #[cfg(feature = "debug")]
    pub fn debug_ops(&self) -> &dyn DebugOps
    where Core: DebugOps {
        &self.core
    }

    // ── breakpoint/watchpoint storage (cfg-stubbed) ──
    // These are Option fields on CPU, set by xdb before run().
}

// ── xcore/src/error.rs additions ──

#[cfg(feature = "debug")]
DebugBreak(usize),           // breakpoint hit at pc
#[cfg(feature = "debug")]
DebugWatch { id: u32, old: u64, new: u64 },  // watchpoint triggered

// ── xcore/src/cpu/riscv/debug.rs ── cfg(feature = "debug")

impl DebugOps for RVCore {
    fn read_register(&self, name: &str) -> Option<u64> {
        match name {
            "pc" => Some(self.pc.as_usize() as u64),
            "privilege" => Some(self.privilege as u64),
            _ => {
                // Try GPR
                if let Some(idx) = gpr_name_to_idx(name) {
                    return Some(self.gpr[idx]);
                }
                // Try CSR
                if let Some(addr) = csr_name_to_addr(name) {
                    return Some(self.csr.get_by_addr(addr));
                }
                None
            }
        }
    }

    fn dump_registers(&self) -> Vec<(&'static str, u64)> {
        let mut out = Vec::with_capacity(35);
        out.push(("pc", self.pc.as_usize() as u64));
        for i in 0u8..32 {
            let reg = RVReg::from_u8(i).unwrap();
            out.push((reg.as_str(), self.gpr[i as usize]));
        }
        // Key CSRs
        for &(name, addr) in &[
            ("mstatus", CsrAddr::mstatus),
            ("mepc", CsrAddr::mepc),
            ("mcause", CsrAddr::mcause),
        ] {
            out.push((name, self.csr.get(addr)));
        }
        out
    }

    fn read_memory(&self, paddr: usize, size: usize) -> XResult<u64> {
        // Bus::read() handles both RAM and MMIO
        self.bus.lock().unwrap().read(paddr, size).map(|v| v as u64)
    }

    fn fetch_inst(&self, paddr: usize) -> XResult<u32> {
        let bus = self.bus.lock().unwrap();
        let lo = bus.read(paddr, 2)? as u32;
        if lo & 0x3 != 0x3 {
            return Ok(lo & 0xFFFF); // 16-bit compressed
        }
        let hi = bus.read(paddr + 2, 2)? as u32;
        Ok(lo | (hi << 16))
    }

    fn disasm(&self, raw: u32) -> String {
        match DECODER.decode(raw) {
            Ok(inst) => format_mnemonic(&inst),
            Err(_) => format!("???  ({:#010x})", raw),
        }
    }
}

// ── xcore/src/trace.rs ── cfg(feature = "trace")

use std::cell::RefCell;

#[derive(Clone)]
pub struct MTraceEntry {
    pub pc: usize,
    pub addr: usize,
    pub size: usize,
    pub op: u8,     // b'R' or b'W'
    pub value: u64,
}

thread_local! {
    static MTRACE_SINK: RefCell<Vec<MTraceEntry>> = const { RefCell::new(Vec::new()) };
}

#[inline]
pub fn record_mtrace(pc: usize, addr: usize, size: usize, op: u8, value: u64) {
    MTRACE_SINK.with(|s| s.borrow_mut().push(MTraceEntry { pc, addr, size, op, value }));
}

pub fn drain_mtrace() -> Vec<MTraceEntry> {
    MTRACE_SINK.with(|s| s.borrow_mut().drain(..).collect())
}

// ── xdb/src/trace.rs ──

use ringbuf::HeapRb;

pub struct TraceManager {
    pub itrace: Option<HeapRb<ITraceEntry>>,
    pub ftrace: Option<HeapRb<FTraceEntry>>,
    // mtrace is drained from xcore::trace::drain_mtrace()
    // and pushed into this buffer by xdb after each step
    pub mtrace: Option<HeapRb<xcore::trace::MTraceEntry>>,
}

pub struct ITraceEntry {
    pub pc: usize,
    pub raw: u32,
    pub mnemonic: String,
}

pub struct FTraceEntry {
    pub pc: usize,
    pub target: usize,
    pub kind: char,   // 'C' call, 'R' return
    pub depth: usize,
}

impl TraceManager {
    pub fn new() -> Self {
        Self { itrace: None, ftrace: None, mtrace: None }
    }

    pub fn enable_itrace(&mut self, cap: usize) {
        self.itrace = Some(HeapRb::new(cap));
    }

    pub fn enable_ftrace(&mut self, cap: usize) {
        self.ftrace = Some(HeapRb::new(cap));
    }

    pub fn enable_mtrace(&mut self, cap: usize) {
        self.mtrace = Some(HeapRb::new(cap));
    }

    pub fn disable_all(&mut self) {
        self.itrace = None;
        self.ftrace = None;
        self.mtrace = None;
    }
}
```

[**API Surface**]

Pre-parser (`xdb/src/cli.rs`):

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

Clap commands:

```rust
#[derive(Debug, Subcommand)]
enum Commands {
    #[command(alias = "s")]
    Step { #[arg(default_value_t = 1)] count: u64 },

    #[command(alias = "c")]
    Continue,

    /// Examine memory or disassemble
    #[command(alias = "x")]
    Examine {
        #[arg(short = 'f', default_value = "i")]
        format: char,
        #[arg(short = 'n', default_value_t = 1)]
        count: usize,
        /// Address (hex, default = current pc)
        addr: Option<String>,
    },

    /// Set breakpoint
    #[command(alias = "b")]
    Break { addr: String },

    /// Delete breakpoint
    #[command(name = "bd")]
    BreakDelete { index: usize },

    /// List breakpoints
    #[command(name = "bl")]
    BreakList,

    /// Set watchpoint
    #[command(alias = "w")]
    Watch { expr: Vec<String> },

    /// Delete watchpoint
    #[command(name = "wd")]
    WatchDelete { id: u32 },

    /// List watchpoints
    #[command(name = "wl")]
    WatchList,

    /// Evaluate expression
    #[command(alias = "p")]
    Print { expr: Vec<String> },

    /// Register/system info
    Info { what: String, name: Option<String> },

    /// Trace control
    Trace { action: String, arg: Option<String> },

    // existing: Load, Reset, Exit
}
```

xdb command flow (no `debug_step`/`debug_run` — uses existing `with_xcpu!`):

```rust
// cmd_continue uses existing CPU::run(), which has cfg-stubbed bp/wp checks
pub fn cmd_continue() -> XResult {
    with_xcpu!(run(u64::MAX))
}

// cmd_step uses existing CPU::run()
pub fn cmd_step(count: u64) -> XResult {
    with_xcpu!(run(count))
}

// After run() returns XError::DebugBreak or DebugWatch,
// xdb's respond() catches them and prints diagnostic:
pub fn respond(line: &str) -> Result<bool, String> {
    let line = preprocess_line(line);
    let args = shlex::split(&line).ok_or("Invalid quoting")?;
    let cli = Cli::try_parse_from(args).map_err(|e| e.to_string())?;
    match cli.command {
        Commands::Step { count } => cmd_step(count),
        Commands::Continue => cmd_continue(),
        Commands::Break { addr } => cmd_break(addr),
        // ...
    }
    .map(|_| true)
    .or_else(|e| {
        match e {
            #[cfg(feature = "debug")]
            XError::DebugBreak(pc) => {
                println!("Breakpoint hit at {:#x}", pc);
                Ok(true)  // return to prompt, not terminate
            }
            #[cfg(feature = "debug")]
            XError::DebugWatch { id, old, new } => {
                println!("Watchpoint {id}: {:#x} → {:#x}", old, new);
                Ok(true)
            }
            _ => {
                terminate!(e);
                Ok(true)
            }
        }
    })
}
```

[**Constraints**]
- C-1: `chumsky` for expression parsing
- C-2: `ringbuf` for trace ring buffers
- C-3: No ELF symbols this round
- C-4: mtrace: `cfg(feature = "trace")`, thread-local sink
- C-5: `DebugOps::read_memory()` → `Bus::read()` — RAM + MMIO
- C-6: Disassembly: `DECODER.decode()` + `format_mnemonic()` in xdb
- C-7: `DebugOps` / debug hooks behind `cfg(feature = "debug")`
- C-8: Pre-parser: `Regex::new()` + `OnceLock`
- C-9: No `debug_step`/`debug_run`. Hooks stubbed in existing `CPU::step()`/`CPU::run()`.

---

## Implement

### Implementation Plan

[**Phase 1: xcore debug facade + error variants**]

- `xcore/Cargo.toml` — add `debug` and `trace` features
- `xcore/src/cpu/debug.rs` — `DebugOps` trait (`cfg(feature = "debug")`)
- `xcore/src/cpu/riscv/debug.rs` — `impl DebugOps for RVCore`
- `xcore/src/error.rs` — `XError::DebugBreak(usize)`, `XError::DebugWatch { .. }`
- `xcore/src/cpu/mod.rs` — `CPU::debug_ops()`, bp/wp `Option` fields, cfg-stubbed checks in `run()`
- `xcore/src/trace.rs` — `MTRACE_SINK` thread-local

[**Phase 2: xdb scaffolding + breakpoints + examine**]

- `xdb/Cargo.toml` — add `ringbuf`, `regex`, enable xcore `debug`+`trace`
- `xdb/src/cli.rs` — `preprocess_line()`, new Commands enum
- `xdb/src/debug.rs` — breakpoint container, `cmd_break`/`cmd_break_delete`/`cmd_break_list`
- `xdb/src/fmt.rs` — `format_mnemonic()`, register display
- `xdb/src/cmd.rs` — `cmd_examine` (x/Ni disasm, x/Nx memory), `cmd_info`
- Wire `respond()` to handle `XError::DebugBreak`

[**Phase 3: Expression evaluator + print + watchpoints**]

- `xdb/src/expr.rs` — chumsky parser
- `cmd_print`, `cmd_watch`/`cmd_watch_delete`/`cmd_watch_list`
- Watchpoint snapshot/check mechanism via CPU's cfg-stubbed hooks
- Wire `respond()` to handle `XError::DebugWatch`

[**Phase 4: Traces (itrace + ftrace + mtrace)**]

- `xdb/src/trace.rs` — `TraceManager`, `ITraceEntry`, `FTraceEntry`
- itrace capture: after each `cmd_step`, read pc + decode via `debug_ops()`
- ftrace detection from decoded instruction:
  - Call: `jal rd` (rd=x1), `jalr rd, rs1, imm` (rd=x1), `c.jalr`
  - Return: `jalr x0, x1, 0` (ret), `c.jr x1`
- mtrace: drain `xcore::trace::drain_mtrace()` after step
- Hook in `xcore/src/cpu/riscv/mm.rs` load/store (`cfg(feature = "trace")`)
- `trace itrace/ftrace/mtrace/show/off` commands

---

## Trade-offs

- T-1: **Debug hooks in CPU::run() vs separate debug_run()** — Stubbing into existing `run()` via `cfg` keeps the API surface unchanged. No new entry points for xdb to call. Debug is transparent. **Chosen per M-002.**

- T-2: **`Bus::read()` vs `Bus::read_ram()`** — `Bus::read()` dispatches to MMIO devices too, enabling ACLINT/PLIC/UART inspection. `read_ram()` is RAM-only. **Prefer `read()` per R-002.**

- T-3: **Named typed fields vs dyn Trace registry** — Typed fields (`itrace: Option<HeapRb<..>>`) are simpler, type-safe, no downcasting. Adding a new trace type means adding a field — acceptable for 3 trace types. **Chosen per R-004/TR-2.**

---

## Validation

[**Unit Tests**]
- V-UT-1: Expression parser
- V-UT-2: Mnemonic formatter
- V-UT-3: Breakpoint operations
- V-UT-4: Pre-parser (`x/5i addr` → `x -f i -n 5 addr`)
- V-UT-5: `DebugOps::read_register` — all GPR/CSR/PC names
- V-UT-6: `DebugOps::read_memory` — RAM + MMIO addresses

[**Integration Tests**]
- V-IT-1: Breakpoint stops execution, continue resumes
- V-IT-2: Watchpoint triggers on value change
- V-IT-3: itrace captures correct sequence
- V-IT-4: ftrace detects jal/ret and c.jr ra
- V-IT-5: `x/5i` disassembles 5 instructions

[**Edge Cases**]
- V-E-1: Breakpoint at current PC
- V-E-2: Watchpoint on unmapped memory
- V-E-3: Compressed instruction disassembly
- V-E-4: `x/4x 0x02000000` — reads ACLINT MMIO registers

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (breakpoints) | V-IT-1 |
| G-2 (watchpoints) | V-IT-2 |
| G-3 (expr eval) | V-UT-1 |
| G-4 (itrace) | V-IT-3 |
| G-5 (mtrace) | Manual |
| G-6 (ftrace) | V-IT-4 |
| G-7 (disasm) | V-IT-5, V-UT-2 |
| I-1 (no debug_step) | Hooks in CPU::run() only |
| I-4 (RAM+MMIO) | V-E-4 |
| I-5 (typed fields) | TraceManager has explicit fields |
| M-001 (rename) | TraceManager |
| M-002 (cfg stubs) | cfg(feature="debug") in CPU::run() |
