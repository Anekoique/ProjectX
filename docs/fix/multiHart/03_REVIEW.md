# `multiHart` REVIEW `03`

> Status: Closed
> Feature: `multiHart`
> Iteration: `03`
> Owner: Reviewer
> Target Plan: `03_PLAN.md`
> Review Scope:
>
> - Plan Correctness
> - Spec Alignment
> - Design Soundness
> - Validation Adequacy
> - Trade-off Advice

---

## Verdict

- Decision: Approved
- Blocking Issues: 0
- Non-Blocking Issues: 1



## Summary

Round 03 absorbs every residual finding from `02_REVIEW.md` cleanly
and is implementation-ready. R-020 (MEDIUM, paddr-threading) is
resolved by adopting TR-8 option (b): the `last_store` record moves
*inside* `Hart::checked_write` itself, gated on
`matches!(op, MemOp::Store | MemOp::Amo)`, eliminating both the
signature-widening churn and the caller-discipline burden. Ground
truth at `mm.rs:271-276` confirms the placement is sound: `pa` is
already bound at line 272 from `access_bus`; `size` and `op` are
function parameters; the function has a single `Ok(())` exit. Caller
audit (`grep -rn 'checked_write' xemu/xcore/src/`) returns exactly
two callsites — `Hart::store` (line 308, `MemOp::Store`) and
`Hart::amo_store` (line 325, `MemOp::Amo`) — with no debug, MMIO
probe, or non-store path invoking `checked_write`. The `op` gate is
correctly defensive against future Fetch / Load callers without
over-triggering today. The I-8 invariant becomes a post-condition of
`checked_write`'s contract, which matches the TR-6 framing
("coverage is a property of memory semantics, not opcode taxonomy")
one level deeper.

R-022 is resolved with a verified count: `grep -c '#\[test\]'
xemu/xcore/src/arch/riscv/device/intc/plic.rs` returns **14**, and
the plan now reads "14 existing PLIC tests" at every site (G-8 line
108, step 14 line 449, V-IT-6 line 542 / 617, Acceptance Mapping
G-8 row line 680). Residual occurrences of the bare token "13" in
the plan all refer to legitimate distinct quantities (13 new PR1
`#[test]` functions per V-UT-1..V-IT-3 minus V-UT-8; step 13
deletes hard-coded `mhartid = 0`; V-UT-13 names the
`amo_invalidates_peer_reservation` case). Gate-matrix arithmetic is
internally consistent: PR1 354 + 13 = 367 lib + 1 + 6 = 374; PR2a
367 + 1 = 368 lib + 1 + 6 = 375; PR2b 368 + 3 = 371 lib + 1 + 6 =
378.

R-023 is resolved with a `match` block (lines 472-476) that mirrors
the `X_DISK` shape at `xdb/src/main.rs:45-53` exactly — same `match
env("…")` outer form, same `Some(s) => …` / `None => default` arms,
same `anyhow::Error` propagation style. R-021 is correctly subsumed
by R-023 (the closure-with-inner-`?` shape is gone). R-024 unifies
both the C-7 prose at line 361 ("`≤ 700 lines`") and the Acceptance
Mapping row at line 689 ("`≤ 700-line budget`") on the 700-line
target.

`wc -l 03_PLAN.md` returns 700 — the plan meets C-7 exactly at the
boundary. This is technically compliant but leaves no headroom for
inline edits during implementation. Flagged as the sole non-blocking
LOW (R-025).

Seam stability is preserved: I-7 holds, no `arch_isolation` edits,
`HartId` is `pub struct HartId(pub u32)` at `arch/riscv/cpu/hart.rs`
with `pub(in crate::arch::riscv)` boundary discipline (I-7 is
explicit that `HartId` never re-exports across `arch::riscv::`).
Response Matrix covers R-020/R-021/R-022/R-023/R-024/TR-8 plus the
prior-rounds collapse and all six inherited MASTER directives
(00-M-001/002, 01-M-001..004) with section pointers. No silent
drift from approved architecture.

The plan reaches the implementation-ready bar with zero CRITICAL,
zero HIGH, zero MEDIUM, and one LOW (line-budget headroom). Approve
and ship to PR1.



---

## Findings

### R-025 Plan body sits exactly at the C-7 700-line ceiling

- Severity: LOW
- Section: Spec / Constraints / C-7
- Type: Maintainability
- Problem:
  `wc -l /Users/anekoique/ProjectX/docs/fix/multiHart/03_PLAN.md`
  returns 700, matching the C-7 budget exactly. Any inline
  clarification, errata, or response-matrix expansion during
  implementation will push the plan over budget without a fresh
  review-round trim. The round-02 budget rebaseline (420 → 700)
  was justified by Response Matrix + test table + I-8 exposition;
  round 03 has consumed the entire margin.
- Why it matters:
  Audit-trail hygiene. If implementation discovers a small spec
  refinement that needs to land in the plan (e.g., a one-line
  clarification of `RESERVATION_GRANULE` at the LR.D / LR.W
  boundary), the executor faces a binary choice between violating
  C-7 silently or opening a round 04. Neither is desirable for
  a "final" plan.
- Recommendation:
  Two acceptable resolutions, executor's choice:
  (a) Do nothing — accept that any plan edits during
  implementation require either a small trim elsewhere in the
  plan body or a relaxation of C-7 to "≤ 750 lines" with a
  one-line rationale appended to the Log.
  (b) Trim ~20 lines of duplicative prose now (e.g., collapse
  the Acceptance Mapping rows for I-1..I-9 that point to the
  same V-UT-* tests already enumerated under G-1..G-10) to
  restore a small editorial buffer.
  Either is non-blocking. Option (a) is cheaper; flag in the
  PR1 description if the limit is grazed.



---

## Trade-off Advice

(No new trade-offs raised in round 03 require reviewer guidance.
TR-8 from round 02 is adopted via T-9; the plan's choice of option
(b) is correct under the current goals — single assignment site,
no signature change, and `pa` already in scope at the chosen
insertion point. T-1..T-8 are unchanged from round 02 and remain
sound.)



---

## Positive Notes

- **R-020 resolution is exactly right.** The plan's adoption of
  TR-8 option (b) is the cleanest of the three options enumerated
  in `02_REVIEW.md`. Verified against ground truth: `mm.rs:271-276`
  shows `checked_write` already binds `pa` from `access_bus` (line
  272), has `size` and `op` as parameters, and exits with `Ok(())`
  — the one-line insertion before `Ok(())` lands cleanly. Caller
  audit is decisive: `grep -rn 'checked_write' xemu/xcore/src/`
  returns three lines (the definition at `mm.rs:271`, plus
  `mm.rs:308` in `Hart::store` and `mm.rs:325` in `Hart::amo_store`).
  No debug write, no MMIO probe, no fetch path calls
  `checked_write` — so the callee-record cannot over-trigger and
  invalidate peer reservations spuriously. The `op` gate
  (`matches!(op, MemOp::Store | MemOp::Amo)`) is correctly
  defensive: it costs one `matches!` per store today, but guards
  the invariant if a future refactor routes Fetch / Load through
  the same primitive.
- **R-022 resolution is verified end-to-end.** `grep -c '#\[test\]'
  xemu/xcore/src/arch/riscv/device/intc/plic.rs` returns 14, and
  the plan now reads "14" at every cited site (G-8 line 108, step
  14 line 449, V-IT-6 line 542, Acceptance Mapping G-8 row line
  680, Phase-2a gate matrix line 542). The five remaining bare
  "13" tokens in the plan body all denote legitimately distinct
  quantities (13 new PR1 `#[test]` functions; step 13 / V-UT-13
  identifiers; "354 baseline, not in the 13 new" at line 609) —
  none refer to the PLIC test count. Gate-matrix arithmetic is
  internally consistent under the corrected count.
- **R-023 mirrors the existing idiom.** Step 18's `X_HARTS` parse
  at lines 472-476 reproduces the `X_DISK` `match` shape at
  `main.rs:45-53` exactly (same outer `match env("…")`, same
  `Some(s) => …` / `None => default` arms, same `anyhow!()` error
  propagation style). The closure-with-inner-`?` shape that
  R-021 flagged is fully gone. R-021 is fairly subsumed.
- **R-024 unifies the C-7 narrative.** Both the C-7 prose at line
  361 ("`Plan body ≤ 700 lines`") and the Acceptance Mapping row
  at line 689 ("`≤ 700-line budget`") now agree. The Summary at
  line 16-29 also references the 700-line target consistently.
  No more "≤ 500 lines" stragglers.
- **Response Matrix is complete and audit-ready.** Rows for R-020,
  R-021, R-022, R-023, R-024, and TR-8 each cite the plan section
  where the resolution lands. Prior-rounds collapse ("R-001..R-019
  + TR-1..TR-7 resolved in `02_PLAN.md` Response Matrix") is
  correct given round-02 closed at Ready=Yes. All six inherited
  MASTER directives appear with applied-status notes.
- **Seam stability holds across all three PRs.** I-7 at lines
  226-228 confirms no new `SEAM_FILES` / `SEAM_ALLOWED_SYMBOLS`
  entries, no `BUS_DEBUG_STRING_PINS` count change, and `Hart` /
  `HartId` never re-exported across the `arch::riscv::` boundary.
  PR2a's PLIC reshape and PR2b's `xemu-2hart.dts` addition do not
  perturb the seam audit surface.
- **HartId placement is decided.** The plan locates `HartId` at
  `arch/riscv/cpu/hart.rs` as `pub struct HartId(pub u32)` with
  `pub(in crate::arch::riscv)` visibility on `Hart` itself,
  resolving the round-01 ambiguity. No top-level `crate::cpu::HartId`
  re-export, consistent with C-3 and inherited MASTER 00-M-001.
- **Gate matrix is concrete per PR.** Each of PR1 / PR2a / PR2b
  enumerates `cargo fmt --check`, `make clippy`, `cargo test
  --workspace`, `cargo test --test arch_isolation`, `make linux`,
  `make debian`, and the difftest corpus. Difftest is correctly
  pinned to `num_harts = 1` per NG-3, and the regression-block
  for PR2a's V-IT-6 is unambiguous ("regression-block, not a new
  `#[test]`").
- **Performance impact of the I-8 hook is bounded.** One
  `matches!` arm dispatch and one `Option<(PhysAddr, usize)>`
  write per store is well within the noise floor of the existing
  `bus.write` + MMU walk on the same path. Plan does not call
  this out explicitly, but the cost is sub-cycle in any
  reasonable measurement. Not worth flagging.



---

## Approval Conditions

### Must Fix
- (none — no unresolved CRITICAL or HIGH or MEDIUM)

### Should Improve
- R-025 (plan body sits at the C-7 700-line ceiling exactly;
  optional editorial trim or constraint relaxation if any inline
  edit is needed during PR1 implementation)

### Trade-off Responses Required
- (none — TR-8 adopted via T-9; no new trade-offs raised in round 03)

### Ready for Implementation
- Yes
- Reason: Round 03 absorbs all four residuals from `02_REVIEW.md`
  (R-020 MEDIUM + R-021/R-022/R-023/R-024 LOW) without introducing
  new findings. R-020's callee-record placement is verified against
  ground truth at `mm.rs:271-276`: `pa` is in scope, only two
  callers (`Hart::store`, `Hart::amo_store`) exist, and the `op`
  gate is correctly defensive. R-022's "14" count is grep-verified
  on `plic.rs`. R-023's `match` block mirrors `main.rs:45-53`
  exactly. R-024 unifies C-7 on 700 lines. R-021 is fairly subsumed
  by R-023. Test arithmetic (374 / 375 / 378) is internally
  consistent across all three PR gate matrices. Seam stability
  (I-7), HartId placement (arch-internal), inherited MASTER
  directives (00-M-001/002, 01-M-001..004), and the Response Matrix
  audit trail all hold. The sole non-blocking R-025 (plan exactly
  at C-7 ceiling) is editorial. Ship to PR1.
