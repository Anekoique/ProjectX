# `trap` PLAN `00`

> Status: Draft
> Feature: `trap`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`

---

## Summary

Install a real HS-mode trap entry in `xvisor/src/hal/arch/riscv/trap.rs`:
a naked function that saves the 36-word `TrapFrame` (declared by the
framework SPEC) onto the current kernel stack, calls a Rust dispatcher,
restores, and `sret`s. `boot.rs` repoints `stvec` from the wfi parking-pad
to this entry. A default-off `trap-canary` cargo feature lets `rust_main`
fire `ebreak` so we can demo the round-trip without polluting the standard
`make run` path.

## Log

*None in 00_PLAN.*

---

## Spec

[**Goals**]

- G-1: Install a real `trap_entry` that saves the framework's `TrapFrame` and `sret`s.
- G-2: Dispatch traps in Rust by `scause` interrupt-bit + exception code.
- G-3: Recover from synchronous `Breakpoint` by advancing `sepc` past the instruction.
- G-4: Demo the trap round-trip behind a default-off `trap-canary` cargo feature.
- G-5: Remove the wfi parking-pad trampoline and point `stvec` at `trap_entry`.

[**Non-goals**]

- NG-1: No H-extension CSR writes (`hedeleg`, `hideleg`, `hstatus`, `hgatp`) — P2.
- NG-2: No interrupt enabling (`sstatus.SIE`, `sie`); P1 takes synchronous traps only.
- NG-3: No nested-trap support; sscratch stays zero in HS-mode this iteration.

[**Architecture**]

```
xvisor/
├── Cargo.toml                                feature `trap-canary` (default off)
├── Makefile                                  `trap-test` target builds + runs with the feature
└── src/
    ├── main.rs                               rust_main fires `ebreak` under cfg(feature = "trap-canary")
    └── hal/arch/riscv/
        ├── boot.rs                           stvec ← trap_entry (wfi trampoline deleted)
        ├── csr.rs                            unchanged: write_stvec already exists
        └── trap.rs                           ⊕ trap_entry (naked) + trap_handler (Rust) + Cause enum
```

[**Data Structure**]

```rust
/// Trap context contract. Field order matches the framework SPEC.
/// Indexed by `offset_of!` from the naked save/restore assembly.
#[repr(C)]
pub struct TrapFrame {
    pub regs:    [usize; 32],   // x0..x31; x0 slot preserved zero
    pub sepc:    usize,
    pub scause:  usize,
    pub stval:   usize,
    pub sstatus: usize,
}

/// Classified `scause`. Top bit of `scause` selects the half; the rest is the cause code.
#[derive(Copy, Clone, Debug)]
pub enum Cause {
    Interrupt(u64),     // raw cause code with interrupt bit stripped
    Exception(u64),     // raw cause code
}
```

[**API Surface**]

```rust
// xvisor/src/hal/arch/riscv/trap.rs (re-exported as hal::arch::trap::*)

/// Trap vector. Address goes into `stvec` in Direct mode (low 2 bits = 0).
/// SAFETY: caller installs this once during boot; not callable from Rust.
#[unsafe(naked)]
pub unsafe extern "C" fn trap_entry();

/// Decode `scause` into interrupt-vs-exception + cause code.
pub fn classify(scause: usize) -> Cause;

/// Rust dispatcher invoked by `trap_entry` after the save sequence.
/// Mutates `frame.sepc` to choose the return PC (`+4` past `ebreak`,
/// unchanged for other recoverable causes).
#[unsafe(no_mangle)]
pub extern "C" fn trap_handler(frame: &mut TrapFrame);
```

[**Constraints**]

- C-1: `trap_entry` is a single naked function in `xvisor/src/hal/arch/riscv/trap.rs`, no separate `.S` file.
- C-2: `trap_entry` saves `x1..x31` then `sepc`/`scause`/`stval`/`sstatus` into a 36-word frame on the current stack; x0 slot is left untouched (zero).
- C-3: Trap entry reuses the caller's stack (sp); no dedicated trap stack — single-context HS-mode, no nesting yet.
- C-4: `trap_entry` calls `trap_handler` with `a0 = &mut TrapFrame` pointing at the frame on stack.
- C-5: After `trap_handler` returns, `trap_entry` restores from the same frame and executes `sret`.
- C-6: `boot.rs` writes `&trap_entry as *const () as usize` into `stvec` via `csr::write_stvec` before calling `rust_main`.
- C-7: Lower two bits of the `stvec` write are zero (Direct mode); MODE field reserved for future Vectored use.
- C-8: The wfi parking-pad (`trap_trampoline`) is removed from `boot.rs`; `stvec` is never zero after boot.
- C-9: `trap_handler` advances `frame.sepc` by 4 on `Exception(3)` (Breakpoint) and returns; every other cause calls `terminate(HaltCode::Failure)` after logging.
- C-10: Logging emits exactly one line per trap: `xvisor: trap cause=0x<hex> sepc=0x<hex> stval=0x<hex>`.
- C-11: `xvisor/Cargo.toml` declares feature `trap-canary` in `[features]`; not in `default`.
- C-12: Under `cfg(feature = "trap-canary")`, `rust_main` issues a single `ebreak` after `tp`-readback and before the banner.
- C-13: `xvisor/Makefile` adds a `trap-test` target that runs `cargo build --release --features trap-canary` then launches QEMU with the same flags as `run`.
- C-14: `make run` builds with default features only; the trap canary stays out of the standard boot path.
- C-15: No new `unsafe` blocks outside `hal/arch/riscv/{boot.rs,trap.rs}`; the `ebreak` site uses `unsafe { core::arch::asm!("ebreak") }`.
- C-16: TrapFrame field order remains `regs[32] / sepc / scause / stval / sstatus` (unchanged from framework SPEC C-7); offsets verified by `const _: () = assert!(...)` static checks in `trap.rs`.

---

## Runtime

[**Main Flow**]

1. Boot finishes `_start` setup (sp / tp / DTB stash / BSS clear).
2. `boot.rs` calls `csr::write_stvec(trap_entry as *const () as usize)`; previous wfi trampoline is gone.
3. `rust_main` runs. If `trap-canary` is on, it executes `ebreak`.
4. HW: `scause ← 3 (Breakpoint)`, `sepc ← addr_of_ebreak`, vectors to `stvec` (= `trap_entry`).
5. `trap_entry` makes room (`addi sp, sp, -288`), stores `x1..x31` into `regs[1..32]`, reads `sepc/scause/stval/sstatus` CSRs, stores them.
6. `trap_entry` calls `trap_handler(frame: &mut TrapFrame)`.
7. `trap_handler` classifies, prints the trap line, advances `sepc += 4`, returns.
8. `trap_entry` restores `x1..x31` + CSR shadows (`csrw sepc, ...` etc.), executes `sret`.
9. Execution resumes at the instruction after `ebreak`; `rust_main` falls through to the banner.
10. `terminate(HaltCode::Success)` halts via SiFive-test finisher.

[**Failure Flow**]

1. Unrecoverable cause (anything other than Breakpoint, or any interrupt in P1):
   `trap_handler` prints the trap line, then calls `terminate(HaltCode::Failure)` —
   no `sret` attempted; the finisher signals QEMU to exit non-zero.
2. Re-entry while in `trap_handler`: out of scope in P1 (`sstatus.SIE` is zero by
   default; HS-mode interrupts disabled). If it ever happens (illegal-instruction
   inside the handler), the second trap also saves to the current sp — the stack
   keeps growing; eventually the guard page (when we add one) panics. P1 stays
   single-trap.

[**State Transitions**]

- HS-mode running rust_main → HS-mode handling trap → HS-mode resumed rust_main
  (the only legal cycle this iteration).
- Trap → halt (terminate) is the only other exit.

---

## Implementation

[**Phase 1 — Trap entry + dispatcher**]

1. `xvisor/src/hal/arch/riscv/trap.rs`:
   - Add `#[unsafe(naked)] pub unsafe extern "C" fn trap_entry()` with
     `core::arch::naked_asm!` doing: `addi sp, sp, -288`, `sd x1..x31`
     into `0..248(sp)`, `csrr/sd` for sepc/scause/stval/sstatus into
     `248..288(sp)`, `mv a0, sp`, `call trap_handler`, `ld` everything
     back, `addi sp, sp, 288`, `sret`.
   - Add `pub fn classify(scause: usize) -> Cause` (top-bit split).
   - Add `#[unsafe(no_mangle)] pub extern "C" fn trap_handler(frame: &mut TrapFrame)`.
     Match: `Exception(3)` → `frame.sepc += 4`; `_` → `terminate(Failure)`.
   - `const _: () = assert!(offset_of!(TrapFrame, sepc) == 32 * 8);` etc.,
     so any future drift breaks the build.

[**Phase 2 — Wire stvec, drop wfi trampoline**]

1. `xvisor/src/hal/arch/riscv/boot.rs`:
   - Delete `trap_trampoline` and `install_trap_trampoline`.
   - Rename / restructure to call
     `csr::write_stvec(trap_entry as *const () as usize)`.
2. Re-export `trap_entry` from `hal::arch` so `boot.rs` can reach it.

[**Phase 3 — Canary demo + Makefile target**]

1. `xvisor/Cargo.toml`: add `trap-canary = []` under `[features]`.
2. `xvisor/src/main.rs`: under `#[cfg(feature = "trap-canary")]`, issue
   `unsafe { core::arch::asm!("ebreak") }` immediately before the banner write.
3. `xvisor/Makefile`: add `trap-test` target — `cargo build --release --features trap-canary` then the same QEMU launch as `run`. `make run` and `make test` unchanged.
4. Update the `xvisor/framework` SPEC `[**CHANGELOG**]` block: C-10 superseded
   by C-6/C-7 of `xvisor/trap` (wfi parking-pad replaced by `trap_entry`).

---

## Trade-offs

- T-1: **Trap stack discipline.** Reuse caller's sp (chosen) vs dedicated
  per-hart trap stack. Reuse is cheapest, fits MAX_HARTS=1 with no
  preemption and no nested traps; the only cost is that a runaway stack
  during exception handling has no visible guard. A dedicated stack costs
  +N KiB per hart and a sscratch pointer; deferred until P2/P3 introduces
  guest contexts that share the host's sp.
- T-2: **Demo gating.** `trap-canary` cargo feature (chosen) vs permanent
  `ebreak` in `rust_main`. Permanent is loud-and-lossless evidence the
  trap path lives, but couples the demo to every `make run` forever and
  makes the canary's removal in P2 awkward. Feature flag matches xemu's
  am-test idiom (selectable workloads) and keeps the default boot path
  identical to P0.
- T-3: **Naked assembly inline vs separate `.S`.** Inline `naked_asm!`
  (chosen) keeps the trap-entry contract in one file with TrapFrame —
  drift between layout and saves becomes a compile error via `offset_of!`
  static asserts. A separate `.S` would buy hand-tuning options we don't
  need yet at the cost of cross-file consistency.
- T-4: **`classify()` as enum vs raw masks.** Enum (chosen) reads naturally
  for the small set of causes P1 cares about and extends cleanly when
  P2 adds H-ext-specific causes (`VirtualSupervisorExternal` etc.).
  Raw masks would be 3 lines shorter and far less self-documenting.

---

## Validation

[**Unit Tests**]

- V-UT-1: `classify(0x0000_0003)` → `Cause::Exception(3)`;
  `classify(0x8000_0000_0000_0005)` → `Cause::Interrupt(5)`. Host-runnable
  by gating `classify` and the test under `#[cfg(any(target_arch = "x86_64", target_arch = "riscv64"))]`.
  *(If host gating proves awkward, mark V-UT-1 N/A and verify via the
  V-IT-1 trap line instead — `make test` stays an `@echo` stub until P1's
  follow-on test framework lands.)*

[**Integration Tests**]

- V-IT-1: `make trap-test` boots under QEMU, prints exactly one
  `xvisor: trap cause=0x3 sepc=0x[0-9a-f]+ stval=0x[0-9a-f]+` line
  followed by the framework's banner
  `xvisor: hello from HS-mode (hartid=0, dtb=0x[0-9a-f]+)`, then exits
  cleanly. Asserted by grep.
- V-IT-2: `make run` (default features) prints **only** the banner — no
  trap line. Asserts the canary is gated off by default.

[**Failure / Robustness**]

- V-F-1: Manually patch `rust_main` (under a scratch `cfg`) to issue an
  unsupported instruction (e.g. `unimp`); confirm the dispatcher prints
  the trap line and QEMU exits with a non-success code via the
  SiFive-test finisher.
- V-F-2: TrapFrame offset asserts (`const _: () = assert!(...)`) trip the
  build if the field order ever drifts. Confirm by temporarily swapping
  two fields in a scratch branch and observing the build break.

[**Edge Cases**]

- V-E-1: `ebreak` at the very last instruction of `rust_main`'s
  pre-banner block: sepc advance must land exactly on the banner-write
  call, no off-by-four.
- V-E-2: TrapFrame slot for x0 is unread by `trap_entry` save and unwritten
  on restore; x0 stays hardwired zero in HW.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-1 (frame round-trip), V-F-2 (layout assert) |
| G-2 | V-UT-1 (classify), V-IT-1 (cause=0x3 line) |
| G-3 | V-IT-1 (banner-after-trap proves sepc advance), V-E-1 |
| G-4 | V-IT-1 (canary on), V-IT-2 (canary off) |
| G-5 | V-IT-2 (default run has no spurious trap) |
| C-1 | inspection: single `trap_entry` in `trap.rs` |
| C-2 | V-F-2 (offset asserts); V-IT-1 (correct save) |
| C-3 | inspection: no sscratch swap in `trap_entry`; sp reused |
| C-4 | V-IT-1 (handler receives & mutates frame) |
| C-5 | V-IT-1 (banner after ebreak proves sret) |
| C-6 | inspection of `boot.rs` after Phase 2 |
| C-7 | inspection: low-2-bits clear in the stvec write |
| C-8 | inspection: `trap_trampoline` symbol absent |
| C-9 | V-IT-1 (recover from Breakpoint), V-F-1 (terminate on other) |
| C-10 | V-IT-1 (line format) |
| C-11 | inspection of `Cargo.toml` |
| C-12 | V-IT-1 (canary fires); V-IT-2 (default off) |
| C-13 | V-IT-1 (target exists and runs) |
| C-14 | V-IT-2 |
| C-15 | clippy: `-D warnings` over `--bins`, no new `unsafe_op_in_unsafe_fn` warnings |
| C-16 | V-F-2 |
