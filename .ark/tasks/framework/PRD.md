# `framework` PRD

---

[**What**]

Stand up the `xvisor` Rust crate as a Type-1 RISC-V hypervisor skeleton that boots on `qemu-system-riscv64 -machine virt -cpu rv64,h=true` via OpenSBI fw_jump, runs in HS-mode on hart 0, prints a banner over the ns16550 UART, and halts cleanly. P0 of the `docs/XVISOR.md` roadmap.

[**Why**]

`docs/XVISOR.md` commits xvisor to a graduated guest-bringup path (P0 hello → P2 H-ext → P3 first guest → P5 Linux). Every later phase inherits the framework P0 establishes: module tree, per-hart slot, trap-frame layout, linker symbols, console driver, halt path. Locking those decisions in P0 — even when most subsystems are empty stubs — prevents the SPEC churn that comparable Rust hypervisors (hvisor, hypocaust-2) suffered when they retrofitted multi-hart and trap-entry conventions after the initial bring-up. The `xvisor/framework` feature SPEC promoted from this task becomes the vocabulary every later xvisor SPEC (`xvisor/trap`, `xvisor/g-stage`, `xvisor/sbi`, …) builds on top of.

[**Outcome**]

- `xvisor/` crate exists under repo root with `Cargo.toml`, `linker.ld`, `src/` tree, and a top-level Makefile target `make xvisor` that produces `xvisor/target/riscv64gc-unknown-none-elf/release/xvisor`.
- `make xvisor-run` boots the ELF under `qemu-system-riscv64 -machine virt -cpu rv64,h=true -smp 1 -m 256M -bios default -kernel <elf> -nographic`, prints a banner line on UART containing the literal string `xvisor: hello from HS-mode` followed by hartid and DTB pointer, and exits cleanly via SiFive-test finisher with exit code 0.
- Module tree is committed in full: `src/{main.rs, boot.s, arch/riscv/{mod.rs, csr.rs, trap.rs}, mm/mod.rs, vcpu/mod.rs, vm/mod.rs, sbi/mod.rs, device/{mod.rs, uart.rs}}` — empty `mod.rs` files carry a one-line doc comment naming the phase that fills them in. `arch/riscv/trap.rs` commits the `TrapFrame` struct and `trap_entry` extern declaration even though no `stvec` is wired.
- `_start` validates `misa.H = 1` before entering Rust; on `H = 0` it prints `"xvisor: H-extension required; pass -cpu rv64,h=true to QEMU"` and halts.
- Per-hart convention locked: `tp` holds `&PerCpu`, `sscratch` is documented as reserved for trap-entry swap, and `PerCpu` is a `#[repr(C)]` struct in `arch/riscv/mod.rs` (or equivalent) with `hartid`, `stack_top`, and reserved padding to a power-of-two size.
- DTB pointer (`a1`) is captured into a `static AtomicUsize DTB_ADDR` before any Rust code that could clobber `a1` runs.
- `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, and the existing `make fmt` / `make clippy` targets pass for the new crate.
- No heap, no `extern crate alloc` in P0. No `unsafe` outside `boot.s`, `arch/riscv/csr.rs`, and `device/uart.rs`.

[**Related Specs**]

- `.ark/specs/features/xemu/multi-hart/SPEC.md` — xvisor inherits the per-hart-state mental model (HartId, per-Core ownership) and ports it from M-mode (`mscratch` / `mhartid`) to HS-mode (`sscratch` / hartid-from-`a0`). P0 commits the analogous `PerCpu` struct; no edits to xemu's SPEC.
- `.ark/specs/features/xemu/csr/SPEC.md` — xvisor's `arch/riscv/csr.rs` mirrors the WARL/shadow CSR conventions established for xemu (named constants, typed wrappers) but for HS-mode + (future) H-ext CSRs. P0 commits S-mode CSR wrappers only; H-ext wrappers are P2. No edits to xemu's SPEC.
- `.ark/specs/features/xlib/SPEC.md` — `xlib` is the shared freestanding C library for xam-built guests; xvisor *itself* does not link xlib (it is `no_std` Rust top-to-bottom), but P3+ guests running under xvisor will reuse xlib unchanged. P0 does not touch xlib. No edits to xlib's SPEC.

Indirect — read for vocabulary precedent but not modified:

- `.ark/specs/features/xemu/devices/SPEC.md` — xvisor's `device/uart.rs` mirrors xam's ns16550 driver shape (LSR THRE poll, `_putch` primitive). P0 ports the driver; no edits to xemu's SPEC.

[**SPEC Path**]

`xvisor/framework`
