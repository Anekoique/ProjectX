# `framework` REVIEW `00`

> Status: Open
> Feature: `framework`
> Iteration: `00`
> Owner: Reviewer
> Target Plan: `00_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking: `0`
- Non-blocking: `9` (0 CRITICAL · 4 HIGH · 3 MEDIUM · 2 LOW)

## Summary

Substantively sound. The PLAN faithfully realises the PRD: every Outcome bullet maps to a Goal or Constraint, the module tree matches the research note's prior-art convergence, the SiFive-test magic values are correct against `xam/xhal/src/platform/qemu_virt/misc.rs`, and the deferred items (traps, H-ext writes, G-stage, heap) line up cleanly with `docs/XVISOR.md` P1–P3. No SPEC contradictions and no CRITICAL design errors. The blocking issues are all addressable in one more iteration: Phase 1's gate command isn't automatable, the cargo + `boot.s` + linker wiring (`build.rs` / `.cargo/config.toml`) is not specified, the top-level Makefile referenced for wrapper targets does not exist in the worktree, and several `## Spec` constraints lean on phase names that won't be meaningful when the block is promoted verbatim to `specs/features/xvisor/framework/SPEC.md`.

---

## Findings

### R-001 `Phase 1 gate is not automatable`

- **Severity:** HIGH
- **Section:** `## Implementation` — Phase 1 "Gate" bullet (lines 188).
- **Problem:** Phase 1's gate is "QEMU boots, hangs at `wfi`, operator quits with Ctrl-A X". A manual Ctrl-A X is not a `make`-able gate. The project standard (`AGENTS.md`: "you must run `make fmt`, `make clippy`, `make run`, and `make test`") presumes every coding-related modification leaves these targets green; a Phase-1 cut that requires an operator to kill QEMU breaks that contract for the duration of the phase.
- **Why it matters:** Without an automatable Phase-1 gate, the PLAN's four-phase ladder collapses to a single landable phase (Phase 2 onwards, once UART writes go through). That's fine in practice but the PLAN currently *claims* four green-able phases, which is misleading.
- **Recommendation:** Either (a) collapse Phase 1 into Phase 2 (build + boot + banner in one landable step) and rename phases accordingly, or (b) relax Phase 1's gate to "`cargo build` + `cargo clippy` succeed; ELF links at `0x80200000` (verified via `objdump -h | grep .text.boot`); no QEMU run." Pick (b) only if landing Phase 1 separately gives a meaningfully smaller diff.

### R-002 `Cargo + boot.s + linker.ld wiring is unspecified`

- **Severity:** HIGH
- **Section:** `## Architecture` (lines 41–67) and `## Implementation` Phase 1 (lines 179–188).
- **Problem:** A standalone `xvisor/Cargo.toml` with `target = riscv64gc-unknown-none-elf`, an external `linker.ld`, and a raw `.s` (not embedded `global_asm!`) requires *one* of: (a) `xvisor/.cargo/config.toml` with `[build] target = "riscv64gc-unknown-none-elf"` and `[target.riscv64gc-unknown-none-elf] rustflags = ["-C", "link-arg=-Txvisor/linker.ld"]`; or (b) a `build.rs` that emits the `cargo:rustc-link-arg=-Txvisor/linker.ld` directive and `cargo:rerun-if-changed=src/boot.s`; or (c) embedding `boot.s` via `core::arch::global_asm!(include_str!("boot.s"))` from a Rust file and dropping the standalone `.s`. The PLAN files (Architecture tree, Phase 1) mention none of these. The existing precedent in this repo (`xam/xhal/build.rs`) uses `build.rs`.
- **Why it matters:** "Cargo will pick up `linker.ld` and `boot.s`" is not true by default. Without one of these mechanisms specified, Phase 1 will hit "linker can't find `_start`" or "boot.s is ignored", and the executor will improvise — which may diverge from this PLAN's stated Architecture.
- **Recommendation:** Add a row to the Architecture tree for either `xvisor/.cargo/config.toml` or `xvisor/build.rs`, and add one bullet under Phase 1 specifying which mechanism wires the linker script and assembles `boot.s`. State the chosen approach as a Constraint so it can't drift.

### R-003 `Top-level Makefile referenced in Phase 1/4 does not exist`

- **Severity:** HIGH
- **Section:** `## Implementation` — Phase 1 line 187, Phase 4 lines 213–215.
- **Problem:** Phase 1 says "Add top-level wrapper targets `make xvisor` / `make xvisor-run` to `/Users/anekoique/ProjectX/Makefile` (or equivalent — confirm during impl)". There is no top-level Makefile in the worktree (`ls /Users/anekoique/ProjectX/.ark/worktrees/feat/xvisor-framework/Makefile` → not found). Sibling crates (`xemu/`, `xlib/`, `xam/`, `resource/`) each ship their own Makefile and are invoked directly. The PRD's Outcome bullet says "top-level Makefile target `make xvisor`" — but there is nowhere to put that target today.
- **Why it matters:** The PRD's user-facing contract (`make xvisor`) depends on a file that does not exist; the PLAN's escape hatch ("or equivalent — confirm during impl") punts a design decision into execution. That's how PLAN drift happens.
- **Recommendation:** Pick one and state it as a Constraint: either (a) create a new top-level `Makefile` in this PLAN with `xvisor` / `xvisor-run` targets delegating to `xvisor/Makefile` — and note this is a brand-new top-level file; or (b) drop the "top-level" framing and have the PRD/SPEC say "`make -C xvisor run`" and "`cd xvisor && make run`" instead. Either is defensible; the PLAN must pick.

### R-004 `## Spec leans on roadmap phase names that won't survive verbatim promotion`

- **Severity:** HIGH
- **Section:** `## Spec` — Data Structure doc-comments (lines 73, 82, 92), API Surface comments (lines 105, 117), Constraints C-13 / C-14 (lines 142–143).
- **Problem:** The Spec block — which is promoted verbatim to `specs/features/xvisor/framework/SPEC.md` — uses phrases like "P1 trap.S indexes by `offset_of!`" (line 82), "P1 caller" (line 113), "P1 defines body in trap.S" (line 117), "P3" / "P4" in C-14 (line 143), and "P0 has..." throughout. The promoted SPEC is the durable contract; six months from now a reader opening `specs/features/xvisor/framework/SPEC.md` will not know what P1 or P3 means without holding `docs/XVISOR.md` open.
- **Why it matters:** Mandatory rejection rule per the reviewer rubric: `## Spec` must be self-contained. The block isn't currently — it inherits roadmap vocabulary the SPEC reader cannot see. The Spec is still mostly readable (a reader can guess "future phase"), so this is HIGH not CRITICAL, but it needs cleanup before promotion.
- **Recommendation:** Replace each "P1"/"P3"/"P4" reference inside the `## Spec` block with the durable substance:
  - line 82: "populated by trap_entry assembly when traps are added" instead of "in P1".
  - line 113 / 117: "future trap-entry caller" instead of "P1 caller / P1 defines body".
  - C-14: "one-line `//!` doc comments naming the future feature that fills them in" instead of "naming the phase that fills them in".
  - C-13: "secondary harts spin in OpenSBI HSM until future multi-hart bring-up wakes them" instead of "P0 has".
  Trade-offs / Runtime / Implementation sections (which are NOT promoted) may keep phase nomenclature.

### R-005 `TrapFrame size choice (32 GPRs vs 31) buried in V-UT-2`

- **Severity:** MEDIUM
- **Section:** `## Spec` — Data Structure (line 86) and `## Validation` — V-UT-2 (line 236).
- **Problem:** The `TrapFrame` reserves `regs: [usize; 32]` — i.e., includes x0 (the hard-wired zero register). The Spec doesn't surface this as a design choice; only V-UT-2 reveals the slot count. Some Rust hypervisors (hvisor among them) use `[usize; 31]` and leave x0 implicit, which saves 8 bytes per frame and aligns the regs array with x1..x31 indexing. Either choice is fine; "32 with x0 always zero" preserves natural indexing (`frame.regs[rd]` works for any encoded `rd`), which is the better choice. But that rationale lives nowhere in the document.
- **Why it matters:** Trap frame layout is the kind of contract that gets retrofitted painfully (the research note already calls this out for hvisor / hypocaust-2). Burying the 32-vs-31 decision in a const-assert means future readers will wonder "should we drop x0?" — a costly question once `trap.S` and Rust handlers index by offset.
- **Recommendation:** Add a Constraint:
  `C-N: TrapFrame.regs has 32 slots (x0..x31); x0's slot is always zero and preserved to keep frame[rd] indexing natural — xvisor/src/arch/riscv/trap.rs.`
  Keep V-UT-2 as-is. Drop the "32 GPRs + sepc/scause/stval/sstatus" comment from the test since the Constraint will carry the meaning.

### R-006 `Failure Flow item 3 admits a silent hang without a corresponding Constraint`

- **Severity:** MEDIUM
- **Section:** `## Runtime` — Failure Flow item 3 (line 167).
- **Problem:** "Trap before `stvec` is wired → silent hang" is honest but unprotected. Non-goal NG-1 forbids P0 from wiring `stvec` — so any unintended exception (an illegal instruction in early Rust, a misaligned access in `boot.s`, etc.) traps to address `0`, fetches garbage, double-faults, triple-bounces, and the operator stares at a dead VM. C-3's `misa.H` check eliminates the most common cause but not all causes.
- **Why it matters:** This is the highest-likelihood operational hazard for P0 (it is precisely the failure mode the research note flags as Medium-likelihood for P1's stack-reentrancy line). Leaving it as a one-line Failure Flow entry under-sells the risk.
- **Recommendation:** Either (a) add a Constraint that pins the hazard: `C-N: P0 leaves stvec at reset value; any unintended HS-mode trap before P1 is a documented unrecoverable hang — xvisor/README.md` and document it in the README; or (b) point `stvec` at a one-instruction `wfi` trampoline emitted in `boot.s` so unintended traps loop visibly rather than triple-bounce. Option (b) is three lines of asm and meaningfully better — recommended.

### R-007 `C-14 module stubs validation is circular`

- **Severity:** MEDIUM
- **Section:** `## Validation` — Acceptance Mapping row for C-14 (line 277) and V-UT-2 / V-UT-3.
- **Problem:** C-14 says "stub modules committed with doc comments". The Acceptance Mapping points to V-UT-2 / V-UT-3, with the reasoning "compile only if `mod` declarations are present". But V-UT-2 / V-UT-3 are const-asserts inside `arch/riscv/`, not `mm/` / `vcpu/` / `vm/` / `sbi/`. The fact that `mod mm;` is reachable from `main.rs` is enforced by `cargo build`, not by V-UT-2. There's no direct validation that each stub module actually carries its `//!` doc comment with the named phase.
- **Why it matters:** A constraint with no honest validation will quietly slip. Reviewers in P1+ will read the SPEC, look at `mm/mod.rs`, and find a bare `//!` with the wrong phase named — or no doc comment at all — and have nowhere to point to.
- **Recommendation:** Add V-UT-N or V-IT-N: a `tests/stubs.rs` integration test (host-runnable) that reads each stub `mod.rs` and asserts `//! ` is the first non-empty line, OR a `clippy::missing-docs-in-private-items` lint enforced under `-D warnings`. The latter is cheaper; pin it via `#![deny(missing_docs)]` at the crate root.

### R-008 `C-11 wording: "#![no_std] forbids extern crate alloc"`

- **Severity:** LOW
- **Section:** `## Spec` — Constraints (line 140).
- **Problem:** Strictly, `#![no_std]` does *not* forbid `extern crate alloc`; the two are independent. The actual constraint is the explicit absence of `extern crate alloc` in `xvisor/src/main.rs` plus the absence of an `alloc`-providing dependency in `xvisor/Cargo.toml`.
- **Why it matters:** Pure wording precision. A reader expecting `cargo deny`-style enforcement from `#![no_std]` will be confused.
- **Recommendation:** Reword:
  `C-11: P0 has no heap; xvisor/Cargo.toml has no allocator dependency and xvisor/src/main.rs has no extern crate alloc — xvisor/Cargo.toml, xvisor/src/main.rs.`

### R-009 `Banner format literal in C-15 has a brittle space character`

- **Severity:** LOW
- **Section:** `## Spec` — Constraints C-15 (line 144) and V-IT-1 regex (line 241).
- **Problem:** C-15 says the banner format is `xvisor: hello from HS-mode (hartid={n}, dtb=0x{addr:x})\n` — note the space after the comma in `, dtb=`. V-IT-1's regex is `^xvisor: hello from HS-mode \(hartid=0, dtb=0x[0-9a-f]+\)$` — matches. PRD Outcome line 16 says "a banner line on UART containing the literal string `xvisor: hello from HS-mode` followed by hartid and DTB pointer" — no exact format specified. Risk: a future editor "polishes" the format (drops a space, adds a colon, capitalises HS) and V-IT-1 silently fails with a confusing regex-mismatch error.
- **Why it matters:** Minor — string-format brittleness — but easy to fix.
- **Recommendation:** Either (a) anchor C-15 on the regex itself: `C-15: Banner matches regex ^xvisor: hello from HS-mode \(hartid=\d+, dtb=0x[0-9a-f]+\)$ — xvisor/src/main.rs.`; or (b) move the format literal into a `const BANNER_FMT: &str = ...;` and reference it from both the print site and the test.

---

## Trade-off Advice

### TR-1 `OpenSBI fw_jump (HS-mode entry) vs -bios none (M-mode entry)`

- **Related Plan Item:** `T-1`
- **Topic:** Compatibility vs Clean Design
- **Reviewer Position:** Prefer A (current choice)
- **Advice:** Adopt. The PLAN picks `-bios default`, which matches the research note's prior-art survey (salus, hvisor, hypocaust-2 all enter in HS-mode post-OpenSBI), aligns with `docs/XVISOR.md` P0 prose ("OpenSBI sits below xvisor"), and avoids reimplementing the M-mode prelude rvvisor pays for.
- **Rationale:** Type-1 layering is honestly modelled (OpenSBI = M-mode firmware below, xvisor = HS-mode owner above). The "what if we want our own M-mode firmware later" branch is genuinely unlikely — xemu plays that role in P9.
- **Required Action:** Keep with clarification — add to the Trade-offs entry one sentence noting that switching modes is a `_start` branch on the entry-privilege bit, costless if needed.

### TR-2 `Direct MMIO UART vs SBI DBCN early-print`

- **Related Plan Item:** `T-2`
- **Topic:** Layering vs Dependency Surface
- **Reviewer Position:** Prefer A (current choice)
- **Advice:** Adopt. Mirroring `xam/xhal/src/platform/xemu/console.rs` line-for-line is a free port; SBI DBCN would create a runtime dependency on OpenSBI console state that the P5 Linux-passthrough story needs to dismantle anyway.
- **Rationale:** A Type-1 owns its console driver. Threading early-print through SBI is a Type-2-shaped decision.
- **Required Action:** Adopt.

### TR-3 `Commit full module tree in P0 vs incremental add`

- **Related Plan Item:** `T-3`
- **Topic:** Naming Discipline vs YAGNI
- **Reviewer Position:** Prefer A (current choice)
- **Advice:** Adopt. The research note's prior-art table shows every credible Rust hypervisor (salus, hvisor, hypocaust-2) converges on the same eight-bucket taxonomy; pre-committing the names eliminates the SPEC churn the note flags as the dominant cost in the prior art.
- **Rationale:** Empty `mod.rs` files cost ~8 lines and zero cognitive overhead; renames in P1–P6 cost a full review cycle each.
- **Required Action:** Adopt. Address R-007's validation gap so the constraint actually has a verifier.

### TR-4 `SiFive-test direct vs SBI SRST for host halt`

- **Related Plan Item:** `T-4`
- **Topic:** Honest Layering vs Convention
- **Reviewer Position:** Prefer A (current choice)
- **Advice:** Adopt. The PLAN's reasoning is correct: a Type-1 owns the machine and shouldn't ask OpenSBI to shut it down. Observable QEMU exit is identical; the code-path delta is purely about who's responsible.
- **Rationale:** Locks the right mental model for P4 (xvisor *provides* SRST to its guest) — guest's SBI SRST goes through xvisor's SBI dispatch, xvisor's host halt goes through SiFive-test directly. Two paths, two owners; one wouldn't have made the layering visible.
- **Required Action:** Adopt.

### TR-5 `boot.s (separate file) vs naked_asm! in Rust`

- **Related Plan Item:** `T-5`
- **Topic:** Auditability vs Single-language Discipline
- **Reviewer Position:** Prefer A (current choice)
- **Advice:** Adopt with one clarification: the PLAN should explicitly state that this is a *new* assembly file (green-field), and therefore the project's "no modifying assembly without permission" rule (per executor's MEMORY.md) does not gate it. The PLAN does say this in T-5 prose; lifting it into a Constraint would close the loop.
- **Rationale:** A standalone `boot.s` is auditable in one view (the entire boot dance — misa check, DTB stash, BSS zero, tp setup — co-located), matches salus / hvisor / hypocaust-2, and is what `docs/XVISOR.md` line 71 already names in the prescribed layout. `naked_asm!` would split the dance across `mod.rs` and `boot.rs`.
- **Required Action:** Adopt. Optionally add a Constraint: `C-N: boot.s is green-field new assembly created by this task; it does not modify any existing .S file — xvisor/src/boot.s.`

### TR-6 `Static PerCpu array + tp vs per-stack-top PerCpu slot`

- **Related Plan Item:** `T-6`
- **Topic:** Indexing Simplicity vs Cache-locality
- **Reviewer Position:** Prefer A (current choice)
- **Advice:** Adopt. The static-array + `tp = &PER_CPU[hartid]` choice is the single-source-of-truth pattern xemu's `multi-hart` SPEC (`HartId(u32)` + `Vec<Core>`) already uses; carrying that vocabulary into HS-mode keeps the two privilege levels mentally aligned.
- **Rationale:** Per-stack-top slots are slightly more cache-local but force `tp` to be derived (sp masked to stack base, then offset). For `MAX_HARTS = 1` in P0 this is moot; for P6+ multi-hart, the array shape is still simpler.
- **Required Action:** Adopt.
