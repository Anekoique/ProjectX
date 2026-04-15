# `F/D Floating-Point Extension` REVIEW `01`

> Status: Open
> Feature: `float`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
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
- Blocking Issues: `3`
- Non-Blocking Issues: `2`

## Summary

Round `01` is materially better than `00`. It resolves the two prior blocking gaps by bringing compressed RV64D load/store instructions into scope and correcting the `FCVT` invalid-input table against the official RISC-V [F](https://docs.riscv.org/reference/isa/unpriv/f-st-ext.html), [D](https://docs.riscv.org/reference/isa/unpriv/d-st-ext.html), [C](https://docs.riscv.org/reference/isa/unpriv/c-st-ext.html), and privileged [machine-level](https://docs.riscv.org/reference/isa/priv/machine.html) chapters. The plan is also much more concrete about decoder, handler, and backend structure than the previous round.

This round is still not ready for implementation. Three design-stage correctness issues remain:

- the proposed `fcsr` / `fflags` / `frm` composition still does not fit the repo’s current CSR alias model and will not behave as written;
- several flag-producing FP instructions still fail to transition `mstatus.FS` to Dirty when they write floating-point status state;
- the sign-injection examples bypass the mandatory NaN-boxing check for narrower non-transfer operations.

Those are plan-level issues, not implementation nits. If left unresolved, the executor can follow the approved document and still ship architecturally wrong FP state handling.

## Findings

### R-001 `The proposed FP CSR composition is still incompatible with the current CSR framework`

- Severity: HIGH
- Section: `Constraints / Phase 1.3`
- Type: API
- Problem:
  The plan defines `fflags` as an alias view into `fcsr`, keeps `frm` as an independent slot, and then re-composes `fcsr` in CSR side effects using raw-address reads and writes. That does not line up with the current descriptor-driven CSR framework in [`xemu/xcore/src/cpu/riscv/csr.rs`](../../../xemu/xcore/src/cpu/riscv/csr.rs) and [`xemu/xcore/src/cpu/riscv/csr/ops.rs`](../../../xemu/xcore/src/cpu/riscv/csr/ops.rs). In the current implementation, `csr_write()` calls `write_with_desc(desc, val)`, and alias descriptors write directly into the aliased storage slot. That means a write to `fflags` would already land in `fcsr`, while `get_by_addr(0x001)` still reads the separate raw slot `0x001`, not the aliased low bits of `fcsr`. The side-effect sketch in `01_PLAN` therefore reads stale data for `fflags` writes and also writes to raw alias addresses in a way the descriptor read path does not consult.
- Why it matters:
  This is the architectural core of FP status state. If this design is approved as written, `fcsr`, `fflags`, and `frm` can silently desynchronize under legal CSR accesses even before arithmetic handlers are implemented.
- Recommendation:
  The next PLAN must choose one coherent design that matches the existing framework:
  - either extend the descriptor model to support shifted subfield aliases;
  - or add explicit specialized read/write handling for `0x001/0x002/0x003` outside the generic alias path.
  The plan should not mix aliased descriptors with raw-slot side effects for the same logical state.

### R-002 `Flag-writing FP instructions still do not reliably transition FS to Dirty`

- Severity: HIGH
- Section: `Invariants / API Surface / Phase 2`
- Type: Correctness
- Problem:
  The plan correctly states in `I-4` that FP instructions that modify FP state must set `mstatus.FS = Dirty`, and the privileged spec treats `fcsr`/`frm`/`fflags` as part of the floating-point state. However, several concrete handler examples that can write `fflags` through `with_flags()` never call `dirty_fp()`: `feq_s`, `flt_s`, `fcvt_w_s`, and `fcvt_wu_s` are explicit examples. The privileged spec says that executing an instruction that possibly modifies floating-point state transitions `FS` from `Initial/Clean` to `Dirty`, and writes to `fcsr` are part of that state.
- Why it matters:
  This breaks lazy save/restore and `SD` summary tracking exactly in the cases where exception flags are architecturally visible but no `f` register is written. A correct implementation cannot leave `FS` clean after updating `fflags`.
- Recommendation:
  The next PLAN must centralize this rule. The cleanest options are:
  - make `with_flags()` responsible for marking `FS` dirty whenever it writes `fflags`;
  - or require every handler that may update `fcsr` to call `dirty_fp()`.
  The plan should not rely on result-register writes alone to drive FP dirtiness.

### R-003 `The sign-injection examples violate NaN-boxing rules for narrow non-transfer operations`

- Severity: HIGH
- Section: `Invariants / Phase 2`
- Type: Spec Alignment
- Problem:
  The `fsgnj_s`, `fsgnjn_s`, and `fsgnjx_s` examples read operands with `self.fpr[rs1] as u32` and `self.fpr[rs2] as u32`. That bypasses `read_f32()` and therefore bypasses the NaN-boxing check defined in the plan’s own `I-2`. The official D chapter is explicit: apart from transfer operations (`FLn/FSn` and `FMV.*`), narrower operations must check whether operands are valid NaN-boxed values and otherwise treat them as canonical NaNs. The official F chapter also states that sign-injection does not canonicalize NaNs, which applies after the narrower operand has first been interpreted correctly.
- Why it matters:
  This is an architectural correctness bug on invalid-boxed single-precision inputs stored in 64-bit `f` registers. Approving the plan as written would allow wrong results for legal mixed-width register contents.
- Recommendation:
  The next PLAN must route all narrow non-transfer FP operands, including `FSGNJ*`, through a canonicalizing helper such as `read_f32()`, then apply sign-bit manipulation to the canonicalized 32-bit payload. Keep the raw-bit path only for transfer instructions where the spec explicitly allows ignoring upper bits.

### R-004 `Linux boot acceptance is still not concretely validated`

- Severity: MEDIUM
- Section: `Goals / Validation / Acceptance Mapping`
- Type: Validation
- Problem:
  `G-8` is still mapped to `V-IT-7`, but `V-IT-7` only says that existing tests remain unaffected. That is not a Linux boot acceptance artifact. Phase 4 says buildroot/busybox `lp64d` initramfs should replace the minimal init and that full validation includes `make run`, yet the validation section never defines a dedicated Linux boot test or pass criterion.
- Why it matters:
  The user-facing purpose of this feature is Linux userspace boot, not just instruction-level regression survival. The round cannot be considered implementation-ready without an explicit acceptance target for that outcome.
- Recommendation:
  Add a distinct validation item for Linux boot, including:
  - the artifact under test;
  - the expected shell/login milestone;
  - and a small command sequence or shutdown criterion that demonstrates userspace is actually functional.

### R-005 `The new FMIN/FMAX NaN validation still has incorrect qNaN flag expectations`

- Severity: MEDIUM
- Section: `Validation`
- Type: Spec Alignment
- Problem:
  `V-E-2` and `V-E-3` still expect `NV` for `fmin.s(qNaN, 1.0)` and for `fmin.s(qNaN, qNaN)`. The official F chapter says if only one operand is NaN the non-NaN operand is returned, and if both operands are NaNs the result is canonical NaN; signaling NaN inputs set the invalid flag. That means quiet-NaN-only cases must not set `NV`.
- Why it matters:
  This is no longer just an “edge-case expansion” issue. The proposed test oracle would fail a correct implementation or encourage an incorrect one.
- Recommendation:
  Split the `FMIN/FMAX` NaN tests into four distinct cases:
  - one qNaN;
  - one sNaN;
  - two qNaNs;
  - any case containing an sNaN.
  Only the signaling-NaN cases should expect `NV`.

---

## Trade-off Advice

### TR-1 `Prefer explicitly decoded rm/funct3 over hidden last-instruction state`

- Related Plan Item: `Phase 2.3`
- Topic: Clean Design vs Minimal Churn
- Reviewer Position: Prefer explicit decoded field
- Advice:
  The reviewer does not recommend the `last_inst_raw` design as the default path for carrying `rm`/`funct3` into FP handlers.
- Rationale:
  The current decoder/dispatch architecture is data-oriented: decode once, then execute based on explicit fields in `DecodedInst`. Adding a mutable `last_inst_raw` side channel to `RVCore` makes handler correctness depend on execution ordering and adds hidden state that is unrelated to architected machine state. That is avoidable design debt in a codebase that is otherwise quite clean.
- Required Action:
  The next PLAN should either:
  - carry `rm`/`funct3` explicitly in the decoded FP forms;
  - or justify why `last_inst_raw` is safer than extending the decoded representation.

### TR-2 `Keep the arithmetic backend choice separate from the FP-CSR state-model fix`

- Related Plan Item: `T-1 / Phase 1.3`
- Topic: Compatibility vs Clean Design
- Reviewer Position: Keep as is with clarification
- Advice:
  The backend choice now looks much better justified. The remaining problem is not which softfloat wrapper to use, but how FP architectural state is represented and updated.
- Rationale:
  The plan is stronger this round on arithmetic backend capabilities, but it still tries to force shifted FP CSR subfields through a generic alias model that does not support them. Those are separate concerns and should be treated independently.
- Required Action:
  Keep the softfloat direction if desired, but redesign the `fcsr/frm/fflags` interface as a first-class state-model decision rather than an incidental CSR side-effect patch.

---

## Positive Notes

- The round genuinely fixes the two blocking review-00 scope issues: compressed RV64D loads/stores are now in scope and the `FCVT` saturation table is corrected.
- `[u64; 32]` storage and a dedicated `DecodedInst::R4` remain the right structural choices for this codebase.
- The plan is much better than `00_PLAN` at distinguishing qNaN and sNaN behavior for arithmetic and compare instructions, even though `FMIN/FMAX` still needs one more correction.

---

## Approval Conditions

### Must Fix
- R-001
- R-002
- R-003

### Should Improve
- R-004
- R-005

### Trade-off Responses Required
- TR-1
- TR-2

### Ready for Implementation
- No
- Reason: No. The round is stronger than `00_PLAN`, but the proposed FP CSR composition, `FS` dirtiness model, and NaN-boxing behavior for narrow non-transfer operations are still not correct enough to approve implementation.
