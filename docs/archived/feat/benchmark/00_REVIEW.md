# `benchmark-adaptation` REVIEW `00`

> Status: Open
> Feature: `benchmark-adaptation`
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

- Decision: Approved with Revisions
- Blocking Issues: 1
- Non-Blocking Issues: 3

## Summary

`00_PLAN` identifies the right high-level scope: add the minimum missing HAL surface in
`xam`, then adapt `alu-tests`, `coremark`, `dhrystone`, and `microbench` without adding
new xemu features. That overall direction is sound.

The main problem is that the heap design is not implementable in the most natural way
for the existing benchmark code. The current plan models heap bounds as Rust-exported
`usize` statics, but the consumers want pointer-like linker symbols. There are also a
few gaps in how the plan proposes to adapt `coremark` and `microbench`: the `main()`
porting strategy is underspecified, and the microbench port does not yet account for
all of the NJU compatibility surface it currently consumes.

So the round is a good draft, but it is not yet detailed or precise enough to be ready
for implementation.

---

## Findings

### R-001 `heap bounds should be linker symbols, not Rust statics`

- Severity: HIGH
- Section: `Data Structure / API Surface / Phase 1`
- Type: API
- Problem:
  The plan defines `heap_start` and `heap_end` as Rust `usize` statics exported to C.
  But the benchmark code, especially `microbench`, wants pointer-style heap bounds and
  uses them in pointer arithmetic. Modeling heap bounds as integer statics forces the C
  side into awkward casts and does not match the natural bare-metal pattern.
- Why it matters:
  Heap bounds are the foundation for the whole benchmark port. If the symbol shape is
  wrong here, the downstream adaptation becomes noisier and more error-prone than it
  needs to be.
- Recommendation:
  Define heap bounds directly in `xam/xhal/linker.lds.S`, for example:
  `_heap_start = _ekernel; _heap_end = 0x88000000;`, and consume them in C as
  `extern char _heap_start[], _heap_end[];`. Do not introduce a Rust `heap.rs` module
  just to re-export linker information.

### R-002 `coremark main() adaptation is underspecified`

- Severity: MEDIUM
- Section: `Constraints / Phase 3`
- Type: Correctness
- Problem:
  The plan correctly notes that CoreMark does not naturally use xam's
  `main(const char *args)` ABI, but "fix `main()` signature" is not specific enough.
  As written, it implies editing the benchmark entrypoint directly without explaining
  whether the argument-parsing path is being removed, shimmed, or disabled by port
  configuration.
- Why it matters:
  CoreMark is sensitive to how platform ports are structured. A vague `main()` rewrite
  invites unnecessary changes in benchmark sources when the adaptation should stay as
  narrow as possible.
- Recommendation:
  State the exact strategy in the next plan. The cleanest option is to use the
  no-argc/no-argv port path explicitly and keep the change inside the CoreMark port
  layer rather than broadly rewriting benchmark logic.

### R-003 `microbench migration does not cover the full compatibility surface`

- Severity: MEDIUM
- Section: `Goals / Phase 3`
- Type: Completeness
- Problem:
  The plan mentions replacing `heap.start` / `heap.end` and `LENGTH`, but `microbench`
  also currently depends on `ROUNDUP`, `ioe_init()`, and the `Area heap` contract from
  the NJU headers. Those dependencies are not fully accounted for in the current plan.
- Why it matters:
  This is a source-level completeness gap. Even if `uptime()` and heap bounds are added,
  `microbench` still will not compile cleanly unless the missing compatibility pieces are
  handled deliberately.
- Recommendation:
  Expand the next plan to define the full target-local compatibility shim for
  `microbench`, including `ROUNDUP`, `LENGTH`, a no-op `ioe_init()`, and the heap area
  representation used by the existing code.

### R-004 `microbench C++ sub-benchmarks need an explicit runtime-compatibility check`

- Severity: MEDIUM
- Section: `Implementation Plan / Validation`
- Type: Maintainability
- Problem:
  `microbench` includes `.cc` sources (`15pz`, `dinic`, `ssort`). The xam build system
  does compile `.cc` files, but the plan does not explicitly verify whether those files
  require extra C++ runtime support beyond the current freestanding toolchain.
- Why it matters:
  If these sub-benchmarks happen to depend on unsupported runtime features, the port can
  fail late at link time after the rest of the design appears complete.
- Recommendation:
  Add an explicit validation step in the next plan to verify all `microbench` C++
  sources build and link as part of the xam target. If any entry requires unsupported
  runtime features, document and exclude it from the initial port instead of failing the
  whole round implicitly.

---

## Trade-off Advice

### TR-1 `Heap via linker symbols vs Rust statics`

- Related Plan Item: `T-1`
- Topic: Simplicity vs Modularity
- Reviewer Position: Prefer Option A
- Advice:
  Use linker symbols for heap bounds.
- Rationale:
  This matches standard bare-metal practice, keeps the ABI pointer-shaped for C, and
  avoids unnecessary Rust-to-C bridge code for information the linker already owns.
- Required Action:
  Adopt linker-script heap symbols in the next round.

### TR-2 `Pre-generate alu-tests`

- Related Plan Item: `T-2`
- Topic: Portability vs Cleanliness
- Reviewer Position: Prefer Option A
- Advice:
  Keep `alu-tests/test.c` as a committed generated artifact instead of requiring
  build-time generation.
- Rationale:
  The target build should not depend on host generator availability. The generated file
  is deterministic and the dependency is one-time, not part of the runtime design.
- Required Action:
  Keep as proposed.

---

## Positive Notes

- The scope boundary is disciplined: HAL additions plus benchmark/test adaptation only.
- The plan correctly avoids widening the work into new xemu features.
- The staged order is sensible: HAL first, then consumers.
- The `uptime()` design based on xemu's 10 MHz `mtime` is directionally correct.

---

## Approval Conditions

### Must Fix
- R-001

### Should Improve
- R-002
- R-003
- R-004

### Trade-off Responses Required
- TR-1
- TR-2

### Ready for Implementation
- No
- Reason: the heap ABI in the current plan is the wrong foundation for the downstream
  benchmark ports, and the consumer adaptation details are still incomplete.
