# `difftest` PLAN `02`

> Status: Revised
> Feature: `difftest`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `01_MASTER.md`

---

## Summary

Refine difftest to address all round-01 blockers: (1) truthful QEMU-first scope with Spike reserved, not claimed; (2) feature-gated MMIO hook on Bus behind `cfg(feature = "difftest")` for zero-cost disabled builds; (3) concrete QEMU single-step interrupt configuration via `Qqemu.sstep=0x7` packet; (4) `difftest` depends on `debug` feature explicitly; (5) arch-level abstraction via `DifftestOps` trait for ISA-portable snapshot/comparison. All difftest orchestration remains in xdb.

## Log

[**Feature Introduce**]

- `DifftestOps` trait in xcore: arch-level abstraction for snapshot export and QEMU register mapping, implemented per-ISA (M-001). Separates arch-specific knowledge (CSR whitelist, QEMU regnum mapping, word size) from the generic harness.
- Concrete QEMU `sstep` configuration: send `Qqemu.sstep=0x7` after connect to guarantee IRQ+timer suppression during single-step.
- `ebreak` detection: DUT halt-on-ebreak triggers sync instead of comparison.
- Extended `mip` mask: `!0x82` (excludes both SSIP bit 1 and MTIP bit 7).

[**Review Adjustments**]

- R-001 (Spike only nominal): Narrowed scope to "QEMU implemented, Spike interface reserved". M-001 from round-00 marked partially satisfied — trait exists, only QEMU backend delivered.
- R-002 (always-on Bus change): MMIO hook is now `cfg(feature = "difftest")` on xcore. Zero cost when disabled. Summary/goals/constraints updated to be truthful.
- R-003 (interrupt/timer unresolved): Added concrete `Qqemu.sstep=0x7` configuration, mtime divergence analysis, MTIP mask, ebreak handling. No longer "validated empirically" — mechanism is specified.
- R-004 (debug/difftest boundary): `difftest` explicitly depends on `debug` in Cargo features. Snapshot uses `DebugOps`. Build system updated.
- R-005 (runtime contract): `dt attach` requires a previously loaded binary. Explicit precondition documented.
- R-006 (RAM-write sync overstated): Explicitly rejected RAM-write sync with reasoning — MMIO instructions write to device registers (not RAM), and the `sync_state` call syncs architectural registers which is sufficient. If a single instruction writes both RAM and MMIO, the next comparison will catch any RAM-caused divergence.

[**Master Compliance**]

- M-001 (arch abstraction): New `DifftestOps` trait in xcore with ISA-specific CSR whitelist, QEMU regnum mapping, word size, and snapshot construction. Implemented for RISC-V, stubbed for LoongArch.
- M-002 (fix R-001..R-005): All addressed — see Response Matrix.
- M-003 (difftest depends on debug): `difftest = ["debug"]` in xcore, `difftest = ["xcore/difftest"]` in xdb.

### Changes from Previous Round

[**Added**]
- `DifftestOps` trait in xcore (arch abstraction per M-001)
- `Qqemu.sstep=0x7` explicit configuration in QemuBackend::new()
- `ebreak` detection and sync-instead-of-compare
- Extended mip mask `!0x82` (SSIP + MTIP)
- `dt attach` precondition: binary must be loaded
- `difftest` -> `debug` feature dependency

[**Changed**]
- MMIO hook: always-compiled AtomicBool -> `cfg(feature = "difftest")` gated
- Spike: "implemented stub" -> "interface reserved, not implemented"
- Interrupt/timer: "validated empirically" -> concrete `sstep` mechanism
- Scope claims narrowed to match actual deliverables

[**Removed**]
- Claim that Spike backend is delivered in this round
- Claim that difftest is independent of debug feature
- "Validated empirically" wording for interrupt semantics

[**Unresolved**]
- Spike FFI backend (reserved, not designed)
- mtime wall-clock drift during long debugger pauses (mitigated by MTIP mask, full fix requires virtual clock mode)

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Scope narrowed: "QEMU implemented, Spike reserved" |
| Review | R-002 | Accepted | MMIO hook is `cfg(feature = "difftest")` |
| Review | R-003 | Accepted | Concrete `Qqemu.sstep=0x7` + MTIP mask + ebreak handling |
| Review | R-004 | Accepted | `difftest` depends on `debug` explicitly |
| Review | R-005 | Accepted | `dt attach` requires loaded binary |
| Review | R-006 | Rejected | RAM-write sync unnecessary: MMIO writes go to devices not RAM; register sync is sufficient; any RAM divergence caught on next comparison |
| Master | M-001 | Applied | `DifftestOps` trait for arch abstraction |
| Master | M-002 | Applied | All R-001..R-005 fixed |
| Master | M-003 | Applied | `difftest = ["debug"]` in Cargo features |

---

## Spec

[**Goals**]

- G-1: Per-instruction state comparison between xemu (DUT) and QEMU (REF), halting on first divergence with report.
- G-2: Pluggable `DiffBackend` trait. QEMU backend implemented. Spike interface reserved.
- G-3: Compare PC + GPR[1..31] + privilege + whitelisted CSRs (arch-defined via `DifftestOps`).
- G-4: MMIO-skip: sync DUT->REF on MMIO-touching instructions.
- G-5: Feature-gated: `cfg(feature = "difftest")` in both xcore (MMIO hook + DifftestOps) and xdb (harness). Zero cost when disabled.
- G-6: Monitor commands: `dt attach qemu`, `dt detach`, `dt status`.
- G-7: Arch-portable: `DifftestOps` trait abstracts ISA-specific snapshot/comparison/QEMU-mapping.

- NG-1: Spike backend not implemented (interface reserved via `DiffBackend` trait).
- NG-2: Memory comparison deferred.
- NG-3: Full CSR dump deferred. Whitelist only.
- NG-4: mtime virtual-clock synchronization deferred. MTIP masked instead.

[**Architecture**]

```
+--------------------------------------------------------------+
|                         xdb process                          |
|                                                              |
|  +==============+    +================================+      |
|  |  CLI / REPL  |--->|  DiffHarness                   |      |
|  |  dt attach   |    |  - backend: Box<dyn DiffBackend>|      |
|  |  dt detach   |    |  - inst_count: u64             |      |
|  |  dt status   |    +===============+================+      |
|  +==============+                    |                       |
|                                      | per-step hook         |
|  +==============+                    |                       |
|  |  CPU (DUT)   |<------------------+                       |
|  |  DebugOps    |    snapshot via DifftestOps                |
|  |  DifftestOps |    -> step REF -> compare                  |
|  +==============+                                            |
|                                                              |
|  +=========================================================+ |
|  |  DiffBackend trait                                      | |
|  |  +==================+    +===================+          | |
|  |  |  QemuBackend     |    |  (future backends) |          | |
|  |  |  GDB RSP + sstep |    |                   |          | |
|  |  +========+=========+    +===================+          | |
|  +===========|=========================================+   | |
+--------------|---------------------------------------------+-+
               | TCP :1234
    +----------v----------+
    |  QEMU process       |
    |  -M virt -s -S      |
    |  Qqemu.sstep=0x7    |
    +---------------------+
```

**Crate boundaries**:
- `xcore`: `DifftestOps` trait (arch snapshot, CSR whitelist, QEMU regnum map) + MMIO `AtomicBool` on Bus. Both behind `cfg(feature = "difftest")`.
- `xdb`: `DiffHarness`, `DiffBackend`, `QemuBackend`, `GdbClient`, CLI commands. Behind `cfg(feature = "difftest")`.

[**Invariants**]

- I-1: DUT executes first, then REF is stepped. Compare after both complete.
- I-2: MMIO-touching instruction -> skip compare, sync DUT->REF registers.
- I-3: `ebreak` in DUT -> halt detection, sync DUT->REF, no comparison.
- I-4: QEMU single-step mask = `0x7` (NOIRQ + NOTIMER). xemu's `check_pending_interrupts()` may still fire — this is detected as an interrupt-delivery divergence and handled by syncing.
- I-5: DiffHarness owned by xdb mainloop.
- I-6: QEMU RAII-killed on drop.
- I-7: Comparison: PC, GPR[1..31], privilege, whitelisted CSRs with masks.
- I-8: Timer CSRs excluded. mip mask = `!0x82` (SSIP + MTIP).
- I-9: `dt attach` requires binary loaded (`X_FILE` set and `cpu.load()` done).
- I-10: Feature disabled -> zero difftest code in both xcore and xdb.

[**Data Structure**]

```rust
// === xcore/src/cpu/difftest.rs (NEW, cfg(feature = "difftest")) ===

/// CSR entry for difftest comparison.
#[derive(Clone, Copy)]
pub struct DiffCsrEntry {
    pub name: &'static str,
    pub mask: u64,
    pub qemu_regnum: usize,  // 4096 + csr_addr for QEMU
}

/// Arch-portable difftest interface. Implemented per-ISA.
pub trait DifftestOps: super::debug::DebugOps {
    /// Word size in bytes (4 for RV32, 8 for RV64).
    fn word_size(&self) -> usize;

    /// CSR whitelist for comparison.
    fn csr_whitelist(&self) -> &'static [DiffCsrEntry];

    /// QEMU privilege mode pseudo-register number.
    fn qemu_priv_regnum(&self) -> usize;

    /// QEMU binary name (e.g. "qemu-system-riscv64").
    fn qemu_bin(&self) -> &'static str;

    /// Capture current DUT state as snapshot.
    fn snapshot(&self) -> ArchSnapshot;
}

/// Architectural state snapshot.
pub struct ArchSnapshot {
    pub pc: u64,
    pub gpr: [u64; 32],
    pub privilege: u64,
    pub csrs: Vec<(DiffCsrEntry, u64)>,
}

/// Comparison result.
pub struct DiffMismatch {
    pub inst_count: u64,
    pub reg_name: &'static str,
    pub dut_val: u64,
    pub ref_val: u64,
}

// === xdb/src/difftest/backend.rs ===

pub trait DiffBackend {
    fn step(&mut self) -> Result<(), String>;
    fn read_snapshot(&mut self) -> Result<ArchSnapshot, String>;
    fn sync_state(&mut self, snapshot: &ArchSnapshot) -> Result<(), String>;
    fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String>;
    fn name(&self) -> &str;
}

// === xdb/src/difftest/mod.rs ===

pub struct DiffHarness {
    backend: Box<dyn DiffBackend>,
    inst_count: u64,
    active: bool,
}

// === xdb/src/difftest/gdb.rs ===

pub struct GdbClient {
    stream: TcpStream,
    buf: Vec<u8>,
}

// === xdb/src/difftest/qemu.rs ===

pub struct QemuBackend {
    proc: Child,
    gdb: GdbClient,
    csr_whitelist: &'static [DiffCsrEntry],
    priv_regnum: usize,
    word_size: usize,
}
```

[**API Surface**]

```rust
// -- DifftestOps (xcore, per-ISA) --
impl DifftestOps for RVCore {
    fn word_size(&self) -> usize;           // cfg_if: 4 or 8
    fn csr_whitelist(&self) -> &'static [DiffCsrEntry];
    fn qemu_priv_regnum(&self) -> usize;    // 4161
    fn qemu_bin(&self) -> &'static str;     // "qemu-system-riscv{32,64}"
    fn snapshot(&self) -> ArchSnapshot;     // via DebugOps::read_register()
}

// -- ArchSnapshot --
impl ArchSnapshot {
    pub fn from_ref_regs(regs: &[u64], csrs: Vec<(DiffCsrEntry, u64)>,
                         priv_mode: u64) -> Self;
    pub fn diff(&self, other: &ArchSnapshot, inst_count: u64) -> Option<DiffMismatch>;
}

// -- GdbClient --
impl GdbClient {
    pub fn connect(addr: &str) -> Result<Self, String>;
    pub fn step(&mut self) -> Result<(), String>;
    pub fn cont(&mut self) -> Result<(), String>;
    pub fn read_regs(&mut self) -> Result<Vec<u64>, String>;
    pub fn write_regs(&mut self, regs: &[u64]) -> Result<(), String>;
    pub fn read_register(&mut self, num: usize) -> Result<u64, String>;
    pub fn write_register(&mut self, num: usize, val: u64) -> Result<(), String>;
    pub fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String>;
    pub fn set_breakpoint(&mut self, addr: usize) -> Result<(), String>;
    pub fn remove_breakpoint(&mut self, addr: usize) -> Result<(), String>;
    pub fn send_raw(&mut self, cmd: &str) -> Result<Vec<u8>, String>;
}

// -- QemuBackend --
impl QemuBackend {
    pub fn new(binary_path: &str, reset_vec: usize,
               csr_whitelist: &'static [DiffCsrEntry],
               priv_regnum: usize, word_size: usize,
               qemu_bin: &str) -> Result<Self, String>;
}
impl DiffBackend for QemuBackend { ... }
impl Drop for QemuBackend { ... }

// -- DiffHarness --
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

[**CSR Whitelist (RISC-V)**]

| CSR | Mask | QEMU regnum | Reason |
|-----|------|-------------|--------|
| mstatus | `u64::MAX` | 4096+0x300 | Privilege/trap state |
| mtvec | `u64::MAX` | 4096+0x305 | Trap vector |
| mepc | `u64::MAX` | 4096+0x341 | Trap return |
| mcause | `u64::MAX` | 4096+0x342 | Trap cause |
| mtval | `u64::MAX` | 4096+0x343 | Trap value |
| medeleg | `u64::MAX` | 4096+0x302 | Exception delegation |
| mideleg | `u64::MAX` | 4096+0x303 | Interrupt delegation |
| mie | `u64::MAX` | 4096+0x304 | Interrupt enable |
| mip | `!0x82` | 4096+0x344 | Pending (exclude SSIP b1 + MTIP b7) |
| stvec | `u64::MAX` | 4096+0x105 | S-mode trap vector |
| sepc | `u64::MAX` | 4096+0x141 | S-mode trap return |
| scause | `u64::MAX` | 4096+0x142 | S-mode trap cause |
| stval | `u64::MAX` | 4096+0x143 | S-mode trap value |
| satp | `u64::MAX` | 4096+0x180 | Page table base |

Excluded: mcycle, minstret, mcounteren, scounteren, mhartid.

[**Constraints**]

- C-1: QEMU in PATH. Checked at `dt attach`.
- C-2: GDB port 1234.
- C-3: Bare-metal `-bios` mode.
- C-4: MMIO hook behind `cfg(feature = "difftest")`.
- C-5: QEMU `-M virt`, RAM at 0x80000000.
- C-6: `difftest` depends on `debug` feature.
- C-7: `dt attach` requires loaded binary.
- C-8: QEMU sstep mask set to `0x7` (NOIRQ+NOTIMER) on connect.

---

## Implement

### Execution Flow

[**Main Flow**]

1. `DIFFTEST=1 make run` -> xdb with `--features difftest` (implies `debug`).
2. Binary loaded via `X_FILE` / `load` command.
3. `dt attach qemu`:
   a. Read arch info from DUT: `with_xcpu(|cpu| (cpu.difftest_ops().qemu_bin(), cpu.difftest_ops().word_size(), cpu.difftest_ops().csr_whitelist(), cpu.difftest_ops().qemu_priv_regnum()))`.
   b. Spawn QEMU: `<qemu_bin> -M virt -m 256M -nographic -s -S -bios <binary>`.
   c. Connect GDB to `127.0.0.1:1234`.
   d. Send `Qqemu.sstep=0x7` to configure single-step mask.
   e. Set breakpoint at reset vector, continue, remove breakpoint.
   f. Create `DiffHarness`.
4. `s` or `c` -> per-step:
   a. `with_xcpu(|cpu| cpu.step())` -> DUT executes.
   b. Check DUT halt (ebreak): if halted, sync DUT->REF, return.
   c. `harness.check_step(dut_halted)`:
      - Snapshot DUT via `DifftestOps::snapshot()`.
      - Check MMIO flag: `with_xcpu(|cpu| cpu.bus_take_mmio_flag())`.
      - `backend.step()` -> step REF.
      - If MMIO or halted: `backend.sync_state(dut_snap)` -> Ok(None).
      - Else: `backend.read_snapshot()`, `dut_snap.diff(ref_snap)`.
      - If interrupt-delivery divergence (DUT PC = mtvec but REF PC = next inst): sync and continue (I-4).
   d. Mismatch -> report, halt. None -> continue.
5. `dt detach` -> kill QEMU.
6. `load`/`reset` -> auto re-init if harness active.

[**Failure Flow**]

1. QEMU not in PATH -> error.
2. GDB connect refused -> error, kill process.
3. `Qqemu.sstep` not supported (old QEMU) -> warn, continue with default.
4. Protocol error -> harness deactivated.
5. QEMU crash -> harness deactivated.
6. `dt attach` without loaded binary -> "Load a binary first".

[**Interrupt Delivery Divergence Detection**]

Because QEMU uses `sstep=0x7` (NOIRQ+NOTIMER), QEMU suppresses interrupts during single-step. xemu's `check_pending_interrupts()` may fire. Detection:

```
After step:
  DUT PC = mtvec (took interrupt)
  REF PC = next sequential instruction (did not take interrupt)
```

When detected: sync DUT state to REF, log "interrupt delivery divergence at inst N, syncing", skip comparison. This is not a bug — it's an expected behavioral difference between the two execution models.

### Implementation Plan

[**Phase 1: xcore -- DifftestOps trait + MMIO hook (~80 lines)**]

New file: `xcore/src/cpu/difftest.rs` (behind `cfg(feature = "difftest")`)

```rust
#[cfg(feature = "difftest")]
pub mod difftest_ops {
    use crate::error::XResult;

    #[derive(Clone, Copy)]
    pub struct DiffCsrEntry {
        pub name: &'static str,
        pub mask: u64,
        pub qemu_regnum: usize,
    }

    pub struct ArchSnapshot {
        pub pc: u64,
        pub gpr: [u64; 32],
        pub privilege: u64,
        pub csrs: Vec<(DiffCsrEntry, u64)>,
    }

    pub struct DiffMismatch {
        pub inst_count: u64,
        pub reg_name: &'static str,
        pub dut_val: u64,
        pub ref_val: u64,
    }

    pub trait DifftestOps: super::debug::DebugOps {
        fn word_size(&self) -> usize;
        fn csr_whitelist(&self) -> &'static [DiffCsrEntry];
        fn qemu_priv_regnum(&self) -> usize;
        fn qemu_bin(&self) -> &'static str;
        fn snapshot(&self) -> ArchSnapshot;
    }

    impl ArchSnapshot {
        pub fn from_ref_regs(
            regs: &[u64],
            csrs: Vec<(DiffCsrEntry, u64)>,
            priv_mode: u64,
        ) -> Self {
            let mut gpr = [0u64; 32];
            gpr.copy_from_slice(&regs[..32]);
            Self { pc: regs[32], gpr, privilege: priv_mode, csrs }
        }

        pub fn diff(
            &self,
            other: &ArchSnapshot,
            inst_count: u64,
        ) -> Option<DiffMismatch> {
            // PC -> GPR[1..31] -> privilege -> CSRs (masked)
            if self.pc != other.pc {
                return Some(DiffMismatch {
                    inst_count, reg_name: "pc",
                    dut_val: self.pc, ref_val: other.pc,
                });
            }
            for i in 1..32 {
                if self.gpr[i] != other.gpr[i] {
                    return Some(DiffMismatch {
                        inst_count,
                        reg_name: crate::isa::RVReg::from_u8(i as u8)
                            .unwrap().name(),
                        dut_val: self.gpr[i], ref_val: other.gpr[i],
                    });
                }
            }
            if self.privilege != other.privilege {
                return Some(DiffMismatch {
                    inst_count, reg_name: "privilege",
                    dut_val: self.privilege, ref_val: other.privilege,
                });
            }
            for (i, (entry, dut_val)) in self.csrs.iter().enumerate() {
                let ref_val = other.csrs[i].1;
                if (dut_val & entry.mask) != (ref_val & entry.mask) {
                    return Some(DiffMismatch {
                        inst_count, reg_name: entry.name,
                        dut_val: *dut_val, ref_val,
                    });
                }
            }
            None
        }
    }
}
```

New file: `xcore/src/cpu/riscv/difftest.rs` (behind `cfg(feature = "difftest")`)

```rust
use super::RVCore;
use crate::cpu::difftest::difftest_ops::*;

const RISCV_CSR_WHITELIST: &[DiffCsrEntry] = &[
    DiffCsrEntry { name: "mstatus",  mask: u64::MAX,  qemu_regnum: 4096 + 0x300 },
    DiffCsrEntry { name: "mtvec",    mask: u64::MAX,  qemu_regnum: 4096 + 0x305 },
    DiffCsrEntry { name: "mepc",     mask: u64::MAX,  qemu_regnum: 4096 + 0x341 },
    DiffCsrEntry { name: "mcause",   mask: u64::MAX,  qemu_regnum: 4096 + 0x342 },
    DiffCsrEntry { name: "mtval",    mask: u64::MAX,  qemu_regnum: 4096 + 0x343 },
    DiffCsrEntry { name: "medeleg",  mask: u64::MAX,  qemu_regnum: 4096 + 0x302 },
    DiffCsrEntry { name: "mideleg",  mask: u64::MAX,  qemu_regnum: 4096 + 0x303 },
    DiffCsrEntry { name: "mie",      mask: u64::MAX,  qemu_regnum: 4096 + 0x304 },
    DiffCsrEntry { name: "mip",      mask: !0x82_u64, qemu_regnum: 4096 + 0x344 },
    DiffCsrEntry { name: "stvec",    mask: u64::MAX,  qemu_regnum: 4096 + 0x105 },
    DiffCsrEntry { name: "sepc",     mask: u64::MAX,  qemu_regnum: 4096 + 0x141 },
    DiffCsrEntry { name: "scause",   mask: u64::MAX,  qemu_regnum: 4096 + 0x142 },
    DiffCsrEntry { name: "stval",    mask: u64::MAX,  qemu_regnum: 4096 + 0x143 },
    DiffCsrEntry { name: "satp",     mask: u64::MAX,  qemu_regnum: 4096 + 0x180 },
];

impl DifftestOps for RVCore {
    fn word_size(&self) -> usize {
        std::mem::size_of::<crate::config::Word>()
    }

    fn csr_whitelist(&self) -> &'static [DiffCsrEntry] {
        RISCV_CSR_WHITELIST
    }

    fn qemu_priv_regnum(&self) -> usize { 4161 }

    fn qemu_bin(&self) -> &'static str {
        if cfg!(isa64) { "qemu-system-riscv64" }
        else { "qemu-system-riscv32" }
    }

    fn snapshot(&self) -> ArchSnapshot {
        let ops: &dyn crate::cpu::debug::DebugOps = self;
        let pc = ops.read_register("pc").unwrap_or(0);
        let privilege = ops.read_register("privilege").unwrap_or(0);
        let mut gpr = [0u64; 32];
        for i in 0..32 {
            let reg = crate::isa::RVReg::from_u8(i as u8).unwrap();
            gpr[i as usize] = ops.read_register(reg.name()).unwrap_or(0);
        }
        let csrs = self.csr_whitelist()
            .iter()
            .map(|e| (*e, ops.read_register(e.name).unwrap_or(0)))
            .collect();
        ArchSnapshot { pc, gpr, privilege, csrs }
    }
}
```

MMIO hook on Bus (feature-gated):

```rust
// xcore/src/device/bus.rs
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Bus {
    // ... existing fields ...
    #[cfg(feature = "difftest")]
    mmio_accessed: AtomicBool,
}

impl Bus {
    pub fn new(...) -> Self {
        Self {
            // ... existing ...
            #[cfg(feature = "difftest")]
            mmio_accessed: AtomicBool::new(false),
        }
    }

    #[cfg(feature = "difftest")]
    pub fn mark_mmio_access(&self) {
        self.mmio_accessed.store(true, Ordering::Relaxed);
    }

    #[cfg(feature = "difftest")]
    pub fn take_mmio_flag(&self) -> bool {
        self.mmio_accessed.swap(false, Ordering::Relaxed)
    }
}

// In Bus::read() / Bus::write(), MMIO dispatch branch:
#[cfg(feature = "difftest")]
self.mark_mmio_access();
```

CPU pass-through:

```rust
// xcore/src/cpu/mod.rs
#[cfg(feature = "difftest")]
impl<Core: CoreOps + debug::DebugOps + difftest::difftest_ops::DifftestOps> CPU<Core> {
    pub fn difftest_ops(&self) -> &dyn difftest::difftest_ops::DifftestOps {
        &self.core
    }

    pub fn bus_take_mmio_flag(&self) -> bool {
        self.bus.lock().unwrap().take_mmio_flag()
    }

    pub fn difftest_snapshot(&self) -> difftest::difftest_ops::ArchSnapshot {
        self.core.snapshot()
    }
}
```

Cargo feature:

```toml
# xcore/Cargo.toml
[features]
debug = []
difftest = ["debug"]
```

[**Phase 2: GDB Protocol Client -- `xdb/src/difftest/gdb.rs` (~200 lines)**]

Same as round-01 design. Key addition: `send_raw()` for `Qqemu.sstep` configuration.

```rust
impl GdbClient {
    pub fn connect(addr: &str) -> Result<Self, String> { ... }

    fn send_packet(&mut self, data: &str) -> Result<(), String> {
        let cksum: u8 = data.bytes().fold(0u8, |a, b| a.wrapping_add(b));
        write!(self.stream, "${data}#{cksum:02x}").map_err(|e| format!("{e}"))?;
        self.stream.flush().map_err(|e| format!("{e}"))?;
        self.recv_ack()
    }

    fn recv_packet(&mut self) -> Result<Vec<u8>, String> {
        // Skip until '$', read until '#', read 2 checksum chars
        // Validate, send '+', return payload
    }

    fn send_recv(&mut self, cmd: &str) -> Result<Vec<u8>, String> {
        self.send_packet(cmd)?;
        self.recv_packet()
    }

    /// Send raw maintenance packet (for Qqemu.sstep etc).
    pub fn send_raw(&mut self, cmd: &str) -> Result<Vec<u8>, String> {
        self.send_recv(cmd)
    }

    pub fn step(&mut self) -> Result<(), String> {
        self.send_recv("vCont;s:p1.-1")?; Ok(())
    }

    pub fn cont(&mut self) -> Result<(), String> {
        self.send_recv("vCont;c:p1.-1")?; Ok(())
    }

    pub fn read_regs(&mut self) -> Result<Vec<u64>, String> {
        let data = self.send_recv("g")?;
        parse_gdb_regs(&data)
    }

    pub fn write_regs(&mut self, regs: &[u64]) -> Result<(), String> {
        self.send_recv(&format!("G{}", encode_regs_hex(regs)))?; Ok(())
    }

    pub fn read_register(&mut self, num: usize) -> Result<u64, String> {
        let data = self.send_recv(&format!("p{num:x}"))?;
        parse_hex_le(&data)
    }

    pub fn write_register(&mut self, num: usize, val: u64) -> Result<(), String> {
        self.send_recv(&format!("P{num:x}={}", encode_le_hex(val)))?; Ok(())
    }

    pub fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String> {
        let hex: String = data.iter().map(|b| format!("{b:02x}")).collect();
        self.send_recv(&format!("M{addr:x},{:x}:{hex}", data.len()))?; Ok(())
    }

    pub fn set_breakpoint(&mut self, addr: usize) -> Result<(), String> {
        self.send_recv(&format!("Z0,{addr:x},4"))?; Ok(())
    }

    pub fn remove_breakpoint(&mut self, addr: usize) -> Result<(), String> {
        self.send_recv(&format!("z0,{addr:x},4"))?; Ok(())
    }
}

fn parse_gdb_regs(hex: &[u8]) -> Result<Vec<u64>, String> {
    // Determine word size from response length: >= 528 -> RV64 (8B), else RV32 (4B)
    let word_size = if hex.len() >= 528 { 8 } else { 4 };
    let chunk = word_size * 2;
    (0..33).map(|i| parse_hex_le(&hex[i * chunk..(i + 1) * chunk])).collect()
}

fn parse_hex_le(hex: &[u8]) -> Result<u64, String> {
    let mut val = 0u64;
    for i in (0..hex.len()).step_by(2) {
        let s = std::str::from_utf8(&hex[i..i + 2]).map_err(|e| e.to_string())?;
        let byte = u8::from_str_radix(s, 16).map_err(|e| e.to_string())?;
        val |= (byte as u64) << (i / 2 * 8);
    }
    Ok(val)
}

fn encode_le_hex(val: u64) -> String {
    let ws = if cfg!(isa64) { 8 } else { 4 };
    (0..ws).map(|i| format!("{:02x}", (val >> (i * 8)) & 0xFF)).collect()
}

fn encode_regs_hex(regs: &[u64]) -> String {
    regs.iter().map(|r| encode_le_hex(*r)).collect()
}
```

[**Phase 3: QemuBackend -- `xdb/src/difftest/qemu.rs` (~120 lines)**]

```rust
impl QemuBackend {
    pub fn new(
        binary_path: &str, reset_vec: usize,
        csr_whitelist: &'static [DiffCsrEntry],
        priv_regnum: usize, word_size: usize,
        qemu_bin: &str,
    ) -> Result<Self, String> {
        // 1. Verify QEMU exists
        let status = std::process::Command::new("which").arg(qemu_bin)
            .output().map_err(|e| format!("{e}"))?;
        if !status.status.success() {
            return Err(format!("{qemu_bin} not found in PATH"));
        }
        // 2. Spawn QEMU
        let proc = std::process::Command::new(qemu_bin)
            .args(["-M", "virt", "-m", "256M", "-nographic",
                   "-s", "-S", "-bios", binary_path])
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .spawn().map_err(|e| format!("spawn: {e}"))?;
        std::thread::sleep(Duration::from_millis(300));
        // 3. Connect GDB
        let mut gdb = GdbClient::connect("127.0.0.1:1234")?;
        // 4. Configure sstep mask: NOIRQ + NOTIMER
        if let Err(e) = gdb.send_raw("Qqemu.sstep=0x7") {
            warn!("Could not set sstep mask (old QEMU?): {e}");
        }
        // 5. Run to reset vector
        gdb.set_breakpoint(reset_vec)?;
        gdb.cont()?;
        gdb.remove_breakpoint(reset_vec)?;
        info!("Difftest: QEMU attached (pid {})", proc.id());
        Ok(Self { proc, gdb, csr_whitelist, priv_regnum, word_size })
    }
}

impl DiffBackend for QemuBackend {
    fn step(&mut self) -> Result<(), String> { self.gdb.step() }

    fn read_snapshot(&mut self) -> Result<ArchSnapshot, String> {
        let regs = self.gdb.read_regs()?;
        let csrs: Vec<_> = self.csr_whitelist.iter().map(|e| {
            (*e, self.gdb.read_register(e.qemu_regnum).unwrap_or(0))
        }).collect();
        let priv_mode = self.gdb.read_register(self.priv_regnum).unwrap_or(0);
        Ok(ArchSnapshot::from_ref_regs(&regs, csrs, priv_mode))
    }

    fn sync_state(&mut self, snap: &ArchSnapshot) -> Result<(), String> {
        let mut regs = snap.gpr.to_vec();
        regs.push(snap.pc);
        self.gdb.write_regs(&regs)?;
        for (entry, val) in &snap.csrs {
            self.gdb.write_register(entry.qemu_regnum, *val)?;
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

[**Phase 4: DiffHarness -- `xdb/src/difftest/mod.rs` (~100 lines)**]

```rust
impl DiffHarness {
    pub fn new(backend: Box<dyn DiffBackend>) -> Self {
        Self { backend, inst_count: 0, active: true }
    }

    pub fn check_step(
        &mut self,
        dut_halted: bool,
    ) -> Result<Option<DiffMismatch>, String> {
        self.inst_count += 1;

        // 1. Snapshot DUT
        let dut_snap = xcore::with_xcpu(|cpu| cpu.difftest_snapshot());

        // 2. Check MMIO flag
        let mmio_hit = xcore::with_xcpu(|cpu| cpu.bus_take_mmio_flag());

        // 3. Step REF
        self.backend.step()?;

        // 4. MMIO or halt -> sync, skip compare
        if mmio_hit || dut_halted {
            self.backend.sync_state(&dut_snap)?;
            if mmio_hit {
                debug!("difftest: MMIO skip at inst {}", self.inst_count);
            }
            return Ok(None);
        }

        // 5. Snapshot REF
        let ref_snap = self.backend.read_snapshot()?;

        // 6. Detect interrupt delivery divergence
        // DUT took interrupt (PC = mtvec area) but REF didn't
        if dut_snap.pc != ref_snap.pc {
            let dut_mtvec = dut_snap.csrs.iter()
                .find(|(e, _)| e.name == "mtvec")
                .map(|(_, v)| *v & !0x3); // mask mode bits
            if dut_mtvec == Some(dut_snap.pc) {
                info!("difftest: interrupt delivery divergence at inst {}, syncing",
                      self.inst_count);
                self.backend.sync_state(&dut_snap)?;
                return Ok(None);
            }
        }

        // 7. Compare
        Ok(dut_snap.diff(&ref_snap, self.inst_count))
    }

    pub fn report_mismatch(m: &DiffMismatch) {
        eprintln!(
            "DIFFTEST MISMATCH at instruction {}:\n  \
             register: {}\n  DUT: {:#018x}\n  REF: {:#018x}",
            m.inst_count, m.reg_name, m.dut_val, m.ref_val
        );
    }

    pub fn is_active(&self) -> bool { self.active }
    pub fn inst_count(&self) -> u64 { self.inst_count }
    pub fn backend_name(&self) -> &str { self.backend.name() }
}
```

[**Phase 5: xdb CLI Integration**]

```rust
// cli.rs: new command variant
#[command(name = "dt")]
Difftest {
    #[command(subcommand)]
    subcmd: DtSubcommand,
},

#[derive(Debug, Subcommand)]
enum DtSubcommand {
    /// Attach difftest backend (requires loaded binary)
    Attach { #[arg(default_value = "qemu")] backend: String },
    /// Detach difftest
    Detach,
    /// Show difftest status
    Status,
}
```

cmd.rs additions:

```rust
pub fn cmd_dt_attach(
    backend_name: &str,
    harness: &mut Option<DiffHarness>,
) -> XResult {
    if harness.is_some() {
        println!("Already attached. Detach first.");
        return Ok(());
    }
    let (binary, reset_vec, whitelist, priv_reg, ws, qbin) = xcore::with_xcpu(|cpu| {
        let ops = cpu.difftest_ops();
        (
            option_env!("X_FILE").unwrap_or("").to_string(),
            0x80000000_usize,
            ops.csr_whitelist(),
            ops.qemu_priv_regnum(),
            ops.word_size(),
            ops.qemu_bin(),
        )
    });
    if binary.is_empty() {
        println!("No binary loaded. Use 'load' first.");
        return Ok(());
    }
    match backend_name {
        "qemu" => {
            let backend = QemuBackend::new(&binary, reset_vec, whitelist, priv_reg, ws, qbin)
                .map_err(|e| { println!("Attach failed: {e}"); xcore::XError::FailedToRead })?;
            *harness = Some(DiffHarness::new(Box::new(backend)));
            println!("Difftest attached (qemu).");
        }
        _ => println!("Unknown backend '{backend_name}'. Available: qemu"),
    }
    Ok(())
}

pub fn cmd_dt_detach(harness: &mut Option<DiffHarness>) -> XResult {
    if harness.take().is_some() {
        println!("Difftest detached.");
    } else {
        println!("Not attached.");
    }
    Ok(())
}

pub fn cmd_dt_status(harness: &Option<DiffHarness>) {
    match harness {
        Some(h) => println!("Difftest: active ({}), {} instructions checked",
                            h.backend_name(), h.inst_count()),
        None => println!("Difftest: not attached"),
    }
}
```

Updated cmd_step/cmd_continue: add `diff: &mut Option<DiffHarness>` parameter.

[**Phase 6: Build System**]

```toml
# xcore/Cargo.toml
[features]
debug = []
difftest = ["debug"]

# xdb/Cargo.toml
[features]
debug = ["xcore/debug"]
difftest = ["xcore/difftest"]
```

```makefile
# Makefile
ifeq ($(DIFFTEST),1)
  feature_args += --features difftest
endif
```

### File Summary

| File | Crate | New? | cfg | Description |
|------|-------|------|-----|-------------|
| `xcore/src/cpu/difftest.rs` | xcore | NEW | `difftest` | DifftestOps trait, ArchSnapshot, DiffMismatch, DiffCsrEntry |
| `xcore/src/cpu/riscv/difftest.rs` | xcore | NEW | `difftest` | RVCore DifftestOps impl, RISCV_CSR_WHITELIST |
| `xcore/src/cpu/mod.rs` | xcore | MOD | `difftest` | CPU pass-through: difftest_ops(), bus_take_mmio_flag(), difftest_snapshot() |
| `xcore/src/device/bus.rs` | xcore | MOD | `difftest` | +AtomicBool mmio_accessed, mark/take |
| `xcore/Cargo.toml` | xcore | MOD | - | `difftest = ["debug"]` |
| `xdb/src/difftest/mod.rs` | xdb | NEW | `difftest` | DiffHarness |
| `xdb/src/difftest/backend.rs` | xdb | NEW | `difftest` | DiffBackend trait |
| `xdb/src/difftest/gdb.rs` | xdb | NEW | `difftest` | GdbClient |
| `xdb/src/difftest/qemu.rs` | xdb | NEW | `difftest` | QemuBackend |
| `xdb/src/cmd.rs` | xdb | MOD | `difftest` | cmd_dt_*, updated step/continue |
| `xdb/src/cli.rs` | xdb | MOD | `difftest` | Dt subcommand |
| `xdb/src/main.rs` | xdb | MOD | `difftest` | DiffHarness wiring |
| `xdb/Cargo.toml` | xdb | MOD | - | `difftest = ["xcore/difftest"]` |

---

## Trade-offs

- T-1: **MMIO hook: cfg-gated AtomicBool vs observer trait** -- AtomicBool behind `cfg(feature = "difftest")`. Zero cost disabled. Observer is overkill for one boolean. Addresses R-002/TR-2.

- T-2: **CSR reads from QEMU: per-CSR `p` command** -- 14 round-trips per step. Standard, portable. Acceptable for correctness-focused difftest.

- T-3: **Spike: stub vs real** -- Not delivered. Trait exists for future backend. Truthful scope per R-001/TR-3.

- T-4: **Interrupt divergence: sync vs error** -- Sync (with log). QEMU's `sstep=0x7` inherently suppresses interrupts. Treating divergence as error would make difftest unusable on interrupt-heavy workloads.

---

## Validation

[**Unit Tests**]

- V-UT-1: GDB packet encode checksum.
- V-UT-2: GDB packet decode + bad checksum rejected.
- V-UT-3: parse_gdb_regs RV64 (33 x 8B).
- V-UT-4: parse_gdb_regs RV32 (33 x 4B).
- V-UT-5: ArchSnapshot::diff identical -> None.
- V-UT-6: ArchSnapshot::diff PC mismatch.
- V-UT-7: ArchSnapshot::diff GPR mismatch.
- V-UT-8: ArchSnapshot::diff privilege mismatch.
- V-UT-9: ArchSnapshot::diff CSR mismatch.
- V-UT-10: ArchSnapshot::diff mip masked bits ignored.
- V-UT-11: encode_le_hex / parse_hex_le round-trip.
- V-UT-12: DiffCsrEntry mask=0 ignores all differences.

[**Integration Tests**]

- V-IT-1: Difftest on cpu-tests-rs (31 tests) -- zero divergence.
- V-IT-2: Difftest on am-tests -- MMIO skip, interrupt sync, no false divergence.
- V-IT-3: Intentional divergence -- caught at correct PC.
- V-IT-4: `dt attach`/`dt detach`/`dt status` lifecycle.
- V-IT-5: `dt attach` without loaded binary -> error message.

[**Failure / Robustness**]

- V-F-1: QEMU not in PATH -> error.
- V-F-2: QEMU crash -> harness deactivated.
- V-F-3: GDB bad checksum -> error.
- V-F-4: `Qqemu.sstep` unsupported -> warn, continue.

[**Edge Cases**]

- V-E-1: First instruction divergence.
- V-E-2: MMIO then non-MMIO -> sync then resume.
- V-E-3: Compressed instruction PC advance.
- V-E-4: Trap (ecall) -- CSRs match after trap commit.
- V-E-5: mret/sret -- privilege comparison.
- V-E-6: ebreak -- DUT halts, sync to REF.
- V-E-7: Interrupt delivery divergence -- DUT takes IRQ, QEMU doesn't, sync.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (per-inst) | V-IT-1, V-IT-3, V-UT-5..10 |
| G-2 (pluggable) | V-IT-4, trait compiles |
| G-3 (PC+GPR+priv+CSR) | V-UT-6..10, V-E-4, V-E-5 |
| G-4 (MMIO skip) | V-IT-2, V-E-2 |
| G-5 (feature-gated) | Build without: zero difftest code |
| G-6 (monitor cmds) | V-IT-4, V-IT-5 |
| G-7 (arch-portable) | DifftestOps trait, RVCore impl |
| C-6 (depends on debug) | Cargo feature chain |
| C-8 (sstep=0x7) | V-F-4, QEMU connect sequence |
