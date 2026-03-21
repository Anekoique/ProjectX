# xemu Development Plan

## Current Status (2026-03-16)

xemu is an early-stage RISC-V emulator (~2,400 lines) in a multi-crate Rust workspace (xcore, xdb, xlogger). It supports RV32/RV64 user-mode instruction execution in interactive and batch modes.

### What Works

- **ISA**: RV32I/RV64I base, M extension (mul/div), C extension (compressed)
- **Memory**: Flat 128 MB physical memory, aligned access, bounds checking
- **Decoding**: pest-based pattern matcher, 100+ instruction patterns
- **Debugger (xdb)**: step, continue, load, reset
- **Logging**: Colored, timestamped, configurable log levels
- **Tests**: 31 cpu-tests-rs (Rust) passing, unit tests in xcore

---

## Comparison with Reference Implementations

### vs Nemu-rust (~5,600 lines, RV64)

| Feature | xemu | Nemu-rust |
|---------|------|-----------|
| ISA width | RV32/RV64 (cfg) | RV64 only |
| Compressed (C ext) | Yes | No |
| CSR registers | Stub (unimplemented) | Full (mstatus, mtvec, satp, etc.) |
| Privilege modes | None (M-mode only) | M/S/U with delegation |
| MMU / Virtual memory | Identity mapping | SV39 + TLB (2048 entries) |
| Interrupts/Exceptions | None | Full (PLIC, timer, ecall trap) |
| Devices | None | UART16550, VGA, Timer, RTC, Keyboard, PLIC, CLINT |
| Difftest | None | QEMU via GDB protocol |
| Debugger | step/continue/load/reset | + breakpoints, watchpoints, expression eval, backtrace |
| Instruction cache | None | Set-associative IBuf (16K entries) |
| Disassembly | None | LLVM-based disassembler |
| Performance profiling | None | Per-instruction counters |

**Key strength of xemu**: Dual RV32/RV64 via `cfg`, compressed instruction support.
**Key gaps**: No CSR, no privilege modes, no devices, no MMU, no debugging beyond step.

### vs remu (~3,800 lines, RV32)

| Feature | xemu | remu |
|---------|------|------|
| ISA width | RV32/RV64 | RV32 only |
| Atomic (A ext) | None | LR/SC, AMO operations |
| CSR registers | Stub | Full 4096-entry array |
| Privilege modes | None | M/S/U with delegation |
| MMU / Virtual memory | Identity mapping | SV32 page tables |
| Interrupts/Exceptions | None | CLINT + PLIC, M/S-mode traps |
| Devices | None | CLINT, PLIC, UART, Timer, VGA, Keyboard, Audio, Disk |
| Tracing | Log only | 7 ring-buffered traces (itrace, mtrace, ftrace, dtrace, etc.) |
| Configuration | Makefile env vars | Kconfig system (.config -> config.rs) |
| Linux boot | No | Boots OpenSBI + Linux 5.15 |

**Key strength of xemu**: Cleaner architecture (traits, generics), RV64 support, compressed insts.
**Key gaps**: Cannot run any OS — missing CSR, privilege, interrupts, devices.

### Architectural Advantages of xemu

1. **Generic ISA trait** (`CPU<Core: CoreOps + MemOps>`) — pluggable ISA backends (LoongArch stub exists)
2. **Workspace crate separation** — xcore (engine), xdb (debugger), xlogger (logging) are independently testable
3. **Pest-based decoder** — declarative instruction patterns, easier to extend
4. **Dual 32/64 via cfg_if** — single codebase for both widths

---

## Development Roadmap

### Phase 1: Foundation (Current → Near-term)

**Goal**: Complete user-mode emulation, pass all cpu-tests.

- [x] RV32I/RV64I base instructions
- [x] M extension (mul/div)
- [x] C extension (compressed)
- [x] Batch mode execution
- [x] 31 cpu-tests-rs passing
- [ ] **C cpu-tests support** — implement `klib` (bare-metal printf/string) or adapt alu-test generator
- [ ] **A extension** — LR/SC, AMO instructions (needed for multi-threaded workloads)

### Phase 2: System Infrastructure

**Goal**: Support privileged execution, lay groundwork for OS.

- [ ] **CSR subsystem** — implement mstatus, mtvec, mepc, mcause, mie, mip, satp, etc.
- [ ] **Privilege modes** — M/S/U mode transitions, trap delegation (medeleg/mideleg)
- [ ] **Exception handling** — ecall, illegal instruction, page fault, breakpoint traps
- [ ] **Interrupt framework** — timer interrupt, external interrupt routing

### Phase 3: Memory Management

**Goal**: Virtual memory and address translation.

- [ ] **SV39 page tables** (RV64) / **SV32** (RV32) — multi-level page walk
- [ ] **TLB** — translation lookaside buffer with flush on satp write
- [ ] **Permission checks** — R/W/X/U bits, page fault generation
- [ ] **MMIO routing** — address-range based dispatch to devices vs physical memory

### Phase 4: Device Emulation

**Goal**: Minimal device set for console I/O and timer.

- [ ] **UART** — serial output (printf support for running programs)
- [ ] **Timer/RTC** — mtime/mtimecmp for OS scheduler
- [ ] **CLINT** — core-local interruptor (software + timer interrupts)
- [ ] **PLIC** — platform-level interrupt controller (external interrupt routing)

### Phase 5: Debugging & Observability

**Goal**: Match Nemu-rust/remu debugging capabilities.

- [ ] **Breakpoints** — address-based pause
- [ ] **Watchpoints** — expression-based pause on value change
- [ ] **Expression evaluator** — arithmetic, register refs, memory deref in debugger
- [ ] **Instruction trace** — ring-buffered itrace for post-mortem analysis
- [ ] **Memory trace** — mtrace for debugging memory issues
- [ ] **Function trace** — ftrace with ELF symbol resolution
- [ ] **Disassembly** — inline disasm of current instruction

### Phase 6: Validation & Performance

**Goal**: Correctness verification and optimization.

- [ ] **Difftest** — compare execution with QEMU/Spike via GDB protocol
- [ ] **Instruction cache** — decoded instruction buffer to skip re-decoding hot paths
- [ ] **Performance counters** — per-instruction statistics, IPC tracking
- [ ] **alu-tests** — comprehensive arithmetic/logic test coverage

### Phase 7: OS Boot (Long-term)

**Goal**: Boot a real operating system.

- [ ] **OpenSBI** — boot SBI firmware in M-mode
- [ ] **Linux kernel** — boot minimal Linux (requires phases 2-4 complete)
- [ ] **VGA framebuffer** — graphical output
- [ ] **Keyboard** — input device for interactive programs
- [ ] **Disk** — block device for filesystem support

---

## Priority Order

The phases above are roughly ordered by dependency, but the practical priority is:

1. **Phase 1** — finish cpu-test coverage (alu-tests, A extension)
2. **Phase 2 + Phase 3** — CSR + privilege + MMU (these are tightly coupled)
3. **Phase 4** — devices (UART first, then timer/CLINT/PLIC)
4. **Phase 5** — debugging (can be developed in parallel with phases 2-4)
5. **Phase 6** — difftest (critical for catching bugs during phase 2-4 development)
6. **Phase 7** — OS boot (the culmination of all previous work)

---

## Design Principles

- **Incremental correctness**: Each phase should be testable in isolation. Add tests before features.
- **cfg-based ISA flexibility**: Maintain RV32/RV64 dual support. New features must work for both widths.
- **Trait-based extensibility**: Device bus, MMU, and ISA all use trait abstraction for pluggability.
- **Minimal dependencies**: Avoid heavy frameworks. The emulator core should remain lean.
- **Immutability where possible**: Prefer returning new values over mutating shared state. Use `with_xcpu!` / `with_mem!` patterns to scope mutation.
