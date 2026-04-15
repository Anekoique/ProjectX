# `Device Emulation` PLAN `02`

> Status: Revised
> Feature: `dev`
> Iteration: `02`
> Owner: Executor
> Depends on:
> - Previous Plan: `01_PLAN.md`
> - Review: `01_REVIEW.md`
> - Master Directive: `01_MASTER.md`

---

## Summary

Implement Phase 4 device emulation in a single scope: ACLINT (MSWI + MTIMER + SSWI), PLIC, UART 16550 (TX + RX via TCP), and SiFive Test Finisher. QEMU virt-compatible memory map (matching KXemu virt DTS as compatibility baseline). `Arc<AtomicU64>` lock-free interrupt delivery with level-triggered device IRQ lines. PLIC uses correct claim/complete semantics — claimed sources are not re-pended until after complete.

## Log

[**Feature Introduce**]

- ACLINT replaces legacy CLINT: single-region layout (MSWI + MTIMER + SSWI) at `0x0200_0000`. Adds SSWI for S-mode software interrupts (sets SSIP in mip). Register map is wire-compatible with legacy CLINT but includes SSWI at offset `0xC000`.
- PLIC `update_pending()` fixed: claimed sources are excluded from re-pending. Only after `complete()` can a still-asserted line re-enter pending on the next tick.
- Single converged device→PLIC architecture: `Bus.device_lines` + `Device::post_tick()`. All alternative designs removed.
- Phase 4B (UART TCP RX) included in this scope with full acceptance criteria.
- Compatibility baseline: KXemu virt DTS (UART `0x100`, CLINT/ACLINT `0x1_0000`, PLIC `0x400_0000`, `timebase-frequency = 10_000_000`).
- Enums for ACLINT offsets, PLIC regions, UART registers. Improved naming throughout.

[**Review Adjustments**]

- R-001 (scope ambiguity): Resolved. Phase 4B is included in this scope with full validation and acceptance mapping for TCP RX.
- R-002 (claimed re-pend bug): Resolved. `update_pending()` now skips sources that are currently claimed by any context. Re-pending only occurs after `complete()`.
- R-003 (multiple designs): Resolved. Only `Bus.device_lines + post_tick()` remains. All alternatives and deliberation traces removed.
- R-004 (compatibility baseline): Resolved. Explicit baseline: KXemu virt DTS. All MMIO sizes, timebase-frequency, and IRQ assignments match this single reference.

[**Master Compliance**]

- M-001 (async future): Acknowledged as SHOULD. Current sync `tick()` is noted as transitional. Architecture does not preclude async evolution. No code change this round.
- M-002 (Bus trait abstraction): Applied. Removed `post_tick(device_lines)` from Device trait. Bus handles IRQ line collection and PLIC notification internally — no device-specific methods leak into the trait.
- M-003 (reduce hard-encoding, add enums): Applied. Added `AclintReg`, `PlicReg`, `UartReg` enums for register offset dispatch. PLIC constants (`NUM_SOURCES`, `NUM_CONTEXTS`) and interrupt bits (`MTIP`, `MSIP`, etc.) are named constants.
- M-004 (better naming): Applied. Renamed throughout: `ext_ip` → `irq_state`, `device_lines` → `irq_lines`, `mtime_snapshot` → `mtime`, `boot_instant` → `epoch`. Struct field names shortened and clarified.
- M-005 (include Phase 4B): Applied. TCP RX fully specified with validation and acceptance.
- M-006 (ACLINT): Applied. Replaced CLINT with ACLINT. Single-region layout with MSWI + MTIMER + SSWI. Added SSWI registers and SSIP handling.

### Changes from Previous Round

[**Added**]
- SSWI sub-device (offsets `0xC000–0xFFFF`): sets SSIP (bit 1) in `irq_state`
- `AclintReg`, `PlicReg`, `UartReg` enums for register dispatch
- Named constants for interrupt bits: `MSIP_BIT`, `SSIP_BIT`, `MTIP_BIT`, `SEIP_BIT`, `MEIP_BIT`
- Phase 4B: UART TCP RX with full implementation detail, validation (V-IT-6, V-IT-7), and acceptance mapping
- PLIC claimed-exclusion logic in `update_pending()`
- Explicit compatibility baseline (KXemu virt DTS)

[**Changed**]
- CLINT → ACLINT (adds SSWI, same base address and layout). Why: M-006 directive.
- `post_tick(device_lines)` removed from Device trait → Bus handles internally. Why: M-002 directive.
- All naming: `ext_ip` → `irq_state`, `device_lines` → `irq_lines`, `mtime_snapshot` → `mtime`, `boot_instant` → `epoch`. Why: M-004 directive.
- PLIC `update_pending()`: claimed sources excluded from re-pend. Why: R-002 correctness bug.
- Raw arrays → Vec in PLIC (already done in 01, confirmed).
- Register dispatch uses enums instead of raw hex. Why: M-003 directive.

[**Removed**]
- All alternative device→PLIC designs (PlicSource registry, find_plic_mut, set_device_lines). Why: R-003.
- All "reconsider" / deliberation traces. Why: R-003.
- `post_tick()` from Device trait. Why: M-002.

[**Unresolved**]
- None. All blocking and non-blocking issues resolved.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Phase 4B included with full validation and acceptance |
| Review | R-002 | Accepted | `update_pending()` skips claimed sources; re-pend only after complete |
| Review | R-003 | Accepted | Single design retained; all alternatives removed |
| Review | R-004 | Accepted | Explicit baseline: KXemu virt DTS |
| Master | M-001 | Acknowledged | Sync tick is transitional; architecture allows async evolution |
| Master | M-002 | Applied | `post_tick()` removed from Device trait; Bus handles internally |
| Master | M-003 | Applied | Added AclintReg, PlicReg, UartReg enums; named constants for interrupt bits |
| Master | M-004 | Applied | Renamed: ext_ip→irq_state, device_lines→irq_lines, mtime_snapshot→mtime, boot_instant→epoch |
| Master | M-005 | Applied | Phase 4B (TCP RX) fully included |
| Master | M-006 | Applied | CLINT replaced with ACLINT (MSWI + MTIMER + SSWI) |
| Trade-off | TR-1 | Rejected | Phase 4B included per M-005; full acceptance provided |
| Trade-off | TR-2 | Adopted | Bus.irq_lines + internal PLIC notification is the only design |

---

## Spec

[**Goals**]

- G-1: ACLINT with MSWI (msip → MSIP), MTIMER (mtime 10MHz + mtimecmp → MTIP), SSWI (setssip → SSIP)
- G-2: PLIC with 32 sources, 2 contexts (M/S), correct level-triggered claim/complete semantics
- G-3a: UART 16550 TX — THR → stdout
- G-3b: UART 16550 RX — TCP socket backend (`127.0.0.1:14514`), PLIC source 10
- G-4: SiFive Test Finisher for bare-metal test exit
- G-5: Lock-free interrupt delivery via `Arc<AtomicU64>` (`irq_state`)

- NG-1: OpenSBI / Device Tree / SBI handoff
- NG-2: Multi-hart support
- NG-3: DMA or scatter-gather I/O
- NG-4: Async device tick (noted as future direction per M-001)

[**Architecture**]

```
                           ┌──────────────────────────────────────┐
  RVCore                   │  Bus (Arc<Mutex<Bus>>)               │
  ├── csr, mmu, pmp       │  ├── Ram    [0x8000_0000, 128M]      │
  ├── irq_state ──poll───► │  ├── ACLINT [0x0200_0000, 0x1_0000]  │
  │   Arc<AtomicU64>       │  ├── PLIC   [0x0C00_0000, 0x400_0000]│
  │   ▲  ▲                │  ├── UART0  [0x1000_0000, 0x100]     │
  │   │  └── ACLINT ──────┤  └── Test   [0x0010_0000, 0x10]      │
  │   └───── PLIC ────────┤                                      │
  └── step()               └──────────────────────────────────────┘
       1. bus.lock().tick()
       2. sync_interrupts()
       3. check_pending_interrupts()
       4. fetch → decode → execute
       5. retire()
```

**Interrupt delivery (CPU ← devices):**

`Arc<AtomicU64>` (`irq_state`), each bit = mip bit position:

| Bit | Name | Writer |
|-----|------|--------|
| 1   | SSIP | ACLINT SSWI (write 1 to setssip) |
| 3   | MSIP | ACLINT MSWI (write 1 to msip) |
| 7   | MTIP | ACLINT MTIMER (mtime >= mtimecmp) |
| 9   | SEIP | PLIC (S-mode context has qualified pending) |
| 11  | MEIP | PLIC (M-mode context has qualified pending) |

**Interrupt delivery (peripheral → PLIC):**

Level-triggered. Bus collects `irq_line()` from each device during `tick()`, stores result in `Bus.irq_lines: u32`. Bus then passes `irq_lines` to PLIC internally (PLIC is located by name in mmio vec). No device-specific method on the Device trait.

```
Bus::tick():
  1. Tick all devices (ACLINT refreshes mtime, UART drains rx_buf)
  2. Collect irq_line() from each device → irq_lines
  3. Find PLIC in mmio, call plic.update(irq_lines) internally
```

[**Invariants**]

- I-1: Hardware-wired mip bits (SSIP, MSIP, MTIP, SEIP, MEIP) are only modified via irq_state merge
- I-2: Per-step ordering: `bus.tick()` → `sync_interrupts()` → `check_pending_interrupts()` → fetch/decode/execute → `retire()`
- I-3: PLIC pending reflects device line state, but **claimed sources are excluded from re-pending**. After `complete()`, if line is still high, source re-enters pending on next tick.
- I-4: Device read/write only access offset-relative addresses
- I-5: mtime sampled only during `Aclint::tick()` — frozen during xdb pause
- I-6: SSWI setssip is edge-triggered: write 1 sets SSIP; read always returns 0

[**Data Structure**]

```rust
// device/mod.rs
pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
    fn irq_line(&self) -> bool { false }
}

// Interrupt bit constants
const SSIP_BIT: u64 = 1 << 1;
const MSIP_BIT: u64 = 1 << 3;
const MTIP_BIT: u64 = 1 << 7;
const SEIP_BIT: u64 = 1 << 9;
const MEIP_BIT: u64 = 1 << 11;
const HW_IP_MASK: Word = (SSIP_BIT | MSIP_BIT | MTIP_BIT | SEIP_BIT | MEIP_BIT) as Word;
```

```rust
// device/aclint.rs
enum AclintReg {
    Msip,                // 0x0000
    MtimecmpLo,          // 0x4000
    MtimecmpHi,          // 0x4004
    MtimeLo,             // 0xBFF8
    MtimeHi,             // 0xBFFC
    Setssip,             // 0xC000
}

pub struct Aclint {
    epoch: Instant,
    mtime: u64,
    msip: u32,
    mtimecmp: u64,
    irq_state: Arc<AtomicU64>,
}
```

```rust
// device/plic.rs
enum PlicReg {
    Priority(usize),      // 0x000000 + src*4
    Pending,              // 0x001000
    Enable(usize),        // 0x002000 + ctx*0x80
    Threshold(usize),     // 0x200000 + ctx*0x1000
    Claim(usize),         // 0x200004 + ctx*0x1000
}

const NUM_SOURCES: usize = 32;
const NUM_CONTEXTS: usize = 2;  // 0=M-mode, 1=S-mode

pub struct Plic {
    priority: Vec<u8>,
    pending: u32,
    enable: Vec<u32>,      // [NUM_CONTEXTS]
    threshold: Vec<u8>,    // [NUM_CONTEXTS]
    claimed: Vec<u32>,     // [NUM_CONTEXTS] source being serviced
    irq_state: Arc<AtomicU64>,
}
```

```rust
// device/uart.rs
enum UartReg {
    Rbr,   // 0: DLAB=0, read
    Thr,   // 0: DLAB=0, write
    Dll,   // 0: DLAB=1
    Ier,   // 1: DLAB=0
    Dlm,   // 1: DLAB=1
    Iir,   // 2: read
    Fcr,   // 2: write
    Lcr,   // 3
    Mcr,   // 4
    Lsr,   // 5
    Msr,   // 6
    Scr,   // 7: scratch
}

pub struct Uart {
    ier: u8,
    lcr: u8,
    mcr: u8,
    dll: u8,
    dlm: u8,
    scr: u8,
    rx_fifo: VecDeque<u8>,
    rx_buf: Arc<Mutex<VecDeque<u8>>>,  // TCP reader thread writes here
}

// device/test_finisher.rs
pub struct TestFinisher;
```

[**API Surface**]

```rust
// Device trait — clean, no device-specific methods
pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
    fn irq_line(&self) -> bool { false }
}

// MmioRegion
struct MmioRegion {
    name: &'static str,
    range: Range<usize>,
    dev: Box<dyn Device>,
    irq_source: u32,    // PLIC source ID (0 = no IRQ)
}

// Bus — handles device→PLIC internally
impl Bus {
    pub fn add_mmio(&mut self, name: &'static str, base: usize, size: usize,
                    dev: Box<dyn Device>, irq_source: u32);
    pub fn tick(&mut self);  // tick devices, collect irq_lines, notify PLIC
}

// Aclint
impl Aclint {
    pub fn new(irq_state: Arc<AtomicU64>) -> Self;
}

// Plic
impl Plic {
    pub fn new(irq_state: Arc<AtomicU64>) -> Self;
    pub fn update(&mut self, irq_lines: u32);  // called by Bus, not by Device trait
}

// Uart
impl Uart {
    pub fn new() -> Self;             // Phase 4A: TX-only
    pub fn with_tcp(port: u16) -> Self;  // Phase 4B: spawns TCP listener thread
}

// TestFinisher
impl TestFinisher {
    pub fn new() -> Self;
}

// RVCore
impl RVCore {
    fn sync_interrupts(&mut self);  // merge irq_state → mip
}
```

[**Constraints**]

- C-1: Compatibility baseline is KXemu virt DTS: UART `0x100` at `0x1000_0000`, ACLINT `0x1_0000` at `0x0200_0000`, PLIC `0x400_0000` at `0x0C00_0000`, `timebase-frequency = 10_000_000`, UART IRQ = PLIC source 10
- C-2: Single hart only
- C-3: UART byte-access only (size != 1 → error)
- C-4: Device::read is `&mut self`
- C-5: mtime is host-wall-clock at 10 MHz, sampled per tick, frozen during xdb pause
- C-6: PLIC source 0 is hardwired to "no interrupt"
- C-7: SSWI setssip read always returns 0 (edge-triggered write-only semantics)

---

## Implement

### Execution Flow

[**step()**]

```rust
fn step(&mut self) -> XResult {
    { let mut bus = self.bus.lock().unwrap(); bus.tick(); }
    self.sync_interrupts();
    if self.check_pending_interrupts() {
        self.retire();
        return Ok(());
    }
    self.trap_on_err(|core| {
        let raw = core.fetch()?;
        let inst = core.decode(raw)?;
        core.execute(inst)
    })?;
    self.retire();
    Ok(())
}
```

[**sync_interrupts()**]

```rust
fn sync_interrupts(&mut self) {
    let ext = self.irq_state.load(Relaxed);
    let mip = self.csr.get(CsrAddr::mip);
    self.csr.set(CsrAddr::mip, (mip & !HW_IP_MASK) | (ext as Word & HW_IP_MASK));
}
```

[**Bus::tick()**]

```rust
pub fn tick(&mut self) {
    // Phase 1: tick all devices
    for region in &mut self.mmio {
        region.dev.tick();
    }
    // Phase 2: collect IRQ lines
    let mut irq_lines: u32 = 0;
    for region in &self.mmio {
        if region.irq_source > 0 && region.dev.irq_line() {
            irq_lines |= 1 << region.irq_source;
        }
    }
    // Phase 3: notify PLIC (find by name, downcast)
    if let Some(plic) = self.mmio.iter_mut()
        .find(|r| r.name == "plic")
        .and_then(|r| r.dev.as_any_mut().downcast_mut::<Plic>())
    {
        plic.update(irq_lines);
    }
}
```

Note: requires `Device` to provide `as_any_mut()` for downcast. Alternative: store PLIC index at registration time. Implementation will choose the cleanest approach — the important contract is that Bus handles PLIC notification internally, not through the Device trait.

[**Failure Flow**]

1. TestFinisher write → `Err(XError::ProgramExit(code))` → CPU halts
2. Unmapped MMIO → `Err(XError::BadAddress)` → trap
3. UART non-byte access → `Err(XError::BadAddress)`

[**State Transition**]

- Device line high → Bus collects in irq_lines → `plic.update(irq_lines)` sets pending (if not claimed) → `evaluate()` sets MEIP/SEIP → sync → mip → interrupt taken
- Claim → records source, clears pending → guest services → Complete → releases claimed → next tick: if line still high, re-pended

### Implementation Plan

[**Step 0: Interrupt Plumbing**]

Files:
- `device/mod.rs`: Extend Device trait (`tick`, `irq_line`, `read(&mut self)`). Add interrupt bit constants.
- `device/bus.rs`: Add `irq_lines: u32`, `irq_source` to MmioRegion. Implement `Bus::tick()` three-phase. Extend `add_mmio()`.
- `cpu/riscv/mod.rs`: Add `irq_state: Arc<AtomicU64>` to RVCore. Implement `sync_interrupts()`. Update `step()`.
- `error.rs`: Add `XError::ProgramExit(u32)`.
- `cpu/mod.rs`: Catch `ProgramExit` in `CPU::step()`.

[**Step 1: ACLINT**]

File: `device/aclint.rs`

```rust
impl Aclint {
    pub fn new(irq_state: Arc<AtomicU64>) -> Self {
        Self { epoch: Instant::now(), mtime: 0, msip: 0, mtimecmp: u64::MAX, irq_state }
    }
    fn refresh_mtime(&mut self) { self.mtime = self.epoch.elapsed().as_nanos() as u64 / 100; }
    fn check_timer(&mut self) {
        if self.mtime >= self.mtimecmp {
            self.irq_state.fetch_or(MTIP_BIT, Relaxed);
        } else {
            self.irq_state.fetch_and(!MTIP_BIT, Relaxed);
        }
    }
    fn decode(offset: usize) -> Option<AclintReg> {
        match offset {
            0x0000 => Some(AclintReg::Msip),
            0x4000 => Some(AclintReg::MtimecmpLo),
            0x4004 => Some(AclintReg::MtimecmpHi),
            0xBFF8 => Some(AclintReg::MtimeLo),
            0xBFFC => Some(AclintReg::MtimeHi),
            0xC000 => Some(AclintReg::Setssip),
            _ => None,
        }
    }
}

impl Device for Aclint {
    fn read(&mut self, offset: usize, _size: usize) -> XResult<Word> {
        Ok(match Self::decode(offset) {
            Some(AclintReg::Msip)        => self.msip as Word,
            Some(AclintReg::MtimecmpLo)  => self.mtimecmp as u32 as Word,
            Some(AclintReg::MtimecmpHi)  => (self.mtimecmp >> 32) as u32 as Word,
            Some(AclintReg::MtimeLo)     => self.mtime as u32 as Word,
            Some(AclintReg::MtimeHi)     => (self.mtime >> 32) as u32 as Word,
            Some(AclintReg::Setssip)     => 0, // I-6: always reads 0
            None                         => 0,
        })
    }

    fn write(&mut self, offset: usize, _size: usize, value: Word) -> XResult {
        match Self::decode(offset) {
            Some(AclintReg::Msip) => {
                self.msip = (value as u32) & 1;
                if self.msip != 0 {
                    self.irq_state.fetch_or(MSIP_BIT, Relaxed);
                } else {
                    self.irq_state.fetch_and(!MSIP_BIT, Relaxed);
                }
            }
            Some(AclintReg::MtimecmpLo) => {
                self.mtimecmp = (self.mtimecmp & !0xFFFF_FFFF) | value as u32 as u64;
                self.check_timer();
            }
            Some(AclintReg::MtimecmpHi) => {
                self.mtimecmp = (self.mtimecmp & 0xFFFF_FFFF) | ((value as u32 as u64) << 32);
                self.check_timer();
            }
            Some(AclintReg::Setssip) => {
                // Edge-triggered: write 1 sets SSIP; write 0 is no-op
                if value as u32 & 1 != 0 {
                    self.irq_state.fetch_or(SSIP_BIT, Relaxed);
                }
            }
            _ => {} // mtime read-only; unmapped ignored
        }
        Ok(())
    }

    fn tick(&mut self) {
        self.refresh_mtime();
        self.check_timer();
    }
}
```

[**Step 2: PLIC**]

File: `device/plic.rs`

```rust
impl Plic {
    pub fn new(irq_state: Arc<AtomicU64>) -> Self {
        Self {
            priority: vec![0; NUM_SOURCES],
            pending: 0,
            enable: vec![0; NUM_CONTEXTS],
            threshold: vec![0; NUM_CONTEXTS],
            claimed: vec![0; NUM_CONTEXTS],
            irq_state,
        }
    }

    fn decode(offset: usize) -> Option<PlicReg> {
        match offset {
            0x000000..=0x00007C if offset % 4 == 0 => Some(PlicReg::Priority(offset / 4)),
            0x001000 => Some(PlicReg::Pending),
            o @ 0x002000..=0x002FFF if (o - 0x002000) % 0x80 == 0 => {
                let ctx = (o - 0x002000) / 0x80;
                (ctx < NUM_CONTEXTS).then(|| PlicReg::Enable(ctx))
            }
            o if o >= 0x200000 && (o - 0x200000) % 0x1000 == 0 => {
                let ctx = (o - 0x200000) / 0x1000;
                (ctx < NUM_CONTEXTS).then(|| PlicReg::Threshold(ctx))
            }
            o if o >= 0x200004 && (o - 0x200004) % 0x1000 == 0 => {
                let ctx = (o - 0x200004) / 0x1000;
                (ctx < NUM_CONTEXTS).then(|| PlicReg::Claim(ctx))
            }
            _ => None,
        }
    }

    /// Called by Bus after collecting irq_lines from devices.
    pub fn update(&mut self, irq_lines: u32) {
        // Level-triggered: update pending from device line state
        // I-3: claimed sources are excluded from re-pending
        for src in 1..NUM_SOURCES {
            let bit = 1u32 << src;
            let is_claimed = self.claimed.iter().any(|&c| c == src as u32);
            if is_claimed {
                // Do not modify pending while claimed — preserve claim/complete contract
                continue;
            }
            if irq_lines & bit != 0 {
                self.pending |= bit;
            } else {
                self.pending &= !bit;
            }
        }
        self.evaluate();
    }

    fn claim(&mut self, ctx: usize) -> u32 {
        let mut best = 0u32;
        let mut best_prio = 0u8;
        for src in 1..NUM_SOURCES {
            let bit = 1u32 << src;
            if self.pending & bit == 0 { continue; }
            if self.enable[ctx] & bit == 0 { continue; }
            if self.priority[src] <= self.threshold[ctx] { continue; }
            if self.priority[src] > best_prio {
                best_prio = self.priority[src];
                best = src as u32;
            }
        }
        if best > 0 {
            self.pending &= !(1 << best);
            self.claimed[ctx] = best;
        }
        best
    }

    fn complete(&mut self, ctx: usize, source: u32) {
        if ctx < NUM_CONTEXTS && self.claimed[ctx] == source {
            self.claimed[ctx] = 0;
        }
        // Note: do NOT re-evaluate here. The next tick() will call update()
        // which will re-pend the source if its line is still high.
        // But we do re-evaluate ext_ip since claimed state changed.
        self.evaluate();
    }

    fn evaluate(&mut self) {
        for ctx in 0..NUM_CONTEXTS {
            let has_qualified = (1..NUM_SOURCES).any(|src| {
                let bit = 1u32 << src;
                self.pending & bit != 0
                    && self.enable[ctx] & bit != 0
                    && self.priority[src] > self.threshold[ctx]
            });
            let ip_bit = if ctx == 0 { MEIP_BIT } else { SEIP_BIT };
            if has_qualified {
                self.irq_state.fetch_or(ip_bit, Relaxed);
            } else {
                self.irq_state.fetch_and(!ip_bit, Relaxed);
            }
        }
    }
}

impl Device for Plic {
    fn read(&mut self, offset: usize, _size: usize) -> XResult<Word> {
        Ok(match Self::decode(offset) {
            Some(PlicReg::Priority(src))  => self.priority[src] as Word,
            Some(PlicReg::Pending)        => self.pending as Word,
            Some(PlicReg::Enable(ctx))    => self.enable[ctx] as Word,
            Some(PlicReg::Threshold(ctx)) => self.threshold[ctx] as Word,
            Some(PlicReg::Claim(ctx))     => self.claim(ctx) as Word,
            None                          => 0,
        })
    }

    fn write(&mut self, offset: usize, _size: usize, value: Word) -> XResult {
        match Self::decode(offset) {
            Some(PlicReg::Priority(src))  => self.priority[src] = value as u8,
            Some(PlicReg::Enable(ctx))    => self.enable[ctx] = value as u32,
            Some(PlicReg::Threshold(ctx)) => {
                self.threshold[ctx] = value as u8;
                self.evaluate();
            }
            Some(PlicReg::Claim(ctx))     => self.complete(ctx, value as u32),
            _ => {}
        }
        Ok(())
    }
}
```

[**Step 3: UART 16550 (TX + RX)**]

File: `device/uart.rs`

Phase 4A (TX-only): `Uart::new()` — no TCP thread, `irq_line()` returns false.

Phase 4B (TX + RX): `Uart::with_tcp(port)` — spawns TCP listener thread.

```rust
impl Uart {
    pub fn new() -> Self {
        Self {
            ier: 0, lcr: 0x03, mcr: 0, dll: 0, dlm: 0, scr: 0,
            rx_fifo: VecDeque::new(),
            rx_buf: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    pub fn with_tcp(port: u16) -> Self {
        let rx_buf = Arc::new(Mutex::new(VecDeque::new()));
        let rx_clone = rx_buf.clone();
        std::thread::spawn(move || {
            let listener = std::net::TcpListener::bind(("127.0.0.1", port)).unwrap();
            info!("UART listening on 127.0.0.1:{port}");
            if let Ok((stream, _)) = listener.accept() {
                use std::io::Read;
                for byte in stream.bytes().flatten() {
                    rx_clone.lock().unwrap().push_back(byte);
                }
            }
        });
        Self { rx_buf, ..Self::new() }
    }

    fn is_dlab(&self) -> bool { self.lcr & 0x80 != 0 }

    fn lsr(&self) -> u8 {
        let dr = if self.rx_fifo.is_empty() { 0 } else { 0x01 };
        dr | 0x60 // THRE + TEMT always set
    }

    fn iir(&self) -> u8 {
        if !self.rx_fifo.is_empty() && self.ier & 0x01 != 0 {
            0xC4 // FIFO enabled + RX data available
        } else {
            0xC1 // FIFO enabled + no interrupt
        }
    }
}

impl Device for Uart {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word> {
        if size != 1 { return Err(XError::BadAddress); }
        Ok(match offset {
            0 if self.is_dlab()  => self.dll,
            0                    => self.rx_fifo.pop_front().unwrap_or(0),
            1 if self.is_dlab()  => self.dlm,
            1                    => self.ier,
            2                    => self.iir(),
            3                    => self.lcr,
            4                    => self.mcr,
            5                    => self.lsr(),
            6                    => 0,
            7                    => self.scr,
            _                    => 0,
        } as Word)
    }

    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult {
        if size != 1 { return Err(XError::BadAddress); }
        let byte = value as u8;
        match offset {
            0 if self.is_dlab()  => self.dll = byte,
            0 => {
                use std::io::Write;
                let _ = std::io::stdout().lock().write_all(&[byte]);
                let _ = std::io::stdout().flush();
            }
            1 if self.is_dlab()  => self.dlm = byte,
            1                    => self.ier = byte & 0x0F,
            2                    => {} // FCR ignored
            3                    => self.lcr = byte,
            4                    => self.mcr = byte,
            7                    => self.scr = byte,
            _                    => {}
        }
        Ok(())
    }

    fn tick(&mut self) {
        // Drain TCP rx_buf into rx_fifo
        if let Ok(mut buf) = self.rx_buf.try_lock() {
            self.rx_fifo.extend(buf.drain(..));
        }
    }

    fn irq_line(&self) -> bool {
        !self.rx_fifo.is_empty() && self.ier & 0x01 != 0
    }
}
```

[**Step 4: SiFive Test Finisher**]

File: `device/test_finisher.rs`

```rust
impl Device for TestFinisher {
    fn read(&mut self, _offset: usize, _size: usize) -> XResult<Word> { Ok(0) }

    fn write(&mut self, offset: usize, _size: usize, value: Word) -> XResult {
        if offset == 0 {
            let val = value as u32;
            match val & 0xFFFF {
                0x5555 => return Err(XError::ProgramExit(0)),
                0x3333 => return Err(XError::ProgramExit(val >> 16)),
                _ => {}
            }
        }
        Ok(())
    }
}

impl TestFinisher {
    pub fn new() -> Self { Self }
}
```

[**Step 5: Integration & Wiring**]

```rust
// RVCore::new() or with_bus() — register all devices
impl RVCore {
    pub fn new() -> Self {
        let irq_state = Arc::new(AtomicU64::new(0));
        let mut bus = Bus::new(CONFIG_MBASE, CONFIG_MSIZE);
        bus.add_mmio("aclint", 0x0200_0000, 0x1_0000,
                     Box::new(Aclint::new(irq_state.clone())), 0);
        bus.add_mmio("plic",   0x0C00_0000, 0x400_0000,
                     Box::new(Plic::new(irq_state.clone())), 0);
        bus.add_mmio("uart0",  0x1000_0000, 0x100,
                     Box::new(Uart::with_tcp(14514)), 10);
        bus.add_mmio("test",   0x0010_0000, 0x10,
                     Box::new(TestFinisher::new()), 0);
        let bus = Arc::new(Mutex::new(bus));
        Self { irq_state, bus, ..Self::default_fields() }
    }
}

// CPU::step() — catch ProgramExit
pub fn step(&mut self) -> XResult {
    match self.core.step() {
        Err(XError::ProgramExit(code)) => {
            self.halt_ret = code as Word;
            self.set_terminated(if code == 0 { State::HALTED } else { State::ABORT })
                .log_termination();
            Ok(())
        }
        other => {
            if self.core.halted() {
                self.set_terminated(State::HALTED).log_termination();
            }
            other
        }
    }
}
```

---

## Trade-offs

- T-1: **Interrupt delivery** — `Arc<AtomicU64>` lock-free. Multi-core ready. Sync `tick()` is transitional (M-001 acknowledged); architecture allows async evolution without changing the `irq_state` interface.

- T-2: **ACLINT mtime** — Host wall clock, 10 MHz, sampled per tick. `timebase-frequency = 10_000_000` (matches KXemu). Frozen during xdb pause (I-5). Trade-off: non-deterministic timing, but acceptable for emulator not targeting cycle-accuracy.

- T-3: **UART RX backend** — TCP socket at `127.0.0.1:14514`. User connects via `nc localhost 14514`. No stdin conflict with xdb. Trade-off: requires second terminal, but cleanly separates debugger and guest I/O.

- T-4: **Device→PLIC** — Bus collects `irq_line()` into `irq_lines`, passes to PLIC internally. No cross-device references. Level-triggered by construction. Trade-off: Bus has PLIC-specific logic in `tick()`, but this is isolated and the Device trait stays clean (M-002).

---

## Validation

[**Unit Tests**]

- V-UT-1: ACLINT mtime increases after successive ticks
- V-UT-2: ACLINT mtimecmp write → MTIP set when mtime >= mtimecmp
- V-UT-3: ACLINT mtimecmp write → MTIP clear when mtime < mtimecmp
- V-UT-4: ACLINT msip write 1 → MSIP set; write 0 → MSIP clear
- V-UT-5: ACLINT setssip write 1 → SSIP set; read returns 0
- V-UT-6: PLIC priority read/write for all sources
- V-UT-7: PLIC enable register per context
- V-UT-8: PLIC claim returns highest-priority enabled pending source above threshold
- V-UT-9: PLIC claim returns 0 when nothing pending
- V-UT-10: PLIC complete releases claimed source
- V-UT-11: PLIC threshold filters low-priority sources
- V-UT-12: PLIC claimed source not re-pended by update() while claimed
- V-UT-13: PLIC source re-pended after complete() when line still high
- V-UT-14: UART THR write
- V-UT-15: UART LSR DR bit reflects rx_fifo state
- V-UT-16: UART DLAB switches offset 0/1 to DLL/DLM
- V-UT-17: UART IIR 0xC4 when rx data + IER.rx; 0xC1 otherwise
- V-UT-18: UART irq_line() true when rx_fifo non-empty and IER.rx enabled
- V-UT-19: TestFinisher write 0x5555 → ProgramExit(0)
- V-UT-20: TestFinisher write (1<<16)|0x3333 → ProgramExit(1)

[**Integration Tests**]

- V-IT-1: ACLINT timer → MTIP → mip → M-mode timer trap
- V-IT-2: PLIC with device irq_line high → MEIP → mip → trap
- V-IT-3: TestFinisher → CPU halts with correct exit code
- V-IT-4: UART FIFO partial read: push 3 bytes, claim, read 1, complete → next tick re-pends (level-triggered)
- V-IT-5: ACLINT mtime frozen between ticks (pause semantics)
- V-IT-6: UART TCP RX: connect, send bytes → rx_fifo populated after tick → LSR.DR set
- V-IT-7: UART TCP RX + PLIC: connect, send bytes → irq_line high → PLIC pending → MEIP

[**Failure / Robustness Validation**]

- V-F-1: ACLINT access to unmapped offsets returns 0
- V-F-2: PLIC claim with nothing pending returns 0
- V-F-3: UART non-byte access returns BadAddress
- V-F-4: PLIC complete with wrong source — no state change
- V-F-5: ACLINT mtime write ignored (read-only)

[**Edge Case Validation**]

- V-E-1: ACLINT mtimecmp = u64::MAX — timer never fires
- V-E-2: PLIC complete with mismatched source — claimed unchanged
- V-E-3: UART all 8 offsets in both DLAB modes
- V-E-4: ACLINT setssip write 0 — no effect on SSIP
- V-E-5: PLIC source 0 — always excluded from claim

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (ACLINT MSWI) | V-UT-4, V-IT-1 |
| G-1 (ACLINT MTIMER) | V-UT-1..3, V-IT-1, V-IT-5, V-E-1 |
| G-1 (ACLINT SSWI) | V-UT-5, V-E-4 |
| G-2 (PLIC) | V-UT-6..13, V-IT-2, V-IT-4, V-E-2, V-E-5 |
| G-3a (UART TX) | V-UT-14..17, V-E-3 |
| G-3b (UART RX) | V-UT-18, V-IT-6, V-IT-7 |
| G-4 (TestFinisher) | V-UT-19..20, V-IT-3 |
| G-5 (irq_state) | V-IT-1, V-IT-2, V-IT-7 |
| C-5 (mtime pause) | V-IT-5 |
| I-3 (claimed exclusion) | V-UT-12, V-UT-13, V-IT-4 |
| I-6 (SSWI edge) | V-UT-5, V-E-4 |

---

## Memory Map (Compatibility Baseline: KXemu virt DTS)

| Device | Base | Guest Size | Internal Decoded | PLIC IRQ |
|--------|------|-----------|------------------|----------|
| SiFive Test Finisher | `0x0010_0000` | `0x10` | offset 0 | — |
| ACLINT | `0x0200_0000` | `0x1_0000` | MSWI, MTIMER, SSWI | — |
| PLIC | `0x0C00_0000` | `0x400_0000` | priority, pending, enable, threshold, claim | — |
| UART0 | `0x1000_0000` | `0x100` | offsets 0-7 | source 10 |
| RAM | `0x8000_0000` | 128 MB | full | — |

## File Organization

```
xcore/src/device/
├── mod.rs            — Device trait (read, write, tick, irq_line), interrupt constants
├── bus.rs            — Bus (+ tick, irq_lines, irq_source, PLIC notification)
├── ram.rs            — Ram
├── aclint.rs         — ACLINT: MSWI + MTIMER + SSWI (new)
├── plic.rs           — PLIC (new)
├── uart.rs           — UART 16550 (new)
└── test_finisher.rs  — SiFive Test Finisher (new)
```
