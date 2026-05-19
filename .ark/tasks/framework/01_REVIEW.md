# `framework` REVIEW `01`

> Status: Closed
> Feature: `framework`
> Iteration: `01`
> Owner: Reviewer
> Target Plan: `01_PLAN.md`
> Scope: Plan correctness · Spec alignment · Design soundness · Validation adequacy · Trade-off advice

---

## Verdict

- Decision: Approved with Revisions
- Blocking: `0`
- Non-blocking: `4` (0 CRITICAL · 0 HIGH · 3 MEDIUM · 1 LOW)

## Summary

The iter-01 PLAN cleanly resolves every CRITICAL/HIGH from `00_REVIEW.md`. The Response Matrix covers R-001..R-009 and TR-1..TR-6 with truthful, verifiable claims; the four-phase ladder collapses to three automatable phases; `build.rs` + `.cargo/config.toml` are now first-class; the top-level Makefile framing is dropped; every `P0..P4` reference inside the promoted `## Spec` block is replaced with durable vocabulary; the `stvec` `wfi` trampoline lands as a new Constraint, Failure Flow entry, and Trade-off T-7; `#![deny(missing_docs)]` and `const BANNER_FMT` are pinned; the `extern crate alloc`/`no_std` wording is corrected. The Spec block (lines 66–197) is genuinely self-contained — grepping it for `P[0-9]` / `phase` returns no hits, and a SPEC-only reader can follow every Constraint without external context. The 20-Constraint count matches the Summary claim; every Goal G-1..G-5 maps to ≥1 Validation row. Remaining issues are all non-blocking: a semantic gap in `#![deny(missing_docs)]`'s ability to catch missing private-module `//!` comments, a minor overstatement in the "mirrors `xam/xhal/build.rs`" claim, a NG-1 wording tension with C-11, and a low-stakes regex anchor question. None require restructuring.

---

## Findings

### R-001 `#![deny(missing_docs)]` does not enforce private-module doc comments by default

- **Severity:** MEDIUM
- **Section:** `## Spec` C-19 (line 196), `## Validation` V-UT-4 (line 289), Response Matrix R-007 row (line 54).
- **Problem:** The built-in `missing_docs` lint fires only on **public** items. The stub `mod mm;` / `mod vcpu;` / `mod vm;` / `mod sbi;` declarations in a `bin` crate are private by default; their `mod.rs` files are also private modules. With only `#![deny(missing_docs)]` at the crate root, omitting the `//!` doc comment on a private stub will **not** trip clippy or rustc. V-UT-4 promises "fails if any stub `mod` lacks a `//!` doc comment", which is the validation behind C-19. As written, that promise is unenforced for private stubs — which is exactly the case the PLAN cares about.
- **Why it matters:** R-007 is the only Validation row pointing at C-19. If the lint silently no-ops on private stubs, C-19 becomes another "validated by code review" constraint — which was the gap R-007 set out to close in the first place.
- **Recommendation:** Either (a) make the stub modules `pub mod`, so `missing_docs` actually fires; or (b) add `#![warn(clippy::missing_docs_in_private_items)]` (with `-D warnings` from `make clippy` promoting it to deny) alongside `#![deny(missing_docs)]`. Option (b) is one extra crate-level attribute and a one-line tweak to V-UT-4's wording. Pin whichever you pick into C-19 so the validation is honest.

### R-002 "Mirrors `xam/xhal/build.rs`" overstates the resemblance

- **Severity:** LOW
- **Section:** `## Implementation` Phase 1 build.rs bullet (line 238), Response Matrix R-002 row (line 49).
- **Problem:** `xam/xhal/build.rs` does **not** use the `cc` crate or emit `cargo:rustc-link-arg`. It reads a `linker.lds.S` template, substitutes `%ARCH%` / `%KERNEL_BASE%` / `%MEM_SIZE%` from `xconfig`, and writes the result into the cargo `OUT_DIR`. The actual link-script wiring in xam happens via the linker's auto-discovery of the generated `linker_<platform>.lds` next to the binaries. The proposed `xvisor/build.rs` is a different shape (emit `cargo:rustc-link-arg=-Txvisor/linker.ld` + `cc::Build` for `boot.s`). The substance is fine — both `cc` and `cargo:rustc-link-arg` are idiomatic — but "mirrors `xam/xhal/build.rs`" misleads a reader who opens both files expecting the same pattern.
- **Why it matters:** Pure framing. The chosen mechanism is correct and standard; the precedent claim is just inaccurate. A future maintainer who reads C-2 and then opens `xam/xhal/build.rs` for the template will be confused.
- **Recommendation:** Drop "mirroring `xam/xhal/build.rs`" from the Phase 1 bullet (line 238) and from Response Matrix R-002. Replace with a direct, self-standing description: "emits `cargo:rustc-link-arg=-Txvisor/linker.ld` + `cargo:rerun-if-changed=src/boot.s`, and uses the `cc` crate (build-dependency) to assemble `boot.s` into a static archive linked into the binary." Keep C-2 as-is; only the precedent attribution needs trimming.

### R-003 NG-1 wording understates what C-11 actually does

- **Severity:** MEDIUM
- **Section:** `## Spec` NG-1 (line 78), C-11 (line 188).
- **Problem:** NG-1 says "No trap entry, no `TrapFrame` save / restore code — only a one-instruction `wfi` trampoline in `stvec`." C-11 says "`boot.s` installs a one-instruction `wfi` trap trampoline in `stvec` before calling `rust_main`". The two are consistent in spirit (single `wfi`, no save/restore) but NG-1's phrasing reads as if installing a trampoline is itself a trap entry being added. A reader scanning Non-goals will hit "only a one-instruction wfi trampoline" and ask "is that a trap entry or isn't it?" — the answer is meant to be "no, it's a parking pad with no register save", but NG-1 doesn't say so explicitly.
- **Why it matters:** Non-goals are SPEC-load-bearing — they're what later phases must not re-litigate. An ambiguous Non-goal invites a future SPEC author to argue "P0 already installed a stvec target, so my new save/restore trampoline is just an extension." That's the kind of drift the iter-00 review was guarding against.
- **Recommendation:** Tighten NG-1 to:
  `NG-1: No trap-entry assembly, no TrapFrame save/restore — the stvec target installed by boot.s is a single-instruction parking pad (wfi + self-loop) with no register save or Rust dispatch.`
  Leave C-11 unchanged; the new NG-1 wording makes its scope unambiguous.

### R-004 V-IT-1 regex anchors vs BANNER_FMT trailing newline

- **Severity:** MEDIUM
- **Section:** `## Spec` API Surface (line 147), `## Validation` V-IT-1 (line 293).
- **Problem:** `BANNER_FMT` is `"xvisor: hello from HS-mode (hartid={}, dtb=0x{:x})\n"` — ends with a literal newline. V-IT-1's regex is `^xvisor: hello from HS-mode \(hartid=0, dtb=0x[0-9a-f]+\)$`. With most line-oriented matchers (`grep -E`, `ripgrep`, Python `re.search(..., re.MULTILINE)`) `$` matches before the `\n`, so the regex will match. But if the `make run` recipe pipes captured stdout into a non-line-oriented matcher (`re.fullmatch`, a host-side Rust regex with `multi_line(false)`), the trailing `\n` will cause `$` to mismatch. The PLAN doesn't pin the matching tool.
- **Why it matters:** V-IT-1 is the load-bearing test for G-1, G-2, G-3, G-4, G-5, and seven of the twenty Constraints. A test that's mode-sensitive to its host tool will fail confusingly in CI on whoever runs `make run` on an unfamiliar machine. The remediation is trivial; leaving it unstated invites the executor to improvise.
- **Recommendation:** Add one sentence to V-IT-1 (or to C-20) pinning the matcher: e.g., "`make run` greps QEMU stdout via `grep -E -- '<regex>'`" (line-oriented, `$` matches before `\n`, regex as written works) **or** "regex is applied with multi-line mode enabled". Either is fine; the choice belongs in the PLAN, not the executor's head.

---

## Trade-off Advice

### TR-1 `stvec wfi trampoline vs leaving stvec at reset`

- **Related Plan Item:** `T-7` (new in iter-01)
- **Topic:** Operational Safety vs Minimal Asm Surface
- **Reviewer Position:** Prefer A (current choice — install the trampoline)
- **Advice:** Adopt. T-7 's reasoning is sound: three lines of asm in `boot.s` (which is already being written) buys a visible-park failure mode instead of a triple-bouncing dead VM. The alternative — "document a silent hang in README" — is a worse trade in every dimension except line-count.
- **Rationale:** The trampoline does not violate NG-1 (single `wfi`, no save/restore vector, no Rust dispatch). It also models the right hygiene for P1+: future trap entry replaces the trampoline by overwriting `stvec` rather than installing it from scratch, so the migration cost is zero. The only refinement is R-003 above — tighten NG-1's wording so the trampoline's scope is unambiguous.
- **Required Action:** Adopt with NG-1 wording tightening per R-003.

### TR-2 `Cargo build mechanism: build.rs + cc crate vs .cargo/config.toml rustflags`

- **Related Plan Item:** `T-2` (TR-2 in iter-00 still stands)
- **Topic:** Build-Time Discoverability vs Single-Config Convention
- **Reviewer Position:** Prefer A (current choice — `build.rs` for link-arg + `cc` for asm)
- **Advice:** Adopt. The chosen approach (build.rs emits `cargo:rustc-link-arg=-Txvisor/linker.ld` + `cargo:rerun-if-changed=src/boot.s`, `cc::Build` assembles `boot.s` into a static lib linked into the binary) is the standard idiom for a bare-metal Rust crate with an external linker script + hand-written asm. It works regardless of whether the user invokes `cargo build` or `make run`.
- **Rationale:** The `.cargo/config.toml`-only alternative (rustflags `link-arg=-Txvisor/linker.ld` + `core::arch::global_asm!`) would also work, but folds the assembly into Rust source and forces `global_asm!`'s string-include semantics — uglier diff against `xam/xhal`'s separate linker.lds.S. Keep R-002's framing fix in mind: the implementation is fine; the precedent claim isn't.
- **Required Action:** Adopt. Drop the "mirrors `xam/xhal/build.rs`" framing per R-002.

### TR-3 `Constraint numbering: renumber on every add vs append-only`

- **Related Plan Item:** No explicit T-N; surfaced by the iter-00 → iter-01 diff.
- **Topic:** Stability vs Linearity
- **Reviewer Position:** Neutral
- **Advice:** Note for future iterations. iter-01 renumbered constraints when inserting C-2/C-3/C-8/C-11/C-15 (so what was iter-00's C-11 became iter-01's C-12, etc.). That's defensible for an unpromoted Spec, but on the deep-tier commit the SPEC freezes — at that point append-only is the durable rule, because external docs may cite "C-12". The Response Matrix correctly flags the renumbering ("Constraint formerly C-11"), so the audit trail is intact.
- **Rationale:** No action this iteration; flagging the convention for the executor's awareness. Once `specs/features/xvisor/framework/SPEC.md` is committed, future changes to that SPEC's constraints should be append-only (C-21, C-22, ...) rather than renumbering — otherwise CHANGELOG citations rot.
- **Required Action:** No PLAN change required. Keep iter-01's numbering as the final pre-promotion shape.

---
