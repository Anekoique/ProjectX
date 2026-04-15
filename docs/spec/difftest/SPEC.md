# `difftest` SPEC

> Source: base spec from [`/docs/archived/feat/difftest/06_PLAN.md`](/docs/archived/feat/difftest/06_PLAN.md),
> with subsequent delta amendments in rounds up to [`/docs/archived/feat/difftest/07_PLAN.md`](/docs/archived/feat/difftest/07_PLAN.md).
> Iteration history and trade-off analysis live under `docs/archived/feat/difftest/`.

---


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
