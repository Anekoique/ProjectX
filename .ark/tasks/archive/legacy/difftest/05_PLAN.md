# `difftest` PLAN `05`

> Status: Revised
> Feature: `difftest`
> Iteration: `05`
> Owner: Executor
> Depends on:
> - Previous Plan: `04_PLAN.md`
> - Review: `04_REVIEW.md`
> - Master Directive: `04_MASTER.md`

---

## Summary

Addresses all round-04 blockers and master directives. Key changes: (1) `CsrValue` carries both raw and masked values — sync uses raw, comparison uses masked; (2) Spike behind separate `difftest-spike` feature, QEMU-only difftest works without Spike installed; (3) CSR whitelist exported from xcore as single source of truth — backends derive register mappings from `CsrValue::addr`; (4) Spike pinned to concrete version; (5) `CoreContext` is arch-dependent (`RVCoreContext as CoreContext`), defined per-ISA; (6) DebugOps trimmed — `read_register`/`dump_registers` removed, replaced by `CoreContext` fields; only memory/disasm methods remain.

## Log

[**Feature Introduce**]

- `CsrValue` with raw + masked: `raw_value` for sync, `masked()` for comparison.
- `difftest-spike` separate feature. `difftest` = QEMU only. `difftest-spike` = QEMU + Spike.
- Single CSR whitelist exported from xcore — backends map `CsrValue::addr` to backend-specific register numbers.
- `CoreContext` arch-dispatched: `pub use RVCoreContext as CoreContext`.
- DebugOps slimmed: removed `read_register`, `dump_registers`. `CoreContext` replaces them.
- Spike pinned to riscv-software-src/riscv-isa-sim v1.1.0.
- ISA string honestly documented as cfg-derived, not "runtime-derived".

[**Review Adjustments**]

- R-001 (masked CSR in sync): `CsrValue` now stores raw value. `masked()` method for comparison. `sync_state()` writes raw. Fixed.
- R-002 (Spike on mandatory build): Split to `difftest-spike` feature. `difftest` alone = QEMU only.
- R-003 (duplicate CSR table): Removed QEMU-side `DIFF_CSR_REF`. Backends read `CoreContext::csrs` as single source of truth. Comparison validates by `addr` not index.
- R-004 (Spike placeholder hash): Pinned to riscv-isa-sim v1.1.0 (2023-12 release).
- R-005 (ISA hard-coded): Honestly documented as cfg-derived ISA subset. Not claimed as runtime-derived.
- R-006 (single-cfg overstated): Rephrased: xdb difftest module is single-gated; xcore has always-compiled CoreContext + few difftest-gated MMIO hooks.

[**Master Compliance**]

- M-001 (CoreContext arch-dependent): `CoreContext` defined per-ISA in `cpu/riscv/context.rs`, dispatched via `pub use RVCoreContext as CoreContext` like `RVCore as Core`.
- M-002 (reduce redundant DebugOps, CoreContext as primary): DebugOps trimmed to only what CoreContext can't provide (memory read, inst fetch, disasm). `read_register`, `dump_registers` removed. xdb uses CoreContext for register inspection (`info reg`), expression evaluation (register lookup), watchpoint checks, and difftest snapshots.

### Changes from Previous Round

[**Added**]
- `CsrValue::raw_value` field (unmasked)
- `CsrValue::masked()` method
- `difftest-spike` separate Cargo feature
- `cpu/riscv/context.rs` with `RVCoreContext`
- Spike pinned to v1.1.0

[**Changed**]
- CoreContext: from debug.rs generic -> arch-specific `RVCoreContext as CoreContext`
- CsrValue: single value -> raw_value + mask (masked() for compare)
- sync_state: writes raw values, not masked
- DebugOps: removed read_register, dump_registers
- ArchSnapshot::diff: validates CSR by addr, not positional index
- Spike: mandatory -> optional `difftest-spike` feature
- ISA string: "runtime-derived" -> "cfg-derived, documented as fixed subset"

[**Removed**]
- `DebugOps::read_register()` (replaced by CoreContext)
- `DebugOps::dump_registers()` (replaced by CoreContext)
- QEMU-side `DIFF_CSR_REF` duplicate table
- `ArchSnapshot` as separate type (use CoreContext directly)

[**Unresolved**]
- Spike upstream API stability (mitigated by version pin + separate feature)
- mtime wall-clock drift (mitigated by MTIP mask)

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | CsrValue stores raw; masked() for compare; sync uses raw |
| Review | R-002 | Accepted | `difftest-spike` separate feature |
| Review | R-003 | Accepted | Single CSR whitelist from xcore; compare by addr |
| Review | R-004 | Accepted | Pinned to riscv-isa-sim v1.1.0 |
| Review | R-005 | Accepted | ISA documented as cfg-derived |
| Review | R-006 | Accepted | Claims rephrased accurately |
| Master | M-001 | Applied | `RVCoreContext as CoreContext` in cpu/riscv/context.rs |
| Master | M-002 | Applied | DebugOps trimmed; CoreContext is primary data source |

---

## Spec

[**Goals**]

- G-1: Per-instruction state comparison (DUT vs REF), halt on first divergence.
- G-2: Two backends: QEMU (production, `difftest` feature) and Spike (experimental, `difftest-spike` feature).
- G-3: Compare PC + GPR[1..31] + privilege + whitelisted CSRs (masked).
- G-4: MMIO-skip: sync DUT->REF using raw (unmasked) CSR values.
- G-5: Feature-gated. xdb difftest module single-gated. xcore has always-compiled CoreContext + difftest-gated MMIO hook.
- G-6: Monitor commands: `dt attach qemu|spike`, `dt detach`, `dt status`.
- G-7: Interrupt-preserving: QEMU `sstep=0x1`.
- G-8: Backend-neutral xcore: exports `CoreContext` plain data. No backend protocol in xcore.
- G-9: Physical-memory correct: QEMU `PhyMemMode:1`.
- G-10: CoreContext is the primary data carrier for both debug and difftest.

- NG-1: Memory comparison deferred.
- NG-2: Full CSR dump deferred.
- NG-3: mtime virtual-clock sync deferred.

[**Architecture**]

```
xcore                              xdb
+---------------------------+      +----------------------------------+
|  RVCore                   |      |  xdb mainloop                   |
|  +--------+               |      |                                  |
|  |DebugOps|               |      |  CoreContext (plain data, Clone) |
|  |        |--context()----+----->|  +-pc, gpr[32], privilege       |
|  |        |               |      |  +-csrs: [CsrValue]             |
|  |        |--read_memory()|      |  |  .name, .addr, .mask,        |
|  |        |--fetch_inst() |      |  |  .raw_value                  |
|  |        |--disasm_raw() |      |  +-word_size, isa               |
|  +--------+               |      |  +----------+---------+         |
|                           |      |             |                    |
|  Bus                      |      |   +---------v---------+         |
|  +--------+               |      |   | DiffHarness       |         |
|  |mmio_   |--take_flag()--+----->|   | .check_step(ctx)  |         |
|  |accessed|               |      |   +---+----------+----+         |
|  +--------+               |      |       |          |              |
+---------------------------+      |  +----v---+ +----v----+         |
                                   |  |QEMU    | |Spike    |         |
                                   |  |GDB RSP | |FFI      |         |
                                   |  +--------+ +---------+         |
                                   +----------------------------------+
```

**DebugOps after M-002 trimming**:
- `context() -> CoreContext` — snapshot (replaces read_register, dump_registers)
- `read_memory(paddr, size) -> XResult<u64>` — interactive memory read
- `fetch_inst(paddr) -> XResult<u32>` — instruction fetch
- `disasm_raw(raw) -> String` — disassembly
- Breakpoint management (unchanged): add/remove/list/set_skip_bp

[**Invariants**]

- I-1: DUT executes first, then REF stepped.
- I-2: MMIO -> sync (raw values), skip compare.
- I-3: ebreak -> halt, sync (raw values), skip.
- I-4: QEMU `sstep=0x1`.
- I-5: QEMU `PhyMemMode:1`.
- I-6: `CoreContext` is plain data, `Clone`. No references.
- I-7: Compare uses `CsrValue::masked()`. Sync uses `CsrValue::raw_value`.
- I-8: Timer CSRs excluded. mip mask = `!0x82`.
- I-9: `dt attach` requires runtime binary path.
- I-10: Feature disabled -> zero difftest code.
- I-11: Unsupported sstep/PhyMemMode -> attach failure.
- I-12: Spike experimental, v1.1.0 pinned.
- I-13: CSR comparison validates by addr, not positional index.

[**Data Structure**]

```rust
// === xcore/src/cpu/riscv/context.rs (always compiled) ===

/// Named CSR snapshot with raw value and comparison mask.
#[derive(Clone, Copy)]
pub struct CsrValue {
    pub name: &'static str,
    pub addr: u16,
    pub mask: u64,
    pub raw_value: u64,  // unmasked, for sync
}

impl CsrValue {
    pub fn masked(&self) -> u64 { self.raw_value & self.mask }
}

/// Arch-specific core context — lightweight snapshot.
#[derive(Clone)]
pub struct RVCoreContext {
    pub pc: u64,
    pub gpr: [u64; 32],
    pub privilege: u64,
    pub csrs: Vec<CsrValue>,
    pub word_size: usize,
    pub isa: &'static str,
}

// === xcore/src/cpu/mod.rs ===
// pub use self::riscv::context::{RVCoreContext as CoreContext, CsrValue};

// === xcore/src/cpu/debug.rs ===

pub trait DebugOps: super::CoreOps {
    // Snapshot — replaces read_register / dump_registers
    fn context(&self) -> super::CoreContext;

    // Breakpoints (unchanged)
    fn add_breakpoint(&mut self, addr: usize) -> u32;
    fn remove_breakpoint(&mut self, id: u32) -> bool;
    fn list_breakpoints(&self) -> &[Breakpoint];
    fn set_skip_bp(&mut self);

    // Memory + disasm (only what CoreContext can't provide)
    fn read_memory(&self, paddr: usize, size: usize) -> XResult<u64>;
    fn fetch_inst(&self, paddr: usize) -> XResult<u32>;
    fn disasm_raw(&self, raw: u32) -> String;
}

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
```

[**API Surface**]

```rust
// -- xcore --
// CoreContext carries everything xdb needs for register inspection + difftest.
// DebugOps::context() is the single API for getting arch state.

impl DebugOps for RVCore {
    fn context(&self) -> CoreContext; // captures pc, gpr, priv, csrs, word_size, isa
    fn read_memory(&self, paddr: usize, size: usize) -> XResult<u64>;
    fn fetch_inst(&self, paddr: usize) -> XResult<u32>;
    fn disasm_raw(&self, raw: u32) -> String;
    // breakpoints unchanged
}

// CPU pass-through:
impl<Core: CoreOps + DebugOps> CPU<Core> {
    pub fn context(&self) -> CoreContext { self.core.context() }
}

// Re-export:
pub use cpu::{CoreContext, CsrValue};

// -- xdb difftest --
impl CoreContext {
    /// Compare two contexts. Returns first mismatch.
    pub fn diff(&self, other: &CoreContext, inst_count: u64) -> Option<DiffMismatch>;
}

impl DiffHarness {
    pub fn new(backend: Box<dyn DiffBackend>) -> Self;
    pub fn check_step(&mut self, dut_ctx: &CoreContext, mmio: bool, halted: bool)
        -> Result<Option<DiffMismatch>, String>;
    pub fn report_mismatch(m: &DiffMismatch);
}

// -- xdb debug commands (adapted to CoreContext) --
// info reg: ctx.gpr + ctx.csrs
// print expr: ctx.gpr[name] lookup + read_memory for *deref
// watchpoint check: same pattern
```

[**How xdb commands migrate to CoreContext (M-002)**]

| Command | Before (DebugOps) | After (CoreContext) |
|---------|-------------------|---------------------|
| `info reg` | `dump_registers()` | `ctx.gpr` + `ctx.csrs` iteration |
| `info reg <name>` | `read_register(name)` | lookup in `ctx.gpr`/`ctx.csrs` by name |
| `p <expr>` | `read_register` callback | `ctx` field lookup; `read_memory` for `*deref` |
| `w <expr>` check | `read_register` callback | same as `p` |
| `x/Ni` | `fetch_inst` + `disasm_raw` | stays on DebugOps (needs memory) |
| `x/Nx`, `x/Nb` | `read_memory` | stays on DebugOps |
| difftest snapshot | `read_register` x34 | `ctx` directly |

The expression evaluator callbacks change from:
```rust
// Before: closure over DebugOps trait object
|name| ops.read_register(name)
```
To:
```rust
// After: closure over CoreContext value
|name| ctx.register_by_name(name)
```

Where `CoreContext` provides a `register_by_name` helper:
```rust
impl RVCoreContext {
    pub fn register_by_name(&self, name: &str) -> Option<u64> {
        match name {
            "pc" => Some(self.pc),
            "privilege" => Some(self.privilege),
            _ => {
                // GPR lookup
                RVReg::from_name(name)
                    .map(|r| self.gpr[r as usize])
                    .or_else(|| {
                        // CSR lookup
                        self.csrs.iter()
                            .find(|c| c.name == name)
                            .map(|c| c.raw_value)
                    })
            }
        }
    }
}
```

[**CSR Whitelist (single source of truth)**]

Defined in `xcore/src/cpu/riscv/context.rs`, exported via CoreContext:

```rust
pub(crate) const DIFF_CSRS: &[(&str, u16, u64)] = &[
    ("mstatus",  0x300, u64::MAX),
    ("mtvec",    0x305, u64::MAX),
    ("mepc",     0x341, u64::MAX),
    ("mcause",   0x342, u64::MAX),
    ("mtval",    0x343, u64::MAX),
    ("medeleg",  0x302, u64::MAX),
    ("mideleg",  0x303, u64::MAX),
    ("mie",      0x304, u64::MAX),
    ("mip",      0x344, !0x82_u64),
    ("stvec",    0x105, u64::MAX),
    ("sepc",     0x141, u64::MAX),
    ("scause",   0x142, u64::MAX),
    ("stval",    0x143, u64::MAX),
    ("satp",     0x180, u64::MAX),
];
```

Backends read `ctx.csrs` and map `CsrValue::addr` to their own register numbers. No duplicate table.

[**Constraints**]

- C-1: QEMU in PATH for `difftest`. Spike source for `difftest-spike`.
- C-2: GDB port 1234.
- C-3: Bare-metal `-bios`.
- C-4: MMIO hook `cfg(feature = "difftest")`.
- C-5: QEMU `-M virt`, RAM at 0x80000000.
- C-6: `difftest` depends on `debug`. `difftest-spike` depends on `difftest`.
- C-7: `dt attach` requires runtime binary path.
- C-8: QEMU `sstep=0x1`, `PhyMemMode:1`. Unsupported = failure.
- C-9: ISA string cfg-derived: `"rv64imac"` or `"rv32imac"`. Fixed subset for this round.
- C-10: RAM-write sync limitation. Documented non-goal.
- C-11: Spike experimental, pinned riscv-isa-sim v1.1.0.

---

## Implement

### Execution Flow

[**Main Flow**]

1. `DIFFTEST=1 make run` -> `--features difftest`.
2. `load <file>` -> xdb stores runtime path.
3. `dt attach qemu`:
   a. Get `CoreContext` for isa/word_size.
   b. Spawn QEMU, connect GDB.
   c. `Qqemu.sstep=0x1` + `Qqemu.PhyMemMode:1`. Fail if unsupported.
   d. Run to reset vector. Create DiffHarness.
4. `s` or `c` -> per-step:
   a. `with_xcpu(|cpu| { cpu.step()?; Ok((cpu.context(), cpu.is_terminated())) })`.
   b. MMIO flag check (if difftest active).
   c. `harness.check_step(&dut_ctx, mmio, halted)`:
      - `backend.step()`.
      - mmio/halted -> `backend.sync_state(&dut_ctx)` (writes raw CSR values) -> None.
      - else -> `backend.read_context()`, `dut_ctx.diff(&ref_ctx)` (uses masked values).
   d. Mismatch -> report, halt.
5. `info reg` -> `with_xcpu(|cpu| cpu.context())` -> print from ctx fields.
6. `p <expr>` -> `with_xcpu(|cpu| cpu.context())` -> `ctx.register_by_name()` + `read_memory` for deref.

### Implementation Plan

[**Phase 1: CoreContext in xcore**]

New file: `xcore/src/cpu/riscv/context.rs` (~70 lines)

```rust
use crate::config::Word;
use super::csr::CsrAddr;

#[derive(Clone, Copy)]
pub struct CsrValue {
    pub name: &'static str,
    pub addr: u16,
    pub mask: u64,
    pub raw_value: u64,
}

impl CsrValue {
    pub fn masked(&self) -> u64 { self.raw_value & self.mask }
}

#[derive(Clone)]
pub struct RVCoreContext {
    pub pc: u64,
    pub gpr: [u64; 32],
    pub privilege: u64,
    pub csrs: Vec<CsrValue>,
    pub word_size: usize,
    pub isa: &'static str,
}

const DIFF_CSRS: &[(&str, u16, u64)] = &[ /* 14 entries */ ];

impl RVCoreContext {
    pub fn register_by_name(&self, name: &str) -> Option<u64> {
        match name {
            "pc" => Some(self.pc),
            "privilege" => Some(self.privilege),
            _ => crate::isa::RVReg::from_name(name)
                .map(|r| self.gpr[r as usize])
                .or_else(|| self.csrs.iter()
                    .find(|c| c.name == name)
                    .map(|c| c.raw_value))
        }
    }
}
```

In `xcore/src/cpu/riscv/mod.rs`:
```rust
pub mod context;
pub use context::RVCoreContext;
```

In `xcore/src/cpu/mod.rs`:
```rust
cfg_if::cfg_if! {
    if #[cfg(riscv)] {
        mod riscv;
        pub use self::riscv::*;
        pub use self::riscv::context::{RVCoreContext as CoreContext, CsrValue};
    }
}
```

In `xcore/src/lib.rs`:
```rust
pub use cpu::{CoreContext, CsrValue};
```

[**Phase 2: Trim DebugOps + impl context()**]

In `xcore/src/cpu/debug.rs`:
```rust
pub trait DebugOps: super::CoreOps {
    fn context(&self) -> super::CoreContext;
    fn add_breakpoint(&mut self, addr: usize) -> u32;
    fn remove_breakpoint(&mut self, id: u32) -> bool;
    fn list_breakpoints(&self) -> &[Breakpoint];
    fn set_skip_bp(&mut self);
    fn read_memory(&self, paddr: usize, size: usize) -> XResult<u64>;
    fn fetch_inst(&self, paddr: usize) -> XResult<u32>;
    fn disasm_raw(&self, raw: u32) -> String;
}
```

In `xcore/src/cpu/riscv/debug.rs`, replace `read_register`/`dump_registers` with `context()`:
```rust
fn context(&self) -> crate::cpu::CoreContext {
    use super::context::{RVCoreContext, CsrValue, DIFF_CSRS};
    let pc = self.pc.as_usize() as u64;
    let privilege = self.privilege as u64;
    let mut gpr = [0u64; 32];
    for i in 0..32 { gpr[i] = word_to_u64(self.gpr[i]); }
    let csrs = DIFF_CSRS.iter().map(|&(name, addr, mask)| {
        let raw = word_to_u64(self.csr.get(CsrAddr::try_from(addr).unwrap()));
        CsrValue { name, addr, mask, raw_value: raw }
    }).collect();
    RVCoreContext {
        pc, gpr, privilege, csrs,
        word_size: std::mem::size_of::<crate::config::Word>(),
        isa: if cfg!(isa64) { "rv64imac" } else { "rv32imac" },
    }
}
```

[**Phase 3: Adapt xdb to CoreContext**]

Update `cmd.rs`:

```rust
// info reg — use CoreContext instead of dump_registers
pub fn cmd_info(what: &str, name: Option<&str>) -> XResult {
    match what {
        "reg" | "r" => with_xcpu(|cpu| {
            let ctx = cpu.context();
            match name {
                Some(n) => match ctx.register_by_name(n) {
                    Some(val) => println!("{n} = {val:#x}"),
                    None => println!("Unknown register: {n}"),
                },
                None => {
                    println!("{:>10} = {:#018x}", "pc", ctx.pc);
                    for i in 0..32 {
                        let rname = xcore::isa::RVReg::from_u8(i as u8).unwrap().name();
                        print!("{rname:>10} = {:#018x}", ctx.gpr[i as usize]);
                        if ((i + 1) % 4) == 0 { println!(); } else { print!("  "); }
                    }
                    println!();
                }
            }
        }),
        _ => println!("Unknown info target: {what}. Try: reg"),
    }
    Ok(())
}

// print / watchpoint — context + read_memory for deref
pub fn cmd_print(expr_str: &str) -> XResult {
    with_xcpu(|cpu| {
        let ctx = cpu.context();
        let ops = cpu.debug_ops();
        match eval_expr(
            expr_str,
            |name| ctx.register_by_name(name),
            |addr, sz| ops.read_memory(addr, sz).ok(),
        ) {
            Ok(val) => println!("{val:#x} ({val})"),
            Err(e) => println!("Error: {e}"),
        }
        Ok(())
    })
}
```

Watchpoint check similarly uses `ctx.register_by_name` + `ops.read_memory`.

[**Phase 4: GDB Client — `xdb/src/difftest/gdb.rs` (~200 lines)**]

Same as previous rounds. GDB RSP over TCP.

[**Phase 5: QEMU Backend — `xdb/src/difftest/qemu.rs` (~140 lines)**]

```rust
fn csr_addr_to_qemu_regnum(addr: u16) -> usize { 4096 + addr as usize }
const QEMU_PRIV_REGNUM: usize = 4161;

fn qemu_bin_for_isa(isa: &str) -> &'static str {
    if isa.starts_with("rv64") { "qemu-system-riscv64" }
    else { "qemu-system-riscv32" }
}

impl QemuBackend {
    pub fn new(binary_path: &str, reset_vec: usize, init_ctx: &CoreContext)
        -> Result<Self, String> {
        // spawn, connect, sstep=0x1, PhyMemMode:1, run to reset_vec
        // Store init_ctx.csrs metadata for snapshot construction
    }
}

impl DiffBackend for QemuBackend {
    fn read_context(&mut self) -> Result<CoreContext, String> {
        let regs = self.gdb.read_regs()?;
        let mut gpr = [0u64; 32];
        gpr.copy_from_slice(&regs[..32]);
        // Read CSRs from ctx.csrs metadata (single source of truth)
        let csrs: Vec<_> = self.csr_meta.iter().map(|meta| {
            let raw = self.gdb.read_register(csr_addr_to_qemu_regnum(meta.addr))
                .unwrap_or(0);
            CsrValue { name: meta.name, addr: meta.addr, mask: meta.mask, raw_value: raw }
        }).collect();
        let priv_mode = self.gdb.read_register(QEMU_PRIV_REGNUM).unwrap_or(0);
        Ok(CoreContext { pc: regs[32], gpr, privilege: priv_mode, csrs,
            word_size: self.word_size, isa: self.isa })
    }

    fn sync_state(&mut self, ctx: &CoreContext) -> Result<(), String> {
        // Write GPR+PC
        let mut regs = ctx.gpr.to_vec();
        regs.push(ctx.pc);
        self.gdb.write_regs(&regs)?;
        // Write raw CSR values (not masked!)
        for csv in &ctx.csrs {
            self.gdb.write_register(csr_addr_to_qemu_regnum(csv.addr), csv.raw_value)?;
        }
        Ok(())
    }
}
```

[**Phase 6: Spike Backend — `xdb/src/difftest/spike.rs` (behind `difftest-spike`)**]

Same FFI design as round-03/04. Pinned to riscv-isa-sim v1.1.0.

`read_context()` reads raw CSR values via `spike_get_csr(addr)`.
`sync_state()` writes raw CSR values via `spike_set_csr(addr, raw_value)`.

[**Phase 7: DiffHarness — `xdb/src/difftest/mod.rs`**]

```rust
impl CoreContext {
    /// Compare for difftest. Uses masked CSR values.
    pub fn diff(&self, other: &CoreContext, inst_count: u64) -> Option<DiffMismatch> {
        if self.pc != other.pc {
            return Some(DiffMismatch { inst_count, reg_name: "pc",
                dut_val: self.pc, ref_val: other.pc });
        }
        for i in 1..32 {
            if self.gpr[i] != other.gpr[i] {
                return Some(DiffMismatch { inst_count,
                    reg_name: /* RVReg name lookup */,
                    dut_val: self.gpr[i], ref_val: other.gpr[i] });
            }
        }
        if self.privilege != other.privilege {
            return Some(DiffMismatch { inst_count, reg_name: "privilege",
                dut_val: self.privilege, ref_val: other.privilege });
        }
        // CSR compare by addr, not index
        for dut_csr in &self.csrs {
            if let Some(ref_csr) = other.csrs.iter().find(|c| c.addr == dut_csr.addr) {
                if dut_csr.masked() != ref_csr.masked() {
                    return Some(DiffMismatch { inst_count, reg_name: dut_csr.name,
                        dut_val: dut_csr.masked(), ref_val: ref_csr.masked() });
                }
            }
        }
        None
    }
}

impl DiffHarness {
    pub fn check_step(&mut self, dut_ctx: &CoreContext, mmio: bool, halted: bool)
        -> Result<Option<DiffMismatch>, String> {
        self.inst_count += 1;
        self.backend.step()?;
        if mmio || halted {
            self.backend.sync_state(dut_ctx)?; // writes raw values
            return Ok(None);
        }
        let ref_ctx = self.backend.read_context()?;
        Ok(dut_ctx.diff(&ref_ctx, self.inst_count)) // compares masked values
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
        // Build spike_wrapper.cc, link libriscv etc.
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
| `xcore/src/cpu/riscv/context.rs` | xcore | NEW | RVCoreContext, CsrValue, DIFF_CSRS, register_by_name |
| `xcore/src/cpu/debug.rs` | xcore | MOD | Trimmed DebugOps (+context, -read_register, -dump_registers) |
| `xcore/src/cpu/riscv/debug.rs` | xcore | MOD | +context() impl, -read_register, -dump_registers |
| `xcore/src/cpu/riscv/mod.rs` | xcore | MOD | +pub mod context |
| `xcore/src/cpu/mod.rs` | xcore | MOD | +CoreContext/CsrValue dispatch, +bus_take_mmio_flag(cfg) |
| `xcore/src/device/bus.rs` | xcore | MOD | +AtomicBool mmio_accessed (cfg difftest) |
| `xcore/src/lib.rs` | xcore | MOD | +re-export CoreContext, CsrValue |
| `xdb/src/difftest/mod.rs` | xdb | NEW | DiffBackend, DiffHarness, diff() on CoreContext |
| `xdb/src/difftest/gdb.rs` | xdb | NEW | GdbClient |
| `xdb/src/difftest/qemu.rs` | xdb | NEW | QemuBackend |
| `xdb/src/difftest/spike.rs` | xdb | NEW | SpikeBackend (cfg difftest-spike) |
| `xdb/build.rs` | xdb | NEW | Spike cc build (cfg difftest-spike) |
| `xdb/src/cmd.rs` | xdb | MOD | Adapted to CoreContext, dt commands |
| `xdb/src/cli.rs` | xdb | MOD | Dt subcommand |
| `xdb/src/main.rs` | xdb | MOD | Wiring, runtime path |

---

## Trade-offs

- T-1: **Raw vs masked in CsrValue** — Both stored. `masked()` for compare, `raw_value` for sync. Small overhead (one extra field per CSR). Correctness requires it.
- T-2: **CoreContext always compiled** — Vec allocation per call. Only on debug/interactive paths, not hot. Clean crate boundary worth it.
- T-3: **Removing read_register from DebugOps** — Breaking change for xdb. Worth it: CoreContext is more efficient (one snapshot vs N calls) and cleaner boundary.
- T-4: **CSR compare by addr** — O(n*m) lookup, but n=m=14. Negligible.

---

## Validation

[**Unit Tests**]

- V-UT-1: GDB packet checksum.
- V-UT-2: parse_gdb_regs RV64/RV32.
- V-UT-3: parse/encode hex round-trip.
- V-UT-4: CoreContext::diff identical -> None.
- V-UT-5: CoreContext::diff PC mismatch.
- V-UT-6: CoreContext::diff GPR mismatch.
- V-UT-7: CoreContext::diff privilege mismatch.
- V-UT-8: CoreContext::diff CSR masked mismatch.
- V-UT-9: CoreContext::diff masked mip bits ignored.
- V-UT-10: CsrValue::masked() correct.
- V-UT-11: CsrValue::raw_value preserved in sync (not masked).
- V-UT-12: CoreContext::register_by_name resolves pc/gpr/csr.
- V-UT-13: context() captures correct RVCore state.

[**Integration Tests**]

- V-IT-1: Difftest (QEMU) on cpu-tests-rs — zero divergence.
- V-IT-2: Difftest (QEMU) on am-tests — MMIO skip.
- V-IT-3: Difftest (Spike) on cpu-tests-rs (if difftest-spike enabled).
- V-IT-4: Intentional divergence caught.
- V-IT-5: `dt attach`/`detach`/`status` lifecycle.
- V-IT-6: `dt attach` without load -> error.
- V-IT-7: `info reg` uses CoreContext correctly.
- V-IT-8: `p $a0 + 1` uses CoreContext for register, read_memory for deref.

[**Failure / Robustness**]

- V-F-1: QEMU not in PATH.
- V-F-2: sstep unsupported.
- V-F-3: PhyMemMode unsupported.
- V-F-4: Spike init fails (only with difftest-spike).

[**Edge Cases**]

- V-E-1: First instruction divergence.
- V-E-2: MMIO then non-MMIO — sync raw CSRs, resume masked compare.
- V-E-3: Compressed instruction.
- V-E-4: Trap (ecall) — CSRs match.
- V-E-5: mret/sret privilege.
- V-E-6: ebreak halt sync preserves raw mip bits.
- V-E-7: Timer interrupt with sstep=0x1.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (per-inst) | V-IT-1, V-IT-3, V-IT-4, V-UT-4..9 |
| G-2 (two backends) | V-IT-1+3, V-F-4 |
| G-3 (PC+GPR+priv+CSR) | V-UT-5..9, V-E-4, V-E-5 |
| G-4 (MMIO sync raw) | V-IT-2, V-E-2, V-UT-11 |
| G-5 (feature-gated) | Build without: zero code |
| G-6 (monitor cmds) | V-IT-5, V-IT-6 |
| G-7 (interrupt) | V-E-7 |
| G-8 (backend-neutral) | No QEMU in xcore |
| G-9 (PhyMemMode) | V-F-3 |
| G-10 (CoreContext primary) | V-IT-7, V-IT-8, V-UT-12, V-UT-13 |
| C-8 (sstep) | V-F-2 |
| C-11 (Spike exp.) | V-IT-3, V-F-4 |
