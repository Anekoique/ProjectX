# `OS Boot` PLAN `01`

> Status: Draft
> Feature: `boot`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `00_MASTER.md`

---

## Summary

Revised plan for Phase 7a: **OpenSBI console bring-up only**. This round narrows scope to a single verifiable milestone — OpenSBI prints its banner to UART. xv6 and Linux are explicitly deferred to future iterations. All four blocking findings from round 00 are resolved: scope is precise, the DT contract is concrete, boot-mode integration is defined via `BootConfig`, and build logic lives outside xemu's Makefile.

## Log

[**Feature Introduce**]

- `BootConfig` enum (`Direct` vs `Firmware`) replaces implicit `FW` env var detection
- Concrete DTS with exact compatible strings, properties, `/chosen`, and memory reservation
- Boot logic Makefile placed in `resource/Makefile`, not in xemu's Makefile
- `misa` CSR initialized with correct IMACSU + MXL value at reset

[**Review Adjustments**]

- R-001: Narrowed scope — G-2 (xv6 shell) and G-3 (Linux) removed from this round entirely
- R-002: Full DTS contract specified with all compatible strings, properties, reserved-memory node
- R-003: `BootConfig` enum + `CPU::boot()` method defined; legacy `CPU::load()` unchanged
- R-004: Round 01 is explicitly OpenSBI-only; acceptance mapping covers only this milestone

[**Master Compliance**]

- M-001: Boot Makefile placed at `resource/Makefile`, not in xemu's Makefile
- M-002: All reviewer findings addressed in Response Matrix below
- M-003: Changes are additive — no existing framework broken; boot ROM is a new Device impl, `BootConfig` is a new enum, `misa` init is a one-line change in `CsrFile::new()`

### Changes from Previous Round

[**Added**]

- `BootConfig` enum with `Direct { file }` and `Firmware { fw, kernel, fdt }` variants
- `CPU::boot(config)` method that dispatches on `BootConfig`
- Concrete DTS with `/reserved-memory` node for OpenSBI resident region
- `resource/Makefile` with `fetch-opensbi`, `build-opensbi`, `dtb` targets
- `resource/xemu.dts` as the single source of truth for hardware description
- `misa` initialization in `CsrFile::new()` with correct extension bits

[**Changed**]

- Scope narrowed to OpenSBI console only (Phase 7a)
- Boot ROM trampoline now includes `a2 = 0` (fw_dynamic info = NULL, safe for fw_jump)
- FDT address moved to `0x87F0_0000` (16-byte aligned, within DRAM, leaves room for 128MB images)

[**Removed**]

- G-2 (xv6 shell), G-3 (Linux shell) from this round's goals
- All `FW`/`KERNEL`/`FDT` env vars from xemu Makefile — boot is driven from `resource/Makefile`

[**Unresolved**]

- xv6 storage strategy (disk vs initramfs) — deferred to round 02
- Linux SBI extensions (TIME, IPI, HSM, etc.) — deferred to round 03

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | G-2/G-3 removed from this round; scope is OpenSBI console only |
| Review | R-002 | Accepted | Full DTS contract in Spec section with all compatible strings, properties, reserved-memory |
| Review | R-003 | Accepted | `BootConfig` enum + `CPU::boot()` defined; legacy `CPU::load()` path unchanged |
| Review | R-004 | Accepted | Round 01 is explicitly OpenSBI-only; acceptance mapping precise |
| Review | TR-1 | Accepted | `resource/` layout defined: pinned upstream rev + fetch/build scripts + `.gitignore` for generated binaries |
| Review | TR-2 | Accepted | Boot ROM kept; `BootConfig` makes boot selection explicit and reset-safe |
| Master | M-001 | Applied | Boot Makefile at `resource/Makefile`, xemu Makefile only gains a `boot` phony that delegates |
| Master | M-002 | Applied | All findings addressed above |
| Master | M-003 | Applied | Additive changes only; existing tests unaffected |

---

## Spec

[**Goals**]

- G-1: Boot OpenSBI `fw_jump.bin` in M-mode, see banner output on UART
- G-2: Provide reproducible build scripts in `resource/` for OpenSBI + DTB
- G-3: `misa` CSR reports correct IMACSU extensions
- G-4: Legacy direct-load mode (`make run FILE=...`) works exactly as before

- NG-1: xv6 or Linux boot (deferred to future rounds)
- NG-2: Multi-hart / SMP
- NG-3: VGA, disk, network devices
- NG-4: ELF loader (OpenSBI provides `.bin`)

[**Architecture**]

```
Boot modes (selected by BootConfig):

  Direct mode (legacy, unchanged):
    CPU reset → PC = 0x8000_0000 → run binary

  Firmware mode (new):
    CPU reset → PC = 0x0000_1000 (boot ROM)
             → a0=0, a1=FDT_ADDR, a2=0
             → jump 0x8000_0000
             → OpenSBI (M-mode)
             → mret to FW_JUMP_ADDR (S-mode kernel, if loaded)

Memory layout (firmware mode):
  0x0000_1000 .. 0x0000_1040  Boot ROM (read-only, 64 bytes)
  0x0200_0000 .. 0x0201_0000  ACLINT
  0x0C00_0000 .. 0x1000_0000  PLIC
  0x1000_0000 .. 0x1000_0100  UART0
  0x8000_0000 .. 0x8800_0000  DRAM (128 MB)
    0x8000_0000                  OpenSBI fw_jump.bin
    0x8020_0000                  Kernel entry (FW_JUMP_ADDR)
    0x87F0_0000                  FDT blob
```

[**Invariants**]

- I-1: `BootConfig::Direct` does not add boot ROM or change reset vector — exact legacy behavior
- I-2: `BootConfig::Firmware` adds boot ROM at `0x1000`, sets reset vector to `0x1000`
- I-3: FDT address is 8-byte aligned and within DRAM bounds
- I-4: Boot ROM is read-only (writes return `BadAddress`)
- I-5: `misa` value matches FDT `riscv,isa` string: `rv64imacsu`
- I-6: OpenSBI entry: M-mode, `a0=0`, `a1=FDT_ADDR`, `a2=0`, `satp=0`, all other GPRs = 0
- I-7: ACLINT layout matches `riscv,clint0` compatible: MSIP@0x0, mtimecmp@0x4000, mtime@0xBFF8
- I-8: PLIC layout matches `sifive,plic-1.0.0`: priority@0x0, pending@0x1000, enable@0x2000, threshold/claim per context
- I-9: UART matches `ns16550a`: base 0x1000_0000, irq 10, clock-frequency 3686400

[**Data Structure**]

```rust
/// Boot configuration — selects between legacy direct-load and firmware boot.
pub enum BootConfig {
    /// Legacy: load one binary at DRAM base, reset to 0x8000_0000.
    Direct { file: Option<String> },
    /// Firmware: load OpenSBI + optional kernel + FDT, reset to boot ROM.
    Firmware {
        fw: String,
        kernel: Option<String>,
        fdt: String,
    },
}

/// Read-only boot ROM device.
pub struct BootRom {
    data: Vec<u8>,
}
```

[**API Surface**]

```rust
// xcore::device::boot_rom
impl BootRom {
    /// Build a boot ROM trampoline that sets a0=0, a1=fdt_addr, jumps to entry.
    pub fn new(fdt_addr: usize, entry: usize) -> Self;
}
impl Device for BootRom { /* read-only */ }

// xcore::cpu::mod
impl CPU<Core> {
    /// Boot from a configuration. Resets CPU, loads images, optionally adds boot ROM.
    pub fn boot(&mut self, config: BootConfig) -> XResult;
}
```

[**Constraints**]

- C-1: No new crate dependencies — boot ROM generates raw bytes, FDT compiled externally with `dtc`
- C-2: Boot ROM ≤ 64 bytes (handful of RV64 instructions)
- C-3: `misa` for RV64: `(2 << 62) | (1<<0) | (1<<2) | (1<<8) | (1<<12) | (1<<18) | (1<<20)` = `0x8000_0000_0014_1101`
- C-4: `resource/Makefile` handles all external artifact build; xemu Makefile only delegates
- C-5: Generated binaries (`.bin`, `.dtb`) in `.gitignore`; only source (`.dts`) and scripts checked in
- C-6: Existing `make run`, `make test`, am-tests all pass unchanged

---

## Implement

### Execution Flow

[**Main Flow — Firmware Boot**]

1. User runs `make boot` from `resource/` (fetches OpenSBI, builds fw_jump.bin + xemu.dtb)
2. `resource/Makefile` invokes xemu: `make -C ../xemu run FW=fw_jump.bin KERNEL=kernel.bin FDT=xemu.dtb`
3. xemu Makefile passes `X_FW`, `X_KERNEL`, `X_FDT` env vars to cargo
4. `xdb/src/main.rs` constructs `BootConfig::Firmware { .. }` from env vars
5. `CPU::boot(config)` resets CPU, loads fw at `0x8000_0000`, kernel at `0x8020_0000`, FDT at `0x87F0_0000`
6. Adds `BootRom::new(0x87F0_0000, 0x8000_0000)` as MMIO at `0x1000`
7. Sets PC to `0x1000` (boot ROM entry)
8. CPU executes boot ROM → sets a0/a1/a2 → jumps to OpenSBI
9. OpenSBI parses FDT, initializes UART/PLIC/ACLINT, prints banner

[**Main Flow — Legacy Direct**]

1. User runs `make run FILE=binary.bin` (unchanged)
2. `xdb` constructs `BootConfig::Direct { file: Some("binary.bin") }`
3. `CPU::boot(config)` resets CPU, loads binary at `0x8000_0000`, PC = `0x8000_0000`
4. No boot ROM added — exact legacy behavior

[**Failure Flow**]

1. Missing firmware file → `CPU::boot()` returns `Err`, xdb prints error
2. FDT file missing → same error path
3. OpenSBI FDT parse failure → OpenSBI itself prints diagnostic to UART
4. Instruction fault during boot → trap handler fires, visible in xdb

[**State Transition**]

- `BootConfig::Direct` → reset PC = 0x8000_0000, no boot ROM
- `BootConfig::Firmware` → reset PC = 0x1000, boot ROM added
- Boot ROM (M-mode) → OpenSBI (M-mode) via `jalr` to 0x8000_0000
- OpenSBI (M-mode) → Kernel (S-mode) via `mret`

### Implementation Plan

[**Step 1: `misa` CSR initialization**]

In `CsrFile::new()`, set `regs[0x301]` to the correct IMACSU + MXL=2 value.

[**Step 2: Boot ROM device**]

New file `xcore/src/device/boot_rom.rs`:
- `BootRom::new(fdt_addr, entry)` generates RV64 trampoline instructions
- Trampoline: `lui a0, 0` / `lui a1, fdt_hi` + `addi a1, fdt_lo` / `lui a2, 0` / `lui t0, entry_hi` + `jalr zero, t0, entry_lo`
- `Device` impl: read returns instruction bytes, write returns `BadAddress`

[**Step 3: `BootConfig` and `CPU::boot()`**]

- Add `BootConfig` enum in `xcore/src/cpu/mod.rs`
- `CPU::boot()` dispatches: `Direct` → existing `load()` path; `Firmware` → load fw + kernel + fdt + add boot ROM + set PC
- Keep `CPU::load()` and `CPU::reset()` unchanged for backward compat

[**Step 4: xdb integration**]

- `xdb/src/main.rs` reads `X_FW`/`X_FDT`/`X_KERNEL` env vars
- If `X_FW` is set → `BootConfig::Firmware`; otherwise → `BootConfig::Direct`

[**Step 5: Makefile plumbing**]

- xemu `Makefile`: add `FW`/`KERNEL`/`FDT` optional vars, pass as `X_FW`/`X_KERNEL`/`X_FDT` env
- xemu `Makefile`: add `boot` phony target that delegates to `resource/Makefile`

[**Step 6: Resource directory**]

- `resource/Makefile`: targets for `fetch-opensbi`, `build-opensbi`, `dtb`, `boot`
- `resource/xemu.dts`: concrete device tree source
- `resource/.gitignore`: ignore `*.bin`, `*.dtb`, `opensbi/` build dir

[**Step 7: DTS file**]

Concrete `resource/xemu.dts`:

```dts
/dts-v1/;

/ {
    #address-cells = <2>;
    #size-cells = <2>;
    compatible = "xemu";
    model = "xemu-riscv64";

    chosen {
        stdout-path = "/soc/serial@10000000";
    };

    memory@80000000 {
        device_type = "memory";
        reg = <0x0 0x80000000 0x0 0x08000000>;
    };

    reserved-memory {
        #address-cells = <2>;
        #size-cells = <2>;
        ranges;

        opensbi@80000000 {
            reg = <0x0 0x80000000 0x0 0x00200000>;
            no-map;
        };
    };

    cpus {
        #address-cells = <1>;
        #size-cells = <0>;
        timebase-frequency = <10000000>;

        cpu0: cpu@0 {
            device_type = "cpu";
            reg = <0>;
            status = "okay";
            compatible = "riscv";
            riscv,isa = "rv64imacsu";
            mmu-type = "riscv,sv39";

            cpu0_intc: interrupt-controller {
                #interrupt-cells = <1>;
                interrupt-controller;
                compatible = "riscv,cpu-intc";
            };
        };
    };

    soc {
        #address-cells = <2>;
        #size-cells = <2>;
        compatible = "simple-bus";
        ranges;

        clint@2000000 {
            compatible = "riscv,clint0";
            reg = <0x0 0x2000000 0x0 0x10000>;
            interrupts-extended = <&cpu0_intc 3>, <&cpu0_intc 7>;
        };

        plic@c000000 {
            compatible = "sifive,plic-1.0.0", "riscv,plic0";
            reg = <0x0 0xc000000 0x0 0x4000000>;
            #interrupt-cells = <1>;
            interrupt-controller;
            interrupts-extended = <&cpu0_intc 11>, <&cpu0_intc 9>;
            riscv,ndev = <31>;
        };

        serial@10000000 {
            compatible = "ns16550a";
            reg = <0x0 0x10000000 0x0 0x100>;
            clock-frequency = <3686400>;
            interrupt-parent = <&plic>;
            interrupts = <10>;
        };
    };
};
```

---

## Trade-offs

- T-1: **Boot ROM vs direct register setup** — Kept Boot ROM (per TR-2). Now backed by explicit `BootConfig` enum that makes boot selection a first-class runtime contract. Legacy mode is `BootConfig::Direct` with zero code path changes.

- T-2: **Offline DTS → DTB vs runtime FDT** — Kept offline `dtc` (per C-1). DTS contract now fully specified with all compatible strings, properties, and reserved-memory.

- T-3: **fw_jump** — Kept. fw_jump is simplest for separate firmware+kernel loading. fw_dynamic deferred until needed.

---

## Validation

[**Unit Tests**]

- V-UT-1: `BootRom` read returns correct trampoline bytes; write returns `BadAddress`
- V-UT-2: `misa` CSR value = `0x8000_0000_0014_1101` after `CsrFile::new()`
- V-UT-3: `mhartid` returns 0
- V-UT-4: `BootRom::new()` trampoline: decoding instructions yields `a0=0`, `a1=FDT_ADDR`, jump to entry

[**Integration Tests**]

- V-IT-1: OpenSBI `fw_jump.bin` prints banner to UART (manual verification with `make boot`)
- V-IT-2: `make test` passes (all 269 unit tests)
- V-IT-3: am-tests `make run` passes (7 tests)
- V-IT-4: cpu-tests-rs pass

[**Failure / Robustness Validation**]

- V-F-1: `CPU::boot(Firmware { fw: "nonexistent", .. })` returns error
- V-F-2: Legacy `make run FILE=...` works exactly as before

[**Edge Case Validation**]

- V-E-1: `BootConfig::Direct` does not register boot ROM — bus has no device at 0x1000
- V-E-2: Boot ROM read at offset > data length returns 0 (no out-of-bounds)
- V-E-3: FDT blob fits within DRAM (0x87F0_0000 + dtb_size < 0x8800_0000)

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (OpenSBI console) | V-IT-1 |
| G-2 (reproducible build) | resource/Makefile builds from source |
| G-3 (misa correct) | V-UT-2 |
| G-4 (legacy unchanged) | V-IT-2, V-IT-3, V-IT-4, V-F-2 |
| C-1 (no new deps) | Code review |
| C-4 (external Makefile) | M-001 compliance |
| C-6 (no regression) | V-IT-2, V-IT-3, V-IT-4 |
