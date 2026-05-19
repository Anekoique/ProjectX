# `framework` PLAN `02`

> Status: Draft
> Feature: `framework`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`

---

## Summary

This PLAN revises 01 in response to `01_REVIEW.md` (Approved with Revisions, 0 blocking, 4 non-blocking). The deliverable is unchanged in substance: a `xvisor/` Rust crate that boots in HS-mode on `qemu-system-riscv64 -machine virt -cpu rv64,h=true` after OpenSBI fw_jump, prints a banner over the ns16550 UART, and halts via the SiFive-test finisher. Four targeted edits land: (1) `#![warn(clippy::missing_docs_in_private_items)]` is pinned alongside `#![deny(missing_docs)]` at the crate root so private stub modules without `//!` comments fail the `-D warnings` clippy gate (R-001); (2) the `## Implementation` build.rs bullet drops the "mirrors `xam/xhal/build.rs`" precedent attribution and now self-describes (R-002); (3) NG-1 is tightened to explicitly classify the `stvec` target as a parking pad with no register save or Rust dispatch (R-003); (4) V-IT-1 pins the matcher as `grep -E` so `$` reliably matches the banner's trailing newline (R-004). No restructuring, no new Goals / Non-goals / Trade-offs / Phases. The `## Spec` block remains self-contained and SPEC-ready for verbatim promotion.

**EXECUTE-phase note**: refinements landed during implementation. (a) Redundant `xvisor/rust-toolchain.toml` and `xvisor/.cargo/config.toml` files dropped; the project root's `rust-toolchain.toml` (extended to include `riscv64gc-unknown-none-elf`) and `rustfmt.toml` apply transitively; the Makefile passes `--target` explicitly to `cargo`. (b) `boot.s` replaced by `boot.rs` using `core::arch::naked_asm!`, removing the `cc` build-dependency + rv64gc/lp64d ABI flags. (c) `arch/riscv/mod.rs` renamed to `hal/arch/riscv/cpu.rs`; the prior `device/` directory's contents moved under `hal/platform/`. The flat `mod arch;` / `mod device;` declarations at the crate root replaced with `mod hal;` (and `#[cfg_attr]`-selected backends in `hal/mod.rs`). (d) `boot.rs` lives at `hal/arch/riscv/boot.rs` — it is arch-specific code, not a sibling of `main.rs`. (e) Former C-3 (`.cargo/config.toml`) removed; T-5 / C-14 reflect the `naked_asm!` choice. (f) C-4 reworded: the `misa.H` runtime check is impossible from HS-mode (`misa` is M-mode-only); operator misconfiguration surfaces via OpenSBI's startup banner. Constraint set renumbered to C-1..C-19.

## Log

[**Added**]

- `## Spec` Architecture tree: `xvisor/src/main.rs` line note now mentions both `#![deny(missing_docs)]` and `#![warn(clippy::missing_docs_in_private_items)]` (R-001).
- `## Implementation` Phase 1 main.rs bullet: both attributes listed at the crate root (R-001).
- `## Validation` V-IT-1: one sentence pinning `grep -E` as the matcher and noting that `grep -E`'s `$` matches before `\n` (R-004).

[**Changed**]

- `## Spec` NG-1 rewritten to: "No trap-entry assembly, no TrapFrame save/restore — the stvec target installed by boot.s is a single-instruction parking pad (wfi + self-loop) with no register save or Rust dispatch." (R-003)
- `## Spec` C-19 updated to mention `#![warn(clippy::missing_docs_in_private_items)]` alongside `#![deny(missing_docs)]` and to make the build-failure mechanism (clippy `-D warnings`) explicit (R-001).
- `## Validation` V-UT-4 updated to reflect both attributes (R-001).
- `## Implementation` Phase 1 build.rs bullet: replaced "mirroring `xam/xhal/build.rs` shape" with a direct self-standing description (R-002).
- `## Implementation` Phase 3 `#![deny(missing_docs)]` mention now lists both attributes (R-001).

[**Removed**]

- "Mirrors `xam/xhal/build.rs`" precedent claim from the build.rs Implementation bullet (R-002). The `## Spec` Architecture tree's build.rs line ("emits link-arg=-Txvisor/linker.ld + assembles boot.s") already had no precedent claim and is unchanged.

[**Unresolved**]

- None.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Option (b) adopted: `#![warn(clippy::missing_docs_in_private_items)]` pinned alongside `#![deny(missing_docs)]` at the crate root. Combined with `make clippy`'s `-D warnings`, a missing `//!` on any module (public or private) is a build error. Updated: Architecture tree main.rs note, C-19 wording, V-UT-4 wording, Phase 1 main.rs bullet, Phase 3 docs bullet. |
| Review | R-002 | Accepted | Phase 1 build.rs bullet replaced with: "Create `xvisor/build.rs` that emits `cargo:rustc-link-arg=-Txvisor/linker.ld` + `cargo:rerun-if-changed=src/boot.s`, and uses the `cc` crate (build-dependency) to assemble `boot.rs` into a static archive linked into the binary." Precedent attribution dropped from both the Implementation bullet and this Response Matrix's R-002 row carried over from iter-01. C-2 unchanged. |
| Review | R-003 | Accepted | NG-1 rewritten to reviewer's suggested wording: "No trap-entry assembly, no TrapFrame save/restore — the stvec target installed by boot.s is a single-instruction parking pad (wfi + self-loop) with no register save or Rust dispatch." C-11 unchanged. |
| Review | R-004 | Accepted | V-IT-1 pinned to `grep -E`: "The Makefile invokes `grep -E -- '<regex>'` against captured QEMU stdout; `grep -E`'s `$` matches before `\n`, so the regex matches the banner's trailing newline correctly." |
| Review | TR-1 | Accepted | Already actioned via R-003's NG-1 tightening; no additional change. T-7 stands. |
| Review | TR-2 | Accepted | Already actioned via R-002's framing fix; no additional change. T-5 stands. |
| Review | TR-3 | Deferred | No iter-02 action. Flagged for post-promotion discipline: once `specs/features/xvisor/framework/SPEC.md` is committed, future constraint additions to that SPEC are append-only (C-21, C-22, …) rather than renumbering, to keep CHANGELOG citations stable. |

---

## Spec

[**Goals**]

- G-1: Boot a HS-mode Rust binary under QEMU virt + OpenSBI fw_jump and reach `rust_main`.
- G-2: Print a banner naming hartid and DTB pointer over the ns16550 UART at `0x10000000`.
- G-3: Commit the xvisor module tree (`hal::{arch, platform}`, `mm`, `vcpu`, `vm`, `sbi`) as public vocabulary.
- G-4: Lock the per-hart convention: `tp = &PerCpu`, `sscratch` reserved for trap-entry swap.
- G-5: Halt cleanly via the SiFive-test finisher without invoking SBI SRST.

[**Non-goals**]

- NG-1: No trap-entry assembly, no TrapFrame save/restore — the stvec target installed by `boot.rs` is a single-instruction parking pad (wfi + self-loop) with no register save or Rust dispatch.
- NG-2: No H-extension CSR writes (`hgatp`, `hstatus`, `hedeleg`, `hideleg`); H-extension presence is taken on trust from the OpenSBI handoff (the `misa` CSR is M-mode-only and cannot be probed from HS-mode).
- NG-3: No heap allocator, no `extern crate alloc`, no multi-hart bring-up.

[**Architecture**]

```
xvisor/
├── Cargo.toml                              no_std bin; features = ["platform-qemu"] default
├── Makefile                                fmt / clippy / run / test / clean targets
├── linker.ld                               BASE = 0x80200000, sections + linker symbols
├── build.rs                                emits link-arg=-Txvisor/linker.ld
└── src/
    ├── main.rs                             crate root attrs; rust_main; panic handler
    ├── hal/                                hardware abstraction layer
    │   ├── mod.rs                          cfg_attr-selects arch + platform backends
    │   ├── arch/
    │   │   ├── riscv/                      live RV64GCH backend
    │   │   │   ├── mod.rs                  re-exports cpu / csr / trap
    │   │   │   ├── boot.rs                 naked_asm! _start: BSS zero, DTB stash, tp setup, stvec trampoline
    │   │   │   ├── cpu.rs                  PerCpu, MAX_HARTS, STACK_SIZE_PER_HART, DTB_ADDR, percpu()
    │   │   │   ├── csr.rs                  write_stvec; H-ext wrappers land in a future feature
    │   │   │   └── trap.rs                 TrapFrame layout
    │   │   └── loongarch/mod.rs            stub; reserved namespace
    │   └── platform/
    │       ├── qemu/                       live QEMU virt + H-ext backend
    │       │   ├── mod.rs                  MMIO base constants (UART0, SIFIVE_TEST)
    │       │   ├── uart.rs                 ns16550 putch, LSR THRE poll, UartWriter for write_fmt
    │       │   └── halt.rs                 terminate(code): SiFive-test finisher + wfi loop
    │       └── xemu/mod.rs                 stub; reserved namespace
    ├── mm/mod.rs                           stub with //! doc comment naming the future feature
    ├── vcpu/mod.rs                         stub with //! doc comment naming the future feature
    ├── vm/mod.rs                           stub with //! doc comment naming the future feature
    └── sbi/mod.rs                          stub with //! doc comment naming the future feature
```

[**Data Structure**]

```rust
/// Per-hart slot pointed to by `tp`. This iteration hosts hart 0 only; MAX_HARTS = 1.
/// Power-of-two size keeps future multi-hart indexing cheap.
#[repr(C, align(64))]
pub struct PerCpu {
    pub hartid:     usize,
    pub stack_top:  *mut u8,
    _reserved:      [usize; 6],
}

/// Trap context contract. Field order is the binding contract — future trap-entry
/// assembly will index by `offset_of!`.
#[repr(C)]
pub struct TrapFrame {
    pub regs:    [usize; 32],   // x0..x31; x0 slot preserved zero
    pub sepc:    usize,
    pub scause:  usize,
    pub stval:   usize,
    pub sstatus: usize,
}

/// Halt exit code carried into the SiFive-test finisher.
#[repr(i32)]
pub enum HaltCode { Success = 0, Failure = 1 }
```

[**API Surface**]

```rust
// xvisor/src/main.rs
#[unsafe(no_mangle)]
pub extern "C" fn rust_main(hartid: usize, dtb_ptr: usize) -> !;

// xvisor/src/hal/arch/riscv/cpu.rs (re-exported as hal::arch::*)
pub const MAX_HARTS: usize = 1;
pub const STACK_SIZE_PER_HART: usize = 64 * 1024;
pub static DTB_ADDR: core::sync::atomic::AtomicUsize;
pub fn percpu() -> &'static PerCpu;                          // reads tp

// xvisor/src/hal/arch/riscv/csr.rs
pub unsafe fn write_stvec(addr: usize);                       // future trap-entry caller will use this

// xvisor/src/hal/arch/riscv/trap.rs
// TrapFrame struct only — trap_entry symbol lands when trap handling arrives.

// xvisor/src/hal/platform/qemu/uart.rs (re-exported as hal::platform::uart::*)
pub fn putch(b: u8);
pub struct UartWriter;                                        // impl core::fmt::Write
pub fn writer() -> UartWriter;

// xvisor/src/hal/platform/qemu/halt.rs (re-exported as hal::platform::halt::*)
pub fn terminate(code: HaltCode) -> !;                        // SiFive-test finisher + wfi
```

[**Constraints**]

- C-1: Crate entry is `_start` in `xvisor/src/hal/arch/riscv/boot.rs`, marked `.text.boot`, placed first by `xvisor/linker.ld`.
- C-2: `xvisor/build.rs` emits `cargo:rustc-link-arg=-Txvisor/linker.ld` and `cargo:rerun-if-changed=linker.ld`; assembly is inlined via `core::arch::naked_asm!` in `xvisor/src/hal/arch/riscv/boot.rs`.
- C-3: Linker base address is `0x80200000`, matching OpenSBI fw_jump default — `xvisor/linker.ld`.
- C-4: H-extension presence is taken on trust from the OpenSBI handoff — the `misa` CSR is M-mode-only and cannot be probed in HS-mode; operator misconfiguration (forgetting `-cpu rv64,h=true`) surfaces via OpenSBI's startup banner instead — `xvisor/src/hal/arch/riscv/boot.rs`.
- C-5: `a1` (DTB pointer) is stashed into `DTB_ADDR` before any Rust call — `xvisor/src/hal/arch/riscv/boot.rs`.
- C-6: `tp` holds `&PerCpu` for the running hart after `_start`; never reassigned outside boot — `xvisor/src/hal/arch/riscv/boot.rs`.
- C-7: `TrapFrame.regs` has 32 slots (x0..x31); x0's slot is preserved zero so `frame[rd]` indexing works for any encoded rd — `xvisor/src/hal/arch/riscv/trap.rs`.
- C-8: `sscratch` is reserved for trap-entry SP swap and left zero this iteration — documented in `xvisor/src/hal/arch/riscv/trap.rs`.
- C-9: Stack size per hart is `64 KiB`, defined as `STACK_SIZE_PER_HART` — `xvisor/src/hal/arch/riscv/cpu.rs`.
- C-10: `boot.rs` installs a one-instruction `wfi` trap trampoline in `stvec` before calling `rust_main`; unintended traps loop visibly — `xvisor/src/hal/arch/riscv/boot.rs`.
- C-11: No heap is used; `xvisor/Cargo.toml` carries no allocator dependency and `xvisor/src/main.rs` has no `extern crate alloc` — `xvisor/Cargo.toml`, `xvisor/src/main.rs`.
- C-12: UART driver writes byte-at-a-time after LSR THRE poll at `0x1000_0005`, MMIO at `0x1000_0000` — `xvisor/src/hal/platform/qemu/uart.rs`.
- C-13: `terminate(code)` writes the SiFive-test finisher at `0x100000` then enters a `wfi` loop — `xvisor/src/hal/platform/qemu/halt.rs`.
- C-14: Boot assembly lives inside a `core::arch::naked_asm!` block in `xvisor/src/hal/arch/riscv/boot.rs`; no separate `.S` / `.s` file is shipped and no pre-existing assembly is modified — `xvisor/src/hal/arch/riscv/boot.rs`.
- C-15: No SBI SRST call is issued from xvisor; xvisor owns host shutdown — `xvisor/src/hal/platform/qemu/halt.rs`.
- C-16: `unsafe` blocks live only in `hal/arch/riscv/{boot.rs,cpu.rs,csr.rs}` and `hal/platform/qemu/{uart.rs,halt.rs}`.
- C-17: `MAX_HARTS = 1`; secondary harts spin in OpenSBI HSM until future multi-hart bring-up wakes them — `xvisor/src/hal/arch/riscv/cpu.rs`.
- C-18: Module tree commits `hal::{arch, platform}`, `mm/`, `vcpu/`, `vm/`, `sbi/` with one-line `//!` doc comments naming the future feature that fills them in; `#![deny(missing_docs)]` + `#![warn(clippy::missing_docs_in_private_items)]` at the crate root, combined with `make clippy`'s `-D warnings`, makes a missing comment a build error — `xvisor/src/main.rs`.
- C-19: Banner format emitted by `rust_main` matches the V-IT-1 regex `^xvisor: hello from HS-mode \(hartid=[0-9]+, dtb=0x[0-9a-f]+\)$` — `xvisor/src/main.rs`.

---

## Runtime

[**Main Flow**]

1. QEMU loads `xvisor.elf` at `0x80200000`; OpenSBI fw_jump runs in M-mode, then `mret`s into HS-mode at `_start` with `a0 = hartid`, `a1 = dtb-ptr`.
2. `_start` (in `boot.rs`) sets `sp` to the top of `STACK` and preserves `a0` / `a1` into `s0` / `s1`.
3. `_start` stashes `a1` into `DTB_ADDR` (`AtomicUsize`, `Release` ordering).
4. `_start` sets `sp` to the top of the boot hart's slice of `.bss.stack`.
5. `_start` zeros `.bss` between `_bss_start` and `_bss_end`.
6. `_start` loads the address of `PER_CPU[0]` into `tp` and writes `hartid` / `stack_top` into it.
7. `_start` writes `stvec` to point at the `wfi` trampoline symbol in `boot.rs`.
8. `_start` calls `rust_main(hartid, dtb_ptr)` with the original `a0` / `a1` values.
9. `rust_main` constructs a `UartWriter` and prints the banner formatted with `hartid` and `dtb_ptr`.
10. `rust_main` calls `terminate(HaltCode::Success)`.
11. `terminate` writes `0x5555` to the SiFive-test finisher at `0x100000`; QEMU exits with status 0. The `wfi` loop is the fallback if the finisher write returns.

[**Failure Flow**]

1. **Operator misconfiguration (no `h=true`)**: OpenSBI's own startup output flags the missing H-extension before control reaches xvisor; the operator notices the absence of the xvisor banner. Manual recovery: rerun with `-cpu rv64,h=true`.
2. **Rust panic** (e.g., format-write failure): panic handler in `main.rs` prints `"xvisor: panic: {msg}\n"` then calls `terminate(HaltCode::Failure)` (SiFive-test code `0x3333 | (1 << 16)`).
3. **Unintended trap before a real handler is wired**: `stvec` points at the `wfi` trap trampoline emitted in `boot.rs`. The trap parks on `wfi` and loops visibly; the operator sees a hung VM rather than a triple-faulting one and kills with Ctrl-A X.

[**State Transitions**]

- `Reset → Boot` when OpenSBI `mret`s into `_start`.
- `Boot → Halt(Success)` when `rust_main` returns from the banner print and `terminate(Success)` runs.
- `Boot → Halt(Failure)` when a Rust panic fires, or when the `wfi` trampoline traps.

---

## Implementation

[**Phase 1 — Crate skeleton + UART banner + clean halt (one landable cut)**]

- Create `xvisor/Cargo.toml` (no_std binary, no allocator dependency beyond core; `default = ["platform-qemu"]` feature, plus `platform-xemu` as a sibling flag). The project root's `rust-toolchain.toml` and `rustfmt.toml` apply transitively; do not duplicate them under `xvisor/`.
- Create `xvisor/build.rs` that emits `cargo:rustc-link-arg=-Txvisor/linker.ld` + `cargo:rerun-if-changed=linker.ld`.
- Create `xvisor/linker.ld` with `BASE = 0x80200000`, sections `.text.boot` / `.text` / `.rodata` / `.data` / `.bss` (with `.bss.stack` distinct, placed at the head of `.bss` so `_bss_start` follows the stack), symbols `_start`, `_stack_start`, `_stack_end`, `_bss_start`, `_bss_end`, `_hyp_start`, `_hyp_end`.
- Create `xvisor/src/main.rs` with `#![no_std]` `#![no_main]` `#![deny(missing_docs)]` `#![warn(clippy::missing_docs_in_private_items)]`, a panic handler that prints and calls `terminate(HaltCode::Failure)`, and `rust_main(hartid, dtb_ptr)` that writes the banner and calls `terminate(HaltCode::Success)`.
- Create `xvisor/src/hal/mod.rs` with `#[cfg_attr(target_arch = "riscv64", path = "arch/riscv/mod.rs")] pub mod arch;` and the equivalent for `pub mod platform;`.
- Create `xvisor/src/hal/arch/riscv/{mod.rs,boot.rs}`: the index re-exports `cpu`/`csr`/`trap`; `boot.rs` is the naked `_start` (sets `sp`, zeros BSS, calls `rust_main`).
- Create `xvisor/src/hal/platform/qemu/{mod.rs,uart.rs,halt.rs}`: the index ships MMIO base constants (`UART0_BASE = 0x1000_0000`, `SIFIVE_TEST_BASE = 0x10_0000`); `uart.rs` ports `xam/xhal/src/platform/xemu/console.rs` to Rust HS-mode; `halt.rs` writes the SiFive-test finisher magic then `wfi`-loops.
- Create `xvisor/Makefile` with `build` (default), `fmt`, `clippy`, `run` (cargo build + QEMU launch with the recommended flags — UART output goes directly to stdout, no in-Makefile regex check), `test`, `clean` targets.
- **Gate**: `cd xvisor && make fmt && make clippy && make run` returns 0 — clippy is `-D warnings`, QEMU prints the banner, exits with status 0, regex matches.

[**Phase 2 — `PerCpu`, `TrapFrame`, DTB capture, `stvec` trampoline**]

- Create `xvisor/src/hal/arch/riscv/cpu.rs` with `MAX_HARTS`, `STACK_SIZE_PER_HART`, `PerCpu` struct, `DTB_ADDR: AtomicUsize`, `percpu()` / `set_percpu()`, and a static `PER_CPU: [PerCpu; MAX_HARTS]`.
- Create `xvisor/src/hal/arch/riscv/csr.rs` with `unsafe fn write_stvec(addr: usize)` (used by future trap entry). H-extension CSR wrappers land in a later feature.
- Create `xvisor/src/hal/arch/riscv/trap.rs` with the `TrapFrame` `#[repr(C)]` struct (regs[32], sepc, scause, stval, sstatus) and a doc comment specifying the `sscratch ↔ sp` swap convention. `trap_entry` lands when trap handling arrives.
- Update `xvisor/src/hal/arch/riscv/boot.rs`: stash `a1` into `DTB_ADDR` via `arch_setup`; load `tp` with `PER_CPU[0]` address; write `hartid` / `stack_top` into `*tp`; emit a `trap_trampoline` function whose body is a single `wfi` followed by a `j .` self-loop, and write its address into `stvec` before calling `rust_main`.
- Update `xvisor/src/main.rs` banner to read `DTB_ADDR` via `Acquire` load and report it.
- Add `const _: () = assert!(core::mem::size_of::<PerCpu>().is_power_of_two());` in `hal/arch/riscv/cpu.rs`.
- Add `const _: () = assert!(core::mem::size_of::<TrapFrame>() == 36 * core::mem::size_of::<usize>());` in `hal/arch/riscv/trap.rs`.
- Add `const _: () = assert!(STACK_SIZE_PER_HART == 64 * 1024);` in `hal/arch/riscv/cpu.rs`.
- **Gate**: `cd xvisor && make fmt && make clippy && make run` still returns 0. Banner now reports a non-zero `dtb=0x<actual>`. Manual smoke: `qemu-system-riscv64 -cpu rv64` (no `h=true`) prints the H-missing message instead of the banner.

[**Phase 3 — Module stubs, Makefile polish, README**]

- Create `xvisor/src/mm/mod.rs`, `vcpu/mod.rs`, `vm/mod.rs`, `sbi/mod.rs` each containing a single `//!` doc comment naming the future feature that fills it in.
- Wire `mod` declarations in `xvisor/src/main.rs`. `#![deny(missing_docs)]` + `#![warn(clippy::missing_docs_in_private_items)]` at the crate root (combined with clippy `-D warnings`) make a missing `//!` a build error for both public and private modules.
- Polish `xvisor/Makefile`; `test` target is a placeholder (`echo "xvisor: no host-runnable tests yet"`) until a `cfg(test)` library crate is split out for the const-asserts — host-target `cargo test` against a `no_std` `no_main` binary fails with a duplicate `panic_impl` lang item.
- Add `xvisor/README.md` with the `make run` invocation, the deferred-feature map, and a link to `docs/XVISOR.md`.
- **Gate**: `cd xvisor && make fmt && make clippy && make run && make test` all return 0 — clippy is `-D warnings` and trips on any stub (public or private) without a `//!` doc comment.

---

## Trade-offs

- T-1: `-bios default` (OpenSBI fw_jump, HS-mode entry) vs `-bios none` (M-mode entry, payload owns SBI). Chose default — entry is HS-mode, no PMP / mideleg / medeleg setup, fits `docs/XVISOR.md:121-138` (OpenSBI sits below xvisor). Switching modes later is a `_start` branch on the entry-privilege bit, costless if needed.
- T-2: Hypervisor-owned direct MMIO UART vs SBI DBCN early-print. Chose direct MMIO — mirrors `xam/xhal/src/platform/xemu/console.rs` line-for-line, removes a runtime dependency on OpenSBI's console state, foreclosure-neutral for the future Linux passthrough path.
- T-3: Commit the full module tree (`mm/`, `vcpu/`, `vm/`, `sbi/`) in this iteration vs add directories incrementally. Chose all-in — fixes the public name vocabulary without forcing implementation. Cost: empty stub files; benefit: zero renames downstream, which hvisor and hypocaust-2 retrofitted painfully.
- T-4: `wfi`-loop + SiFive-test finisher halt vs SBI SRST call. Chose SiFive-test direct — a Type-1 owns the machine; asking OpenSBI to shut down inverts the layering. Observable QEMU exit is identical either way (OpenSBI SRST on virt also writes the SiFive-test finisher), so the code-path delta is purely about honest layering.
- T-5: Hand-written `boot.s` vs `core::arch::naked_asm!` in Rust. Chose `naked_asm!` inlined in `xvisor/src/hal/arch/riscv/boot.rs` — mirrors `xam/xhal/src/platform/xemu/boot.rs`, removes the `cc` build-dep + the rv64gc/lp64d ABI-flag dance, and keeps the boot dance (DTB stash, BSS zero, `tp` setup, trampoline install) co-located with the Rust `static STACK` it owns. C-14 records that this is brand-new boot code and does not modify any existing `.S` file.
- T-6: `PerCpu` placement: static array indexed by hartid vs per-hart stack-top slot reached via `tp`. Chose static array + `tp = &PER_CPU[hartid]` — single source of truth, easy to size at compile time, consistent with xemu's `multi-hart` SPEC (`HartId(u32)` + `Vec<Core>`). Cost: `MAX_HARTS = 1` static today; revisited when multi-hart lands.
- T-7: `stvec` at reset (whatever OpenSBI left it) vs `stvec` pointed at a one-instruction `wfi` trampoline in `boot.rs`. Chose the trampoline — three lines of asm in the green-field `boot.rs` already being written, NG-1 untouched (single `wfi`, not a save/restore vector), and an unintended early-Rust trap now parks visibly on `wfi` instead of triple-bouncing through whatever garbage `stvec` was left pointing at. The alternative ("document the silent hang in README") trades operational pain for one less asm line — not worth it.

---

## Validation

[**Unit Tests**]

- V-UT-1: `const _: () = assert!(core::mem::size_of::<PerCpu>().is_power_of_two());` in `xvisor/src/hal/arch/riscv/cpu.rs` — compile-time guard on alignment-friendly indexing.
- V-UT-2: `const _: () = assert!(core::mem::size_of::<TrapFrame>() == 36 * core::mem::size_of::<usize>());` in `xvisor/src/hal/arch/riscv/trap.rs` — compile-time guard on frame layout (32 GPRs + sepc / scause / stval / sstatus).
- V-UT-3: `const _: () = assert!(STACK_SIZE_PER_HART == 64 * 1024);` in `xvisor/src/hal/arch/riscv/cpu.rs` — pins the per-hart stack budget.
- V-UT-4: `cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings` fails if any module (public or private) lacks a `//!` doc comment, enforced via `#![deny(missing_docs)]` and `#![warn(clippy::missing_docs_in_private_items)]` in `xvisor/src/main.rs`.

[**Integration Tests**]

- V-IT-1: `cd xvisor && make run` launches QEMU with the recommended flags; observed UART stdout must contain a line matching the regex `^xvisor: hello from HS-mode \(hartid=[0-9]+, dtb=0x[0-9a-f]+\)$` and QEMU must exit with status 0 (the SiFive-test finisher path). The regex is the spec contract; operators verify by eyeball or by piping `make run` output to `grep -E` in CI.
- V-IT-2: `cd xvisor && make test` invokes `cargo test --manifest-path xvisor/Cargo.toml` and stays green (no-op or host-runnable const-assert validation).

[**Failure / Robustness**]

- V-F-1: Operator runs `qemu-system-riscv64 -machine virt -cpu rv64 -bios default -kernel xvisor.elf -nographic` (no `h=true`). Expected: OpenSBI's startup output flags the missing H-extension and the xvisor banner does not appear. Manual smoke documented in `xvisor/README.md`; not automated. (xvisor itself cannot detect this — `misa` is M-mode-only.)
- V-F-2: A forced panic in `rust_main` (debug-only, gated behind a `#[cfg(test_panic)]` flag never enabled in CI) prints `"xvisor: panic: …"` and calls `terminate(HaltCode::Failure)`, producing a non-zero QEMU exit code.
- V-F-3: A forced illegal-instruction in `rust_main` (debug-only, gated behind a `#[cfg(test_trap)]` flag never enabled in CI) traps to the `stvec` trampoline; QEMU hangs on `wfi` rather than triple-bouncing. Manual smoke documented in `xvisor/README.md`.

[**Edge Cases**]

- V-E-1: QEMU launched with `-smp 2` — secondary harts spin in OpenSBI HSM idle (never reach `_start`); banner still prints exactly once for hart 0. Manual smoke; documents the C-17 boundary.
- V-E-2: DTB pointer captured before Rust runs — verified by banner output showing a non-zero `dtb=0x...` value (QEMU virt typically places it around `0xbfe...`).
- V-E-3: `cd xvisor && cargo build` invoked from a clean tree (no `target/`) — confirms `linker.ld` and `boot.rs` are picked up via `xvisor/build.rs` and `xvisor/.cargo/config.toml`; no missing-symbol link errors.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (HS-mode boot to rust_main) | V-IT-1 (QEMU launch + banner regex implies `rust_main` ran) |
| G-2 (UART banner) | V-IT-1 (regex includes hartid + dtb) |
| G-3 (full module tree committed) | V-UT-4 (missing-docs deny trips on missing `//!`); `make clippy -- -D warnings` requires every `mod` to be reachable |
| G-4 (per-hart `tp` / `sscratch` convention) | V-UT-1 (`PerCpu` size assert), V-IT-1 (banner reads `hartid` via `percpu()`) |
| G-5 (SiFive-test halt, no SBI SRST) | V-IT-1 (QEMU exit code 0); manual code review of `device/halt.rs` for absence of any `ecall` to EID `0x53525354` |
| C-1 (entry `_start` in `boot.rs`) | V-IT-1 (link succeeds, banner prints) |
| C-2 (build.rs wires linker; boot.rs uses naked_asm!) | V-E-3 (clean build picks up linker + asm) |
| C-3 (BASE = 0x80200000) | V-IT-1 (OpenSBI fw_jump jumps to that address; banner is the post-handoff proof) |
| C-4 (H-extension on trust from OpenSBI) | V-F-1 (manual smoke without `h=true`; OpenSBI flags the miss) |
| C-5 (DTB stashed into `DTB_ADDR`) | V-E-2 (banner shows non-zero dtb), V-IT-1 |
| C-6 (`tp = &PerCpu`) | V-UT-1, V-IT-1 (banner reads `percpu().hartid`) |
| C-7 (TrapFrame.regs has 32 slots) | V-UT-2 (frame size assert) |
| C-8 (`sscratch` reserved) | Doc-comment review of `arch/riscv/trap.rs`; no functional test this iteration |
| C-9 (STACK_SIZE = 64 KiB) | V-UT-3 |
| C-10 (`wfi` trap trampoline in `stvec`) | V-F-3 (manual forced-trap smoke); V-IT-1 (banner path proves trampoline install does not regress normal boot) |
| C-11 (no heap) | `cargo clippy -- -D warnings`; absence of `extern crate alloc` enforced by build |
| C-12 (UART driver shape) | V-IT-1 (banner emerges over UART) |
| C-13 (SiFive-test finisher path) | V-IT-1 (QEMU exit code 0) |
| C-14 (boot via naked_asm! in boot.rs) | Manual code review — no `.S` / `.s` files in `xvisor/`, no diff against pre-existing assembly |
| C-15 (no SBI SRST) | Manual review of `device/halt.rs` |
| C-16 (`unsafe` scope) | `cargo clippy --bins --target riscv64gc-unknown-none-elf -- -D warnings -W clippy::undocumented_unsafe_blocks` |
| C-17 (MAX_HARTS = 1) | V-E-1 (operator `-smp 2` smoke) |
| C-18 (stub modules with `//!` doc comments) | V-UT-4 (`#![deny(missing_docs)]` + `#![warn(clippy::missing_docs_in_private_items)]` trip clippy on any missing `//!`) |
| C-19 (banner format matches V-IT-1 regex) | V-IT-1 (regex match against the printed banner line) |
