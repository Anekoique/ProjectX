# F/D Floating-Point Extension IMPL 04

> Status: Ready for Review
> Feature: `float`
> Iteration: `04`
> Owner: Executor
> Based on: `03_PLAN.md` + `04_PLAN.md` (final)

---

## Summary

Implemented RISC-V F and D floating-point extensions: 52 scalar instructions (26 F + 26 D), 8 FMA instructions, 4 compressed D load/store, FP CSR composition, mstatus.FS tracking, and bare-metal AM-test validation.

## Scope

**Completed:**
- `fpr: [u64; 32]` in RVCore with NaN-boxing
- `DecodedInst::FR` (rd, rs1, rs2, rm) and `FR4` (rd, rs1, rs2, rs3, rm)
- `InstFormat::FR`/`FR4` in decoder with proper extraction
- 70 instruction patterns in `riscv.instpat` (52 scalar + 8 FMA + 4 compressed + 6 load/store)
- `softfloat_pure` (pure Rust Berkeley softfloat-3 port) as IEEE 754 backend
- `fcsr`/`fflags`/`frm` CSRs with specialized `fp_csr_read`/`fp_csr_write`
- `dirty_fp()` on all FP state mutations including CSR writes
- `misa` F(5) + D(3) bits, `mstatus.FS` initialized to Initial
- Debug ISA string updated to `rv64imafdc`
- Debugger FP CSR read routed through `fp_csr_read`
- `format_mnemonic` for FR/FR4 disassembly
- AM-test: F arithmetic, D arithmetic, FCVT, FCLASS, FS dirtiness
- Comment support added to pest grammar

**Deviations from Plan:**
- D-001: Used `softfloat_pure` (pure Rust, git dep) instead of `softfloat-wrapper` (C FFI). Reason: `softfloat-sys` does not build on aarch64-apple-darwin. API difference: operations return `(result, flags_u8)` tuples instead of global flags — cleaner design.
- D-002: `fpr` indexed with `reg as usize` instead of `Index<RVReg>` impl for `[u64]`. Reason: on RV64 where `Word = u64`, the impl conflicts with existing `Index<RVReg> for [Word]`.

## Verification Results

- `cargo fmt --check`: clean
- `cargo clippy`: clean (no warnings)
- `cargo test -p xcore`: 272 passed, 0 failed
- AM-tests: 8/8 PASS (uart-putc, timer-read, timer-irq, soft-irq, plic-access, csr-warl, trap-ecall, float)

## Files Changed

| File | Change |
|------|--------|
| `xcore/Cargo.toml` | Added `softfloat_pure` dependency |
| `xcore/src/cpu/riscv/mod.rs` | Added `fpr`, `require_fp`, `dirty_fp`, `MStatus` import |
| `xcore/src/cpu/riscv/inst.rs` | Added `mod float` |
| `xcore/src/cpu/riscv/inst/float.rs` | NEW: All F/D instruction handlers |
| `xcore/src/cpu/riscv/inst/compressed.rs` | Added C.FLD/C.FSD/C.FLDSP/C.FSDSP handlers |
| `xcore/src/cpu/riscv/csr.rs` | Added fcsr/fflags/frm CSRs, misa F+D, FS=Initial init |
| `xcore/src/cpu/riscv/csr/ops.rs` | Added `fp_csr_read`/`fp_csr_write` with `dirty_fp` |
| `xcore/src/cpu/riscv/debug.rs` | ISA string, FP CSR debug read, FR/FR4 disasm |
| `xcore/src/isa/riscv/decoder.rs` | Added FR/FR4 DecodedInst variants + extraction |
| `xcore/src/isa/riscv/inst.rs` | Added FR/FR4 InstFormat + FromStr |
| `xcore/src/isa/instpat/riscv.instpat` | Added 70 FP instruction patterns |
| `xcore/src/isa/instpat/riscv.pest` | Added COMMENT rule for `//` lines |
| `xcore/src/utils/macros.rs` | Added FR/FR4 rows to `rv_inst_table!` |
| `am-tests/src/tests/float.c` | NEW: Bare-metal FP validation test |
| `am-tests/include/amtest.h` | Added `test_float` declaration |
| `am-tests/src/main.c` | Added float test dispatch |
| `am-tests/Makefile` | Added `f` to ALL and name mapping |
