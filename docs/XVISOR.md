# xvisor Roadmap

A Type-1 RISC-V hypervisor at `ProjectX/xvisor/`, built incrementally to boot
an unmodified Linux kernel as a guest VM. Development target is QEMU
(`-machine virt -cpu rv64,h=true`); once stable, `xemu` gains H-extension
support and xvisor self-hosts on it.

## Goal

End state, in one sentence: `make linux` under `xvisor` brings an unmodified
Linux 6.1+ to an interactive shell over the host UART, using only the
RISC-V H-extension for isolation — no Linux modifications, no
firmware tricks.

Linux is the *final* guest, not the first. The roadmap reaches it through a
graduated series of guests built on the existing xam / xlib / xkernels
infrastructure — each one a guest whose correct behaviour we already know,
so any divergence is unambiguously xvisor's fault.

## Non-Goals (for now)

- Multi-tenant scheduling. Single guest, dedicated host CPUs, no scheduler.
- Live migration, snapshotting, checkpoint/restore.
- IOMMU. DMA confinement is deferred until we pass through a real PCIe device.
- Real hardware bring-up (SiFive U74, VisionFive 2). QEMU + xemu cover the
  learning goals; hardware is a follow-up.
- Performance tuning. Correctness first; xemu's Phase 9 perf work is the
  template for when we get there.

## Host Platforms

| Platform                        | Status             | Notes                                                      |
| ------------------------------- | ------------------ | ---------------------------------------------------------- |
| QEMU `virt`, `-cpu rv64,h=true` | Primary dev target | Reference H-ext implementation; well-tested.               |
| `xemu` (post H-ext extension)   | Phase P9 target    | Self-hosting closes the loop with the rest of the project. |
| Real hardware                   | Out of scope       | Possible after P9. Requires HS-mode-capable silicon.       |

## Infrastructure Reuse Strategy

xemu's bare-metal stack already validates RISC-V system code at every
privilege level the project has shipped so far. Rather than write a
throwaway hand-rolled guest for P3 and discover bugs only when Linux
panics in P7, xvisor reuses the existing infrastructure as guests:

| Component                     | Reuse                                                                       | Changes needed                                                                                                                                      |
| ----------------------------- | --------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------- |
| **xam** (HAL)                 | New `xvisor-guest` platform target alongside `xemu` and `riscv64-qemu-virt` | Trap init writes `stvec` (not `mtvec`); `_putch` / `set_mtimecmp` route through SBI; timer uses Sstc `stimecmp`. No refactor of existing platforms. |
| **xlib** (C library)          | As-is                                                                       | None — privilege-agnostic.                                                                                                                          |
| **xkernels** (test workloads) | am-tests, cpu-tests, alu-tests, benchmarks all become guest workloads       | Build matrix gains `PLATFORM=xvisor-guest`; test bodies unchanged.                                                                                  |
| **resource/**                 | OpenSBI v1.3.1, Linux 6.1.44 Image, initramfs, DTBs                         | None — reused verbatim from xemu.                                                                                                                   |
| **xemu** itself               | Reference oracle for guest correctness during P3.5 / P4.5                   | Difftest: xemu running an am-test natively vs xvisor running the same am-test as a VS-mode guest must produce identical visible output.             |

**The key insight.** xemu is the only RISC-V system this project trusts
end-to-end. Every guest from P3 through P6 is something xemu already runs
correctly. When xvisor's behaviour diverges, the bug is in xvisor — not in
the guest, not in the toolchain, not in the SBI shim. This collapses the
debugging search space dramatically compared to "load Linux and see what
breaks."

**xvisor itself is *not* an xam guest.** xvisor is bare-metal HS-mode,
owns its own trap vector, runs its own bump allocator, and accesses H-ext
CSRs directly. xam's M-mode HAL doesn't fit. xvisor borrows xam's
*patterns* — linker script layout, platform descriptor model, trap frame
layout — but its scaffolding is its own.

## Repo Layout (new)

```
ProjectX/
├── xvisor/                        ← new top-level sibling
│   ├── Cargo.toml                 # bare-metal RV64 no_std binary
│   ├── Makefile                   # qemu/run/test/difftest targets
│   ├── rust-toolchain.toml        # inherits ProjectX pin unless P0 forces newer
│   ├── linker.ld                  # HS-mode entry + section layout
│   ├── src/
│   │   ├── main.rs                # crate root, bootstrap
│   │   ├── boot.s                 # _start, the only handwritten assembly
│   │   ├── arch/riscv/
│   │   │   ├── mod.rs
│   │   │   ├── csr.rs             # hgatp, hstatus, vsatp, hedeleg, hideleg,
│   │   │   │                       # htimedelta, hie, hip, ...
│   │   │   ├── trap.S             # VM-exit save/restore, dispatch into Rust
│   │   │   └── trap.rs            # exit reason classification + handlers
│   │   ├── mm/
│   │   │   ├── mod.rs
│   │   │   ├── g_stage.rs         # G-stage (Sv39x4) page-table builder
│   │   │   └── allocator.rs       # bump allocator for the hypervisor heap
│   │   ├── vcpu/
│   │   │   ├── mod.rs
│   │   │   ├── context.rs         # vCPU register file (GPRs + VS-CSRs)
│   │   │   └── run.rs             # sret-into-guest run loop
│   │   ├── vm/
│   │   │   └── mod.rs             # VM struct: one guest, its memory, its vCPUs
│   │   ├── sbi/
│   │   │   ├── mod.rs             # dispatch on (EID, FID)
│   │   │   ├── base.rs            # probe / get_spec_version / get_impl_id
│   │   │   ├── timer.rs           # set_timer via htimedelta + vstimecmp (Sstc)
│   │   │   ├── ipi.rs             # send_ipi (single-hart no-op in P5)
│   │   │   └── srst.rs            # system_reset → host shutdown
│   │   ├── device/
│   │   │   ├── mod.rs
│   │   │   ├── uart.rs            # ns16550 passthrough → emulation later
│   │   │   ├── plic.rs            # PLIC virtualization (P6)
│   │   │   └── virtio_blk.rs      # post-P6 (optional)
│   │   ├── guest_loader.rs        # parse Image / DTB / initramfs, place in GPA
│   │   └── lib.rs
│   └── docs/
│       └── DESIGN.md              # written incrementally, one section per phase
├── xam/
│   └── xhal/src/platform/
│       └── xvisor_guest/          ← new platform target (P2.5)
└── resource/                      # reused from xemu: opensbi/, linux/, dtb sources
```

## Phases

Each phase is a self-contained, commit-sized deliverable with a runnable
QEMU command and a clear pass/fail. **Do not start phase N+1 until N's demo
runs reliably.**

### P0 — Bare-metal "hello"

**Goal.** xvisor binary boots into HS-mode under QEMU and prints to UART.
No guest, no H-ext setup beyond what OpenSBI gives us for free.

**Mechanism.** QEMU `-bios default` boots OpenSBI in M-mode, which then
jumps to our `_start` in HS-mode at `0x80200000`. Our boot sequence zeros
BSS, sets up a stack, and calls `main` in Rust. `main` writes bytes to the
ns16550 UART at `0x10000000`.

**Demo.** `make hello` prints `xvisor: hello, HS-mode\n` and halts in a `wfi`
loop.

**What you'll learn.** RISC-V boot protocol (`a0 = hartid`, `a1 = DTB ptr`),
linker scripts for bare-metal Rust, OpenSBI's role as M-mode firmware.

**Estimated effort.** Small — half a day.

---

### P1 — Trap framework

**Goal.** Take a deliberate trap (`ebreak`), report `scause`/`sepc`, and
return cleanly. Done **before** any H-extension setup, because H-ext
traps reuse this same machinery.

**Mechanism.** Write `stvec`. Trap entry in `trap.S` saves the full GPR set
into a `TrapFrame`, calls a Rust handler with `&mut TrapFrame`, restores,
and `sret`s. Test by hitting `ebreak` from HS-mode and printing the cause.

**Demo.** `make trap-test` deliberately faults, prints
`trap: cause=0x3 sepc=0x8020XXXX`, advances `sepc` past the `ebreak`, and
continues.

**What you'll learn.** Trap entry/exit pairing, why context save is the
hardest part to get right, the difference between exceptions and interrupts
in `scause`.

**Estimated effort.** Small — one day.

---

### P2 — H-extension enablement

**Goal.** Verify the CPU advertises H-ext (`misa.H == 1`), program the H-ext
CSRs we need for guest entry, and set up trap delegation so VS-mode events
land in HS-mode.

**Mechanism.**

- Check `misa[7] == 1`. Panic if zero (QEMU silently boots without H-ext if
  you forget `-cpu rv64,h=true` — a real and costly mistake to spot late).
- Configure `hedeleg` and `hideleg` so guest-page-faults, guest-ecalls,
  guest-illegal-insts, and VS-mode interrupts are taken in HS-mode.
- Zero `hgatp` (no G-stage translation yet — guest will see bare physical
  memory in P3 until G-stage is wired up).
- Configure `hstatus` so a future `sret` lands in VS-mode rather than
  HS-mode.

**Demo.** `make hext-check` dumps `misa`, `hstatus`, `hedeleg`, `hideleg`,
and asserts each field is what we expect. No guest entered yet.

**What you'll learn.** The H-extension privilege model (HS = "S-mode of the
host", VS = "S-mode of the guest"), why H-ext is "add a virtual S-mode
beneath your existing S-mode," CSR semantics for the delegation registers.

**Estimated effort.** Small-to-medium — one to two days, most of it reading
the RISC-V Privileged Spec H-ext chapter.

---

### P2.5 — xam VS-mode platform (`xvisor-guest`)

**Goal.** Teach xam to produce binaries that run as a VS-mode guest. No
xvisor involvement yet — the binary just has to *build* and run correctly
under xemu booted directly into S-mode (xemu doesn't have H-ext yet, so
"VS-mode" really means "S-mode" here; the distinction matters only once
xvisor wraps it).

**Mechanism.**

- New platform target: `xam/xhal/src/platform/xvisor_guest/`. Mirrors the
  existing `xemu/` platform structure (`boot.rs`, `console.rs`, `misc.rs`,
  `timer.rs`, `trap.rs`, `trap.S`, `mod.rs`).
- New config: `xam/xconfig/configs/platforms/xvisor-guest.toml`.
- New build script: `xam/scripts/platforms/xvisor-guest.mk` (linker script
  places the guest at the GPA xvisor will load it to).
- **Trap init** writes `stvec` (not `mtvec`); trap frame keeps the same
  layout but renames `mepc`/`mcause` to `sepc`/`scause` (or adds aliases).
- **`_putch`** issues SBI `console_putchar` (EID `0x01`, legacy) for now.
  Later phases may switch to direct UART MMIO once xvisor exposes
  passthrough.
- **`set_mtimecmp`** issues SBI `set_timer` (EID `0x54494D45`).
  `mtime`/`uptime` read via the `time` CSR (CSR `0xc01`), which xvisor
  exposes via `htimedelta`.

**Smoke test (under xemu, S-mode boot).** Build the simplest am-test
(`hello`) for `PLATFORM=xvisor-guest` and boot it under xemu directly in
S-mode (xemu's existing S-mode boot path or via a thin M-mode shim).
Confirms the new platform compiles and runs end-to-end before xvisor is
involved.

**Demo.** `make -C xam K=xkernels/tests/am-tests PLATFORM=xvisor-guest TEST=hello`
produces an ELF that prints via SBI `console_putchar`. Boot it under xemu
and see the output.

**What you'll learn.** What changes between M-mode and S-mode bare-metal
code (it's less than you'd think — mostly trap vector and CSR names).
The legacy SBI putchar / timer ABI, which is what most existing bare-metal
RISC-V code already targets.

**Estimated effort.** Small-to-medium — two to three days. Most of the
work is the linker script and the platform descriptor; trap/timer code
is mostly copy-modify from the existing `xemu/` platform.

---

### P3 — Hello guest

**Goal.** Load the simplest xam guest (the `xvisor-guest` `hello` am-test
from P2.5) into G-stage-mapped guest physical memory, enter it via `sret`,
handle its SBI `console_putchar` exit, print the byte on its behalf, return
to the guest, watch it halt.

**Mechanism.**

- Build a G-stage page table (Sv39x4 — the H-ext's modified Sv39 with
  4-page root) mapping GPA `0x80200000..` → HPA where we copied the guest.
- Load the P2.5 hello am-test ELF as the guest. **No new throwaway guest
  code is written for this phase.**
- Set `vsatp = 0` (guest runs bare-physical in VS-mode for now), set
  `sepc` to the guest's entry point, `sret`.
- Handler in `trap.rs` recognizes `scause = 10` (VS-mode environment call),
  decodes the SBI call as `console_putchar`, prints the byte, advances
  `vsepc`, returns. Other SBI calls panic with a clear "unhandled (EID,
  FID)" message — fed forward to P4.

**Demo.** `make hello-guest` prints `xvisor: hello from guest` followed by
the am-test's expected output (`Hello, World!` or equivalent).

**What you'll learn.** G-stage page-table format (subtly different from
Sv39 — the root has 4 pages, not 1), VM-entry via `sret` with `hstatus.SPV
= 1`, what a VM-exit actually *is* (it's just a trap; nothing more
magical than P1's `ebreak`).

**Estimated effort.** Medium — three to five days.

---

### P3.5 — am-tests as guests (graduated correctness)

**Goal.** Run the full am-test suite as VS-mode guests under xvisor. Each
test stresses a different facet of xvisor's exit handling against a guest
whose correct behaviour is already known.

**Mechanism.**

- For each am-test, build the `xvisor-guest` flavour and load it under
  xvisor exactly as in P3.
- Each test surfaces new SBI calls or trap categories that xvisor must
  handle:
  - `csr-test` — exercises VS-mode CSR access. Forces xvisor to handle (or
    delegate) S-mode CSR reads/writes that don't have direct VS aliases.
  - `trap-test` — guest takes its own internal traps. Confirms `hedeleg`
    is set such that delegated exceptions stay in VS-mode without
    bouncing to HS-mode.
  - `intr-test` — guest waits on timer interrupt. Forces SBI `set_timer`
    handling (Sstc-based, see P4) ahead of schedule, in a controlled
    setting where the guest's expected behaviour is "print N then halt".
  - `float-test` — guest uses F/D. Confirms `vsstatus.FS` virtualization
    works and FP state is preserved across VM exits.
- **Correctness oracle.** Run each am-test natively under xemu (where it
  already passes) and capture stdout. Run the same am-test as a guest
  under xvisor on QEMU. Stdout must match byte-for-byte. This is the
  poor-man's difftest until P9 enables real per-instruction difftest.

**Demo.** `make guest-tests` runs every `xvisor-guest`-flavoured am-test
and reports pass/fail by stdout comparison.

**What you'll learn.** Which SBI calls real guests actually issue (mostly
console + timer). The asymmetry between exceptions xvisor wants to handle
(SBI ecalls) and ones it wants to delegate (page faults the guest itself
caused). Where `vsstatus` shadow-register semantics start to bite.

**Estimated effort.** Medium — three to five days, plus iteration as each
test surfaces a new exit reason.

---

### P4 — SBI shim

**Goal.** Implement enough SBI v2.0 for Linux's early boot. Most of the
groundwork is already in P3.5 — this phase formalises the dispatch table
and adds the calls am-tests don't exercise.

**Mechanism.** Linux issues SBI calls (`ecall` from VS-mode) for timer,
IPI, console, and reset. We need:

- **Base extension** (EID `0x10`): `get_spec_version`, `get_impl_id`,
  `get_impl_version`, `probe_extension`. Determines what Linux thinks we
  support.
- **Timer extension** (EID `0x54494D45`): `set_timer`. Implement using
  `htimedelta` (offset between guest time and host time) and `vstimecmp`
  (Sstc — supervisor timer compare for VS-mode). Avoid emulated mtime —
  Sstc gives the guest a direct hardware timer. (Already partly done in
  P3.5 via the legacy timer ABI; this phase moves to Sstc and adds the
  v2.0 calling convention.)
- **IPI extension** (EID `0x735049`): `send_ipi`. Single-hart, no-op
  for now.
- **SRST extension** (EID `0x53525354`): `system_reset`. Bridge to host
  shutdown via the SiFive test finisher (`xemu` already supports this).
- **Console extension** (EID `0x4442434E` — DBCN): `console_write_byte` and
  friends. The modern v2.0 console ABI Linux uses; the legacy
  `console_putchar` from P3 stays as a fallback.

**Demo.** `make sbi-test` runs a bespoke guest (or a benchmark from
xkernels) that exercises each SBI call and verifies behaviour. Add a
printer in the SBI dispatch path for every unhandled `(EID, FID)` — the
unknowns are exactly what P5 needs to handle before Linux gets further.

**What you'll learn.** SBI as the guest-VMM ABI, why Sstc matters (timer
without trap-and-emulate), the spec/impl/version negotiation Linux does
on boot.

**Estimated effort.** Medium — three to five days.

---

### P4.5 — xkernels benchmarks as guests

**Goal.** Run `coremark`, `dhrystone`, and `microbench` from `xkernels/`
as VS-mode guests. Forces xvisor through realistic instruction mixes
including F/D, atomics, and longer execution windows than the am-tests.

**Mechanism.** Same loader as P3, same `xvisor-guest` xam platform. The
benchmarks already build for `PLATFORM=xemu` and report scores; running
them as guests verifies xvisor doesn't silently corrupt long-running
computation.

**Demo.** `make guest-coremark` produces a CoreMark score within ±5 % of
the same benchmark's xemu-native score. Same for dhrystone and microbench.

**What you'll learn.** Where the per-VM-exit overhead lands on real
workloads. Whether F/D virtualization (via `vsstatus.FS`) is solid. A
preview of the perf work that comes after correctness.

**Estimated effort.** Small — one to two days, mostly Makefile and DTB
plumbing. Bugs surfaced here may take longer to chase, but those bugs
were going to bite Linux anyway.

---

### P5 — Linux to "early printk"

**Goal.** Boot the kernel far enough to see `Booting Linux on hart 0`
followed by early printk. Don't aim for userspace yet.

**Mechanism.**

- Load `resource/linux/arch/riscv/boot/Image` at GPA `0x80200000`.
- Load `resource/xemu.dtb` (simplified guest version, one CPU, ns16550 at
  `0x10000000`) at a GPA we tell the guest.
- Map UART MMIO (`0x10000000..0x10001000`) directly into G-stage as
  passthrough (`R/W/X = 1`, no emulation). Linux writes the UART and the
  bytes show up on the host's UART transparently.
- Set `a0 = 0` (hartid), `a1 = guest_DTB_GPA`, `sepc = 0x80200000`,
  `hstatus.SPV = 1`, then `sret`.

**Demo.** `make linux-early` shows kernel banner, decompressor banner,
"Booting Linux on hart 0", initial printks. Eventually it panics looking
for something we haven't provided yet — that panic is P6's input.

**What you'll learn.** RISC-V Linux boot protocol (`a0`/`a1` ABI),
why DTB is the discovery mechanism for early boot, why UART passthrough is
a defensible shortcut for a single-guest hypervisor.

**Estimated effort.** Medium — one week. Substantially de-risked by P3.5
and P4.5: the SBI ABI is exercised, F/D is exercised, the loader is
exercised. What's left is mostly DTB-driven discovery.

---

### P6 — Linux to shell

**Goal.** Reach the busybox/initramfs shell prompt.

**Mechanism.** Whatever P5's panic told us we're missing. Likely:

- **PLIC virtualization.** Either trap-and-emulate the PLIC's MMIO claim/
  complete registers, or pass it through with care. Linux needs working
  external interrupts for the UART RX path.
- **VS-mode external interrupt injection.** Set `hvip.VSEIP` to inject;
  Linux's PLIC driver handles the rest.
- **Whatever additional SBI calls** Linux's PLIC/console drivers issue
  beyond P4's set.

**Demo.** `make linux` → `/ # _` prompt over UART; commands work.

**What you'll learn.** Interrupt virtualization at the PLIC level, the
difference between *delivery* (the host PLIC fires) and *injection* (we
make VS-mode see a virtual interrupt), why most production hypervisors
don't pass through interrupt controllers.

**Estimated effort.** Large — one to two weeks. Most of the time will be
reading panics and learning what Linux expects.

---

### P7 — xv6 as guest (optional consolidation)

**Goal.** Boot `resource/xv6/` as a guest. Optional consolidation between
P6 (Linux to shell) and P9 (xemu H-ext). Skip if P6 went smoothly.

**Mechanism.** xv6 is simpler than Linux but exercises a different SBI
surface and different DTB layout. Treating xv6 as a regression target
catches anything we accidentally specialised for Linux 6.1.

**Demo.** `make xv6-guest` reaches the xv6 shell.

**Estimated effort.** Small-to-medium — two to four days. May surface
bugs in PLIC virtualization that Linux's drivers happened to mask.

---

### P8 — Debian VirtIO-blk guest (optional)

**Goal.** Boot Debian 13 Trixie as a guest with VirtIO-blk rootfs, mirroring
xemu's Phase 10 milestone.

**Mechanism.** VirtIO-blk needs MMIO virtualization (likely trap-and-emulate
on the MMIO config region, possibly passthrough on the queue pages).
`virtio_blk.rs` in the layout above is reserved for this.

**Demo.** `make debian-guest` → Debian login prompt.

**Estimated effort.** Medium — one week. Mostly a port of xemu's VirtIO-blk
work to "what does virtualization of the same device look like."

---

### P9 — xemu gains H-extension support

**Goal.** xvisor self-hosts on xemu. Closes the project loop: xemu emulates
the H-ext, xvisor uses the H-ext, both stay correct via difftest against
QEMU.

**Mechanism.** Independent work in `xemu/xcore`:

- Add HS and VS privilege levels alongside the existing M/S/U.
- Implement H-ext CSRs: `hgatp`, `hstatus`, `hedeleg`, `hideleg`, `hie`,
  `hip`, `hvip`, `htimedelta`, `htval`, `htinst`, `hgeie`, `hgeip`, and
  the VS shadows (`vsstatus`, `vsie`, `vstvec`, `vsscratch`, `vsepc`,
  `vscause`, `vstval`, `vsip`, `vsatp`, `vstimecmp`).
- Implement two-stage translation (VS-stage Sv39 → G-stage Sv39x4).
- Implement `hfence.vvma` and `hfence.gvma` with appropriate TLB
  invalidation.
- Implement guest-page-fault, virtual-instruction, and VS-mode interrupt
  causes.
- Wire `csr_table!`'s `@difftest` annotation into the new CSRs so existing
  difftest catches regressions against QEMU.

**Demo.** `xemu xvisor.elf` boots Linux on top of xvisor on top of xemu.
Difftest against `qemu -cpu rv64,h=true` running the same xvisor passes
per-instruction for the entire boot.

**What you'll learn.** The full H-ext from the emulator side; why two-stage
translation is the most expensive thing on the per-VM-exit path in real
hardware; how QEMU implements it.

**Estimated effort.** Very large — three to six weeks, comparable to xemu's
Phase 3 (MMU) in scope. Worth opening as its own `/ark:design --deep` task.

## Why this order

Four principles drive the sequencing:

- **No guest until the scaffolding is proven** (P0–P2). Type-1 hypervisor
  bugs in P3+ are hard to debug; trap/CSR bugs caught in P0–P2 are easy
  because there's no guest to confuse the signals with.
- **Reuse before invention** (P2.5 onward). Every guest from P3 forward is
  built on xam / xlib / xkernels — code whose correct behaviour is already
  proven on xemu. Any divergence under xvisor is unambiguously xvisor's
  fault.
- **Graduated guests before Linux** (P3.5, P4.5 before P5). Linux is the
  hardest guest. Reaching it through am-tests → benchmarks → Linux means
  every Linux panic is *new* signal, not "we don't handle ecall yet."
- **The emulator extension is dead last** (P9). Don't write the H-ext
  emulator until xvisor is correct against QEMU's reference. Otherwise
  you're debugging two unknowns at once — a classic way to lose a month.

## Reused Artifacts from ProjectX

| Asset                       | Source                                    | Use in xvisor                                                                                     |
| --------------------------- | ----------------------------------------- | ------------------------------------------------------------------------------------------------- |
| OpenSBI v1.3.1              | `resource/opensbi/`                       | M-mode firmware below xvisor (QEMU `-bios default` uses its own copy; ours is for hardware later) |
| Linux 6.1.44 Image          | `resource/linux/` build                   | The guest kernel from P5 onward                                                                   |
| initramfs                   | `resource/linux/rootfs/`                  | Guest userspace                                                                                   |
| `xemu.dts` family           | `resource/`                               | Starting point for guest DTB                                                                      |
| xv6 / Debian images         | `resource/xv6/`, `resource/debian/`       | Regression guests in P7 / P8                                                                      |
| **xam HAL**                 | `xam/xhal/`                               | New `xvisor-guest` platform target (P2.5); existing platforms unchanged                           |
| **xlib**                    | `xlib/`                                   | Used as-is by every am-test running as a guest                                                    |
| **xkernels**                | `xkernels/tests/`, `xkernels/benchmarks/` | am-tests = correctness guests (P3, P3.5); benchmarks = stress guests (P4.5)                       |
| ns16550 driver model        | `xemu/xcore/src/device/uart`              | Reference when we emulate UART instead of passing it through                                      |
| CSR mask semantics          | `xemu/xcore/src/cpu/csr`                  | Reference for `xvisor/src/arch/riscv/csr.rs` (different CSR subset, same WARL conventions)        |
| `xemu` as oracle            | `xemu` running am-tests natively          | Stdout comparison for P3.5 guest correctness                                                      |
| `.ark/specs/project/` rules | `.ark/`                                   | xvisor follows project-wide conventions                                                           |

## Open Questions

These shape decisions in P0–P2.5; flagging now rather than deciding silently
later.

1. **Toolchain pin.** ProjectX uses `nightly-2026-03-15`. Bare-metal
   `riscv64gc-unknown-none-elf` builds usually work on that pin. Inherit,
   or pin a separate nightly for `xvisor/`?
2. **xam platform naming.** `xvisor-guest` is descriptive but long. Shorter
   alternatives: `vsmode`, `smode-sbi`, `guest`. `xvisor-guest` reads best
   in `make PLATFORM=…` invocations and pairs with the existing `xemu` /
   `riscv64-qemu-virt` naming convention; keep unless someone objects.
3. **xam SBI putchar: legacy vs DBCN.** P2.5 starts with legacy
   `console_putchar` (EID `0x01`, FID `0`) because every existing
   bare-metal RISC-V codebase uses it. Migrate xam to v2.0 DBCN
   (`0x4442434E`) once P4 is done, or stay on legacy forever?
4. **Guest kernel version.** Stay on Linux 6.1.44 (matches xemu's boot
   recipe), or move to a newer LTS (6.6, 6.12) for xvisor? 6.1 is known to
   boot under QEMU H-ext; newer kernels add SBI calls we'd have to handle
   sooner.
5. **SMP scope.** Single-hart guest through P6, multi-hart only after? The
   hvisor "static partitioning, one zone per pCPU, no scheduler" model is
   the smallest possible multi-hart story; defer entirely until P6 lands.
6. **Ark workflow.** Open this as `/ark:design --deep "xvisor: type-1
   RISC-V hypervisor"` per ProjectX conventions, or treat it as a
   long-running parallel project outside Ark? Deep-tier with one Ark task
   *per phase* is probably the right grain — xvisor as a whole is too
   large for a single PLAN.
7. **Difftest scope for P9.** Difftest against `qemu -cpu rv64,h=true` is
   the obvious reference. Spike has H-ext support too — worth supporting
   as a second backend, or QEMU alone?
8. **xam refactor risk.** Adding `xvisor-guest` as a new platform should
   not require any change to existing `xemu` / `qemu_virt` platforms. If
   P2.5 surfaces a need to refactor shared code (e.g., trap frame layout
   diverging between M-mode and S-mode), pause and decide whether to (a)
   keep `xvisor-guest` fully independent, (b) extract shared trap-frame
   helpers, or (c) refactor xam more broadly. Decide in the moment, not
   upfront.

## Risks and Mitigations

| Risk                                                                      | Likelihood | Mitigation                                                                                                                                                                                                                                                    |
| ------------------------------------------------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| RISC-V H-ext spec ambiguity (rare cases where QEMU and the spec disagree) | Medium     | Test against QEMU first; treat QEMU as ground truth until P9 forces a comparison with the actual spec.                                                                                                                                                        |
| Guest panics in P5–P6 are deep in Linux internals                         | High       | P3.5 and P4.5 exercise SBI, F/D, and timer paths against known-good guests *before* Linux. Build Linux with `CONFIG_DEBUG_KERNEL=y` and `earlyprintk` so remaining panics are loud.                                                                           |
| `vsatp` / Sstc semantics under emulation differ from hardware             | Medium     | Pin to QEMU first; only port to xemu (P9) once xvisor is stable. Real hardware bring-up is explicitly out of scope.                                                                                                                                           |
| P9 doubles project size                                                   | High       | P9 is gated. P6 is a complete deliverable on its own (xvisor on QEMU boots Linux); P9 is "nice to have, closes the loop." Treat as separate Ark task.                                                                                                         |
| H-ext support in `nightly-2026-03-15` rustc backend                       | Low        | The H-ext CSRs are accessed via inline assembly, not LLVM intrinsics. Should work on any recent nightly. Verify in P0.                                                                                                                                        |
| xam `xvisor-guest` platform leaks M-mode assumptions                      | Medium     | P2.5 includes a smoke test under xemu's S-mode boot path *before* xvisor is involved. Decouples xam correctness from xvisor correctness.                                                                                                                      |
| am-tests rely on xemu-specific MMIO not present under xvisor              | Medium     | P3.5 routes everything through SBI; direct MMIO in am-tests (ACLINT, UART) is replaced by SBI calls in the `xvisor-guest` platform's putch/timer. Tests using raw MMIO (PLIC am-test) are out of scope for P3.5 — covered later when xvisor virtualises PLIC. |

## Reference Material

- **RISC-V Privileged Spec, Chapter H** — the authoritative source for
  hgatp/hstatus/VS-mode semantics. Read before P2.
- **SBI Specification v2.0** — defines the EID/FID surface. Skim before P4.
- **QEMU `hw/riscv/`** — reference H-ext implementation. Read alongside P9.
- **hvisor `src/arch/riscv64/`** (`resources/systems/hvisor` in the
  Astervisor repo) — the closest production-quality Rust template; uses
  H-ext on real hardware and QEMU. Don't copy; consult when stuck.
- **rust-shyper, salus** — other Rust RISC-V hypervisor codebases worth
  knowing exist.

## Status

| Phase | Status      | Notes                            |
| ----: | ----------- | -------------------------------- |
|    P0 | Not started | Awaiting plan approval           |
|    P1 | Not started |                                  |
|    P2 | Not started |                                  |
|  P2.5 | Not started | xam `xvisor-guest` platform      |
|    P3 | Not started | First guest = P2.5 hello am-test |
|  P3.5 | Not started | am-tests as guests               |
|    P4 | Not started | SBI shim (formalised)            |
|  P4.5 | Not started | Benchmarks as guests             |
|    P5 | Not started |                                  |
|    P6 | Not started |                                  |
|    P7 | Not started | Optional: xv6 guest              |
|    P8 | Not started | Optional: Debian guest           |
|    P9 | Not started | Gated on P6 completion           |
