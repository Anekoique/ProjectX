# `trap` PLAN `01`

> Status: Revised
> Feature: `trap`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`

---

## Summary

Install a real HS-mode trap entry. The save/restore sequence lives in
`xvisor/src/hal/arch/riscv/trap/trap.S`, pulled into the crate via
`global_asm!`; the assembly saves the 36-word `TrapFrame` (declared by
the framework SPEC) onto the current kernel stack, calls the Rust
dispatcher in `trap/mod.rs`, restores, and `sret`s. `boot.rs` repoints
`stvec` from the wfi parking-pad to `trap_entry`. A default-off
`trap-canary` cargo feature lets `rust_main` fire `ebreak` to demo the
round-trip; `make trap-test` automates the boot+grep assertion.

## Log

[**Added**]

- C-3 (extended) — explicit sscratch-stays-zero clause, mirrored from
  framework C-8 (so trap SPEC is self-contained when promoted).
- C-5b — restore set is exactly `{sepc, sstatus}`; `scause`/`stval` are
  not written back. Closes R-003.
- C-2 augmentation — explicit `sd zero, 0(sp)` after the frame allocation
  so the x0 slot in the frame really is zero, honouring framework C-7's
  documented intent. Closes R-007.
- C-15 rewrite — self-consistent unsafe-allowlist that names every
  permitted site (`hal/arch/riscv/{boot.rs,trap.rs}` always; `main.rs`
  only under `cfg(feature = "trap-canary")`). Closes R-001.
- C-17 — `make trap-test` captures QEMU stdout via `tee` and asserts both
  the trap line and the post-trap banner via `grep -E`. Closes R-002.
- C-18 — UART invariant: ns16550 is in a printable state from OpenSBI
  handoff, so the dispatcher's `writeln!` is safe at the earliest possible
  trap point. Documents R-005.
- C-19 — `trap_entry`'s visibility is `pub(crate)`; the only legitimate
  referrer is `boot.rs` within the same crate. Closes R-006.

[**Changed**]

- API surface — `trap_entry` declared `pub(crate)` (was `pub`). Inner
  type of `Cause` is `usize` (was `u64`); idiomatic for `usize`-typed
  CSRs on RV64. Closes R-006, adopts TR-4 nudge.
- `Cause` lives in a sub-module `cause` of `trap.rs` (arch-local at
  `hal/arch/riscv/trap/cause.rs`) for readability and to keep
  H-ext-specific cause additions clustered. Host `cargo test` is not
  attempted this iteration — see V-UT-1 and T-5 for the deferral
  rationale. Partially closes R-004 (the hedge is replaced by an
  explicit N/A on V-UT-1 with a follow-on phase named).
- T-1 strengthened — sscratch SP swap is *forced*, not optional, the
  moment P2/P3 introduces VS-mode → HS-mode traps; reusing the guest's
  sp is a correctness bug, not a performance choice. Reflects TR-2.
- T-2 extended — notes the canary-as-feature idiom is anticipated for
  `hext-check` (P2) and beyond, so this is the first instance of a
  repeating pattern. Reflects TR-1.
- V-UT-1 — marked N/A this iteration; the host test harness lands
  when a later phase has enough portable pure code to justify the
  scaffolding. G-2 rides V-IT-1 (cause=0x3 line proves classify routed
  the dispatcher correctly).
- Framework SPEC C-16 (allowlist) is acknowledged as **extended**, not
  silently widened: the extension is captured in trap SPEC C-15. The
  framework SPEC itself remains unchanged on disk this iteration; the
  trap SPEC carries the supersede declaration.

[**Removed**]

- None. C-10 (00_PLAN single-line trap-line format) is preserved with
  the same number and same wording.

[**Unresolved**]

- None.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Rewrote C-15 self-consistently; added Log Changed entry naming the framework-C-16 extension. |
| Review | R-002 | Accepted (option a) | Added C-17 — `make trap-test` captures stdout via `tee` + asserts two grep patterns. Phase 3 step 3 updated to match. |
| Review | R-003 | Accepted | Added C-5b — restore set is `{sepc, sstatus}`; `scause`/`stval` not restored. Phase 1 step 1 sketch reflects the same. |
| Review | R-004 | Partially accepted | Extracted `Cause` + `classify` into arch-local `mod cause`. V-UT-1 marked N/A this iteration (no host harness yet for the bin-only crate); G-2 rides V-IT-1 instead. The host harness lands when a later phase introduces enough portable code to justify it. |
| Review | R-005 | Accepted | Added C-18 documenting the OpenSBI-handoff UART invariant; Failure Flow points at it. |
| Review | R-006 | Accepted | `trap_entry` declared `pub(crate)`; documented in C-19. |
| Review | R-007 | Accepted (option a) | Added `sd zero, 0(sp)` to the save sequence; C-2 updated. |
| Review | TR-1 | Adopt with clarification | Extended T-2 with the canary-pattern note. |
| Review | TR-2 | Adopt with clarification | Strengthened T-1's sscratch-swap-is-forced sentence. |
| Review | TR-3 | Adopt | No change required. |
| Review | TR-4 | Adopt + nudge | `Cause`'s inner type is `usize`. |

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
├── build.rs                                  rerun-if-changed for trap.S
├── Makefile                                  `trap-test` target: build + run + tee + grep
└── src/
    ├── main.rs                               rust_main fires `ebreak` under cfg(feature = "trap-canary")
    └── hal/arch/riscv/
        ├── boot.rs                           stvec ← trap_entry (wfi trampoline deleted)
        ├── csr.rs                            unchanged: write_stvec already exists
        └── trap/
            ├── mod.rs                        TrapFrame + global_asm!(trap.S) + trap_handler + instruction_width()
            ├── trap.S                        trap_entry save/restore assembly
            └── cause.rs                      Cause enum + classify() — arch-local
```

[**Data Structure**]

```rust
/// Trap context contract. Field order matches the framework SPEC.
/// Indexed by `offset_of!` from `trap.S`'s save/restore sequence; the
/// `offset_of!` const-asserts in `mod.rs` pin both sides at build time.
#[repr(C)]
pub struct TrapFrame {
    pub regs:    [usize; 32],   // x0..x31; x0 slot stored zero (sd zero, 0(sp))
    pub sepc:    usize,
    pub scause:  usize,
    pub stval:   usize,
    pub sstatus: usize,
}

/// Classified `scause`. Top bit selects half; the rest is the cause code.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Cause {
    Interrupt(usize),
    Exception(usize),
}
```

[**API Surface**]

```rust
// xvisor/src/hal/arch/riscv/trap/mod.rs (re-exported as hal::arch::trap::*)

// `trap_entry` is defined in trap.S and surfaced via global_asm!.
// The Rust declaration is the extern symbol the linker resolves.
core::arch::global_asm!(include_str!("trap.S"));

unsafe extern "C" {
    /// Trap vector. Address goes into `stvec` in Direct mode (low 2 bits = 0).
    /// Not callable from Rust — the address is taken once during boot.
    pub(crate) fn trap_entry();
}

/// Rust dispatcher invoked by `trap_entry` after the save sequence.
/// Mutates `frame.sepc` to choose the return PC (advanced past `ebreak`
/// by `instruction_width(sepc)` for `Cause::Exception(3)`; other causes
/// terminate without modifying the frame).
#[unsafe(no_mangle)]
pub extern "C" fn trap_handler(frame: &mut TrapFrame);

// xvisor/src/hal/arch/riscv/trap/cause.rs
pub fn classify(scause: usize) -> Cause;
```

[**Constraints**]

- C-1: `trap_entry` is defined in `xvisor/src/hal/arch/riscv/trap/trap.S` and pulled into the crate via `core::arch::global_asm!(include_str!("trap.S"))` in `trap/mod.rs`. The `.S` file is the single source of truth for the save/restore sequence.
- C-2: `trap_entry` allocates a 288-byte frame on entry, writes `sd zero, 0(sp)` for the x0 slot, then stores `x1..x31` and the four CSRs into the frame.
- C-3: Trap entry reuses the caller's stack (sp); `sscratch` stays zero this iteration, matching framework C-8 — no SP swap.
- C-4: `trap_entry` calls `trap_handler` with `a0 = sp` (= `&mut TrapFrame` after the save sequence).
- C-5: After `trap_handler` returns, `trap_entry` restores `x1..x31` and the CSR shadows, then executes `sret`.
- C-5b: The CSR restore set is exactly `{sepc, sstatus}`; `scause` and `stval` are HW-set inputs and are *not* written back.
- C-6: `boot.rs` writes `&trap_entry as *const () as usize` into `stvec` via `csr::write_stvec` before calling `rust_main`.
- C-7: Lower two bits of the `stvec` write are zero (Direct mode); MODE field reserved for future Vectored use.
- C-8: The wfi parking-pad (`trap_trampoline`) is removed from `boot.rs`; `stvec` is never zero after boot.
- C-9: `trap_handler` advances `frame.sepc` past the faulting instruction on `Cause::Exception(3)` (Breakpoint) and returns; the advance is computed by `instruction_width(sepc)` which reads the leading halfword at `sepc` and returns 2 for compressed (`c.ebreak`) or 4 for standard (`ebreak`) encodings. Every other cause calls `terminate(HaltCode::Failure)` after logging.
- C-10: Logging emits exactly one line per trap: `xvisor: trap cause=0x<hex> sepc=0x<hex> stval=0x<hex>`.
- C-11: `xvisor/Cargo.toml` declares feature `trap-canary` in `[features]`; not in `default`.
- C-12: Under `cfg(feature = "trap-canary")`, `rust_main` issues a single `ebreak` after `tp`-readback and before the banner.
- C-13: `xvisor/Makefile` adds a `trap-test` target that runs `cargo build --release --features trap-canary` then launches QEMU with the same flags as `run`, piping QEMU stdout through `tee /tmp/xvisor-trap.log` so the grep assertion in C-17 can read it.
- C-14: `make run` builds with default features only; the trap canary stays out of the standard boot path.
- C-15: Unsafe blocks added by this feature live only in `hal/arch/riscv/{boot.rs,trap/mod.rs}` and, gated by `cfg(feature = "trap-canary")`, in `xvisor/src/main.rs` at the single `ebreak` site. Framework SPEC C-16's allowlist is extended accordingly; this constraint is the durable record.
- C-16: TrapFrame field order remains `regs[32] / sepc / scause / stval / sstatus` (unchanged from framework SPEC C-7); offsets verified by `const _: () = assert!(...)` static checks in `trap/mod.rs`.
- C-17: `make trap-test` asserts (post-QEMU exit) that `/tmp/xvisor-trap.log` matches `^xvisor: trap cause=0x3 sepc=0x[0-9a-f]+ stval=0x[0-9a-f]+$` AND matches `^xvisor: hello from HS-mode \(hartid=0, dtb=0x[0-9a-f]+\)$`; missing either line fails the target with a non-zero exit code.
- C-18: The ns16550 UART at `0x10000000` is in a printable state from the moment OpenSBI hands control to `_start`; `trap_handler`'s `writeln!` is safe at the earliest possible trap point (immediately after `csr::write_stvec`).
- C-19: `trap_entry` has `pub(crate)` visibility; the only legitimate referrer is `boot.rs` taking its address for the `stvec` install.

---

## Runtime

[**Main Flow**]

1. Boot finishes `_start` setup (sp / tp / DTB stash / BSS clear).
2. `boot.rs` calls `csr::write_stvec(trap_entry as *const () as usize)`; the wfi trampoline is gone.
3. `rust_main` runs. If `trap-canary` is on, it executes `ebreak`.
4. HW: `scause ← 3 (Breakpoint)`, `sepc ← addr_of_ebreak`, vectors to `stvec` (= `trap_entry`).
5. `trap_entry` allocates the 288-byte frame (`addi sp, sp, -288`), writes `sd zero, 0(sp)` (x0 slot), then stores `x1..x31` into `8..248(sp)` and `sepc`/`scause`/`stval`/`sstatus` into `248..288(sp)`.
6. `trap_entry` issues `mv a0, sp` and `call trap_handler`.
7. `trap_handler` calls `cause::classify(frame.scause)`, prints the trap line via the UART writer, advances `frame.sepc += 4` for `Cause::Exception(3)`, returns. Other causes fall through to `terminate(HaltCode::Failure)`.
8. `trap_entry` loads `sepc` and `sstatus` back from the frame via `csrw`, loads `x1..x31` from `8..248(sp)`, deallocates (`addi sp, sp, 288`), executes `sret`.
9. Execution resumes at the instruction after `ebreak`; `rust_main` falls through to the banner.
10. `terminate(HaltCode::Success)` halts via the SiFive-test finisher.

[**Failure Flow**]

1. Unrecoverable cause (any cause other than `Exception(3)` in P1, including any interrupt): `trap_handler` writes the trap line through the UART (safe per C-18) and then calls `terminate(HaltCode::Failure)`. No `sret` is attempted; the SiFive-test finisher signals QEMU to exit non-zero. Even if the UART were unreachable (theoretical on QEMU virt — ns16550 is always live there), `terminate()` itself writes the finisher MMIO directly without going through the UART, so the test exit code is preserved.
2. Re-entry while in `trap_handler`: out of scope in P1 (`sstatus.SIE` is zero by default; HS-mode interrupts disabled). If a sync trap fires *inside* the handler — e.g. an illegal-instruction in the dispatcher itself — the second trap would also save to the current sp; the stack would keep growing. Acceptable for P1; a per-hart trap stack with sscratch swap is the structural fix and lands in P2/P3 where it is *forced* by VS-mode → HS-mode traps anyway.

[**State Transitions**]

- HS-mode running rust_main → HS-mode handling trap → HS-mode resumed rust_main (the only legal cycle this iteration).
- Trap → halt (terminate) is the only other exit.

---

## Implementation

[**Phase 1 — Trap entry + dispatcher**]

1. `xvisor/src/hal/arch/riscv/trap/trap.S` (new) — the trap-entry assembly:
   ```
   .section .text
   .globl trap_entry
   .balign 4
   trap_entry:
       addi sp, sp, -288
       sd   zero, 0(sp)              # x0 slot
       sd   x1,  8(sp)
       ... (x2..x31 sequential, 8-byte stride) ...
       sd   x31, 248(sp)
       csrr t0, sepc;    sd t0, 256(sp)
       csrr t0, scause;  sd t0, 264(sp)
       csrr t0, stval;   sd t0, 272(sp)
       csrr t0, sstatus; sd t0, 280(sp)
       mv   a0, sp
       call trap_handler
       ld   t0, 256(sp); csrw sepc, t0
       ld   t0, 280(sp); csrw sstatus, t0
       ld   x1,  8(sp)
       ... (x2..x31 sequential, skipping x0) ...
       ld   x31, 248(sp)
       addi sp, sp, 288
       sret
   ```
2. `xvisor/src/hal/arch/riscv/trap/mod.rs`:
   - Move the `TrapFrame` struct here (already exists in `trap.rs` — convert
     `trap.rs` to `trap/mod.rs`).
   - Declare `pub mod cause;`.
   - Pull the assembly in with
     `core::arch::global_asm!(include_str!("trap.S"))` and declare
     `unsafe extern "C" { pub(crate) fn trap_entry(); }` as the symbol
     handle.
   - Add `#[unsafe(no_mangle)] pub extern "C" fn trap_handler(frame: &mut TrapFrame)` —
     calls `cause::classify(frame.scause)`, formats the trap line, advances
     `frame.sepc` by `instruction_width(frame.sepc)` on
     `Cause::Exception(3)`, calls `terminate(HaltCode::Failure)` on every
     other classification.
   - `const _: () = assert!(offset_of!(TrapFrame, regs) == 0);`
     `const _: () = assert!(offset_of!(TrapFrame, sepc) == 32 * 8);`
     (and equivalents for `scause`, `stval`, `sstatus`) so any future
     field-order drift breaks the build.
3. `xvisor/build.rs` — emit `cargo:rerun-if-changed=src/hal/arch/riscv/trap/trap.S` so cargo notices when the assembly changes.

2. `xvisor/src/hal/arch/riscv/trap/cause.rs`:
   - `#[derive(Copy, Clone, Debug, PartialEq, Eq)] pub enum Cause { Interrupt(usize), Exception(usize) }`
   - `pub fn classify(scause: usize) -> Cause` — masks the top bit of
     `scause` (`1 << (usize::BITS - 1)`) to select the half, returns the
     remainder as the cause code.
   - `#[cfg(test)] mod tests { … }` exercising the classifier.

[**Phase 2 — Wire stvec, drop wfi trampoline**]

1. `xvisor/src/hal/arch/riscv/boot.rs`:
   - Delete `trap_trampoline` and `install_trap_trampoline`.
   - Add `pub(super) fn install_trap_vector()` that calls
     `csr::write_stvec(trap::trap_entry as *const () as usize)`.
   - Call site in `_start`'s post-clear-bss sequence: replace the trampoline
     install with `install_trap_vector()`.
2. Re-export `trap_entry` from `hal::arch::riscv` so `boot.rs` can reach it via the existing module surface.

[**Phase 3 — Canary demo + Makefile target**]

1. `xvisor/Cargo.toml`:
   - Add `[features]` section if absent, `trap-canary = []` entry.
2. `xvisor/src/main.rs`:
   - Add under the existing `rust_main` body, immediately before the banner
     `writeln!`:
       ```rust
       #[cfg(feature = "trap-canary")]
       unsafe { core::arch::asm!("ebreak") };
       ```
3. `xvisor/Makefile`:
   - Add `trap-test` target:
       ```
       trap-test:
       	cargo build --release --features trap-canary
       	$(QEMU_SYSTEM) $(QEMU_FLAGS) -kernel $(BIN) | tee /tmp/xvisor-trap.log
       	grep -E '^xvisor: trap cause=0x3 sepc=0x[0-9a-f]+ stval=0x[0-9a-f]+$$' /tmp/xvisor-trap.log
       	grep -E '^xvisor: hello from HS-mode \(hartid=0, dtb=0x[0-9a-f]+\)$$' /tmp/xvisor-trap.log
       ```
     (variable names match the existing `run` target.)
4. **Spec drift acknowledgement**: the framework SPEC's C-10 (wfi parking-pad as `stvec` target) and C-16 (unsafe allowlist) are superseded/extended by trap SPEC C-6/C-8/C-15. No edit to the framework SPEC on disk this iteration — the trap SPEC is the durable record; the framework SPEC will pick up its CHANGELOG entry when a future task touches it (per Ark features INDEX policy).

---

## Trade-offs

- T-1: **Trap stack discipline.** Reuse caller's sp (chosen) vs dedicated
  per-hart trap stack. Reuse is cheapest, fits MAX_HARTS=1 with no
  preemption and no nested traps; the only cost is that a runaway stack
  during exception handling has no visible guard. P2/P3 *forces* the
  dedicated-stack flip — sscratch SP swap is mandatory once VS-mode →
  HS-mode traps land, because the incoming sp at that boundary is the
  *guest's* sp and reusing it is a correctness bug (privilege-mixing UB
  hazard), not a performance choice.
- T-2: **Demo gating.** `trap-canary` cargo feature (chosen) vs permanent
  `ebreak` in `rust_main`. Permanent is loud-and-lossless evidence the
  trap path lives, but couples the demo to every `make run` forever and
  makes the canary's removal in P2 awkward. Feature flag matches xemu's
  am-test idiom (selectable workloads) and keeps the default boot path
  identical to P0. The same idiom is anticipated for `hext-check` (P2)
  and later phase demos — `trap-canary` is the first instance of a
  repeating pattern.
- T-3: **Inline naked_asm! vs separate `.S` file.** Standalone
  `trap/trap.S` pulled in via `core::arch::global_asm!(include_str!(...))`
  (chosen) — 100+ lines of save/restore are easier to read, diff, and
  re-target when they live as plain assembly than when embedded as a
  Rust string literal inside a `naked_asm!` block. The `.S` filename
  also lets editors / `gdb` / objdump treat it as first-class assembly.
  Cross-file layout consistency is enforced by the `offset_of!` const
  asserts in `trap/mod.rs`: any drift between `TrapFrame` field order
  and the `.S` immediates would have to slip past *both* the assert and
  V-IT-1's round-trip, so the separation is safe. The earlier
  `naked_asm!` inline form was a transient EXECUTE choice; the `.S`
  form is what ships.
- T-4: **`classify()` as enum vs raw masks.** Enum (chosen) reads naturally
  for the small set of causes P1 cares about and extends cleanly when P2
  adds H-ext-specific causes (`VirtualSupervisorExternal` etc.). Raw masks
  would be 3 lines shorter and far less self-documenting. Inner type is
  `usize` to match the `usize`-typed CSR pipeline on RV64.
- T-5: **`Cause`/`classify` placement.** Kept arch-local at
  `hal/arch/riscv/trap/cause.rs` (chosen) rather than promoted to a
  workspace-level sibling crate. `Cause` is RISC-V-specific
  (`scause` encoding, H-ext cause additions to come in P2), so the
  arch-tree placement matches `hal/arch/<arch>/trap/` ownership.
  The cost is that host `cargo test` cannot reach the classifier through
  the bin crate; G-2's automated check moves to V-IT-1, and the host
  test harness lands when a later phase has enough portable pure code
  to make a separate crate pay for itself.

---

## Validation

[**Unit Tests**]

- V-UT-1: N/A this iteration. `cause` is arch-specific code that lives
  inside `hal/arch/riscv/trap/`, not a generic sibling crate; the xvisor
  bin's `no_std` + `no_main` + `panic_handler` scaffold blocks host
  `cargo test` and the cost of restructuring (dual lib/bin target, or a
  separate sibling crate) is not justified for a four-line classifier
  whose behaviour is observable at runtime via V-IT-1. The host test
  harness lands when a later phase introduces enough arch-portable code
  to make it pay for itself (e.g. P4's SBI shim has both bin-side
  dispatch and pure decoding that would benefit from `cargo test`).
  Until then, `make test` is an `@echo` stub matching the framework
  task's pattern, and G-2's automated coverage runs through V-IT-1.

[**Integration Tests**]

- V-IT-1: `make trap-test` boots under QEMU with `--features trap-canary`,
  pipes stdout to `/tmp/xvisor-trap.log`, then asserts both
  `^xvisor: trap cause=0x3 sepc=0x[0-9a-f]+ stval=0x[0-9a-f]+$` and
  `^xvisor: hello from HS-mode \(hartid=0, dtb=0x[0-9a-f]+\)$` via
  `grep -E`. Either grep missing → non-zero exit.
- V-IT-2: `make run` (default features) prints **only** the banner — no
  trap line. Manually inspected this iteration (a default-features grep
  would mirror V-IT-1's plumbing once a regression harness exists; P1
  scope keeps the default-path test as inspection).

[**Failure / Robustness**]

- V-F-1: Manually patch `rust_main` (under a scratch `cfg`) to issue an
  unsupported instruction (e.g. `unimp`); confirm the dispatcher prints
  the trap line and QEMU exits with a non-success code via the SiFive-test
  finisher. (Scratch-branch verification, not part of the committed test
  surface.)
- V-F-2: TrapFrame offset asserts (`const _: () = assert!(...)`) trip the
  build if the field order ever drifts. Confirm by temporarily swapping
  two fields in a scratch branch and observing the build break.

[**Edge Cases**]

- V-E-1: `ebreak` at the very last instruction of `rust_main`'s
  pre-banner block: sepc advance must land exactly on the banner-write
  call, no off-by-four.
- V-E-2: TrapFrame slot for x0 is written zero by `trap_entry`'s
  `sd zero, 0(sp)` (per C-2); x0 stays hardwired zero in HW; Rust code
  may index `frame.regs[0]` safely.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 | V-IT-1 (frame round-trip), V-F-2 (layout assert) |
| G-2 | V-IT-1 (cause=0x3 line proves classify() routed correctly) |
| G-3 | V-IT-1 (banner-after-trap proves sepc advance), V-E-1 |
| G-4 | V-IT-1 (canary on), V-IT-2 (canary off) |
| G-5 | V-IT-2 (default run has no spurious trap) |
| C-1 | inspection: `trap_entry` defined once in `trap/trap.S`, pulled in by `global_asm!` in `trap/mod.rs` |
| C-2 | V-F-2 (offset asserts), V-IT-1 (correct save), V-E-2 (x0 zero) |
| C-3 | inspection: no sscratch swap in `trap_entry`; sp reused |
| C-4 | V-IT-1 (handler receives & mutates frame) |
| C-5 | V-IT-1 (banner after ebreak proves sret) |
| C-5b | inspection: only `{sepc, sstatus}` written back in the restore sequence |
| C-6 | inspection of `boot.rs` after Phase 2 |
| C-7 | inspection: low-2-bits clear in the stvec write |
| C-8 | inspection: `trap_trampoline` symbol absent |
| C-9 | V-IT-1 (recover from Breakpoint), V-F-1 (terminate on other) |
| C-10 | V-IT-1 (line format) |
| C-11 | inspection of `Cargo.toml` |
| C-12 | V-IT-1 (canary fires), V-IT-2 (default off) |
| C-13 | V-IT-1 (target exists, builds, runs, tees log) |
| C-14 | V-IT-2 |
| C-15 | clippy: `-D warnings` over `--bins`; inspection: only allowlisted files contain `unsafe {}` blocks |
| C-16 | V-F-2 |
| C-17 | V-IT-1 (the two greps are the assertion) |
| C-18 | V-IT-1 (UART works at first ebreak); inspection of C-18 narrative |
| C-19 | clippy / rustc: `pub(crate)` visibility on `trap_entry` |
