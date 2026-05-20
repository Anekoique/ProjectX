# `trap` REVIEW `00`

> Status: Open
> Feature: `trap`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking: 0
- Non-blocking: 7 (2 HIGH, 3 MEDIUM, 2 LOW)

## Summary

The PLAN is substantively sound: TrapFrame layout, naked save/restore, the `classify()`/`trap_handler` dispatcher split, the `trap-canary` feature gating, and the supersede of the wfi parking-pad all hang together and respect the framework SPEC's binding contracts (C-7 TrapFrame, C-8 sscratch reservation, C-10 superseded explicitly via CHANGELOG). The 36-word / 288-byte frame is RV64-ABI-aligned (288 mod 16 = 0) and the naked entry's `mv a0, sp` → `extern "C" fn trap_handler(&mut TrapFrame)` is ABI-correct. Two HIGH issues need addressing in 01: (1) C-15's unsafe-allowlist sentence is internally contradictory and silently extends framework C-16 to `main.rs` without an explicit supersede; (2) the `make trap-test` target as drafted does not actually wire the grep assertion that V-IT-1 leans on — Implementation Phase 3 step 3 only launches QEMU. Smaller items: sret CSR-restore ordering, V-UT-1's host-gating hedge, and the missing failure-flow detail when an unrecoverable cause hits before any UART write succeeds. No CRITICAL findings; this is fixable in one revision.

---

## Findings

### R-001 `C-15 unsafe-block allowlist is self-contradictory and silently widens framework C-16`

- **Severity:** HIGH
- **Section:** `## Spec` → `[**Constraints**]` C-15; also `## Implementation` Phase 3 step 2
- **Problem:** C-15 reads "No new `unsafe` blocks outside `hal/arch/riscv/{boot.rs,trap.rs}`; the `ebreak` site uses `unsafe { core::arch::asm!("ebreak") }`." The `ebreak` site lives in `xvisor/src/main.rs` (see C-12 and Phase 3 step 2: "`xvisor/src/main.rs`: under `#[cfg(feature = "trap-canary")]`, issue `unsafe { core::arch::asm!("ebreak") }`"). So the first half of C-15 forbids a `main.rs` unsafe block, and the second half mandates one — self-contradictory. Separately, the framework SPEC C-16 anchors the project-wide unsafe allowlist to `hal/arch/riscv/{boot.rs,cpu.rs,csr.rs}` + `hal/platform/qemu/{uart.rs,halt.rs}`. The trap PLAN extends that allowlist to include `trap.rs` (legitimate, since the feature owns it) and to allow a `main.rs` unsafe site (under a feature gate), but neither extension appears in `## Log` as a Changed/Removed entry against framework C-16. Per the mandatory rejection rule, contradicting a framework constraint requires an explicit supersede.
- **Why it matters:** Without an explicit framework-C-16 supersede recorded in `## Log`, the next reviewer (or the spec-promotion step) cannot tell whether the unsafe-site widening was intentional or an oversight. The contradiction inside C-15 will also confuse the executor: which clause governs?
- **Recommendation:** Rewrite C-15 to be self-consistent — e.g. "Unsafe blocks added by this feature live only in `hal/arch/riscv/{boot.rs,trap.rs}` and, gated by `cfg(feature = "trap-canary")`, in `xvisor/src/main.rs` at the single `ebreak` site." Add a `## Log` entry: "Framework SPEC C-16 unsafe allowlist extended to add `hal/arch/riscv/trap.rs` (this feature's trap entry) and, under `cfg(feature = "trap-canary")`, the single `ebreak` site in `main.rs`. Captured in trap SPEC C-15." This makes the spec change auditable and resolves the textual contradiction.

### R-002 `make trap-test does not wire the grep assertion V-IT-1 depends on`

- **Severity:** HIGH
- **Section:** `## Spec` C-13, `## Implementation` Phase 3 step 3, `## Validation` V-IT-1
- **Problem:** V-IT-1 promises "`make trap-test` boots under QEMU, prints exactly one ... line followed by the framework's banner ... then exits cleanly. Asserted by grep." But C-13 and the Phase 3 step 3 only specify "`cargo build --release --features trap-canary` then launches QEMU with the same flags as `run`." The current `xvisor/Makefile` `run` target shells QEMU directly (`$(QEMU_SYSTEM) $(QEMU_FLAGS)`) — stdout streams to the terminal, never captured into a variable, never piped to grep. So as drafted, `make trap-test` will run the canary and exit (because `terminate(HaltCode::Success)` halts QEMU via SiFive-test) but **no assertion fires** — V-IT-1's grep is aspirational.
- **Why it matters:** Five validations (G-1, G-2, G-3, G-4, G-5 acceptance rows; C-2/C-4/C-5/C-9/C-10/C-12 inspection rows) lean on V-IT-1. If V-IT-1 is "run it and look at the screen", G-4 / G-5 lose their automated check and the next iteration's regression-test discipline degrades. The framework iteration already established that `make test` is an `@echo` stub; trap is the first phase that has a real round-trip to assert, so it should set the harness pattern for P2+.
- **Recommendation:** Either (a) make C-13 explicit that `trap-test` captures QEMU stdout (e.g. via `tee /tmp/trap.log` plus a `grep -E '^xvisor: trap cause=0x3 sepc=0x[0-9a-f]+ stval=0x[0-9a-f]+$' /tmp/trap.log && grep -E 'hello from HS-mode' /tmp/trap.log` follow-up) and add the corresponding Implementation step, or (b) downgrade V-IT-1 to "observe the two lines in the terminal" and note that automated assertion is deferred to a later test-harness phase. Option (a) keeps the validation honest; option (b) keeps the iteration small but should be paired with a TODO for P2. Choose one and reflect the choice in both `## Spec` (C-13) and `## Validation` (V-IT-1).

### R-003 `Sret CSR-restore ordering left implicit`

- **Severity:** MEDIUM
- **Section:** `## Runtime` Main Flow step 8, `## Implementation` Phase 1 step 1
- **Problem:** Step 8 says "trap_entry restores x1..x31 + CSR shadows (`csrw sepc, ...` etc.), executes `sret`." The Phase 1 description echoes "`ld` everything back ... `sret`." The ordering of CSR restores matters slightly: `sstatus` must be restored before `sret` (since `sret` consumes `sstatus.SPP`/`SPIE` to choose target privilege and re-enable interrupts), and `sepc` must be restored before `sret` (since `sret` jumps to `sepc`). `scause`/`stval` need not be restored at all in this iteration — they are read-only inputs to the handler, not part of the trap return contract. The PLAN does not say which CSRs are restored and which are not, leaving it to the executor.
- **Why it matters:** A naive implementation that restores `scause`/`stval` is harmless but wastes two instructions per trap; a less-naive implementation that forgets `sstatus` will silently leave `SIE` in the wrong state when P2 turns interrupts on. Locking this down now is cheap and prevents a P2 regression-hunt.
- **Recommendation:** In `## Spec`, add a constraint (or extend C-5) along the lines of "On return, `trap_entry` restores `sepc` and `sstatus` from the frame (in either order) and executes `sret`; `scause` and `stval` are not written back — they are HW-set inputs only, not part of the return contract." Reflect the same in Phase 1 step 1's restore sketch.

### R-004 `V-UT-1's host-gating hedge undermines its acceptance role`

- **Severity:** MEDIUM
- **Section:** `## Validation` Unit Tests V-UT-1; Acceptance Mapping (G-2 row)
- **Problem:** V-UT-1 is the only Validation mapping for G-2 ("Dispatch traps in Rust by `scause` interrupt-bit + exception code") in the Acceptance Mapping table — but the PLAN immediately hedges: "*If host gating proves awkward, mark V-UT-1 N/A and verify via the V-IT-1 trap line instead — `make test` stays an `@echo` stub until P1's follow-on test framework lands.*" Since V-IT-1 itself has wiring issues (R-002), G-2 can end up with no automated coverage at all.
- **Why it matters:** `classify()` is pure, side-effect-free arithmetic on `usize`; gating its unit test under `#[cfg(test)]` in `trap.rs` with no `cfg(target_arch = ...)` predicate should compile and run under `cargo test` on the host, regardless of `riscv64gc-unknown-none-elf` being the production target. The PLAN's hedge reads like uncertainty about whether the crate will compile for `cargo test` (since it's `no_std`+`no_main`+`panic_handler`). That's solvable by extracting `classify()` and the `Cause` enum into a sub-module compiled with `#![cfg_attr(test, no_main = false)]` or by placing the test in `tests/` under a `cfg(target_arch = "x86_64")` gate; or by leaving the `@echo` stub in place but committing to V-IT-1 (which then must be wired per R-002).
- **Recommendation:** Drop the hedge. Either commit to V-UT-1 as a real `cargo test` and outline the gating (host build of just the `Cause`/`classify` module — small and feasible since `classify` is `fn(usize) -> Cause` with no environment), or drop V-UT-1 outright and rely on V-IT-1 (provided R-002 is fixed). The current "we'll try, and if it doesn't work, mark it N/A" leaves G-2 with no committed validation.

### R-005 `Failure flow leaves UART-pre-init traps unspecified`

- **Severity:** MEDIUM
- **Section:** `## Runtime` Failure Flow
- **Problem:** Failure Flow step 1 says an unrecoverable cause calls `terminate(HaltCode::Failure)` after logging. `terminate()` and the trap-line logging both go through the UART writer. In the canary path that's fine — UART is already usable when `rust_main` fires `ebreak`. But the trap entry is installed by `boot.rs` *before* `rust_main` runs (per framework C-10 supersede in this PLAN). If a stray trap fires between `csr::write_stvec(trap_entry...)` and the first UART byte in `rust_main` (e.g. an illegal-instruction from a misbuilt binary), the dispatcher will try to `writeln!` through a UART that's still in whatever state OpenSBI left it. In practice the ns16550 LSR THRE poll handles that, but the PLAN doesn't say.
- **Why it matters:** Doesn't break P1 — UART is in fact usable at OpenSBI handoff — but documenting the invariant prevents a P2/P3 surprise when timer-interrupt setup or H-ext CSR writes start firing traps earlier in `rust_main`.
- **Recommendation:** Add one sentence to Failure Flow noting the UART is usable from the moment `stvec` is installed (OpenSBI's ns16550 is in a printable state on QEMU virt), so the dispatcher's `writeln!` is safe at the earliest possible trap point. If you want to be extra-defensive, note that a UART-unreachable trap (theoretical, not on QEMU virt) would still reach `terminate()`'s SiFive-test finisher write since it bypasses the UART.

### R-006 `trap_entry exposed as pub unsafe extern "C" with empty body in API Surface`

- **Severity:** LOW
- **Section:** `## Spec` → `[**API Surface**]`
- **Problem:** The API Surface declares `pub unsafe extern "C" fn trap_entry();` with no body — fine as a signature sketch, but the `pub` visibility plus a comment "not callable from Rust" reads slightly off. Naked trap entries are usually `pub(crate)` (or `pub(super)`) because the only legitimate referrer is `boot.rs`'s `csr::write_stvec` call site within the same crate. Exposing it as `pub` invites future code outside `hal::arch::riscv` to take its address, which silently breaks the "installed once during boot" SAFETY contract in the doc comment.
- **Why it matters:** Cosmetic / encapsulation; not a build break.
- **Recommendation:** Consider `pub(crate)` (or just `pub` inside `mod trap` and re-exported only via `hal::arch::riscv` for `boot.rs`). If `pub` is deliberate (planned consumers in P2's H-ext code), say so in a comment.

### R-007 `Trap-frame x0 slot left undefined-but-not-zero by save sequence`

- **Severity:** LOW
- **Section:** `## Spec` C-2; `## Runtime` Main Flow step 5; `## Validation` V-E-2
- **Problem:** C-2 says "x0 slot is left untouched (zero)". V-E-2 echoes "TrapFrame slot for x0 is unread by `trap_entry` save and unwritten on restore; x0 stays hardwired zero in HW." That's true *only* if the stack frame's 288 bytes are pre-zeroed — but `addi sp, sp, -288` just decrements sp; the 288 bytes are whatever was on the stack before. If `regs[0]` is then read by Rust code (e.g. `frame.regs[some_runtime_index]` where `some_runtime_index == 0`), it sees stack garbage, not zero. In this iteration the dispatcher only reads `frame.scause`/`frame.sepc`/`frame.stval`, so it's moot — but the "x0 slot preserved zero so `frame.regs[rd]` indexing works for any encoded rd" claim from framework C-7 (and the trap PLAN's TrapFrame doc comment) does not hold without an explicit `sd zero, 0(sp)` in the save sequence.
- **Why it matters:** Lurking footgun for P2 / H-ext, where guest emulation may index `frame.regs[rd]` with `rd` decoded from a faulting instruction.
- **Recommendation:** Either (a) add `sd zero, 0(sp)` to the save sequence so the x0 slot really is zero (one instruction, no performance impact), or (b) clarify C-2 to say the x0 slot is left undefined this iteration and indexable-from-Rust safety lands when emulation needs it. Option (a) matches the framework's documented intent more cleanly.

---

## Trade-off Advice

### TR-1 `Trap-canary as cargo feature vs permanent ebreak`

- **Related Plan Item:** T-2
- **Topic:** Compatibility vs Clean Design
- **Reviewer Position:** Prefer the chosen direction (cargo feature)
- **Advice:** Keep the `trap-canary` feature flag as drafted.
- **Rationale:** Permanent `ebreak` in `rust_main` would pollute every P2+ `make run` and would have to be removed (with a corresponding spec change) the moment the H-ext CSR-write phase introduces real traps to validate. Cargo features cost almost nothing in xvisor's build matrix and mirror xemu's selectable workload idiom. The decisive argument is that P2's hext-check demo is naturally a separate cargo feature (or its own Makefile target) for the same reason; trap-canary sets the pattern.
- **Required Action:** Keep with clarification — add a one-sentence note in T-2 that the same feature-flag idiom is anticipated for `hext-check` (P2) and beyond, so the executor knows the canary is the first instance of a repeating pattern.

### TR-2 `Trap stack discipline: reuse caller sp vs dedicated trap stack`

- **Related Plan Item:** T-1
- **Topic:** Performance vs Safety
- **Reviewer Position:** Prefer the chosen direction (reuse caller sp) for P1; revisit at P2/P3
- **Advice:** Keep `sscratch = 0` / reuse caller sp for this iteration. Note in T-1 that the dedicated-stack flip is forced (not optional) the moment P2/P3 introduces a VS-mode → HS-mode trap, because the incoming sp at that boundary is the *guest's* sp and using it is a privilege-mixing UB hazard.
- **Rationale:** The framework SPEC's C-8 reserves sscratch "for trap-entry SP swap and left zero this iteration"; the trap PLAN's C-3 mirrors that ("no sscratch swap") — fully consistent. The reuse cost is one missing guard page during HS-mode self-traps, which is acceptable when nothing in HS-mode is intentionally trapping outside the canary. But P2's guest-entry preparation will produce VS-mode → HS-mode traps where sp at trap entry belongs to the guest, and reusing it is a correctness bug, not a performance choice. Surfacing that now keeps the iteration cap from being burned later.
- **Required Action:** Keep with clarification — extend T-1's last sentence ("deferred until P2/P3 introduces guest contexts that share the host's sp") to read more strongly: "P2/P3 forces the dedicated-stack flip — sscratch SP swap is mandatory once VS-mode → HS-mode traps land; reusing the guest's sp is incorrect, not just slower."

### TR-3 `Inline naked_asm! vs separate .S file`

- **Related Plan Item:** T-3
- **Topic:** Maintainability vs Tunability
- **Reviewer Position:** Prefer the chosen direction (inline `naked_asm!`)
- **Advice:** Keep inline naked_asm!.
- **Rationale:** Framework C-14 already requires inline `naked_asm!` for boot.rs; trap.rs taking the same approach keeps the codebase consistent and the `offset_of!` static-assert trick only works if the layout struct and the save sequence live in the same translation unit. Separate `.S` files don't pay for themselves until someone needs hand-tuned instruction scheduling — not in P1, and probably not before P9.
- **Required Action:** Adopt.

### TR-4 `Cause as enum vs raw masks`

- **Related Plan Item:** T-4
- **Topic:** Clarity vs Brevity
- **Reviewer Position:** Prefer the chosen direction (enum)
- **Advice:** Keep `Cause::Interrupt(u64) | Cause::Exception(u64)`.
- **Rationale:** P2 immediately adds H-ext-specific causes (VirtualSupervisorExternal = 12, VirtualSupervisorTimer = 10, etc. for interrupts; guest-page-fault = 20/21/23 for exceptions) where pattern-matching on a typed enum reads dramatically better than `if scause >> 63 == 0 && scause & 0xff == ...`. The three-line "raw masks" alternative would have to be replaced wholesale in P2 anyway.
- **Required Action:** Adopt. Optional: consider whether the inner type should be `usize` instead of `u64` for cleaner indexing on RV64 — they happen to be the same on this target but `usize` is more idiomatic in a `usize`-typed CSR pipeline.
