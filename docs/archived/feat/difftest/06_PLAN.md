# `difftest` PLAN `06`

> Status: Revised
> Feature: `difftest`
> Iteration: `06`
> Owner: Executor
> Depends on:
> - Previous Plan: `05_PLAN.md`
> - Review: `05_REVIEW.md`
> - Master Directive: `05_MASTER.md`

---

## Summary

Fixes all round-05 blockers. Key changes: (1) `diff()` is a free function in xdb, not an inherent impl on foreign type; (2) `read_register` restored in DebugOps for full debugger CSR coverage — CoreContext carries the difftest whitelist, DebugOps covers the rest; (3) CoreContext carries `gprs: Vec<(&'static str, u64)>` named pairs so xdb never references `RVReg`; (4) `cmd_continue` explicitly falls back to per-step loop when difftest attached; (5) difftest CSR whitelist uses `CsrAddr` variants directly from `csr_table!` macro — no separate `DIFF_CSRS` array; (6) missing CSR counterpart in diff = mismatch.

## Log

[**Feature Introduce**]

- `diff_contexts()` free function in xdb — takes two `&CoreContext`, returns `Option<DiffMismatch>`.
- `CoreContext::gprs` as `Vec<(&'static str, u64)>` — self-describing, arch-neutral for xdb.
- `cmd_continue` difftest integration: when harness attached, use per-step loop (no `cpu.run()` bypass).
- CSR whitelist integrated with `csr_table!`: uses `CsrAddr::mstatus` etc. directly.
- Missing CSR counterpart in diff = mismatch (fail loud).

[**Review Adjustments**]

- R-001 (inherent impl on foreign type): `diff()` moved to free function `diff_contexts()` in xdb.
- R-002 (debugger CSR regression): `read_register` restored in DebugOps. CoreContext carries difftest whitelist; debugger uses `read_register` for arbitrary CSRs beyond whitelist.
- R-003 (RISC-V leaks into xdb): CoreContext carries `gprs: Vec<(&'static str, u64)>` named pairs. xdb iterates names, never imports `RVReg`. Non-RISC-V branch documented as gated out for this round.
- R-004 (continue fast path): `cmd_continue` explicitly checks difftest harness. If attached, falls back to per-step loop with `check_step()` per instruction.
- R-005 (missing CSR = silent skip): Changed to mismatch. Both contexts must have same CSR addr set.
- R-006 (Spike pin date): Corrected to v1.1.0 / `530af85` / 2021-12-17.

[**Master Compliance**]

- M-001 (functional style, clean code): Used iterator chains, `map`/`collect` for context construction. Minimal structs.
- M-002 (integrate with csr_table!): Removed separate `DIFF_CSRS`/`CsrValue` types. Whitelist uses `CsrAddr` enum variants. CSR snapshot is `Vec<(CsrAddr, &'static str, u64, u64)>` tuples (addr, name, mask, raw_value) — built from the existing CSR framework.
- M-003 (CoreContext as parameter, not self): `diff_contexts(&dut, &ref, count)` free function. CoreContext is passed as `&CoreContext` parameter everywhere — no methods impl'd on it from xdb.

### Changes from Previous Round

[**Added**]
- `diff_contexts()` free function in xdb
- `CoreContext::gprs` as named pairs
- `cmd_continue` difftest-aware path
- Missing CSR = mismatch (fail loud)

[**Changed**]
- `CoreContext::diff()` inherent impl -> `diff_contexts()` free fn
- `CsrValue` struct -> inline tuple `(u16, &str, u64, u64)` in CoreContext
- `read_register` restored in DebugOps
- `CoreContext::gpr: [u64; 32]` -> `CoreContext::gprs: Vec<(&str, u64)>` named
- ISA whitelist: separate const array -> `CsrAddr` enum references
- Spike pin: corrected date

[**Removed**]
- `CsrValue` struct (M-002: integrate into CSR framework)
- `DIFF_CSRS` separate const array
- `register_by_name()` inherent impl on CoreContext (can't add to foreign type; use free fn or DebugOps)

[**Unresolved**]
- Spike upstream API (mitigated: pinned version + separate feature)
- mtime drift (mitigated: MTIP mask)

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Free function `diff_contexts()` in xdb |
| Review | R-002 | Accepted | `read_register` restored in DebugOps |
| Review | R-003 | Accepted | Named `gprs` pairs; non-RISC-V gated out explicitly |
| Review | R-004 | Accepted | `cmd_continue` per-step when difftest attached |
| Review | R-005 | Accepted | Missing CSR = mismatch |
| Review | R-006 | Accepted | Spike v1.1.0 / 530af85 / 2021-12-17 |
| Master | M-001 | Applied | Functional iterator chains |
| Master | M-002 | Applied | No separate CsrValue; uses CsrAddr from csr_table! |
| Master | M-003 | Applied | CoreContext as parameter; diff is free fn |

---

## Spec

[**Goals**]

- G-1: Per-instruction state comparison (DUT vs REF), halt on first divergence.
- G-2: Two backends: QEMU (production, `difftest`) and Spike (experimental, `difftest-spike`).
- G-3: Compare PC + GPRs + privilege + whitelisted CSRs (masked).
- G-4: MMIO-skip: sync DUT->REF using raw CSR values.
- G-5: Feature-gated. xdb difftest module single-gated. xcore has always-compiled CoreContext + difftest-gated MMIO hook.
- G-6: Monitor commands: `dt attach qemu|spike`, `dt detach`, `dt status`.
- G-7: Interrupt-preserving: QEMU `sstep=0x1`.
- G-8: Backend-neutral xcore. No QEMU/Spike protocol in xcore.
- G-9: Physical-memory: QEMU `PhyMemMode:1`.
- G-10: CoreContext is primary data carrier. DebugOps retained only for memory/disasm + arbitrary CSR reads.

- NG-1: Memory comparison deferred.
- NG-2: Full CSR dump deferred.
- NG-3: mtime sync deferred.

[**Architecture**]

```
xcore                              xdb
+---------------------------+      +------------------------------------+
|  RVCore                   |      |                                    |
|  +--------+               |      |  CoreContext (&ctx parameter)      |
|  |DebugOps|--context()----+----->|  +- pc: u64                       |
|  |        |               |      |  +- gprs: [("ra",v), ("sp",v)...] |
|  |        |--read_register|      |  +- privilege: u64                |
|  |        |--read_memory()|      |  +- csrs: [(addr,name,mask,raw)] |
|  |        |--fetch_inst() |      |  +- word_size, isa                |
|  |        |--disasm_raw() |      |  +---+----------------------------+
|  +--------+               |      |      |                             |
|                           |      |  diff_contexts(&dut, &ref, n)     |
|  Bus (cfg difftest)       |      |      |                             |
|  +--------+               |      |  +---v-----------+                |
|  |mmio_   |--take_flag()--+----->|  | DiffHarness   |                |
|  |accessed|               |      |  +---+-----------+                |
|  +--------+               |      |      |                             |
+---------------------------+      |  +---v---+ +------+               |
                                   |  |QEMU   | |Spike |               |
                                   |  |GDB RSP| |FFI   |               |
                                   |  +-------+ +------+               |
                                   +------------------------------------+
```

[**Invariants**]

- I-1: DUT first, then REF. Compare after both.
- I-2: MMIO -> sync raw values, skip compare.
- I-3: ebreak -> sync, skip.
- I-4: QEMU `sstep=0x1`.
- I-5: QEMU `PhyMemMode:1`.
- I-6: CoreContext is Clone, plain data, no references. Passed as `&CoreContext`.
- I-7: Comparison uses masked CSRs. Sync uses raw.
- I-8: Timer CSRs excluded. mip mask = `!0x82`.
- I-9: Missing CSR counterpart = mismatch.
- I-10: `cmd_continue` with difftest attached uses per-step loop.
- I-11: `read_register` covers full CSR namespace for debugger. CoreContext covers difftest whitelist.
- I-12: xdb never imports arch-specific register types (no `RVReg` in xdb).

[**Data Structure**]

```rust
// === xcore/src/cpu/riscv/context.rs (always compiled) ===

/// Arch-specific core context snapshot.
/// Plain data, Clone. Carries self-describing named registers.
#[derive(Clone)]
pub struct RVCoreContext {
    pub pc: u64,
    pub gprs: Vec<(&'static str, u64)>,   // [("zero",0), ("ra",v), ...]
    pub privilege: u64,
    pub csrs: Vec<CsrSnapshot>,           // difftest whitelist
    pub word_size: usize,
    pub isa: &'static str,
}

/// CSR snapshot entry — uses CsrAddr from csr_table! macro.
#[derive(Clone, Copy)]
pub struct CsrSnapshot {
    pub addr: u16,
    pub name: &'static str,
    pub mask: u64,         // comparison mask
    pub raw_value: u64,    // unmasked, for sync
}

impl CsrSnapshot {
    pub fn masked(&self) -> u64 { self.raw_value & self.mask }
}

// === dispatched in cpu/mod.rs ===
// pub use self::riscv::context::{RVCoreContext as CoreContext, CsrSnapshot};

// === xdb/src/difftest/mod.rs ===

pub struct DiffMismatch {
    pub inst_count: u64,
    pub reg_name: &'static str,
    pub dut_val: u64,
    pub ref_val: u64,
}

pub trait DiffBackend {
    fn step(&mut self) -> Result<(), String>;
    fn read_context(&mut self) -> Result<CoreContext, String>;
    fn sync_state(&mut self, ctx: &CoreContext) -> Result<(), String>;
    fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String>;
    fn name(&self) -> &str;
}

pub struct DiffHarness {
    backend: Box<dyn DiffBackend>,
    inst_count: u64,
}

/// Compare two contexts. Free function — not inherent impl on foreign type.
pub fn diff_contexts(
    dut: &CoreContext, refr: &CoreContext, inst_count: u64
) -> Option<DiffMismatch> { ... }
```

[**API Surface**]

```rust
// -- xcore DebugOps (trimmed but read_register preserved) --
pub trait DebugOps: super::CoreOps {
    fn context(&self) -> super::CoreContext;     // snapshot for difftest + info reg
    fn read_register(&self, name: &str) -> Option<u64>;  // full CSR namespace for debugger
    fn read_memory(&self, paddr: usize, size: usize) -> XResult<u64>;
    fn fetch_inst(&self, paddr: usize) -> XResult<u32>;
    fn disasm_raw(&self, raw: u32) -> String;
    // breakpoints unchanged
    fn add_breakpoint(&mut self, addr: usize) -> u32;
    fn remove_breakpoint(&mut self, id: u32) -> bool;
    fn list_breakpoints(&self) -> &[Breakpoint];
    fn set_skip_bp(&mut self);
}

// dump_registers() removed — use context().gprs instead.

// -- xdb difftest (free functions, CoreContext as parameter) --
pub fn diff_contexts(dut: &CoreContext, refr: &CoreContext, count: u64) -> Option<DiffMismatch>;

impl DiffHarness {
    pub fn new(backend: Box<dyn DiffBackend>) -> Self;
    pub fn check_step(&mut self, dut_ctx: &CoreContext, mmio: bool, halted: bool)
        -> Result<Option<DiffMismatch>, String>;
}
```

[**CSR Whitelist (integrated with csr_table!)**]

In `xcore/src/cpu/riscv/context.rs`, the whitelist uses `CsrAddr` variants directly:

```rust
use super::csr::CsrAddr;

/// Difftest CSR whitelist. Uses CsrAddr from csr_table! macro.
const DIFFTEST_CSRS: &[(CsrAddr, u64)] = &[
    (CsrAddr::mstatus,  u64::MAX),
    (CsrAddr::mtvec,    u64::MAX),
    (CsrAddr::mepc,     u64::MAX),
    (CsrAddr::mcause,   u64::MAX),
    (CsrAddr::mtval,    u64::MAX),
    (CsrAddr::medeleg,  u64::MAX),
    (CsrAddr::mideleg,  u64::MAX),
    (CsrAddr::mie,      u64::MAX),
    (CsrAddr::mip,      !0x82_u64),
    (CsrAddr::stvec,    u64::MAX),
    (CsrAddr::sepc,     u64::MAX),
    (CsrAddr::scause,   u64::MAX),
    (CsrAddr::stval,    u64::MAX),
    (CsrAddr::satp,     u64::MAX),
];
```

This eliminates the duplicate `DIFF_CSRS` raw-address table. The `CsrAddr` enum is the single source of truth from `csr_table!`.

[**Constraints**]

- C-1: QEMU in PATH for `difftest`. Spike source for `difftest-spike`.
- C-2: GDB port 1234.
- C-3: Bare-metal `-bios`.
- C-4: MMIO hook `cfg(feature = "difftest")` in bus.rs.
- C-5: QEMU `-M virt`, RAM at 0x80000000.
- C-6: `difftest` depends on `debug`. `difftest-spike` depends on `difftest`.
- C-7: `dt attach` requires runtime binary path.
- C-8: QEMU `sstep=0x1`, `PhyMemMode:1`. Unsupported = failure.
- C-9: ISA string cfg-derived: `"rv64imac"` / `"rv32imac"`. Fixed subset.
- C-10: RAM-write sync limitation. Documented non-goal.
- C-11: Spike experimental, pinned v1.1.0 / 530af85 / 2021-12-17.
- C-12: Non-RISC-V arch gated out for this round. `CoreContext` dispatch only covers `#[cfg(riscv)]`.

---

## Implement

### Execution Flow

[**Main Flow**]

1. `DIFFTEST=1 make run` -> `--features difftest`.
2. `load <file>` -> xdb stores runtime path.
3. `dt attach qemu`:
   a. `with_xcpu(|cpu| cpu.context())` -> get ISA/word_size.
   b. Spawn QEMU, connect GDB.
   c. `sstep=0x1` + `PhyMemMode:1`. Fail if unsupported.
   d. Run to reset vector. Create DiffHarness.
4. `s` or `c`:
   a. Step DUT: `with_xcpu(|cpu| { cpu.step()?; Ok((cpu.context(), cpu.is_terminated())) })`.
   b. MMIO flag: `with_xcpu(|cpu| cpu.bus_take_mmio_flag())`.
   c. `harness.check_step(&ctx, mmio, halted)`:
      - `backend.step()`.
      - mmio/halted -> `backend.sync_state(&ctx)` (raw values) -> None.
      - else -> `backend.read_context()`, `diff_contexts(&ctx, &ref_ctx, count)`.
   d. Mismatch -> report, halt.
5. `info reg` -> `with_xcpu(|cpu| cpu.context())` -> iterate `ctx.gprs`.
6. `info reg <name>` -> `with_xcpu(|cpu| cpu.debug_ops().read_register(name))` (full CSR coverage).
7. `p $csr` / `w $csr` -> `read_register(name)` for any CSR; `read_memory` for `*deref`.

[**Continue path (R-004 fix)**]

```rust
pub fn cmd_continue(watch_mgr: &mut WatchManager, diff: &mut Option<DiffHarness>) -> XResult {
    // Fast path: no watchpoints AND no difftest -> cpu.run()
    if watch_mgr.is_empty() && diff.is_none() {
        return with_xcpu!(run(u64::MAX));
    }
    // Slow path: per-step loop for watchpoints and/or difftest
    loop {
        let (ctx, done) = with_xcpu(|cpu| -> XResult<(CoreContext, bool)> {
            cpu.step()?;
            Ok((cpu.context(), cpu.is_terminated()))
        })?;
        if done { break; }
        // Difftest check
        if let Some(ref mut h) = diff {
            let mmio = with_xcpu(|cpu| cpu.bus_take_mmio_flag());
            match h.check_step(&ctx, mmio, false) {
                Ok(Some(m)) => { DiffHarness::report_mismatch(&m); return Ok(()); }
                Ok(None) => {}
                Err(e) => { println!("Difftest error: {e}"); *diff = None; return Ok(()); }
            }
        }
        if let Some(hit) = check_watchpoints(watch_mgr) {
            println!("{hit}"); return Ok(());
        }
    }
    Ok(())
}
```

### Implementation Plan

[**Phase 1: CoreContext in xcore (~60 lines)**]

New file: `xcore/src/cpu/riscv/context.rs`

```rust
use super::csr::CsrAddr;
use crate::{config::Word, isa::RVReg};

#[derive(Clone, Copy)]
pub struct CsrSnapshot {
    pub addr: u16,
    pub name: &'static str,
    pub mask: u64,
    pub raw_value: u64,
}

impl CsrSnapshot {
    pub fn masked(&self) -> u64 { self.raw_value & self.mask }
}

#[derive(Clone)]
pub struct RVCoreContext {
    pub pc: u64,
    pub gprs: Vec<(&'static str, u64)>,
    pub privilege: u64,
    pub csrs: Vec<CsrSnapshot>,
    pub word_size: usize,
    pub isa: &'static str,
}

/// Difftest CSR whitelist — references CsrAddr from csr_table! macro.
const DIFFTEST_CSRS: &[(CsrAddr, u64)] = &[
    (CsrAddr::mstatus,  u64::MAX),
    (CsrAddr::mtvec,    u64::MAX),
    (CsrAddr::mepc,     u64::MAX),
    (CsrAddr::mcause,   u64::MAX),
    (CsrAddr::mtval,    u64::MAX),
    (CsrAddr::medeleg,  u64::MAX),
    (CsrAddr::mideleg,  u64::MAX),
    (CsrAddr::mie,      u64::MAX),
    (CsrAddr::mip,      !0x82_u64),
    (CsrAddr::stvec,    u64::MAX),
    (CsrAddr::sepc,     u64::MAX),
    (CsrAddr::scause,   u64::MAX),
    (CsrAddr::stval,    u64::MAX),
    (CsrAddr::satp,     u64::MAX),
];
```

In `xcore/src/cpu/riscv/debug.rs`, add `context()` to DebugOps impl:

```rust
fn context(&self) -> crate::cpu::CoreContext {
    use super::context::*;
    let pc = self.pc.as_usize() as u64;
    let gprs = (0u8..32)
        .map(|i| {
            let r = RVReg::from_u8(i).unwrap();
            (r.name(), word_to_u64(self.gpr[i as usize]))
        })
        .collect();
    let csrs = DIFFTEST_CSRS.iter()
        .map(|&(addr, mask)| {
            let raw = word_to_u64(self.csr.get(addr));
            let name = CsrAddr::from_name_by_addr(addr as u16)
                .unwrap_or("???");
            CsrSnapshot { addr: addr as u16, name, mask, raw_value: raw }
        })
        .collect();
    RVCoreContext {
        pc, gprs,
        privilege: self.privilege as u64,
        csrs,
        word_size: std::mem::size_of::<Word>(),
        isa: if cfg!(isa64) { "rv64imac" } else { "rv32imac" },
    }
}
```

Note: `CsrAddr` already has `from_name()` from `csr_table!`. We need a reverse: addr-to-name. Add to the `csr_table!` macro a `name()` method:

```rust
// In csr_table! macro expansion, add:
impl CsrAddr {
    pub fn name(self) -> &'static str {
        match self {
            $( Self::$name => stringify!($name), )*
        }
    }
}
```

Then context construction simplifies to:

```rust
let csrs = DIFFTEST_CSRS.iter()
    .map(|&(addr, mask)| CsrSnapshot {
        addr: addr as u16,
        name: addr.name(),
        mask,
        raw_value: word_to_u64(self.csr.get(addr)),
    })
    .collect();
```

Dispatch in `cpu/mod.rs`:

```rust
cfg_if::cfg_if! {
    if #[cfg(riscv)] {
        pub use self::riscv::context::{RVCoreContext as CoreContext, CsrSnapshot};
    }
}
```

Re-export in `lib.rs`:

```rust
pub use cpu::{CoreContext, CsrSnapshot};
```

[**Phase 2: DebugOps update**]

In `xcore/src/cpu/debug.rs`:

```rust
pub trait DebugOps: super::CoreOps {
    fn context(&self) -> super::CoreContext;
    fn read_register(&self, name: &str) -> Option<u64>;  // kept for full debugger coverage
    fn read_memory(&self, paddr: usize, size: usize) -> XResult<u64>;
    fn fetch_inst(&self, paddr: usize) -> XResult<u32>;
    fn disasm_raw(&self, raw: u32) -> String;
    fn add_breakpoint(&mut self, addr: usize) -> u32;
    fn remove_breakpoint(&mut self, id: u32) -> bool;
    fn list_breakpoints(&self) -> &[Breakpoint];
    fn set_skip_bp(&mut self);
}
// dump_registers() removed — replaced by context().gprs
```

CPU pass-through:

```rust
impl<Core: CoreOps + DebugOps> CPU<Core> {
    pub fn context(&self) -> CoreContext { self.core.context() }
    // read_register, read_memory, etc. still available via debug_ops()
}
```

[**Phase 3: Adapt xdb to CoreContext**]

`info reg` (all):

```rust
pub fn cmd_info(what: &str, name: Option<&str>) -> XResult {
    match what {
        "reg" | "r" => with_xcpu(|cpu| {
            match name {
                Some(n) => match cpu.debug_ops().read_register(n) {
                    Some(val) => println!("{n} = {val:#x}"),
                    None => println!("Unknown register: {n}"),
                },
                None => {
                    let ctx = cpu.context();
                    print!("{:>10} = {:#018x}\n", "pc", ctx.pc);
                    for (i, (name, val)) in ctx.gprs.iter().enumerate() {
                        print!("{name:>10} = {val:#018x}");
                        if (i + 1) % 4 == 0 { println!(); } else { print!("  "); }
                    }
                    println!();
                }
            }
        }),
        _ => println!("Unknown info target: {what}. Try: reg"),
    }
    Ok(())
}
```

`p <expr>` / `w <expr>` — register lookup via `read_register` (full CSR coverage), memory via `read_memory`:

```rust
// Unchanged — still uses DebugOps callbacks:
let ops = cpu.debug_ops();
eval_expr(expr, |name| ops.read_register(name), |addr, sz| ops.read_memory(addr, sz).ok())
```

[**Phase 4: GDB Client — `xdb/src/difftest/gdb.rs` (~200 lines)**]

Same as previous rounds.

[**Phase 5: QEMU Backend — `xdb/src/difftest/qemu.rs` (~140 lines)**]

```rust
fn csr_addr_to_qemu_regnum(addr: u16) -> usize { 4096 + addr as usize }
const QEMU_PRIV_REGNUM: usize = 4161;

fn qemu_bin_for_isa(isa: &str) -> &'static str {
    if isa.starts_with("rv64") { "qemu-system-riscv64" }
    else { "qemu-system-riscv32" }
}

pub struct QemuBackend {
    proc: Child,
    gdb: GdbClient,
    csr_meta: Vec<(u16, &'static str, u64)>, // (addr, name, mask) from init_ctx
    word_size: usize,
    isa: &'static str,
}

impl QemuBackend {
    pub fn new(binary_path: &str, reset_vec: usize, init_ctx: &CoreContext)
        -> Result<Self, String> {
        // ... spawn, sstep=0x1, PhyMemMode:1, run to reset_vec ...
        let csr_meta = init_ctx.csrs.iter()
            .map(|c| (c.addr, c.name, c.mask))
            .collect();
        Ok(Self { proc, gdb, csr_meta, word_size: init_ctx.word_size, isa: init_ctx.isa })
    }
}

impl DiffBackend for QemuBackend {
    fn read_context(&mut self) -> Result<CoreContext, String> {
        let regs = self.gdb.read_regs()?;
        // Build gprs from register dump — positional, name from init context would require
        // passing names. Instead use generic x0..x31 names (arch-neutral).
        let gprs: Vec<_> = (0..32)
            .map(|i| {
                let name: &'static str = /* from cached init_ctx.gprs names */;
                (name, regs[i])
            })
            .collect();
        let csrs: Vec<_> = self.csr_meta.iter()
            .map(|&(addr, name, mask)| {
                let raw = self.gdb.read_register(csr_addr_to_qemu_regnum(addr))
                    .unwrap_or(0);
                CsrSnapshot { addr, name, mask, raw_value: raw }
            })
            .collect();
        let priv_mode = self.gdb.read_register(QEMU_PRIV_REGNUM).unwrap_or(0);
        Ok(CoreContext {
            pc: regs[32], gprs, privilege: priv_mode, csrs,
            word_size: self.word_size, isa: self.isa,
        })
    }

    fn sync_state(&mut self, ctx: &CoreContext) -> Result<(), String> {
        let mut regs: Vec<u64> = ctx.gprs.iter().map(|(_, v)| *v).collect();
        regs.push(ctx.pc);
        self.gdb.write_regs(&regs)?;
        for csr in &ctx.csrs {
            self.gdb.write_register(
                csr_addr_to_qemu_regnum(csr.addr),
                csr.raw_value,  // raw! not masked
            )?;
        }
        Ok(())
    }
}
```

[**Phase 6: Spike Backend — `xdb/src/difftest/spike.rs` (cfg difftest-spike)**]

Same FFI design. Pinned to riscv-isa-sim v1.1.0 / 530af85. Only built when `difftest-spike` enabled.

[**Phase 7: DiffHarness + diff_contexts — `xdb/src/difftest/mod.rs`**]

```rust
/// Compare two contexts. Free function, not inherent impl.
pub fn diff_contexts(
    dut: &CoreContext, refr: &CoreContext, inst_count: u64,
) -> Option<DiffMismatch> {
    // PC
    if dut.pc != refr.pc {
        return Some(DiffMismatch { inst_count, reg_name: "pc",
            dut_val: dut.pc, ref_val: refr.pc });
    }
    // GPRs (skip x0, compare by position — both ordered identically)
    for (i, ((dname, dval), (_, rval))) in
        dut.gprs.iter().zip(refr.gprs.iter()).enumerate()
    {
        if i == 0 { continue; } // x0 always zero
        if dval != rval {
            return Some(DiffMismatch { inst_count, reg_name: dname,
                dut_val: *dval, ref_val: *rval });
        }
    }
    // Privilege
    if dut.privilege != refr.privilege {
        return Some(DiffMismatch { inst_count, reg_name: "privilege",
            dut_val: dut.privilege, ref_val: refr.privilege });
    }
    // CSRs — match by addr, fail loud on missing
    for dut_csr in &dut.csrs {
        match refr.csrs.iter().find(|c| c.addr == dut_csr.addr) {
            Some(ref_csr) => {
                if dut_csr.masked() != ref_csr.masked() {
                    return Some(DiffMismatch { inst_count, reg_name: dut_csr.name,
                        dut_val: dut_csr.masked(), ref_val: ref_csr.masked() });
                }
            }
            None => {
                return Some(DiffMismatch { inst_count,
                    reg_name: dut_csr.name,
                    dut_val: dut_csr.masked(), ref_val: 0 });
            }
        }
    }
    None
}

impl DiffHarness {
    pub fn check_step(&mut self, dut_ctx: &CoreContext, mmio: bool, halted: bool)
        -> Result<Option<DiffMismatch>, String> {
        self.inst_count += 1;
        self.backend.step()?;
        if mmio || halted {
            self.backend.sync_state(dut_ctx)?;
            return Ok(None);
        }
        let ref_ctx = self.backend.read_context()?;
        Ok(diff_contexts(dut_ctx, &ref_ctx, self.inst_count))
    }
}
```

[**Phase 8: Build System**]

```toml
# xcore/Cargo.toml
[features]
debug = []
difftest = ["debug"]

# xdb/Cargo.toml
[features]
debug = ["xcore/debug"]
difftest = ["xcore/difftest"]
difftest-spike = ["difftest"]

[build-dependencies]
cc = "1"
```

```rust
// xdb/build.rs
fn main() {
    if std::env::var("CARGO_FEATURE_DIFFTEST_SPIKE").is_ok() {
        // build spike_wrapper.cc, link libs
    }
}
```

```makefile
ifeq ($(DIFFTEST),1)
  feature_args += --features difftest
endif
ifeq ($(SPIKE),1)
  feature_args += --features difftest-spike
endif
```

### File Summary

| File | Crate | New/Mod | Description |
|------|-------|---------|-------------|
| `xcore/src/cpu/riscv/context.rs` | xcore | NEW | RVCoreContext, CsrSnapshot, DIFFTEST_CSRS |
| `xcore/src/cpu/riscv/csr.rs` | xcore | MOD | +CsrAddr::name() in csr_table! macro |
| `xcore/src/cpu/riscv/debug.rs` | xcore | MOD | +context() impl, -dump_registers |
| `xcore/src/cpu/riscv/mod.rs` | xcore | MOD | +pub mod context |
| `xcore/src/cpu/debug.rs` | xcore | MOD | +context(), -dump_registers |
| `xcore/src/cpu/mod.rs` | xcore | MOD | +CoreContext/CsrSnapshot dispatch |
| `xcore/src/device/bus.rs` | xcore | MOD | +AtomicBool (cfg difftest) |
| `xcore/src/lib.rs` | xcore | MOD | +re-export CoreContext, CsrSnapshot |
| `xdb/src/difftest/mod.rs` | xdb | NEW | DiffBackend, DiffHarness, diff_contexts() |
| `xdb/src/difftest/gdb.rs` | xdb | NEW | GdbClient |
| `xdb/src/difftest/qemu.rs` | xdb | NEW | QemuBackend |
| `xdb/src/difftest/spike.rs` | xdb | NEW | SpikeBackend (cfg difftest-spike) |
| `xdb/build.rs` | xdb | NEW | Spike cc build |
| `xdb/src/cmd.rs` | xdb | MOD | info reg via ctx.gprs, dt cmds, continue fix |
| `xdb/src/cli.rs` | xdb | MOD | Dt subcommand |
| `xdb/src/main.rs` | xdb | MOD | Wiring, runtime path |

---

## Trade-offs

- T-1: **Raw vs masked in CsrSnapshot** — Both stored. `masked()` for compare, `raw_value` for sync. Correct.
- T-2: **read_register kept** — CoreContext covers difftest whitelist. `read_register` covers full debugger namespace. Clear separation.
- T-3: **dump_registers removed** — Replaced by `context().gprs`. One fewer DebugOps method, same functionality.
- T-4: **CSR compare by addr** — O(n*m) but n=m=14. Negligible. Catches missing entries.
- T-5: **GPR names in context** — Vec of `(&str, u64)` instead of `[u64; 32]`. Slightly more allocation. Worth it: xdb never imports RVReg.

---

## Validation

[**Unit Tests**]

- V-UT-1: GDB packet checksum.
- V-UT-2: parse_gdb_regs RV64/RV32.
- V-UT-3: parse/encode hex round-trip.
- V-UT-4: diff_contexts identical -> None.
- V-UT-5: diff_contexts PC mismatch.
- V-UT-6: diff_contexts GPR mismatch.
- V-UT-7: diff_contexts privilege mismatch.
- V-UT-8: diff_contexts CSR masked mismatch.
- V-UT-9: diff_contexts masked mip bits ignored.
- V-UT-10: diff_contexts missing CSR = mismatch.
- V-UT-11: CsrSnapshot::masked() correct.
- V-UT-12: CsrSnapshot::raw_value preserved in sync.
- V-UT-13: context() captures correct state.
- V-UT-14: CsrAddr::name() matches stringify.

[**Integration Tests**]

- V-IT-1: Difftest (QEMU) on cpu-tests-rs — zero divergence.
- V-IT-2: Difftest (QEMU) on am-tests — MMIO skip.
- V-IT-3: Difftest (Spike) on cpu-tests-rs (if difftest-spike).
- V-IT-4: Intentional divergence caught.
- V-IT-5: `dt attach`/`detach`/`status` lifecycle.
- V-IT-6: `dt attach` without load -> error.
- V-IT-7: `info reg` uses context().gprs.
- V-IT-8: `info reg mstatus` uses read_register (full coverage).
- V-IT-9: `c` with difftest attached uses per-step loop.

[**Failure / Robustness**]

- V-F-1: QEMU not in PATH.
- V-F-2: sstep unsupported.
- V-F-3: PhyMemMode unsupported.
- V-F-4: Spike init fails (difftest-spike only).

[**Edge Cases**]

- V-E-1: First instruction divergence.
- V-E-2: MMIO then non-MMIO — sync raw, resume masked compare.
- V-E-3: Compressed instruction.
- V-E-4: Trap (ecall).
- V-E-5: mret/sret privilege.
- V-E-6: ebreak halt sync preserves raw mip.
- V-E-7: Timer interrupt with sstep=0x1.
- V-E-8: `continue` with difftest catches divergence mid-run.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-1, V-IT-4, V-UT-4..10 |
| G-2 | V-IT-1+3 |
| G-3 | V-UT-5..9, V-E-4, V-E-5 |
| G-4 | V-IT-2, V-E-2, V-UT-12 |
| G-5 | Build without: zero code |
| G-6 | V-IT-5, V-IT-6 |
| G-7 | V-E-7 |
| G-8 | No RVReg in xdb |
| G-9 | V-F-3 |
| G-10 | V-IT-7, V-IT-8 |
| I-9 (missing CSR) | V-UT-10 |
| I-10 (continue) | V-IT-9, V-E-8 |
