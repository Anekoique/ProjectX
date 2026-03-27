# CSR Subsystem Implementation Review

> Review date: 2026-03-21
> Scope: full CSR subsystem implementation in `xemu/xcore`
> Basis: local code inspection + `cargo test -p xcore`

## Findings

### [HIGH] Illegal instructions still bypass the new trap path

File:
- `xemu/xcore/src/cpu/riscv/inst.rs:17-29`
- `xemu/xcore/src/cpu/riscv/mod.rs:83-108`
- `xemu/xcore/src/cpu/riscv/inst/base.rs:15-28`

Issue:

The new CSR/trap subsystem introduces `pending_trap`, but architectural illegal-instruction cases still escape through `Err(XError::InvalidInst)`.

This currently happens in two places:

- `dispatch()` returns `Err(XError::InvalidInst)` for unhandled decoded instructions
- several instruction handlers still return `InvalidInst` directly for ISA-illegal cases, especially RV64-only instructions on RV32 and invalid compressed encodings

As a result, these cases do not go through:

- `raise_trap(IllegalInstruction, ...)`
- `commit_pending_trap()`
- `mepc/sepc`, `mcause/scause`, `mtval/stval`
- delegation logic

That leaves the emulator with a split exception model:

- CSR/privileged violations use architectural traps
- other illegal instructions still abort through `XError`

Fix:

Add a translation point that converts architectural `InvalidInst` cases into `IllegalInstruction` traps before they leave the core execution path.

The cleanest choices are:

1. translate in `RVCore::execute()` around `dispatch()`
2. or refactor instruction handlers so architectural illegal cases call `raise_trap(...)` directly

Do not leave both models active long-term.

### [HIGH] `ebreak` is still implemented as host termination policy, not as an architectural trap

File:
- `xemu/xcore/src/cpu/riscv/mod.rs:90-101`
- `xemu/xcore/src/cpu/riscv/inst/privileged.rs:20-26`
- `xemu/xcore/src/cpu/riscv/inst/compressed.rs:315-321`

Issue:

`ebreak` and `c.ebreak` now correctly create `pending_trap(Breakpoint)`, but `RVCore::execute()` intercepts that trap before `commit_pending_trap()` runs:

- it checks `pending_trap`
- detects `Breakpoint`
- clears the trap
- advances `pc`
- returns `Err(XError::ToTerminate)`

So breakpoint never becomes a real architectural trap.

That means:

- `mepc/scause/mtval` or `sepc/scause/stval` are never written
- delegation cannot happen
- debugger/batch behavior is mixed into core execution semantics

Fix:

Let `Breakpoint` follow the normal trap path all the way through `commit_pending_trap()`.

If batch mode should stop on `ebreak`, make that decision outside `RVCore`, in the runner or debugger layer after the architectural trap has been committed.

### [HIGH] Interrupt CSRs exist, but interrupts still cannot occur during real execution

File:
- `xemu/xcore/src/cpu/mod.rs:95-100`
- `xemu/xcore/src/cpu/riscv/mod.rs:83-108`
- `xemu/xcore/src/cpu/riscv/trap_handler.rs:47-55`

Issue:

The CSR subsystem now models:

- `mie`
- `mip`
- `mideleg`
- `mtvec/stvec`
- interrupt causes in `TrapCause`

But the runtime execution path still has no interrupt sampling logic.

`CPU::step()` is still only:

1. `fetch()`
2. `decode()`
3. `execute()`

There is no:

- `check_pending_interrupt()`
- pre-fetch interrupt sampling
- direct path from pending interrupt bits to `raise_trap(TrapCause::Interrupt(...))`

So interrupt handling currently exists only in unit tests that manually construct traps, not in real execution.

Fix:

Add interrupt sampling before `fetch()` in `CPU::step()` or an equivalent top-level boundary.

Once an interrupt is taken:

- do not fetch/decode/execute the next instruction
- raise the interrupt trap
- commit it through the same trap machinery

### [MEDIUM] `sstatus` can incorrectly write the read-only `SD` summary bit

File:
- `xemu/xcore/src/cpu/riscv/csr/mstatus.rs:29-40`
- `xemu/xcore/src/cpu/riscv/csr/mod.rs:89-126`

Issue:

`SSTATUS_MASK` is reused as the write mask for the `sstatus` shadow CSR:

- `SSTATUS_MASK` includes `SD`
- `sstatus` uses `RW(SSTATUS_MASK)`

But `SD` is a summary bit, not a software-writable field.

This creates an inconsistency:

- direct `mstatus` writes block `SD` because `MStatus::WRITABLE` excludes it
- `sstatus` writes can still set it through the alias path

Fix:

Split `sstatus` view and write masks:

- one mask for readable S-mode-visible bits
- one stricter write mask excluding `SD`

Do not reuse the visibility mask as the write mask when read-only summary bits are present.

### [MEDIUM] `cycle` is currently implemented as another `instret`

File:
- `xemu/xcore/src/cpu/riscv/csr/mod.rs:188-191`
- `xemu/xcore/src/cpu/riscv/mod.rs:104-107`

Issue:

`increment_counters()` increments both `cycle` and `instret` together, and `execute()` skips both when the current instruction traps.

That makes `cycle` behave like retired-instruction count, not like elapsed cycles.

Even for a simple emulator, these two counters should not be locked together this tightly:

- `instret` should depend on retirement
- `cycle` should advance per step or per modeled cycle budget

Fix:

Split the logic into:

- `increment_cycle()`
- `increment_instret()`

Call `instret` only on successful retirement. Call `cycle` on every architectural step, including trap-taking paths if that is the chosen model.

## Verification

Local verification completed:

```bash
cd xemu && cargo test -p xcore
```

Result:

- `160/160` tests passed

This means the issues above are semantic design/behavior gaps, not compile failures or obvious broken tests.

## Review Summary

| Severity | Count | Status |
|----------|-------|--------|
| CRITICAL | 0 | pass |
| HIGH     | 3 | warn |
| MEDIUM   | 2 | info |
| LOW      | 0 | note |

Verdict: WARNING

## Bottom Line

The CSR subsystem is structurally much better than the pre-CSR state, and the storage / privilege / trap layering is now mostly in the right shape.

The main remaining risk is semantic inconsistency:

- some architectural exceptions already use the trap path
- some still escape through `XError`
- breakpoint is still treated as runner policy inside the core
- interrupt delivery is modeled in data structures but not wired into execution

Those are the first issues to fix before building more privileged-mode features on top.
