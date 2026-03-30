# `difftest` PLAN `00`

> Status: Draft
> Feature: `difftest`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

Add per-instruction differential testing to xemu by comparing CPU state against a reference emulator (QEMU) after each instruction. The DUT (xemu) and REF (QEMU) execute the same binary in lock-step; after every instruction, {PC, GPR[0..31]} are compared. On mismatch, execution halts with a detailed divergence report. Enabled via `cfg(feature = "difftest")` on xcore, controlled by `DIFFTEST=1` in the Makefile.

## Log

None (initial plan).

---

## Spec

[**Goals**]

- G-1: Per-instruction state comparison between xemu (DUT) and QEMU (REF), halting on first divergence with a clear report (PC, register name, DUT value, REF value).
- G-2: GDB Remote Serial Protocol client to communicate with QEMU's GDB stub (`-s -S` flags), supporting: step (`vCont;s`), read registers (`g`), memory write (`M`), set/remove breakpoint (`Z0`/`z0`), continue (`vCont;c`).
- G-3: Feature-gated (`cfg(feature = "difftest")`) — zero cost when disabled. Only the per-instruction comparison hook in `step()` is on the hot path.
- G-4: Transparent MMIO skip — when DUT executes an instruction that touches MMIO (device I/O), sync DUT state to REF instead of comparing, because QEMU's device model differs.
- G-5: Integration with existing build system: `DIFFTEST=1 make run` spawns QEMU, connects, and runs difftest mode.

- NG-1: CSR comparison is not in initial scope (QEMU CSR access requires per-register `p` packets and layout varies by QEMU version). Can be added incrementally.
- NG-2: Spike integration is not in initial scope (requires FFI/C++ build). QEMU is sufficient as first reference.
- NG-3: Memory comparison is not in initial scope (register divergence catches most bugs).

[**Architecture**]

```
┌─────────────────────────────────────────────────────────┐
│                     xemu process                        │
│                                                         │
│  ┌─────────────┐     ┌──────────────────────────────┐   │
│  │  CPU<Core>   │     │  DifftestContext              │   │
│  │  (DUT)       │────▶│  - GdbClient (TCP:1234)      │   │
│  │  step()      │     │  - ref_step()                │   │
│  │  + difftest  │     │  - ref_read_regs()           │   │
│  │    hook      │     │  - ref_memcpy()              │   │
│  └─────────────┘     │  - checkregs()               │   │
│                       └──────────┬───────────────────┘   │
│                                  │ TCP (GDB RSP)         │
└──────────────────────────────────┼───────────────────────┘
                                   │
                    ┌──────────────▼───────────────┐
                    │  QEMU process                │
                    │  qemu-system-riscv{32,64}    │
                    │  -M virt -s -S -nographic    │
                    │  -bios <same binary>         │
                    └──────────────────────────────┘
```

**Crate layout**: The difftest module lives in `xcore` as `cpu/difftest.rs` (DifftestContext + GdbClient). The hook is inserted in `RVCore::step()` behind `cfg(feature = "difftest")`.

[**Invariants**]

- I-1: DUT always executes first, then REF is stepped. Comparison happens after both have executed one instruction.
- I-2: On MMIO instruction (detected by observing bus access during step), skip comparison and sync DUT→REF state instead.
- I-3: DifftestContext is owned by CPU, initialized once during load, destroyed on drop.
- I-4: QEMU process is spawned by xemu and killed on drop (RAII).
- I-5: Register comparison checks PC + GPR[1..31] (x0 is hardwired zero, always skip).
- I-6: When difftest feature is disabled, zero code from the difftest module is compiled.

[**Data Structure**]

```rust
// xcore/src/cpu/difftest.rs

/// GDB Remote Serial Protocol client over TCP.
struct GdbClient {
    stream: TcpStream,
}

/// Snapshot of CPU register state for comparison.
pub struct RegSnapshot {
    pub pc: u64,
    pub gpr: [u64; 32],
}

/// Difftest context managing the reference emulator.
pub struct DifftestContext {
    qemu_proc: Child,
    gdb: GdbClient,
    /// Track whether last DUT instruction touched MMIO
    mmio_accessed: bool,
}
```

[**API Surface**]

```rust
// GdbClient
impl GdbClient {
    fn connect(addr: &str) -> XResult<Self>;
    fn step(&mut self) -> XResult;
    fn read_regs(&mut self) -> XResult<RegSnapshot>;
    fn write_mem(&mut self, addr: usize, data: &[u8]) -> XResult;
    fn continue_to(&mut self, addr: usize) -> XResult;
    fn set_breakpoint(&mut self, addr: usize) -> XResult;
    fn remove_breakpoint(&mut self, addr: usize) -> XResult;
}

// DifftestContext
impl DifftestContext {
    pub fn init(binary_path: &str, reset_vec: usize) -> XResult<Self>;
    pub fn check_after_step(&mut self, dut: &RegSnapshot) -> XResult;
    pub fn sync_from_dut(&mut self, dut: &RegSnapshot) -> XResult;
    pub fn set_mmio_accessed(&mut self);
}

// RegSnapshot
impl RegSnapshot {
    pub fn from_core(core: &RVCore) -> Self;
    pub fn diff(&self, other: &RegSnapshot) -> Option<DifftestMismatch>;
}

pub struct DifftestMismatch {
    pub reg_name: &'static str,
    pub dut_val: u64,
    pub ref_val: u64,
}
```

[**Constraints**]

- C-1: QEMU must be installed and accessible in PATH (`qemu-system-riscv32` or `qemu-system-riscv64`).
- C-2: GDB port 1234 is hardcoded (QEMU default). Future: make configurable.
- C-3: Only bare-metal firmware mode (`-bios`). No Linux kernel difftest.
- C-4: MMIO detection is conservative — any bus access outside RAM range during a step sets the mmio flag.
- C-5: QEMU is started with `-M virt` machine type, matching xemu's memory map (RAM at 0x80000000).
- C-6: The difftest feature flag is independent of the debug feature flag. Both can be enabled simultaneously.

---

## Implement

### Execution Flow

[**Main Flow**]

1. `init_xcore()` → if difftest feature enabled, spawn QEMU with `-s -S -bios <binary>`, connect GDB client to `127.0.0.1:1234`, `continue_to(reset_vector)`.
2. Each `RVCore::step()`:
   a. Execute DUT instruction (existing logic).
   b. `difftest_ctx.gdb.step()` — step REF one instruction.
   c. `RegSnapshot::from_core(self)` — capture DUT state.
   d. `difftest_ctx.gdb.read_regs()` — capture REF state.
   e. If `mmio_accessed`: `sync_from_dut(dut_snapshot)` → skip comparison, clear flag.
   f. Else: `dut_snapshot.diff(ref_snapshot)` → if mismatch, return `XError::DifftestMismatch`.
3. On `DifftestMismatch`: print detailed report (instruction count, PC, divergent register, both values), halt.

[**Failure Flow**]

1. QEMU not found → `XError::DifftestInit("qemu-system-riscv{32,64} not found in PATH")`.
2. GDB connection refused → `XError::DifftestInit("Cannot connect to QEMU GDB stub at 127.0.0.1:1234")`.
3. GDB protocol error (bad checksum, unexpected response) → `XError::DifftestProtocol(msg)`.
4. QEMU crashes mid-execution → GDB read returns error → `XError::DifftestRefCrash`.

[**State Transition**]

- Idle → Initialized: `DifftestContext::init()` succeeds.
- Initialized → Running: first `step()` with difftest hook.
- Running → Failed: `DifftestMismatch` or protocol error.
- Running → Done: DUT program exits normally.
- Any → Cleanup: `DifftestContext::drop()` kills QEMU process.

### Implementation Plan

[**Phase 1: GDB Protocol Client**]

New file: `xcore/src/cpu/difftest/gdb.rs`

- TCP connection to QEMU GDB stub.
- Packet framing: `$<data>#<checksum>` with ACK/NACK.
- Commands: `g` (read regs), `G` (write regs), `M` (write mem), `vCont;s` (step), `Z0`/`z0` (breakpoint), `vCont;c` (continue).
- Register parsing: 33 registers × 8 bytes (RV64) or 4 bytes (RV32), little-endian hex.

[**Phase 2: DifftestContext + RegSnapshot**]

New file: `xcore/src/cpu/difftest/mod.rs`

- QEMU process lifecycle (spawn, connect, drop/kill).
- `RegSnapshot` capture from `RVCore` fields.
- `RegSnapshot::diff()` comparison logic.
- `DifftestMismatch` report formatting.
- MMIO skip logic.

[**Phase 3: Integration into step() + Build System**]

- Add `difftest` field to `RVCore` (behind `cfg`): `Option<DifftestContext>`.
- Hook in `RVCore::step()`: after `retire()`, call difftest check.
- MMIO detection: instrument `Bus::read`/`Bus::write` to set a thread-local or atomic flag when accessing MMIO region.
- Cargo feature: `difftest = []` in xcore's `Cargo.toml`.
- xdb forwards: `difftest = ["xcore/difftest"]`.
- Makefile: `DIFFTEST=1` → `--features difftest`, also passes `X_DIFFTEST_BIN=<path>` for the binary.
- `CPU::load()` initializes DifftestContext when feature is on.

---

## Trade-offs

- T-1: **MMIO detection mechanism**
  - Option A: Thread-local flag set by `Bus::read/write` when addr is in MMIO range. Zero-cost when difftest disabled (flag never read). Simple, but slightly invasive to Bus.
  - Option B: Observer trait on Bus (like REMU). Cleaner abstraction but heavier — adds a trait object or generic parameter to Bus.
  - Option C: Post-hoc detection by comparing DUT PC with known MMIO instruction patterns. Fragile, doesn't generalize.
  - Leaning: Option A — minimal invasion, good enough for bare-metal.

- T-2: **DifftestContext ownership**
  - Option A: Owned by `RVCore` as `Option<DifftestContext>` field (behind cfg). Direct access in step(), no indirection.
  - Option B: Owned by `CPU<Core>` wrapper. Cleaner separation but requires passing context through step() or using a callback.
  - Option C: Global static (like XCPU). Avoids plumbing but introduces coupling.
  - Leaning: Option A — keeps everything in core, matches how breakpoints are stored.

- T-3: **Register width handling (RV32 vs RV64)**
  - Option A: Always use u64 in RegSnapshot, zero-extend RV32 values. Simple, works for both.
  - Option B: Generic RegSnapshot<W> parameterized by word size. Type-safe but adds complexity.
  - Leaning: Option A — QEMU's GDB stub returns u64 for RV64 and u32 for RV32, we can normalize to u64 for both.

- T-4: **When to initialize QEMU**
  - Option A: During `CPU::load()` when binary is loaded. Natural — QEMU needs the same binary.
  - Option B: During `init_xcore()`. Earlier, but binary path not yet known.
  - Leaning: Option A.

---

## Validation

[**Unit Tests**]

- V-UT-1: GDB packet framing — encode/decode `$g#67` correctly, checksum validation.
- V-UT-2: GDB register parsing — parse hex-encoded 33-register response into RegSnapshot for both RV32 and RV64.
- V-UT-3: RegSnapshot::diff — identical snapshots → None, single GPR mismatch → Some with correct name.
- V-UT-4: RegSnapshot::diff — PC mismatch detected before GPR check.
- V-UT-5: RegSnapshot::from_core — captures all 32 GPRs + PC from RVCore.

[**Integration Tests**]

- V-IT-1: Full difftest run on `cpu-tests-rs` — all 31 tests should pass with zero divergence.
- V-IT-2: Difftest on `am-tests` (bare-metal) — UART test triggers MMIO skip, other tests pass clean.
- V-IT-3: Intentional divergence test — modify one instruction in DUT, verify difftest catches it and reports correct location.

[**Failure / Robustness Validation**]

- V-F-1: QEMU not in PATH → graceful error message, no panic.
- V-F-2: QEMU crashes mid-run → detect via GDB read failure, report and halt.
- V-F-3: GDB packet corruption → checksum mismatch triggers retry or error.

[**Edge Case Validation**]

- V-E-1: First instruction divergence — catches immediately at reset vector.
- V-E-2: MMIO instruction followed by non-MMIO — sync then compare resumes correctly.
- V-E-3: Compressed instruction (2-byte) vs standard (4-byte) — PC advances correctly in both DUT and REF.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (per-inst comparison) | V-IT-1, V-IT-3, V-UT-3, V-UT-4 |
| G-2 (GDB protocol) | V-UT-1, V-UT-2, V-F-3 |
| G-3 (feature-gated) | Build without feature: zero difftest code compiled |
| G-4 (MMIO skip) | V-IT-2, V-E-2 |
| G-5 (build integration) | V-IT-1 via `DIFFTEST=1 make run` |
| C-1 (QEMU required) | V-F-1 |
| C-5 (virt machine) | V-IT-1, V-IT-2 |
