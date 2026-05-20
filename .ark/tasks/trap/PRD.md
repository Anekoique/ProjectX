# `trap` PRD

---

[**What**]

Install a real HS-mode trap entry: save the 36-word `TrapFrame` declared by
the framework SPEC, dispatch by `scause` into a Rust handler, return via
`sret`. Replaces the wfi parking-pad trampoline installed in P0 and lights up
the trap path every later phase reuses.

[**Why**]

Every downstream xvisor phase is trap dispatch with different `scause`
values — H-extension guest exits (P2/P3), SBI ecall handling (P4), Linux
boot exceptions (P5). The framework SPEC already committed the binding
TrapFrame layout (C-7) and the `sscratch` SP-swap reservation (C-8); P0
parked them behind a wfi loop. P1 is where they go live, in isolation,
before guest semantics pile on. The roadmap calls this out as
"P1 — Trap framework: take a deliberate trap, report scause/sepc, return
cleanly — done **before** any H-extension setup, because H-ext traps reuse
this same machinery."

[**Outcome**]

- `xvisor/src/hal/arch/riscv/trap.rs` ships a `trap_entry` naked function
  that saves all 32 GPRs + `sepc`/`scause`/`stval`/`sstatus` into a
  TrapFrame on the current kernel stack, calls a Rust dispatcher, restores,
  and `sret`s.
- `xvisor/src/hal/arch/riscv/boot.rs` writes `&trap_entry as usize` into
  `stvec` (Direct mode, lower 2 bits zero) instead of the wfi trampoline.
  The wfi trampoline function is removed.
- A Rust dispatcher classifies `scause` into interrupt-vs-exception by the
  top bit and switches on the exception code; `Breakpoint (cause=3)`
  advances `sepc` by 4 and returns; every other cause logs and calls
  `terminate(Failure)`.
- `xvisor/Cargo.toml` adds a `trap-canary` cargo feature (default off).
  When enabled, `rust_main` issues `ebreak` before the banner; the
  dispatcher prints `xvisor: trap cause=0x3 sepc=0x... stval=0x...`,
  advances sepc, and execution falls through to the banner — banner-after-
  trap is the round-trip proof.
- `make trap-test` builds the binary with `--features trap-canary`, runs
  it under QEMU, and asserts both the trap line and the post-trap banner
  appear on stdout.
- `make run` (without the feature) is unchanged from P0: boot → banner →
  terminate; no spurious trap.

[**Related Specs**]

- `specs/features/xvisor/framework/SPEC.md` — extends. Honours the
  declared TrapFrame field order (C-7) and the sscratch reservation (C-8).
  Supersedes C-10 (wfi trampoline): `stvec` now targets the real
  `trap_entry`. A `[**CHANGELOG**]` entry on the framework SPEC will note
  C-10's supersession.

[**SPEC Path**]

xvisor/trap
