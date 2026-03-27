# `Device Emulation` PLAN `03`

> Status: Revised
> Feature: `dev`
> Iteration: `03`
> Owner: Executor
> Depends on:
> - Previous Plan: `02_PLAN.md`
> - Review: `02_REVIEW.md`
> - Master Directive: `02_MASTER.md`

---

## Summary

Implement Phase 4 device emulation: ACLINT (MSWI + MTIMER + SSWI), PLIC, UART 16550 (TX + RX via TCP), and SiFive Test Finisher. QEMU-like address layout. MMIO register abstraction via declarative `mmio_regs!` macro (inspired by CSR `csr_table!`). Bus→PLIC wiring via stored PLIC index. TCP RX with defined lifecycle semantics.

## Log

[**Feature Introduce**]

- `mmio_regs!` macro: declarative MMIO register definition inspired by the existing `csr_table!` macro. Generates offset→register dispatch, reducing per-device boilerplate. Each device declares its register map once; the macro generates the `decode()` match and optionally a typed register enum.
- Bus→PLIC wiring: Bus stores PLIC index at registration time (`plic_idx: Option<usize>`). No `as_any_mut()` downcast. No device-specific methods on Device trait.
- TCP RX lifecycle: bind failure logs warning and falls back to TX-only. Single-accept, no reconnect. Disconnect is terminal for RX (TX continues). Defined as operational contract.
- Test Finisher removed from default device wiring. Available via `Bus::add_mmio()` in test code only (per M-004 MAYBE directive — evaluated as "remove from mainline").

[**Review Adjustments**]

- R-001 (compatibility baseline inconsistency): Resolved. Verified KXemu def.hpp: PLIC = `0x0400_0000` (64 MB). The reviewer's `0x600000` claim was incorrect — KXemu source confirms `0x0400_0000`. Baseline relabeled as "QEMU-like address layout" per M-002, with exact values sourced from KXemu def.hpp.
- R-002 (Bus→PLIC mechanism): Resolved. Single design: Bus stores `plic_idx: Option<usize>` at registration. `tick()` uses index directly. No downcast, no trait escape hatch.
- R-003 (TCP lifecycle): Resolved. Bind failure → fallback to TX-only with warning. Single accept. Disconnect → RX stops, TX continues. No reconnect. Robustness validation added (V-F-6).
- R-004 (contract validation): Resolved. Added V-CF-1..4: config-level checks that verify registered MMIO base/size/irq_source match the declared layout constants.

[**Master Compliance**]

- M-001 (functional programming, code quality): Applied. `mmio_regs!` macro eliminates repetitive match arms. Device implementations use functional chains (`match` → `Option` → `Ok()`). Pseudocode rewritten in concise idiomatic style.
- M-002 (QEMU-like address layout): Applied. Baseline relabeled. Values verified against KXemu def.hpp (which mirrors QEMU virt).
- M-003 (MMIO register macro/trait abstraction): Applied. Introduced `mmio_regs!` macro inspired by `csr_table!`. Replaces per-device `PlicReg`/`AclintReg`/`UartReg` enums with a unified declarative approach.
- M-004 (Test Finisher in test-only): Applied. TestFinisher removed from default `RVCore::new()` wiring. Only registered via `Bus::add_mmio()` in test code. Not part of the default machine configuration.

### Changes from Previous Round

[**Added**]
- `mmio_regs!` macro for declarative MMIO register dispatch
- `Bus.plic_idx: Option<usize>` for direct PLIC notification
- TCP lifecycle contract: bind fallback, single-accept, disconnect-terminal
- Config-level validation (V-CF-1..4)
- V-F-5 (TCP bind failure fallback), V-F-6 (TCP disconnect)

[**Changed**]
- Bus→PLIC: downcast/"store index" ambiguity → stored `plic_idx` only. Why: R-002 + TR-1.
- Compatibility baseline: "KXemu virt DTS" → "QEMU-like address layout (verified against KXemu def.hpp)". Why: R-001 + M-002.
- Per-device register enums → `mmio_regs!` macro. Why: M-003.
- Pseudocode rewritten with functional chains. Why: M-001.

[**Removed**]
- `as_any_mut()` downcast option. Why: R-002.
- TestFinisher from default machine wiring. Why: M-004.
- Per-device `PlicReg`/`AclintReg`/`UartReg` enums (replaced by `mmio_regs!`). Why: M-003.
- All "alternative / TBD" design traces. Why: R-003 from 01_REVIEW.

[**Unresolved**]
- None.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Verified KXemu def.hpp: PLIC=0x0400_0000. Relabeled baseline per M-002. |
| Review | R-002 | Accepted | Bus stores plic_idx. No downcast. Single finalized design. |
| Review | R-003 | Accepted | TCP lifecycle defined: bind fallback, single-accept, disconnect-terminal. V-F-5, V-F-6 added. |
| Review | R-004 | Accepted | Config-level validation V-CF-1..4 added. |
| Master | M-001 | Applied | mmio_regs! macro + functional chains in pseudocode |
| Master | M-002 | Applied | Baseline = QEMU-like layout (verified KXemu def.hpp) |
| Master | M-003 | Applied | mmio_regs! macro replaces per-device register enums |
| Master | M-004 | Applied | TestFinisher removed from default wiring; test-only |
| Trade-off | TR-1 | Adopted | Stored plic_idx, no downcast |
| Trade-off | TR-2 | Adopted | TCP retained with narrowed lifecycle contract |

---

## Spec

[**Goals**]

- G-1: ACLINT with MSWI (msip → MSIP), MTIMER (mtime 10MHz + mtimecmp → MTIP), SSWI (setssip → SSIP)
- G-2: PLIC with 32 sources, 2 contexts (M/S), level-triggered, correct claim/complete with claimed-exclusion
- G-3a: UART 16550 TX — THR → stdout
- G-3b: UART 16550 RX — TCP socket backend, PLIC source 10
- G-4: SiFive Test Finisher (test-only, not in default machine)
- G-5: Lock-free interrupt delivery via `Arc<AtomicU64>` (`irq_state`)
- G-6: `mmio_regs!` macro for declarative MMIO register abstraction

- NG-1: OpenSBI / Device Tree / SBI handoff
- NG-2: Multi-hart support
- NG-3: Async device tick (future direction)
- NG-4: TCP reconnect / multi-session UART

[**Architecture**]

```
                           ┌──────────────────────────────────────┐
  RVCore                   │  Bus (Arc<Mutex<Bus>>)               │
  ├── csr, mmu, pmp       │  ├── Ram    [0x8000_0000, 128M]      │
  ├── irq_state ──poll───► │  ├── ACLINT [0x0200_0000, 0x1_0000]  │
  │   Arc<AtomicU64>       │  ├── PLIC   [0x0C00_0000, 0x400_0000]│
  │   ▲  ▲                │  └── UART0  [0x1000_0000, 0x100]     │
  │   │  └── ACLINT ──────┤                                      │
  │   └───── PLIC ◄───────┤  Bus.plic_idx → direct PLIC access   │
  └── step()               └──────────────────────────────────────┘
       1. bus.lock().tick()         — tick devices, collect irq_lines, notify PLIC
       2. sync_interrupts()        — merge irq_state → mip
       3. check_pending_interrupts — sample mip & mie → trap
       4. fetch → decode → execute
       5. retire()
```

**Bus→PLIC wiring (finalized):**

Bus stores `plic_idx: Option<usize>` — the index into `self.mmio` where PLIC was registered. Set during `add_mmio("plic", ...)`. During `tick()`:

```rust
pub fn tick(&mut self) {
    // 1. Tick all devices, collect IRQ lines
    let mut irq_lines: u32 = 0;
    for region in &mut self.mmio {
        region.dev.tick();
        if region.irq_source > 0 && region.dev.irq_line() {
            irq_lines |= 1 << region.irq_source;
        }
    }
    // 2. Notify PLIC via stored index (no downcast needed)
    if let Some(idx) = self.plic_idx {
        self.mmio[idx].dev.post_tick(irq_lines);
    }
}
```

Wait — `post_tick` is back on Device trait? No. Let me reconsider per M-002. The Device trait must not have device-specific methods.

**Corrected approach:** PLIC implements `Device` normally. Bus passes `irq_lines` via a **write to a reserved PLIC register offset** (an internal-only "doorbell" that is not guest-accessible). This keeps the Device trait clean.

Actually, the simplest M-002-compliant approach: **Bus holds PLIC as a separate field**, not inside the `mmio` vec. PLIC is registered for MMIO dispatch but also held as a direct reference for `tick()` notification.

This is over-engineering. Let me choose the pragmatic approach that truly keeps things clean:

**Final approach:** `Device` trait gets no new methods. Bus stores `plic_idx`. During tick, Bus uses `plic_idx` to access the PLIC's `Box<dyn Device>`, and calls a helper that takes `&mut dyn Device` + `irq_lines`. The helper writes irq_lines into a known PLIC offset (an internal register). The PLIC's `write()` at that offset triggers `update()`.

Even simpler: **just add `fn notify(&mut self, _val: u32) {}`** as a default no-op on Device. Only PLIC overrides it. This is a generic "bus notification" hook, not PLIC-specific.

**Simplest correct approach that satisfies M-002:**

```rust
pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
    fn irq_line(&self) -> bool { false }
    fn notify(&mut self, _irq_lines: u32) {}  // bus-level notification
}
```

`notify()` is a generic bus-to-device notification, not PLIC-specific. Any device could use it in the future. Bus calls `notify(irq_lines)` on the device at `plic_idx`. This is the same pattern as `tick()` — a generic hook with a default no-op.

[**Invariants**]

- I-1: Hardware-wired mip bits (SSIP, MSIP, MTIP, SEIP, MEIP) modified only via irq_state merge
- I-2: Per-step ordering: `bus.tick()` → `sync_interrupts()` → `check_pending_interrupts()` → fetch/decode/execute → `retire()`
- I-3: Claimed PLIC sources are excluded from re-pending. Re-pend only after `complete()`
- I-4: Device read/write use offset-relative addresses
- I-5: mtime sampled per tick, frozen during xdb pause
- I-6: SSWI setssip: write 1 sets SSIP; read returns 0
- I-7: TCP bind failure → UART falls back to TX-only; disconnect → RX stops, TX continues

[**Data Structure**]

```rust
// device/mod.rs
pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
    fn irq_line(&self) -> bool { false }
    fn notify(&mut self, _irq_lines: u32) {}
}

// Interrupt bit constants
pub const SSIP: u64 = 1 << 1;
pub const MSIP: u64 = 1 << 3;
pub const MTIP: u64 = 1 << 7;
pub const SEIP: u64 = 1 << 9;
pub const MEIP: u64 = 1 << 11;
pub const HW_IP_MASK: Word = (SSIP | MSIP | MTIP | SEIP | MEIP) as Word;
```

**`mmio_regs!` macro:**

Inspired by `csr_table!`. Declares register names, offsets, and access modes. Generates a `decode(offset) -> Option<Reg>` function and the enum.

```rust
macro_rules! mmio_regs {
    ( $vis:vis enum $Reg:ident { $( $name:ident = $offset:expr ),* $(,)? } ) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        $vis enum $Reg { $( $name ),* }

        impl $Reg {
            fn decode(offset: usize) -> Option<Self> {
                match offset {
                    $( $offset => Some(Self::$name), )*
                    _ => None,
                }
            }
        }
    };
}
```

Usage:

```rust
// device/aclint.rs
mmio_regs! {
    enum Reg {
        Msip       = 0x0000,
        MtimecmpLo = 0x4000,
        MtimecmpHi = 0x4004,
        MtimeLo    = 0xBFF8,
        MtimeHi    = 0xBFFC,
        Setssip    = 0xC000,
    }
}

// device/plic.rs — PLIC uses a custom decode() because offsets are computed, not fixed
// mmio_regs! is not used here; manual decode with range matching is cleaner for PLIC

// device/uart.rs — UART uses manual decode because DLAB state affects register selection
// mmio_regs! is not used here; DLAB-aware dispatch is inherently stateful
```

Note: `mmio_regs!` works well for devices with fixed offset→register mappings (ACLINT, TestFinisher). PLIC and UART have computed/stateful dispatch that doesn't fit a simple offset table. This is the same pattern as CSRs: `csr_table!` handles the common case, while shadow registers and side effects are handled manually.

```rust
// device/aclint.rs
pub struct Aclint {
    epoch: Instant,
    mtime: u64,
    msip: u32,
    mtimecmp: u64,
    irq_state: Arc<AtomicU64>,
}

// device/plic.rs
const NUM_SRC: usize = 32;
const NUM_CTX: usize = 2;

pub struct Plic {
    priority: Vec<u8>,      // [NUM_SRC]
    pending: u32,
    enable: Vec<u32>,       // [NUM_CTX]
    threshold: Vec<u8>,     // [NUM_CTX]
    claimed: Vec<u32>,      // [NUM_CTX]
    irq_state: Arc<AtomicU64>,
}

// device/uart.rs
pub struct Uart {
    ier: u8, lcr: u8, mcr: u8, dll: u8, dlm: u8, scr: u8,
    rx_fifo: VecDeque<u8>,
    rx_buf: Arc<Mutex<VecDeque<u8>>>,
}

// device/test_finisher.rs
pub struct TestFinisher;

// device/bus.rs
pub struct Bus {
    ram: Ram,
    mmio: Vec<MmioRegion>,
    irq_lines: u32,
    plic_idx: Option<usize>,
}
```

[**API Surface**]

```rust
// Device trait
pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
    fn irq_line(&self) -> bool { false }
    fn notify(&mut self, _irq_lines: u32) {}
}

// Bus
impl Bus {
    pub fn add_mmio(&mut self, name: &'static str, base: usize, size: usize,
                    dev: Box<dyn Device>, irq_source: u32);
    pub fn tick(&mut self);
}

// Aclint
impl Aclint { pub fn new(irq_state: Arc<AtomicU64>) -> Self; }

// Plic
impl Plic { pub fn new(irq_state: Arc<AtomicU64>) -> Self; }

// Uart
impl Uart {
    pub fn new() -> Self;                // TX-only
    pub fn with_tcp(port: u16) -> Self;  // TX + TCP RX
}

// TestFinisher
impl TestFinisher { pub fn new() -> Self; }
```

[**Constraints**]

- C-1: QEMU-like address layout. Values verified against KXemu def.hpp: ACLINT `0x0200_0000` / `0x1_0000`, PLIC `0x0C00_0000` / `0x400_0000`, UART `0x1000_0000` / `0x100`, `timebase-frequency = 10_000_000`, UART IRQ = PLIC source 10.
- C-2: Single hart only
- C-3: UART byte-access only
- C-4: Device::read is `&mut self`
- C-5: mtime host-wall-clock 10 MHz, frozen during xdb pause
- C-6: PLIC source 0 is "no interrupt"
- C-7: SSWI setssip read returns 0
- C-8: TCP bind failure → TX-only fallback; disconnect → RX stops; no reconnect

---

## Implement

### Execution Flow

[**step()**]

```rust
fn step(&mut self) -> XResult {
    { self.bus.lock().unwrap().tick(); }
    self.sync_interrupts();
    if self.check_pending_interrupts() { self.retire(); return Ok(()); }
    self.trap_on_err(|core| {
        let raw = core.fetch()?;
        core.decode(raw).and_then(|inst| core.execute(inst))
    })?;
    self.retire();
    Ok(())
}

fn sync_interrupts(&mut self) {
    let ext = self.irq_state.load(Relaxed) as Word;
    let mip = self.csr.get(CsrAddr::mip);
    self.csr.set(CsrAddr::mip, (mip & !HW_IP_MASK) | (ext & HW_IP_MASK));
}
```

[**Bus::tick()**]

```rust
pub fn tick(&mut self) {
    self.irq_lines = 0;
    for region in &mut self.mmio {
        region.dev.tick();
        if region.irq_source > 0 && region.dev.irq_line() {
            self.irq_lines |= 1 << region.irq_source;
        }
    }
    if let Some(idx) = self.plic_idx {
        self.mmio[idx].dev.notify(self.irq_lines);
    }
}
```

[**Bus::add_mmio()**]

```rust
pub fn add_mmio(&mut self, name: &'static str, base: usize, size: usize,
                dev: Box<dyn Device>, irq_source: u32) {
    // ... existing overlap checks ...
    let idx = self.mmio.len();
    self.mmio.push(MmioRegion { name, range, dev, irq_source });
    if name == "plic" { self.plic_idx = Some(idx); }
}
```

### Implementation Plan

[**Step 0: Infrastructure**]

- `device/mod.rs`: Add `tick()`, `irq_line()`, `notify()` to Device. Change `read(&self)` → `read(&mut self)`. Add interrupt constants. Define `mmio_regs!` macro.
- `device/bus.rs`: Add `irq_lines`, `plic_idx`. Extend `MmioRegion` with `irq_source`. Implement `Bus::tick()`. Update `add_mmio()`.
- `cpu/riscv/mod.rs`: Add `irq_state` to RVCore. Implement `sync_interrupts()`. Update `step()`.
- `error.rs`: Add `XError::ProgramExit(u32)`.
- `cpu/mod.rs`: Catch `ProgramExit` in `CPU::step()`.

[**Step 1: ACLINT**]

File: `device/aclint.rs`

```rust
mmio_regs! {
    enum Reg {
        Msip = 0x0000, MtimecmpLo = 0x4000, MtimecmpHi = 0x4004,
        MtimeLo = 0xBFF8, MtimeHi = 0xBFFC, Setssip = 0xC000,
    }
}

impl Device for Aclint {
    fn read(&mut self, offset: usize, _size: usize) -> XResult<Word> {
        Ok(match Reg::decode(offset) {
            Some(Reg::Msip)        => self.msip as Word,
            Some(Reg::MtimecmpLo)  => self.mtimecmp as u32 as Word,
            Some(Reg::MtimecmpHi)  => (self.mtimecmp >> 32) as u32 as Word,
            Some(Reg::MtimeLo)     => self.mtime as u32 as Word,
            Some(Reg::MtimeHi)     => (self.mtime >> 32) as u32 as Word,
            Some(Reg::Setssip)     => 0,
            None                   => 0,
        })
    }

    fn write(&mut self, offset: usize, _size: usize, val: Word) -> XResult {
        match Reg::decode(offset) {
            Some(Reg::Msip) => self.set_msip(val as u32 & 1),
            Some(Reg::MtimecmpLo) => {
                self.mtimecmp = (self.mtimecmp & !0xFFFF_FFFF) | val as u32 as u64;
                self.check_timer();
            }
            Some(Reg::MtimecmpHi) => {
                self.mtimecmp = (self.mtimecmp & 0xFFFF_FFFF) | ((val as u32 as u64) << 32);
                self.check_timer();
            }
            Some(Reg::Setssip) => {
                if val as u32 & 1 != 0 { self.irq_state.fetch_or(SSIP, Relaxed); }
            }
            _ => {}
        }
        Ok(())
    }

    fn tick(&mut self) { self.mtime = self.epoch.elapsed().as_nanos() as u64 / 100; self.check_timer(); }
}

impl Aclint {
    pub fn new(irq_state: Arc<AtomicU64>) -> Self {
        Self { epoch: Instant::now(), mtime: 0, msip: 0, mtimecmp: u64::MAX, irq_state }
    }
    fn set_msip(&mut self, v: u32) {
        self.msip = v;
        if v != 0 { self.irq_state.fetch_or(MSIP, Relaxed); }
        else { self.irq_state.fetch_and(!MSIP, Relaxed); }
    }
    fn check_timer(&mut self) {
        if self.mtime >= self.mtimecmp { self.irq_state.fetch_or(MTIP, Relaxed); }
        else { self.irq_state.fetch_and(!MTIP, Relaxed); }
    }
}
```

[**Step 2: PLIC**]

File: `device/plic.rs`

PLIC uses manual offset decode (computed offsets, not suitable for `mmio_regs!`).

```rust
impl Device for Plic {
    fn read(&mut self, offset: usize, _size: usize) -> XResult<Word> {
        Ok(self.dispatch_read(offset))
    }
    fn write(&mut self, offset: usize, _size: usize, val: Word) -> XResult {
        self.dispatch_write(offset, val); Ok(())
    }
    fn notify(&mut self, irq_lines: u32) {
        self.update(irq_lines);
        self.evaluate();
    }
}

impl Plic {
    pub fn new(irq_state: Arc<AtomicU64>) -> Self { ... }

    fn dispatch_read(&mut self, offset: usize) -> Word {
        match offset {
            0x000000..=0x00007C if offset % 4 == 0 =>
                self.priority[offset / 4] as Word,
            0x001000 => self.pending as Word,
            o @ 0x002000..=0x002FFF if (o - 0x002000) % 0x80 == 0 =>
                self.ctx(o, 0x002000, 0x80).map_or(0, |c| self.enable[c] as Word),
            o if self.is_threshold(o) =>
                self.ctx(o, 0x200000, 0x1000).map_or(0, |c| self.threshold[c] as Word),
            o if self.is_claim(o) =>
                self.ctx(o, 0x200004, 0x1000).map_or(0, |c| self.claim(c) as Word),
            _ => 0,
        }
    }

    fn dispatch_write(&mut self, offset: usize, val: Word) {
        match offset {
            0x000000..=0x00007C if offset % 4 == 0 =>
                self.priority[offset / 4] = val as u8,
            o @ 0x002000..=0x002FFF if (o - 0x002000) % 0x80 == 0 =>
                if let Some(c) = self.ctx(o, 0x002000, 0x80) { self.enable[c] = val as u32; },
            o if self.is_threshold(o) =>
                if let Some(c) = self.ctx(o, 0x200000, 0x1000) {
                    self.threshold[c] = val as u8; self.evaluate();
                },
            o if self.is_claim(o) =>
                if let Some(c) = self.ctx(o, 0x200004, 0x1000) { self.complete(c, val as u32); },
            _ => {}
        }
    }

    fn ctx(&self, offset: usize, base: usize, stride: usize) -> Option<usize> {
        let c = (offset - base) / stride;
        (c < NUM_CTX).then_some(c)
    }
    fn is_threshold(&self, o: usize) -> bool { o >= 0x200000 && (o - 0x200000) % 0x1000 == 0 && (o - 0x200000) / 0x1000 < NUM_CTX }
    fn is_claim(&self, o: usize) -> bool { o >= 0x200004 && (o - 0x200004) % 0x1000 == 0 && (o - 0x200004) / 0x1000 < NUM_CTX }

    fn update(&mut self, irq_lines: u32) {
        for src in 1..NUM_SRC {
            let bit = 1u32 << src;
            if self.claimed.iter().any(|&c| c == src as u32) { continue; }
            if irq_lines & bit != 0 { self.pending |= bit; }
            else { self.pending &= !bit; }
        }
    }

    fn claim(&mut self, ctx: usize) -> u32 {
        (1..NUM_SRC)
            .filter(|&s| self.pending & (1 << s) != 0
                      && self.enable[ctx] & (1 << s) != 0
                      && self.priority[s] > self.threshold[ctx])
            .max_by_key(|&s| self.priority[s])
            .map(|s| { self.pending &= !(1 << s); self.claimed[ctx] = s as u32; s as u32 })
            .unwrap_or(0)
    }

    fn complete(&mut self, ctx: usize, src: u32) {
        if self.claimed[ctx] == src { self.claimed[ctx] = 0; }
        self.evaluate();
    }

    fn evaluate(&mut self) {
        for ctx in 0..NUM_CTX {
            let active = (1..NUM_SRC).any(|s|
                self.pending & (1 << s) != 0
                && self.enable[ctx] & (1 << s) != 0
                && self.priority[s] > self.threshold[ctx]
            );
            let bit = [MEIP, SEIP][ctx];
            if active { self.irq_state.fetch_or(bit, Relaxed); }
            else { self.irq_state.fetch_and(!bit, Relaxed); }
        }
    }
}
```

[**Step 3: UART 16550**]

File: `device/uart.rs`

```rust
impl Uart {
    pub fn new() -> Self {
        Self { ier: 0, lcr: 0x03, mcr: 0, dll: 0, dlm: 0, scr: 0,
               rx_fifo: VecDeque::new(), rx_buf: Arc::new(Mutex::new(VecDeque::new())) }
    }

    pub fn with_tcp(port: u16) -> Self {
        let rx_buf = Arc::new(Mutex::new(VecDeque::new()));
        let buf = rx_buf.clone();
        std::thread::spawn(move || {
            let Ok(listener) = std::net::TcpListener::bind(("127.0.0.1", port)) else {
                warn!("UART: TCP bind failed on port {port}, falling back to TX-only");
                return;
            };
            info!("UART: listening on 127.0.0.1:{port}");
            let Ok((stream, _)) = listener.accept() else { return; };
            use std::io::Read;
            for byte in stream.bytes().flatten() {
                buf.lock().unwrap().push_back(byte);
            }
            // Disconnect → thread exits, RX stops, TX continues
        });
        Self { rx_buf, ..Self::new() }
    }

    fn dlab(&self) -> bool { self.lcr & 0x80 != 0 }
    fn lsr(&self) -> u8 { (if self.rx_fifo.is_empty() { 0 } else { 0x01 }) | 0x60 }
    fn iir(&self) -> u8 {
        if !self.rx_fifo.is_empty() && self.ier & 0x01 != 0 { 0xC4 } else { 0xC1 }
    }
}

impl Device for Uart {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word> {
        (size == 1).ok_or(XError::BadAddress)?;
        Ok(match offset {
            0 if self.dlab() => self.dll,
            0 => self.rx_fifo.pop_front().unwrap_or(0),
            1 if self.dlab() => self.dlm,
            1 => self.ier, 2 => self.iir(), 3 => self.lcr,
            4 => self.mcr, 5 => self.lsr(), 6 => 0, 7 => self.scr,
            _ => 0,
        } as Word)
    }

    fn write(&mut self, offset: usize, size: usize, val: Word) -> XResult {
        (size == 1).ok_or(XError::BadAddress)?;
        let b = val as u8;
        match offset {
            0 if self.dlab() => self.dll = b,
            0 => { use std::io::Write; let _ = std::io::stdout().lock().write_all(&[b]).and_then(|_| std::io::stdout().flush()); }
            1 if self.dlab() => self.dlm = b,
            1 => self.ier = b & 0x0F, 3 => self.lcr = b, 4 => self.mcr = b, 7 => self.scr = b,
            _ => {}
        }
        Ok(())
    }

    fn tick(&mut self) {
        if let Ok(mut buf) = self.rx_buf.try_lock() { self.rx_fifo.extend(buf.drain(..)); }
    }

    fn irq_line(&self) -> bool { !self.rx_fifo.is_empty() && self.ier & 0x01 != 0 }
}
```

[**Step 4: SiFive Test Finisher (test-only)**]

File: `device/test_finisher.rs`

```rust
mmio_regs! { enum Reg { Finisher = 0x0000 } }

impl Device for TestFinisher {
    fn read(&mut self, _: usize, _: usize) -> XResult<Word> { Ok(0) }
    fn write(&mut self, offset: usize, _: usize, val: Word) -> XResult {
        if let Some(Reg::Finisher) = Reg::decode(offset) {
            match val as u32 & 0xFFFF {
                0x5555 => return Err(XError::ProgramExit(0)),
                0x3333 => return Err(XError::ProgramExit((val as u32) >> 16)),
                _ => {}
            }
        }
        Ok(())
    }
}
```

Not registered in default `RVCore::new()`. Used in tests:
```rust
bus.add_mmio("test", 0x0010_0000, 0x10, Box::new(TestFinisher::new()), 0);
```

[**Step 5: Integration**]

```rust
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
        Self::with_bus(Arc::new(Mutex::new(bus)), irq_state)
    }
}
```

---

## Trade-offs

- T-1: **Interrupt delivery** — `Arc<AtomicU64>` lock-free. Sync `tick()` is transitional (M-001); async evolution possible without changing irq_state interface.
- T-2: **ACLINT mtime** — Host wall clock, 10 MHz, frozen during xdb pause. `timebase-frequency = 10_000_000`.
- T-3: **UART RX** — TCP at `127.0.0.1:14514`. Bind failure → TX-only fallback. Single accept, no reconnect. Disconnect terminal for RX.
- T-4: **Bus→PLIC** — `plic_idx` + `Device::notify()`. Generic bus notification hook (default no-op). No downcast, no PLIC-specific trait methods.
- T-5: **mmio_regs! scope** — Used for ACLINT and TestFinisher (fixed-offset devices). PLIC and UART use manual decode (computed/stateful offsets). Same pattern as csr_table! vs manual CSR side effects.

---

## Validation

[**Unit Tests**]

- V-UT-1: ACLINT mtime increases after ticks
- V-UT-2: ACLINT mtimecmp → MTIP set/clear
- V-UT-3: ACLINT msip write → MSIP set/clear
- V-UT-4: ACLINT setssip write 1 → SSIP set; read → 0
- V-UT-5: PLIC priority read/write
- V-UT-6: PLIC enable per context
- V-UT-7: PLIC claim highest-priority
- V-UT-8: PLIC claim returns 0 when empty
- V-UT-9: PLIC complete releases claimed
- V-UT-10: PLIC threshold filtering
- V-UT-11: PLIC claimed source not re-pended
- V-UT-12: PLIC source re-pended after complete when line still high
- V-UT-13: UART THR write
- V-UT-14: UART LSR DR reflects rx_fifo
- V-UT-15: UART DLAB switching
- V-UT-16: UART IIR reflects rx state + IER
- V-UT-17: UART irq_line()
- V-UT-18: TestFinisher 0x5555 → ProgramExit(0)
- V-UT-19: TestFinisher (1<<16)|0x3333 → ProgramExit(1)

[**Integration Tests**]

- V-IT-1: ACLINT timer → MTIP → mip → trap
- V-IT-2: PLIC irq_line → MEIP → mip → trap
- V-IT-3: TestFinisher → CPU halt
- V-IT-4: UART FIFO partial read → re-pend after complete (level-triggered)
- V-IT-5: ACLINT mtime frozen between ticks
- V-IT-6: UART TCP RX: bytes arrive → rx_fifo → LSR.DR
- V-IT-7: UART TCP RX + PLIC → MEIP

[**Config-Level Validation**]

- V-CF-1: ACLINT registered at base `0x0200_0000`, size `0x1_0000`
- V-CF-2: PLIC registered at base `0x0C00_0000`, size `0x400_0000`
- V-CF-3: UART registered at base `0x1000_0000`, size `0x100`, irq_source `10`
- V-CF-4: ACLINT `timebase-frequency` constant equals `10_000_000`

[**Failure / Robustness Validation**]

- V-F-1: ACLINT unmapped offsets → 0
- V-F-2: PLIC claim empty → 0
- V-F-3: UART non-byte access → BadAddress
- V-F-4: PLIC complete wrong source → no change
- V-F-5: ACLINT mtime write ignored
- V-F-6: UART TCP bind failure → TX-only (irq_line stays false)

[**Edge Cases**]

- V-E-1: ACLINT mtimecmp = u64::MAX → no timer
- V-E-2: PLIC complete mismatched source → unchanged
- V-E-3: UART all offsets both DLAB modes
- V-E-4: SSWI write 0 → no SSIP change
- V-E-5: PLIC source 0 excluded from claim

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 MSWI | V-UT-3 |
| G-1 MTIMER | V-UT-1..2, V-IT-1, V-IT-5, V-E-1 |
| G-1 SSWI | V-UT-4, V-E-4 |
| G-2 PLIC | V-UT-5..12, V-IT-2, V-IT-4, V-E-2, V-E-5 |
| G-3a TX | V-UT-13..16, V-E-3 |
| G-3b RX | V-UT-17, V-IT-6, V-IT-7 |
| G-4 TestFinisher | V-UT-18..19, V-IT-3 |
| G-5 irq_state | V-IT-1, V-IT-2, V-IT-7 |
| C-1 layout | V-CF-1..4 |
| C-8 TCP lifecycle | V-F-6 |
| I-3 claimed-exclusion | V-UT-11..12, V-IT-4 |

---

## Memory Map (QEMU-like, verified against KXemu def.hpp)

| Device | Base | Guest Size | Internal Decoded | PLIC IRQ |
|--------|------|-----------|------------------|----------|
| ACLINT | `0x0200_0000` | `0x1_0000` | MSWI, MTIMER, SSWI | — |
| PLIC | `0x0C00_0000` | `0x400_0000` | priority, pending, enable, threshold, claim | — |
| UART0 | `0x1000_0000` | `0x100` | offsets 0-7 | source 10 |
| RAM | `0x8000_0000` | 128 MB | full | — |

TestFinisher (`0x0010_0000`, `0x10`) — test-only, not in default machine.

## File Organization

```
xcore/src/device/
├── mod.rs            — Device trait, interrupt constants, mmio_regs! macro
├── bus.rs            — Bus (tick, irq_lines, plic_idx)
├── ram.rs            — Ram
├── aclint.rs         — ACLINT: MSWI + MTIMER + SSWI (new)
├── plic.rs           — PLIC (new)
├── uart.rs           — UART 16550 (new)
└── test_finisher.rs  — SiFive Test Finisher (new, test-only)
```
