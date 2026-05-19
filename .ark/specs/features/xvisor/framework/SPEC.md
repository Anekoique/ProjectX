
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
