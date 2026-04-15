# `OS Boot` PLAN `00`

> Status: Draft
> Feature: `boot`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

Boot OpenSBI firmware in M-mode, then chain-load an S-mode kernel (xv6-riscv, then Linux). This is Phase 7 of the xemu roadmap — the culmination of all previous work (ISA, CSR, MMU, devices, traps, interrupts).

The plan follows a **three-stage boot chain**: xemu reset → OpenSBI (M-mode) → kernel (S-mode). The emulator provides a Flattened Device Tree (FDT) describing its hardware, a boot ROM trampoline at the reset vector, and pre-built firmware/kernel binaries in `resource/`. No new devices are required for the first milestone (OpenSBI console output); VGA and disk are deferred to later iterations.

## Log

None (initial plan).

---

## Spec

[**Goals**]

- G-1: Boot OpenSBI `fw_jump.bin` to M-mode console output (UART "Hello from OpenSBI")
- G-2: Chain-load xv6-riscv kernel in S-mode via OpenSBI, reach xv6 shell prompt
- G-3: Chain-load minimal Linux (initramfs) in S-mode, reach `/ #` shell
- G-4: Provide pre-built binaries and build scripts in `resource/` for reproducibility
- G-5: Pass FDT to firmware describing xemu's exact hardware topology

- NG-1: Multi-hart / SMP support (single hart only)
- NG-2: VGA framebuffer, disk, or network devices
- NG-3: Custom OpenSBI platform (use `generic` platform)
- NG-4: ELF loader (raw binary loading is sufficient; OpenSBI/Linux provide `.bin` images)

[**Architecture**]

```
Boot flow:

  ┌─────────────────┐     a0=hartid     ┌──────────────┐     a0=hartid     ┌────────────┐
  │  Boot ROM       │     a1=&fdt       │  OpenSBI     │     a1=&fdt       │  Kernel    │
  │  (0x1000, 64B)  │ ──────────────>   │  (0x80000000)│ ──────────────>   │ (FW_JUMP_  │
  │  set a0/a1/a2   │     jump          │  M-mode init │     mret          │  ADDR)     │
  │  jump 0x80000000│                   │  PMP, SBI    │                   │  S-mode    │
  └─────────────────┘                   └──────────────┘                   └────────────┘

Memory layout:
  0x0000_1000  Boot ROM (trampoline, 64 bytes)
  0x0200_0000  ACLINT (MSWI + MTIMER + SSWI)
  0x0C00_0000  PLIC (32 sources, 2 contexts)
  0x1000_0000  UART0 (NS16550A)
  0x8000_0000  DRAM base (128 MB)
  0x8000_0000  OpenSBI fw_jump.bin loaded here
  0x8020_0000  Kernel loaded here (FW_JUMP_ADDR)
  0x8700_0000  FDT blob loaded here (near top of DRAM, 2MB-aligned)
```

The boot ROM is a new MMIO device at `0x1000` containing a small RISC-V trampoline that:
1. Sets `a0 = 0` (hartid)
2. Sets `a1 = FDT_ADDR` (where FDT is loaded in DRAM)
3. Jumps to `0x8000_0000` (OpenSBI entry)

The reset vector (`RESET_VECTOR`) changes from `0x8000_0000` to `0x1000` when booting firmware.

[**Invariants**]

- I-1: OpenSBI requires M-mode entry, `a0`=hartid, `a1`=FDT physical address (8-byte aligned)
- I-2: FDT must accurately describe xemu hardware: ACLINT base/size, PLIC base/size/ndev, UART base/irq, memory base/size, CPU ISA string, timebase-frequency
- I-3: `misa` CSR must report extensions matching the FDT `riscv,isa` string (I, M, A, C, S, U)
- I-4: ACLINT register layout must match `riscv,clint0` compatible (MSIP@0x0, mtimecmp@0x4000, mtime@0xBFF8) — already correct
- I-5: PLIC must support both M-mode and S-mode contexts with correct `interrupts-extended` mapping
- I-6: UART must respond to NS16550A register protocol — already implemented
- I-7: Boot ROM must be read-only from guest perspective
- I-8: Kernel address (`FW_JUMP_ADDR`) must not overlap OpenSBI reserved region (~256KB from DRAM base)

[**Data Structure**]

```rust
/// Read-only boot ROM device — holds the trampoline instructions.
pub struct BootRom {
    data: Vec<u8>,
}

impl Device for BootRom {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, ..) -> XResult { Err(XError::BadAddress) }  // read-only
}
```

No new Rust types for FDT — we compile a `.dts` file offline with `dtc` and load the resulting `.dtb` as raw bytes into DRAM at `FDT_ADDR`.

[**API Surface**]

```rust
/// Generate the boot ROM trampoline as raw bytes.
/// `fdt_addr`: physical address where FDT is loaded in DRAM.
fn build_boot_rom(fdt_addr: usize) -> Vec<u8>;

/// Load firmware + kernel + FDT into the bus, add boot ROM device,
/// set reset vector to boot ROM address.
fn setup_boot(bus: &mut Bus, fw_path: &str, kernel_path: Option<&str>, fdt_path: &str);
```

Build system additions (Makefile):

```makefile
# Boot targets
OPENSBI  ?= resource/opensbi/fw_jump.bin
KERNEL   ?= resource/xv6/kernel.bin
FDT      ?= resource/xemu.dtb

boot: $(OPENSBI) $(KERNEL) $(FDT)
	$(MAKE) run FILE=$(OPENSBI) KERNEL=$(KERNEL) FDT=$(FDT)
```

[**Constraints**]

- C-1: No new crate dependencies for FDT generation — use external `dtc` tool, load `.dtb` as bytes
- C-2: Boot ROM is a minimal MMIO device (≤64 bytes of RISC-V instructions)
- C-3: `misa` must return correct value: bit 0 (A) | bit 2 (C) | bit 8 (I) | bit 12 (M) | bit 18 (S) | bit 20 (U) | MXL=2 (RV64) in bits [63:62]
- C-4: `mhartid` must return 0 (single-hart)
- C-5: OpenSBI `generic` platform with `FW_JUMP_ADDR=0x80200000` — no custom platform code
- C-6: Pre-built binaries in `resource/` with build scripts for reproducibility
- C-7: Existing am-tests and cpu-tests must continue to pass (boot ROM only active when firmware is loaded)

---

## Implement

### Execution Flow

[**Main Flow**]

1. User runs `make boot` (or `make run` with `FW`/`KERNEL`/`FDT` env vars)
2. Emulator detects firmware mode: `FW` env var set → boot mode; otherwise legacy direct-load mode
3. Boot setup:
   a. Load `fw_jump.bin` at `0x8000_0000`
   b. Load `kernel.bin` at `0x8020_0000` (if provided)
   c. Load `xemu.dtb` at `FDT_ADDR` (0x8700_0000)
   d. Generate boot ROM trampoline with `FDT_ADDR` baked in
   e. Add boot ROM as MMIO device at `0x1000`
   f. Set `RESET_VECTOR` to `0x1000`
4. CPU starts in M-mode at `0x1000` (boot ROM)
5. Boot ROM sets `a0=0`, `a1=FDT_ADDR`, jumps to `0x8000_0000`
6. OpenSBI initializes: parses FDT, configures PMP, sets up SBI handler, prints banner
7. OpenSBI drops to S-mode, jumps to `FW_JUMP_ADDR` (`0x8020_0000`) with `a0=0`, `a1=FDT_ADDR`
8. Kernel boots in S-mode, uses SBI ecalls for console/timer

[**Failure Flow**]

1. Missing firmware binary → clear error message, abort before CPU start
2. FDT address misaligned → panic at setup time (programming error)
3. OpenSBI fails to parse FDT → OpenSBI prints error to UART (visible to user)
4. Illegal instruction during boot → trap handler fires, visible via xdb debugger
5. Timer not firing → xdb `info reg mip` shows stuck MTIP, difftest comparison reveals divergence

[**State Transition**]

- Boot ROM (M-mode, PC=0x1000) → OpenSBI (M-mode, PC=0x8000_0000) via jump
- OpenSBI (M-mode) → Kernel (S-mode, PC=0x8020_0000) via mret
- Kernel (S-mode) → SBI handler (M-mode) via ecall → back to S-mode via mret

### Implementation Plan

[**Phase 7a: OpenSBI Console (this iteration)**]

1. **`misa` CSR fix** — Return correct value encoding xemu's extensions (IMACSU + MXL)
2. **Boot ROM device** — Read-only MMIO at `0x1000`, generates RV64 trampoline
3. **FDT source file** — `resource/xemu.dts` describing xemu hardware, compiled with `dtc`
4. **Boot loader** — `setup_boot()` loads fw + kernel + FDT into bus, registers boot ROM
5. **Makefile integration** — `make boot` target, `FW`/`KERNEL`/`FDT` env vars
6. **Resource scripts** — `resource/opensbi/build.sh` cross-compiles OpenSBI `fw_jump.bin`
7. **Validation** — OpenSBI prints banner to UART

[**Phase 7b: xv6-riscv (next iteration)**]

1. Build xv6-riscv kernel as `kernel.bin`
2. Verify SBI legacy ecalls work (putchar, set_timer)
3. xv6 shell prompt via UART PTY

[**Phase 7c: Linux (future iteration)**]

1. Build minimal Linux kernel + initramfs
2. Implement modern SBI extensions (TIME, IPI, RFENCE, HSM, SRST, BASE)
3. Linux `/ #` prompt

---

## Trade-offs

- T-1: **Boot ROM vs direct register setup**
  - Option A: Add a boot ROM device at `0x1000` with a trampoline (QEMU approach). More realistic, matches hardware convention, works with any firmware.
  - Option B: Directly set `a0`/`a1` registers and PC=`0x8000_0000` at reset (simpler, NEMU approach). Less code, but ties boot logic to CPU reset path.
  - Current choice: **Option A** — clean separation, extensible for multi-hart later, matches QEMU memory map convention.

- T-2: **FDT: compile-time `.dts` → `.dtb` vs runtime generation**
  - Option A: Write `.dts` file, compile with `dtc` at build time, load `.dtb` as bytes. Simple, standard tooling, human-readable source.
  - Option B: Generate FDT in Rust at runtime using a library. More flexible but adds dependency, harder to debug.
  - Current choice: **Option A** — matches C-1 (no new deps), FDT is static for xemu's fixed hardware.

- T-3: **OpenSBI firmware type: `fw_jump` vs `fw_payload` vs `fw_dynamic`**
  - `fw_jump`: Simplest. Firmware jumps to a compile-time fixed address. Emulator loads firmware + kernel separately.
  - `fw_payload`: Single binary (firmware + kernel embedded). Simplest loading, but requires rebuilding OpenSBI for each kernel.
  - `fw_dynamic`: Most flexible (QEMU default). Requires `struct fw_dynamic_info` in memory + `a2` register setup.
  - Current choice: **`fw_jump`** — good balance of simplicity and flexibility. Separate kernel loading allows swapping kernels without rebuilding OpenSBI.

---

## Validation

[**Unit Tests**]

- V-UT-1: Boot ROM device returns correct trampoline instructions on read, rejects writes
- V-UT-2: `misa` CSR returns correct extension bits (IMACSU + MXL=2)
- V-UT-3: `mhartid` CSR returns 0
- V-UT-4: `build_boot_rom()` generates valid RV64 instructions that set `a0=0`, `a1=FDT_ADDR`, jump to `0x8000_0000`

[**Integration Tests**]

- V-IT-1: OpenSBI `fw_jump.bin` boots to banner output on UART (stdout capture)
- V-IT-2: Existing am-tests continue to pass (no regression from boot ROM addition)
- V-IT-3: Existing cpu-tests-rs continue to pass

[**Failure / Robustness Validation**]

- V-F-1: Missing firmware file produces clear error, does not crash
- V-F-2: Boot without FDT fails gracefully (OpenSBI may print error, emulator stays debuggable)
- V-F-3: Invalid FDT blob doesn't crash emulator (OpenSBI handles parse failure)

[**Edge Case Validation**]

- V-E-1: Legacy mode (no `FW` env var) behaves exactly as before — boot ROM not added, RESET_VECTOR=0x8000_0000
- V-E-2: FDT loaded at top of DRAM doesn't overflow RAM bounds
- V-E-3: Boot ROM reads at non-4-byte-aligned offsets work correctly (for compressed instruction fetch)

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (OpenSBI console) | V-IT-1: banner on UART |
| G-5 (FDT) | V-UT-4: correct trampoline; V-IT-1: OpenSBI parses FDT |
| C-1 (no new deps) | Code review: no new entries in Cargo.toml |
| C-3 (misa) | V-UT-2: correct extension bits |
| C-7 (no regression) | V-IT-2, V-IT-3: existing tests pass |
