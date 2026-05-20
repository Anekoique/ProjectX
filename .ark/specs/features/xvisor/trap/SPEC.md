
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
