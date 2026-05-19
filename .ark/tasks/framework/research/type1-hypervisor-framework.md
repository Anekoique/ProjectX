# Type-1 Hypervisor Framework — research note for `framework`

- Query: P0 framework decisions and the long-term SPEC vocabulary for a Type-1
  RISC-V hypervisor (xvisor) booting on QEMU `-machine virt -cpu rv64,h=true`.
- Scope: mixed (internal — xam platform conventions; external — prior art and
  RISC-V Privileged H-extension spec).
- Date: 2026-05-18.

---

## Summary  (actionable headlines for the PRD author)

- **Boot mode for P0: `-bios default` (OpenSBI fw_jump payload at `0x80200000`,
  HS-mode entry, `a0 = hartid`, `a1 = fdt-ptr`).** Every credible
  modern Rust H-ext hypervisor — salus, hvisor, hypocaust-2, rvvisor, hikami —
  starts from this convention. Picking it makes P0 a 200-line job and *does
  not* foreclose `-bios none` later (P0's `_start` can simply branch on `a0`
  if we ever need a payload-side M-mode entry).
- **P0 must lock in the per-hart "scratch slot" convention now even with no
  traps.** `sscratch` (or `tp`) → per-hart `PerCpu` struct, with a static
  array sized at compile time. salus and hvisor both do this on hart 0 in P0
  equivalents; trying to retrofit it in P1 means rewriting `trap.S`.
- **Console: hypervisor-owned direct MMIO UART (ns16550 at `0x10000000`) for
  P0, eventually a service.** Matches xam's `platform/xemu/console.rs` so xam
  patterns transfer directly. Flagged as a tactical choice — Linux-in-VM (P5+)
  passes the UART through; xv6 / multi-tenant (P7+) will trap-and-emulate.
  P0 doesn't foreclose either path.
- **Halt semantics: `wfi`-loop, no SBI SRST.** xvisor is the SBI provider, not
  a client. Use SiFive-test (`0x100000`) magic for QEMU shutdown, mirroring
  xam's `platform/xemu/misc.rs:ebreak` shape. SBI SRST gets implemented in P4
  *for the guest*; the hypervisor itself owns the host shutdown path directly.
- **Module names that prior art converges on**: `arch/riscv/`, `mm/`, `vcpu/`,
  `vm/`, `sbi/`, `device/`, plus `boot.S` + linker — the `docs/XVISOR.md`
  layout already aligns. P0 only needs `main.rs` + `boot.s` + `arch/riscv/`
  skeletons + `console`. **Stub `mm/`, `vcpu/`, `vm/`, `sbi/`, `device/` with
  one-line `mod.rs` files so the public module tree is committed in P0**; P1-P3
  fill them in without renames.
- **H-extension is RV-Priv ratified (2021); 1.0 spec stable; QEMU H-ext is
  the canonical reference implementation.** No version-pin worry for P0.

---

## Type-1 vs Type-2  (1 paragraph)

A Type-1 ("bare-metal") hypervisor *is* the privileged code that owns the
machine — there is no host OS between it and the silicon. On RISC-V H-ext,
"Type-1" specifically means: the hypervisor runs in **HS-mode** (the
hypervisor-extended supervisor mode, see Priv ISA §5.1
[H-extension v1.0](https://docs.riscv.org/reference/isa/priv/hypervisor.html)),
**owns** the trap vector via `stvec`, **owns** the second-stage page-table
root via `hgatp`, **owns** the timer (the `time` CSR via Sstc and
`htimedelta`), **owns** the interrupt controller (PLIC / APLIC / IMSIC) by
either passing it through or trap-and-emulating it, and **owns** the SBI
console on behalf of all guests. M-mode firmware (OpenSBI) sits below it but
plays no role in per-VM-exit hot paths beyond IPI / SRST and what the
hypervisor itself escalates. Contrast Type-2 (KVM / Linux-on-RISC-V): the
host kernel owns those resources and the hypervisor is a kernel module
mediating per-VM context switches inside that kernel's address space.

---

## Module decomposition  (prior art comparative survey)

### Convergence table — top-level Rust source layout

| Project              | Lang   | LoC (src) | Top-level modules (just the dirs)                                                                                                                                                                                                                                                                  | P0-equivalent boot                                                                                                       |
| -------------------- | ------ | --------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------ |
| **salus** (Rivos)    | Rust   | ~15k      | `src/main.rs` + flat: `asm.rs`, `start.S`, `trap.S`, `trap.rs`, `vm.rs`, `vm_cpu.rs`, `vm_pages.rs`, `vm_interrupts.rs`, `vm_id.rs`, `vm_pmu.rs`, `smp.rs`, `host_vm.rs`, `hyp_layout.rs`, `hyp_map.rs`, `umode.rs`, `guest_tracking.rs`, `backtrace.rs`. Plus large external crates (`drivers/`, `riscv-page-tables/`, `riscv-regs/`, `riscv-pages/`). | `start.S:_start` runs in S-mode (post-OpenSBI), clears BSS, sets `sp = _stack_end`, calls `_primary_init` → `_primary_main`. |
| **hvisor** (syswonder) | Rust | ~8k       | `src/arch/riscv64/{consts,cpu,csr,entry,hypercall,ipi,mm,paging,s1pt,s2pt,sbi,time,trap.S,trap.rs,zone}.rs`; top-level `config.rs`, `consts.rs`, `cpu_data.rs`, `device/`, `hypercall/`, `memory/`, `pci/`, `platform/`, `zone.rs`.                                                              | `arch_entry()` `naked_asm` maps hartid → cpu_id via `BOARD_HARTID_MAP`, sets per-CPU stack, atomically elects master, clears BSS, calls `rust_main`. |
| **hypocaust-2** (KuangjuX) | Rust | ~3k     | `src/{boards,device_emu,drivers,guest,hyp_alloc,mm,page_table,sync}/`, plus `console.rs`, `constants.rs`, `detect.rs`, `error.rs`, `hypervisor.rs`, `main.rs`, `sbi.rs`, `lang_items.rs`, `linker-qemu.ld`.                                                                                       | `_start` (`naked`) sets per-hart stack from `a0`, jumps to `hentry`; binary base `0x80200000`; embeds guest kernel + DTB via `include_bytes!`. |
| **rvvisor** (lmt-swallow) | Rust | ~2k    | `hypervisor/src/{boot,debug,guest,hypervisor,main,memlayout,mkernel,paging,plic,riscv,uart,util,virtio}.rs` + `boot.S`, `hypervisor.S`, `mkernel.S`. Flat, no `arch/` indirection.                                                                                                                | `boot.S` enters in M-mode (`-bios none` style), sets `mepc = mkernel_entry`, `mret`s into M-mode kernel that prepares HS-mode then `mret`s into HS-mode hypervisor entry. |
| **miralis** (CharlyCst) | Rust | ~6k     | `src/{arch/{metal,pmp,registers,trap},device,driver,platform,policy,virt}.rs` + `host.rs`, `decoder.rs`, `modules.rs`, `main.rs`, `benchmark/`. *Not* HS-mode — Miralis is M-mode that *virtualises* HS-mode firmware. Useful for trap-frame and CSR-table patterns, not for layout.            | Linker script puts `_start_address` from config, sets stack, calls `main` with `a0=hartid`, `a1=dtb`.                    |
| **rustsbi-prototyper** (rustsbi/) | Rust | ~6k | `prototyper/src/{firmware,platform,riscv,sbi}/` + `cfg.rs`, `devicetree.rs`, `fail.rs`, `macros.rs`, `main.rs`. M-mode (provides SBI to its S-mode payload — opposite role from xvisor, but the closest *Rust* analog of OpenSBI's structure).                                                | `rust_main(hart_id, opaque, nonstandard_a2)` — boot hart vs others, hart-feature detection, then jumps into next-stage S-mode payload. |

Citations (commit/path):

- salus — `rivosinc/salus` @ master, `src/main.rs`, `src/start.S`, `src/smp.rs`.
- hvisor — `syswonder/hvisor` @ master, `src/arch/riscv64/entry.rs`,
  `src/arch/riscv64/cpu.rs`, `src/arch/riscv64/mod.rs`.
- hypocaust-2 — `KuangjuX/hypocaust-2` @ master, `src/main.rs`,
  `src/linker-qemu.ld`.
- rvvisor — `lmt-swallow/rvvisor` @ master, `hypervisor/src/main.rs`,
  `hypervisor/src/boot.S`.
- miralis — `CharlyCst/miralis` @ master, `src/main.rs`, `src/arch/mod.rs`,
  `misc/linker-script.x`.
- rustsbi-prototyper — `rustsbi/rustsbi` @ master, `prototyper/prototyper/src/main.rs`.

### Distilled minimal module taxonomy

Across the four most relevant comparables (salus, hvisor, hypocaust-2,
rvvisor), every project has, at minimum, these eight buckets — naming
varies, role doesn't:

| Bucket (canonical name in `docs/XVISOR.md`)     | What it owns                                                                              | Prior-art names                                                                              |
| ----------------------------------------------- | ----------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| `main.rs` + `boot.s` + linker                   | Crate entry, BSS zero, stack setup, `rust_main` call                                       | `_start` / `start.S` / `entry.rs` / `boot.S` (universal)                                     |
| `arch/riscv/csr.rs`                             | H-ext + S-mode CSR read/write wrappers, named constants for register addresses             | hvisor `arch/riscv64/csr.rs`; salus uses `riscv-regs` external crate; hypocaust uses `riscv-rs` |
| `arch/riscv/trap.{S,rs}`                        | Trap entry assembly, save/restore, `__trap_dispatch` Rust handler                          | All — universal pairing of `.S` + `.rs`                                                      |
| `mm/` (heap, page-table-builder, G-stage later) | Hyp allocator, eventual G-stage (Sv39x4) builder                                            | salus `hyp-alloc/` external + `vm_pages.rs`; hvisor `mm/` + `arch/riscv64/{paging,s1pt,s2pt}.rs`; hypocaust `mm/` + `page_table/` |
| `vcpu/` (or vm_cpu)                             | vCPU register file (GPRs + VS-CSRs), run-loop, `sret`-into-guest                            | salus `vm_cpu.rs`; hvisor `arch/riscv64/cpu.rs` (`ArchCpu`); hypocaust `guest/`               |
| `vm/` (or zone)                                 | Per-guest struct: memory, vCPUs, ID, state                                                  | salus `vm.rs`+`host_vm.rs`; hvisor `zone.rs`; hypocaust `guest/`                              |
| `sbi/` (dispatch on EID/FID)                    | Inbound `ecall` from VS → handler                                                           | salus `sbi-rs/` external crate; hvisor `arch/riscv64/sbi.rs`; hypocaust `sbi.rs`              |
| `device/` (UART, PLIC, virtio later)            | MMIO routing, emulation vs passthrough                                                      | salus `drivers/` external; hvisor `device/`; hypocaust `device_emu/`+`drivers/`               |
| `platform/` or `consts/` (memory map, board)    | Per-board MMIO addresses (CLINT, PLIC, UART), hart count, RAM range                         | hvisor `platform/`; hypocaust `boards/qemu.rs`; salus `hyp_layout.rs`                         |

**For P0 specifically**: only the first three buckets need actual code; the
others should land as `mod.rs` files with a one-line doc comment, so the
public module tree is committed in P0 and P1-P3 implementations slot in
without renames.

---

## Boot/handoff contract on QEMU virt

### Mode A — `-bios default` (OpenSBI fw_jump)

| Aspect              | Value                                                                                                                                                                                       |
| ------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Load address        | `0x80200000` (the default `FW_JUMP_ADDR` everyone bakes into linker scripts and Makefiles: see gem5/gem5-resources, Vyond/sbi, Penglai-Enclave, runninglinuxkernel/riscv_programming_practice for cross-confirmation). DRAM base is `0x80000000`; OpenSBI itself sits in `[0x80000000, 0x80200000)`. |
| Entry privilege     | **HS-mode** when the CPU has H-ext (`misa.H=1`) and OpenSBI is built for `generic` platform; OpenSBI's domain configuration sets "Next Mode : S-mode" but on H-ext-capable harts that is HS-mode by definition (HS-mode *is* S-mode plus the H bit). |
| `a0`                | hartid (the cold-boot hart; secondary harts spin in OpenSBI HSM until started via SBI HSM ext)                                                                                              |
| `a1`                | physical address of the flattened device tree (DTB)                                                                                                                                         |
| `sp`                | undefined — payload sets its own                                                                                                                                                            |
| `satp`              | `0` (bare paging; no S-stage translation active)                                                                                                                                            |
| `sstatus.SIE`       | `0` (interrupts disabled at S/HS)                                                                                                                                                          |
| `sie`               | `0`                                                                                                                                                                                         |
| `stvec`             | undefined (must be set before any trap can fire)                                                                                                                                            |
| H-ext CSRs          | undefined — OpenSBI doesn't touch them; first read returns reset value (mostly zero on QEMU)                                                                                                |
| PMP                 | OpenSBI configures PMP to give S-mode access to all RAM except its own image                                                                                                                |

This is the convention salus (`start.S`), hvisor (`arch_entry`), and
hypocaust-2 (`_start`) all assume. The xemu Makefile in
`/Users/anekoique/ProjectX/xam/xhal/src/platform/xemu/boot.rs:6-22` shows
the equivalent shape for the M-mode bare-metal case (different `mtvec`/`mret`,
same stack/BSS shape).

### Mode B — `-bios none` (direct payload)

| Aspect              | Value                                                                                          |
| ------------------- | ---------------------------------------------------------------------------------------------- |
| Load address        | `0x80000000` (DRAM base; payload loaded via `-kernel` or `-device loader,addr=0x80000000,file=…`) |
| Entry privilege     | **M-mode** — no firmware, the payload *is* the firmware                                        |
| `a0`                | hartid                                                                                         |
| `a1`                | DTB physical address (QEMU still provides one, written into the MROM and chained via the reset vector) |
| Everything else     | Reset state — payload must do its own M-mode setup (PMP, `mideleg`, `medeleg`, then `mret` into HS-mode) |

This is the rvvisor convention (`boot.S` → `mkernel.S` → HS-mode).
**Strictly more work for P0**: you have to write the M-mode → HS-mode
transition that OpenSBI would have done. Pays off only if we want to embed
our own M-mode SBI provider (e.g., rustsbi-style) — explicitly *not* the goal
of `docs/XVISOR.md` (xvisor is HS-mode and below it is OpenSBI).

### Mode C — Custom firmware (rustsbi, others)

rustsbi-prototyper expects `rust_main(hart_id, opaque, nonstandard_a2)` —
`opaque` is the DTB address (same as OpenSBI's `a1`), `nonstandard_a2` is a
RustSBI-specific boot-info pointer the payload may ignore. From the payload's
point of view, **the contract is identical to OpenSBI's `a0=hartid,
a1=dtb`** as long as you don't read `a2`. No payload-side changes needed.

### Recommendation for P0 — **Mode A (`-bios default`)**

Reasons:

1. **Smallest credible P0 codebase.** No M-mode setup, no PMP juggling — the
   hypervisor's first instruction runs in HS-mode and can immediately write
   the UART. Compare with rvvisor's `boot.S` + `mkernel.S` + `mkernel.rs` ≈ 200
   lines of M-mode prelude that xvisor would have to maintain forever.
2. **Aligns with `docs/XVISOR.md:121-138`** which already names OpenSBI as
   the M-mode firmware below xvisor. `-bios default` is the QEMU realisation.
3. **Locks in the `a0=hartid, a1=dtb` ABI** before P5's Linux load step,
   where xvisor passes the same convention *into the guest* via `sret`. Same
   contract, two levels — one mental model.
4. **Doesn't foreclose Mode B.** If P7+ ever wants xvisor to be its own
   M-mode firmware (unlikely, given xemu's role), P0's `_start` can branch
   on the privilege bit. No SPEC churn.
5. **OpenSBI already in `resource/opensbi/v1.3.1`** per `docs/XVISOR.md:514`.
   No new dependency.

For P0's QEMU invocation, the recommended command is approximately:

```
qemu-system-riscv64 -nographic -machine virt -cpu rv64,h=true -smp 1 \
    -m 256M -bios default -kernel xvisor.elf
```

(QEMU's `-kernel` flag triggers fw_jump dynamic into the ELF entry. The DTB
is auto-generated by QEMU and pointed to via `a1`.)

QEMU virt memory map (confirmed from `qemu/qemu` master @ `hw/riscv/virt.c`):

| Region              | Base         | Size                   |
| ------------------- | ------------ | ---------------------- |
| DEBUG               | `0x0`        | `0x100`                |
| MROM (reset vector) | `0x1000`     | `0xf000`               |
| SiFive-test (finisher) | `0x100000` | `0x1000`               |
| RTC                 | `0x101000`   | `0x1000`               |
| CLINT               | `0x2000000`  | `0x10000`              |
| ACLINT_SSWI         | `0x2F00000`  | `0x4000`               |
| PCIe PIO            | `0x3000000`  | `0x10000`              |
| PLIC                | `0xc000000`  | depends on hart count  |
| APLIC-M / APLIC-S   | `0xc000000` / `0xd000000` | hart-count dependent |
| VIRTIO              | `0x10001000` | `0x1000` × N           |
| **UART0 (ns16550)** | **`0x10000000`** | `0x100`            |
| FLASH               | `0x20000000` | `0x4000000`            |
| IMSIC-M / IMSIC-S   | `0x24000000` / `0x28000000` | hart-count dep |
| PCIe ECAM           | `0x30000000` | `0x10000000`           |
| PCIe MMIO           | `0x40000000` | `0x40000000`           |
| **DRAM**            | **`0x80000000`** | `-m` flag (typically 128 M to 8 G) |

UART0 confirmed at `0x10000000` matches `xam/xhal/src/platform/xemu/console.rs:1-2`
(`UART_THR: *mut u8 = 0x1000_0000`). Direct port.

---

## P0 minimum-viable framework  (what P0 MUST own so the SPEC doesn't churn)

This is the load-bearing section. Each row is the *forward-locked* contract
P0 must establish even though P0 itself doesn't exercise it.

| Decision                       | P0 value                                                                                                                                                                                                                                                                                                                  | Why "lock it now"                                                                                                                                                                                                                                                                                                                  |
| ------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Privilege at runtime**       | HS-mode after OpenSBI hands off. Never re-enter M-mode.                                                                                                                                                                                                                                                                   | Determines which CSR set the code uses (`stvec` not `mtvec`, `sret` not `mret`). If P0 accidentally runs in M-mode (e.g., via `-bios none`), every CSR symbol in P1+ has the wrong name and trap.S re-issues are guaranteed.                                                                                                                |
| **Per-hart state slot**        | `tp` register holds `&PerCpu` for the current hart. `sscratch` is reserved for trap-entry swap. Single static `[PerCpu; MAX_HARTS]` array, `MAX_HARTS = 1` for P0 but the const is named. `PerCpu` has at minimum: `hartid: usize`, `stack_top: *mut u8`, `_pad: [u8; …]` to round to a power-of-two for cheap indexing. | hvisor's `arch_entry` does this on day one (see `src/arch/riscv64/cpu.rs:ArchCpu` and `entry.rs:per_cpu_size`). Without it, P1 trap.S has nowhere to land `sscratch` and every later phase has to invent the convention twice. Mirrors xemu's `multi-hart` SPEC (`HartId(u32)` + `Vec<Core>`) — same mental model, different privilege. |
| **Linker script discipline**   | `BASE = 0x80200000`. Sections: `.text.boot` first (entry symbol), then `.text`, `.rodata`, `.data`, `.bss` (with `.bss.stack` distinct so we can size stack independently). Symbols: `_start`, `_stack_start`, `_stack_end`, `_bss_start`, `_bss_end`, `_hyp_end`. No guest regions yet — but reserve a comment block where they will go (e.g., `/* GPA region: starts at _hyp_end */`). | Every comparable does this exact layout (hypocaust `src/linker-qemu.ld:1-60`, salus `src/salus_lds.tmpl`). Choosing different symbol names later means rewriting every external reference in `boot.s` and the future `mm/` module.                                                                                                |
| **Stack size**                 | `STACK_SIZE_PER_HART = 64 KiB` (16 × 4 K pages, matches hvisor `PER_CPU_SIZE` and hypocaust's `BOOT_STACK_SIZE = 16 * PAGE_SIZE`).                                                                                                                                                                                          | 64 KiB is enough for trap reentry + Rust `println!` formatting + future `vm_pages` walk. Smaller (4 K) bites you in P5 when Linux's printk chains hit format depth. Bigger is wasted in `.bss` for a single-hart P0.                                                                                                              |
| **Console**                    | **Hypervisor-owned direct ns16550 MMIO** at `0x10000000`. A `console.rs` module with `_putch(b: u8)` and a writer that prints via spin-poll on LSR THRE bit. No interrupts.                                                                                                                                                | Mirrors `xam/xhal/src/platform/xemu/console.rs:1-12` line-for-line, swapping `_putch` from C ABI to Rust. Forecloses **nothing** important: in P5 Linux gets UART passthrough (Linux's driver hits the same MMIO), in P6/P7 multi-tenant will trap-and-emulate (hypervisor's `console.rs` becomes the emulation backend). Either path keeps the P0 module. |
| **Panic / halt**               | `wfi`-loop in a 1-instruction tight loop, *after* writing a panic line. Use SiFive-test finisher (`0x100000`) for clean QEMU shutdown on `terminate(code)` — exactly the shape of `xam/xhal/src/platform/xemu/misc.rs:1-13`. **No SBI SRST call from inside xvisor.** xvisor *provides* SRST to its guest in P4; it doesn't *consume* it. | Conceptually critical: a Type-1 doesn't ask OpenSBI to shut the machine down — it owns the machine. OpenSBI SRST goes through a domain mechanism that on QEMU virt also writes the SiFive-test finisher, so the observable effect is identical, but the code path keeps the layering honest.                                          |
| **Trap-entry SP/tp/sscratch**  | Even though P0 has no `stvec` set, **emit the trap-frame layout struct** (`#[repr(C)] struct TrapFrame { regs: [usize; 32], sepc, scause, ... }`) and the `sscratch ↔ sp` swap convention as a doc comment in P0. Concretely: P0 commits an empty `arch/riscv/trap.rs` containing the `TrapFrame` struct and a placeholder `trap_entry` declaration; the actual `trap.S` is P1's job. | This is the single biggest cause of SPEC churn in comparable projects (hvisor renamed `ArchCpu` once between v0.1 and v0.2; hypocaust-2 renamed `TrapContext` three times). Committing the struct field order in P0 — even with no code that reads it — is the cheapest possible insurance.    |
| **H-ext detection**            | P0 reads `misa` and panics if bit 7 (`H`) is zero, with the literal message `"H-extension required; pass -cpu rv64,h=true to QEMU"`. Even though P0 doesn't *use* H-ext yet, this catches the most common operator mistake (`docs/XVISOR.md:170-174` already calls this out).                                              | Free insurance, three lines of code. Without it, P2's H-ext-enabled boot fails with an illegal-instruction trap on the first `csrr hgatp` — and a developer with no trap framework yet (P1 is not done) has nothing to debug with.                                                                                                  |
| **DTB pointer storage**        | P0 stashes `a1` into a `static AtomicUsize DTB_ADDR` before any allocation, even though P0 doesn't parse the DTB. Stored as physical address.                                                                                                                                                                              | P2+ DTB parsing needs this address; if it's not captured in `_start`, a0/a1 have been clobbered by the time anything Rust runs. salus does the equivalent via `_primary_init(a0, a1)` taking both as args.                                                                                                                       |
| **No allocator yet**           | P0 uses zero heap. Stack-only printing. Static arrays for any structure.                                                                                                                                                                                                                                                  | Letting a bump allocator land in P0 means deciding *now* whether it's `linked_list_allocator`, `buddy_system_allocator`, salus-style `HypAlloc`, or a hand-roll. That's a P1-grade decision; force it to P1. (hypocaust uses `buddy_system_allocator`; salus rolls its own page-tracker; hvisor uses `buddy_system_allocator`. No convergence.) |
| **Module skeleton**            | `main.rs`, `boot.s`, `linker.ld`, `arch/riscv/{mod,csr,trap}.rs`, `mm/mod.rs` (empty), `vcpu/mod.rs` (empty), `vm/mod.rs` (empty), `sbi/mod.rs` (empty), `device/{mod,uart}.rs`. Every `mod.rs` has a `//!` doc comment naming the phase that fills it in.                                                                  | Commits the public-module taxonomy in P0 (the SPEC's tree of names) without forcing implementation. P1-P6 add files inside these dirs; nothing moves.                                                                                                                                                                              |

### What P0 explicitly *does not* own

These are deferred and the SPEC must NOT promise them:

- Trap entry / dispatch (P1)
- H-ext CSR writes (P2)
- G-stage page table (P3)
- vCPU register file struct definition — only the *trap frame* (P3)
- SBI inbound dispatch (P3.5 / P4)
- Multi-hart anything (P6+ per `docs/XVISOR.md:550`)

P0's SPEC contract therefore mentions all eleven rows above as "Constraints"
in a single feature SPEC under `.ark/specs/features/xvisor/framework/SPEC.md`
(speculation: that's the likely path given `docs/XVISOR.md:107-113` already
puts `xvisor/` as a sibling top-level dir).

---

## Risks  (Rust Type-1-specific, ≤5)

| Risk                                                                                                          | Likelihood | Mitigation                                                                                                                                                                                                                                                                          |
| ------------------------------------------------------------------------------------------------------------- | ---------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **`&'static mut` aliasing during boot.** Rust's borrow checker hates the `_start` → BSS-zero → mutable-global pattern; comparable projects all use `naked` asm + careful single-writer comments. | High       | Use `#[unsafe(naked)] _start` (like xemu `boot.rs:6-22`), and access `PerCpu` arrays through raw pointers in `unsafe { … }` blocks with one-line SAFETY comments per access — like hvisor `arch_entry`. No `&mut` until after BSS zero **on the boot hart only**.                                       |
| **Interior mutability + future multi-hart races.** P0 is single-hart and tempting to splatter `static mut` everywhere; P6 multi-hart then needs `Once`, `Mutex`, or per-CPU patterns retrofitted. | Medium     | Even in P0, use `Once<T>` (from `spin` or `sync` crate) for any global initialised once. Per-hart state always goes through the `tp` slot, never a global with a hartid lookup — that decision propagates from xemu's `multi-hart` SPEC (`per-hart state owned by Core`). |
| **Allocator-before-MM ordering bug.** Easy to introduce a `Box::new` (or `format!`) before the page allocator is initialised, in early-boot code that runs *before* `mm::init`. Subtle late-binding panic.    | Medium     | P0 forbids the heap entirely — no `extern crate alloc`. P1 may add it but only behind a `#[global_allocator]` that asserts `mm::init_done` at construction. salus's pattern: `HypAlloc::init` is a precondition checked in `_primary_main` before any `alloc::` is reachable.                              |
| **H-ext detection failing silently.** If `misa.H` is zero (operator forgot `h=true` on QEMU, or running on a non-H silicon), the first `csrr hgatp` in P2 traps as illegal instruction *into a trap vector that doesn't exist yet*, producing a double-fault triple-bounce that QEMU presents as a silent hang. | Medium     | P0 must include the `misa.H` check from row 8 of the P0 table above. Three lines of code, eliminates a whole class of confusion.                                                                                                                                                |
| **Trap-stack reentrancy in future P1.** RISC-V doesn't auto-switch SP on trap entry — if the trap fires while already on the hyp stack and saves the frame on the same stack, recursion is unbounded. Especially nasty during a panic-from-trap path. | Low (P0)   | The TrapFrame struct committed in P0 (row 7) reserves a slot for "saved SP" and the doc comment specifies `sscratch ↔ sp` swap on entry. P1 trap.S can then add a guard band check (`bgeu sp, _stack_bot, ok; j _double_fault`) without struct churn.                              |

---

## Recommendations to PRD/PLAN

1. **Bind P0's load address to `0x80200000` and entry mode to HS-mode (post-OpenSBI).** Document the QEMU invocation in PRD; lift it into Makefile in PLAN.
2. **Commit the full module tree in P0** (`arch/riscv/`, `mm/`, `vcpu/`, `vm/`, `sbi/`, `device/`), even if every `mod.rs` is empty. Forecloses naming churn in P1-P6.
3. **Lock the per-hart convention now: `tp = &PerCpu`, `sscratch` reserved for trap swap.** Even though P0 has neither traps nor multi-hart, declaring it in P0's SPEC saves a refactor in P1.
4. **Commit the `TrapFrame` struct in P0's `arch/riscv/trap.rs`**, with `trap_entry` as a placeholder (`extern "C" fn`) the P1 task will define. Lock field order against future code-review churn.
5. **Use `xam/xhal/src/platform/xemu/console.rs` as the literal template** for xvisor's `device/uart.rs`. Same UART, same MMIO layout, same THRE-poll. Re-implementing it from scratch is rework.
6. **Mirror `xam/xhal/src/platform/xemu/misc.rs::terminate` for halt semantics**, calling the SiFive-test finisher at `0x100000` with the appropriate magic word (`0x5555` for shutdown success, `0x3333 | (code << 16)` for fail). Avoid SBI SRST for the host shutdown path — xvisor owns the machine, not OpenSBI.
7. **Add a one-line `misa.H` check in `_start` before calling `rust_main`**. Three lines of asm, panic with a literal string. Catches the most common operator mistake.
8. **No heap, no `extern crate alloc` in P0.** Defer to P1's bump-allocator decision. Static arrays only.
9. **Name P0's feature SPEC `xvisor/framework`** per `docs/XVISOR.md:107` repo layout — promote in this deep-tier task.
10. **For the SPEC `[**Constraints**]` block**, mirror xemu's style (numbered `C-N` with file path/line — see `.ark/specs/features/xemu/multi-hart/SPEC.md:82-89`). Tie each constraint to a file in `xvisor/src/`.

---

## References

### Primary sources (specs / docs)

- RISC-V Privileged ISA, H-extension v1.0 (ratified 2021, part of Priv 1.12):
  [riscv-priv H chapter (five-embeddev mirror)](https://five-embeddev.com/riscv-priv-isa-manual/Priv-v1.12/hypervisor.html)
- RISC-V ratified specifications library — H-extension v1.0:
  [docs.riscv.org/reference/isa/priv/hypervisor.html](https://docs.riscv.org/reference/isa/priv/hypervisor.html)
- RISC-V unified DB H-extension (latest draft tracking):
  [riscv-unified-db H](https://riscv-software-src.github.io/riscv-unified-db/manual/html/isa/isa_20240411/exts/H.html)
- OpenSBI `fw_jump.md`:
  [riscv-software-src/opensbi docs/firmware/fw_jump.md](https://github.com/riscv-software-src/opensbi/blob/master/docs/firmware/fw_jump.md)
- OpenSBI `qemu_virt.md`:
  [riscv-software-src/opensbi docs/platform/qemu_virt.md](https://github.com/riscv-software-src/opensbi/blob/master/docs/platform/qemu_virt.md)
- QEMU virt machine docs:
  [qemu.org docs/master/system/riscv/virt.html](https://www.qemu.org/docs/master/system/riscv/virt.html)
- QEMU virt source (memory map authority):
  [qemu/qemu hw/riscv/virt.c master](https://github.com/qemu/qemu/blob/master/hw/riscv/virt.c)
- RISC-V SBI v2.0 spec (for P4 horizon):
  [riscv-non-isa/riscv-sbi-doc releases](https://github.com/riscv-non-isa/riscv-sbi-doc/releases)

### Prior-art Rust Type-1 RISC-V hypervisors

- **salus** (Rivos Inc., production-grade, TEE-focused):
  [rivosinc/salus master](https://github.com/rivosinc/salus) — `src/main.rs`,
  `src/start.S`, `src/smp.rs`, `MEMORY.md`. ~15 k LoC, S-mode boot, post-OpenSBI,
  per-CPU at the top of each secondary stack via `tp`. **Best reference for
  long-term Rust framework patterns.**
- **hvisor** (syswonder, edge-device, multi-arch):
  [syswonder/hvisor master](https://github.com/syswonder/hvisor) —
  `src/arch/riscv64/{entry,cpu,csr,mod}.rs`. ~8 k LoC, S/HS-mode boot,
  hartid → cpu_id mapping for non-contiguous board hartids, atomic master-CPU
  election, BSS clear by master only. **Best reference for the boot dance
  with `naked_asm`.**
- **hypocaust-2** (KuangjuX, learning-grade, H-ext on QEMU virt):
  [KuangjuX/hypocaust-2 master](https://github.com/KuangjuX/hypocaust-2) —
  `src/main.rs`, `src/linker-qemu.ld`. ~3 k LoC, embeds guest kernel via
  `include_bytes!`. **Closest match for xvisor's near-term scope; the
  linker script is essentially what xvisor's P0 should produce.**
- **rvvisor** (lmt-swallow, pedagogical, `-bios none`):
  [lmt-swallow/rvvisor master](https://github.com/lmt-swallow/rvvisor) —
  `hypervisor/src/{main,boot,hypervisor}.rs`. ~2 k LoC, M-mode entry. **Useful
  as the negative example for Mode B above.**
- **miralis** (CharlyCst, M-mode firmware that virtualises HS firmware — not
  a Type-1 in the same sense):
  [CharlyCst/miralis master](https://github.com/CharlyCst/miralis). ~6 k LoC.
  **Useful for trap-frame/CSR-table patterns; not a layout reference.**

### Secondary references

- rustsbi (M-mode SBI implementation in Rust):
  [rustsbi/rustsbi master](https://github.com/rustsbi/rustsbi) —
  `prototyper/prototyper/src/main.rs`. **The Rust counterpart of OpenSBI;
  useful for understanding what's *below* xvisor.**
- Hikami (educational RISC-V hypervisor):
  [Alignof/hikami master](https://github.com/Alignof/hikami) —
  `hikami_core/src/h_extension/csrs.rs`, `src/hypervisor_init.rs`.
- ACE-RISCV (IBM, security-monitor in Rust, references all H-ext CSRs):
  [IBM/ACE-RISCV master](https://github.com/IBM/ACE-RISCV) —
  `security-monitor/src/core/architecture/riscv/control_status_registers.rs`.
- DuVisor (IPADS, paper-grade RISC-V hypervisor):
  [IPADS-DuVisor/DuVisor master](https://github.com/IPADS-DuVisor/DuVisor).

### Internal references (this codebase)

- `/Users/anekoique/ProjectX/docs/XVISOR.md` — the project roadmap (P0-P9).
- `/Users/anekoique/ProjectX/xam/xhal/src/platform/xemu/boot.rs:1-22` — the
  M-mode-equivalent boot shape xvisor's `boot.s` will mirror in HS-mode.
- `/Users/anekoique/ProjectX/xam/xhal/src/platform/xemu/console.rs:1-12` —
  the literal ns16550 driver to port into `xvisor/src/device/uart.rs`.
- `/Users/anekoique/ProjectX/xam/xhal/src/platform/xemu/misc.rs:1-18` — the
  halt / terminate pattern (`ebreak`-then-`wfi`) to mirror.
- `/Users/anekoique/ProjectX/xam/xhal/src/platform/xemu/trap.S` and `trap.rs` —
  the M-mode trap frame layout to translate to HS-mode (replace `m*` CSRs
  with `s*`; structure identical).
- `/Users/anekoique/ProjectX/.ark/specs/features/xemu/multi-hart/SPEC.md` —
  the per-hart state model to inherit (HartId, per-Core ownership).
- `/Users/anekoique/ProjectX/.ark/specs/features/xemu/csr/SPEC.md`,
  `inst/SPEC.md`, `mm/SPEC.md` — vocabulary precedents for the future xvisor
  SPECs (in P2, P3, P3+).
- `/Users/anekoique/ProjectX/.ark/specs/features/xlib/SPEC.md` — example of a
  promoted feature SPEC's `[**Goals**]` / `[**Non-goals**]` /
  `[**Constraints**]` block structure for the new `xvisor/framework/SPEC.md`.

## Caveats / Not found

- **The exact OpenSBI handoff register state for the `time` CSR delegation
  bit (`menvcfg.STCE`) on QEMU virt** — not verified in this pass. P2 PRD
  should re-check; the comment in hvisor `arch/riscv64/cpu.rs:CSR_HENVCFG`
  suggests OpenSBI leaves it disabled and the hypervisor must set
  `henvcfg.STCE = 1` to enable Sstc-based VS-mode timers. Speculation: this
  is moot for P0 because P0 doesn't touch timers.
- **Whether QEMU virt's PLIC-vs-APLIC choice affects P0** — confirmed: not
  in P0 scope; PLIC interrupts are masked until P1's `stvec` is wired and
  `sie` gates them anyway. The choice (`-cpu rv64,h=true,Smaia=on` for
  APLIC/IMSIC vs the default PLIC) is a P6 question.
- **rustc backend support for H-ext inline asm constraints** — to confirm in
  P0: the H-ext CSRs (`hgatp = 0x680`, etc.) are not named by `csrrw` mnemonic
  in older binutils; xvisor will likely write them by number. Cross-check
  against `nightly-2026-03-15` once `arch/riscv/csr.rs` exists. (Mitigated:
  `docs/XVISOR.md:575` already flags this as Low likelihood.)
- **`-bios default` vs explicit `-bios resource/opensbi/v1.3.1/...`** — the
  default ships with QEMU's bundled OpenSBI (typically older). Recommendation:
  start with `-bios default` for P0, switch to the explicit pinned OpenSBI
  binary in P2 once we depend on specific H-ext-related behaviour. Decision
  is reversible; flagged here so P0 PRD doesn't lock to the wrong copy.
