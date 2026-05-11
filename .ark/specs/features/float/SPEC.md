[**Goals**]

- G-1: Implement RV32F / RV64F (single-precision) — 26 ops — via `softfloat_pure` (pure-Rust Berkeley softfloat-3).
- G-2: Implement RV32D / RV64D (double-precision) — 26 ops + C.FLD / C.FSD / C.FLDSP / C.FSDSP compressed loads/stores.
- G-3: Track FP-extension state via `mstatus.FS` (Off / Initial / Clean / Dirty) and recompute `mstatus.SD` per write.
- G-4: Surface FP exception flags (NV / DZ / OF / UF / NX) and per-instruction rounding mode (`rm`) through `fcsr` / `fflags` / `frm` aliases.
- G-5: NaN-box single-precision values in the 64-bit f-register file so RV64F and RV64D share storage.

[**Non-goals**]

- NG-1: No Q (quad-precision) or Zfh (half-precision) support — out of scope.
- NG-2: No hardware-style timing for FP — operations are functionally accurate, not cycle-modeled.
- NG-3: No FP traps on signalling NaN — softfloat-3 quiets sNaN per RISC-V spec; no `Invalid` trap is raised.

[**Architecture**]

```
xemu/xcore/src/arch/riscv/cpu/
├── inst/float.rs       impl RVCore { fp_read32, fp_write32, fp_read64, fp_write64, rm() }
│                       + per-op handlers (fadd.s / fmul.s / fmadd.d / fcvt.d.s / ...)
├── csr/mstatus.rs      bitflags! MStatus { FS_OFF, FS_INITIAL, FS_CLEAN, FS_DIRTY, SD }
└── csr.rs              csr_table! { fcsr, fflags, frm } with view_mask + view_shift aliasing

[deps] softfloat-pure = { git = "https://github.com/HarryR/softfloat-pure" }
```

`fflags` is `fcsr[4:0]` view; `frm` is `fcsr[7:5] >> 5` view (descriptor has `view_shift = 5`).

[**Data Structure**]

```rust
// f-register file (32 NaN-boxed 64-bit slots).
struct FRegFile { regs: [u64; 32] }

// Softfloat per-op result carries computed flags.
struct FpResult<T> { value: T, flags: u8 }
```

[**API Surface**]

```rust
impl RVCore {
    // Read/write the FP register file with NaN-boxing semantics.
    pub(in crate::arch::riscv) fn fp_read32 (&self, r: RVReg) -> u32;
    pub(in crate::arch::riscv) fn fp_write32(&mut self, r: RVReg, val: u32);
    pub(in crate::arch::riscv) fn fp_read64 (&self, r: RVReg) -> u64;
    pub(in crate::arch::riscv) fn fp_write64(&mut self, r: RVReg, val: u64);

    // Effective rounding mode for the current instruction.
    pub(in crate::arch::riscv) fn rm(&self, static_rm: u8) -> u8;

    // Apply softfloat exception flags into `fcsr.fflags`.
    pub(in crate::arch::riscv) fn fp_apply_flags(&mut self, flags: u8);
}
```

[**Constraints**]

- C-1: All FP arithmetic uses `softfloat_pure`; no host `f32` / `f64` ops on guest values — `xemu/xcore/src/arch/riscv/cpu/inst/float.rs:8`.
- C-2: Single-precision values stored in `f`-registers are NaN-boxed (upper 32 bits = `0xFFFFFFFF`); `fp_read32` validates the box — `xemu/xcore/src/arch/riscv/cpu/inst/float.rs`.
- C-3: Per-instruction `rm` field selects the rounding mode; `rm = 0b111 (DYN)` falls back to `frm` — `xemu/xcore/src/arch/riscv/cpu/inst/float.rs:130`.
- C-4: FP instruction execution sets `mstatus.FS = Dirty`; trying to execute FP with `FS = Off` raises `Exception::IllegalInstruction` — enforced by `AccessRule::RequireFP` on `fcsr` / `fflags` / `frm` — `xemu/xcore/src/arch/riscv/cpu/csr.rs:23`.
- C-5: `fcsr`, `fflags`, `frm` are descriptor aliases (`view_shift`, `view_mask`) into the same storage — `xemu/xcore/src/arch/riscv/cpu/csr.rs:37`.
- C-6: `mstatus.SD` is recomputed on every `mstatus.FS` or `mstatus.XS` change — `xemu/xcore/src/arch/riscv/cpu/csr/ops.rs:47`.
- C-7: DTS `riscv,isa = "rv64imafdcsu_sstc"` declares F/D presence; `misa` reports the same — `resource/xemu.dts`.

[**CHANGELOG**]

- `2026-05-11` `port-to-ark`: rebuilt from current code under `xemu/xcore/src/arch/riscv/cpu/inst/float.rs` + `csr/mstatus.rs`. Pre-port running notes preserved at `.ark/tasks/archive/legacy/float/`.
