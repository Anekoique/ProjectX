# `difftest` PLAN `03`

> Status: Revised
> Feature: `difftest`
> Iteration: `03`
> Owner: Executor
> Depends on:
> - Previous Plan: `02_PLAN.md`
> - Review: `02_REVIEW.md`
> - Master Directive: `02_MASTER.md`

---

## Summary

Final revision addressing all round-02 blockers. Key changes: (1) QEMU stepping uses `sstep=0x1` (interrupts+timers enabled) so interrupt delivery is actually verified, not synced away; (2) xcore's `DifftestOps` is backend-neutral — no QEMU register numbers or binary names; QEMU-specific mapping lives in xdb; (3) `dt attach` uses runtime-tracked binary path, not `option_env!`; (4) unsupported `sstep` is a hard attach failure; (5) Spike backend designed with real FFI C API via `cc` + `spike-diff` wrapper; (6) difftest code in `debug.rs` behind `cfg(feature = "difftest")` per M-001; (7) simplified structures per M-002/M-003.

## Log

[**Feature Introduce**]

- QEMU `sstep=0x1`: interrupts and timers fire during single-step, making interrupt delivery comparable between DUT and REF.
- Spike backend with real FFI: `spike-diff` C wrapper compiled via `cc` crate, exposing init/step/get_regs/sync_regs/get_csr.
- Runtime binary path tracking in xdb for `dt attach`.
- Backend-neutral xcore: `DifftestOps` only exposes CSR address + name + mask. QEMU regnum mapping and Spike FFI details live in xdb.

[**Review Adjustments**]

- R-001 (sstep makes interrupts untestable): Changed to `sstep=0x1`. Interrupts fire during QEMU single-step. No more sync-away for interrupt divergence — a real mismatch is a real bug.
- R-002 (unsupported sstep fallback): Hard failure. `dt attach` aborts if `Qqemu.sstep` is unsupported.
- R-003 (QEMU details in xcore): Removed `qemu_regnum`, `qemu_bin()`, `qemu_priv_regnum()` from xcore. `DifftestOps` only has CSR addr/name/mask. xdb maps CSR addr to QEMU regnum.
- R-004 (X_FILE compile-time): xdb tracks `loaded_binary_path: Option<String>` at runtime. `dt attach` reads it.
- R-005 (RAM-write sync): Narrowed claim — "for current single-core bare-metal, one instruction does not combine RAM write + MMIO write". Documented as limitation.
- R-006 (M-003 literal): M-003 from round-01 said "test feature" — interpreted as `debug` feature (the existing feature that enables test/debug hooks). Explicitly acknowledged.

[**Master Compliance**]

- M-001 (no new riscv file): DifftestOps impl goes in `cpu/riscv/debug.rs` behind `cfg(feature = "difftest")`.
- M-002 (clean/concise/elegant): Simplified ArchSnapshot to use arrays not Vecs where possible. Flattened module structure.
- M-003 (reduce complex structure): Merged DiffCsrEntry fields into a simple tuple array. Removed separate `backend.rs` — trait goes in `difftest/mod.rs`.
- M-004 (Spike detail): Full Spike FFI design with C wrapper, function signatures, build.rs, and SpikeBackend impl.
- M-005 (fix REVIEW problems): All R-001..R-005 addressed.

### Changes from Previous Round

[**Added**]
- QEMU `sstep=0x1` (interrupts enabled during step)
- Spike FFI backend with C wrapper and build system
- Runtime binary path in xdb
- CSR addr-based mapping (backend-neutral xcore)
- xdb-side QEMU regnum table

[**Changed**]
- `sstep=0x7` -> `sstep=0x1` (interrupt-preserving)
- DifftestOps: removed qemu_regnum/qemu_bin/qemu_priv_regnum
- DiffCsrEntry: removed qemu_regnum field, uses CSR addr instead
- `option_env!("X_FILE")` -> runtime `loaded_binary_path`
- Unsupported sstep: warn -> hard failure
- Separate `backend.rs` -> merged into `difftest/mod.rs`
- New riscv/difftest.rs -> merged into riscv/debug.rs

[**Removed**]
- Interrupt delivery divergence sync-away logic
- QEMU-specific fields in xcore traits
- Separate `cpu/riscv/difftest.rs` file
- Separate `difftest/backend.rs` file

[**Unresolved**]
- Spike build requires Spike source tree (documented as prerequisite)
- mtime wall-clock drift (mitigated by MTIP mask, full fix deferred)

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | `sstep=0x1`, interrupts comparable |
| Review | R-002 | Accepted | Hard failure on unsupported sstep |
| Review | R-003 | Accepted | xcore backend-neutral; QEMU mapping in xdb |
| Review | R-004 | Accepted | Runtime binary path in xdb |
| Review | R-005 | Accepted | Narrowed claim, documented as limitation |
| Review | R-006 | Accepted | M-003 explicitly clarified as "debug" feature |
| Master | M-001 | Applied | DifftestOps in debug.rs, not new file |
| Master | M-002 | Applied | Simplified structures |
| Master | M-003 | Applied | Flattened modules, fewer types |
| Master | M-004 | Applied | Full Spike FFI design below |
| Master | M-005 | Applied | All R-001..R-005 fixed |

---

## Spec

[**Goals**]

- G-1: Per-instruction state comparison (DUT vs REF), halt on first divergence.
- G-2: Two backends: QEMU (GDB RSP) and Spike (FFI). Pluggable via `DiffBackend` trait.
- G-3: Compare PC + GPR[1..31] + privilege + whitelisted CSRs (masked).
- G-4: MMIO-skip: sync DUT->REF on MMIO-touching instructions.
- G-5: Feature-gated `cfg(feature = "difftest")` in xcore + xdb. Zero cost disabled.
- G-6: Monitor commands: `dt attach qemu|spike`, `dt detach`, `dt status`.
- G-7: Interrupt-preserving: QEMU steps with `sstep=0x1`, interrupts fire normally.
- G-8: Backend-neutral xcore: no QEMU/Spike protocol details in xcore.

- NG-1: Memory comparison deferred.
- NG-2: Full CSR dump deferred. Whitelist only.
- NG-3: mtime virtual-clock sync deferred (MTIP masked).

[**Architecture**]

```
+--------------------------------------------------------------+
|                         xdb process                          |
|                                                              |
|  +==============+    +================================+      |
|  |  CLI / REPL  |--->|  DiffHarness                   |      |
|  |  dt attach   |    |  - backend: Box<dyn DiffBackend>|      |
|  |  dt detach   |    |  - inst_count: u64             |      |
|  +==============+    +===============+================+      |
|                                      |                       |
|  +==============+                    | per-step               |
|  |  CPU (DUT)   |<------------------+                       |
|  |  DifftestOps |    snapshot -> step REF -> compare         |
|  +==============+                                            |
|                                                              |
|  +=========================================================+ |
|  |  DiffBackend trait                                      | |
|  |  +=================+    +====================+          | |
|  |  | QemuBackend     |    | SpikeBackend       |          | |
|  |  | GDB RSP         |    | FFI (libspike)     |          | |
|  |  | sstep=0x1       |    | spike_difftest_*   |          | |
|  |  | QEMU_CSR_MAP[]  |    | via cc crate       |          | |
|  |  +=======+==========+    +=========+==========+          | |
|  +==========|=========================|====================+ |
+-------------|-------------------------|----------------------+
              | TCP :1234               | in-process FFI
   +----------v----------+    +--------v---------+
   |  QEMU process       |    |  Spike (static)  |
   |  -M virt -s -S      |    |  libriscv.a      |
   +---------------------+    +------------------+
```

[**Invariants**]

- I-1: DUT executes first, then REF stepped. Compare after both complete.
- I-2: MMIO -> skip compare, sync DUT->REF.
- I-3: ebreak -> DUT halts, sync DUT->REF, no compare.
- I-4: QEMU `sstep=0x1` — interrupts fire during step. A divergence in interrupt entry is a real mismatch, not synced away.
- I-5: DiffHarness owned by xdb.
- I-6: Backend processes/libs RAII-cleaned on drop.
- I-7: Compare: PC, GPR[1..31], privilege, whitelisted CSRs (masked).
- I-8: Timer CSRs excluded. mip mask = `!0x82` (SSIP + MTIP).
- I-9: `dt attach` requires loaded binary (runtime path).
- I-10: Feature disabled -> zero difftest code.
- I-11: Unsupported `sstep` -> attach failure (not warn).

[**Data Structure**]

```rust
// === xcore/src/cpu/debug.rs (behind cfg(feature = "difftest")) ===

/// CSR whitelist entry: name, CSR address, comparison mask.
#[cfg(feature = "difftest")]
pub struct DiffCsr {
    pub name: &'static str,
    pub addr: u16,
    pub mask: u64,
}

/// Architectural state snapshot.
#[cfg(feature = "difftest")]
pub struct ArchSnapshot {
    pub pc: u64,
    pub gpr: [u64; 32],
    pub privilege: u64,
    pub csrs: Vec<(&'static str, u64)>, // (name, masked_value)
}

/// Comparison result.
#[cfg(feature = "difftest")]
pub struct DiffMismatch {
    pub inst_count: u64,
    pub reg_name: &'static str,
    pub dut_val: u64,
    pub ref_val: u64,
}

/// Arch-portable difftest interface. Backend-neutral.
#[cfg(feature = "difftest")]
pub trait DifftestOps: DebugOps {
    fn word_size(&self) -> usize;
    fn csr_whitelist(&self) -> &'static [DiffCsr];
    fn snapshot(&self) -> ArchSnapshot;
}

// === xdb/src/difftest/mod.rs ===

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
// -- xcore DifftestOps (in debug.rs, cfg(feature = "difftest")) --
impl DifftestOps for RVCore {
    fn word_size(&self) -> usize;
    fn csr_whitelist(&self) -> &'static [DiffCsr];
    fn snapshot(&self) -> ArchSnapshot;
}

impl ArchSnapshot {
    pub fn diff(&self, other: &ArchSnapshot, inst_count: u64) -> Option<DiffMismatch>;
}

// -- xdb DiffBackend --
// QemuBackend, SpikeBackend implement DiffBackend

// -- xdb DiffHarness --
impl DiffHarness {
    pub fn new(backend: Box<dyn DiffBackend>) -> Self;
    pub fn check_step(&mut self, dut_halted: bool) -> Result<Option<DiffMismatch>, String>;
    pub fn report_mismatch(m: &DiffMismatch);
}

// -- xdb commands --
pub fn cmd_dt_attach(backend: &str, harness: &mut Option<DiffHarness>) -> XResult;
pub fn cmd_dt_detach(harness: &mut Option<DiffHarness>) -> XResult;
pub fn cmd_dt_status(harness: &Option<DiffHarness>);
```

[**CSR Whitelist (RISC-V, defined in debug.rs)**]

```rust
#[cfg(feature = "difftest")]
pub(crate) const RISCV_DIFF_CSRS: &[DiffCsr] = &[
    DiffCsr { name: "mstatus",  addr: 0x300, mask: u64::MAX },
    DiffCsr { name: "mtvec",    addr: 0x305, mask: u64::MAX },
    DiffCsr { name: "mepc",     addr: 0x341, mask: u64::MAX },
    DiffCsr { name: "mcause",   addr: 0x342, mask: u64::MAX },
    DiffCsr { name: "mtval",    addr: 0x343, mask: u64::MAX },
    DiffCsr { name: "medeleg",  addr: 0x302, mask: u64::MAX },
    DiffCsr { name: "mideleg",  addr: 0x303, mask: u64::MAX },
    DiffCsr { name: "mie",      addr: 0x304, mask: u64::MAX },
    DiffCsr { name: "mip",      addr: 0x344, mask: !0x82_u64 },
    DiffCsr { name: "stvec",    addr: 0x105, mask: u64::MAX },
    DiffCsr { name: "sepc",     addr: 0x141, mask: u64::MAX },
    DiffCsr { name: "scause",   addr: 0x142, mask: u64::MAX },
    DiffCsr { name: "stval",    addr: 0x143, mask: u64::MAX },
    DiffCsr { name: "satp",     addr: 0x180, mask: u64::MAX },
];
```

[**Constraints**]

- C-1: QEMU in PATH. Spike source tree for Spike backend.
- C-2: GDB port 1234.
- C-3: Bare-metal `-bios` mode.
- C-4: MMIO hook behind `cfg(feature = "difftest")`.
- C-5: QEMU `-M virt`, RAM at 0x80000000.
- C-6: `difftest` depends on `debug` feature.
- C-7: `dt attach` requires loaded binary (runtime path, not compile-time).
- C-8: QEMU `sstep=0x1` required. Unsupported = attach failure.
- C-9: RAM-write sync limitation: current model assumes one instruction doesn't combine RAM write + MMIO write. Documented.

---

## Implement

### Execution Flow

[**Main Flow**]

1. `DIFFTEST=1 make run` -> `--features difftest` (implies `debug`).
2. `load <file>` -> xdb stores `loaded_binary_path = Some(path)`.
3. `dt attach qemu`:
   a. Check `loaded_binary_path` is Some.
   b. Query arch info: `with_xcpu(|cpu| (cpu.difftest_ops().word_size(), cpu.difftest_ops().csr_whitelist()))`.
   c. Resolve QEMU binary from xdb-side table (ISA cfg -> binary name).
   d. Spawn QEMU, connect GDB, send `Qqemu.sstep=0x1`, verify response. Fail if unsupported.
   e. Run to reset vector, create DiffHarness.
4. `dt attach spike`:
   a. Check `loaded_binary_path` is Some.
   b. Init Spike FFI: `spike_difftest_init(layout, pc, gpr, xlen, isa)`.
   c. Copy binary to Spike memory: `spike_difftest_copy_mem()`.
   d. Create DiffHarness with SpikeBackend.
5. `s` or `c` -> per-step:
   a. DUT step.
   b. `harness.check_step(dut_halted)`:
      - Snapshot DUT via `DifftestOps::snapshot()`.
      - Check MMIO flag.
      - Step REF.
      - MMIO or halt -> sync, skip.
      - Else -> snapshot REF, `diff()`. Mismatch = real bug.
   c. Mismatch -> report, halt. None -> continue.

[**Failure Flow**]

1. QEMU not in PATH -> error.
2. GDB refused -> error.
3. `sstep=0x1` unsupported -> "QEMU version too old, sstep not supported", attach fails.
4. Spike init fails -> error.
5. `dt attach` without loaded binary -> "Load a binary first".

### Implementation Plan

[**Phase 1: xcore changes (~60 lines in debug.rs + ~5 lines in bus.rs)**]

In `xcore/src/cpu/debug.rs`, add behind `cfg(feature = "difftest")`:

```rust
#[cfg(feature = "difftest")]
#[derive(Clone, Copy)]
pub struct DiffCsr {
    pub name: &'static str,
    pub addr: u16,
    pub mask: u64,
}

#[cfg(feature = "difftest")]
pub struct ArchSnapshot {
    pub pc: u64,
    pub gpr: [u64; 32],
    pub privilege: u64,
    pub csrs: Vec<(&'static str, u64)>,
}

#[cfg(feature = "difftest")]
pub struct DiffMismatch {
    pub inst_count: u64,
    pub reg_name: &'static str,
    pub dut_val: u64,
    pub ref_val: u64,
}

#[cfg(feature = "difftest")]
impl ArchSnapshot {
    pub fn from_ref(pc: u64, gpr: [u64; 32], privilege: u64,
                    csrs: Vec<(&'static str, u64)>) -> Self {
        Self { pc, gpr, privilege, csrs }
    }

    pub fn diff(&self, other: &Self, inst_count: u64) -> Option<DiffMismatch> {
        if self.pc != other.pc {
            return Some(DiffMismatch { inst_count, reg_name: "pc",
                dut_val: self.pc, ref_val: other.pc });
        }
        for i in 1..32 {
            if self.gpr[i] != other.gpr[i] {
                return Some(DiffMismatch { inst_count,
                    reg_name: crate::isa::RVReg::from_u8(i as u8).unwrap().name(),
                    dut_val: self.gpr[i], ref_val: other.gpr[i] });
            }
        }
        if self.privilege != other.privilege {
            return Some(DiffMismatch { inst_count, reg_name: "privilege",
                dut_val: self.privilege, ref_val: other.privilege });
        }
        for (i, (name, dut_val)) in self.csrs.iter().enumerate() {
            let ref_val = other.csrs[i].1;
            if dut_val != ref_val {
                return Some(DiffMismatch { inst_count, reg_name: name,
                    dut_val: *dut_val, ref_val });
            }
        }
        None
    }
}

#[cfg(feature = "difftest")]
pub trait DifftestOps: DebugOps {
    fn word_size(&self) -> usize;
    fn csr_whitelist(&self) -> &'static [DiffCsr];
    fn snapshot(&self) -> ArchSnapshot;
}
```

In `xcore/src/cpu/riscv/debug.rs`, add at bottom behind `cfg(feature = "difftest")`:

```rust
#[cfg(feature = "difftest")]
use crate::cpu::debug::{ArchSnapshot, DiffCsr, DifftestOps};

#[cfg(feature = "difftest")]
pub(crate) const RISCV_DIFF_CSRS: &[DiffCsr] = &[
    DiffCsr { name: "mstatus",  addr: 0x300, mask: u64::MAX },
    DiffCsr { name: "mtvec",    addr: 0x305, mask: u64::MAX },
    DiffCsr { name: "mepc",     addr: 0x341, mask: u64::MAX },
    DiffCsr { name: "mcause",   addr: 0x342, mask: u64::MAX },
    DiffCsr { name: "mtval",    addr: 0x343, mask: u64::MAX },
    DiffCsr { name: "medeleg",  addr: 0x302, mask: u64::MAX },
    DiffCsr { name: "mideleg",  addr: 0x303, mask: u64::MAX },
    DiffCsr { name: "mie",      addr: 0x304, mask: u64::MAX },
    DiffCsr { name: "mip",      addr: 0x344, mask: !0x82_u64 },
    DiffCsr { name: "stvec",    addr: 0x105, mask: u64::MAX },
    DiffCsr { name: "sepc",     addr: 0x141, mask: u64::MAX },
    DiffCsr { name: "scause",   addr: 0x142, mask: u64::MAX },
    DiffCsr { name: "stval",    addr: 0x143, mask: u64::MAX },
    DiffCsr { name: "satp",     addr: 0x180, mask: u64::MAX },
];

#[cfg(feature = "difftest")]
impl DifftestOps for RVCore {
    fn word_size(&self) -> usize {
        std::mem::size_of::<crate::config::Word>()
    }

    fn csr_whitelist(&self) -> &'static [DiffCsr] {
        RISCV_DIFF_CSRS
    }

    fn snapshot(&self) -> ArchSnapshot {
        let ops: &dyn crate::cpu::debug::DebugOps = self;
        let pc = ops.read_register("pc").unwrap_or(0);
        let privilege = ops.read_register("privilege").unwrap_or(0);
        let mut gpr = [0u64; 32];
        for i in 0..32 {
            let r = crate::isa::RVReg::from_u8(i as u8).unwrap();
            gpr[i as usize] = ops.read_register(r.name()).unwrap_or(0);
        }
        let csrs = RISCV_DIFF_CSRS.iter().map(|e| {
            let raw = ops.read_register(e.name).unwrap_or(0);
            (e.name, raw & e.mask)
        }).collect();
        ArchSnapshot { pc, gpr, privilege, csrs }
    }
}
```

MMIO hook in `bus.rs` (cfg-gated):

```rust
#[cfg(feature = "difftest")]
mmio_accessed: std::sync::atomic::AtomicBool,

// In MMIO dispatch:
#[cfg(feature = "difftest")]
self.mmio_accessed.store(true, std::sync::atomic::Ordering::Relaxed);

#[cfg(feature = "difftest")]
pub fn take_mmio_flag(&self) -> bool {
    self.mmio_accessed.swap(false, std::sync::atomic::Ordering::Relaxed)
}
```

CPU pass-through in `cpu/mod.rs`:

```rust
#[cfg(feature = "difftest")]
impl<Core: CoreOps + debug::DebugOps + debug::DifftestOps> CPU<Core> {
    pub fn difftest_ops(&self) -> &dyn debug::DifftestOps { &self.core }
    pub fn difftest_snapshot(&self) -> debug::ArchSnapshot { self.core.snapshot() }
    pub fn bus_take_mmio_flag(&self) -> bool {
        self.bus.lock().unwrap().take_mmio_flag()
    }
}
```

Cargo:
```toml
# xcore/Cargo.toml
[features]
debug = []
difftest = ["debug"]
```

[**Phase 2: GDB Client -- `xdb/src/difftest/gdb.rs` (~200 lines)**]

Same as round-02. GDB RSP over TCP: connect, send_packet/recv_packet with checksum, step/cont/read_regs/write_regs/read_register/write_register/write_mem/set_breakpoint/remove_breakpoint/send_raw.

Helpers: `parse_gdb_regs()`, `parse_hex_le()`, `encode_le_hex()`, `encode_regs_hex()`.

[**Phase 3: QEMU Backend -- `xdb/src/difftest/qemu.rs` (~130 lines)**]

```rust
/// QEMU CSR register number = 4096 + CSR address.
/// This mapping lives in xdb, NOT in xcore.
fn csr_addr_to_qemu_regnum(addr: u16) -> usize { 4096 + addr as usize }

const QEMU_PRIV_REGNUM: usize = 4161;

fn qemu_binary_name() -> &'static str {
    if cfg!(isa64) { "qemu-system-riscv64" }
    else { "qemu-system-riscv32" }
}

impl QemuBackend {
    pub fn new(binary_path: &str, reset_vec: usize,
               csr_whitelist: &'static [DiffCsr], word_size: usize,
    ) -> Result<Self, String> {
        let qemu_bin = qemu_binary_name();
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
        // 4. Set sstep=0x1 (ENABLE, no NOIRQ/NOTIMER)
        let resp = gdb.send_raw("Qqemu.sstep=0x1")?;
        if resp.is_empty() || resp == b"E01" {
            let _ = proc.kill();
            return Err("QEMU does not support Qqemu.sstep. Version too old.".into());
        }
        // 5. Run to reset vector
        gdb.set_breakpoint(reset_vec)?;
        gdb.cont()?;
        gdb.remove_breakpoint(reset_vec)?;
        Ok(Self { proc, gdb, csr_whitelist, word_size })
    }
}

impl DiffBackend for QemuBackend {
    fn step(&mut self) -> Result<(), String> { self.gdb.step() }

    fn read_snapshot(&mut self) -> Result<ArchSnapshot, String> {
        let regs = self.gdb.read_regs()?;
        let mut gpr = [0u64; 32];
        gpr.copy_from_slice(&regs[..32]);
        let pc = regs[32];
        let csrs: Vec<_> = self.csr_whitelist.iter().map(|e| {
            let regnum = csr_addr_to_qemu_regnum(e.addr);
            let val = self.gdb.read_register(regnum).unwrap_or(0) & e.mask;
            (e.name, val)
        }).collect();
        let priv_mode = self.gdb.read_register(QEMU_PRIV_REGNUM).unwrap_or(0);
        Ok(ArchSnapshot::from_ref(pc, gpr, priv_mode, csrs))
    }

    fn sync_state(&mut self, snap: &ArchSnapshot) -> Result<(), String> {
        let mut regs = snap.gpr.to_vec();
        regs.push(snap.pc);
        self.gdb.write_regs(&regs)?;
        for (i, (_, val)) in snap.csrs.iter().enumerate() {
            let regnum = csr_addr_to_qemu_regnum(self.csr_whitelist[i].addr);
            self.gdb.write_register(regnum, *val)?;
        }
        Ok(())
    }

    fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String> {
        self.gdb.write_mem(addr, data)
    }

    fn name(&self) -> &str { "qemu" }
}

impl Drop for QemuBackend {
    fn drop(&mut self) {
        let _ = self.proc.kill();
        let _ = self.proc.wait();
    }
}
```

[**Phase 4: Spike Backend -- `xdb/src/difftest/spike.rs` (~150 lines)**]

**C FFI wrapper** (`xdb/src/difftest/spike_ffi.h` + `spike_wrapper.cc`):

The wrapper is a thin C-API layer around Spike's C++ internals, following the ysyx/REMU pattern:

```c
// spike_ffi.h
#pragma once
#include <stdint.h>
#include <stddef.h>

typedef struct spike_ctx spike_ctx_t;

typedef struct {
    uintptr_t base;
    size_t size;
} spike_mem_region_t;

spike_ctx_t* spike_init(const spike_mem_region_t* regions, size_t n,
                        uint32_t init_pc, uint32_t xlen, const char* isa);
void spike_fini(spike_ctx_t* ctx);
int  spike_step(spike_ctx_t* ctx);           // 0=ok, 1=exit, -1=error
void spike_get_pc(spike_ctx_t* ctx, uint64_t* out);
void spike_get_gpr(spike_ctx_t* ctx, uint64_t out[32]);
uint64_t spike_get_csr(spike_ctx_t* ctx, uint16_t addr);
uint64_t spike_get_priv(spike_ctx_t* ctx);
void spike_set_pc(spike_ctx_t* ctx, uint64_t pc);
void spike_set_gpr(spike_ctx_t* ctx, const uint64_t gpr[32]);
void spike_set_csr(spike_ctx_t* ctx, uint16_t addr, uint64_t val);
void spike_copy_mem(spike_ctx_t* ctx, uintptr_t addr, const void* data, size_t len);
int  spike_write_mem(spike_ctx_t* ctx, uintptr_t addr, const void* data, size_t len);
```

**Rust FFI bindings** (`xdb/src/difftest/spike.rs`):

```rust
#[cfg(feature = "difftest")]
mod spike_ffi {
    use std::os::raw::c_char;

    #[repr(C)]
    pub struct SpikeMemRegion { pub base: usize, pub size: usize }

    pub enum SpikeCtx {}

    extern "C" {
        pub fn spike_init(regions: *const SpikeMemRegion, n: usize,
                         init_pc: u32, xlen: u32, isa: *const c_char) -> *mut SpikeCtx;
        pub fn spike_fini(ctx: *mut SpikeCtx);
        pub fn spike_step(ctx: *mut SpikeCtx) -> i32;
        pub fn spike_get_pc(ctx: *mut SpikeCtx, out: *mut u64);
        pub fn spike_get_gpr(ctx: *mut SpikeCtx, out: *mut u64);
        pub fn spike_get_csr(ctx: *mut SpikeCtx, addr: u16) -> u64;
        pub fn spike_get_priv(ctx: *mut SpikeCtx) -> u64;
        pub fn spike_set_pc(ctx: *mut SpikeCtx, pc: u64);
        pub fn spike_set_gpr(ctx: *mut SpikeCtx, gpr: *const u64);
        pub fn spike_set_csr(ctx: *mut SpikeCtx, addr: u16, val: u64);
        pub fn spike_copy_mem(ctx: *mut SpikeCtx, addr: usize,
                             data: *const u8, len: usize);
        pub fn spike_write_mem(ctx: *mut SpikeCtx, addr: usize,
                              data: *const u8, len: usize) -> i32;
    }
}

pub struct SpikeBackend {
    ctx: *mut spike_ffi::SpikeCtx,
    csr_whitelist: &'static [DiffCsr],
}

impl SpikeBackend {
    pub fn new(binary_path: &str, reset_vec: usize,
               csr_whitelist: &'static [DiffCsr], word_size: usize,
    ) -> Result<Self, String> {
        let region = spike_ffi::SpikeMemRegion { base: 0x8000_0000, size: 256 * 1024 * 1024 };
        let xlen = (word_size * 8) as u32;
        let isa = if xlen == 64 { c"rv64imac" } else { c"rv32imac" };
        let ctx = unsafe {
            spike_ffi::spike_init(&region, 1, reset_vec as u32, xlen, isa.as_ptr())
        };
        if ctx.is_null() {
            return Err("Spike init failed".into());
        }
        // Load binary into Spike memory
        let bytes = std::fs::read(binary_path).map_err(|e| format!("read binary: {e}"))?;
        unsafe { spike_ffi::spike_copy_mem(ctx, reset_vec, bytes.as_ptr(), bytes.len()) };
        Ok(Self { ctx, csr_whitelist })
    }
}

impl DiffBackend for SpikeBackend {
    fn step(&mut self) -> Result<(), String> {
        let ret = unsafe { spike_ffi::spike_step(self.ctx) };
        match ret {
            0 => Ok(()),
            1 => Ok(()), // program exit
            _ => Err("Spike step error".into()),
        }
    }

    fn read_snapshot(&mut self) -> Result<ArchSnapshot, String> {
        let mut pc = 0u64;
        let mut gpr = [0u64; 32];
        unsafe {
            spike_ffi::spike_get_pc(self.ctx, &mut pc);
            spike_ffi::spike_get_gpr(self.ctx, gpr.as_mut_ptr());
        }
        let privilege = unsafe { spike_ffi::spike_get_priv(self.ctx) };
        let csrs: Vec<_> = self.csr_whitelist.iter().map(|e| {
            let val = unsafe { spike_ffi::spike_get_csr(self.ctx, e.addr) } & e.mask;
            (e.name, val)
        }).collect();
        Ok(ArchSnapshot::from_ref(pc, gpr, privilege, csrs))
    }

    fn sync_state(&mut self, snap: &ArchSnapshot) -> Result<(), String> {
        unsafe {
            spike_ffi::spike_set_pc(self.ctx, snap.pc);
            spike_ffi::spike_set_gpr(self.ctx, snap.gpr.as_ptr());
            for (i, (_, val)) in snap.csrs.iter().enumerate() {
                spike_ffi::spike_set_csr(self.ctx, self.csr_whitelist[i].addr, *val);
            }
        }
        Ok(())
    }

    fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String> {
        let ret = unsafe { spike_ffi::spike_write_mem(self.ctx, addr, data.as_ptr(), data.len()) };
        if ret == 0 { Ok(()) } else { Err("Spike write_mem failed".into()) }
    }

    fn name(&self) -> &str { "spike" }
}

impl Drop for SpikeBackend {
    fn drop(&mut self) {
        if !self.ctx.is_null() {
            unsafe { spike_ffi::spike_fini(self.ctx) };
        }
    }
}
```

**Build system** (`xdb/build.rs`):

```rust
#[cfg(feature = "difftest")]
fn build_spike_wrapper() {
    let spike_dir = std::env::var("SPIKE_DIR")
        .unwrap_or_else(|_| "/opt/spike".to_string());
    cc::Build::new()
        .cpp(true)
        .std("c++17")
        .file("src/difftest/spike_wrapper.cc")
        .include(&format!("{spike_dir}/include"))
        .compile("spike_wrapper");
    println!("cargo:rustc-link-search=native={spike_dir}/lib");
    println!("cargo:rustc-link-lib=static=riscv");
    println!("cargo:rustc-link-lib=static=softfloat");
    println!("cargo:rustc-link-lib=static=fdt");
    println!("cargo:rustc-link-lib=static=fesvr");
    println!("cargo:rustc-link-lib=static=disasm");
    println!("cargo:rustc-link-lib=dylib=stdc++");
}

fn main() {
    #[cfg(feature = "difftest")]
    build_spike_wrapper();
}
```

[**Phase 5: DiffHarness -- `xdb/src/difftest/mod.rs` (~100 lines)**]

```rust
#[cfg(feature = "difftest")]
pub mod gdb;
#[cfg(feature = "difftest")]
pub mod qemu;
#[cfg(feature = "difftest")]
pub mod spike;

#[cfg(feature = "difftest")]
use xcore::cpu::debug::{ArchSnapshot, DiffCsr, DiffMismatch};

#[cfg(feature = "difftest")]
pub trait DiffBackend {
    fn step(&mut self) -> Result<(), String>;
    fn read_snapshot(&mut self) -> Result<ArchSnapshot, String>;
    fn sync_state(&mut self, snapshot: &ArchSnapshot) -> Result<(), String>;
    fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String>;
    fn name(&self) -> &str;
}

#[cfg(feature = "difftest")]
pub struct DiffHarness {
    backend: Box<dyn DiffBackend>,
    inst_count: u64,
}

#[cfg(feature = "difftest")]
impl DiffHarness {
    pub fn new(backend: Box<dyn DiffBackend>) -> Self {
        Self { backend, inst_count: 0 }
    }

    pub fn check_step(&mut self, dut_halted: bool) -> Result<Option<DiffMismatch>, String> {
        self.inst_count += 1;
        let dut_snap = xcore::with_xcpu(|cpu| cpu.difftest_snapshot());
        let mmio = xcore::with_xcpu(|cpu| cpu.bus_take_mmio_flag());
        self.backend.step()?;
        if mmio || dut_halted {
            self.backend.sync_state(&dut_snap)?;
            return Ok(None);
        }
        let ref_snap = self.backend.read_snapshot()?;
        Ok(dut_snap.diff(&ref_snap, self.inst_count))
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
#[cfg(feature = "difftest")]
let mut diff_harness: Option<difftest::DiffHarness> = None;
let mut loaded_binary_path: Option<String> = option_env!("X_FILE")
    .filter(|s| !s.is_empty()).map(String::from);
```

On `load` command: update `loaded_binary_path`.

In `cli.rs`:
```rust
#[cfg(feature = "difftest")]
#[command(name = "dt")]
Difftest { #[command(subcommand)] subcmd: DtSubcommand },
```

In `cmd.rs`:
```rust
#[cfg(feature = "difftest")]
pub fn cmd_dt_attach(backend: &str, binary_path: &Option<String>,
                     harness: &mut Option<DiffHarness>) -> XResult {
    let path = binary_path.as_deref()
        .ok_or_else(|| { println!("No binary loaded."); xcore::XError::FailedToRead })?;
    let (ws, wl) = xcore::with_xcpu(|cpu| {
        let ops = cpu.difftest_ops();
        (ops.word_size(), ops.csr_whitelist())
    });
    let backend: Box<dyn DiffBackend> = match backend {
        "qemu" => Box::new(QemuBackend::new(path, 0x8000_0000, wl, ws)?),
        "spike" => Box::new(SpikeBackend::new(path, 0x8000_0000, wl, ws)?),
        _ => { println!("Unknown backend. Available: qemu, spike"); return Ok(()); }
    };
    *harness = Some(DiffHarness::new(backend));
    println!("Difftest attached ({}).", backend);
    Ok(())
}
```

[**Phase 7: Build System**]

```toml
# xcore/Cargo.toml
[features]
debug = []
difftest = ["debug"]

# xdb/Cargo.toml
[features]
debug = ["xcore/debug"]
difftest = ["xcore/difftest"]

[build-dependencies]
cc = { version = "1", optional = true }

[features]
difftest = ["xcore/difftest", "cc"]
```

```makefile
ifeq ($(DIFFTEST),1)
  feature_args += --features difftest
endif
```

### File Summary

| File | Crate | New/Mod | cfg | Lines |
|------|-------|---------|-----|-------|
| `xcore/src/cpu/debug.rs` | xcore | MOD | `difftest` | +60 (DiffCsr, ArchSnapshot, DiffMismatch, DifftestOps) |
| `xcore/src/cpu/riscv/debug.rs` | xcore | MOD | `difftest` | +40 (DifftestOps impl, RISCV_DIFF_CSRS) |
| `xcore/src/cpu/mod.rs` | xcore | MOD | `difftest` | +8 (pass-through) |
| `xcore/src/device/bus.rs` | xcore | MOD | `difftest` | +10 (AtomicBool) |
| `xdb/src/difftest/mod.rs` | xdb | NEW | `difftest` | ~60 (DiffBackend, DiffHarness) |
| `xdb/src/difftest/gdb.rs` | xdb | NEW | `difftest` | ~200 (GdbClient) |
| `xdb/src/difftest/qemu.rs` | xdb | NEW | `difftest` | ~130 (QemuBackend) |
| `xdb/src/difftest/spike.rs` | xdb | NEW | `difftest` | ~150 (SpikeBackend + FFI) |
| `xdb/src/difftest/spike_wrapper.cc` | xdb | NEW | build | ~100 (C++ wrapper) |
| `xdb/src/difftest/spike_ffi.h` | xdb | NEW | build | ~20 (C header) |
| `xdb/build.rs` | xdb | NEW | `difftest` | ~20 (cc build) |
| `xdb/src/cmd.rs` | xdb | MOD | `difftest` | +40 (dt commands, step/continue hooks) |
| `xdb/src/cli.rs` | xdb | MOD | `difftest` | +10 (Dt subcommand) |
| `xdb/src/main.rs` | xdb | MOD | `difftest` | +10 (wiring) |

---

## Trade-offs

- T-1: **sstep=0x1 (interrupts enabled) vs sstep=0x7 (suppressed)** — 0x1 preserves interrupt semantics for real comparison. May produce more divergences from mtime drift, but MTIP is masked. Decision: 0x1 (correctness over convenience).

- T-2: **Spike build complexity** — Requires Spike source tree + autotools + C++ compiler. Acceptable for development machine. QEMU backend is the zero-dependency fallback.

- T-3: **RAM-write sync** — Documented limitation: single instruction doesn't combine RAM+MMIO write in current ISA model. If this changes, the MMIO observer can be extended.

---

## Validation

[**Unit Tests**]

- V-UT-1: GDB packet checksum encode/decode.
- V-UT-2: parse_gdb_regs RV64/RV32.
- V-UT-3: parse_hex_le / encode_le_hex round-trip.
- V-UT-4: ArchSnapshot::diff identical -> None.
- V-UT-5: ArchSnapshot::diff PC mismatch.
- V-UT-6: ArchSnapshot::diff GPR mismatch.
- V-UT-7: ArchSnapshot::diff privilege mismatch.
- V-UT-8: ArchSnapshot::diff CSR mismatch.
- V-UT-9: ArchSnapshot::diff masked mip bits ignored.
- V-UT-10: csr_addr_to_qemu_regnum correct.

[**Integration Tests**]

- V-IT-1: Difftest (QEMU) on cpu-tests-rs — zero divergence.
- V-IT-2: Difftest (QEMU) on am-tests — MMIO skip, no false divergence.
- V-IT-3: Difftest (Spike) on cpu-tests-rs — zero divergence.
- V-IT-4: Intentional divergence caught.
- V-IT-5: `dt attach`/`dt detach`/`dt status` lifecycle.
- V-IT-6: `dt attach` without load -> error.

[**Failure / Robustness**]

- V-F-1: QEMU not in PATH -> error.
- V-F-2: sstep unsupported -> attach failure.
- V-F-3: Spike init fails -> error.
- V-F-4: QEMU crash -> harness deactivated.

[**Edge Cases**]

- V-E-1: First instruction divergence.
- V-E-2: MMIO then non-MMIO.
- V-E-3: Compressed instruction.
- V-E-4: Trap (ecall) — CSRs match.
- V-E-5: mret/sret privilege.
- V-E-6: ebreak halt sync.
- V-E-7: Timer interrupt with sstep=0x1 — real comparison.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (per-inst) | V-IT-1, V-IT-3, V-IT-4, V-UT-4..9 |
| G-2 (two backends) | V-IT-1+3 (QEMU+Spike) |
| G-3 (PC+GPR+priv+CSR) | V-UT-5..9, V-E-4, V-E-5 |
| G-4 (MMIO skip) | V-IT-2, V-E-2 |
| G-5 (feature-gated) | Build without: zero code |
| G-6 (monitor cmds) | V-IT-5, V-IT-6 |
| G-7 (interrupt-preserving) | V-E-7, sstep=0x1 |
| G-8 (backend-neutral xcore) | No QEMU/Spike in xcore |
| C-8 (sstep required) | V-F-2 |
