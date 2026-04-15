# `Device Emulation` Final Plan

> Status: Approved for Implementation
> Feature: `dev`
> Iterations: 00–05
> Owner: Executor

---

## Summary

Phase 4 device emulation: ACLINT (MSWI + MTIMER + SSWI), PLIC, UART 16550 (TX default + opt-in TCP RX), SiFive Test Finisher (test-only). xemu internal layout (QEMU-like). `Arc<AtomicU64>` interrupt delivery. Level-triggered device IRQ lines. Bus→PLIC via `plic_idx + Device::notify()`.

**Intentional deltas from QEMU virt:** ACLINT replaces CLINT; TestFinisher is test-only.

**UART RX scope (amended during implementation):** Default machine ships TX-only (`Uart::new()`) for deterministic behavior. TCP RX is available via `Uart::with_tcp(port)` as an opt-in feature. Rationale: a hardwired TCP port silently degrades to TX-only on port conflicts, making "shipped" RX unreliable. TX-only default is always correct.

---

## Architecture

```
  RVCore                    Bus (Arc<Mutex<Bus>>)
  ├── irq_state ──poll──►  ├── Ram    [0x8000_0000, 128M]
  │   Arc<AtomicU64>       ├── ACLINT [0x0200_0000, 0x1_0000]  ──► irq_state
  │   ▲                    ├── PLIC   [0x0C00_0000, 0x400_0000] ──► irq_state
  │   └── ACLINT, PLIC     ├── UART0  [0x1000_0000, 0x100]     ──► irq_line → PLIC
  │                         └── (Test) [0x0010_0000, 0x10]      test-only
  └── step()
       1. bus.tick()         — tick devices, collect irq_lines, notify PLIC
       2. sync_interrupts()  — merge irq_state → mip
       3. check_pending_interrupts()
       4. fetch/decode/execute
       5. retire()
```

| irq_state bit | Name | Writer |
|---------------|------|--------|
| 1 | SSIP | ACLINT SSWI |
| 3 | MSIP | ACLINT MSWI |
| 7 | MTIP | ACLINT MTIMER |
| 9 | SEIP | PLIC ctx 1 |
| 11 | MEIP | PLIC ctx 0 |

---

## Device Trait

```rust
pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
    fn irq_line(&self) -> bool { false }
    fn notify(&mut self, _irq_lines: u32) {}
}
```

- `tick()`: per-step state update (ACLINT refreshes mtime, UART drains rx_buf)
- `irq_line()`: level-triggered IRQ output (Bus collects into irq_lines)
- `notify()`: generic bus-to-device hook (only PLIC overrides — receives collected irq_lines)

Rationale for `notify()`: avoids downcast, avoids cross-device references, avoids PLIC-specific Bus logic. One default-no-op method is minimal cost for clean decoupling.

---

## Bus

```rust
pub struct Bus {
    ram: Ram,
    mmio: Vec<MmioRegion>,
    plic_idx: Option<usize>,  // set when name == "plic"
}

struct MmioRegion {
    name: &'static str,
    range: Range<usize>,
    dev: Box<dyn Device>,
    irq_source: u32,  // PLIC source ID (0 = no IRQ)
}
```

```rust
pub fn tick(&mut self) {
    let mut irq_lines: u32 = 0;
    for r in &mut self.mmio {
        r.dev.tick();
        if r.irq_source > 0 && r.dev.irq_line() {
            irq_lines |= 1 << r.irq_source;
        }
    }
    if let Some(i) = self.plic_idx {
        self.mmio[i].dev.notify(irq_lines);
    }
}
```

---

## ACLINT

MSWI + MTIMER + SSWI in single 64KB region. Wire-compatible with legacy CLINT layout.

```rust
mmio_regs! {
    enum Reg {
        Msip = 0x0000, MtimecmpLo = 0x4000, MtimecmpHi = 0x4004,
        MtimeLo = 0xBFF8, MtimeHi = 0xBFFC, Setssip = 0xC000,
    }
}

pub struct Aclint {
    epoch: Instant, mtime: u64, msip: u32, mtimecmp: u64,
    irq_state: Arc<AtomicU64>,
}
```

- mtime: host wall clock, 10 MHz (`epoch.elapsed().as_nanos() / 100`), sampled per tick only (frozen during xdb pause)
- mtimecmp: write lo/hi halves; check_timer on each write and tick
- msip: bit 0 only; sets/clears MSIP in irq_state
- setssip: edge-triggered write-only; write 1 sets SSIP; read always returns 0
- `timebase-frequency = 10_000_000`

---

## PLIC

32 sources, 2 contexts (0=M, 1=S). Level-triggered. Claimed-exclusion.

```rust
pub struct Plic {
    priority: Vec<u8>, pending: u32, enable: Vec<u32>,
    threshold: Vec<u8>, claimed: Vec<u32>,
    irq_state: Arc<AtomicU64>,
}
```

Register decode uses manual range-matching (computed offsets).

**update(irq_lines):** For each source 1..32: skip if claimed; set pending if line high; clear if low.

**claim(ctx):** Find highest-priority enabled pending source above threshold. Clear pending, store in claimed.

**complete(ctx, src):** If claimed matches, release. Re-evaluate MEIP/SEIP.

**evaluate():** For each context, check if any qualified pending source exists. Set/clear MEIP/SEIP in irq_state.

---

## UART 16550

```rust
pub struct Uart {
    ier: u8, lcr: u8, mcr: u8, dll: u8, dlm: u8, scr: u8,
    rx_fifo: VecDeque<u8>, rx_buf: Arc<Mutex<VecDeque<u8>>>,
}
```

- TX: THR write → stdout
- RX: `Uart::with_tcp(port)` spawns TCP listener. Bind failure → TX-only fallback with warning. Single accept. Disconnect → RX stops, TX continues. (Disconnect not validated this round.)
- DLAB (lcr bit 7) switches offset 0/1 between data/divisor registers
- LSR: DR from rx_fifo, THRE/TEMT always set
- IIR: 0xC4 when rx data + IER.rx; 0xC1 otherwise
- irq_line: `!rx_fifo.is_empty() && ier & 1 != 0`
- Byte-access only (size != 1 → error)

---

## TestFinisher (test-only)

```rust
pub struct TestFinisher;
```

Write `0x5555` → `ProgramExit(0)`. Write `(code<<16)|0x3333` → `ProgramExit(code)`. Not in default machine wiring.

---

## Invariants

- I-1: mip hardware bits modified only via irq_state merge
- I-2: Ordering: tick → sync → check → fetch/execute → retire
- I-3: Claimed PLIC sources excluded from re-pending until complete
- I-4: Devices use offset-relative addresses
- I-5: mtime frozen during xdb pause
- I-6: SSWI: write 1 sets SSIP; read returns 0
- I-7: TCP bind failure → TX-only fallback

---

## Constraints

- C-1: xemu internal layout (QEMU-like address/size shape). Deltas: ACLINT replaces CLINT; TestFinisher test-only.
- C-2: Single hart
- C-3: UART byte-access only
- C-4: `Device::read(&mut self)`
- C-5: mtime host 10MHz, frozen during xdb pause, `timebase-frequency=10_000_000`
- C-6: PLIC source 0 = no interrupt
- C-7: SSWI read returns 0
- C-8: TCP bind failure → TX-only fallback (validated). Disconnect behavior is defined but not validated this round.

---

## Memory Map

| Device | Base | Size | PLIC IRQ |
|--------|------|------|----------|
| ACLINT | `0x0200_0000` | `0x1_0000` | — |
| PLIC | `0x0C00_0000` | `0x400_0000` | — |
| UART0 | `0x1000_0000` | `0x100` | 10 |
| RAM | `0x8000_0000` | 128 MB | — |

TestFinisher: `0x0010_0000` / `0x10` — test-only.

---

## Files

```
xcore/src/device/
├── mod.rs            — Device trait, constants, mmio_regs!
├── bus.rs            — Bus (tick, plic_idx)
├── ram.rs
├── aclint.rs         — ACLINT (new)
├── plic.rs           — PLIC (new)
├── uart.rs           — UART 16550 (new)
└── test_finisher.rs  — TestFinisher (new, test-only)
```

---

## Implementation Steps

1. **Step 0 — Infrastructure**: Extend Device trait, Bus (tick/plic_idx/irq_source), RVCore (irq_state/sync_interrupts/step), XError::ProgramExit
2. **Step 1 — ACLINT**: aclint.rs + tests
3. **Step 2 — PLIC**: plic.rs + tests
4. **Step 3 — UART**: uart.rs + tests
5. **Step 4 — TestFinisher**: test_finisher.rs + tests
6. **Step 5 — Wiring**: Register devices in RVCore::new(), integration tests
