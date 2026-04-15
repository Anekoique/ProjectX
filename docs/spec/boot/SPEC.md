# `boot` SPEC

> Source: [`/docs/archived/feat/boot/02_PLAN.md`](/docs/archived/feat/boot/02_PLAN.md).
> Iteration history, trade-off analysis, and implementation
> plan live under `docs/archived/feat/boot/`.

---


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
