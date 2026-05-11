# `difftest` IMPL `07` (FINAL)

> Feature: `difftest`
> Iteration: `07`
> Owner: Executor
> Approved Plan: `07_PLAN.md`

---

## Completed Scope

Phase 6: Difftest framework for xemu — per-instruction state comparison against QEMU and Spike reference emulators.

### xcore Infrastructure

- **`csr_table!` macro extension** — `@ difftest` / `@ difftest(mask)` annotation per CSR entry. Auto-generates `DIFFTEST_CSRS: &[(CsrAddr, u64)]` and `CsrAddr::name()`. Default mask = `u64::MAX`; only `mip` uses `!0x82_u64`, `mstatus` uses `!0xF_0000_0000_u64` (excludes SXL/UXL).
- **`CoreContext`** (`cpu/riscv/context.rs`) — lightweight, `Clone` snapshot: PC, named GPRs `Vec<(&str, u64)>`, privilege, CSR tuples `(addr, name, mask, raw_value)`, word_size, ISA string. Arch-dispatched as `RVCoreContext as CoreContext`. Always compiled (no cfg gate).
- **`DebugOps::context()`** — always compiled. Builds `CoreContext` from RVCore state using `DIFFTEST_CSRS`.
- **`DebugOps::read_register()`** — descriptor-aware via `find_desc()` + `read_with_desc()`. Correctly handles shadow CSRs (sstatus, sie, sip).
- **`dump_registers()`** — removed. Replaced by `context().gprs` in xdb.
- **MMIO flag** — `AtomicBool` on `Bus`, `cfg(feature = "difftest")` gated (only hot-path hook).
- **`RESET_VECTOR`** — made `pub`, re-exported from xcore.
- **`State` enum** — renamed to PascalCase (`Idle`, `Halted`, `Abort`).
- **`CPU::load_default_image()`** — extracted to deduplicate `reset()` and `load(None)`.

### xdb Difftest Module

- **`DiffBackend` trait** — `step()`, `read_context()`, `sync_state()`, `write_mem()`, `name()`.
- **`QemuBackend`** — GDB RSP over TCP. Spawns QEMU with `-M virt -s -S -bios`. Configures `sstep=0x7` (NOIRQ+NOTIMER) and `PhyMemMode:1`. Syncs DUT initial state to QEMU. RAII kills QEMU on drop.
- **`SpikeBackend`** — FFI via C++ wrapper (`tools/difftest/spike/`). Links `libriscv`, `libsoftfloat`, `libfesvr`, `libdisasm`. Syncs DUT initial state. Built automatically when Spike is installed (`SPIKE_DIR` env).
- **`GdbClient`** — full GDB RSP: packet framing, pushback byte for protocol alignment, checksum validation with proper error propagation.
- **`diff_contexts()`** — free function. Checks PC → GPR[1..31] → privilege → CSRs (masked by addr, symmetric). Missing CSR = mismatch.
- **`DiffHarness`** — wraps backend. `check_step()` runs BEFORE halt break. `run_difftest()` helper deduplicates step/continue paths.
- **CLI** — `dt attach qemu|spike`, `dt detach`, `dt status`. Continue fast path only when no hooks.
- **`info reg`** — uses `context().gprs` for full dump; `read_register()` for named lookup.

### Build System

- `DIFFTEST=1 make run` — enables both QEMU and Spike backends
- `cfg(feature = "difftest")` only on Bus MMIO flag (hot path)
- Spike wrapper at `tools/difftest/spike/`, built by `xdb/build.rs` via `cc` crate
- `SPIKE_DIR` defaults to `/opt/homebrew`

## Deviations from Plan

| Deviation | Reason |
|-----------|--------|
| `DIFF_CSR_SET` eliminated | Integrated into `csr_table!` macro via `@ difftest` annotation |
| `dump_registers()` removed | Replaced by `context().gprs` |
| Spike implemented (not stub) | Went beyond plan — full FFI with C++ wrapper |
| `sstep=0x7` (not `0x1`) | QEMU virt machine timer fires during step with `0x1`, causing false divergence |
| `mstatus` mask `!0xF_0000_0000` | Spike sets SXL/UXL bits that xemu doesn't |
| `State` → PascalCase | Code review finding — consistency with Rust conventions |
| `RESET_VECTOR` made public | Eliminates magic number in `cmd_dt_attach` |

## Verification Results

| Test | Result |
|------|--------|
| xcore unit tests (269) | PASS |
| xdb unit tests without difftest (6) | PASS |
| xdb unit tests with difftest (19) | PASS |
| clippy (without features) | PASS — zero warnings |
| clippy (with difftest) | PASS — zero warnings |
| fmt | PASS |
| **Difftest QEMU: dummy** | PASS — 16 instructions |
| **Difftest QEMU: add** | PASS — 16 instructions |
| **Difftest QEMU: fib** | PASS — 531 instructions |
| **Difftest QEMU: recursion** | PASS — 3,356 instructions |
| **Difftest QEMU: select-sort** | PASS — 5,386 instructions |
| **Difftest QEMU: goldbach** | PASS — 655 instructions |
| **Difftest Spike: dummy** | PASS — 16 instructions |
| **Difftest Spike: add** | PASS — 16 instructions |
| **Difftest Spike: fib** | PASS — 531 instructions |
| **Difftest Spike: recursion** | PASS — 3,356 instructions |
| **Difftest Spike: select-sort** | PASS — 5,386 instructions |

## Files Changed

**xcore:**
- `cpu/riscv/csr.rs` — `csr_table!` macro: +`@ difftest` annotation, +`DIFFTEST_CSRS`, +`CsrAddr::name()`
- `cpu/riscv/context.rs` — NEW: `RVCoreContext`
- `cpu/riscv/debug.rs` — +`context()`, fix `read_register()` (descriptor-aware), fix U-type disasm, -`dump_registers()`
- `cpu/riscv/mod.rs` — +`pub mod context`, +`CoreContext` dispatch
- `cpu/debug.rs` — +`context()` in DebugOps, -`dump_registers()`
- `cpu/mod.rs` — +`CoreContext` pass-through, +`load_default_image()`, +`pub RESET_VECTOR`, `State` PascalCase, +difftest `bus_take_mmio_flag()`
- `device/bus.rs` — +`AtomicBool mmio_accessed` (cfg difftest)
- `lib.rs` — +re-export `CoreContext`, `RVReg`, `RESET_VECTOR`
- `Cargo.toml` — +`difftest = ["debug"]`

**xdb:**
- `difftest/mod.rs` — NEW: `DiffBackend`, `DiffHarness`, `diff_contexts()`, 7 tests
- `difftest/gdb.rs` — NEW: `GdbClient`, 4 tests
- `difftest/qemu.rs` — NEW: `QemuBackend`
- `difftest/spike.rs` — NEW: `SpikeBackend` (FFI)
- `build.rs` — NEW: Spike cc build
- `cmd.rs` — +`run_difftest()` helper, +dt commands, `info reg` via context, +`parse_addr` fix
- `cli.rs` — +`Dt` subcommand, +WatchDelete feedback, regex fix
- `main.rs` — +difftest wiring
- `Cargo.toml` — +`difftest`, +`cc` build-dep

**Tools:**
- `tools/difftest/spike/spike_wrapper.cc` — NEW: C++ Spike FFI wrapper
- `tools/difftest/spike/spike_ffi.h` — NEW: C header

**Build:**
- `Makefile` — +`DIFFTEST=1`, fix `$(verbose)`
