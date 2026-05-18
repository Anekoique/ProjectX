[**Goals**]

- G-1: Decode RV32I / RV64I base + M, A, Zicsr, Zifencei, C (compressed), F, D, and Privileged into a single `DecodedInst` enum.
- G-2: Drive decode from a declarative pattern table (`riscv.instpat`) parsed by a pest grammar (`riscv.pest`).
- G-3: Treat ISA width as compile-time via `cfg(isa32)` / `cfg(isa64)`; RV64-only instructions return `InvalidInst` on RV32.
- G-4: Expand compressed (16-bit) instructions to 32-bit form at decode time so execute paths see only one shape per format.

[**Non-goals**]

- NG-1: No vector (V) or hypervisor (H) extensions — out of scope.
- NG-2: No cycle-accurate timing — decode is functional only.
- NG-3: No runtime ISA-extension toggling — `misa` writes are WARL-masked to the build's selection.

[**Architecture**]

```
xemu/xcore/src/isa/
├── mod.rs                cfg-gated re-exports — IMG, DECODER, DecodedInst, InstFormat, InstKind, RVReg
├── instpat/
│   ├── riscv.pest        pest grammar for `pat = name (rd, rs1, rs2) ?> opcode_pattern` lines
│   └── riscv.instpat     declarative instruction table (200+ patterns)
└── riscv/
    ├── decoder.rs        RVDecoder + LazyLock<DECODER> + DecodedInst enum + decode(inst: u32) -> XResult<DecodedInst>
    ├── inst.rs           InstFormat + InstKind (all mnemonics)
    ├── reg.rs            RVReg enum (x0..x31, f0..f31)
    └── mod.rs            sub-module re-exports
```

[**Data Structure**]

```rust
pub enum InstFormat { R, I, S, B, U, J, FR, FR4, C }

pub enum DecodedInst {
    R   { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg },
    FR  { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg, rm: u8 },
    FR4 { kind: InstKind, rd: RVReg, rs1: RVReg, rs2: RVReg, rs3: RVReg, rm: u8 },
    I   { kind: InstKind, rd: RVReg, rs1: RVReg,             imm: SWord },
    S   { kind: InstKind,            rs1: RVReg, rs2: RVReg, imm: SWord },
    B   { kind: InstKind,            rs1: RVReg, rs2: RVReg, imm: SWord },
    U   { kind: InstKind, rd: RVReg,                         imm: SWord },
    J   { kind: InstKind, rd: RVReg,                         imm: SWord },
    C   { kind: InstKind, inst: u32 },     // raw 32-bit form post-expansion
}

pub struct RVDecoder { /* pest-compiled pattern table */ }
pub enum RVReg { x0, ..., x31, f0, ..., f31 }
```

[**API Surface**]

```rust
pub static DECODER: LazyLock<RVDecoder>;

impl RVDecoder {
    pub fn from_instpat(instpat_code: &str) -> XResult<Self>;
    pub fn decode(&self, inst: u32) -> XResult<DecodedInst>;
}

impl InstKind {
    pub fn as_str (&self) -> &'static str;
    pub fn is_load(&self) -> bool;
}

impl Display for DecodedInst { /* GAS-style mnemonic formatting */ }
```

[**Constraints**]

- C-1: ISA width is selected at compile time via `cfg(isa32)` / `cfg(isa64)`; runtime mode-switching is forbidden.
- C-2: RV64-only instructions return `Err(XError::InvalidInst)` on RV32 — `trap_on_err` re-maps to `Exception::IllegalInstruction`.
- C-3: Instruction patterns live in `xemu/xcore/src/isa/instpat/riscv.instpat`; new instructions require BOTH a pattern entry AND an execute arm in `xemu/xcore/src/arch/riscv/cpu/inst/*.rs`.
- C-4: Compressed (16-bit) instructions are expanded to 32-bit form at decode; execute paths see only the eight non-`C` `DecodedInst` variants — `xemu/xcore/src/isa/riscv/decoder.rs`.
- C-5: `DECODER` is a process-wide `LazyLock` populated once from `riscv.instpat` at first use — `xemu/xcore/src/isa/riscv/decoder.rs:23`.
- C-6: `DecodedInst` is `Copy + Clone + PartialEq + Eq` so the icache can store lines by value — `xemu/xcore/src/isa/riscv/decoder.rs:168`.

[**CHANGELOG**]

- `2026-05-11` `port-to-ark`: rebuilt from current code under `xemu/xcore/src/isa/`. Pre-port running-notes preserved at `.ark/tasks/archive/legacy/inst/SPEC_LEGACY.md` (running-notes feature, no prior iteration history).
