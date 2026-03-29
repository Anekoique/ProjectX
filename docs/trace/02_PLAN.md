# `trace` PLAN `02`

> Status: Revised
> Feature: `trace`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `01_MASTER.md`

---

## Summary

Resolves all 4 blockers + 1 medium from round 01 and all 3 master directives. Concrete code-level detail for the xcore facade, xdb command parser, debugger stepping loop, mtrace bridge, and trace trait abstraction.

## Log

[**Feature Introduce**]

- Pre-parser splits GDB-style `/Nf` before clap — concrete regex + dispatch code
- `DebugOps` trait on `CoreOps` behind `cfg(feature = "debug")` — arch-agnostic debug facade
- Debugger memory reads use `Bus::read_ram()` (physical, no MMU side effects) — explicit design choice
- mtrace via `cfg(feature = "trace")` thread-local `Vec<MTraceEntry>` in xcore, drained by xdb
- `Trace` trait with `register_trace!` macro for scalable trace registration
- ftrace ring-buffered (bounded), consistent with itrace/mtrace

[**Review Adjustments**]

- R-001: Pre-parser regex extracts `/Nf` from `x` command before clap tokenization. Concrete code shown.
- R-002: New `DebugOps` trait extends `CoreOps` behind `cfg(feature = "debug")`. RV-specific impl on `RVCore`. Termination state readable via existing `CPU` public fields.
- R-003: Debugger reads use `Bus::read_ram(&self, addr, size)` — `&self` (immutable), bypasses MMU. No TLB mutation. Documented as physical-address access.
- R-004: mtrace bridge: `thread_local! { MTRACE_SINK: RefCell<Vec<MTraceEntry>> }`. xcore pushes, xdb drains. Feature-gated.
- R-005: ftrace uses `HeapRb` like itrace/mtrace. Bounded.

[**Master Compliance**]

- M-001: `Trace` trait + `register_trace!` macro for scalable trace registration. Each trace type (itrace, mtrace, ftrace) implements `Trace`. Feature-gated via `cfg(feature = "itrace")`, `cfg(feature = "mtrace")`, `cfg(feature = "ftrace")`.
- M-002: `DebugOps` trait is arch-agnostic. RV-specific register names/CSRs not exposed through CPU — only through `DebugOps` which returns `(name, value)` pairs. CPU itself gains minimal `cfg(feature = "debug")`-gated accessor.
- M-003: No CSR-specific APIs on `CPU`. `DebugOps::read_register(name) -> Option<u64>` is the single entry point. Arch impl maps names to GPR/CSR/PC internally.

### Changes from Previous Round

[**Added**]
- Pre-parser for GDB slash syntax (concrete regex code)
- `DebugOps` trait definition
- `Bus::read_ram(&self)` for side-effect-free debugger reads
- mtrace thread-local sink design
- `Trace` trait + `register_trace!`

[**Changed**]
- Debugger memory: virtual → physical (Bus::read_ram, no MMU)
- ftrace: unbounded Vec → bounded HeapRb
- CSR access: direct API → name-based `read_register()`

[**Removed**]
- Direct `read_csr()`, `read_gpr()`, `read_privilege()` methods on CPU
- Virtual-address debugger reads (replaced by physical)

[**Unresolved**]
- None

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review 01 | R-001 | Accepted | Pre-parser regex before clap |
| Review 01 | R-002 | Accepted | `DebugOps` trait behind `cfg(feature="debug")` |
| Review 01 | R-003 | Accepted | `Bus::read_ram(&self)` — physical, no side effects |
| Review 01 | R-004 | Accepted | Thread-local `MTRACE_SINK` in xcore, drained by xdb |
| Review 01 | R-005 | Accepted | ftrace bounded via HeapRb |
| Master 01 | M-001 | Applied | `Trace` trait + feature-gated registration |
| Master 01 | M-002 | Applied | `DebugOps` trait for CPU facade |
| Master 01 | M-003 | Applied | `read_register(name)` — no CSR-specific API on CPU |

---

## Spec

[**Goals**]
- G-1: Breakpoints
- G-2: Watchpoints
- G-3: Expression evaluator (chumsky)
- G-4: Instruction trace (itrace)
- G-5: Memory trace (mtrace)
- G-6: Function trace (ftrace)
- G-7: Disassembly

- NG-1: No ELF symbols
- NG-2: No difftest
- NG-3: No GDB remote protocol

[**Architecture**]

```
xdb (owns all debug/trace state)
 ├─ cli.rs          — pre-parser + clap commands
 ├─ cmd.rs          — command handlers
 ├─ expr.rs         — chumsky expression parser (NEW)
 ├─ state.rs        — DebugState: bp + wp (NEW)
 ├─ trace.rs        — TraceState: itrace/mtrace/ftrace ring buffers (NEW)
 ├─ fmt.rs          — mnemonic formatter + display (NEW)
 └─ run.rs          — StopReason + debug stepping loop (NEW)

xcore (minimal debug surface behind cfg features)
 ├─ cpu/core.rs     — CoreOps unchanged
 ├─ cpu/debug.rs    — DebugOps trait (NEW, cfg(feature="debug"))
 ├─ cpu/riscv/debug.rs — RVCore impl DebugOps (NEW, cfg(feature="debug"))
 ├─ device/bus.rs   — Bus::read_ram(&self) already exists
 └─ trace.rs        — MTRACE_SINK thread-local (NEW, cfg(feature="trace"))
```

[**Invariants**]
- I-1: xcore has zero knowledge of breakpoints/watchpoints
- I-2: `DebugOps` is behind `cfg(feature = "debug")`
- I-3: mtrace hooks behind `cfg(feature = "trace")`
- I-4: Debugger memory reads use `Bus::read_ram(&self)` — physical addresses, immutable, no MMU
- I-5: Ring buffers bounded via `ringbuf::HeapRb`
- I-6: Expression evaluator reads only, never writes
- I-7: Pre-parser handles `x/Nf` syntax before clap sees tokens

[**Data Structure**]

```rust
// ── xcore/src/cpu/debug.rs ── (cfg(feature = "debug"))

/// Arch-agnostic debug facade. Implemented per-ISA.
pub trait DebugOps {
    /// Read a named register. Accepts GPR names ("a0", "sp"),
    /// CSR names ("mstatus", "mepc"), and "pc".
    /// Returns None if name is unknown.
    fn read_register(&self, name: &str) -> Option<u64>;

    /// List all register names with current values.
    /// Returns (name, value) pairs grouped by category.
    fn dump_registers(&self) -> Vec<(&'static str, u64)>;

    /// Fetch raw instruction bytes at a physical address.
    fn fetch_inst_at(&self, paddr: usize) -> XResult<u32>;

    /// Decode raw instruction bytes into a displayable mnemonic.
    fn disasm(&self, raw: u32) -> String;
}

// ── xcore/src/cpu/riscv/debug.rs ── (cfg(feature = "debug"))

impl DebugOps for RVCore {
    fn read_register(&self, name: &str) -> Option<u64> {
        // "pc" → self.pc.as_usize() as u64
        // "a0".."a7", "t0".."t6", "s0".."s11", "sp", "ra", etc → self.gpr[reg]
        // "mstatus", "mepc", etc → self.csr.get(addr)
        // "privilege" → self.privilege as u64
        // _ → None
    }

    fn dump_registers(&self) -> Vec<(&'static str, u64)> {
        let mut regs = Vec::with_capacity(34);
        regs.push(("pc", self.pc.as_usize() as u64));
        for i in 0..32 {
            regs.push((RVReg::from_u8(i).unwrap().as_str(), self.gpr[i as usize]));
        }
        regs.push(("mstatus", self.csr.get(CsrAddr::mstatus)));
        regs
    }

    fn fetch_inst_at(&self, paddr: usize) -> XResult<u32> {
        let bus = self.bus.lock().unwrap();
        let lo = bus.read_ram(paddr, 2)? as u32;
        if lo & 0x3 != 0x3 {
            return Ok(lo & 0xFFFF); // compressed: 16-bit
        }
        let hi = bus.read_ram(paddr + 2, 2)? as u32;
        Ok(lo | (hi << 16))
    }

    fn disasm(&self, raw: u32) -> String {
        match DECODER.decode(raw) {
            Ok(inst) => format_mnemonic(&inst),
            Err(_) => format!("unknown ({:#010x})", raw),
        }
    }
}

// ── xcore/src/trace.rs ── (cfg(feature = "trace"))

use std::cell::RefCell;

pub struct MTraceEntry {
    pub pc: usize,
    pub addr: usize,
    pub size: usize,
    pub op: u8,     // b'R' or b'W'
    pub value: u64,
}

thread_local! {
    pub static MTRACE_SINK: RefCell<Vec<MTraceEntry>> = RefCell::new(Vec::new());
}

/// Called from xcore load/store paths when trace feature is enabled.
#[inline]
pub fn record_mtrace(pc: usize, addr: usize, size: usize, op: u8, value: u64) {
    MTRACE_SINK.with(|sink| {
        sink.borrow_mut().push(MTraceEntry { pc, addr, size, op, value });
    });
}

/// Called by xdb to drain accumulated entries.
pub fn drain_mtrace() -> Vec<MTraceEntry> {
    MTRACE_SINK.with(|sink| sink.borrow_mut().drain(..).collect())
}

// ── xdb/src/run.rs ──

pub enum StopReason {
    Stepped,
    Breakpoint(usize),
    Watchpoint { id: u32, expr: String, old: u64, new: u64 },
    ProgramExit(u32),
    Error(String),
}

// ── xdb/src/state.rs ──

use std::collections::BTreeSet;

pub struct DebugState {
    pub breakpoints: BTreeSet<usize>,
    pub watchpoints: Vec<Watchpoint>,
    next_wp_id: u32,
}

pub struct Watchpoint {
    pub id: u32,
    pub expr_text: String,
    pub prev_value: Option<u64>,
}

// ── xdb/src/trace.rs ──

use ringbuf::HeapRb;

/// Trait for all trace types. Enables scalable registration.
pub trait Trace {
    fn name(&self) -> &'static str;
    fn is_enabled(&self) -> bool;
    fn clear(&mut self);
    fn display(&self);
}

pub struct TraceState {
    traces: Vec<Box<dyn Trace>>,
}

impl TraceState {
    pub fn new() -> Self {
        let mut ts = Self { traces: Vec::new() };
        // Register built-in traces
        ts.register(Box::new(ITrace::new()));
        ts.register(Box::new(FTrace::new()));
        ts
    }

    pub fn register(&mut self, trace: Box<dyn Trace>) {
        self.traces.push(trace);
    }

    pub fn get<T: Trace + 'static>(&self) -> Option<&T> {
        self.traces.iter().find_map(|t| (t.as_ref() as &dyn std::any::Any).downcast_ref::<T>())
    }

    pub fn get_mut<T: Trace + 'static>(&mut self) -> Option<&mut T> {
        self.traces.iter_mut().find_map(|t| {
            (t.as_mut() as &mut dyn std::any::Any).downcast_mut::<T>()
        })
    }
}

pub struct ITrace {
    enabled: bool,
    buf: HeapRb<ITraceEntry>,
}

pub struct ITraceEntry {
    pub pc: usize,
    pub raw: u32,
    pub mnemonic: String,
}

impl Trace for ITrace {
    fn name(&self) -> &'static str { "itrace" }
    fn is_enabled(&self) -> bool { self.enabled }
    fn clear(&mut self) { /* clear ring buffer */ }
    fn display(&self) { /* print entries */ }
}

pub struct FTrace {
    enabled: bool,
    buf: HeapRb<FTraceEntry>,
    depth: usize,
}

pub struct FTraceEntry {
    pub pc: usize,
    pub target: usize,
    pub kind: char,    // 'C' or 'R'
    pub depth: usize,
}

impl Trace for FTrace { /* ... */ }
```

[**API Surface**]

Pre-parser (in `xdb/src/cli.rs`):

```rust
/// Expand GDB-style x/Nf syntax before clap parsing.
/// "x/5i 0x80000000" → "x -f i -n 5 0x80000000"
/// "x/10x"           → "x -f x -n 10"
/// "x"               → "x" (default: 1i at pc)
fn preprocess_line(line: &str) -> String {
    let re = regex!(r"^x/(\d+)?([ixbd])(.*)");
    if let Some(caps) = re.captures(line.trim()) {
        let count = caps.get(1).map_or("1", |m| m.as_str());
        let fmt = &caps[2];
        let rest = caps[3].trim();
        format!("x -f {fmt} -n {count} {rest}")
    } else {
        line.to_string()
    }
}
```

xdb clap commands:

```rust
#[derive(Debug, Subcommand)]
enum Commands {
    #[command(alias = "s")]
    Step { #[arg(default_value_t = 1)] count: u64 },

    #[command(alias = "c")]
    Continue,

    /// Examine memory / disassemble
    #[command(alias = "x")]
    Examine {
        #[arg(short, default_value = "i")]
        f: char,           // format: i(nstr), x(hex), b(yte), d(ecimal)
        #[arg(short, default_value_t = 1)]
        n: usize,          // count
        addr: Option<String>,  // hex address, default = pc
    },

    /// Set/delete/list breakpoints
    #[command(alias = "b")]
    Break {
        #[command(subcommand)]
        action: BreakAction,
    },

    /// Set/delete/list watchpoints
    #[command(alias = "w")]
    Watch {
        #[command(subcommand)]
        action: WatchAction,
    },

    /// Evaluate and print expression
    #[command(alias = "p")]
    Print { expr: Vec<String> },  // joined as single expr

    /// Show register/system info
    Info {
        #[command(subcommand)]
        what: InfoCmd,
    },

    /// Trace control
    Trace {
        #[command(subcommand)]
        action: TraceAction,
    },

    // ... existing: Load, Reset, Exit
}

#[derive(Debug, Subcommand)]
enum BreakAction {
    /// Set breakpoint at address
    #[command(name = "")]  // default: b <addr>
    Set { addr: String },
    /// Delete breakpoint
    #[command(alias = "d")]
    Delete { index: usize },
    /// List breakpoints
    #[command(alias = "l")]
    List,
}
```

Debug stepping loop (`xdb/src/run.rs`):

```rust
pub fn debug_step(
    cpu: &mut CPU<Core>,
    state: &mut DebugState,
    traces: &mut TraceState,
) -> StopReason {
    let pc = cpu.core.pc().as_usize();

    // 1. Breakpoint check
    if state.breakpoints.contains(&pc) {
        return StopReason::Breakpoint(pc);
    }

    // 2. Snapshot watchpoint values
    let old_values: Vec<Option<u64>> = state.watchpoints.iter()
        .map(|wp| eval_expr(&wp.expr_text, cpu).ok())
        .collect();

    // 3. Execute one step
    match cpu.step() {
        Err(e) => return StopReason::Error(e.to_string()),
        Ok(()) => {}
    }

    // 4. Check termination
    if cpu.state.is_terminated() {
        return StopReason::ProgramExit(cpu.halt_ret as u32);
    }

    // 5. Capture itrace (using DebugOps)
    #[cfg(feature = "debug")]
    if let Some(it) = traces.get_mut::<ITrace>() {
        if it.is_enabled() {
            let raw = cpu.core.fetch_inst_at(pc).unwrap_or(0);
            let mnemonic = cpu.core.disasm(raw);
            it.push(ITraceEntry { pc, raw, mnemonic });
        }
    }

    // 6. Capture ftrace (detect call/return from decoded instruction)
    #[cfg(feature = "debug")]
    if let Some(ft) = traces.get_mut::<FTrace>() {
        if ft.is_enabled() {
            // ... detect jal/jalr/ret patterns from decoded inst
        }
    }

    // 7. Drain mtrace from xcore thread-local
    #[cfg(feature = "trace")]
    if let Some(mt) = traces.get_mut::<MTrace>() {
        if mt.is_enabled() {
            for entry in xcore::trace::drain_mtrace() {
                mt.push(entry);
            }
        }
    }

    // 8. Watchpoint check
    for (i, wp) in state.watchpoints.iter_mut().enumerate() {
        let new_val = eval_expr(&wp.expr_text, cpu).ok();
        if old_values[i] != new_val {
            let old = old_values[i].unwrap_or(0);
            let new = new_val.unwrap_or(0);
            wp.prev_value = new_val;
            return StopReason::Watchpoint {
                id: wp.id,
                expr: wp.expr_text.clone(),
                old,
                new,
            };
        }
    }

    StopReason::Stepped
}

pub fn debug_continue(
    cpu: &mut CPU<Core>,
    state: &mut DebugState,
    traces: &mut TraceState,
) -> StopReason {
    loop {
        match debug_step(cpu, state, traces) {
            StopReason::Stepped => continue,
            reason => return reason,
        }
    }
}
```

[**Constraints**]
- C-1: `chumsky` for expression parsing
- C-2: `ringbuf` for ring buffers
- C-3: No ELF symbols
- C-4: mtrace: `cfg(feature = "trace")`, thread-local sink
- C-5: Debugger reads: physical via `Bus::read_ram(&self)`, documented as physical
- C-6: Disassembly reuses `DECODER` + `format_mnemonic()` in xdb
- C-7: `DebugOps` trait behind `cfg(feature = "debug")`
- C-8: Pre-parser regex for `x/Nf` before clap

---

## Implement

### Implementation Plan

[**Phase 1: xcore facade + xdb scaffolding**]

xcore:
- `cpu/debug.rs` — `DebugOps` trait (behind `cfg(feature = "debug")`)
- `cpu/riscv/debug.rs` — `impl DebugOps for RVCore`
- `Cargo.toml` — add `debug` and `trace` features
- `trace.rs` — `MTRACE_SINK` thread-local + `record_mtrace` + `drain_mtrace` (behind `cfg(feature = "trace")`)

xdb:
- `run.rs` — `StopReason` enum, `debug_step()`, `debug_continue()`
- `state.rs` — `DebugState` (bp + wp containers)
- `trace.rs` — `Trace` trait, `TraceState`, `ITrace`, `FTrace`
- `fmt.rs` — `format_mnemonic()`, register display
- Wire `cmd_step`/`cmd_continue` through `debug_step`/`debug_continue`
- `Cargo.toml` — add `ringbuf`, `chumsky`, `regex` deps; enable xcore `debug`+`trace` features

[**Phase 2: Breakpoints + Examine + Info**]

- `cli.rs` — pre-parser for `x/Nf`, `Break`, `Examine`, `Info` commands
- `cmd.rs` — `cmd_break`, `cmd_examine`, `cmd_info`
- Breakpoint check in `debug_step()`
- `x/Ni` — disassemble using `DebugOps::fetch_inst_at()` + `disasm()`
- `x/Nx` — examine memory via `Bus::read_ram()`
- `info reg` — `DebugOps::dump_registers()` with colored output

[**Phase 3: Expression evaluator + Print + Watchpoints**]

- `expr.rs` — chumsky parser for `$reg`, `*addr`, arithmetic, comparisons
- `p <expr>` — evaluate and print
- `w`/`w d`/`w l` commands
- Watchpoint eval in `debug_step()` after `cpu.step()`

[**Phase 4: Traces (itrace + ftrace + mtrace)**]

- itrace capture in `debug_step()` using `DebugOps`
- ftrace call/return detection:
  - Call: `jal rd` (rd=ra), `jalr rd, rs1, imm` (rd=ra), `c.jal`, `c.jalr`
  - Return: `jalr x0, ra, 0`, `c.jr ra`
- mtrace hooks in xcore `load()`/`store()` paths (`cfg(feature = "trace")`)
- `trace itrace/mtrace/ftrace/show/off` commands
- `trace show` dumps ring buffers with formatted output

---

## Trade-offs

- T-1: **Debugger reads: physical vs virtual** — Physical via `Bus::read_ram(&self)` avoids MMU side effects (R-003). Tradeoff: addresses don't match what guest code sees if MMU is active. Acceptable: most bare-metal debugging uses identity-mapped addresses. Future round can add virtual mode. **Prefer physical.**

- T-2: **`DebugOps` location: xcore trait vs xdb direct access** — Trait in xcore keeps ISA details encapsulated. xdb never imports `CsrAddr`, `RVReg`, etc. Future LoongArch backend just implements the same trait. **Prefer trait in xcore.**

- T-3: **ftrace: bounded ring vs call-stack-only** — Ring buffer is consistent with itrace/mtrace and bounded (R-005). Call-stack-only would require paired call/return tracking. **Prefer ring buffer.**

---

## Validation

[**Unit Tests**]
- V-UT-1: Expression parser (literals, registers, deref, arithmetic, precedence, errors)
- V-UT-2: Mnemonic formatter (all InstKind produce valid strings)
- V-UT-3: Breakpoint set/remove/contains
- V-UT-4: Pre-parser (`x/5i addr` → `x -f i -n 5 addr`)
- V-UT-5: `DebugOps::read_register` — all GPR/CSR names resolve

[**Integration Tests**]
- V-IT-1: Breakpoint stops at address, continue resumes
- V-IT-2: Watchpoint on `$a0` triggers on change
- V-IT-3: itrace ring buffer captures correct instruction sequence
- V-IT-4: ftrace detects jal/ret and c.jr ra
- V-IT-5: `x/5i 0x80000000` produces 5 disassembled lines

[**Edge Cases**]
- V-E-1: Breakpoint at current PC triggers on continue
- V-E-2: Watchpoint on unmapped memory → graceful error
- V-E-3: Compressed instruction disassembly (c.addi, c.sw, c.beqz)
- V-E-4: Pre-parser with no format: `x 0x80000000` → default `x/1i`

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (breakpoints) | V-IT-1 |
| G-2 (watchpoints) | V-IT-2 |
| G-3 (expr eval) | V-UT-1 |
| G-4 (itrace) | V-IT-3 |
| G-5 (mtrace) | Manual verification |
| G-6 (ftrace) | V-IT-4 |
| G-7 (disasm) | V-IT-5, V-UT-2 |
| I-1 (xdb owns debug) | No debug state in xcore |
| I-4 (physical reads) | Bus::read_ram(&self) used |
| I-7 (pre-parser) | V-UT-4 |
| M-001 (Trace trait) | TraceState with register() |
| M-002 (DebugOps) | Trait behind cfg(feature) |
| M-003 (no CSR on CPU) | read_register(name) only |
