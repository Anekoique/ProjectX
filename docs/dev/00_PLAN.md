# Device Emulation PLAN 00

> Status: Draft
> Feature: `dev`
> Iteration: `00`
> Owner: Executor
> Depends on:
> - Previous Plan: none
> - Review: none
> - Master Directive: none

---

## Summary

Implement Phase 4 device emulation: CLINT, PLIC, UART 16550, and SiFive Test Finisher.
QEMU virt / SiFive-compatible memory map. `Arc<AtomicU64>` lock-free interrupt delivery
between devices and CPU. Reference designs: KXemu, Nemu-rust, REMU.

## Log

None (initial plan).

---

## Spec

[**Goals**]

- G-1: CLINT with mtime (host 10MHz wall clock), mtimecmp, msip — drives MTIP/MSIP
- G-2: PLIC with 32 sources, 2 contexts (M/S), priority/pending/enable/threshold/claim/complete — drives MEIP/SEIP
- G-3: UART 16550 with TX (stdout) and RX (stdin) — PLIC source 10
- G-4: SiFive Test Finisher for bare-metal test exit signaling
- G-5: Lock-free interrupt delivery via `Arc<AtomicU64>` shared between CPU and devices

- NG-1: OpenSBI / Device Tree / SBI handoff (future phase)
- NG-2: Multi-hart support (single hart only)
- NG-3: DMA or scatter-gather I/O

[**Architecture**]

```
                          ┌────────────────────────────────┐
  RVCore                  │  Bus (Arc<Mutex<Bus>>)         │
  ├── csr, mmu, pmp      │  ├── Ram   [0x8000_0000, 128M] │
  ├── ext_ip ─────poll──► │  ├── CLINT [0x0200_0000, 64K]  │
  │   Arc<AtomicU64>      │  ├── PLIC  [0x0C00_0000, 64M]  │
  │   ▲  ▲  ▲            │  ├── UART0 [0x1000_0000, 8B]   │
  │   │  │  └─ PLIC ─────┤  └── Test  [0x0010_0000, 16B]  │
  │   │  └─── CLINT ─────┤                                │
  │   └────── (future)    └────────────────────────────────┘
  └── step()
       └── merge ext_ip → mip
       └── check_pending_interrupts()
       └── fetch → decode → execute
       └── retire()
```

Interrupt delivery: devices and CPU share `Arc<AtomicU64>` (`ext_ip`).
Each bit corresponds to a mip bit position:

| Bit | Interrupt | Source |
|-----|-----------|--------|
| 3   | MSIP      | CLINT msip register |
| 7   | MTIP      | CLINT mtime >= mtimecmp |
| 9   | SEIP      | PLIC (S-mode context has pending) |
| 11  | MEIP      | PLIC (M-mode context has pending) |

Write path: devices call `ext_ip.fetch_or(bit, Relaxed)` / `fetch_and(!bit, Relaxed)`.
Read path: CPU merges ext_ip into mip at start of each `step()`.

Device-to-PLIC signaling: `Arc<AtomicU32>` (`DeviceIrq`) shared between
peripheral devices and PLIC. Devices set `1 << source_id`, PLIC reads in `tick()`.

[**Invariants**]

- I-1: Hardware-wired mip bits (MTIP, MEIP, SEIP, MSIP) are only modified via ext_ip merge, never by CSR write instructions
- I-2: `Bus::tick()` runs before `sync_external_interrupts()` which runs before `check_pending_interrupts()` — strict ordering per step
- I-3: PLIC claim atomically clears pending bit and stores claimed source; complete clears claimed state
- I-4: Device `read`/`write` only access offset-relative addresses; devices have no knowledge of their base address

[**Data Structure**]

```rust
// device/mod.rs
pub type ExtIp = Arc<AtomicU64>;
pub type DeviceIrq = Arc<AtomicU32>;

pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
}

// device/clint.rs
pub struct Clint {
    boot_instant: Instant,
    msip: u32,
    mtimecmp: u64,
    ext_ip: ExtIp,
}

// device/plic.rs
pub struct Plic {
    priority: [u8; 32],
    pending: u32,
    enable: [[u32; 1]; 2],   // ctx 0=M, ctx 1=S
    threshold: [u8; 2],
    claimed: [u32; 2],
    device_pending: DeviceIrq,
    ext_ip: ExtIp,
}

// device/uart.rs
pub struct Uart {
    ier: u8, iir: u8, lcr: u8, mcr: u8,
    dll: u8, dlm: u8, scratch: u8,
    rx_fifo: VecDeque<u8>,
    rx_buf: Arc<Mutex<VecDeque<u8>>>,  // stdin thread writes here
    device_irq: DeviceIrq,
    source_id: u32,
}

// device/test_finisher.rs
pub struct TestFinisher;
```

[**API Surface**]

```rust
// Clint
impl Clint {
    pub fn new(ext_ip: ExtIp) -> Self;
    fn mtime(&self) -> u64;        // host nanos / 100 = 10 MHz
    fn check_timer(&mut self);     // update MTIP in ext_ip
}

// Plic
impl Plic {
    pub fn new(device_pending: DeviceIrq, ext_ip: ExtIp) -> Self;
    fn claim(&mut self, ctx: usize) -> u32;
    fn complete(&mut self, ctx: usize, source: u32);
    fn update_ext_ip(&mut self);   // re-evaluate MEIP/SEIP
}

// Uart
impl Uart {
    pub fn new(device_irq: DeviceIrq, source_id: u32) -> Self;
}

// Bus addition
impl Bus {
    pub fn tick(&mut self);
}

// RVCore addition
impl RVCore {
    fn sync_external_interrupts(&mut self);
}
```

[**Constraints**]

- C-1: All MMIO addresses follow QEMU virt machine layout for future DT/OpenSBI compatibility
- C-2: Single hart only — no per-hart indexing in CLINT/PLIC
- C-3: UART byte-access only (size != 1 returns error for register ops)
- C-4: Device::read changed from `&self` to `&mut self` (UART needs to pop rx_fifo)

---

## Implement

### Execution Flow

[**Main Flow**]

1. `bus.tick()` — each device updates internal state (CLINT checks timer, PLIC merges device_pending, UART drains rx_buf)
2. `sync_external_interrupts()` — load ext_ip, merge hw bits into mip
3. `check_pending_interrupts()` — sample mip & mie, raise trap if enabled
4. If interrupt taken: `retire()` and return
5. Otherwise: `fetch → decode → execute → retire`

[**Failure Flow**]

1. TestFinisher write → `Err(XError::ProgramExit(code))` — CPU catches and halts
2. MMIO access to unmapped region → `Err(XError::BadAddress)` → trap
3. UART byte-access violation → `Err(XError::BadAddress)`

[**State Transition**]

- ext_ip bit set by device → mip bit set by sync → interrupt taken by check_pending_interrupts → trap handler
- PLIC: device_pending bit → pending bit (tick) → claim (read) → complete (write) → re-evaluate

### Implementation Plan

[**Step 0: Interrupt Plumbing**]

- `device/mod.rs`: Add `ExtIp`, `DeviceIrq` type aliases, `Device::tick()`, change `read(&self)` to `read(&mut self)`
- `device/bus.rs`: Add `Bus::tick()`
- `cpu/riscv/mod.rs`: Add `ext_ip: ExtIp` to RVCore, add `sync_external_interrupts()`
- `error.rs`: Add `XError::ProgramExit(u32)`

[**Step 1: CLINT**]

- `device/clint.rs`: mtime (host 10MHz), mtimecmp, msip
- Register map: msip @0x0000, mtimecmp @0x4000, mtime @0xBFF8
- `tick()` checks mtime >= mtimecmp, updates MTIP in ext_ip
- Wire into Bus

[**Step 2: PLIC**]

- `device/plic.rs`: 32 sources, 2 contexts
- Register map: priority @0x0, pending @0x1000, enable @0x2000, threshold @0x200000, claim @0x200004
- `tick()` reads device_pending atomic, merges into pending, evaluates MEIP/SEIP
- Claim/complete logic

[**Step 3: UART 16550**]

- `device/uart.rs`: 8 registers with DLAB switching
- TX: THR write → stdout
- RX: background stdin thread → rx_buf → tick() drains to rx_fifo
- PLIC interrupt via device_irq when rx_fifo non-empty and IER.rx enabled

[**Step 4: SiFive Test Finisher**]

- `device/test_finisher.rs`: write 0x5555 → ProgramExit(0), write (code<<16)|0x3333 → ProgramExit(code)

[**Step 5: Integration**]

- Wire all devices into Bus at CPU construction
- End-to-end: CLINT timer → MTIP → trap; UART RX → PLIC → MEIP → trap; Test Finisher → halt

## Trade-offs

- T-1: **Interrupt delivery** — `Arc<AtomicU64>` (lock-free, multi-core ready) vs direct mip write (simpler). Chose atomic for decoupling and future multi-core.
- T-2: **CLINT mtime source** — Host wall clock (realistic, non-deterministic) vs instruction count (deterministic, testable). Chose host clock for realism; tests use short timeouts.
- T-3: **UART RX backend** — Direct stdin (simple, conflicts with xdb) vs TCP/PTY (complex, no conflict). Chose stdin with background thread; need to address xdb conflict.
- T-4: **Device-to-PLIC signaling** — `Arc<AtomicU32>` (decoupled) vs PLIC polls devices directly (tighter coupling). Chose atomic for consistency with ext_ip pattern.

## Validation

[**Unit Tests**]

- V-UT-1: CLINT mtime reads increase over time
- V-UT-2: CLINT mtimecmp write → MTIP set/clear in ext_ip
- V-UT-3: CLINT msip write → MSIP set/clear in ext_ip
- V-UT-4: PLIC priority/enable register read/write
- V-UT-5: PLIC claim returns highest-priority pending source
- V-UT-6: PLIC complete clears claimed, re-evaluates MEIP/SEIP
- V-UT-7: PLIC threshold filters low-priority sources
- V-UT-8: UART THR write (verify side effect)
- V-UT-9: UART LSR reports DR when rx_fifo has data
- V-UT-10: UART DLAB switching between data/divisor registers
- V-UT-11: TestFinisher write 0x5555 → ProgramExit(0)
- V-UT-12: TestFinisher write (1<<16)|0x3333 → ProgramExit(1)

[**Integration Tests**]

- V-IT-1: CLINT timer → MTIP → mip → check_pending_interrupts → trap fires
- V-IT-2: UART RX → PLIC → MEIP → mip → trap fires
- V-IT-3: TestFinisher → CPU halts with correct exit code

[**Failure / Robustness Validation**]

- V-F-1: Access to unmapped CLINT hart slots returns 0
- V-F-2: PLIC claim with nothing pending returns 0
- V-F-3: UART non-byte access returns error

[**Edge Case Validation**]

- V-E-1: CLINT mtimecmp set to u64::MAX — timer never fires
- V-E-2: PLIC complete with wrong source ID — no state change
- V-E-3: UART read at all 8 offsets in both DLAB modes

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (CLINT) | V-UT-1..3, V-IT-1, V-E-1 |
| G-2 (PLIC) | V-UT-4..7, V-IT-2, V-E-2 |
| G-3 (UART) | V-UT-8..10, V-IT-2, V-E-3 |
| G-4 (TestFinisher) | V-UT-11..12, V-IT-3 |
| G-5 (ext_ip) | V-IT-1, V-IT-2 |
| C-1 (QEMU virt layout) | Address constants match spec |
| C-3 (UART byte-only) | V-F-3 |

---

## File Organization

```
xcore/src/device/
├── mod.rs            — Device trait, ExtIp, DeviceIrq
├── bus.rs            — Bus (+ tick())
├── ram.rs            — Ram
├── clint.rs          — CLINT (new)
├── plic.rs           — PLIC (new)
├── uart.rs           — UART 16550 (new)
└── test_finisher.rs  — SiFive Test Finisher (new)
```

## Memory Map

| Device | Base | Size | PLIC IRQ |
|--------|------|------|----------|
| SiFive Test Finisher | `0x0010_0000` | 16 B | — |
| CLINT | `0x0200_0000` | `0x1_0000` (64 KB) | — |
| PLIC | `0x0C00_0000` | `0x0400_0000` (64 MB) | — |
| UART0 | `0x1000_0000` | 8 B | source 10 |
| RAM | `0x8000_0000` | 128 MB | — |

## Open Design Points

1. UART stdin raw mode: need termios for non-canonical byte-at-a-time input
2. PLIC re-evaluation after complete: re-scan or defer to next tick
3. Bus construction: explicit `add_mmio` calls in RVCore::new() or factory
4. Device::read mutability: change to `&mut self` (all callers hold `&mut Bus`)
