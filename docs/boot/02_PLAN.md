# `OS Boot` PLAN `02`

> Status: Approved for Implementation
> Feature: `boot`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `01_MASTER.md`

---

## Summary

Final plan for Phase 7a: OpenSBI console bring-up. All blocking findings from rounds 00 and 01 are resolved. This round fixes three specific gaps: (1) correct RV64 address materialization in the boot ROM, (2) a rebuild-based reset lifecycle for boot mode, and (3) a self-consistent DTS + build contract.

## Log

[**Feature Introduce**]

- Boot ROM trampoline uses `li` pseudo-instruction expansion (lui + addiw + slli + addi) for safe 64-bit address materialization — no sign-extension bugs
- Reset lifecycle: `BootConfig` persisted; `CPU::reset()` reapplies it, rebuilding bus from scratch
- DTS uses phandle labels consistently; `resource/Makefile` is OpenSBI-only for this round (no kernel arg)
- `misa` literal corrected to include C bit: `0x8000_0000_0014_1105`

[**Review Adjustments**]

- R-001: Trampoline uses shift-based constant materialization; validation executes instructions and checks register values
- R-002: `BootConfig` stored in `CPU`; `reset()` calls `boot()` to rebuild machine state from config
- R-003: DTS label `plic:` added; round-02 boot flow is firmware-only (no kernel arg); Makefile boundary consistent
- R-004: `misa` literal corrected: bit 2 (C) included → `0x8000_0000_0014_1105`

[**Master Compliance**]

- M-001: All reviewer findings addressed
- M-002: Additive changes only; existing framework untouched; all new code clean and minimal

### Changes from Previous Round

[**Added**]

- Shift-based address materialization for boot ROM (handles high 32-bit physical addresses correctly on RV64)
- `CPU` stores `BootConfig`; `reset()` reapplies boot configuration
- `RVCore` factory method `RVCore::build(config)` that constructs bus + devices + optional boot ROM in one pass

[**Changed**]

- `misa` literal: `0x8000_0000_0014_1101` → `0x8000_0000_0014_1105` (added bit 2 for C extension)
- DTS: added `plic:` label on PLIC node so `interrupt-parent = <&plic>` resolves
- `resource/Makefile` boot target is firmware-only for round 02 (kernel loading is future work)
- xemu Makefile: `FW`/`FDT` are pass-through only (no `KERNEL` in this round)

[**Removed**]

- `KERNEL` env var from round-02 boot flow (OpenSBI-only, no kernel loaded)
- Pseudocode trampoline from round 01 (replaced with concrete instruction encoding)

[**Unresolved**]

- Kernel loading (xv6/Linux) — deferred to round 03+

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review 01 | R-001 | Accepted | Shift-based address materialization; test executes trampoline and checks final register values |
| Review 01 | R-002 | Accepted | `BootConfig` persisted in CPU; `reset()` reapplies via `boot()`; bus rebuilt from scratch |
| Review 01 | R-003 | Accepted | DTS label fixed; boot flow is firmware-only; Makefile boundary consistent |
| Review 01 | R-004 | Accepted | `misa` literal corrected to `0x8000_0000_0014_1105` |
| Review 01 | TR-1 | Accepted | Rebuild model chosen over MMIO mutation; `RVCore::build()` constructs fresh bus per boot |
| Master 01 | M-001 | Applied | All findings addressed |
| Master 01 | M-002 | Applied | Additive, non-breaking changes |

---

## Spec

[**Goals**]

- G-1: Boot OpenSBI `fw_jump.bin` in M-mode, see banner on UART
- G-2: Reproducible build scripts in `resource/` (fetch OpenSBI source, cross-compile, compile DTS)
- G-3: `misa` CSR reports correct IMACSU extensions
- G-4: Legacy direct-load mode works exactly as before

- NG-1: xv6 or Linux boot (future rounds)
- NG-2: Multi-hart / SMP
- NG-3: VGA, disk, network

[**Architecture**]

```
BootConfig::Direct (legacy):
  RVCore::build(Direct) → Bus(RAM + ACLINT + PLIC + UART) → PC = 0x8000_0000

BootConfig::Firmware:
  RVCore::build(Firmware) → Bus(RAM + ACLINT + PLIC + UART + BootRom@0x1000)
                          → load fw_jump.bin @ 0x8000_0000
                          → load xemu.dtb @ FDT_ADDR
                          → PC = 0x1000

Boot ROM trampoline (8 instructions, 32 bytes):
  lui   a0, 0                          # a0 = 0 (hartid)
  lui   a1, 0x87F00                    # a1 = 0xFFFF_FFFF_87F0_0000 (sign-ext)
  slli  a1, a1, 1                      # a1 <<= 1 → need different approach
  ... (actual: use lui + srli zero-extension pattern)

Correct materialization for 0x87F0_0000:
  lui   a1, 0x10FE0     # a1 = 0x0000_0000_10FE_0000 (0x87F0_0000 >> 3 = 0x10FE_0000)
  slli  a1, a1, 3       # a1 = 0x0000_0000_87F0_0000 ✓

Correct materialization for 0x8000_0000:
  lui   t0, 0x10000     # t0 = 0x0000_0000_1000_0000 (0x8000_0000 >> 3 = 0x1000_0000)
  slli  t0, t0, 3       # t0 = 0x0000_0000_8000_0000 ✓

Full trampoline (7 instructions = 28 bytes):
  lui   a0, 0           # a0 = 0 (hartid)
  lui   a1, FDT>>15     # upper bits of FDT_ADDR / 8
  slli  a1, a1, 3       # a1 = FDT_ADDR
  lui   a2, 0           # a2 = 0 (no fw_dynamic_info)
  lui   t0, ENTRY>>15   # upper bits of ENTRY / 8
  slli  t0, t0, 3       # t0 = ENTRY (0x8000_0000)
  jalr  zero, t0, 0     # jump to OpenSBI

Memory layout:
  0x0000_1000  Boot ROM (32 bytes, read-only)
  0x0200_0000  ACLINT (64 KB)
  0x0C00_0000  PLIC (64 MB)
  0x1000_0000  UART0 (256 B)
  0x8000_0000  DRAM (128 MB)
    0x8000_0000  OpenSBI fw_jump.bin
    0x8020_0000  (reserved for future kernel)
    0x87F0_0000  FDT blob
```

[**Invariants**]

- I-1: `BootConfig::Direct` produces exact same bus/PC/behavior as current code
- I-2: `BootConfig::Firmware` adds boot ROM, loads fw + FDT, sets PC = 0x1000
- I-3: `CPU::reset()` reapplies the stored `BootConfig` — mode is persistent across resets
- I-4: Boot ROM is read-only; writes return `BadAddress`
- I-5: `misa` = `0x8000_0000_0014_1105` (MXL=2, IMACSU)
- I-6: FDT address 8-byte aligned, within DRAM
- I-7: Trampoline correctly materializes all 32-bit physical addresses on RV64 without sign-extension

[**Data Structure**]

```rust
/// Boot configuration.
#[derive(Clone)]
pub enum BootConfig {
    /// Legacy: one binary at DRAM base.
    Direct { file: Option<String> },
    /// Firmware: OpenSBI + FDT, optional kernel.
    Firmware { fw: String, fdt: String },
}

/// Read-only boot ROM.
pub struct BootRom { data: Vec<u8> }
```

[**API Surface**]

```rust
impl BootRom {
    pub fn new(fdt_addr: u32, entry: u32) -> Self;
}
impl Device for BootRom { /* read-only */ }

impl CPU<Core> {
    pub fn boot(&mut self, config: BootConfig) -> XResult;
}
```

[**Constraints**]

- C-1: No new crate dependencies
- C-2: Boot ROM ≤ 32 bytes (7 RV64I instructions)
- C-3: `misa` = `0x8000_0000_0014_1105`
- C-4: `resource/Makefile` owns external builds; xemu Makefile delegates only
- C-5: `.gitignore` for generated binaries under `resource/`
- C-6: Existing tests pass unchanged

---

## Implement

### Implementation Plan

[**Step 1: `misa` initialization**]

`CsrFile::new()` sets `regs[0x301] = MISA_VALUE` where `MISA_VALUE` encodes IMACSU + MXL=2.

[**Step 2: Boot ROM device**]

New `xcore/src/device/boot_rom.rs`. `BootRom::new(fdt_addr, entry)` emits 7 RV32I instructions using `lui` + `slli` for zero-extension-safe address construction. `Device::read()` returns instruction bytes; `write()` returns `BadAddress`.

Address materialization pattern for value V where V fits in 32 bits and bit 31 is set:
```
lui  reg, (V >> 15) & 0xFFFFF   # load (V/8) into upper 20 bits (no sign-ext issue since bit 31 of V/8 is 0)
slli reg, reg, 3                 # shift left 3 to recover V
```
This works because V >> 3 clears bit 31, so `lui` sign-extension is harmless.

For V=0: `lui reg, 0`.

[**Step 3: `BootConfig` + `CPU::boot()`**]

- Add `BootConfig` enum in `xcore/src/cpu/mod.rs`
- Store `boot_config: BootConfig` field in `CPU`
- `CPU::boot(config)`: stores config, calls `reset()` internally
- `CPU::reset()`: rebuilds core via `Core::new()` or `Core::with_boot_rom(fdt_addr)`, then loads images per config
- Legacy `CPU::load()` wraps as `BootConfig::Direct`

[**Step 4: xdb integration**]

- `xdb/src/main.rs`: if `X_FW` env var set, construct `BootConfig::Firmware`; else `BootConfig::Direct`
- `xdb_repl` reset command uses stored boot config

[**Step 5: xemu Makefile**]

- Add optional `FW`/`FDT` vars, pass as `X_FW`/`X_FDT` to cargo
- Add `boot` phony target: `$(MAKE) -C ../resource boot`

[**Step 6: resource/ directory**]

- `resource/xemu.dts` — compilable DTS with correct labels
- `resource/Makefile` — `fetch-opensbi`, `build-opensbi`, `dtb`, `boot` targets
- `resource/.gitignore` — `*.bin`, `*.dtb`, `opensbi/`

---

## Validation

[**Unit Tests**]

- V-UT-1: `BootRom::new()` read returns correct bytes; write returns `BadAddress`
- V-UT-2: `misa` = `0x8000_0000_0014_1105` after `CsrFile::new()`
- V-UT-3: Trampoline register-value test: execute 7 instructions on a fresh core, verify `a0=0`, `a1=FDT_ADDR`, `t0=ENTRY`

[**Integration Tests**]

- V-IT-1: OpenSBI banner on UART (manual `make boot`)
- V-IT-2: `make test` passes
- V-IT-3: am-tests pass
- V-IT-4: cpu-tests-rs pass

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-1 |
| G-2 | resource/Makefile builds from source |
| G-3 | V-UT-2 |
| G-4 | V-IT-2, V-IT-3, V-IT-4 |
| C-3 | V-UT-2 |
| C-6 | V-IT-2, V-IT-3, V-IT-4 |
