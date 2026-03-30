# `difftest` PLAN `01`

> Status: Revised
> Feature: `difftest`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `00_MASTER.md`

---

## Summary

Redesign difftest as an **xdb-owned monitor harness** with a pluggable backend trait supporting both QEMU and Spike. xcore's role is limited to a minimal snapshot export through the existing `DebugOps` trait — no new RVCore fields, no new cfg blocks, no process management in xcore. The only xcore change is a single `AtomicBool` on `Bus` for MMIO detection (always compiled). The harness lives entirely in xdb behind `cfg(feature = "difftest")`, orchestrating per-instruction DUT/REF comparison of PC + GPR + privilege + whitelisted CSRs.

## Log

[**Feature Introduce**]

- `DiffBackend` trait in xdb: abstract reference emulator operations (init, step, read_regs, sync_state).
- `QemuBackend`: GDB RSP client connecting to `qemu-system-riscv{32,64}`.
- `SpikeBackend`: stub in Phase 1 (returns "not implemented"), FFI in future Phase 2.
- `ArchSnapshot`: structured state capture (PC, GPR[0..31], privilege, whitelisted CSRs) built from existing `DebugOps::read_register()`.
- `DiffHarness`: xdb-owned orchestrator wrapping `cmd_step`/`cmd_continue` with per-instruction REF comparison.
- MMIO skip via `AtomicBool` on Bus, syncing DUT state to REF on MMIO-touching instructions.

[**Review Adjustments**]

- R-001 (ownership in wrong layer): Moved all difftest orchestration to xdb. xcore provides only snapshot via `DebugOps`.
- R-002 (QEMU-only): Introduced `DiffBackend` trait; both QEMU and Spike are first-class backend kinds.
- R-003 (comparison too weak): Expanded to PC + GPR + privilege + 14 whitelisted CSRs.
- R-004 (interrupt/timer underspecified): Added explicit interrupt/timer semantics section; timer CSRs excluded.
- R-005 (MMIO skip incomplete): MMIO-skip now syncs registers to REF.
- R-006 (runtime control): Added xdb commands: `dt attach`, `dt detach`, `dt status`.

[**Master Compliance**]

- M-001 (support both Spike and QEMU): `DiffBackend` trait with `QemuBackend` and `SpikeBackend`.
- M-002 (difftest in xdb): All orchestration, backend management, comparison logic in xdb.
- M-003 (minimize xcore changes): Zero new RVCore fields/cfg. One `AtomicBool` on Bus (always compiled). Snapshot via existing `DebugOps`.
- M-004 (more implementation detail): Full GDB protocol detail, packet format, QEMU spawn args, comparison algorithm, file-level plan below.

### Changes from Previous Round

[**Added**]
- `DiffBackend` trait with step/read_snapshot/sync_state/write_mem/name
- `SpikeBackend` stub
- `ArchSnapshot` with CSR whitelist (14 CSRs) and mask-based comparison
- Privilege mode comparison
- `AtomicBool` on Bus for MMIO detection
- xdb commands: `dt attach <backend>`, `dt detach`, `dt status`
- Interrupt/timer single-step semantics section

[**Changed**]
- Ownership: RVCore -> xdb DiffHarness
- GDB client: xcore -> xdb module
- Comparison scope: PC+GPR -> PC+GPR+privilege+CSR whitelist

[**Removed**]
- All difftest code from xcore
- DifftestContext as xcore type
- GdbClient as xcore type

[**Unresolved**]
- Spike FFI build (deferred to future iteration, stub provided)
- Exact QEMU interrupt delivery during GDB single-step (validated empirically)

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | All difftest in xdb; xcore limited to DebugOps snapshot |
| Review | R-002 | Accepted | DiffBackend trait; QemuBackend + SpikeBackend |
| Review | R-003 | Accepted | PC + GPR + privilege + 14 whitelisted CSRs |
| Review | R-004 | Accepted | Explicit interrupt/timer section; timer CSRs excluded |
| Review | R-005 | Accepted | MMIO-skip syncs registers to REF |
| Review | R-006 | Accepted | xdb `dt` commands for attach/detach/status |
| Master | M-001 | Applied | DiffBackend trait abstracts QEMU/Spike |
| Master | M-002 | Applied | All orchestration in xdb |
| Master | M-003 | Applied | Zero new RVCore fields/cfg; one AtomicBool on Bus |
| Master | M-004 | Applied | Full implementation detail below |

---

## Spec

[**Goals**]

- G-1: Per-instruction state comparison between xemu (DUT) and reference (REF), halting on first divergence with report: instruction count, PC, register name, DUT value, REF value.
- G-2: Pluggable backend trait (`DiffBackend`) supporting QEMU (GDB RSP) and Spike (stub).
- G-3: Compare PC + GPR[1..31] + privilege mode + 14 whitelisted CSRs per instruction.
- G-4: MMIO-skip: when DUT touches MMIO, sync DUT state to REF, skip comparison.
- G-5: Feature-gated (`cfg(feature = "difftest")`) in xdb only. xcore change limited to one `AtomicBool` on Bus.
- G-6: Monitor-integrated: `dt attach qemu`, `dt detach`, `dt status` commands.

- NG-1: Spike FFI deferred. Phase 1 = trait + QEMU backend + Spike stub.
- NG-2: Memory comparison beyond MMIO-skip sync deferred.
- NG-3: Full CSR dump deferred. Only whitelisted CSRs.

[**Architecture**]

```
+--------------------------------------------------------------+
|                         xdb process                          |
|                                                              |
|  +==============+    +================================+      |
|  |  CLI / REPL  |--->|  DiffHarness                   |      |
|  |  dt attach   |    |  - backend: Box<dyn DiffBackend>|      |
|  |  dt detach   |    |  - inst_count: u64             |      |
|  |  dt status   |    |  - active: bool                |      |
|  +==============+    +===============+================+      |
|                                      |                       |
|  +==============+                    | per-step hook         |
|  |  CPU (DUT)   |<------------------+                       |
|  |  via with_   |    step DUT -> snapshot DUT                |
|  |  xcpu()      |    -> step REF -> snapshot REF -> compare  |
|  +==============+                                            |
|                                                              |
|  +=========================================================+ |
|  |            DiffBackend trait                             | |
|  |  +==================+    +===================+          | |
|  |  |  QemuBackend     |    |  SpikeBackend     |          | |
|  |  |  (GDB RSP/TCP)   |    |  (stub)           |          | |
|  |  +========+=========+    +=========+=========+          | |
|  +===========|========================|====================+ |
+--------------|------------------------|----------------------+
               | TCP :1234              | (future: dlopen)
    +----------v----------+             v
    |  QEMU process       |      (Spike shared lib)
    |  -M virt -s -S      |
    |  -bios <binary>     |
    +---------------------+
```

[**Invariants**]

- I-1: DUT executes first, then REF is stepped. Comparison after both complete.
- I-2: On MMIO-touching instruction, skip comparison and sync DUT->REF.
- I-3: DiffHarness owned by xdb mainloop. Created on `dt attach`, destroyed on `dt detach` or exit.
- I-4: QEMU process RAII-managed (killed on `QemuBackend::drop()`).
- I-5: Comparison: PC, GPR[1..31], privilege, whitelisted CSRs. x0 skipped.
- I-6: Timer CSRs (mcycle, minstret, mcounteren, scounteren) excluded.
- I-7: When difftest feature disabled, zero difftest code compiles.
- I-8: `ArchSnapshot` built entirely from `DebugOps::read_register()`.

[**Data Structure**]

```rust
// === xdb/src/difftest/mod.rs ===

pub struct ArchSnapshot {
    pub pc: u64,
    pub gpr: [u64; 32],
    pub privilege: u64,
    pub csrs: Vec<(CsrEntry, u64)>,
}

#[derive(Clone, Copy)]
pub struct CsrEntry {
    pub name: &'static str,
    pub mask: u64,
}

pub struct DiffMismatch {
    pub inst_count: u64,
    pub reg_name: &'static str,
    pub dut_val: u64,
    pub ref_val: u64,
}

pub struct DiffHarness {
    backend: Box<dyn DiffBackend>,
    inst_count: u64,
    active: bool,
}

// === xdb/src/difftest/backend.rs ===

pub trait DiffBackend {
    fn step(&mut self) -> Result<(), String>;
    fn read_snapshot(&mut self) -> Result<ArchSnapshot, String>;
    fn sync_state(&mut self, snapshot: &ArchSnapshot) -> Result<(), String>;
    fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String>;
    fn name(&self) -> &str;
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
}

// === xdb/src/difftest/spike.rs ===

pub struct SpikeBackend; // stub
```

[**API Surface**]

```rust
// -- GdbClient --
impl GdbClient {
    pub fn connect(addr: &str) -> Result<Self, String>;
    pub fn step(&mut self) -> Result<(), String>;
    pub fn read_regs(&mut self) -> Result<Vec<u64>, String>;
    pub fn write_regs(&mut self, regs: &[u64]) -> Result<(), String>;
    pub fn write_mem(&mut self, addr: usize, data: &[u8]) -> Result<(), String>;
    pub fn read_register(&mut self, reg_num: usize) -> Result<u64, String>;
    pub fn write_register(&mut self, reg_num: usize, val: u64) -> Result<(), String>;
    pub fn set_breakpoint(&mut self, addr: usize) -> Result<(), String>;
    pub fn remove_breakpoint(&mut self, addr: usize) -> Result<(), String>;
    pub fn cont(&mut self) -> Result<(), String>;
}

// -- QemuBackend --
impl QemuBackend {
    pub fn new(binary_path: &str, reset_vec: usize) -> Result<Self, String>;
}
impl DiffBackend for QemuBackend { ... }
impl Drop for QemuBackend { ... }

// -- SpikeBackend --
impl DiffBackend for SpikeBackend {
    // All return Err("Spike backend not yet implemented")
}

// -- DiffHarness --
impl DiffHarness {
    pub fn new(backend: Box<dyn DiffBackend>) -> Self;
    pub fn check_step(&mut self) -> Result<Option<DiffMismatch>, String>;
    pub fn report_mismatch(m: &DiffMismatch);
}

// -- ArchSnapshot --
impl ArchSnapshot {
    pub fn from_dut(ops: &dyn xcore::DebugOps) -> Self;
    pub fn from_ref_regs(regs: &[u64], csrs: Vec<(CsrEntry, u64)>, priv_mode: u64) -> Self;
    pub fn diff(&self, other: &ArchSnapshot, inst_count: u64) -> Option<DiffMismatch>;
}

// -- xdb commands --
pub fn cmd_dt_attach(backend: &str, harness: &mut Option<DiffHarness>) -> XResult;
pub fn cmd_dt_detach(harness: &mut Option<DiffHarness>) -> XResult;
pub fn cmd_dt_status(harness: &Option<DiffHarness>);
```

[**CSR Whitelist**]

| CSR | Mask | Reason |
|-----|------|--------|
| mstatus | full | Privilege/trap state |
| mtvec | full | Trap vector |
| mepc | full | Trap return address |
| mcause | full | Trap cause |
| mtval | full | Trap value |
| medeleg | full | Exception delegation |
| mideleg | full | Interrupt delegation |
| mie | full | Interrupt enable |
| mip | `!0x2` | Pending interrupts (exclude SSIP bit 1) |
| stvec | full | S-mode trap vector |
| sepc | full | S-mode trap return |
| scause | full | S-mode trap cause |
| stval | full | S-mode trap value |
| satp | full | Page table base |

Excluded: mcycle, minstret, mcounteren, scounteren, mhartid.

[**Constraints**]

- C-1: QEMU must be in PATH. Checked at `dt attach` with clear error.
- C-2: GDB port 1234 (QEMU default).
- C-3: Bare-metal `-bios` mode only.
- C-4: MMIO detection: one `AtomicBool` on Bus (always compiled).
- C-5: QEMU `-M virt`, RAM at 0x80000000.
- C-6: `difftest` feature independent of `debug` feature.
- C-7: Binary path from `X_FILE` env var.

---

## Implement

### Execution Flow

[**Main Flow**]

1. `DIFFTEST=1 make run` starts xdb with difftest feature.
2. `dt attach qemu` -> spawn QEMU (`-M virt -m 256M -nographic -s -S -bios <binary>`), connect GDB to `127.0.0.1:1234`, continue to reset vector. Create `DiffHarness`.
3. `s` or `c` -> per-step:
   a. `with_xcpu(|cpu| cpu.step())` -- DUT executes.
   b. `harness.check_step()`:
      - `ArchSnapshot::from_dut(ops)` -- capture DUT via DebugOps.
      - Check MMIO: `with_xcpu(|cpu| cpu.bus_take_mmio_flag())`.
      - `backend.step()` -- step REF.
      - If MMIO: `backend.sync_state(dut_snap)` -> Ok(None).
      - Else: `backend.read_snapshot()`, `dut_snap.diff(ref_snap)` -> mismatch or None.
   c. Mismatch -> print report, halt. None -> increment count, continue.
4. `dt detach` -> kill QEMU, drop harness.
5. `load`/`reset` with active harness -> auto re-init.

[**Failure Flow**]

1. QEMU not in PATH -> error, no harness.
2. GDB refused -> error, kill orphan.
3. Protocol error -> harness deactivated.
4. QEMU crash -> GDB read fails, harness deactivated.
5. `dt attach spike` -> "not implemented".

### Implementation Plan

[**Phase 1: GDB Protocol Client -- `xdb/src/difftest/gdb.rs`**]

GDB RSP over TCP. ~200 lines.

Packet format: `$<data>#<2-hex-digit checksum>`

```rust
impl GdbClient {
    pub fn connect(addr: &str) -> Result<Self, String> {
        let stream = TcpStream::connect(addr)
            .map_err(|e| format!("GDB connect: {e}"))?;
        stream.set_read_timeout(Some(Duration::from_secs(5)))
            .map_err(|e| format!("timeout: {e}"))?;
        let mut c = Self { stream, buf: Vec::with_capacity(4096) };
        c.recv_ack()?;
        Ok(c)
    }

    fn send_packet(&mut self, data: &str) -> Result<(), String> {
        let cksum: u8 = data.bytes().fold(0u8, |a, b| a.wrapping_add(b));
        write!(self.stream, "${data}#{cksum:02x}")
            .map_err(|e| format!("send: {e}"))?;
        self.stream.flush().map_err(|e| format!("flush: {e}"))?;
        self.recv_ack()
    }

    fn recv_packet(&mut self) -> Result<Vec<u8>, String> {
        // 1. Skip bytes until '$'
        // 2. Read until '#'
        // 3. Read 2 checksum hex chars
        // 4. Validate checksum
        // 5. Send '+' (ACK)
        // 6. Return payload bytes
    }

    fn send_recv(&mut self, cmd: &str) -> Result<Vec<u8>, String> {
        self.send_packet(cmd)?;
        self.recv_packet()
    }

    fn recv_ack(&mut self) -> Result<(), String> {
        // Read one byte, expect '+'
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
        let hex = encode_regs_hex(regs);
        self.send_recv(&format!("G{hex}"))?; Ok(())
    }

    pub fn read_register(&mut self, num: usize) -> Result<u64, String> {
        let data = self.send_recv(&format!("p{num:x}"))?;
        parse_hex_le(&data)
    }

    pub fn write_register(&mut self, num: usize, val: u64) -> Result<(), String> {
        let hex = encode_le_hex(val);
        self.send_recv(&format!("P{num:x}={hex}"))?; Ok(())
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

// Register layout: QEMU riscv = GPR[0..31] + PC, little-endian.
// RV64: 33 * 16 hex chars = 528. RV32: 33 * 8 hex chars = 264.
fn parse_gdb_regs(hex: &[u8]) -> Result<Vec<u64>, String> {
    let word_size = if hex.len() >= 528 { 8 } else { 4 };
    let chunk = word_size * 2; // hex chars per register
    (0..33)
        .map(|i| parse_hex_le(&hex[i * chunk..(i + 1) * chunk]))
        .collect()
}

fn parse_hex_le(hex: &[u8]) -> Result<u64, String> {
    // Parse pairs of hex chars as bytes, interpret little-endian
    let mut val = 0u64;
    for i in (0..hex.len()).step_by(2) {
        let byte = u8::from_str_radix(
            std::str::from_utf8(&hex[i..i + 2]).map_err(|e| e.to_string())?,
            16,
        ).map_err(|e| e.to_string())?;
        val |= (byte as u64) << (i / 2 * 8);
    }
    Ok(val)
}

fn encode_le_hex(val: u64) -> String {
    // Encode as little-endian hex (word_size bytes)
    let word_size = if cfg!(isa64) { 8 } else { 4 };
    (0..word_size)
        .map(|i| format!("{:02x}", (val >> (i * 8)) & 0xFF))
        .collect()
}

fn encode_regs_hex(regs: &[u64]) -> String {
    regs.iter().map(|r| encode_le_hex(*r)).collect()
}
```

GDB command reference:

| Command | Purpose | Format |
|---------|---------|--------|
| `g` | Read all regs | `$g#67` -> hex dump |
| `G<hex>` | Write all regs | `$G...#xx` -> `OK` |
| `p<n>` | Read register n | `$p20#d3` -> hex |
| `P<n>=<hex>` | Write register n | `$P20=...#xx` -> `OK` |
| `M<a>,<l>:<hex>` | Write memory | -> `OK` |
| `vCont;s:p1.-1` | Single step | -> `$T05...#xx` |
| `vCont;c:p1.-1` | Continue | -> `$T05...#xx` |
| `Z0,<a>,4` | Set breakpoint | -> `OK` |
| `z0,<a>,4` | Remove breakpoint | -> `OK` |

[**Phase 2: ArchSnapshot + Comparison -- `xdb/src/difftest/mod.rs`**]

```rust
const CSR_WHITELIST: &[CsrEntry] = &[
    CsrEntry { name: "mstatus",  mask: u64::MAX },
    CsrEntry { name: "mtvec",    mask: u64::MAX },
    CsrEntry { name: "mepc",     mask: u64::MAX },
    CsrEntry { name: "mcause",   mask: u64::MAX },
    CsrEntry { name: "mtval",    mask: u64::MAX },
    CsrEntry { name: "medeleg",  mask: u64::MAX },
    CsrEntry { name: "mideleg",  mask: u64::MAX },
    CsrEntry { name: "mie",      mask: u64::MAX },
    CsrEntry { name: "mip",      mask: !0x2_u64 },
    CsrEntry { name: "stvec",    mask: u64::MAX },
    CsrEntry { name: "sepc",     mask: u64::MAX },
    CsrEntry { name: "scause",   mask: u64::MAX },
    CsrEntry { name: "stval",    mask: u64::MAX },
    CsrEntry { name: "satp",     mask: u64::MAX },
];

impl ArchSnapshot {
    pub fn from_dut(ops: &dyn xcore::DebugOps) -> Self {
        let pc = ops.read_register("pc").unwrap_or(0);
        let privilege = ops.read_register("privilege").unwrap_or(0);
        let mut gpr = [0u64; 32];
        for i in 0..32 {
            let reg = xcore::isa::RVReg::from_u8(i as u8).unwrap();
            gpr[i] = ops.read_register(reg.name()).unwrap_or(0);
        }
        let csrs = CSR_WHITELIST
            .iter()
            .map(|e| (*e, ops.read_register(e.name).unwrap_or(0)))
            .collect();
        Self { pc, gpr, privilege, csrs }
    }

    pub fn from_ref_regs(
        regs: &[u64],
        csrs: Vec<(CsrEntry, u64)>,
        priv_mode: u64,
    ) -> Self {
        let mut gpr = [0u64; 32];
        gpr.copy_from_slice(&regs[..32]);
        Self { pc: regs[32], gpr, privilege: priv_mode, csrs }
    }

    pub fn diff(&self, other: &ArchSnapshot, inst_count: u64) -> Option<DiffMismatch> {
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
                    reg_name: xcore::isa::RVReg::from_u8(i as u8).unwrap().name(),
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
```

[**Phase 3: QemuBackend -- `xdb/src/difftest/qemu.rs`**]

```rust
impl QemuBackend {
    pub fn new(binary_path: &str, reset_vec: usize) -> Result<Self, String> {
        let qemu_bin = if cfg!(isa64) {
            "qemu-system-riscv64"
        } else {
            "qemu-system-riscv32"
        };
        // Verify QEMU exists
        let status = std::process::Command::new("which").arg(qemu_bin)
            .output().map_err(|e| format!("{e}"))?;
        if !status.status.success() {
            return Err(format!("{qemu_bin} not found in PATH"));
        }
        // Spawn QEMU
        let proc = std::process::Command::new(qemu_bin)
            .args(["-M", "virt", "-m", "256M", "-nographic",
                   "-s", "-S", "-bios", binary_path])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| format!("spawn {qemu_bin}: {e}"))?;
        std::thread::sleep(Duration::from_millis(300));
        // Connect GDB
        let mut gdb = GdbClient::connect("127.0.0.1:1234")?;
        gdb.set_breakpoint(reset_vec)?;
        gdb.cont()?;
        gdb.remove_breakpoint(reset_vec)?;
        Ok(Self { proc, gdb })
    }
}

impl DiffBackend for QemuBackend {
    fn step(&mut self) -> Result<(), String> { self.gdb.step() }

    fn read_snapshot(&mut self) -> Result<ArchSnapshot, String> {
        let regs = self.gdb.read_regs()?;
        let csrs: Vec<(CsrEntry, u64)> = CSR_WHITELIST.iter().map(|e| {
            let num = csr_name_to_qemu_regnum(e.name);
            let val = self.gdb.read_register(num).unwrap_or(0);
            (*e, val)
        }).collect();
        let priv_mode = self.gdb.read_register(QEMU_PRIV_REGNUM).unwrap_or(0);
        Ok(ArchSnapshot::from_ref_regs(&regs, csrs, priv_mode))
    }

    fn sync_state(&mut self, snap: &ArchSnapshot) -> Result<(), String> {
        let mut regs = snap.gpr.to_vec();
        regs.push(snap.pc);
        self.gdb.write_regs(&regs)?;
        for (entry, val) in &snap.csrs {
            let num = csr_name_to_qemu_regnum(entry.name);
            self.gdb.write_register(num, *val)?;
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

/// QEMU riscv GDB: CSR register number = 4096 + CSR address.
fn csr_name_to_qemu_regnum(name: &str) -> usize {
    let addr = match name {
        "mstatus"  => 0x300, "medeleg"  => 0x302,
        "mideleg"  => 0x303, "mie"      => 0x304,
        "mtvec"    => 0x305, "mepc"     => 0x341,
        "mcause"   => 0x342, "mtval"    => 0x343,
        "mip"      => 0x344, "stvec"    => 0x105,
        "sepc"     => 0x141, "scause"   => 0x142,
        "stval"    => 0x143, "satp"     => 0x180,
        _ => 0,
    };
    4096 + addr
}

const QEMU_PRIV_REGNUM: usize = 4161;
```

[**Phase 4: MMIO Detection -- `xcore/src/device/bus.rs`**]

The only xcore change. One `AtomicBool` field on Bus:

```rust
use std::sync::atomic::{AtomicBool, Ordering};

pub struct Bus {
    // ... existing fields ...
    mmio_accessed: AtomicBool,
}

// In Bus::new():
mmio_accessed: AtomicBool::new(false),

// In Bus::read() and Bus::write(), inside MMIO dispatch branch:
self.mmio_accessed.store(true, Ordering::Relaxed);

// Public API:
impl Bus {
    pub fn take_mmio_flag(&self) -> bool {
        self.mmio_accessed.swap(false, Ordering::Relaxed)
    }
}
```

CPU pass-through in `cpu/mod.rs`:

```rust
impl<Core: CoreOps> CPU<Core> {
    pub fn bus_take_mmio_flag(&self) -> bool {
        self.bus.lock().unwrap().take_mmio_flag()
    }
}
```

[**Phase 5: xdb Integration**]

CLI additions in `cli.rs`:

```rust
#[command(name = "dt")]
Difftest {
    #[command(subcommand)]
    subcmd: DtSubcommand,
},

#[derive(Debug, Subcommand)]
enum DtSubcommand {
    Attach { #[arg(default_value = "qemu")] backend: String },
    Detach,
    Status,
}
```

Updated `cmd_step`/`cmd_continue` signatures add `diff: &mut Option<DiffHarness>`:

```rust
pub fn cmd_step(count: u64, watch_mgr: &mut WatchManager,
                diff: &mut Option<DiffHarness>) -> XResult {
    for _ in 0..count {
        let done = with_xcpu(|cpu| -> XResult<bool> {
            cpu.step()?;
            Ok(cpu.is_terminated())
        })?;
        if done { break; }
        if let Some(ref mut h) = diff {
            match h.check_step() {
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

DiffHarness in `main.rs`:

```rust
let mut diff_harness: Option<DiffHarness> = None;
// Pass &mut diff_harness to respond()
```

[**Phase 6: Build System**]

```toml
# xdb/Cargo.toml
[features]
debug = ["xcore/debug"]
difftest = []
```

```makefile
# Makefile
ifeq ($(DIFFTEST),1)
  feature_args += --features difftest
endif
```

### Interrupt/Timer Single-Step Semantics

1. Timer CSRs excluded (mcycle, minstret, mcounteren, scounteren).
2. mip mask excludes SSIP (bit 1).
3. Interrupt delivery divergence reported with context suggesting skip.
4. Validated empirically via V-IT-2 (am-tests with ACLINT timers).

### File Summary

| File | Crate | New? | Description |
|------|-------|------|-------------|
| `xdb/src/difftest/mod.rs` | xdb | NEW | DiffHarness, ArchSnapshot, CsrEntry, DiffMismatch |
| `xdb/src/difftest/backend.rs` | xdb | NEW | DiffBackend trait |
| `xdb/src/difftest/gdb.rs` | xdb | NEW | GdbClient (GDB RSP/TCP) |
| `xdb/src/difftest/qemu.rs` | xdb | NEW | QemuBackend |
| `xdb/src/difftest/spike.rs` | xdb | NEW | SpikeBackend (stub) |
| `xdb/src/cmd.rs` | xdb | MOD | +cmd_dt_attach/detach/status; updated step/continue sigs |
| `xdb/src/cli.rs` | xdb | MOD | +Dt subcommand |
| `xdb/src/main.rs` | xdb | MOD | +DiffHarness wiring |
| `xcore/src/device/bus.rs` | xcore | MOD | +AtomicBool mmio_accessed, mark/take |
| `xcore/src/cpu/mod.rs` | xcore | MOD | +bus_take_mmio_flag() |

---

## Trade-offs

- T-1: **MMIO detection: AtomicBool vs observer trait** -- AtomicBool is one field + one store. Observer adds trait/generic for one boolean. Decision: AtomicBool (M-003 compliance).

- T-2: **CSR read from QEMU: `p` per-CSR (14 round-trips) vs batch** -- `p` is standard GDB, portable. Batch is non-standard. Decision: `p` (correctness over speed).

- T-3: **Spike Phase 1: full FFI vs stub** -- Stub. QEMU proves the framework; Spike FFI is a separate iteration.

---

## Validation

[**Unit Tests**]

- V-UT-1: GDB packet encode -- `$g#67` checksum.
- V-UT-2: GDB packet decode -- `$OK#9a` valid, bad checksum rejected.
- V-UT-3: parse_gdb_regs -- 33 regs x 8 bytes, little-endian (RV64).
- V-UT-4: parse_gdb_regs -- 33 regs x 4 bytes (RV32).
- V-UT-5: ArchSnapshot::diff -- identical -> None.
- V-UT-6: ArchSnapshot::diff -- PC mismatch -> Some("pc").
- V-UT-7: ArchSnapshot::diff -- GPR[5] mismatch -> Some("t0").
- V-UT-8: ArchSnapshot::diff -- privilege mismatch.
- V-UT-9: ArchSnapshot::diff -- CSR mismatch (mstatus).
- V-UT-10: ArchSnapshot::diff -- mip masked bit ignored.
- V-UT-11: CsrEntry mask semantics.

[**Integration Tests**]

- V-IT-1: Difftest on cpu-tests-rs (31 tests) -- zero divergence.
- V-IT-2: Difftest on am-tests -- MMIO skip, no false divergence.
- V-IT-3: Intentional divergence -- caught at correct PC.
- V-IT-4: `dt attach`/`dt detach`/`dt status` lifecycle.

[**Failure / Robustness**]

- V-F-1: QEMU not in PATH -> error.
- V-F-2: QEMU crash -> harness deactivated.
- V-F-3: GDB bad checksum -> error.
- V-F-4: `dt attach spike` -> "not implemented".

[**Edge Cases**]

- V-E-1: First instruction divergence.
- V-E-2: MMIO then non-MMIO -> sync then resume.
- V-E-3: Compressed instruction PC advance.
- V-E-4: Trap (ecall) -- CSRs match.
- V-E-5: mret/sret -- privilege comparison.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (per-inst) | V-IT-1, V-IT-3, V-UT-5..11 |
| G-2 (pluggable) | V-IT-4, V-F-4 |
| G-3 (PC+GPR+priv+CSR) | V-UT-6..10, V-E-4, V-E-5 |
| G-4 (MMIO skip) | V-IT-2, V-E-2 |
| G-5 (xdb feature) | Build without feature: zero difftest code |
| G-6 (monitor cmds) | V-IT-4 |
| C-1 (QEMU) | V-F-1 |
| C-4 (AtomicBool) | V-IT-2 |
