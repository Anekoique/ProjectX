# `Device Emulation` PLAN `04`

> Status: Revised
> Feature: `dev`
> Iteration: `04`
> Owner: Executor
> Depends on:
> - Previous Plan: `03_PLAN.md`
> - Review: `03_REVIEW.md`
> - Master Directive: `03_MASTER.md`

---

## Summary

Phase 4 device emulation: ACLINT, PLIC, UART 16550 (TX + TCP RX), SiFive Test Finisher (test-only).

This round eliminates all prior ambiguities:
- Compatibility baseline: **xemu internal layout** (QEMU-like, no external DTS claim).
- Bus→PLIC: **`plic_idx` + `Device::notify()`**. `notify()` is an explicit, documented, generic Device trait hook.
- TCP lifecycle: contract narrowed to validated scope only.
- `mmio_regs!`: fixed-offset helper, not a unified abstraction.

## Log

[**Review Adjustments**]

- R-001 (baseline split): Resolved. Dropped all "KXemu DTS" / "verified against" claims. Baseline is now **xemu's own layout constants**, described as QEMU-like for orientation only. No external contract claimed.
- R-002 (Bus→PLIC inconsistency): Resolved. `Device::notify()` is the final API. Explicitly documented as a generic bus-to-device hook. All prior deliberation traces removed. Entire document is consistent.
- R-003 (TCP lifecycle validation): Resolved. Contract narrowed: this round validates bind-failure fallback and single-accept happy path only. Disconnect/reconnect behavior is documented as defined-but-not-validated (future scope).
- R-004 (mmio_regs! overstated): Resolved. Relabeled as "fixed-offset MMIO helper". PLIC/UART manual decode is a deliberate design choice.

[**Master Compliance**]

- M-001 (code quality): Applied. Functional chains throughout pseudocode.
- M-002 (QEMU-like baseline): Applied. Layout described as "QEMU-like" without external DTS backing.
- M-003 (mmio_regs! semantics): Applied. Renamed scope: "fixed-offset register helper for simple devices".

### Changes from Previous Round

[**Changed**]
- Baseline: "verified against KXemu def.hpp" → "xemu internal layout (QEMU-like)". Why: R-001.
- `Device::notify()`: now explicitly listed in trait definition, API Surface, Architecture, and Trade-offs as the chosen mechanism. All "no extra trait hook" claims removed. Why: R-002.
- TCP contract: narrowed to bind-fallback + single-accept. Disconnect/reconnect listed as defined-but-unvalidated. Why: R-003.
- `mmio_regs!`: "unified declarative approach" → "fixed-offset helper for ACLINT/TestFinisher". Why: R-004 + M-003.

[**Removed**]
- All "KXemu DTS" / "verified against" external contract claims. Why: R-001.
- All prior deliberation traces (post_tick, doorbell, separate PLIC field, etc.). Why: R-002.

[**Unresolved**]
- TCP disconnect/reconnect validation deferred to future scope.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Baseline = xemu internal layout, QEMU-like orientation only |
| Review | R-002 | Accepted | `notify()` is the final API, explicitly documented everywhere |
| Review | R-003 | Accepted | TCP contract narrowed to validated scope |
| Review | R-004 | Accepted | mmio_regs! = fixed-offset helper, not unified abstraction |
| Master | M-001 | Applied | Functional chains in pseudocode |
| Master | M-002 | Applied | QEMU-like baseline, no external DTS claim |
| Master | M-003 | Applied | mmio_regs! relabeled as fixed-offset helper |
| Trade-off | TR-1 | Adopted | TCP contract narrowed |
| Trade-off | TR-2 | Adopted | notify() explicitly owned as final API |

---

## Spec

[**Goals**]

- G-1: ACLINT (MSWI + MTIMER + SSWI)
- G-2: PLIC (32 sources, 2 contexts, level-triggered, claimed-exclusion)
- G-3a: UART TX (THR → stdout)
- G-3b: UART RX (TCP socket, PLIC source 10)
- G-4: SiFive Test Finisher (test-only)
- G-5: `Arc<AtomicU64>` interrupt delivery
- G-6: `mmio_regs!` fixed-offset helper for simple devices

- NG-1: OpenSBI / DT / SBI handoff
- NG-2: Multi-hart
- NG-3: Async tick
- NG-4: TCP reconnect

[**Architecture**]

```
  RVCore                    Bus (Arc<Mutex<Bus>>)
  ├── irq_state ──poll──►  ├── Ram    [0x8000_0000]
  │   Arc<AtomicU64>       ├── ACLINT [0x0200_0000]  ──► irq_state (MSIP/MTIP/SSIP)
  │   ▲                    ├── PLIC   [0x0C00_0000]  ──► irq_state (MEIP/SEIP)
  │   └── ACLINT, PLIC     ├── UART0  [0x1000_0000]  ──► irq_line() → PLIC
  │                         └── (Test) [0x0010_0000]  test-only
  └── step()
       1. bus.tick()
       2. sync_interrupts()
       3. check_pending_interrupts()
       4. fetch/decode/execute
       5. retire()
```

**Device trait (final):**

```rust
pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}           // per-step state update
    fn irq_line(&self) -> bool { false }  // level-triggered IRQ output
    fn notify(&mut self, _irq_lines: u32) {}  // bus-level notification (used by PLIC)
}
```

`notify()` is a generic bus-to-device hook. Default no-op. Only PLIC overrides it. Bus calls `notify(irq_lines)` on the device at `plic_idx` after collecting IRQ lines from all devices.

**Bus::tick() (final):**

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

[**Invariants**]

- I-1: mip hardware bits modified only via irq_state merge
- I-2: Ordering: tick → sync → check → fetch/execute → retire
- I-3: Claimed PLIC sources excluded from re-pending until complete
- I-4: Devices use offset-relative addresses
- I-5: mtime frozen during xdb pause
- I-6: SSWI: write 1 sets SSIP; read returns 0
- I-7: TCP bind failure → TX-only; disconnect → RX stops

[**Data Structure**]

```rust
// device/mod.rs — interrupt constants
pub const SSIP: u64 = 1 << 1;
pub const MSIP: u64 = 1 << 3;
pub const MTIP: u64 = 1 << 7;
pub const SEIP: u64 = 1 << 9;
pub const MEIP: u64 = 1 << 11;
pub const HW_IP_MASK: Word = (SSIP | MSIP | MTIP | SEIP | MEIP) as Word;

// mmio_regs! — fixed-offset helper for simple devices (ACLINT, TestFinisher)
// PLIC and UART use manual decode (computed/stateful offsets — deliberate choice)
macro_rules! mmio_regs {
    ( $vis:vis enum $Reg:ident { $( $name:ident = $offset:expr ),* $(,)? } ) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        $vis enum $Reg { $( $name ),* }
        impl $Reg {
            fn decode(offset: usize) -> Option<Self> {
                match offset { $( $offset => Some(Self::$name), )* _ => None }
            }
        }
    };
}
```

```rust
// device/bus.rs
pub struct Bus {
    ram: Ram,
    mmio: Vec<MmioRegion>,
    plic_idx: Option<usize>,
}
struct MmioRegion {
    name: &'static str,
    range: Range<usize>,
    dev: Box<dyn Device>,
    irq_source: u32,
}

// device/aclint.rs
pub struct Aclint {
    epoch: Instant, mtime: u64, msip: u32, mtimecmp: u64,
    irq_state: Arc<AtomicU64>,
}

// device/plic.rs
pub struct Plic {
    priority: Vec<u8>, pending: u32, enable: Vec<u32>,
    threshold: Vec<u8>, claimed: Vec<u32>,
    irq_state: Arc<AtomicU64>,
}

// device/uart.rs
pub struct Uart {
    ier: u8, lcr: u8, mcr: u8, dll: u8, dlm: u8, scr: u8,
    rx_fifo: VecDeque<u8>, rx_buf: Arc<Mutex<VecDeque<u8>>>,
}

// device/test_finisher.rs
pub struct TestFinisher;
```

[**API Surface**]

```rust
pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
    fn irq_line(&self) -> bool { false }
    fn notify(&mut self, _irq_lines: u32) {}
}

impl Bus {
    pub fn add_mmio(&mut self, name: &'static str, base: usize, size: usize,
                    dev: Box<dyn Device>, irq_source: u32);
    pub fn tick(&mut self);
}
impl Aclint { pub fn new(irq_state: Arc<AtomicU64>) -> Self; }
impl Plic   { pub fn new(irq_state: Arc<AtomicU64>) -> Self; }
impl Uart   { pub fn new() -> Self; pub fn with_tcp(port: u16) -> Self; }
impl TestFinisher { pub fn new() -> Self; }
```

[**Constraints**]

- C-1: xemu internal layout (QEMU-like): ACLINT `0x0200_0000`/`0x1_0000`, PLIC `0x0C00_0000`/`0x400_0000`, UART `0x1000_0000`/`0x100`, `timebase-frequency=10_000_000`, UART IRQ=source 10
- C-2: Single hart
- C-3: UART byte-access only
- C-4: `Device::read(&mut self)`
- C-5: mtime host 10MHz, frozen during xdb pause
- C-6: PLIC source 0 = no interrupt
- C-7: SSWI read returns 0
- C-8: TCP: bind failure → TX-only fallback; disconnect → RX stops; no reconnect (disconnect not validated this round)

---

## Implement

### Execution Flow

```rust
fn step(&mut self) -> XResult {
    { self.bus.lock().unwrap().tick(); }
    self.sync_interrupts();
    if self.check_pending_interrupts() { self.retire(); return Ok(()); }
    self.trap_on_err(|core| core.decode(core.fetch()?).and_then(|i| core.execute(i)))?;
    self.retire();
    Ok(())
}

fn sync_interrupts(&mut self) {
    let ext = self.irq_state.load(Relaxed) as Word;
    let mip = self.csr.get(CsrAddr::mip);
    self.csr.set(CsrAddr::mip, (mip & !HW_IP_MASK) | (ext & HW_IP_MASK));
}
```

### Implementation Plan

[**Step 0: Infrastructure**]

- `device/mod.rs`: `Device` trait with `read(&mut self)`, `write`, `tick`, `irq_line`, `notify`. Interrupt constants. `mmio_regs!` macro.
- `device/bus.rs`: `MmioRegion.irq_source`, `Bus.plic_idx`. `add_mmio()` sets `plic_idx` when `name == "plic"`. `Bus::tick()` as shown above.
- `cpu/riscv/mod.rs`: `irq_state: Arc<AtomicU64>` in RVCore. `sync_interrupts()`. Updated `step()`.
- `error.rs`: `XError::ProgramExit(u32)`.
- `cpu/mod.rs`: Catch `ProgramExit`.

[**Step 1: ACLINT**]

```rust
mmio_regs! { enum Reg { Msip=0x0000, MtimecmpLo=0x4000, MtimecmpHi=0x4004,
                         MtimeLo=0xBFF8, MtimeHi=0xBFFC, Setssip=0xC000 } }

impl Device for Aclint {
    fn read(&mut self, offset: usize, _: usize) -> XResult<Word> {
        Ok(match Reg::decode(offset) {
            Some(Reg::Msip)        => self.msip as Word,
            Some(Reg::MtimecmpLo)  => self.mtimecmp as u32 as Word,
            Some(Reg::MtimecmpHi)  => (self.mtimecmp >> 32) as u32 as Word,
            Some(Reg::MtimeLo)     => self.mtime as u32 as Word,
            Some(Reg::MtimeHi)     => (self.mtime >> 32) as u32 as Word,
            Some(Reg::Setssip)     => 0,
            None => 0,
        })
    }
    fn write(&mut self, offset: usize, _: usize, v: Word) -> XResult {
        match Reg::decode(offset) {
            Some(Reg::Msip) => self.set_msip(v as u32 & 1),
            Some(Reg::MtimecmpLo) => { self.mtimecmp = (self.mtimecmp & !0xFFFF_FFFF) | v as u32 as u64; self.check_timer(); }
            Some(Reg::MtimecmpHi) => { self.mtimecmp = (self.mtimecmp & 0xFFFF_FFFF) | ((v as u32 as u64) << 32); self.check_timer(); }
            Some(Reg::Setssip) => { if v as u32 & 1 != 0 { self.irq_state.fetch_or(SSIP, Relaxed); } }
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

```rust
const NUM_SRC: usize = 32;
const NUM_CTX: usize = 2;

impl Device for Plic {
    fn read(&mut self, off: usize, _: usize) -> XResult<Word> { Ok(self.dispatch_read(off)) }
    fn write(&mut self, off: usize, _: usize, v: Word) -> XResult { self.dispatch_write(off, v); Ok(()) }
    fn notify(&mut self, irq_lines: u32) { self.update(irq_lines); self.evaluate(); }
}
impl Plic {
    pub fn new(irq_state: Arc<AtomicU64>) -> Self {
        Self { priority: vec![0; NUM_SRC], pending: 0, enable: vec![0; NUM_CTX],
               threshold: vec![0; NUM_CTX], claimed: vec![0; NUM_CTX], irq_state }
    }
    fn ctx(&self, off: usize, base: usize, stride: usize) -> Option<usize> {
        let c = (off - base) / stride; (c < NUM_CTX).then_some(c)
    }
    fn dispatch_read(&mut self, off: usize) -> Word {
        match off {
            0..=0x7C if off % 4 == 0     => self.priority[off / 4] as Word,
            0x1000                        => self.pending as Word,
            o @ 0x2000..=0x20FF if (o - 0x2000) % 0x80 == 0 =>
                self.ctx(o, 0x2000, 0x80).map_or(0, |c| self.enable[c] as Word),
            o if o >= 0x200000 && o < 0x200000 + NUM_CTX * 0x1000 && o % 0x1000 == 0 =>
                self.ctx(o, 0x200000, 0x1000).map_or(0, |c| self.threshold[c] as Word),
            o if o >= 0x200004 && o < 0x200004 + NUM_CTX * 0x1000 && (o - 4) % 0x1000 == 0 =>
                self.ctx(o, 0x200004, 0x1000).map_or(0, |c| self.claim(c) as Word),
            _ => 0,
        }
    }
    fn dispatch_write(&mut self, off: usize, v: Word) {
        match off {
            0..=0x7C if off % 4 == 0     => self.priority[off / 4] = v as u8,
            o @ 0x2000..=0x20FF if (o - 0x2000) % 0x80 == 0 =>
                if let Some(c) = self.ctx(o, 0x2000, 0x80) { self.enable[c] = v as u32; },
            o if o >= 0x200000 && o < 0x200000 + NUM_CTX * 0x1000 && o % 0x1000 == 0 =>
                if let Some(c) = self.ctx(o, 0x200000, 0x1000) { self.threshold[c] = v as u8; self.evaluate(); },
            o if o >= 0x200004 && o < 0x200004 + NUM_CTX * 0x1000 && (o - 4) % 0x1000 == 0 =>
                if let Some(c) = self.ctx(o, 0x200004, 0x1000) { self.complete(c, v as u32); },
            _ => {}
        }
    }
    fn update(&mut self, lines: u32) {
        for s in 1..NUM_SRC {
            let bit = 1u32 << s;
            if self.claimed.iter().any(|&c| c == s as u32) { continue; }
            if lines & bit != 0 { self.pending |= bit; } else { self.pending &= !bit; }
        }
    }
    fn claim(&mut self, ctx: usize) -> u32 {
        (1..NUM_SRC)
            .filter(|&s| self.pending & (1 << s) != 0 && self.enable[ctx] & (1 << s) != 0
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
                self.pending & (1 << s) != 0 && self.enable[ctx] & (1 << s) != 0
                && self.priority[s] > self.threshold[ctx]);
            let bit = [MEIP, SEIP][ctx];
            if active { self.irq_state.fetch_or(bit, Relaxed); }
            else { self.irq_state.fetch_and(!bit, Relaxed); }
        }
    }
}
```

[**Step 3: UART 16550**]

```rust
impl Uart {
    pub fn new() -> Self {
        Self { ier: 0, lcr: 0x03, mcr: 0, dll: 0, dlm: 0, scr: 0,
               rx_fifo: VecDeque::new(), rx_buf: Arc::new(Mutex::new(VecDeque::new())) }
    }
    pub fn with_tcp(port: u16) -> Self {
        let buf = Arc::new(Mutex::new(VecDeque::<u8>::new()));
        let rx = buf.clone();
        std::thread::spawn(move || {
            let Ok(l) = std::net::TcpListener::bind(("127.0.0.1", port)) else {
                warn!("UART: bind failed on {port}, TX-only"); return;
            };
            info!("UART: listening on 127.0.0.1:{port}");
            let Ok((s, _)) = l.accept() else { return; };
            use std::io::Read;
            for b in s.bytes().flatten() { rx.lock().unwrap().push_back(b); }
        });
        Self { rx_buf: buf, ..Self::new() }
    }
    fn dlab(&self) -> bool { self.lcr & 0x80 != 0 }
    fn lsr(&self) -> u8 { (if self.rx_fifo.is_empty() { 0 } else { 0x01 }) | 0x60 }
    fn iir(&self) -> u8 { if !self.rx_fifo.is_empty() && self.ier & 1 != 0 { 0xC4 } else { 0xC1 } }
}
impl Device for Uart {
    fn read(&mut self, off: usize, sz: usize) -> XResult<Word> {
        (sz == 1).ok_or(XError::BadAddress)?;
        Ok(match off {
            0 if self.dlab() => self.dll, 0 => self.rx_fifo.pop_front().unwrap_or(0),
            1 if self.dlab() => self.dlm, 1 => self.ier,
            2 => self.iir(), 3 => self.lcr, 4 => self.mcr, 5 => self.lsr(), 6 => 0, 7 => self.scr,
            _ => 0,
        } as Word)
    }
    fn write(&mut self, off: usize, sz: usize, v: Word) -> XResult {
        (sz == 1).ok_or(XError::BadAddress)?;
        let b = v as u8;
        match off {
            0 if self.dlab() => self.dll = b,
            0 => { use std::io::Write; let _ = std::io::stdout().lock().write_all(&[b]).and_then(|_| std::io::stdout().flush()); }
            1 if self.dlab() => self.dlm = b,
            1 => self.ier = b & 0x0F, 3 => self.lcr = b, 4 => self.mcr = b, 7 => self.scr = b,
            _ => {}
        }
        Ok(())
    }
    fn tick(&mut self) { if let Ok(mut b) = self.rx_buf.try_lock() { self.rx_fifo.extend(b.drain(..)); } }
    fn irq_line(&self) -> bool { !self.rx_fifo.is_empty() && self.ier & 1 != 0 }
}
```

[**Step 4: TestFinisher (test-only)**]

```rust
mmio_regs! { enum Reg { Finisher = 0x0000 } }
impl Device for TestFinisher {
    fn read(&mut self, _: usize, _: usize) -> XResult<Word> { Ok(0) }
    fn write(&mut self, off: usize, _: usize, v: Word) -> XResult {
        if let Some(Reg::Finisher) = Reg::decode(off) {
            match v as u32 & 0xFFFF {
                0x5555 => return Err(XError::ProgramExit(0)),
                0x3333 => return Err(XError::ProgramExit((v as u32) >> 16)),
                _ => {}
            }
        }
        Ok(())
    }
}
```

[**Step 5: Wiring**]

```rust
impl RVCore {
    pub fn new() -> Self {
        let irq_state = Arc::new(AtomicU64::new(0));
        let mut bus = Bus::new(CONFIG_MBASE, CONFIG_MSIZE);
        bus.add_mmio("aclint", 0x0200_0000, 0x1_0000, Box::new(Aclint::new(irq_state.clone())), 0);
        bus.add_mmio("plic",   0x0C00_0000, 0x400_0000, Box::new(Plic::new(irq_state.clone())), 0);
        bus.add_mmio("uart0",  0x1000_0000, 0x100, Box::new(Uart::with_tcp(14514)), 10);
        Self::with_bus(Arc::new(Mutex::new(bus)), irq_state)
    }
}
```

---

## Trade-offs

- T-1: `Arc<AtomicU64>` lock-free interrupt delivery. Sync tick transitional.
- T-2: mtime host 10MHz, frozen during xdb pause. `timebase-frequency=10_000_000`.
- T-3: UART TCP at `127.0.0.1:14514`. Bind failure → TX-only. Single accept. Disconnect → RX stops. (Disconnect not validated this round.)
- T-4: Bus→PLIC via `plic_idx + Device::notify()`. `notify()` is a generic Device trait hook (default no-op). Only PLIC overrides it. This adds one method to the trait surface — accepted as the minimal, clean mechanism that avoids downcast, cross-device references, and PLIC-specific Bus logic.
- T-5: `mmio_regs!` is a fixed-offset helper for simple devices (ACLINT, TestFinisher). PLIC/UART use manual decode because their offsets are computed or state-dependent.

---

## Validation

[**Unit Tests**]

- V-UT-1..4: ACLINT mtime, mtimecmp→MTIP, msip→MSIP, setssip→SSIP
- V-UT-5..12: PLIC priority, enable, claim, complete, threshold, claimed-exclusion, re-pend
- V-UT-13..17: UART THR, LSR, DLAB, IIR, irq_line
- V-UT-18..19: TestFinisher 0x5555, 0x3333

[**Integration Tests**]

- V-IT-1: ACLINT timer → MTIP → trap
- V-IT-2: PLIC irq_line → MEIP → trap
- V-IT-3: TestFinisher → CPU halt
- V-IT-4: UART FIFO partial read → re-pend (level-triggered)
- V-IT-5: mtime frozen between ticks
- V-IT-6: TCP RX bytes → rx_fifo → LSR.DR
- V-IT-7: TCP RX + PLIC → MEIP

[**Config-Level**]

- V-CF-1..3: Verify base/size/irq_source of ACLINT, PLIC, UART match C-1 constants
- V-CF-4: `timebase-frequency` constant = 10_000_000

[**Failure / Robustness**]

- V-F-1: ACLINT unmapped → 0
- V-F-2: PLIC claim empty → 0
- V-F-3: UART non-byte → BadAddress
- V-F-4: PLIC complete wrong source → no change
- V-F-5: ACLINT mtime write ignored
- V-F-6: TCP bind failure → TX-only

[**Edge Cases**]

- V-E-1: mtimecmp=MAX → no timer
- V-E-2: PLIC complete mismatch → unchanged
- V-E-3: UART all offsets both DLAB
- V-E-4: SSWI write 0 → no SSIP
- V-E-5: PLIC source 0 excluded

[**Acceptance Mapping**]

| Goal | Validation |
|------|------------|
| G-1 MSWI/MTIMER/SSWI | V-UT-1..4, V-IT-1, V-IT-5, V-E-1, V-E-4 |
| G-2 PLIC | V-UT-5..12, V-IT-2, V-IT-4, V-E-2, V-E-5 |
| G-3a TX | V-UT-13..16, V-E-3 |
| G-3b RX | V-UT-17, V-IT-6, V-IT-7 |
| G-4 Test | V-UT-18..19, V-IT-3 |
| G-5 irq_state | V-IT-1, V-IT-2, V-IT-7 |
| C-1 layout | V-CF-1..4 |
| C-8 TCP | V-F-6 |
| I-3 claimed | V-UT-11..12, V-IT-4 |

---

## Memory Map (xemu internal, QEMU-like)

| Device | Base | Size | PLIC IRQ |
|--------|------|------|----------|
| ACLINT | `0x0200_0000` | `0x1_0000` | — |
| PLIC | `0x0C00_0000` | `0x400_0000` | — |
| UART0 | `0x1000_0000` | `0x100` | 10 |
| RAM | `0x8000_0000` | 128 MB | — |

TestFinisher `0x0010_0000`/`0x10` — test-only.

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
