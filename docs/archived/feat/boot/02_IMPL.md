# `OS Boot` IMPL `02`

> Feature: `boot`
> Iteration: `02`
> Owner: Executor
> Approved Plan: `02_PLAN.md`

---

## Summary

Implemented Phase 7: OS Boot. xemu now boots OpenSBI firmware, xv6-riscv (with ramdisk), and Linux (with initramfs). The boot chain is: OpenSBI (M-mode) → kernel (S-mode), or direct M-mode boot for xv6.

## Completed Scope

### Emulator Changes (`xcore`)

**CSR additions:**
- `misa` (0x301) — initialized with IMACSU + MXL at reset (was 0)
- `stimecmp` (0x14D) — Sstc timer compare, fires STIP when `mtime >= stimecmp`
- `menvcfg` (0x30A) — M-mode environment config (STCE/PBMTE/FIOM mask)
- `senvcfg` (0x10A) — S-mode environment config (FIOM mask)
- `time` (0xC01) — read-only shadow of ACLINT mtime, counter-gated

**New instructions:**
- `fence` — NOP (single-hart, no cache)
- `fence.i` — NOP (no instruction cache)
- `wfi` — NOP with privilege checks (U-mode traps, S-mode TW check)

**Boot infrastructure:**
- `BootConfig` enum: `Direct { file }` | `Firmware { fw, kernel, initrd, fdt }`
- `BootMode` in `CoreOps` trait: arch-independent boot mode contract
- `RVCore::setup_boot()` — sets a0/a1 registers per SBI convention, configures ebreak behavior
- `CPU::boot()` stores config, `reset()` reapplies it — boot mode survives reset
- Firmware loads OpenSBI at 0x80000000, kernel at 0x80200000, initrd at 0x84000000, FDT at 0x87F00000

**Bug fixes discovered during boot:**
- Hardware A/D bit update in page walk — replaced Svade faulting with automatic A/D bit setting (xv6/Linux expect hardware update, not software fault handling)
- PMP NAPOT full-address-space overflow — `napot_range()` computed wrong base address for `pmpaddr=0x3FFFFFFFFFFFFF` due to shift overflow, blocking S-mode access to kernel memory
- UART THRE interrupt — NS16550A transmit-holding-register-empty interrupt was not implemented; xv6 TX pipeline relies on THRE interrupts to drive character output
- THRE interrupt storm — delayed THRE assertion by one tick to prevent per-character trap overhead (140x slowdown)
- ebreak behavior — bare-metal halt vs firmware trap dispatch, controlled by `BootMode`

### Resource Directory (`resource/`)

**Build system:**
- `Makefile` — top-level, includes `opensbi.mk`, `xv6.mk`, `linux.mk`
- `opensbi.mk` — fetch + build OpenSBI v1.3.1 fw_jump.bin
- `xv6.mk` — fetch + patch + build xv6-riscv with ramdisk driver
- `linux.mk` — download pre-built Linux Image + rootfs from bootlin.com
- `Dockerfile` — x86_64 Ubuntu build environment (for Buildroot, optional)

**Device tree:**
- `xemu.dts` — describes xemu hardware: ACLINT, PLIC, UART, memory, CPU ISA, initrd addresses

**xv6 patches:**
- `patches/xv6/ramdisk.patch` — Makefile: swap virtio_disk → ramdisk, link embedded fs.img
- `patches/xv6/ramdisk.c` — ramdisk driver serving block I/O from in-memory fs.img

### Boot Chains Verified

| Target | Boot Chain | Status |
|--------|-----------|--------|
| OpenSBI | Boot ROM → OpenSBI (M-mode) → banner | Working |
| xv6 | Direct load → start.c (M-mode) → mret → main (S-mode) → shell | Working |
| Linux | OpenSBI → mret → Linux (S-mode) → initramfs → init | Working (slow, needs Phase 8 optimization) |

## Deviations from Approved Plan

| Plan | Implementation | Reason |
|------|---------------|--------|
| Boot ROM device at 0x1000 | Direct register setup via `setup_boot()` | Simpler — no MMIO device, no address materialization, just set a0/a1/PC |
| `set_pc`/`set_gpr` on CoreOps | `setup_boot(BootMode)` on CoreOps | Cleaner abstraction — arch-specific details stay in RVCore |
| Svade A/D faulting | Hardware A/D update | xv6/Linux expect hardware to set A/D bits |

## Verification

- 278 xcore unit tests passing
- 7 am-tests passing (UART, ACLINT, PLIC, CSR, trap, interrupts)
- xv6 boots to interactive shell via PTY
- Linux 6.1 boots to initramfs unpacking, reaches init
- clippy clean, fmt clean
