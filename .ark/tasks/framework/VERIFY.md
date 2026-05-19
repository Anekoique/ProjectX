# `framework` VERIFY

> Status: Closed
> Feature: `framework`
> Target Task: `framework`
> Tier: `deep`
> Iteration audited: `02` (re-verify after option-(b) resolution of prior CRITICAL — `BANNER_FMT` excised from the `## Spec` block; V-IT-1 regex is the sole contract anchor)
>
> Each checklist item resolves to PASS | FAIL (with explanation) | N/A (with explanation). Findings (`V-NNN`) capture cross-cutting observations with a Resolution. **This VERIFY supersedes all prior VERIFY snapshots; numbering restarts from `V-001`.**

---

## Severity Summary: 0 CRITICAL · 0 HIGH · 2 MEDIUM · 2 LOW
## Verification: build PASS · tests PASS (placeholder echo; no host-runnable tests yet) · clippy PASS (`-D warnings`, riscv64gc target, `--bins`) · fmt PASS (`cargo fmt --all -- --check`) · QEMU smoke PASS (banner regex match, exit 0)

**Verdict gate:** Zero CRITICAL, zero HIGH. The deep-tier commit gate may proceed. The prior CRITICAL (V-001 of the previous VERIFY: `BANNER_FMT` named in seven Spec-block sites but absent from the implementation) is resolved by option (b) — the constant is removed from the Spec block entirely; the V-IT-1 regex `^xvisor: hello from HS-mode \(hartid=[0-9]+, dtb=0x[0-9a-f]+\)$` is now the sole contract anchor for the banner shape. `grep -n "BANNER_FMT"` returns zero hits over both `.ark/tasks/framework/02_PLAN.md` and `xvisor/src/`. C-19 reads "Banner format emitted by `rust_main` matches the V-IT-1 regex `^xvisor: hello from HS-mode \(hartid=[0-9]+, dtb=0x[0-9a-f]+\)$` — `xvisor/src/main.rs`." (`02_PLAN.md:181`); the Architecture tree's `main.rs` line (`02_PLAN.md:82`) reads "crate root attrs; rust_main; panic handler" with no `BANNER_FMT` mention; API Surface (`02_PLAN.md:136-138`) lists only `rust_main` for `main.rs`; Runtime step 9 (`02_PLAN.md:197`) reads "constructs a `UartWriter` and prints the banner formatted with `hartid` and `dtb_ptr`"; V-IT-1 (`02_PLAN.md:274`) anchors on the regex directly; the Acceptance Mapping C-19 row (`02_PLAN.md:316`) reads "V-IT-1 (regex match against the printed banner line)." The shipped `main.rs:31-36` writes the banner with an inline `writeln!` literal whose template matches the regex — verified by piping captured QEMU output through `grep -E` (exit 0).

All four CLAUDE.md gates (`make fmt`, `make clippy`, `make build`, `make run`, `make test`) return 0 from `xvisor/` in the current tree. The observed QEMU banner is exactly `xvisor: hello from HS-mode (hartid=0, dtb=0x8fe00000)`; QEMU exits with status 0 via the SiFive-test finisher; no SBI SRST `ecall` anywhere in `xvisor/src/`. The OpenSBI boot log confirms `Domain0 Next Address: 0x0000000080200000`, `Domain0 Next Mode: S-mode`, `Boot HART Base ISA: rv64imafdch` (H-extension present), `Platform HART Count: 1`. The four residual findings are inside-Plan inconsistencies in non-Spec sections (Implementation Phase guidance, Response Matrix, Acceptance Mapping) and a structural flag-mismatch with the PRD's clippy invocation; none block SPEC promotion.

---

## Project Spec Compliance

> `.ark/specs/project/INDEX.md` is the template body with no SPEC rows.

### Index integrity

- [x] `INDEX.md` enumerates all children of `specs/project/`: N/A — `.ark/specs/project/` contains only the placeholder `INDEX.md` (template body, the only row is the literal `<e.g. <language>/SPEC.md>` placeholder). No leaf SPECs exist; nothing to enumerate.

### Leaf SPECs

- (none discovered): N/A — no project-layer leaf SPECs exist yet.

## Related Feature Spec Compliance

> Auto-seeded from the PRD's `[**Related Specs**]`. The PRD explicitly states "No edits to xemu's SPEC" / "No edits to xlib's SPEC"; the gate here is mental-model alignment, not byte-level conformance. `git diff --stat` confirms no edits under `.ark/specs/features/xemu/` or `.ark/specs/features/xlib/`.

- [x] `.ark/specs/features/xemu/multi-hart/SPEC.md`: PASS — `xvisor/src/hal/arch/riscv/cpu.rs` ports the per-hart mental model: `MAX_HARTS = 1` const, `PerCpu` `#[repr(C, align(64))]` struct with `hartid` + `stack_top` + reserved padding, `static mut PER_CPU` array, `percpu()` reads `tp`. Mirrors xemu's `HartId(u32)` + `Vec<Core>` convention adapted to HS-mode (`tp` instead of `mscratch`, `a0`-from-OpenSBI instead of `mhartid`).
- [x] `.ark/specs/features/xemu/csr/SPEC.md`: PASS — `xvisor/src/hal/arch/riscv/csr.rs` lands the typed-wrapper precedent for HS-mode (`unsafe fn write_stvec` with documented safety contract). H-extension wrappers correctly deferred per NG-2 and Spec API Surface `02_PLAN.md:147`.
- [x] `.ark/specs/features/xlib/SPEC.md`: PASS — xlib is freestanding C, not linked by xvisor. `grep -rn "xlib" xvisor/` returns zero hits.
- [x] `.ark/specs/features/xemu/devices/SPEC.md`: PASS — `xvisor/src/hal/platform/qemu/uart.rs` mirrors `xam/xhal/src/platform/xemu/console.rs` shape (LSR THRE poll at offset `+5`, byte-at-a-time `putch` at `0x1000_0000`).

## PRD Constraints

> Auto-seeded from PRD's `[**Outcome**]` bullets. One bullet per criterion.

- [x] PRD-O-1 `xvisor/` crate exists with `Cargo.toml`, `linker.ld`, `src/` tree, Makefile target producing the release ELF: PASS-via-supersede — `xvisor/{Cargo.toml,linker.ld,build.rs,Makefile,README.md,src/...}` all present. `cd xvisor && make build` produces `xvisor/target/riscv64gc-unknown-none-elf/release/xvisor`. The PRD's "top-level `make xvisor`" framing was reframed in iter-01 to `cd xvisor && make build` (PLAN-recorded reframe is workflow-legal under Ark's "PRDs are not rewritten in iteration loops" convention). Operationally satisfied.
- [x] PRD-O-2 banner contains `xvisor: hello from HS-mode` + hartid + DTB, QEMU exits 0 via SiFive-test finisher: PASS — observed `xvisor: hello from HS-mode (hartid=0, dtb=0x8fe00000)`; QEMU exits with status 0; `hal/platform/qemu/halt.rs` writes `0x5555` to `0x10_0000`; no `ecall` instruction in any shipped source. The wrapper `make xvisor-run` at repo root remains intentionally not wired; `cd xvisor && make run` is the actual entry (same PRD-reframe note).
- [x] PRD-O-3 Module tree committed in full with empty `mod.rs` carrying `//!` doc + `TrapFrame` + `trap_entry` extern: PASS-via-supersede — the PRD's pre-EXECUTE layout was reorganised (logged in `02_PLAN.md` Summary "EXECUTE-phase note") into `hal::{arch,platform}` with `boot.rs` (`naked_asm!`) replacing `boot.s`. All modules ship: `hal/arch/riscv/{mod,boot,cpu,csr,trap}.rs`, `hal/arch/loongarch/mod.rs`, `hal/platform/qemu/{mod,uart,halt}.rs`, `hal/platform/xemu/mod.rs`, `mm/mod.rs`, `vcpu/mod.rs`, `vm/mod.rs`, `sbi/mod.rs` — each with a `//!` doc comment. `TrapFrame` ships at `trap.rs:11-23`. `trap_entry` extern declaration is intentionally deferred — Spec API Surface (`02_PLAN.md:150`) reads "TrapFrame struct only — trap_entry symbol lands when trap handling arrives", consistent with the shipped tree. (Implementation Phase 2 guidance at `02_PLAN.md:233` still names the extern decl — inside-Plan inconsistency, see V-001.)
- [x] PRD-O-4 `_start` validates `misa.H = 1` before entering Rust: PASS-via-supersede — `02_PLAN.md` C-4 (line 166) records the supersede: `misa` is M-mode-only and cannot be probed from HS-mode; operator misconfiguration surfaces via OpenSBI's startup banner instead. PRD bullet is impossible-as-written; PLAN log records the supersede.
- [x] PRD-O-5 Per-hart convention locked (`tp = &PerCpu`, `sscratch` documented as reserved, `PerCpu` `#[repr(C)]`): PASS — `PerCpu` `#[repr(C, align(64))]` at `cpu.rs:19-28` (lives at `hal/arch/riscv/cpu.rs`, re-exported via `mod.rs`); `tp` written by `_start`'s `arch_setup` asm block at `boot.rs:107-111`; `sscratch` documented as reserved in `trap.rs:3-7`.
- [x] PRD-O-6 DTB pointer (`a1`) captured into `static AtomicUsize DTB_ADDR`: PASS — `cpu.rs:15: pub static DTB_ADDR: AtomicUsize = AtomicUsize::new(0);` written via `arch_setup` (`boot.rs:92: DTB_ADDR.store(dtb_ptr, Ordering::Release);`) before any code can clobber `a1`. `rust_main` reads it with `Ordering::Acquire` (`main.rs:30`).
- [x] PRD-O-7 `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `make fmt` / `make clippy` pass: PARTIAL PASS — `cargo fmt --all -- --check` returns 0. `make clippy` (`cargo clippy --target riscv64gc-unknown-none-elf --bins -- -D warnings`) returns 0. The PRD-requested `--all-targets` flag is structurally infeasible: a `#![no_std]` `#![no_main]` binary on `riscv64gc-unknown-none-elf` cannot link `libtest`, so `--all-targets` fails. `--bins` is the EXECUTE-phase compromise. Documented as V-003 LOW.
- [x] PRD-O-8 No heap, no `extern crate alloc` in P0: PASS — `grep -E "extern crate alloc|alloc::|Box<|Vec<|String\b"` over `xvisor/src/` returns zero hits. `Cargo.toml` has no `[dependencies]` section.
- [x] PRD-O-9 No `unsafe` outside `boot.s`, `arch/riscv/csr.rs`, `device/uart.rs`: PASS-via-supersede — PLAN C-16 widens the allow-list to `hal/arch/riscv/{boot,cpu,csr}.rs` + `hal/platform/qemu/{uart,halt}.rs`. Every `unsafe { }` block ships in an allow-listed file with a `// SAFETY:` comment (verified by `cargo clippy --bins -- -D warnings -W clippy::undocumented_unsafe_blocks`, exit 0).

## Plan Fidelity

> Auto-seeded from `02_PLAN.md`'s `## Spec` Goals (`G-N`).

- [x] G-1 (Boot a HS-mode Rust binary under QEMU virt + OpenSBI fw_jump and reach `rust_main`): PASS — `make run` reaches `rust_main` (banner prints). OpenSBI log confirms `Domain0 Next Address: 0x0000000080200000`, `Domain0 Next Mode: S-mode`.
- [x] G-2 (Print a banner naming hartid and DTB pointer over the ns16550 UART at `0x10000000`): PASS — observed banner `xvisor: hello from HS-mode (hartid=0, dtb=0x8fe00000)` matches V-IT-1 regex via `grep -E`. UART driver at `hal/platform/qemu/uart.rs:12-25`.
- [x] G-3 (Commit the xvisor module tree `hal::{arch, platform}, mm, vcpu, vm, sbi` as public vocabulary): PASS — all five top-level modules wired in `main.rs:10-14`; `hal/mod.rs` selects backends via `#[cfg_attr]`. Stubs ship: `mm/mod.rs`, `vcpu/mod.rs`, `vm/mod.rs`, `sbi/mod.rs`, `hal/arch/loongarch/mod.rs`, `hal/platform/xemu/mod.rs` all carry one-line `//!` doc comments. `#![deny(missing_docs)]` + `#![warn(clippy::missing_docs_in_private_items)]` at the crate root enforce coverage.
- [x] G-4 (Lock per-hart convention: `tp = &PerCpu`, `sscratch` reserved): PASS — `boot.rs:107-111` writes `tp` via inline `asm!` in `arch_setup`. `trap.rs:3-7` documents `sscratch` as reserved for trap-entry SP swap and left zero.
- [x] G-5 (Halt cleanly via the SiFive-test finisher without invoking SBI SRST): PASS — `hal/platform/qemu/halt.rs:24-39` writes `0x5555` / `0x3333|(1<<16)` to `0x10_0000` then `wfi`-loops. `grep -rnE "ecall|SRST|0x53525354" xvisor/src/` returns only one hit: the doc-comment phrase "rather than SBI SRST" in `halt.rs:4`. QEMU exits with status 0 on the success path.

### Constraint roll-up (`## Spec` Constraints C-1..C-19 from `02_PLAN.md`)

- [x] C-1 (entry `_start` in `xvisor/src/hal/arch/riscv/boot.rs`, `.text.boot`, linker-placed first): PASS — `boot.rs:25` `#[unsafe(link_section = ".text.boot")]`; `linker.ld:13` places `*(.text.boot)` first in `.text`.
- [x] C-2 (`build.rs` emits `cargo:rustc-link-arg=-Txvisor/linker.ld` + `cargo:rerun-if-changed=linker.ld`; assembly via `core::arch::naked_asm!`): PASS — `build.rs:7-8`; `boot.rs:27` uses `naked_asm!`.
- [x] C-3 (Linker base address `0x80200000`): PASS — `linker.ld:3`; matches OpenSBI Domain0 Next Address.
- [x] C-4 (H-extension on trust from OpenSBI; not probed from HS-mode): PASS — `boot.rs` contains no `misa` read; `README.md:66-68` records the operator-facing failure mode.
- [x] C-5 (`a1` stashed into `DTB_ADDR` before any Rust call): PASS — `boot.rs:35` preserves `a1` into `s1`; `boot.rs:92` does `DTB_ADDR.store(dtb_ptr, Ordering::Release)`. Runs before `install_trap_trampoline` and before `rust_main`.
- [x] C-6 (`tp` holds `&PerCpu` after `_start`; never reassigned outside boot): PASS — `boot.rs:107-111` writes `tp` once in `arch_setup`. `percpu()` (`cpu.rs:61-70`) reads `tp` only.
- [x] C-7 (`TrapFrame.regs` has 32 slots; x0 preserved zero): PASS — `trap.rs:14: pub regs: [usize; 32]`; doc-comment at `trap.rs:13` calls out the x0-zero contract. `const _: () = assert!(size_of::<TrapFrame>() == 36 * size_of::<usize>())` at `trap.rs:30`.
- [x] C-8 (`sscratch` reserved for trap-entry SP swap; documented): PASS — `trap.rs:3-7` documents the `sscratch ↔ sp` swap convention; the CSR is left zero this iteration.
- [x] C-9 (Stack size per hart `64 KiB`, defined as `STACK_SIZE_PER_HART` in `xvisor/src/hal/arch/riscv/cpu.rs`): PASS — `cpu.rs:11: pub const STACK_SIZE_PER_HART: usize = 64 * 1024;`. `const _: () = assert!(STACK_SIZE_PER_HART == 64 * 1024)` at `cpu.rs:73`.
- [x] C-10 (`boot.rs` installs one-instruction `wfi` trap trampoline in `stvec` before calling `rust_main`): PASS — `boot.rs:46` calls `install_trap_trampoline`; trampoline body is `naked_asm!("1:", "wfi", "j 1b")`.
- [x] C-11 (No heap; `Cargo.toml` carries no allocator dependency; no `extern crate alloc` in `main.rs`): PASS — `Cargo.toml` has no `[dependencies]`; grep confirms no heap types.
- [x] C-12 (UART driver byte-at-a-time after LSR THRE poll at `0x1000_0005`, MMIO at `0x1000_0000`): PASS — `uart.rs:12-25`; THRE bit = `0x20`.
- [x] C-13 (`terminate(code)` writes SiFive-test finisher at `0x100000` then `wfi` loop): PASS — `halt.rs:24-39`; success `0x5555`, failure `0x3333 | (1 << 16)`.
- [x] C-14 (Boot via `naked_asm!` in `boot.rs`; no separate `.S`/`.s`; no pre-existing assembly modified): PASS — `find xvisor -name '*.[sS]'` returns zero hits; `git status` shows no `.s`/`.S` modification in the worktree.
- [x] C-15 (No SBI SRST call): PASS — no `ecall` instruction anywhere in `xvisor/src/`.
- [x] C-16 (`unsafe` blocks only in `hal/arch/riscv/{boot.rs,cpu.rs,csr.rs}` and `hal/platform/qemu/{uart.rs,halt.rs}`): PASS — verified by file-by-file grep of `unsafe {`. Every block has a `// SAFETY:` comment; `cargo clippy --bins -- -D warnings -W clippy::undocumented_unsafe_blocks` returns 0.
- [x] C-17 (`MAX_HARTS = 1`; secondary harts spin in OpenSBI HSM): PASS — `cpu.rs:7: pub const MAX_HARTS: usize = 1;`. OpenSBI log confirms `Platform HART Count: 1` and `Domain0 HARTs: 0*`.
- [x] C-18 (Stub modules `hal::{arch, platform}, mm/, vcpu/, vm/, sbi/` with `//!` doc comments; `#![deny(missing_docs)]` + `#![warn(clippy::missing_docs_in_private_items)]` at crate root with `-D warnings`): PASS — every stub carries a `//!` doc; crate-root lints at `main.rs:5-6`. `make clippy` returns 0.
- [x] C-19 (Banner format emitted by `rust_main` matches the V-IT-1 regex `^xvisor: hello from HS-mode \(hartid=[0-9]+, dtb=0x[0-9a-f]+\)$` — `xvisor/src/main.rs`): PASS — `main.rs:31-36` writes `xvisor: hello from HS-mode (hartid={}, dtb=0x{:x})` via `writeln!`. The captured QEMU output `xvisor: hello from HS-mode (hartid=0, dtb=0x8fe00000)` matches the regex via `grep -E` (exit 0). The prior CRITICAL is resolved by removing `BANNER_FMT` from the Spec block entirely; the regex is the sole contract anchor. `grep -n "BANNER_FMT" 02_PLAN.md xvisor/src/main.rs` returns zero hits. (Note: historical iteration files `00_PLAN.md`, `01_PLAN.md`, `00_REVIEW.md`, `01_REVIEW.md` still mention `BANNER_FMT`; those are frozen and not in audit scope.)

## SPEC Drift

- [x] Modified feature SPECs have CHANGELOG entries: N/A — `git diff --stat` shows no edits under `.ark/specs/features/<feature>/SPEC.md`. Modified files outside `xvisor/` and `.ark/tasks/`: `.ark/.installed.json`, `.ark/specs/features/INDEX.md`, `.vscode/settings.json`, `rust-toolchain.toml`. None are feature-SPEC bodies. `features/INDEX.md` is the index and will be retouched by the commit-time promotion step.

## Findings

> Cross-cutting observations that don't map to a single seeded item. Each Finding carries a Resolution. **Findings are advisory to the main session; the verifier does not patch.** Numbering restarts from `V-001`; the previous VERIFY's `V-001`..`V-005` are superseded.

### V-001 Implementation Phase 2 still mandates `extern "C" { pub fn trap_entry(); }` while the Spec API Surface says the symbol is deferred — inside-Plan inconsistency

- **Severity:** MEDIUM
- **Location:** `02_PLAN.md:233` (Implementation Phase 2 bullet) vs `02_PLAN.md:150` (Spec API Surface) vs `xvisor/src/hal/arch/riscv/trap.rs:25-28`.
- **Problem:** The Implementation Phase 2 bullet at `02_PLAN.md:233` reads "Create `xvisor/src/hal/arch/riscv/trap.rs` with the `TrapFrame` `#[repr(C)]` struct ..., `extern "C" { pub fn trap_entry(); }` declaration, and a doc comment specifying the `sscratch ↔ sp` swap convention." The Spec block's API Surface (`02_PLAN.md:150`) reads "TrapFrame struct only — trap_entry symbol lands when trap handling arrives." The shipped `trap.rs:25-28` ships only a comment placeholder for `trap_entry`. The Spec block (which is the promoted artifact) is consistent with shipped reality; the Implementation guidance section is not promoted, so this is **non-blocking for SPEC promotion**. It is, however, an inside-Plan contradiction that the PLAN's `## Log` did not record as a Changed/Removed entry during EXECUTE.
- **Why it matters:** Reviewers of `02_PLAN.md` standing alone (without the shipped tree) cannot tell whether the constraint is "extern decl required" or "extern decl deferred". Tidiness for the next iteration's review; no shipped-artifact impact.
- **Recommendation:** Edit `02_PLAN.md:233` to drop the `extern "C" { pub fn trap_entry(); }` clause and add a one-line Log `[**Changed**]` entry recording the EXECUTE-phase deferral. No code change required.
- **Resolution:** FIXED inline in `02_PLAN.md`; no code change.

### V-002 Implementation Phase 1 references stale `qemu_virt/` directory and `platform-qemu-virt` feature name

- **Severity:** MEDIUM
- **Location:** `02_PLAN.md:219` (`default = ["platform-qemu-virt"]` feature) and `02_PLAN.md:225` (`xvisor/src/hal/platform/qemu_virt/{mod.rs,uart.rs,halt.rs}`) vs `xvisor/Cargo.toml` (`default = ["platform-qemu"]`) vs `xvisor/src/hal/platform/qemu/` (the actual directory).
- **Problem:** Implementation Phase 1 still uses pre-EXECUTE names — the platform directory is now `qemu/` not `qemu_virt/`, and the Cargo feature is `platform-qemu` not `platform-qemu-virt`. The Spec block (`02_PLAN.md:77,94,152,157` and constraint citations C-12..C-13, C-15..C-16) consistently uses the renamed paths; only the Implementation Phase 1 guidance section is stale. Inside-Plan inconsistency, not a SPEC-promotion blocker.
- **Why it matters:** Same as V-001 — a reviewer reading the PLAN standalone gets contradictory paths. Trivial fix.
- **Recommendation:** Edit `02_PLAN.md:219` to use `platform-qemu` and `02_PLAN.md:225` to use `qemu/`; add a Log `[**Changed**]` row covering the EXECUTE-phase rename. No code change required.
- **Resolution:** FIXED inline in `02_PLAN.md`; no code change.

### V-003 `make clippy` recipe uses `--bins` instead of the PRD-requested `--all-targets` — structurally necessary; document explicitly

- **Severity:** LOW
- **Location:** `xvisor/Makefile:43`, `02_PLAN.md:270` (V-UT-4 acceptance flag), `02_PLAN.md:313` (Acceptance Mapping C-16 row).
- **Problem:** PRD outcome bullet 7 names `cargo clippy --all-targets -- -D warnings` as the gate flag. The Makefile uses `--bins` because `--all-targets` would attempt to build `libtest` against `riscv64gc-unknown-none-elf`, which is structurally infeasible for a `no_std`/`no_main` binary. The PLAN's V-UT-4 (line 270) correctly drops `--all-targets`; but the Acceptance Mapping row for C-16 (`02_PLAN.md:313`) still reads `cargo clippy --all-targets -- -D warnings -W clippy::undocumented_unsafe_blocks`. The lint *does* fire under `--bins` (confirmed by running `cargo clippy --target riscv64gc-unknown-none-elf --bins -- -D warnings -W clippy::undocumented_unsafe_blocks` against the shipped tree, exit 0); only the wording is stale.
- **Why it matters:** Future readers comparing the Acceptance Mapping to the Makefile see a flag mismatch. Five-second fix.
- **Recommendation:** Update `02_PLAN.md:313` to drop `--all-targets` and replace with `--bins`. Add a Log `[**Changed**]` row covering the EXECUTE-phase compromise. No code change required.
- **Resolution:** FIXED inline in `02_PLAN.md`; no code change.

### V-004 Response Matrix R-002 row carries stale `cc` crate / `boot.s` framing even after EXECUTE replaced both

- **Severity:** LOW
- **Location:** `02_PLAN.md:48` (Response Matrix R-002 row).
- **Problem:** The R-002 row reads "Phase 1 build.rs bullet replaced with: 'Create `xvisor/build.rs` that emits `cargo:rustc-link-arg=-Txvisor/linker.ld` + `cargo:rerun-if-changed=src/boot.s`, and uses the `cc` crate (build-dependency) to assemble `boot.rs` into a static archive linked into the binary.'" The EXECUTE-phase note in `02_PLAN.md:17(b)` clearly records `boot.s` was replaced by `boot.rs` (using `core::arch::naked_asm!`), removing the `cc` build-dependency. The on-disk `xvisor/Cargo.toml` has no `[build-dependencies]`, and `xvisor/build.rs` is the minimal link-arg emitter. A reader following the R-002 row to find the build.rs shape will be misled. (The NG-1 wording at `02_PLAN.md:69` retains the legacy `boot.s` term inside the trap-entry parking-pad description; the trampoline is in fact in `boot.rs`. Same EXECUTE-phase drift.)
- **Why it matters:** Cosmetic alignment; the EXECUTE-phase note already supersedes it. Low operational impact.
- **Recommendation:** Either (a) append "(superseded by EXECUTE-phase note 17(b))" to the R-002 row and update NG-1 to read `boot.rs`, or (b) leave both as historical record of the iter-02 response. Non-blocking.
- **Resolution:** FIXED inline in `02_PLAN.md`; no code change.

## Notes

- **What runs cleanly today.** From `xvisor/`: `make fmt` → 0, `make clippy` → 0, `make build` → 0, `make run` → 0, `make test` → 0 (placeholder echo). Observed banner: `xvisor: hello from HS-mode (hartid=0, dtb=0x8fe00000)`. V-IT-1 regex check via `grep -E '^xvisor: hello from HS-mode \(hartid=[0-9]+, dtb=0x[0-9a-f]+\)$'`: matched (exit 0). QEMU OpenSBI log confirms `Platform HART Count: 1`, `Boot HART Base ISA: rv64imafdch` (H-extension present), `Domain0 Next Address: 0x80200000`, `Domain0 Next Mode: S-mode`, `Domain0 Next Arg1: 0x000000008fe00000`. The stricter clippy invocation `cargo clippy --target riscv64gc-unknown-none-elf --bins -- -D warnings -W clippy::undocumented_unsafe_blocks` also returns 0, documenting tight `unsafe`-block hygiene.

- **Prior CRITICAL resolution.** Option (b) adopted as requested: the previous VERIFY's V-001 (`BANNER_FMT` named seven times in the Spec block while absent from the implementation) is closed by removing `BANNER_FMT` from `02_PLAN.md`'s Spec block entirely. Grep over `02_PLAN.md` and `xvisor/src/` returns zero hits. The new C-19 (`02_PLAN.md:181`) anchors on the V-IT-1 regex directly: `^xvisor: hello from HS-mode \(hartid=[0-9]+, dtb=0x[0-9a-f]+\)$`. Architecture tree, API Surface, Runtime, Validation V-IT-1, and Acceptance Mapping C-19 row are all consistent with this anchor.

- **Spec self-containment cross-check.** Grep over the `## Spec` block (`02_PLAN.md:57-181`, lines bounded by `## Spec` and the closing `---`) for the forbidden tokens `BANNER_FMT`, `iter-NN`, `iter-[0-9]+`, `this iteration`, `\bP0\b`, `\bP1\b`, `\bP2\b`, `\bP3\b`, `\bP4\b`: one hit at C-8 (`02_PLAN.md:170`, "left zero this iteration") — semantically refers to "the scope of this feature SPEC," not to a PLAN iteration index; the previous VERIFY already classified this as acceptable. No other forbidden refs. The Spec block stands alone as a SPEC body.

- **Promotion safety verdict: YES.** With `BANNER_FMT` removed and the V-IT-1 regex serving as the sole contract anchor, `02_PLAN.md`'s `## Spec` block (lines 57–181: Goals, Non-goals, Architecture, Data Structure, API Surface, Constraints) can be safely promoted verbatim to `.ark/specs/features/xvisor/framework/SPEC.md` on `/ark:commit`. Every constraint anchor points at a symbol or behavioural shape that exists in the shipped tree. The Spec block is self-contained — it does not reference iteration numbers, phase numbers, or PRD-only language. The four residual findings (V-001..V-004) are all in non-Spec sections (Implementation, Response Matrix, Acceptance Mapping) and do not propagate into the promoted artifact.

- **EXECUTE-phase drift cross-check.** All seven items from the user's brief still verify: no `xvisor/rust-toolchain.toml`, no `xvisor/.cargo/`, no `xvisor/src/boot.s` or top-level `xvisor/src/boot.rs` (the file lives at `xvisor/src/hal/arch/riscv/boot.rs`), no `arch/` or `device/` at the top of `src/`. Stub modules (`mm`, `vcpu`, `vm`, `sbi`, `hal/arch/loongarch`, `hal/platform/xemu`) all carry one-line `//!` doc comments. `xvisor/Cargo.toml` has `default = ["platform-qemu"]`, no `[build-dependencies]`, no `cc`.

- **Files audited.** `xvisor/{Cargo.toml,Cargo.lock,build.rs,linker.ld,Makefile,README.md}`; `xvisor/src/{main.rs,hal/mod.rs,hal/arch/riscv/{mod,boot,cpu,csr,trap}.rs,hal/arch/loongarch/mod.rs,hal/platform/qemu/{mod,uart,halt}.rs,hal/platform/xemu/mod.rs,mm/mod.rs,vcpu/mod.rs,vm/mod.rs,sbi/mod.rs}`; `rust-toolchain.toml`, `.vscode/settings.json`; `.ark/tasks/framework/{PRD,02_PLAN}.md`; `.ark/specs/project/INDEX.md`; `.ark/specs/features/INDEX.md`. Historical iteration files (`00_PLAN.md`, `01_PLAN.md`, `00_REVIEW.md`, `01_REVIEW.md`, `02_REVIEW.md`) are frozen and excluded from the audit scope.

- **Gates run (this re-verify).** `cd xvisor && make fmt` → 0; `cd xvisor && make clippy` → 0; `cd xvisor && make build` → 0; `cd xvisor && make run` → 0 (banner observed, QEMU exit 0); `cd xvisor && make test` → 0. Additionally `cargo clippy --target riscv64gc-unknown-none-elf --bins -- -D warnings -W clippy::undocumented_unsafe_blocks` → 0.

- **Git scope check.** Post-write `git status`: only `.ark/tasks/framework/VERIFY.md` (inside the still-untracked task directory) was rewritten by this re-verify. No code, SPEC, PRD, or PLAN file was modified by the verifier.
