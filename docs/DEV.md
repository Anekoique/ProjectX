# xemu Development Plan

## Current Status (2026-03-30)

xemu is a RISC-V emulator in a multi-crate Rust workspace (xcore, xdb, xlogger) with a companion bare-metal C library (xlib). It supports RV32/RV64 with full privileged execution (M/S/U modes), trap handling, interrupt routing, virtual memory, and device emulation.

### What Works

- **ISA**: RV32I/RV64I base, M (mul/div), A (atomics: LR/SC + 9 AMO ops), C (compressed), Zicsr
- **CSR subsystem**: mstatus/sstatus (WARL), mtvec/stvec (direct + vectored), mepc/sepc, mcause/scause, medeleg/mideleg, mcounteren/scounteren, shadow registers (sie→mie, sip→mip, sstatus→mstatus), satp with MMU side effects, pmpcfg/pmpaddr with lock semantics
- **Privilege modes**: M/S/U transitions, trap delegation, mret/sret with MPRV handling
- **Trap handling**: Exception dispatch (ecall per mode, illegal instruction, breakpoint, page faults), interrupt priority/masking (MIE/SIE gating, global enable, delegation), vectored mode
- **Memory subsystem**: Device trait + Bus (Ram + MMIO routing), MMU (SV32/SV39 page walk, Svade), TLB (64-entry direct-mapped, ASID-tagged), PMP (16 entries, TOR/NA4/NAPOT, lock semantics), sfence.vma
- **Device emulation**: ACLINT (MSWI + MTIMER 10MHz + SSWI), PLIC (32 sources, 2 contexts, level-triggered), UART 16550 (TX + PTY-based RX via `Uart::with_pty()`), `IrqState` lock-free interrupt delivery
- **Decoding**: pest-based pattern matcher, 130 instruction patterns
- **xlib (klib)**: Freestanding C library — printf/sprintf (format.c), puts/putch (stdio.c), memset/memcpy/strlen/strcmp/strcat/strchr (string.c)
- **Debugger (xdb)**: breakpoints (stable IDs), watchpoints (expression-based), expression evaluator (`$reg`, `*addr`, arithmetic), disassembly (`x/Ni`), memory examine (`x/Nx`), register inspect (`info reg`), GDB-style `x/Nf` pre-parser, difftest (`dt attach qemu|spike`)
- **Difftest**: Per-instruction DUT/REF comparison against QEMU (GDB RSP) and Spike (FFI). Compares PC + GPR + privilege + 14 whitelisted CSRs (masked). MMIO-skip with raw-value sync. `csr_table!` macro `@ difftest` annotation auto-generates whitelist. Feature-gated (`DIFFTEST=1`)
- **Logging**: Colored, timestamped, configurable levels. Per-instruction trace (`LOG=trace`), device/CSR debug (`LOG=debug`), lifecycle info (`LOG=info`). Comprehensive coverage across trap handler, memory access, CSR side effects, PLIC, ACLINT, UART, Bus.
- **Tests**: 269 unit tests passing, 31 cpu-tests-rs, 7 am-tests (bare-metal: UART, ACLINT, PLIC, CSR, trap, interrupts), keyboard test (interactive PTY echo), alu-tests (22k+ arithmetic checks), rtc clock test
- **Benchmarks**: coremark (1000 iterations), dhrystone (500k runs), microbench (10 sub-benchmarks including C++)
- **CI**: GitHub Actions pipeline (fmt, clippy, unit tests, cpu-tests-rs, cpu-tests-c, am-tests, alu-tests, benchmarks)
- **xam HAL**: `_putch` (UART console), `mtime`/`set_mtimecmp` (ACLINT timer), `uptime()` (microseconds), `init_trap`/`TrapFrame` (trap entry), `mainargs` (compile-time argument passing), `_heap_start`/`_heap_end` (linker symbols)
- **xlib**: printf/sprintf, string ops, `assert.h` (C/C++-safe), `extern "C"` guards for C++ compatibility

---

## Comparison with Reference Implementations

### vs Nemu-rust (~5,600 lines, RV64)

| Feature | xemu | Nemu-rust |
|---------|------|-----------|
| ISA width | RV32/RV64 (cfg) | RV64 only |
| Compressed (C ext) | Yes | No |
| Atomic (A ext) | Full (LR/SC + AMO) | No |
| CSR registers | Full WARL + shadows | Full (mstatus, mtvec, satp, etc.) |
| Privilege modes | M/S/U with delegation | M/S/U with delegation |
| MMU / Virtual memory | SV32/SV39, TLB (64), PMP (16) | SV39 + TLB (2048 entries) |
| Interrupts/Exceptions | Full (priority, delegation, vectored) | Full (PLIC, timer, ecall trap) |
| Devices | ACLINT, PLIC, UART 16550 (PTY RX) | UART16550, VGA, Timer, RTC, Keyboard, PLIC, CLINT |
| Difftest | QEMU (GDB RSP) + Spike (FFI) | QEMU via GDB protocol |
| Debugger | step/continue/load/reset | + breakpoints, watchpoints, expression eval, backtrace |
| Instruction cache | None | Set-associative IBuf (16K entries) |
| Disassembly | None | LLVM-based disassembler |
| Performance profiling | None | Per-instruction counters |

**Key strength of xemu**: Dual RV32/RV64 via `cfg`, compressed + atomic extensions, clean WARL CSR model, dual-backend difftest.
**Key gaps**: No VGA, no disk.

### vs remu (~3,800 lines, RV32)

| Feature | xemu | remu |
|---------|------|------|
| ISA width | RV32/RV64 | RV32 only |
| Atomic (A ext) | Full (LR/SC + AMO .w/.d) | LR/SC, AMO operations |
| CSR registers | Full WARL + shadows | Full 4096-entry array |
| Privilege modes | M/S/U with delegation | M/S/U with delegation |
| MMU / Virtual memory | SV32/SV39, TLB, PMP | SV32 page tables |
| Interrupts/Exceptions | Full trap framework | CLINT + PLIC, M/S-mode traps |
| Devices | ACLINT, PLIC, UART 16550 | CLINT, PLIC, UART, Timer, VGA, Keyboard, Audio, Disk |
| Tracing | Log only | 7 ring-buffered traces (itrace, mtrace, ftrace, dtrace, etc.) |
| Configuration | Makefile env vars | Kconfig system (.config -> config.rs) |
| Linux boot | No | Boots OpenSBI + Linux 5.15 |

**Key strength of xemu**: Cleaner architecture (traits, generics), RV64 support, compressed + atomic insts, full trap delegation, dual-backend difftest.
**Key gaps**: No VGA, audio, disk. Fewer device types overall.

### Architectural Advantages of xemu

1. **Generic ISA trait** (`CPU<Core: CoreOps + MemOps>`) — pluggable ISA backends (LoongArch stub exists)
2. **Workspace crate separation** — xcore (engine), xdb (debugger), xlogger (logging) are independently testable
3. **Pest-based decoder** — declarative instruction patterns, easier to extend
4. **Dual 32/64 via cfg_if** — single codebase for both widths
5. **WARL CSR model** — write-mask + shadow register architecture matches spec precisely

---

## Development Roadmap

### Phase 1: Foundation — COMPLETE

**Goal**: Complete user-mode emulation, pass all cpu-tests.

- [x] RV32I/RV64I base instructions
- [x] M extension (mul/div)
- [x] C extension (compressed)
- [x] Batch mode execution
- [x] 31 cpu-tests-rs passing
- [x] **A extension** — LR/SC, AMO instructions (22 ops, .w + .d variants)
- [x] **xlib (klib)** — bare-metal C library (printf, string, stdio)

### Phase 2: System Infrastructure — COMPLETE

**Goal**: Support privileged execution, lay groundwork for OS.

- [x] **CSR subsystem** — mstatus, mtvec, mepc, mcause, mie, mip, satp, medeleg/mideleg, counteren, WARL masks, shadow registers
- [x] **Privilege modes** — M/S/U mode transitions, trap delegation
- [x] **Exception handling** — ecall (per mode), illegal instruction, breakpoint, page fault causes defined
- [x] **Interrupt framework** — priority-based interrupt selection, MIE/SIE gating, global enable, delegation routing, vectored mtvec/stvec

### Phase 3: Memory Management — COMPLETE

**Goal**: Virtual memory, address translation, and device bus.

- [x] **Device trait + Bus** — `Device` trait (read/write), `Bus` with Ram + MMIO dispatch, `Arc<Mutex<Bus>>` shared ownership
- [x] **SV39 page tables** (RV64) / **SV32** (RV32) — multi-level page walk with Svade A/D enforcement
- [x] **TLB** — 64-entry direct-mapped, ASID-tagged, global page support, sfence.vma flush
- [x] **PMP** — 16 entries, TOR/NA4/NAPOT matching, partial-overlap detection, lock semantics
- [x] **Permission checks** — R/W/X/U bits, SUM/MXR, MPRV effective privilege, page fault generation
- [x] **CSR side effects** — satp→MMU, mstatus→SUM/MXR, pmpcfg/pmpaddr→PMP with lock writeback

### Phase 4: Device Emulation — COMPLETE

**Goal**: Minimal device set for console I/O, timer, and interrupt routing.

- [x] **ACLINT** — MSWI (msip → MSIP), MTIMER (mtime 10MHz + mtimecmp → MTIP), SSWI (setssip → SSIP)
- [x] **PLIC** — 32 sources, 2 contexts (M/S), level-triggered, claim/complete with claimed-exclusion
- [x] **UART 16550** — TX (stdout), opt-in TCP RX, DLAB register switching, PLIC source 10
- [x] **Integration** — `IrqState` lock-free interrupt delivery, `Bus::tick()` + `set_irq_sink()`, `sync_interrupts()` in step(), device reset

### Phase 5: Debugging & Observability — COMPLETE

**Goal**: Debugger commands and execution observability.

- [x] **Breakpoints** — address-based with stable IDs, `skip_bp_once` for step-after-hit
- [x] **Watchpoints** — expression-based value-change detection, validated at creation
- [x] **Expression evaluator** — `$reg`, `*addr` deref, arithmetic, comparisons, parentheses
- [x] **Disassembly** — `x/Ni addr` using `DebugOps` + `format_mnemonic()` for all instruction formats
- [x] **Memory examine** — `x/Nx addr` (hex words), `x/Nb addr` (bytes)
- [x] **Register inspect** — `info reg [name]` with GPR/CSR/PC name resolution
- [x] **Execution logging** — `trace!()` per instruction, `debug!()` per memory/device/trap, `info!()` lifecycle events. Replaces ring-buffered traces with `log!()` levels via xlogger.

### Phase 6: Difftest — COMPLETE

**Goal**: Correctness verification via reference comparison.

- [x] **Difftest framework** — `DiffBackend` trait, `DiffHarness`, `diff_contexts()` free function. Per-instruction comparison of PC + GPR + privilege + 14 whitelisted CSRs (masked). MMIO-skip with raw-value sync
- [x] **QEMU backend** — GDB RSP client, `sstep=0x7` (NOIRQ+NOTIMER), `PhyMemMode:1`, initial state sync
- [x] **Spike backend** — FFI via C++ wrapper (`tools/difftest/spike/`), links libriscv/libsoftfloat/libfesvr/libdisasm
- [x] **CoreContext** — arch-dispatched snapshot (`RVCoreContext as CoreContext`). `csr_table!` macro `@ difftest` annotation auto-generates whitelist
- [x] **Monitor integration** — `dt attach qemu|spike`, `dt detach`, `dt status`. Hooks in `cmd_step`/`cmd_continue`
- [ ] **CI integration** — run difftest against reference on existing test programs (deferred: requires QEMU/Spike in CI)

### Keyboard (UART Serial Console) — COMPLETE

**Goal**: Guest serial input via separate terminal for interactive post-boot I/O.

- [x] **PTY-backed UART** — `Uart::with_pty()` creates pseudo-terminal pair; master fd for TX/RX, slave for user attachment via `screen`
- [x] **Bus device replacement** — `Bus::replace_device()` + `CPU::replace_device()` for binary-layer UART injection
- [x] **Build system** — `BATCH` replaced with `DEBUG` feature flag; am-tests batch execution clean
- [x] **Keyboard am-test** — interactive echo test (`TEST=k`), polls UART RBR

### Phase 7: OS Boot

**Goal**: Boot a real operating system.

- [ ] **OpenSBI** — boot SBI firmware in M-mode
- [ ] **Linux kernel** — boot minimal Linux (requires phases 2-6 complete)
- [ ] **VGA framebuffer** — graphical output
- [x] **Keyboard** — PTY-based UART serial console (completed as prerequisite)
- [ ] **Disk** — block device for filesystem support

### Phase 8: Performance Optimization (Post-boot)

**Goal**: Optimize emulation speed after correctness is established.

- [ ] **Instruction cache** — decoded instruction buffer to skip re-decoding hot paths
- [ ] **Performance counters** — per-instruction statistics, IPC tracking
- [ ] **Hot-path profiling** — identify and optimize critical execution paths

---

## Priority Order

The critical path to OS boot is:

1. ~~**Phase 3 (MMU)**~~ — COMPLETE
2. ~~**Phase 4 (Devices)**~~ — COMPLETE
3. ~~**Phase 5 (Debugging)**~~ — COMPLETE
4. ~~**Phase 6 (Difftest)**~~ — COMPLETE (framework + QEMU/Spike backends; CI integration deferred)
5. ~~**Keyboard**~~ — COMPLETE (PTY-based UART serial console)
6. **Phase 7 (OS boot)** — the culmination of all previous work
7. **Phase 8 (Performance)** — optimize after correctness is proven

---

## Design Principles

- **Incremental correctness**: Each phase should be testable in isolation. Add tests before features.
- **cfg-based ISA flexibility**: Maintain RV32/RV64 dual support. New features must work for both widths.
- **Trait-based extensibility**: Device bus, MMU, and ISA all use trait abstraction for pluggability.
- **Minimal dependencies**: Avoid heavy frameworks. The emulator core should remain lean.
- **Immutability where possible**: Prefer returning new values over mutating shared state. Use functional chains (`.and_then().map_err()`) to scope mutation.
- **Single-lock access path**: Hold one `MutexGuard` across translate→PMP→bus to avoid double-lock overhead.
