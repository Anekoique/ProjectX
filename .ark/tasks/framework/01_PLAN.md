# `framework` PLAN `01`

> Status: Draft
> Feature: `framework`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`

---

## Summary

This PLAN revises 00 in response to `00_REVIEW.md`. The deliverable is unchanged in substance: a `xvisor/` Rust crate that boots in HS-mode on `qemu-system-riscv64 -machine virt -cpu rv64,h=true` after OpenSBI fw_jump, prints a banner over the ns16550 UART, and halts via the SiFive-test finisher. Five revisions land in this iteration: (1) the four-phase ladder collapses to three landable phases, each with an automatable `cd xvisor && make fmt && make clippy && make run` gate; (2) `xvisor/build.rs` + `xvisor/.cargo/config.toml` are added to the architecture so Cargo picks up `linker.ld` and `boot.s` deterministically — mirroring `xam/xhal/build.rs`; (3) the "top-level Makefile" framing is dropped, since none exists in the worktree — the contract is `cd xvisor && make run`; (4) every `P1` / `P2` / `P3` / `P4` reference inside the promoted `## Spec` block is rewritten to durable vocabulary so the SPEC reads correctly when promoted verbatim to `specs/features/xvisor/framework/SPEC.md`; (5) `stvec` is pointed at a one-instruction `wfi` trampoline in `boot.s` so unintended traps loop visibly rather than triple-bouncing. The constraint set grows from 15 to 20, V-UT count grows from 3 to 4, and one new trade-off (T-7) records the trampoline choice.

## Log

[**Added**]

- `## Spec` Constraints: C-2 (build.rs wires linker + boot.s rebuild), C-3 (.cargo/config.toml pins target), C-8 (TrapFrame.regs has 32 slots), C-11 (stvec wfi trampoline), C-15 (boot.s is green-field new assembly).
- `## Trade-offs` T-7: rationale for the stvec wfi trampoline vs leaving stvec at reset.
- `## Validation` V-UT-4: clippy `-D warnings` plus `#![deny(missing_docs)]` enforces that every stub `mod` carries a `//!` doc comment.

[**Changed**]

- `## Implementation` collapses from four phases to three, each with an automatable `make fmt && make clippy && make run` gate from inside `xvisor/`.
- `## Spec` Constraints renumbered after additions; every roadmap phase name (`P0` / `P1` / `P2` / `P3` / `P4`) inside the Spec block replaced with durable substance ("when traps are added" / "future trap-entry caller" / "future feature that fills them in" / "this iteration").
- `## Spec` Constraint formerly C-11 (no heap) rewritten to drop the misleading "`#![no_std]` forbids" claim; replaced with a concrete absence-of-dependency statement.
- `## Spec` Constraint formerly C-15 (banner format) re-anchored on `const BANNER_FMT: &str` in `xvisor/src/main.rs`, referenced by both the print site and the V-IT-1 regex.
- `## Runtime` Failure Flow item 3 rewritten: unintended traps now loop on the `wfi` trampoline rather than triple-bouncing.
- `## Trade-offs` T-1 gains the sentence noting `-bios none` is a `_start` branch on the entry-privilege bit, costless if needed.
- `## Trade-offs` T-5 gains the explicit reference to the new green-field Constraint C-15.

[**Removed**]

- The "top-level Makefile" framing — there is no `/Users/anekoique/ProjectX/Makefile`. The PRD bullet survives unchanged (PRDs aren't rewritten in iteration loops); the PLAN reframes the user contract as `cd xvisor && make run`.
- The original Phase 1 "boots silently, operator quits with Ctrl-A X" gate (not automatable).

[**Unresolved**]

- None.

[**Response Matrix**]

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Phase 1 collapsed into Phase 2: revised plan is three phases — (1) crate skeleton + UART + banner + clean halt; (2) misa.H + PerCpu + TrapFrame + DTB capture; (3) module stubs + Makefile polish + docs. Each phase's gate is `cd xvisor && make fmt && make clippy && make run`. |
| Review | R-002 | Accepted | Architecture adds `xvisor/build.rs` and `xvisor/.cargo/config.toml`. `build.rs` mirrors `xam/xhal/build.rs`: emits `cargo:rustc-link-arg=-Txvisor/linker.ld`, `cargo:rerun-if-changed=src/boot.s`, and assembles `boot.s` via the `cc` crate. `.cargo/config.toml` pins `target = "riscv64gc-unknown-none-elf"`. Constraints C-2 and C-3 added. |
| Review | R-003 | Accepted | Top-level Makefile framing dropped. PLAN Summary, Implementation phase gates, and Outcome-equivalent language now say `cd xvisor && make run`. PRD itself unchanged (iteration loops don't rewrite PRDs); reframing documented here. |
| Review | R-004 | Accepted | Every "P1" / "P2" / "P3" / "P4" / "P0 has" reference inside the promoted `## Spec` block (Data Structure doc-comments, API Surface comments, C-13, C-14) is replaced with durable vocabulary. `## Trade-offs`, `## Runtime`, `## Implementation`, `## Validation` blocks may keep phase names — they are not promoted. |
| Review | R-005 | Accepted | New Constraint C-8: `TrapFrame.regs has 32 slots (x0..x31); x0's slot is preserved zero so frame[rd] indexing works for any encoded rd — xvisor/src/arch/riscv/trap.rs.` V-UT-2 unchanged. |
| Review | R-006 | Accepted | Option (b) adopted: `boot.s` installs a one-instruction `wfi` trampoline in `stvec` before calling `rust_main`. Constraint C-11 added; Failure Flow item 3 updated; Trade-off T-7 added with rationale. Does not violate NG-1 (single `wfi`, not a save/restore vector). |
| Review | R-007 | Accepted | `#![deny(missing_docs)]` pinned at the crate root. V-UT-4 added: `cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings` fails if any stub `mod` lacks a `//!` doc comment. C-19 Acceptance Mapping points to V-UT-4. |
| Review | R-008 | Accepted | C-12 (formerly C-11) reworded to drop the misleading "`#![no_std]` forbids" claim: `No heap is used; xvisor/Cargo.toml carries no allocator dependency and xvisor/src/main.rs has no extern crate alloc — xvisor/Cargo.toml, xvisor/src/main.rs.` |
| Review | R-009 | Accepted | Option (b) adopted: format string moved into `const BANNER_FMT: &str` in `xvisor/src/main.rs`, referenced from both the print site and V-IT-1's regex. Constraint C-20 (formerly C-15) rewritten accordingly. |
| Review | TR-1 | Accepted | Kept current choice (`-bios default`). T-1 gains one sentence noting that switching modes is a `_start` branch on the entry-privilege bit, costless if needed. |
| Review | TR-2 | Accepted | Kept current choice (direct MMIO UART). No PLAN change required. |
| Review | TR-3 | Accepted | Kept current choice (commit full module tree). R-007's validation gap closed via V-UT-4. |
| Review | TR-4 | Accepted | Kept current choice (SiFive-test finisher, no SBI SRST). No PLAN change required. |
| Review | TR-5 | Accepted | Kept current choice (`boot.s` separate file). New Constraint C-15 added: `boot.s is green-field new assembly created by this task; it does not modify any existing .S file — xvisor/src/boot.s.` Closes the loop with the executor's MEMORY.md "no modify asm without permission" rule. |
| Review | TR-6 | Accepted | Kept current choice (static `PER_CPU` array + `tp = &PER_CPU[hartid]`). No PLAN change required. |

---

## Spec

[**Goals**]

- G-1: Boot a HS-mode Rust binary under QEMU virt + OpenSBI fw_jump and reach `rust_main`.
- G-2: Print a banner naming hartid and DTB pointer over the ns16550 UART at `0x10000000`.
- G-3: Commit the xvisor module tree (`arch`, `mm`, `vcpu`, `vm`, `sbi`, `device`) as public vocabulary.
- G-4: Lock the per-hart convention: `tp = &PerCpu`, `sscratch` reserved for trap-entry swap.
- G-5: Halt cleanly via the SiFive-test finisher without invoking SBI SRST.

[**Non-goals**]

- NG-1: No trap entry, no `TrapFrame` save / restore code — only a one-instruction `wfi` trampoline in `stvec`.
- NG-2: No H-extension CSR writes (`hgatp`, `hstatus`, `hedeleg`, `hideleg`); only `misa.H` is read.
- NG-3: No heap allocator, no `extern crate alloc`, no multi-hart bring-up.

[**Architecture**]

```
xvisor/
├── Cargo.toml                              no_std binary; no allocator dependency
├── Makefile                                fmt / clippy / run / test / clean targets
├── linker.ld                               BASE = 0x80200000, sections + linker symbols
├── rust-toolchain.toml                     inherits ProjectX nightly pin
├── build.rs                                emits link-arg=-Txvisor/linker.ld + assembles boot.s
├── .cargo/
│   └── config.toml                         [build] target = "riscv64gc-unknown-none-elf"
└── src/
    ├── main.rs                             #![no_std] #![no_main] #![deny(missing_docs)]; BANNER_FMT, rust_main, panic handler
    ├── boot.s                              naked _start: misa.H check, tp/sp setup, BSS zero, DTB stash, stvec=trap_trampoline, wfi-trampoline body
    ├── arch/
    │   └── riscv/
    │       ├── mod.rs                      PerCpu, MAX_HARTS, STACK_SIZE_PER_HART, DTB_ADDR, percpu()
    │       ├── csr.rs                      S-mode CSR helpers (read_misa, misa_has_h, write_stvec)
    │       └── trap.rs                     TrapFrame layout + extern "C" fn trap_entry placeholder
    ├── mm/
    │   └── mod.rs                          stub with //! doc comment naming the future feature
    ├── vcpu/
    │   └── mod.rs                          stub with //! doc comment naming the future feature
    ├── vm/
    │   └── mod.rs                          stub with //! doc comment naming the future feature
    ├── sbi/
    │   └── mod.rs                          stub with //! doc comment naming the future feature
    └── device/
        ├── mod.rs                          MMIO base constants (UART0, SIFIVE_TEST)
        ├── uart.rs                         ns16550 putch, LSR THRE poll, UartWriter for write_fmt
        └── halt.rs                         terminate(code): SiFive-test finisher + wfi loop
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
pub const BANNER_FMT: &str = "xvisor: hello from HS-mode (hartid={}, dtb=0x{:x})\n";

#[unsafe(no_mangle)]
pub extern "C" fn rust_main(hartid: usize, dtb_ptr: usize) -> !;

// xvisor/src/arch/riscv/mod.rs
pub const MAX_HARTS: usize = 1;
pub const STACK_SIZE_PER_HART: usize = 64 * 1024;
pub static DTB_ADDR: core::sync::atomic::AtomicUsize;
pub fn percpu() -> &'static PerCpu;                          // reads tp
pub unsafe fn set_percpu(p: &'static PerCpu);                // writes tp; called only from _start

// xvisor/src/arch/riscv/csr.rs
pub fn read_misa() -> usize;
pub fn misa_has_h() -> bool;
pub unsafe fn write_stvec(addr: usize);                       // future trap-entry caller will use this

// xvisor/src/arch/riscv/trap.rs
unsafe extern "C" { pub fn trap_entry();  }                   // future trap-entry assembly defines body

// xvisor/src/device/uart.rs
pub fn putch(b: u8);
pub struct UartWriter;                                        // impl core::fmt::Write
pub fn writer() -> UartWriter;

// xvisor/src/device/halt.rs
pub fn terminate(code: HaltCode) -> !;                        // SiFive-test finisher + wfi
```

[**Constraints**]

- C-1: Crate entry is `_start` in `xvisor/src/boot.s`, marked `.text.boot`, placed first by `xvisor/linker.ld`.
- C-2: `xvisor/build.rs` emits `cargo:rustc-link-arg=-Txvisor/linker.ld` and `cargo:rerun-if-changed=src/boot.s`, and assembles `boot.s`.
- C-3: `xvisor/.cargo/config.toml` sets `[build] target = "riscv64gc-unknown-none-elf"` so `cargo build` from `xvisor/` selects the riscv target.
- C-4: Linker base address is `0x80200000`, matching OpenSBI fw_jump default — `xvisor/linker.ld`.
- C-5: `_start` checks `misa[7] == 1` and prints + halts on zero before any Rust call — `xvisor/src/boot.s`.
- C-6: `a1` (DTB pointer) is stashed into `DTB_ADDR` before any Rust call — `xvisor/src/boot.s`.
- C-7: `tp` holds `&PerCpu` for the running hart after `_start`; never reassigned outside boot — `xvisor/src/boot.s`.
- C-8: `TrapFrame.regs` has 32 slots (x0..x31); x0's slot is preserved zero so `frame[rd]` indexing works for any encoded rd — `xvisor/src/arch/riscv/trap.rs`.
- C-9: `sscratch` is reserved for trap-entry SP swap and left zero this iteration — documented in `xvisor/src/arch/riscv/trap.rs`.
- C-10: Stack size per hart is `64 KiB`, defined as `STACK_SIZE_PER_HART` — `xvisor/src/arch/riscv/mod.rs`.
- C-11: `boot.s` installs a one-instruction `wfi` trap trampoline in `stvec` before calling `rust_main`; unintended traps loop visibly — `xvisor/src/boot.s`.
- C-12: No heap is used; `xvisor/Cargo.toml` carries no allocator dependency and `xvisor/src/main.rs` has no `extern crate alloc` — `xvisor/Cargo.toml`, `xvisor/src/main.rs`.
- C-13: UART driver writes byte-at-a-time after LSR THRE poll at `0x1000_0005`, MMIO at `0x1000_0000` — `xvisor/src/device/uart.rs`.
- C-14: `terminate(code)` writes the SiFive-test finisher at `0x100000` then enters a `wfi` loop — `xvisor/src/device/halt.rs`.
- C-15: `boot.s` is green-field new assembly created by this task; it does not modify any existing `.S` file — `xvisor/src/boot.s`.
- C-16: No SBI SRST call is issued from xvisor; xvisor owns host shutdown — `xvisor/src/device/halt.rs`.
- C-17: `unsafe` blocks live only in `boot.s`, `arch/riscv/csr.rs`, `device/uart.rs`, and `device/halt.rs`.
- C-18: `MAX_HARTS = 1`; secondary harts spin in OpenSBI HSM until future multi-hart bring-up wakes them — `xvisor/src/arch/riscv/mod.rs`.
- C-19: Module tree commits `mm/`, `vcpu/`, `vm/`, `sbi/`, `device/` with one-line `//!` doc comments naming the future feature that fills them in; `#![deny(missing_docs)]` in `xvisor/src/main.rs` makes a missing comment a build error.
- C-20: Banner format is `xvisor/src/main.rs::BANNER_FMT` and matches V-IT-1's regex — `xvisor/src/main.rs`.

---

## Runtime

[**Main Flow**]

1. QEMU loads `xvisor.elf` at `0x80200000`; OpenSBI fw_jump runs in M-mode, then `mret`s into HS-mode at `_start` with `a0 = hartid`, `a1 = dtb-ptr`.
2. `_start` (in `boot.s`) reads `misa`; if bit 7 (H) is zero, falls through to the H-missing print + `wfi` loop.
3. `_start` stashes `a1` into `DTB_ADDR` (`AtomicUsize`, `Release` ordering).
4. `_start` sets `sp` to the top of the boot hart's slice of `.bss.stack`.
5. `_start` zeros `.bss` between `_bss_start` and `_bss_end`.
6. `_start` loads the address of `PER_CPU[0]` into `tp` and writes `hartid` / `stack_top` into it.
7. `_start` writes `stvec` to point at the `wfi` trampoline symbol in `boot.s`.
8. `_start` calls `rust_main(hartid, dtb_ptr)` with the original `a0` / `a1` values.
9. `rust_main` constructs a `UartWriter` and prints `BANNER_FMT` formatted with `hartid` and `dtb_ptr`.
10. `rust_main` calls `terminate(HaltCode::Success)`.
11. `terminate` writes `0x5555` to the SiFive-test finisher at `0x100000`; QEMU exits with status 0. The `wfi` loop is the fallback if the finisher write returns.

[**Failure Flow**]

1. **`misa.H == 0`**: `_start` prints `"xvisor: H-extension required; pass -cpu rv64,h=true to QEMU\n"` byte-by-byte via direct UART MMIO, then enters a `wfi` loop. Operator kills QEMU with Ctrl-A X.
2. **Rust panic** (e.g., format-write failure): panic handler in `main.rs` prints `"xvisor: panic: {msg}\n"` then calls `terminate(HaltCode::Failure)` (SiFive-test code `0x3333 | (1 << 16)`).
3. **Unintended trap before a real handler is wired**: `stvec` points at the `wfi` trap trampoline emitted in `boot.s`. The trap parks on `wfi` and loops visibly; the operator sees a hung VM rather than a triple-faulting one and kills with Ctrl-A X.

[**State Transitions**]

- `Reset → Boot` when OpenSBI `mret`s into `_start`.
- `Boot → Halt(Success)` when `rust_main` returns from the banner print and `terminate(Success)` runs.
- `Boot → Halt(Failure)` when `_start`'s `misa.H` check fails, when a Rust panic fires, or when the `wfi` trampoline traps.

---

## Implementation

[**Phase 1 — Crate skeleton + UART banner + clean halt (one landable cut)**]

- Create `xvisor/Cargo.toml` (no_std binary, no allocator dependency beyond core).
- Create `xvisor/rust-toolchain.toml` inheriting the project's nightly pin.
- Create `xvisor/.cargo/config.toml` with `[build] target = "riscv64gc-unknown-none-elf"`.
- Create `xvisor/build.rs` mirroring `xam/xhal/build.rs` shape: emit `cargo:rustc-link-arg=-Txvisor/linker.ld`, `cargo:rerun-if-changed=src/boot.s`, and assemble `boot.s` via the `cc` crate (build-dependency).
- Create `xvisor/linker.ld` with `BASE = 0x80200000`, sections `.text.boot` / `.text` / `.rodata` / `.data` / `.bss` (with `.bss.stack` distinct), symbols `_start`, `_stack_start`, `_stack_end`, `_bss_start`, `_bss_end`, `_hyp_end`.
- Create `xvisor/src/main.rs` with `#![no_std]` `#![no_main]` `#![deny(missing_docs)]`, the `BANNER_FMT` const, a panic handler that prints and calls `terminate(HaltCode::Failure)`, and `rust_main(hartid, dtb_ptr)` that writes the banner and calls `terminate(HaltCode::Success)`.
- Create `xvisor/src/boot.s` (new green-field assembly): naked `_start`, sets `sp`, zeros BSS, calls `rust_main(a0, a1)`. No misa check yet (Phase 2), no DTB stash yet (Phase 2), no `tp` setup yet (Phase 2). `stvec` is still at reset value at this point — acceptable because no trap should fire in the banner-and-halt path.
- Create `xvisor/src/device/mod.rs` with MMIO base constants (`UART0_BASE = 0x1000_0000`, `SIFIVE_TEST_BASE = 0x10_0000`).
- Create `xvisor/src/device/uart.rs` porting `xam/xhal/src/platform/xemu/console.rs` to Rust HS-mode: `putch(b: u8)`, `UartWriter: core::fmt::Write`, `writer()`.
- Create `xvisor/src/device/halt.rs` with `terminate(code: HaltCode)` writing SiFive-test magic (`0x5555` on success, `0x3333 | (code as u32) << 16` on failure) then a `wfi` loop.
- Create `xvisor/Makefile` with `fmt`, `clippy`, `clean`, `run` (cargo build + QEMU launch with the recommended flags), `test` targets. `run` greps QEMU output for the V-IT-1 banner regex.
- **Gate**: `cd xvisor && make fmt && make clippy && make run` returns 0 — clippy is `-D warnings`, QEMU prints the banner, exits with status 0, regex matches.

[**Phase 2 — `misa.H` check, `PerCpu`, `TrapFrame`, DTB capture, `stvec` trampoline**]

- Create `xvisor/src/arch/riscv/mod.rs` with `MAX_HARTS`, `STACK_SIZE_PER_HART`, `PerCpu` struct, `DTB_ADDR: AtomicUsize`, `percpu()` / `set_percpu()`, and a static `PER_CPU: [PerCpu; MAX_HARTS]`.
- Create `xvisor/src/arch/riscv/csr.rs` with `read_misa()`, `misa_has_h()`, and `unsafe fn write_stvec(addr: usize)` (used by future trap entry).
- Create `xvisor/src/arch/riscv/trap.rs` with the `TrapFrame` `#[repr(C)]` struct (regs[32], sepc, scause, stval, sstatus), `extern "C" { pub fn trap_entry(); }` declaration, and a doc comment specifying the `sscratch ↔ sp` swap convention.
- Update `xvisor/src/boot.s`: insert `misa.H` check at top with literal-string print + `wfi` on failure; stash `a1` into `DTB_ADDR` via raw store; load `tp` with `PER_CPU[0]` address; write `hartid` / `stack_top` into `*tp`; emit a `.text.trap_trampoline` symbol whose body is a single `wfi` followed by a `j .` self-loop, and write its address into `stvec` before calling `rust_main`.
- Update `xvisor/src/main.rs` banner to read `DTB_ADDR` via `Acquire` load and report it via `BANNER_FMT`.
- Add `const _: () = assert!(core::mem::size_of::<PerCpu>().is_power_of_two());` in `arch/riscv/mod.rs`.
- Add `const _: () = assert!(core::mem::size_of::<TrapFrame>() == 36 * core::mem::size_of::<usize>());` in `arch/riscv/trap.rs`.
- Add `const _: () = assert!(STACK_SIZE_PER_HART == 64 * 1024);` in `arch/riscv/mod.rs`.
- **Gate**: `cd xvisor && make fmt && make clippy && make run` still returns 0. Banner now reports a non-zero `dtb=0x<actual>`. Manual smoke: `qemu-system-riscv64 -cpu rv64` (no `h=true`) prints the H-missing message instead of the banner.

[**Phase 3 — Module stubs, Makefile polish, README**]

- Create `xvisor/src/mm/mod.rs`, `vcpu/mod.rs`, `vm/mod.rs`, `sbi/mod.rs` each containing a single `//!` doc comment naming the future feature that fills it in.
- Wire `mod` declarations in `xvisor/src/main.rs`. `#![deny(missing_docs)]` at the crate root makes a missing `//!` a build error.
- Polish `xvisor/Makefile` so `run` aliases produce identical builds; add `test` target wired to `cargo test --manifest-path xvisor/Cargo.toml` (host-runnable const-asserts; no-op otherwise).
- Add `xvisor/README.md` with the `make run` invocation, the deferred-feature map, and a link to `docs/XVISOR.md`.
- **Gate**: `cd xvisor && make fmt && make clippy && make run && make test` all return 0 — clippy is `-D warnings` and trips on any stub without a `//!` doc comment.

---

## Trade-offs

- T-1: `-bios default` (OpenSBI fw_jump, HS-mode entry) vs `-bios none` (M-mode entry, payload owns SBI). Chose default — entry is HS-mode, no PMP / mideleg / medeleg setup, fits `docs/XVISOR.md:121-138` (OpenSBI sits below xvisor). Switching modes later is a `_start` branch on the entry-privilege bit, costless if needed.
- T-2: Hypervisor-owned direct MMIO UART vs SBI DBCN early-print. Chose direct MMIO — mirrors `xam/xhal/src/platform/xemu/console.rs` line-for-line, removes a runtime dependency on OpenSBI's console state, foreclosure-neutral for the future Linux passthrough path.
- T-3: Commit the full module tree (`mm/`, `vcpu/`, `vm/`, `sbi/`) in this iteration vs add directories incrementally. Chose all-in — fixes the public name vocabulary without forcing implementation. Cost: empty stub files; benefit: zero renames downstream, which hvisor and hypocaust-2 retrofitted painfully.
- T-4: `wfi`-loop + SiFive-test finisher halt vs SBI SRST call. Chose SiFive-test direct — a Type-1 owns the machine; asking OpenSBI to shut down inverts the layering. Observable QEMU exit is identical either way (OpenSBI SRST on virt also writes the SiFive-test finisher), so the code-path delta is purely about honest layering.
- T-5: Hand-written `boot.s` vs `naked_asm!` in Rust. Chose `boot.s` (green-field new assembly file) — keeps `misa.H` check + DTB stash + BSS zero + `tp` setup + trampoline install in one auditable place; matches every comparable Rust hypervisor (salus, hvisor, hypocaust-2). C-15 records that this is brand-new assembly and does not modify any existing `.S` file.
- T-6: `PerCpu` placement: static array indexed by hartid vs per-hart stack-top slot reached via `tp`. Chose static array + `tp = &PER_CPU[hartid]` — single source of truth, easy to size at compile time, consistent with xemu's `multi-hart` SPEC (`HartId(u32)` + `Vec<Core>`). Cost: `MAX_HARTS = 1` static today; revisited when multi-hart lands.
- T-7: `stvec` at reset (whatever OpenSBI left it) vs `stvec` pointed at a one-instruction `wfi` trampoline in `boot.s`. Chose the trampoline — three lines of asm in the green-field `boot.s` already being written, NG-1 untouched (single `wfi`, not a save/restore vector), and an unintended early-Rust trap now parks visibly on `wfi` instead of triple-bouncing through whatever garbage `stvec` was left pointing at. The alternative ("document the silent hang in README") trades operational pain for one less asm line — not worth it.

---

## Validation

[**Unit Tests**]

- V-UT-1: `const _: () = assert!(core::mem::size_of::<PerCpu>().is_power_of_two());` in `xvisor/src/arch/riscv/mod.rs` — compile-time guard on alignment-friendly indexing.
- V-UT-2: `const _: () = assert!(core::mem::size_of::<TrapFrame>() == 36 * core::mem::size_of::<usize>());` in `xvisor/src/arch/riscv/trap.rs` — compile-time guard on frame layout (32 GPRs + sepc / scause / stval / sstatus).
- V-UT-3: `const _: () = assert!(STACK_SIZE_PER_HART == 64 * 1024);` in `xvisor/src/arch/riscv/mod.rs` — pins the per-hart stack budget.
- V-UT-4: `cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings` fails if any stub `mod` lacks a `//!` doc comment, enforced via `#![deny(missing_docs)]` in `xvisor/src/main.rs`.

[**Integration Tests**]

- V-IT-1: `cd xvisor && make run` launches QEMU with the recommended flags, captures stdout, asserts the regex `^xvisor: hello from HS-mode \(hartid=0, dtb=0x[0-9a-f]+\)$` appears, and confirms QEMU exit code is 0. The regex source-of-truth lives next to `BANNER_FMT` in `xvisor/src/main.rs`.
- V-IT-2: `cd xvisor && make test` invokes `cargo test --manifest-path xvisor/Cargo.toml` and stays green (no-op or host-runnable const-assert validation).

[**Failure / Robustness**]

- V-F-1: Operator runs `qemu-system-riscv64 -machine virt -cpu rv64 -bios default -kernel xvisor.elf -nographic` (no `h=true`). Expected stdout contains `"xvisor: H-extension required; pass -cpu rv64,h=true to QEMU"`. Manual smoke documented in `xvisor/README.md`; not automated in this iteration.
- V-F-2: A forced panic in `rust_main` (debug-only, gated behind a `#[cfg(test_panic)]` flag never enabled in CI) prints `"xvisor: panic: …"` and calls `terminate(HaltCode::Failure)`, producing a non-zero QEMU exit code.
- V-F-3: A forced illegal-instruction in `rust_main` (debug-only, gated behind a `#[cfg(test_trap)]` flag never enabled in CI) traps to the `stvec` trampoline; QEMU hangs on `wfi` rather than triple-bouncing. Manual smoke documented in `xvisor/README.md`.

[**Edge Cases**]

- V-E-1: QEMU launched with `-smp 2` — secondary harts spin in OpenSBI HSM idle (never reach `_start`); banner still prints exactly once for hart 0. Manual smoke; documents the C-18 boundary.
- V-E-2: DTB pointer captured before Rust runs — verified by banner output showing a non-zero `dtb=0x...` value (QEMU virt typically places it around `0xbfe...`).
- V-E-3: `cd xvisor && cargo build` invoked from a clean tree (no `target/`) — confirms `linker.ld` and `boot.s` are picked up via `xvisor/build.rs` and `xvisor/.cargo/config.toml`; no missing-symbol link errors.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (HS-mode boot to rust_main) | V-IT-1 (QEMU launch + banner regex implies `rust_main` ran) |
| G-2 (UART banner) | V-IT-1 (regex includes hartid + dtb) |
| G-3 (full module tree committed) | V-UT-4 (missing-docs deny trips on missing `//!`); `make clippy -- -D warnings` requires every `mod` to be reachable |
| G-4 (per-hart `tp` / `sscratch` convention) | V-UT-1 (`PerCpu` size assert), V-IT-1 (banner reads `hartid` via `percpu()`) |
| G-5 (SiFive-test halt, no SBI SRST) | V-IT-1 (QEMU exit code 0); manual code review of `device/halt.rs` for absence of any `ecall` to EID `0x53525354` |
| C-1 (entry `_start` in `boot.s`) | V-IT-1 (link succeeds, banner prints) |
| C-2 (build.rs wires linker + boot.s) | V-E-3 (clean build picks up linker + asm) |
| C-3 (.cargo/config.toml pins target) | V-E-3 (clean `cargo build` from `xvisor/` selects riscv target) |
| C-4 (BASE = 0x80200000) | V-IT-1 (OpenSBI fw_jump jumps to that address; banner is the post-handoff proof) |
| C-5 (`misa.H` check) | V-F-1 (manual smoke without `h=true`) |
| C-6 (DTB stashed into `DTB_ADDR`) | V-E-2 (banner shows non-zero dtb), V-IT-1 |
| C-7 (`tp = &PerCpu`) | V-UT-1, V-IT-1 (banner reads `percpu().hartid`) |
| C-8 (TrapFrame.regs has 32 slots) | V-UT-2 (frame size assert) |
| C-9 (`sscratch` reserved) | Doc-comment review of `arch/riscv/trap.rs`; no functional test this iteration |
| C-10 (STACK_SIZE = 64 KiB) | V-UT-3 |
| C-11 (`wfi` trap trampoline in `stvec`) | V-F-3 (manual forced-trap smoke); V-IT-1 (banner path proves trampoline install does not regress normal boot) |
| C-12 (no heap) | `cargo clippy -- -D warnings`; absence of `extern crate alloc` enforced by build |
| C-13 (UART driver shape) | V-IT-1 (banner emerges over UART) |
| C-14 (SiFive-test finisher path) | V-IT-1 (QEMU exit code 0) |
| C-15 (boot.s green-field) | Manual code review — no diff against any pre-existing `.S` |
| C-16 (no SBI SRST) | Manual review of `device/halt.rs` |
| C-17 (`unsafe` scope) | `cargo clippy --all-targets -- -D warnings -W clippy::undocumented_unsafe_blocks` |
| C-18 (MAX_HARTS = 1) | V-E-1 (operator `-smp 2` smoke) |
| C-19 (stub modules with `//!` doc comments) | V-UT-4 (`#![deny(missing_docs)]` trips clippy on any missing `//!`) |
| C-20 (banner format anchored on BANNER_FMT) | V-IT-1 (regex match against the format constant) |
