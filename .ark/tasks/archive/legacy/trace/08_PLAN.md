# `trace` PLAN `08`

> Status: Revised
> Feature: `trace`
> Iteration: `08`
> Owner: Executor
> Depends on:
> - Previous Plan: `07_PLAN.md`
> - Review: `07_REVIEW.md`
> - Master Directive: `07_MASTER.md`

---

## Summary

Resolves all 3 blockers. Introduces `CoreDebugOps` trait alongside `CoreOps`. All addresses are **physical** with explicit documentation. Breakpoints use **stable numeric IDs** with address-based storage. Clean functional patterns per M-001.

## Log

[**Feature Introduce**]

- `CoreDebugOps` trait: `add_breakpoint`, `remove_breakpoint_by_id`, `list_breakpoints`, `set_skip_bp`
- `DebugOps` trait (read-only): `read_register`, `dump_registers`, `read_memory`, `fetch_inst`, `disasm_raw`
- All addresses physical. `x/Ni` defaults to physical PC. Documented.
- Breakpoints: `Vec<Breakpoint { id, addr }>` with stable IDs. `bd <id>` deletes by ID.
- Functional patterns: `Option::map`, `Iterator::find`, `or_else` chains

[**Review Adjustments**]

- R-001 (07): `CoreDebugOps` trait defined explicitly. `CPU` impl bounded by `Core: CoreOps + CoreDebugOps + DebugOps`. All pass-through methods type-check.
- R-002 (07): All addresses physical. Documented in command help. `x/Ni` defaults to `cpu.pc()` which returns physical address. Future round adds `xv` command for virtual reads when MMU-enabled execution is debugged.
- R-003 (07): Breakpoints store `(id: u32, addr: usize)`. `bl` shows stable IDs. `bd <id>` deletes by ID. IDs are monotonically increasing, never reused.

[**Master Compliance**]

- M-001 (07): Functional patterns: `find_map`, `filter`, `map`, `or_else` chains in expr eval and register lookup.
- M-002 (07): All reviewer findings fixed.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review 07 | R-001 | Accepted | `CoreDebugOps` trait |
| Review 07 | R-002 | Accepted | All physical, documented |
| Review 07 | R-003 | Accepted | Stable breakpoint IDs |
| Review 07 | TR-1 | Accepted | Explicit debug trait |
| Review 07 | TR-2 | Accepted | Stable IDs over sorted-set indices |
| Master 07 | M-001 | Applied | Functional patterns |
| Master 07 | M-002 | Applied | All findings fixed |

---

## Spec

[**Goals**]
- G-1: Breakpoints — stable IDs, checked in `step()`, skip-on-resume
- G-2: Watchpoints — expression-based, xdb step loop
- G-3: Expression evaluator (chumsky)
- G-4: Disassembly — `x/Ni addr`
- G-5: Memory examine — `x/Nx addr`
- G-6: Register inspect — `info reg`
- G-7: Execution logging via `log!()`

[**Architecture**]

```
┌──────────────────────────────────────────────┐
│ xdb                                          │
│  cli.rs        — preprocess + clap           │
│  cmd.rs        — handlers                    │
│  expr.rs       — chumsky parser              │
│  watchpoint.rs — WatchManager                │
│  main.rs       — respond(), wp step loop     │
├──────────────────────────────────────────────┤
│ xcore public API (traits + CPU methods)      │
│                                              │
│  trait CoreOps     { pc, bus, reset, step,   │
│                      halted, halt_ret }      │
│                                              │
│  trait CoreDebugOps: CoreOps {               │  cfg(feature="debug")
│    add_breakpoint(addr) -> u32               │
│    remove_breakpoint(id) -> bool             │
│    list_breakpoints() -> &[(u32, usize)]     │
│    set_skip_bp()                             │
│  }                                           │
│                                              │
│  trait DebugOps {                            │  cfg(feature="debug")
│    read_register(name) -> Option<u64>        │
│    dump_registers() -> Vec<(&str, u64)>      │
│    read_memory(paddr, size) -> XResult<u64>  │
│    fetch_inst(paddr) -> XResult<u32>         │
│    disasm_raw(u32) -> String                 │
│  }                                           │
│                                              │
│  CPU<Core: CoreOps + CoreDebugOps + DebugOps>│
│    .add_breakpoint(addr) -> u32              │  delegates to core
│    .remove_breakpoint(id) -> bool            │
│    .list_breakpoints() -> &[(u32, usize)]    │
│    .set_skip_bp()                            │
│    .debug_ops() -> &dyn DebugOps             │
│    .pc() -> usize                            │
│    .is_terminated() -> bool                  │
├──────────────────────────────────────────────┤
│ RVCore                                       │
│  impl CoreOps       (existing)               │
│  impl CoreDebugOps  (breakpoint storage)     │  cfg(feature="debug")
│  impl DebugOps      (read-only inspection)   │  cfg(feature="debug")
│  step(): bp check + trace!() + debug!()      │
└──────────────────────────────────────────────┘
```

[**Invariants**]
- I-1: `CoreDebugOps` and `DebugOps` are separate traits behind `cfg(feature = "debug")`.
- I-2: `CPU` pass-through methods bounded by `Core: CoreOps + CoreDebugOps + DebugOps`.
- I-3: All debugger addresses are **physical**. Documented in command help text.
- I-4: Breakpoint IDs are `u32`, monotonically increasing, never reused.
- I-5: `set_skip_bp()` prevents re-trigger on resume after breakpoint hit.
- I-6: Watchpoints in xdb. Step loop when active.
- I-7: Debugger reads: `Bus::read_ram(&self)`. Physical RAM only.

[**Data Structure**]

```rust
// ═══ xcore/src/cpu/debug.rs ═══ cfg(feature = "debug")

use crate::error::XResult;
use crate::isa::DecodedInst;

/// Breakpoint storage entry.
pub struct Breakpoint {
    pub id: u32,
    pub addr: usize,
}

/// Debug control trait — breakpoint management.
/// Implemented per-arch, called through CPU pass-through.
pub trait CoreDebugOps: super::CoreOps {
    fn add_breakpoint(&mut self, addr: usize) -> u32;
    fn remove_breakpoint(&mut self, id: u32) -> bool;
    fn list_breakpoints(&self) -> &[Breakpoint];
    fn set_skip_bp(&mut self);
}

/// Read-only inspection trait — registers, memory, disasm.
pub trait DebugOps {
    fn read_register(&self, name: &str) -> Option<u64>;
    fn dump_registers(&self) -> Vec<(&'static str, u64)>;
    fn read_memory(&self, paddr: usize, size: usize) -> XResult<u64>;
    fn fetch_inst(&self, paddr: usize) -> XResult<u32>;
    fn disasm_raw(&self, raw: u32) -> String;
}

/// Format decoded instruction to mnemonic string.
pub fn format_mnemonic(inst: &DecodedInst) -> String {
    // R-type: "add rd, rs1, rs2"
    // I-type: "addi rd, rs1, imm"
    // S-type: "sw rs2, imm(rs1)"
    // B-type: "beq rs1, rs2, imm"
    // U-type: "lui rd, imm"
    // J-type: "jal rd, imm"
    // C-type: "c.addi rd, imm"
    todo!("match on DecodedInst variants")
}

// ═══ xcore/src/cpu/riscv/debug.rs ═══ cfg(feature = "debug")

impl CoreDebugOps for RVCore {
    fn add_breakpoint(&mut self, addr: usize) -> u32 {
        let id = self.next_bp_id;
        self.next_bp_id += 1;
        self.breakpoints.push(Breakpoint { id, addr });
        id
    }

    fn remove_breakpoint(&mut self, id: u32) -> bool {
        self.breakpoints.iter()
            .position(|bp| bp.id == id)
            .map(|pos| { self.breakpoints.remove(pos); })
            .is_some()
    }

    fn list_breakpoints(&self) -> &[Breakpoint] {
        &self.breakpoints
    }

    fn set_skip_bp(&mut self) {
        self.skip_bp_once = true;
    }
}

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

    fn dump_registers(&self) -> Vec<(&'static str, u64)> {
        std::iter::once(("pc", self.pc.as_usize() as u64))
            .chain((0u8..32).map(|i| {
                let r = RVReg::try_from(i).unwrap();
                (r.name(), self.gpr[i as usize])
            }))
            .collect()
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
        DECODER.decode(raw)
            .map(|inst| format_mnemonic(&inst))
            .unwrap_or_else(|_| format!("???  ({raw:#010x})"))
    }
}

/// Map GPR ABI name → index.
fn gpr_name_to_idx(name: &str) -> Option<usize> {
    // "zero"→0, "ra"→1, "sp"→2, ..., "t6"→31
    // Also "x0"→0, "x1"→1, ..., "x31"→31
    RVReg::try_from_name(name).map(|r| r as usize)
}

/// Map CSR name → address.
fn csr_name_to_addr(name: &str) -> Option<u16> {
    CsrAddr::from_name(name).map(|a| a as u16)
}

// ═══ xcore/src/cpu/riscv/mod.rs ═══ new fields + step hooks

pub struct RVCore {
    // ... existing fields ...
    #[cfg(feature = "debug")]
    breakpoints: Vec<Breakpoint>,
    #[cfg(feature = "debug")]
    next_bp_id: u32,
    #[cfg(feature = "debug")]
    skip_bp_once: bool,
}

impl CoreOps for RVCore {
    fn step(&mut self) -> XResult {
        // existing: bus.tick(), sync_interrupts, check_pending_interrupts

        #[cfg(feature = "debug")]
        {
            let pc = self.pc.as_usize();
            if !self.skip_bp_once
                && self.breakpoints.iter().any(|bp| bp.addr == pc)
            {
                return Err(XError::DebugBreak(pc));
            }
            self.skip_bp_once = false;
        }

        self.trap_on_err(|core| {
            let raw = core.fetch()?;
            let inst = core.decode(raw)?;

            trace!("{:#010x}: {:08x}  {}", core.pc.as_usize(), raw, format_mnemonic(&inst));

            core.execute(inst)
        })?;

        self.retire();
        Ok(())
    }
}

// mm.rs load/store:
// debug!("R [{addr:#x}+{size}] = {val:#x}");
// debug!("W [{addr:#x}+{size}] = {val:#x}");

// ═══ xcore/src/cpu/mod.rs ═══ CPU pass-through

// Public accessors (always available)
impl<Core: CoreOps> CPU<Core> {
    pub fn pc(&self) -> usize { self.core.pc().as_usize() }
    pub fn is_terminated(&self) -> bool { self.state.is_terminated() }
}

// Debug pass-through (requires debug traits)
#[cfg(feature = "debug")]
impl<Core: CoreDebugOps + DebugOps> CPU<Core> {
    pub fn add_breakpoint(&mut self, addr: usize) -> u32 {
        self.core.add_breakpoint(addr)
    }
    pub fn remove_breakpoint(&mut self, id: u32) -> bool {
        self.core.remove_breakpoint(id)
    }
    pub fn list_breakpoints(&self) -> &[Breakpoint] {
        self.core.list_breakpoints()
    }
    pub fn set_skip_bp(&mut self) {
        self.core.set_skip_bp();
    }
    pub fn debug_ops(&self) -> &dyn DebugOps {
        &self.core
    }
}

// ═══ xcore/src/error.rs ═══

#[cfg(feature = "debug")]
DebugBreak(usize),

// ═══ xdb/src/expr.rs ═══

// Grammar (chumsky):
//   expr    = compare
//   compare = arith (("==" | "!=") arith)?
//   arith   = term (('+' | '-') term)*
//   term    = unary (('*' | '/' | '%') unary)*
//   unary   = '*' unary | '-' unary | atom
//   atom    = '$' REG | "0x" HEX | DECIMAL | '(' expr ')'

/// Evaluate expression.
/// `read_reg`: "$name" → value
/// `read_mem`: physical addr + size → value
pub fn eval_expr(
    input: &str,
    read_reg: impl Fn(&str) -> Option<u64>,
    read_mem: impl Fn(usize, usize) -> Option<u64>,
) -> Result<u64, String> {
    let ast = parse(input)?;
    eval(&ast, &read_reg, &read_mem)
}

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

    pub fn add(&mut self, expr: String, init: Option<u64>) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        self.wps.push(Watchpoint { id, expr_text: expr, prev_value: init });
        id
    }

    pub fn remove(&mut self, id: u32) -> bool {
        self.wps.iter()
            .position(|w| w.id == id)
            .map(|pos| { self.wps.remove(pos); })
            .is_some()
    }

    pub fn list(&self) -> &[Watchpoint] { &self.wps }

    /// Check all watchpoints. Returns first triggered.
    pub fn check(&mut self, eval: impl Fn(&str) -> Option<u64>)
        -> Option<(u32, String, u64, u64)>
    {
        self.wps.iter_mut().find_map(|wp| {
            let new_val = eval(&wp.expr_text);
            (wp.prev_value != new_val).then(|| {
                let old = wp.prev_value.unwrap_or(0);
                let new = new_val.unwrap_or(0);
                let expr = wp.expr_text.clone();
                wp.prev_value = new_val;
                (wp.id, expr, old, new)
            })
        })
    }
}

// ═══ xdb command flow ═══

fn cmd_continue(watch_mgr: &mut WatchManager) -> XResult {
    with_xcpu(|cpu| {
        if watch_mgr.is_empty() {
            return cpu.run(u64::MAX);      // fast path
        }
        loop {                              // wp step loop
            cpu.step()?;
            if cpu.is_terminated() { break; }
            if let Some((id, expr, old, new)) = watch_mgr.check(|e| {
                eval_expr(e,
                    |name| cpu.debug_ops().read_register(name),
                    |addr, sz| cpu.debug_ops().read_memory(addr, sz).ok(),
                ).ok()
            }) {
                println!("Watchpoint {id}: {expr} changed {old:#x} → {new:#x}");
                return Ok(());
            }
        }
        Ok(())
    })
}

fn respond(line: &str, watch_mgr: &mut WatchManager) -> Result<bool, String> {
    let line = preprocess_line(line);
    let args = shlex::split(&line).ok_or("Invalid quoting")?;
    let cli = Cli::try_parse_from(args).map_err(|e| e.to_string())?;
    match cli.command {
        Commands::Step { count } => cmd_step(count, watch_mgr),
        Commands::Continue => cmd_continue(watch_mgr),
        Commands::Break { addr } => {
            let addr = parse_addr(&addr)?;
            let id = with_xcpu(|cpu| cpu.add_breakpoint(addr));
            println!("Breakpoint {id} at {addr:#x}");
            Ok(())
        }
        Commands::BreakDelete { id } => {
            let ok = with_xcpu(|cpu| cpu.remove_breakpoint(id));
            if ok { println!("Deleted breakpoint {id}"); }
            else { println!("No breakpoint {id}"); }
            Ok(())
        }
        Commands::BreakList => {
            with_xcpu(|cpu| {
                let bps = cpu.list_breakpoints();
                if bps.is_empty() { println!("No breakpoints."); }
                else { bps.iter().for_each(|bp| println!("  #{}: {:#x}", bp.id, bp.addr)); }
            });
            Ok(())
        }
        Commands::Examine { format, count, addr } => cmd_examine(format, count, addr),
        Commands::Print { expr } => cmd_print(&expr.join(" ")),
        Commands::Info { what, name } => cmd_info(&what, name.as_deref()),
        Commands::Watch { expr } => cmd_watch(&expr.join(" "), watch_mgr),
        Commands::WatchDelete { id } => { watch_mgr.remove(id); Ok(()) }
        Commands::WatchList => {
            watch_mgr.list().iter()
                .for_each(|w| println!("  #{}: {}", w.id, w.expr_text));
            Ok(())
        }
        // existing: Load, Reset, Exit
    }
    .map(|_| true)
    .or_else(|e| match e {
        #[cfg(feature = "debug")]
        XError::DebugBreak(pc) => {
            with_xcpu(|cpu| cpu.set_skip_bp());
            println!("Breakpoint at {pc:#x}");
            Ok(true)
        }
        _ => { terminate!(e); Ok(true) }
    })
}
```

[**API Surface**]

Commands (all physical addresses):
```
s [N]              — step N instructions
c                  — continue (fast path or wp loop)
x/Ni [paddr]       — disassemble N insts at physical addr (default: pc)
x/Nx [paddr]       — examine N hex words (default: pc)
x/Nb [paddr]       — examine N bytes
b <paddr>          — set breakpoint, returns ID
bd <id>            — delete breakpoint by ID
bl                 — list breakpoints (ID + address)
w <expr>           — watch expression
wd <id>            — delete watchpoint by ID
wl                 — list watchpoints
p <expr>           — evaluate and print
info reg [name]    — register dump
l <file>           — load binary
r                  — reset
q                  — quit

Note: all addresses are physical. When MMU paging is active,
use LOG=trace to observe virtual→physical translation.
```

Logging levels:
```
LOG=trace  → per-instruction: "0x80000000: 00000297  auipc t0, 0"
LOG=debug  → per-memory-access: "R [0x80001000+4] = 0xdeadbeef"
LOG=info   → lifecycle (load, trap, halt)
LOG=off    → silent
```

[**Constraints**]
- C-1: `chumsky` for expr parser
- C-2: `regex` for pre-parser
- C-3: Physical addresses only. Documented.
- C-4: `cfg(feature = "debug")` for all debug code
- C-5: `CoreDebugOps` + `DebugOps` traits, `CPU` bounded by both
- C-6: Breakpoint IDs: `u32`, monotonic, never reused
- C-7: wp step loop when active, `run()` when inactive
- C-8: `set_skip_bp()` for step-after-breakpoint

---

## Implement

### Implementation Plan

[**Phase 1: xcore traits + breakpoints + logging**]
- `xcore/Cargo.toml` — `debug` feature
- `xcore/src/cpu/debug.rs` — `Breakpoint`, `CoreDebugOps`, `DebugOps`, `format_mnemonic()`
- `xcore/src/cpu/riscv/debug.rs` — impl both traits for RVCore
- `xcore/src/cpu/riscv/mod.rs` — bp fields, bp check in `step()`, `trace!()` per inst
- `xcore/src/cpu/riscv/mm.rs` — `debug!()` per memory access
- `xcore/src/cpu/mod.rs` — `CPU` pass-through bounded by `Core: CoreDebugOps + DebugOps`
- `xcore/src/error.rs` — `XError::DebugBreak(usize)`

[**Phase 2: xdb commands**]
- `xdb/Cargo.toml` — `regex`; enable xcore `debug`
- `xdb/src/cli.rs` — `preprocess_line()`, expanded Commands
- `xdb/src/cmd.rs` — `cmd_break*`, `cmd_examine`, `cmd_info`
- `xdb/src/main.rs` — `respond()` with `DebugBreak` handling, `WatchManager`

[**Phase 3: Expression evaluator + watchpoints**]
- `xdb/Cargo.toml` — add `chumsky`
- `xdb/src/expr.rs` — chumsky parser + evaluator
- `xdb/src/watchpoint.rs` — `WatchManager`
- `cmd_print`, `cmd_watch*`, wp step loop in `cmd_continue`/`cmd_step`

---

## Validation

[**Unit Tests**]
- V-UT-1: Expression parser
- V-UT-2: format_mnemonic (all DecodedInst variants)
- V-UT-3: CoreDebugOps: add/remove/list breakpoints, stable IDs
- V-UT-4: WatchManager: add/remove/check, value-change detection
- V-UT-5: Pre-parser transforms
- V-UT-6: DebugOps: read_register all names, read_memory valid/invalid

[**Integration Tests**]
- V-IT-1: Breakpoint stops at addr, `set_skip_bp` advances past
- V-IT-2: Watchpoint triggers during continue (step loop)
- V-IT-3: `x/5i 0x80000000` disassembles 5 instructions
- V-IT-4: `x/4x 0x80000000` shows 4 hex words
- V-IT-5: `bd <id>` deletes correct breakpoint regardless of list order

[**Edge Cases**]
- V-E-1: Step at breakpoint — advances one instruction
- V-E-2: Delete bp by ID, verify other IDs unchanged
- V-E-3: `x/4x 0x02000000` → "not in RAM"
- V-E-4: Continue without watchpoints → fast path
- V-E-5: `p $sp + 4 * 2` — correct precedence

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (breakpoints) | V-IT-1, V-IT-5 |
| G-2 (watchpoints) | V-IT-2 |
| G-3 (expr eval) | V-UT-1, V-E-5 |
| G-4 (disasm) | V-IT-3 |
| G-5 (memory) | V-IT-4 |
| G-6 (registers) | V-UT-6 |
| G-7 (logging) | LOG=trace verified |
| I-2 (trait bounds) | CPU impl bounded by both traits |
| I-3 (physical) | V-E-3 |
| I-4 (stable IDs) | V-E-2 |
