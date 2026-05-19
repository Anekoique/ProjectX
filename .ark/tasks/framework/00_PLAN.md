# `framework` PLAN `00`

> Status: Draft
> Feature: `framework`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: `none`
> - Review: `none`

---

## Summary

This PLAN proposes the P0 deliverable for `docs/XVISOR.md`: a `xvisor/` Rust crate that boots in HS-mode on `qemu-system-riscv64 -machine virt -cpu rv64,h=true` after OpenSBI fw_jump, prints a banner over the ns16550 UART, and halts via the SiFive-test finisher. It commits the durable framework vocabulary every later xvisor phase inherits — module tree (`arch/riscv/`, `mm/`, `vcpu/`, `vm/`, `sbi/`, `device/`), per-hart slot via `tp = &PerCpu`, `TrapFrame` field order, linker symbol set, HS-mode entry contract (`a0 = hartid`, `a1 = dtb-ptr`), and a `wfi`-based halt path that owns the machine rather than delegating to SBI SRST. P0 leaves `mm/`, `vcpu/`, `vm/`, and `sbi/` as stubs whose `mod.rs` files name the phase that fills them in. The work is decomposed into four vertical-slice phases, each ending with `make fmt && make clippy && make xvisor-run` green. The promoted SPEC under `.ark/specs/features/xvisor/framework/SPEC.md` makes these decisions binding for P1+ without forcing P0 to implement traps, H-ext CSRs, or G-stage paging.

## Log `None in 00_PLAN`

*None in 00_PLAN.*

---

## Spec

[**Goals**]

- G-1: Boot a HS-mode Rust binary under QEMU virt + OpenSBI fw_jump and reach `rust_main`.
- G-2: Print a banner naming hartid and DTB pointer over the ns16550 UART at `0x10000000`.
- G-3: Commit the full xvisor module tree (`arch`, `mm`, `vcpu`, `vm`, `sbi`, `device`) as the public vocabulary.
- G-4: Lock the per-hart convention: `tp = &PerCpu`, `sscratch` reserved for trap-entry swap.
- G-5: Halt cleanly via the SiFive-test finisher without invoking SBI SRST.

[**Non-goals**]

- NG-1: No trap entry, no `stvec` write, no `TrapFrame` save/restore code (P1 owns).
- NG-2: No H-ext CSR writes (`hgatp`, `hstatus`, `hedeleg`, `hideleg`); only `misa.H` is read (P2 owns).
- NG-3: No heap allocator, no `extern crate alloc`, no multi-hart bring-up (P1 / P6 own respectively).

[**Architecture**]

```
xvisor/
├── Cargo.toml                              no_std bin, target = riscv64gc-unknown-none-elf
├── Makefile                                xvisor / xvisor-run targets, delegates to cargo
├── linker.ld                               BASE = 0x80200000, sections + linker symbols
├── rust-toolchain.toml                     inherits ProjectX nightly pin
└── src/
    ├── main.rs                             #![no_std] #![no_main]; calls rust_main; panic handler
    ├── boot.s                              naked _start: misa.H check, tp/sp setup, BSS zero, DTB stash
    ├── arch/
    │   └── riscv/
    │       ├── mod.rs                      PerCpu, MAX_HARTS, STACK_SIZE_PER_HART, DTB_ADDR
    │       ├── csr.rs                      S-mode CSR read/write helpers (read_misa, write_stvec, ...)
    │       └── trap.rs                     TrapFrame layout + extern "C" fn trap_entry placeholder (P1)
    ├── mm/
    │   └── mod.rs                          stub: //! hyp heap + G-stage builder (filled in P3)
    ├── vcpu/
    │   └── mod.rs                          stub: //! vCPU register file + run loop (filled in P3)
    ├── vm/
    │   └── mod.rs                          stub: //! per-guest VM struct (filled in P3)
    ├── sbi/
    │   └── mod.rs                          stub: //! inbound SBI ecall dispatch (filled in P4)
    └── device/
        ├── mod.rs                          MMIO base constants (UART0, SIFIVE_TEST)
        ├── uart.rs                         ns16550 _putch, LSR THRE poll, Writer for write_fmt
        └── halt.rs                         terminate(code): SiFive-test finisher + wfi loop
```

[**Data Structure**]

```rust
/// Per-hart slot pointed to by `tp`. P0 hosts hart 0 only; MAX_HARTS = 1.
/// Power-of-two size for cheap indexing in future P6 multi-hart.
#[repr(C, align(64))]
pub struct PerCpu {
    pub hartid:     usize,
    pub stack_top:  *mut u8,
    _reserved:      [usize; 6],
}

/// Trap context committed in P0; populated by trap_entry assembly in P1.
/// Field order is the contract — P1 trap.S indexes by `offset_of!`.
#[repr(C)]
pub struct TrapFrame {
    pub regs:    [usize; 32],   // x0..x31
    pub sepc:    usize,
    pub scause:  usize,
    pub stval:   usize,
    pub sstatus: usize,
}

/// Halt exit code carried into SiFive-test finisher.
#[repr(i32)]
pub enum HaltCode { Success = 0, Failure = 1 }
```

[**API Surface**]

```rust
// xvisor/src/main.rs
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
pub unsafe fn write_stvec(addr: usize);                       // P1 will call

// xvisor/src/arch/riscv/trap.rs
unsafe extern "C" { pub fn trap_entry();  }                   // P1 defines body in trap.S

// xvisor/src/device/uart.rs
pub fn putch(b: u8);
pub struct UartWriter;                                        // impl core::fmt::Write
pub fn writer() -> UartWriter;

// xvisor/src/device/halt.rs
pub fn terminate(code: HaltCode) -> !;                        // SiFive-test finisher + wfi
```

[**Constraints**]

- C-1: Crate entry is `_start` in `xvisor/src/boot.s`, marked `.text.boot`, placed first by `xvisor/linker.ld`.
- C-2: Linker base address is `0x80200000`, matching OpenSBI fw_jump default — `xvisor/linker.ld`.
- C-3: `_start` checks `misa[7] == 1` and prints + halts on zero before any Rust call — `xvisor/src/boot.s`.
- C-4: `a1` (DTB pointer) is stashed into `DTB_ADDR` before any Rust call — `xvisor/src/boot.s`.
- C-5: `tp` holds `&PerCpu` for the running hart after `_start`; never reassigned outside boot — `xvisor/src/boot.s`.
- C-6: `sscratch` is reserved for trap-entry SP swap; P0 leaves it zero — documented in `xvisor/src/arch/riscv/trap.rs`.
- C-7: Stack size per hart is `64 KiB`, defined as `STACK_SIZE_PER_HART` — `xvisor/src/arch/riscv/mod.rs`.
- C-8: UART driver writes byte-at-a-time after LSR THRE poll at `0x1000_0005`, MMIO at `0x1000_0000` — `xvisor/src/device/uart.rs`.
- C-9: `terminate(code)` writes the SiFive-test finisher at `0x100000` then enters a `wfi` loop — `xvisor/src/device/halt.rs`.
- C-10: No SBI SRST call is issued from xvisor; xvisor owns host shutdown — `xvisor/src/device/halt.rs`.
- C-11: P0 has no heap; `#![no_std]` crate forbids `extern crate alloc` — `xvisor/Cargo.toml`, `xvisor/src/main.rs`.
- C-12: `unsafe` blocks live only in `boot.s`, `arch/riscv/csr.rs`, `device/uart.rs`, and `device/halt.rs`.
- C-13: `MAX_HARTS = 1` for P0; secondary harts spin in OpenSBI HSM and are never woken — `xvisor/src/arch/riscv/mod.rs`.
- C-14: Module tree commits `mm/`, `vcpu/`, `vm/`, `sbi/`, `device/` with one-line `//!` doc comments naming the phase that fills them in.
- C-15: Banner format is `xvisor: hello from HS-mode (hartid={n}, dtb=0x{addr:x})\n` — `xvisor/src/main.rs`.

---

## Runtime

[**Main Flow**]

1. QEMU loads `xvisor.elf` at `0x80200000`; OpenSBI fw_jump runs in M-mode, then `mret`s into HS-mode at `_start` with `a0 = hartid`, `a1 = dtb-ptr`.
2. `_start` (in `boot.s`) reads `misa`; if bit 7 (H) is zero, falls through to the H-missing print + `wfi` loop.
3. `_start` stashes `a1` into `DTB_ADDR` (`AtomicUsize`, `Release` ordering).
4. `_start` sets `sp` to the top of the boot hart's slice of `.bss.stack`.
5. `_start` zeros `.bss` between `_bss_start` and `_bss_end`.
6. `_start` loads the address of `PER_CPU[0]` into `tp` and writes `hartid` / `stack_top` into it.
7. `_start` calls `rust_main(hartid, dtb_ptr)` with the original `a0` / `a1` values.
8. `rust_main` constructs a `UartWriter` and prints the banner string with `hartid` and `dtb_ptr`.
9. `rust_main` calls `terminate(HaltCode::Success)`.
10. `terminate` writes `0x5555` to the SiFive-test finisher at `0x100000`; QEMU exits with status 0. The `wfi` loop is the fallback if the finisher write returns.

[**Failure Flow**]

1. **`misa.H == 0`**: `_start` prints `"xvisor: H-extension required; pass -cpu rv64,h=true to QEMU\n"` byte-by-byte via direct UART MMIO, then enters a `wfi` loop. QEMU does not exit; operator kills with Ctrl-A X.
2. **Rust panic** (e.g., format-write failure): panic handler in `main.rs` prints `"xvisor: panic: {msg}\n"` then calls `terminate(HaltCode::Failure)` (SiFive-test code `0x3333 | (1 << 16)`).
3. **Trap before `stvec` is wired**: any exception in P0 vectors to `stvec = 0`, fetching from address 0 traps again — operator-visible silent hang, mitigated by C-3's `misa.H` check catching the most common cause.

[**State Transitions**]

- `Reset → Boot` when OpenSBI `mret`s into `_start`.
- `Boot → Halt(Success)` when `rust_main` returns from the banner print and `terminate(Success)` runs.
- `Boot → Halt(Failure)` when `_start`'s `misa.H` check fails or a Rust panic fires.

---

## Implementation

[**Phase 1 — Crate skeleton boots silently**]

- Create `xvisor/Cargo.toml` (no_std binary, no dependencies beyond core).
- Create `xvisor/rust-toolchain.toml` inheriting `nightly-2026-03-15`.
- Create `xvisor/linker.ld` with `BASE = 0x80200000`, sections `.text.boot` / `.text` / `.rodata` / `.data` / `.bss` (with `.bss.stack` distinct), symbols `_start`, `_stack_start`, `_stack_end`, `_bss_start`, `_bss_end`, `_hyp_end`.
- Create `xvisor/src/main.rs` with `#![no_std]` `#![no_main]`, a panic handler that enters a `wfi` loop, and an empty `rust_main` that calls a placeholder halt.
- Create `xvisor/src/boot.s` (new green-field assembly): naked `_start`, sets `sp`, zeros BSS, calls `rust_main(a0, a1)`.
- Create `xvisor/Makefile` with `xvisor` (cargo build) and `xvisor-run` (cargo build + qemu-system-riscv64 launch) targets.
- Add top-level wrapper targets `make xvisor` / `make xvisor-run` to `/Users/anekoique/ProjectX/Makefile` (or equivalent — confirm during impl) that delegate to `xvisor/Makefile`.
- **Gate**: `cd xvisor && make fmt && cargo clippy --target riscv64gc-unknown-none-elf -- -D warnings && make xvisor-run` boots, QEMU hangs at `wfi`, operator quits with Ctrl-A X. No output yet (UART driver is P2).

[**Phase 2 — UART banner + clean halt**]

- Create `xvisor/src/device/mod.rs` with MMIO base constants (`UART0_BASE = 0x1000_0000`, `SIFIVE_TEST_BASE = 0x10_0000`).
- Create `xvisor/src/device/uart.rs` porting `xam/xhal/src/platform/xemu/console.rs:1-12` to Rust HS-mode: `putch(b: u8)`, `UartWriter: core::fmt::Write`, `writer()`.
- Create `xvisor/src/device/halt.rs` with `terminate(code: HaltCode)` writing SiFive-test magic (`0x5555` on success, `0x3333 | (code as u32) << 16` on failure) then `wfi` loop.
- Update `xvisor/src/main.rs`: `rust_main(hartid, dtb_ptr)` writes the banner via `core::fmt::Write::write_fmt`, then calls `terminate(HaltCode::Success)`.
- Update panic handler to print `"xvisor: panic: …\n"` then `terminate(HaltCode::Failure)`.
- **Gate**: `make xvisor-run` prints `xvisor: hello from HS-mode (hartid=0, dtb=0x...)` and QEMU exits with status 0. Add an integration smoke that greps the output for the banner regex.

[**Phase 3 — H-ext check, PerCpu, TrapFrame**]

- Create `xvisor/src/arch/riscv/mod.rs` with `MAX_HARTS`, `STACK_SIZE_PER_HART`, `PerCpu` struct, `DTB_ADDR: AtomicUsize`, `percpu()` / `set_percpu()`, and a static `PER_CPU: [PerCpu; MAX_HARTS]`.
- Create `xvisor/src/arch/riscv/csr.rs` with `read_misa()`, `misa_has_h()`, and `unsafe fn write_stvec(addr: usize)` (P1 caller).
- Create `xvisor/src/arch/riscv/trap.rs` with the `TrapFrame` `#[repr(C)]` struct (regs[32], sepc, scause, stval, sstatus) and `extern "C" { pub fn trap_entry(); }` declaration. Doc-comment specifies `sscratch ↔ sp` swap convention.
- Update `xvisor/src/boot.s`: insert `misa.H` check at top with literal-string print + `wfi` on failure; stash `a1` into `DTB_ADDR` via raw store (assembly); load `tp` with `PER_CPU[0]` address; write `hartid` / `stack_top` into `*tp`.
- Update `xvisor/src/main.rs` banner to read `DTB_ADDR` via `Acquire` load and report it.
- Add `const _: () = assert!(core::mem::size_of::<PerCpu>().is_power_of_two());` in `arch/riscv/mod.rs`.
- **Gate**: `make xvisor-run` still prints the banner. Banner now reports `hartid=0, dtb=0x<actual>`. Manual smoke: `qemu-system-riscv64 -cpu rv64` (no `h=true`) prints the H-missing message instead of the banner.

[**Phase 4 — Module stubs, Makefile polish, docs**]

- Create `xvisor/src/mm/mod.rs`, `vcpu/mod.rs`, `vm/mod.rs`, `sbi/mod.rs` each containing a single `//!` doc comment naming the filling phase.
- Wire `mod` declarations in `xvisor/src/main.rs`.
- Polish `xvisor/Makefile` with `fmt`, `clippy`, `clean`, `run` (alias of `xvisor-run`), `test` (cargo test on host where applicable).
- Add `xvisor/README.md` with the `make xvisor-run` invocation, link to `docs/XVISOR.md`, and the deferred-phase map.
- Confirm `make fmt && make clippy && make run` and `make test` from the new top-level wrappers all stay green.
- **Gate**: `make fmt` (project root) succeeds; `make clippy` succeeds with `-D warnings` for the new crate; `make xvisor-run` still passes the banner regex check; `make test` is at minimum a no-op (no host-runnable tests beyond const-asserts).

---

## Trade-offs

- T-1: `-bios default` (OpenSBI fw_jump, HS-mode entry) vs `-bios none` (M-mode entry, payload owns SBI). Chose default — entry is HS-mode, no PMP/mideleg/medeleg setup, fits docs/XVISOR.md:121-138 (OpenSBI sits below xvisor). Cost: any future "xvisor as M-mode firmware" path needs a new boot mode; deemed unlikely given xemu's role.
- T-2: Hypervisor-owned direct MMIO UART (this PLAN) vs SBI DBCN early-print. Chose direct MMIO — mirrors `xam/xhal/src/platform/xemu/console.rs` line-for-line, removes a runtime dependency on OpenSBI's console state, foreclosure-neutral for P5 Linux passthrough and P6/P7 trap-and-emulate.
- T-3: Commit the full module tree (`mm/`, `vcpu/`, `vm/`, `sbi/`) in P0 vs add directories incrementally. Chose all-in — fixes the public name vocabulary (the SPEC's tree of names) without forcing implementation. Cost: empty stub files; benefit: zero renames in P1-P6, which hvisor and hypocaust-2 retrofitted painfully.
- T-4: `wfi`-loop + SiFive-test finisher halt vs SBI SRST call. Chose SiFive-test direct — a Type-1 owns the machine; asking OpenSBI to shut down inverts the layering. Observable QEMU exit is identical either way (OpenSBI SRST on virt also writes the SiFive-test finisher), so the code-path delta is purely about honest layering.
- T-5: Hand-written `boot.s` vs `naked_asm!` in Rust. Chose `boot.s` (green-field new assembly file) — keeps `misa.H` check + DTB stash + BSS zero + `tp` setup in one auditable place; matches every comparable Rust hypervisor (salus, hvisor, hypocaust-2). Per project rule, creating new assembly in a brand-new crate is permitted; this file does not modify any existing `.S`.
- T-6: `PerCpu` placement: static array indexed by hartid vs per-hart stack-top slot reached via `tp`. Chose static array + `tp = &PER_CPU[hartid]` — single source of truth, easy to size at compile time, consistent with xemu's `multi-hart` SPEC (`HartId(u32)` + `Vec<Core>`). Cost: `MAX_HARTS = 1` static today; tradeoff revisited at P6.

---

## Validation

[**Unit Tests**]

- V-UT-1: `const _: () = assert!(core::mem::size_of::<PerCpu>().is_power_of_two());` in `xvisor/src/arch/riscv/mod.rs` — compile-time guard on alignment-friendly indexing.
- V-UT-2: `const _: () = assert!(core::mem::size_of::<TrapFrame>() == 36 * core::mem::size_of::<usize>());` in `xvisor/src/arch/riscv/trap.rs` — compile-time guard on frame layout (32 GPRs + sepc/scause/stval/sstatus).
- V-UT-3: `const _: () = assert!(STACK_SIZE_PER_HART == 64 * 1024);` in `xvisor/src/arch/riscv/mod.rs` — pins the per-hart stack budget at the value documented in C-7.

[**Integration Tests**]

- V-IT-1: `make xvisor-run` (script under `xvisor/Makefile`) launches QEMU with the recommended flags, captures stdout, asserts the regex `^xvisor: hello from HS-mode \(hartid=0, dtb=0x[0-9a-f]+\)$` appears, and confirms QEMU exit code is 0.
- V-IT-2: `make test` from the project root invokes `cargo test --manifest-path xvisor/Cargo.toml` and stays green (no-op or host-runnable const-assert validation).

[**Failure / Robustness**]

- V-F-1: Operator runs `qemu-system-riscv64 -machine virt -cpu rv64 -bios default -kernel xvisor.elf -nographic` (no `h=true`). Expected stdout contains `"xvisor: H-extension required; pass -cpu rv64,h=true to QEMU"`. Manual smoke documented in `xvisor/README.md`; not automated in P0 (script harness lands in P1).
- V-F-2: A forced panic in `rust_main` (debug-only, gated behind a `#[cfg(test_panic)]` flag never enabled in CI) prints `"xvisor: panic: …"` and calls `terminate(HaltCode::Failure)`, producing QEMU exit code != 0.

[**Edge Cases**]

- V-E-1: QEMU launched with `-smp 2` — secondary harts spin in OpenSBI HSM idle (never reach `_start`); banner still prints exactly once for hart 0. Manual smoke; documents the C-13 boundary.
- V-E-2: DTB pointer captured before Rust runs — verified by banner output showing a non-zero `dtb=0x...` value (QEMU virt typically places it around `0xbfe...`).
- V-E-3: `make xvisor` invoked from a clean tree (no `target/`) — confirms `linker.ld` and `boot.s` are picked up by Cargo via the build configuration; no missing-symbol link errors.

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (HS-mode boot to rust_main) | V-IT-1 (QEMU launch + banner regex implies `rust_main` ran) |
| G-2 (UART banner) | V-IT-1 (regex includes hartid + dtb) |
| G-3 (full module tree committed) | V-UT-2, V-UT-3 (compile-time guards live in committed module files); `make clippy -- -D warnings` requires every `mod` to be reachable |
| G-4 (per-hart `tp`/`sscratch` convention) | V-UT-1 (`PerCpu` size assert), V-IT-1 (banner reads `hartid` via `percpu()`) |
| G-5 (SiFive-test halt, no SBI SRST) | V-IT-1 (QEMU exit code 0); manual code review of `device/halt.rs` for absence of any `ecall` to EID `0x53525354` |
| C-1 (entry `_start` in `boot.s`) | V-IT-1 (link succeeds, banner prints) |
| C-2 (BASE = 0x80200000) | V-IT-1 (OpenSBI fw_jump jumps to that address; banner is the post-handoff proof) |
| C-3 (`misa.H` check) | V-F-1 (manual smoke without `h=true`) |
| C-4 (DTB stashed into `DTB_ADDR`) | V-E-2 (banner shows non-zero dtb), V-IT-1 |
| C-5 (`tp = &PerCpu`) | V-UT-1, V-IT-1 (banner reads `percpu().hartid`) |
| C-6 (`sscratch` reserved) | Doc-comment review of `arch/riscv/trap.rs`; no functional test in P0 |
| C-7 (STACK_SIZE = 64 KiB) | V-UT-3 |
| C-8 (UART driver shape) | V-IT-1 (banner emerges over UART) |
| C-9 (SiFive-test finisher path) | V-IT-1 (QEMU exit code 0) |
| C-10 (no SBI SRST) | Manual review of `device/halt.rs` |
| C-11 (no heap) | `cargo clippy -- -D warnings`; absence of `extern crate alloc` enforced by build |
| C-12 (`unsafe` scope) | `cargo clippy --all-targets -- -D warnings -W clippy::undocumented_unsafe_blocks` (Phase 4) |
| C-13 (MAX_HARTS = 1) | V-E-1 (operator `-smp 2` smoke) |
| C-14 (stub modules committed) | V-UT-2 / V-UT-3 references compile only if `mod` declarations are present |
| C-15 (banner format) | V-IT-1 (regex match) |
