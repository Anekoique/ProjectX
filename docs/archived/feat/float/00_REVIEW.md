# `F/D Floating-Point Extension` REVIEW `00`

> Status: Open
> Feature: `float`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Rejected
- Blocking Issues: `2`
- Non-Blocking Issues: `2`

## Summary

The overall direction is sound. Keeping FP architectural state in a dedicated `fpr` array, modeling NaN-boxing explicitly, and localizing execution in `inst/float.rs` all fit the current framework well. The plan is also anchored to the right spec surfaces: the official RISC-V [F](https://docs.riscv.org/reference/isa/unpriv/f-st-ext.html), [D](https://docs.riscv.org/reference/isa/unpriv/d-st-ext.html), [C](https://docs.riscv.org/reference/isa/unpriv/c-st-ext.html), and privileged [machine-level](https://docs.riscv.org/reference/isa/priv/machine.html) chapters.

This round is still not ready for implementation. Two blocking gaps remain:

- the plan incorrectly defers compressed double-precision load/store instructions even though `C`+`D` implies they must exist on RV64, which makes the Linux/buildroot goal materially unsafe;
- the planned `FCVT` invalid-input semantics are wrong against the official manual, so the document currently approves incorrect architectural behavior.

In addition, the validation story still overstates how easily difftest can be extended in this codebase, and a few NaN-sensitive validation cases are not yet precise enough for a “strict spec compliance” round.

---

## Findings

### R-001 `Compressed D instructions are incorrectly deferred`

- Severity: HIGH
- Section: `Constraints / Implementation Plan / Validation`
- Type: Spec Alignment
- Problem:
  `C-5` states that compressed FP loads/stores can be deferred because on RV64 the relevant opcode slots are already used by integer compressed instructions. That is only true for the single-precision `C.FLW*` family. The official C chapter explicitly states that if `C` is implemented, the relevant compressed floating-point load/store instructions must be provided whenever the corresponding floating-point extension is also implemented, and RV64D includes `C.FLD`, `C.FSD`, `C.FLDSP`, and `C.FSDSP`.
- Why it matters:
  The plan’s own goal is standard Linux userspace with buildroot/busybox on a `C`+`D` machine. Toolchains targeting `rv64gc`/`lp64d` can legally emit compressed double save/restore and stack traffic. Leaving these instructions out creates an ISA hole that can surface as an illegal instruction during boot or userspace execution.
- Recommendation:
  The next PLAN must bring `C.FLD`, `C.FSD`, `C.FLDSP`, and `C.FSDSP` into scope, including decode, execution, disassembly, and validation. If the executor wants to defer them, then the advertised ISA and Linux boot target must be narrowed accordingly, which would conflict with `G-7`.

### R-002 `FCVT invalid-result semantics are specified incorrectly`

- Severity: HIGH
- Section: `Constraints / Validation`
- Type: Correctness
- Problem:
  `C-7` says that float-to-int overflow or NaN should return “0 for unsigned NaN” and “max-negative for signed NaN”. That is not what the official manual defines. The F chapter’s conversion table specifies that `+∞` or `NaN` return the maximum positive signed value or maximum unsigned value, while negative overflow and `-∞` return the minimum signed value or zero. The D chapter explicitly says `FCVT.int.D` uses the same invalid-input behavior as `FCVT.int.S`.
- Why it matters:
  This is direct architectural behavior, not a documentation nit. If implemented as written, `FCVT.W[U].{S,D}` and `FCVT.L[U].{S,D}` will disagree with the spec and with reference models on NaN and infinity cases.
- Recommendation:
  Replace `C-7` with the full table-driven behavior from the spec, and extend validation to cover negative overflow, `-∞`, `+∞`, `NaN`, and RV64 sign-extension for `FCVT.W[U].*`.

### R-003 `Reference-model integration is under-scoped for the current framework`

- Severity: MEDIUM
- Section: `Architecture / Implementation Plan / Validation`
- Type: Validation
- Problem:
  Phase 4 reduces difftest work to “add `fpr` to `CoreContext`”. In the current codebase that is not sufficient. [`xemu/xcore/src/cpu/riscv/context.rs`](../../../xemu/xcore/src/cpu/riscv/context.rs) only carries `pc`, `gprs`, `privilege`, and `csrs`; [`xemu/xcore/src/cpu/riscv/debug.rs`](../../../xemu/xcore/src/cpu/riscv/debug.rs) hardcodes the ISA string to `rv64imac`/`rv32imac`; [`xemu/xdb/src/difftest/qemu.rs`](../../../xemu/xdb/src/difftest/qemu.rs) bulk-syncs only GPRs plus PC; and the Spike FFI exposed by [`xemu/tools/difftest/spike/spike_ffi.h`](../../../xemu/tools/difftest/spike/spike_ffi.h) has no FP register getters/setters at all.
- Why it matters:
  The plan presents QEMU/Spike-backed validation as part of the correctness story, but the current backend contract cannot compare or synchronize floating-point architectural state. Without an explicit integration plan, the reviewable validation surface is materially weaker than the document claims.
- Recommendation:
  Expand Phase 4 to cover `isa` string updates, FP register transport in `RVCoreContext`, QEMU FP register mapping, Spike FFI extensions for FP state, and explicit comparison rules for `fpr`, `fcsr`, and `mstatus.FS`. If that work is intentionally deferred, narrow the round’s validation claims instead of implying full difftest parity.

### R-004 `NaN-sensitive validation cases are still too coarse`

- Severity: MEDIUM
- Section: `Validation`
- Type: Spec Alignment
- Problem:
  `V-F-2` currently states `fadd.s(NaN, 1.0) -> canonical NaN + NV flag`. The official F chapter is more specific than that. Arithmetic NaN results are canonical by default, but invalid-flag behavior depends on the instruction and on whether the input is a quiet NaN or signaling NaN. The same chapter also gives special one-NaN behavior for `FMIN/FMAX`, and different quiet/signaling rules for `FEQ` versus `FLT/FLE`. The current validation matrix does not cover those distinctions precisely enough.
- Why it matters:
  The user requirement here is strict conformity to the official manual. If the validation plan blurs qNaN and sNaN behavior, it can either reject a correct implementation or bless a wrong one.
- Recommendation:
  Split NaN validation into qNaN versus sNaN cases, add explicit `FEQ` versus `FLT/FLE` flag checks, and add `FMIN/FMAX` tests for one-NaN, two-NaN, and signed-zero behavior.

---

## Trade-off Advice

### TR-1 `Keep a Berkeley softfloat path, but tighten the backend acceptance criteria`

- Related Plan Item: `T-1`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Need More Justification
- Advice:
  The reviewer agrees with the correctness-first direction, but the next PLAN should not lock `softfloat-wrapper` purely by name. The important property is whether the chosen API exposes per-operation rounding mode control, accrued exception flags, and raw bit-preserving transfers without host-FPU reinterpretation.
- Rationale:
  The hard parts of F/D are not just `fadd` and `fdiv`; they are NaN-boxing, flag accounting, conversion edge cases, and move/sign rules. A convenience wrapper that hides too much behind host `f32`/`f64` conversions can quietly lose spec-required behavior.
- Required Action:
  The next PLAN should state the required backend capabilities explicitly and name the fallback to lower-level bindings if the wrapper API is insufficient.

### TR-2 `Raw u64 storage and a dedicated R4 form are the right structural choices`

- Related Plan Item: `T-2 / T-3`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Prefer Option A
- Advice:
  Keep `[u64; 32]` for FP register storage and add a dedicated `DecodedInst::R4`.
- Rationale:
  Raw integer storage matches NaN-boxing, `FMV.*`, and load/store bit preservation better than `f64`-typed storage, and a dedicated `R4` form is clearer than smuggling `rs3` through the existing `R` format. Both choices align with the current decoder/dispatch style.
- Required Action:
  Keep these trade-offs as-is, but the next PLAN should explicitly list the required updates to `rv_inst_table!`, disassembly formatting, and decoder tests so the impact is fully accounted for.

---

## Positive Notes

- The plan correctly treats NaN-boxing and `mstatus.FS` as first-class design constraints instead of post-implementation cleanup.
- Using raw `u64` FP storage is a strong fit for this emulator because it preserves architectural bit patterns for `FLD/FSD`, `FMV.*`, and narrower-value boxing.
- Keeping floating-point execution in `inst/float.rs` matches the existing instruction-module layout and should keep the resulting codebase clean.

---

## Approval Conditions

### Must Fix
- R-001
- R-002

### Should Improve
- R-003
- R-004

### Trade-off Responses Required
- T-1
- T-2
- T-3

### Ready for Implementation
- No
- Reason: No. The current round still defers required RV64 compressed D instructions and specifies incorrect `FCVT` invalid-input behavior, so it is not yet safe to approve as a spec-conformant F/D plan.
