# `difftest` PLAN `04`

> Status: Revised
> Feature: `difftest`
> Iteration: `04`
> Owner: Executor
> Depends on:
> - Previous Plan: `03_PLAN.md`
> - Review: `03_REVIEW.md`
> - Master Directive: `03_MASTER.md`

---

## Summary

Final design. xcore exports a lightweight `CoreContext` struct — a plain-data snapshot of architectural state (PC, GPR, privilege, named CSRs, ISA metadata). xdb receives `CoreContext` and dispatches it to difftest/debug without reaching back into xcore internals. The `difftest` module in xdb is enabled by a single `cfg(feature = "difftest")` on the module declaration. QEMU backend enables `PhyMemMode` for correct physical-memory access. Spike is scoped as experimental with a pinned version. Runtime binary path replaces all `option_env!` usage.

## Log

[**Feature Introduce**]

- `CoreContext` struct in xcore: lightweight, `Clone` snapshot containing PC, GPR[32], privilege, CSR name-value pairs, word_size, ISA string. Constructed by `DebugOps::context()`. Passed out to xdb as plain data — no trait objects, no callbacks into xcore.
- QEMU `PhyMemMode`: send `Qqemu.PhyMemMode:1` after connect for correct physical-memory access post-satp.
- Spike scoped as experimental, pinned to a known-good commit.
- Single `cfg(feature = "difftest")` on `mod difftest` in xdb. No scattered cfg attributes.

[**Review Adjustments**]

- R-001 (QEMU PhyMemMode): Added `Qqemu.PhyMemMode:1` to attach sequence. Failure = attach failure.
- R-002 (Spike build not buildable): Fixed Cargo.toml — single `[features]` table, `cc` as normal build-dep, build.rs uses `std::env::var("CARGO_FEATURE_DIFFTEST")`.
- R-003 (Spike non-public API): Spike scoped as experimental, pinned to commit hash. Not equivalent to QEMU path. Documented as "may break with upstream Spike updates".
- R-004 (X_FILE compile-time): Removed all `option_env!("X_FILE")` from difftest paths. `loaded_binary_path` seeded from `std::env::var("X_FILE")` (runtime) or set by `load` command.
- R-005 (Spike ISA hard-coded): ISA string derived from xcore's `CoreContext::isa` field.
- R-006 (RAM-write sync): Elevated to constraint C-10 as permanent documented limitation.

[**Master Compliance**]

- M-001 (single cfg): One `#[cfg(feature = "difftest")]` on `mod difftest` in xdb. One on the `CoreContext` CSR whitelist section in xcore. No scattered cfg blocks.
- M-002 (CoreContext): New `CoreContext` struct passed from xcore to xdb. xdb dispatches it to debug/difftest. xcore exposes only `DebugOps::context() -> CoreContext`. No trait objects crossing crate boundary for difftest. Clean separation.

### Changes from Previous Round

[**Added**]
- `CoreContext` struct (plain data, Clone, no trait objects)
- `DebugOps::context() -> CoreContext`
- QEMU `PhyMemMode:1` in attach sequence
- Spike pinned commit + experimental scope
- ISA string in CoreContext for Spike configuration
- Constraint C-10 (RAM-write sync limitation)

[**Changed**]
- DifftestOps trait removed — replaced by CoreContext plain data
- Scattered `cfg(feature = "difftest")` -> single module-level cfg
- `option_env!("X_FILE")` -> `std::env::var("X_FILE")` runtime
- Spike: first-class -> experimental with compatibility warning
- Cargo.toml: fixed build-dep and features

[**Removed**]
- `DifftestOps` trait (replaced by CoreContext data)
- `difftest_ops()` / `difftest_snapshot()` CPU methods
- All `option_env!("X_FILE")` in difftest paths
- Scattered cfg blocks

[**Unresolved**]
- Spike upstream API stability (mitigated by pinned commit)
- mtime wall-clock drift (mitigated by MTIP mask)

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | `Qqemu.PhyMemMode:1` in attach, failure = abort |
| Review | R-002 | Accepted | Fixed Cargo.toml, build.rs uses CARGO_FEATURE_DIFFTEST |
| Review | R-003 | Accepted | Spike = experimental, pinned commit |
| Review | R-004 | Accepted | `std::env::var("X_FILE")` runtime, no option_env! |
| Review | R-005 | Accepted | ISA string from CoreContext |
| Review | R-006 | Accepted | Elevated to constraint C-10 |
| Master | M-001 | Applied | Single cfg on mod difftest |
| Master | M-002 | Applied | CoreContext struct, see architecture below |

---

## Spec

[**Goals**]

- G-1: Per-instruction state comparison (DUT vs REF), halt on first divergence.
- G-2: Two backends: QEMU (GDB RSP, production) and Spike (FFI, experimental).
- G-3: Compare PC + GPR[1..31] + privilege + whitelisted CSRs (masked).
- G-4: MMIO-skip: sync DUT->REF on MMIO-touching instructions.
- G-5: Feature-gated. Single `cfg(feature = "difftest")` per crate. Zero cost disabled.
- G-6: Monitor commands: `dt attach qemu|spike`, `dt detach`, `dt status`.
- G-7: Interrupt-preserving: QEMU `sstep=0x1`.
- G-8: Backend-neutral xcore: exports `CoreContext` plain data only.
- G-9: Physical-memory correct: QEMU `PhyMemMode:1`.

- NG-1: Memory comparison deferred.
- NG-2: Full CSR dump deferred.
- NG-3: mtime virtual-clock sync deferred.

[**Architecture**]

```
xcore                              xdb
+---------------------------+      +----------------------------------+
|                           |      |                                  |
|  RVCore                   |      |  xdb mainloop                   |
|  +-----------+            |      |  +------------------+           |
|  | DebugOps  |--context()-+----->|  | CoreContext       |           |
|  +-----------+            |      |  | (plain data,     |           |
|  | gpr[32]   |            |      |  |  Clone, no refs) |           |
|  | pc, priv  |            |      |  +--------+---------+           |
|  | csr[]     |            |      |           |                     |
|  +-----------+            |      |     +-----v------+              |
|                           |      |     | ArchSnapshot|              |
|  Bus                      |      |     | ::from()   |              |
|  +--------+               |      |     +-----+------+              |
|  |mmio_   |--take_flag()--+----->|           |                     |
|  |accessed|               |      |     +-----v------+              |
|  +--------+               |      |     | DiffHarness|              |
|                           |      |     | check_step |              |
+---------------------------+      |     +-----+------+              |
                                   |           |                     |
                                   |     +-----v-----------+        |
                                   |     | DiffBackend      |        |
                                   |     | +------+ +-----+ |        |
                                   |     | |QEMU  | |Spike| |        |
                                   |     | |GDB   | |FFI  | |        |
                                   |     | +------+ +-----+ |        |
                                   |     +------------------+        |
                                   +----------------------------------+
```

**Key insight (M-002)**: xcore builds a `CoreContext` (lightweight value type) and hands it off. xdb never calls back into xcore for register reads during difftest. The context carries everything needed: PC, GPR, privilege, CSR snapshot, word size, ISA string. This decouples xdb from xcore's internal types completely.

[**Invariants**]

- I-1: DUT executes first, then REF stepped. Compare after both.
- I-2: MMIO -> sync, skip compare.
- I-3: ebreak -> halt, sync, skip.
- I-4: QEMU `sstep=0x1`. Interrupt divergence = real bug.
- I-5: QEMU `PhyMemMode:1`. Memory access is physical.
- I-6: `CoreContext` is a plain-data value. No references, no trait objects. `Clone`.
- I-7: Compare: PC, GPR[1..31], privilege, whitelisted CSRs (masked).
- I-8: Timer CSRs excluded. mip mask = `!0x82`.
- I-9: `dt attach` requires runtime-loaded binary path.
- I-10: Feature disabled -> zero difftest code.
- I-11: Unsupported sstep or PhyMemMode -> attach failure.
- I-12: Spike is experimental. Pinned to known-good commit.

[**Data Structure**]

```rust
// === xcore/src/cpu/debug.rs ===

/// Lightweight architectural context snapshot.
/// Plain data, Clone, no references — safe to pass across crate boundary.
#[derive(Clone)]
pub struct CoreContext {
    pub pc: u64,
    pub gpr: [u64; 32],
    pub privilege: u64,
    pub csrs: Vec<CsrValue>,
    pub word_size: usize,  // 4 or 8
    pub isa: String,       // "rv64imac" or "rv32imac"
}

/// Named CSR with its value and comparison mask.
#[derive(Clone, Copy)]
pub struct CsrValue {
    pub name: &'static str,
    pub addr: u16,
    pub mask: u64,
    pub value: u64,  // already masked
}

/// Existing DebugOps trait — add context() method.
pub trait DebugOps: super::CoreOps {
    // ... existing methods ...
    fn context(&self) -> CoreContext;
}

// === xdb/src/difftest/mod.rs (single cfg on mod) ===

pub struct ArchSnapshot {
    pub pc: u64,
    pub gpr: [u64; 32],
    pub privilege: u64,
    pub csrs: Vec<CsrValue>,
}

pub struct DiffMismatch {
    pub inst_count: u64,
    pub reg_name: &'static str,
    pub dut_val: u64,
    pub ref_val: u64,
}

pub trait DiffBackend {
    fn step(&mut self) -> Result<(), String>;
    fn read_snapshot(&mut self) -> Result<ArchSnapshot, String>;
    fn sync_state(&mut self, snapshot: &ArchSnapshot) -> Result<(), String>;
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
// -- xcore (always compiled, no cfg) --
impl DebugOps for RVCore {
    fn context(&self) -> CoreContext;  // NEW
    // ... existing methods unchanged ...
}

// xcore public re-export:
pub use cpu::debug::{CoreContext, CsrValue};

// CPU pass-through:
impl<Core: CoreOps + DebugOps> CPU<Core> {
    pub fn context(&self) -> CoreContext { self.core.context() }
}

// -- xdb (behind single cfg(feature = "difftest") on mod) --
impl ArchSnapshot {
    pub fn from_context(ctx: &CoreContext) -> Self;
    pub fn from_ref(pc: u64, gpr: [u64; 32], priv: u64, csrs: Vec<CsrValue>) -> Self;
    pub fn diff(&self, other: &Self, count: u64) -> Option<DiffMismatch>;
}

impl DiffHarness {
    pub fn new(backend: Box<dyn DiffBackend>) -> Self;
    pub fn check_step(&mut self, ctx: &CoreContext, mmio: bool, halted: bool)
        -> Result<Option<DiffMismatch>, String>;
}

// QemuBackend::new(path, reset_vec, csr_whitelist, word_size)
// SpikeBackend::new(path, reset_vec, csr_whitelist, word_size, isa)
```

[**CSR Whitelist**]

Defined in `xcore/src/cpu/riscv/debug.rs` as a const array, always compiled (part of `context()` output):

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

[**Constraints**]

- C-1: QEMU in PATH. Spike source tree for Spike backend.
- C-2: GDB port 1234.
- C-3: Bare-metal `-bios` mode.
- C-4: MMIO hook behind `cfg(feature = "difftest")`.
- C-5: QEMU `-M virt`, RAM at 0x80000000.
- C-6: `difftest` depends on `debug` feature.
- C-7: `dt attach` requires runtime binary path.
- C-8: QEMU `sstep=0x1` required. Unsupported = failure.
- C-9: QEMU `PhyMemMode:1` required. Unsupported = failure.
- C-10: RAM-write sync limitation: assumes single instruction doesn't combine RAM write + MMIO write. Documented as non-goal for current scope; future work if multi-write instructions are added.
- C-11: Spike experimental. Pinned to commit `<hash>`. May break with upstream updates.

---

## Implement

### Execution Flow

[**Main Flow**]

1. `DIFFTEST=1 make run` -> `--features difftest`.
2. `load <file>` -> xdb stores `loaded_binary_path = Some(path)`.
3. `dt attach qemu`:
   a. Check `loaded_binary_path`.
   b. `with_xcpu(|cpu| cpu.context())` -> get `CoreContext` with word_size and isa.
   c. Resolve QEMU binary from `CoreContext::isa` ("rv64" -> `qemu-system-riscv64`).
   d. Spawn QEMU, connect GDB.
   e. Send `Qqemu.sstep=0x1`. Verify. Fail if unsupported.
   f. Send `Qqemu.PhyMemMode:1`. Verify. Fail if unsupported.
   g. Run to reset vector. Create DiffHarness.
4. `dt attach spike`:
   a. Check `loaded_binary_path`.
   b. Get `CoreContext` for word_size and isa.
   c. Init Spike FFI with isa string, memory layout.
   d. Copy binary to Spike memory.
   e. Create DiffHarness.
5. `s` or `c` -> per-step:
   a. `with_xcpu(|cpu| { cpu.step()?; Ok((cpu.context(), cpu.is_terminated())) })`.
   b. `with_xcpu(|cpu| cpu.bus_take_mmio_flag())` (only when difftest active).
   c. `harness.check_step(&ctx, mmio, halted)`:
      - `ArchSnapshot::from_context(&ctx)` -> DUT snapshot.
      - `backend.step()`.
      - If mmio or halted: `backend.sync_state(&dut_snap)` -> None.
      - Else: `backend.read_snapshot()`, `diff()`.
   d. Mismatch -> report, halt.

[**Failure Flow**]

1. QEMU not in PATH -> error.
2. `sstep=0x1` unsupported -> "Attach failed: QEMU sstep not supported".
3. `PhyMemMode:1` unsupported -> "Attach failed: QEMU PhyMemMode not supported".
4. Spike init fails -> error.
5. No binary loaded -> "Load a binary first".

### Implementation Plan

[**Phase 1: CoreContext in xcore (~40 lines)**]

In `xcore/src/cpu/debug.rs`, add (always compiled, no cfg):

```rust
#[derive(Clone)]
pub struct CsrValue {
    pub name: &'static str,
    pub addr: u16,
    pub mask: u64,
    pub value: u64,
}

#[derive(Clone)]
pub struct CoreContext {
    pub pc: u64,
    pub gpr: [u64; 32],
    pub privilege: u64,
    pub csrs: Vec<CsrValue>,
    pub word_size: usize,
    pub isa: String,
}

pub trait DebugOps: super::CoreOps {
    // ... existing methods ...
    fn context(&self) -> CoreContext;
}
```

In `xcore/src/cpu/riscv/debug.rs`, add to DebugOps impl:

```rust
const DIFF_CSRS: &[(&str, u16, u64)] = &[
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

fn context(&self) -> CoreContext {
    let pc = self.pc.as_usize() as u64;
    let privilege = self.privilege as u64;
    let mut gpr = [0u64; 32];
    for i in 0..32 {
        gpr[i] = word_to_u64(self.gpr[i]);
    }
    let csrs = DIFF_CSRS.iter().map(|&(name, addr, mask)| {
        let raw = word_to_u64(self.csr.get(CsrAddr::try_from(addr).unwrap()));
        CsrValue { name, addr, mask, value: raw & mask }
    }).collect();
    let isa = if cfg!(isa64) { "rv64imac" } else { "rv32imac" }.to_string();
    let word_size = std::mem::size_of::<crate::config::Word>();
    CoreContext { pc, gpr, privilege, csrs, word_size, isa }
}
```

CPU pass-through in `cpu/mod.rs`:

```rust
impl<Core: CoreOps + debug::DebugOps> CPU<Core> {
    pub fn context(&self) -> debug::CoreContext { self.core.context() }
}
```

Re-export in `lib.rs`:

```rust
pub use cpu::debug::{CoreContext, CsrValue};
```

MMIO hook in `bus.rs` (cfg-gated, single block):

```rust
#[cfg(feature = "difftest")]
mmio_accessed: std::sync::atomic::AtomicBool,

// In MMIO read/write dispatch:
#[cfg(feature = "difftest")]
self.mmio_accessed.store(true, std::sync::atomic::Ordering::Relaxed);

#[cfg(feature = "difftest")]
pub fn take_mmio_flag(&self) -> bool {
    self.mmio_accessed.swap(false, std::sync::atomic::Ordering::Relaxed)
}
```

CPU pass-through for mmio flag:

```rust
#[cfg(feature = "difftest")]
pub fn bus_take_mmio_flag(&self) -> bool {
    self.bus.lock().unwrap().take_mmio_flag()
}
```

Cargo features:

```toml
# xcore/Cargo.toml
[features]
debug = []
difftest = ["debug"]
```

[**Phase 2: GDB Client -- `xdb/src/difftest/gdb.rs` (~200 lines)**]

GDB RSP over TCP. Same design as round-03.

Key methods: connect, send_packet, recv_packet, send_recv, step, cont, read_regs, write_regs, read_register, write_register, write_mem, set_breakpoint, remove_breakpoint, send_raw.

Helpers: parse_gdb_regs, parse_hex_le, encode_le_hex, encode_regs_hex.

[**Phase 3: QEMU Backend -- `xdb/src/difftest/qemu.rs` (~140 lines)**]

```rust
/// Map CSR address to QEMU GDB register number.
/// QEMU riscv: regnum = 4096 + csr_addr. Lives in xdb, not xcore.
fn csr_to_qemu_regnum(addr: u16) -> usize { 4096 + addr as usize }
const QEMU_PRIV_REGNUM: usize = 4161;

fn qemu_bin_for_isa(isa: &str) -> &'static str {
    if isa.starts_with("rv64") { "qemu-system-riscv64" }
    else { "qemu-system-riscv32" }
}

pub struct QemuBackend {
    proc: Child,
    gdb: GdbClient,
    csrs: &'static [(&'static str, u16, u64)], // CSR whitelist reference
}

impl QemuBackend {
    pub fn new(binary_path: &str, reset_vec: usize,
               ctx: &CoreContext) -> Result<Self, String> {
        let qemu_bin = qemu_bin_for_isa(&ctx.isa);
        // 1. Verify QEMU
        let out = std::process::Command::new("which").arg(qemu_bin)
            .output().map_err(|e| format!("{e}"))?;
        if !out.status.success() {
            return Err(format!("{qemu_bin} not found in PATH"));
        }
        // 2. Spawn
        let proc = std::process::Command::new(qemu_bin)
            .args(["-M", "virt", "-m", "256M", "-nographic",
                   "-s", "-S", "-bios", binary_path])
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .spawn().map_err(|e| format!("spawn: {e}"))?;
        std::thread::sleep(Duration::from_millis(300));
        // 3. Connect GDB
        let mut gdb = GdbClient::connect("127.0.0.1:1234")?;
        // 4. sstep=0x1 (interrupts enabled)
        let resp = gdb.send_raw("Qqemu.sstep=0x1")?;
        if resp.is_empty() || resp.starts_with(b"E") {
            let _ = proc.kill();
            return Err("QEMU sstep not supported. Version too old.".into());
        }
        // 5. PhyMemMode:1 (physical memory access)
        let resp = gdb.send_raw("Qqemu.PhyMemMode:1")?;
        if resp.is_empty() || resp.starts_with(b"E") {
            let _ = proc.kill();
            return Err("QEMU PhyMemMode not supported. Version too old.".into());
        }
        // 6. Run to reset vector
        gdb.set_breakpoint(reset_vec)?;
        gdb.cont()?;
        gdb.remove_breakpoint(reset_vec)?;
        info!("Difftest: QEMU attached (pid {})", proc.id());
        // Build CSR reference from context
        Ok(Self { proc, gdb, csrs: DIFF_CSR_REF })
    }
}

/// Static CSR reference matching xcore's whitelist (addr + mask).
/// Kept in sync manually — both use the same CSR addr constants.
const DIFF_CSR_REF: &[(&str, u16, u64)] = &[
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

impl DiffBackend for QemuBackend {
    fn step(&mut self) -> Result<(), String> { self.gdb.step() }

    fn read_snapshot(&mut self) -> Result<ArchSnapshot, String> {
        let regs = self.gdb.read_regs()?;
        let mut gpr = [0u64; 32];
        gpr.copy_from_slice(&regs[..32]);
        let csrs: Vec<_> = self.csrs.iter().map(|&(name, addr, mask)| {
            let val = self.gdb.read_register(csr_to_qemu_regnum(addr)).unwrap_or(0) & mask;
            CsrValue { name, addr, mask, value: val }
        }).collect();
        let priv_mode = self.gdb.read_register(QEMU_PRIV_REGNUM).unwrap_or(0);
        Ok(ArchSnapshot::from_ref(regs[32], gpr, priv_mode, csrs))
    }

    fn sync_state(&mut self, snap: &ArchSnapshot) -> Result<(), String> {
        let mut regs = snap.gpr.to_vec();
        regs.push(snap.pc);
        self.gdb.write_regs(&regs)?;
        for csv in &snap.csrs {
            self.gdb.write_register(csr_to_qemu_regnum(csv.addr), csv.value)?;
        }
        Ok(())
    }

    fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String> {
        self.gdb.write_mem(addr, data)
    }

    fn name(&self) -> &str { "qemu" }
}

impl Drop for QemuBackend { ... } // kill + wait
```

[**Phase 4: Spike Backend -- `xdb/src/difftest/spike.rs` (~150 lines)**]

Experimental. Pinned to Spike commit `<hash>`. C wrapper via cc crate.

FFI header (`xdb/src/difftest/spike_ffi.h`):
```c
typedef struct spike_ctx spike_ctx_t;
typedef struct { uintptr_t base; size_t size; } spike_mem_t;
spike_ctx_t* spike_init(const spike_mem_t* mem, size_t n,
                        uint32_t pc, uint32_t xlen, const char* isa);
void     spike_fini(spike_ctx_t* ctx);
int      spike_step(spike_ctx_t* ctx);
void     spike_get_pc(spike_ctx_t* ctx, uint64_t* out);
void     spike_get_gpr(spike_ctx_t* ctx, uint64_t out[32]);
uint64_t spike_get_csr(spike_ctx_t* ctx, uint16_t addr);
uint64_t spike_get_priv(spike_ctx_t* ctx);
void     spike_set_pc(spike_ctx_t* ctx, uint64_t pc);
void     spike_set_gpr(spike_ctx_t* ctx, const uint64_t gpr[32]);
void     spike_set_csr(spike_ctx_t* ctx, uint16_t addr, uint64_t val);
void     spike_copy_mem(spike_ctx_t* ctx, uintptr_t addr,
                        const void* data, size_t len);
```

Rust FFI + SpikeBackend:
```rust
mod ffi { extern "C" { /* matching spike_ffi.h */ } }

pub struct SpikeBackend {
    ctx: *mut ffi::SpikeCtx,
    csrs: Vec<(&'static str, u16, u64)>,
}

impl SpikeBackend {
    pub fn new(binary_path: &str, reset_vec: usize,
               ctx: &CoreContext) -> Result<Self, String> {
        let region = ffi::SpikeMemRegion { base: 0x8000_0000, size: 256 << 20 };
        let xlen = (ctx.word_size * 8) as u32;
        let isa_c = std::ffi::CString::new(ctx.isa.as_str())
            .map_err(|e| format!("{e}"))?;
        let spike = unsafe {
            ffi::spike_init(&region, 1, reset_vec as u32, xlen, isa_c.as_ptr())
        };
        if spike.is_null() { return Err("Spike init failed".into()); }
        let bytes = std::fs::read(binary_path).map_err(|e| format!("{e}"))?;
        unsafe { ffi::spike_copy_mem(spike, reset_vec, bytes.as_ptr(), bytes.len()) };
        let csrs = ctx.csrs.iter().map(|c| (c.name, c.addr, c.mask)).collect();
        Ok(Self { ctx: spike, csrs })
    }
}

impl DiffBackend for SpikeBackend {
    fn step(&mut self) -> Result<(), String> { ... }
    fn read_snapshot(&mut self) -> Result<ArchSnapshot, String> { ... }
    fn sync_state(&mut self, snap: &ArchSnapshot) -> Result<(), String> { ... }
    fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String> { ... }
    fn name(&self) -> &str { "spike" }
}
impl Drop for SpikeBackend { ... }
```

Build system (`xdb/build.rs`):
```rust
fn main() {
    if std::env::var("CARGO_FEATURE_DIFFTEST").is_ok() {
        let spike_dir = std::env::var("SPIKE_DIR")
            .unwrap_or_else(|_| "/opt/spike".to_string());
        cc::Build::new()
            .cpp(true).std("c++17")
            .file("src/difftest/spike_wrapper.cc")
            .include(&format!("{spike_dir}/include"))
            .compile("spike_wrapper");
        println!("cargo:rustc-link-search=native={spike_dir}/lib");
        for lib in ["riscv", "softfloat", "fdt", "fesvr", "disasm"] {
            println!("cargo:rustc-link-lib=static={lib}");
        }
        println!("cargo:rustc-link-lib=dylib=stdc++");
    }
}
```

Cargo:
```toml
# xdb/Cargo.toml
[features]
debug = ["xcore/debug"]
difftest = ["xcore/difftest"]

[build-dependencies]
cc = "1"
```

[**Phase 5: DiffHarness + ArchSnapshot -- `xdb/src/difftest/mod.rs`**]

```rust
#[cfg(feature = "difftest")]
pub mod difftest;

// In xdb/src/difftest/mod.rs:
mod gdb;
mod qemu;
mod spike;

use xcore::{CoreContext, CsrValue};

pub struct ArchSnapshot {
    pub pc: u64,
    pub gpr: [u64; 32],
    pub privilege: u64,
    pub csrs: Vec<CsrValue>,
}

pub struct DiffMismatch {
    pub inst_count: u64,
    pub reg_name: &'static str,
    pub dut_val: u64,
    pub ref_val: u64,
}

pub trait DiffBackend { ... }

impl ArchSnapshot {
    pub fn from_context(ctx: &CoreContext) -> Self {
        Self { pc: ctx.pc, gpr: ctx.gpr, privilege: ctx.privilege,
               csrs: ctx.csrs.clone() }
    }

    pub fn from_ref(pc: u64, gpr: [u64; 32], priv_mode: u64,
                    csrs: Vec<CsrValue>) -> Self {
        Self { pc, gpr, privilege: priv_mode, csrs }
    }

    pub fn diff(&self, other: &Self, inst_count: u64) -> Option<DiffMismatch> {
        if self.pc != other.pc {
            return Some(DiffMismatch { inst_count, reg_name: "pc",
                dut_val: self.pc, ref_val: other.pc });
        }
        for i in 1..32 {
            if self.gpr[i] != other.gpr[i] {
                return Some(DiffMismatch { inst_count,
                    reg_name: xcore::isa::RVReg::from_u8(i as u8).unwrap().name(),
                    dut_val: self.gpr[i], ref_val: other.gpr[i] });
            }
        }
        if self.privilege != other.privilege {
            return Some(DiffMismatch { inst_count, reg_name: "privilege",
                dut_val: self.privilege, ref_val: other.privilege });
        }
        for (i, csv) in self.csrs.iter().enumerate() {
            if csv.value != other.csrs[i].value {
                return Some(DiffMismatch { inst_count, reg_name: csv.name,
                    dut_val: csv.value, ref_val: other.csrs[i].value });
            }
        }
        None
    }
}

pub struct DiffHarness {
    backend: Box<dyn DiffBackend>,
    inst_count: u64,
}

impl DiffHarness {
    pub fn new(backend: Box<dyn DiffBackend>) -> Self {
        Self { backend, inst_count: 0 }
    }

    pub fn check_step(&mut self, ctx: &CoreContext, mmio: bool, halted: bool)
        -> Result<Option<DiffMismatch>, String> {
        self.inst_count += 1;
        let dut = ArchSnapshot::from_context(ctx);
        self.backend.step()?;
        if mmio || halted {
            self.backend.sync_state(&dut)?;
            return Ok(None);
        }
        let ref_snap = self.backend.read_snapshot()?;
        Ok(dut.diff(&ref_snap, self.inst_count))
    }

    pub fn report_mismatch(m: &DiffMismatch) {
        eprintln!("DIFFTEST MISMATCH at instruction {}:\n  \
                   register: {}\n  DUT: {:#018x}\n  REF: {:#018x}",
                  m.inst_count, m.reg_name, m.dut_val, m.ref_val);
    }

    pub fn inst_count(&self) -> u64 { self.inst_count }
    pub fn backend_name(&self) -> &str { self.backend.name() }
}
```

[**Phase 6: xdb CLI + Runtime State**]

In `main.rs`:
```rust
let mut loaded_binary_path: Option<String> = std::env::var("X_FILE").ok()
    .filter(|s| !s.is_empty());
#[cfg(feature = "difftest")]
let mut diff_harness: Option<difftest::DiffHarness> = None;
```

On `load <file>`: `loaded_binary_path = Some(file.clone())`.

Step/continue hook (cfg-gated call site):
```rust
// In cmd_step, after cpu.step():
#[cfg(feature = "difftest")]
if let Some(ref mut h) = diff {
    let ctx = xcore::with_xcpu(|cpu| cpu.context());
    let mmio = xcore::with_xcpu(|cpu| cpu.bus_take_mmio_flag());
    match h.check_step(&ctx, mmio, done) {
        Ok(Some(m)) => { DiffHarness::report_mismatch(&m); return Ok(()); }
        Ok(None) => {}
        Err(e) => { println!("Difftest error: {e}"); *diff = None; return Ok(()); }
    }
}
```

### File Summary

| File | Crate | New/Mod | cfg | Description |
|------|-------|---------|-----|-------------|
| `xcore/src/cpu/debug.rs` | xcore | MOD | none | +CoreContext, CsrValue, DebugOps::context() |
| `xcore/src/cpu/riscv/debug.rs` | xcore | MOD | none | +context() impl, DIFF_CSRS whitelist |
| `xcore/src/cpu/mod.rs` | xcore | MOD | `difftest` | +bus_take_mmio_flag(); context() pass-through (no cfg) |
| `xcore/src/device/bus.rs` | xcore | MOD | `difftest` | +AtomicBool mmio_accessed |
| `xcore/src/lib.rs` | xcore | MOD | none | +re-export CoreContext, CsrValue |
| `xdb/src/difftest/mod.rs` | xdb | NEW | `difftest` | DiffBackend, DiffHarness, ArchSnapshot |
| `xdb/src/difftest/gdb.rs` | xdb | NEW | `difftest` | GdbClient |
| `xdb/src/difftest/qemu.rs` | xdb | NEW | `difftest` | QemuBackend (sstep+PhyMemMode) |
| `xdb/src/difftest/spike.rs` | xdb | NEW | `difftest` | SpikeBackend (FFI, experimental) |
| `xdb/src/difftest/spike_wrapper.cc` | xdb | NEW | build | C++ Spike wrapper |
| `xdb/src/difftest/spike_ffi.h` | xdb | NEW | build | C header |
| `xdb/build.rs` | xdb | NEW | `difftest` | cc build for Spike |
| `xdb/src/cmd.rs` | xdb | MOD | `difftest` | dt commands, step/continue hooks |
| `xdb/src/cli.rs` | xdb | MOD | `difftest` | Dt subcommand |
| `xdb/src/main.rs` | xdb | MOD | `difftest` | Wiring, runtime path |

---

## Trade-offs

- T-1: **sstep=0x1 + PhyMemMode:1** — Correct. Requires modern QEMU (7.0+). Older QEMU = attach failure. Acceptable trade.
- T-2: **Spike experimental** — Honest scope. QEMU is production path. Spike for users who need it and accept instability.
- T-3: **CoreContext always compiled** — Small overhead (one Vec allocation per context() call). Only called during debug/difftest step, not hot path. Clean crate boundary worth it.
- T-4: **Duplicate CSR whitelist (xcore + xdb/qemu)** — Both use same addr constants. Kept in sync via review. Alternative (shared const) would require xcore to export the table, which is acceptable and could be done.

---

## Validation

[**Unit Tests**]

- V-UT-1: GDB packet checksum.
- V-UT-2: parse_gdb_regs RV64/RV32.
- V-UT-3: parse_hex_le / encode_le_hex round-trip.
- V-UT-4: ArchSnapshot::diff identical -> None.
- V-UT-5: ArchSnapshot::diff PC mismatch.
- V-UT-6: ArchSnapshot::diff GPR mismatch.
- V-UT-7: ArchSnapshot::diff privilege mismatch.
- V-UT-8: ArchSnapshot::diff CSR mismatch.
- V-UT-9: ArchSnapshot::diff masked mip bits ignored.
- V-UT-10: CoreContext::context() captures correct state.
- V-UT-11: ArchSnapshot::from_context preserves all fields.

[**Integration Tests**]

- V-IT-1: Difftest (QEMU) on cpu-tests-rs — zero divergence.
- V-IT-2: Difftest (QEMU) on am-tests — MMIO skip, no false divergence.
- V-IT-3: Difftest (Spike) on cpu-tests-rs — zero divergence.
- V-IT-4: Intentional divergence caught.
- V-IT-5: `dt attach`/`detach`/`status` lifecycle.
- V-IT-6: `dt attach` without load -> error.

[**Failure / Robustness**]

- V-F-1: QEMU not in PATH.
- V-F-2: sstep unsupported -> failure.
- V-F-3: PhyMemMode unsupported -> failure.
- V-F-4: Spike init fails.

[**Edge Cases**]

- V-E-1: First instruction divergence.
- V-E-2: MMIO then non-MMIO.
- V-E-3: Compressed instruction.
- V-E-4: Trap (ecall) — CSRs match.
- V-E-5: mret/sret privilege.
- V-E-6: ebreak halt sync.
- V-E-7: Timer interrupt with sstep=0x1.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (per-inst) | V-IT-1, V-IT-3, V-IT-4, V-UT-4..9 |
| G-2 (two backends) | V-IT-1+3 |
| G-3 (PC+GPR+priv+CSR) | V-UT-5..9, V-E-4, V-E-5 |
| G-4 (MMIO skip) | V-IT-2, V-E-2 |
| G-5 (feature-gated) | Build without: zero code |
| G-6 (monitor cmds) | V-IT-5, V-IT-6 |
| G-7 (interrupt) | V-E-7 |
| G-8 (backend-neutral) | CoreContext, no QEMU in xcore |
| G-9 (PhyMemMode) | V-F-3 |
| C-8 (sstep) | V-F-2 |
| C-11 (Spike experimental) | V-IT-3, V-F-4 |
