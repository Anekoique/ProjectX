# `Device Emulation` PLAN `01`

> Status: Revised
> Feature: `dev`
> Iteration: `01`
> Owner: Executor
> Depends on:
> - Previous Plan: `00_PLAN.md`
> - Review: `00_REVIEW.md`
> - Master Directive: `00_MASTER.md`

---

## Summary

Implement Phase 4 device emulation: CLINT, PLIC, UART 16550, and SiFive Test Finisher.
QEMU virt / SiFive-compatible memory map with guest-visible region sizes aligned to DTS conventions.
`Arc<AtomicU64>` lock-free interrupt delivery with **level-triggered** device IRQ lines.
UART RX via TCP backend (avoids xdb stdin conflict). UART TX-only in Phase 4A; RX in Phase 4B.

## Log

[**Feature Introduce**]

- Level-triggered IRQ model: `DeviceIrq` replaced by per-device `irq_line` method on Device trait. PLIC re-samples device state every `tick()` instead of consuming one-shot event bits.
- UART dual-phase: Phase 4A = TX-only (stdout); Phase 4B = RX via TCP socket backend, decoupled from xdb stdin.
- Guest-visible MMIO sizes aligned to qemu-virt DTS: UART `0x100`, PLIC `0x400_0000`, CLINT `0x1_0000`.
- Detailed function-level design for each device (CLINT read/write dispatch, PLIC claim/complete logic, UART register bank switching).
- CLINT mtime debugger semantics: mtime uses host wall clock but only advances between `step()` calls (no drift during xdb pause).

[**Review Adjustments**]

- R-001 (UART stdin conflict): Resolved. Phase 4A is TX-only. Phase 4B uses TCP socket for RX, never occupies xdb stdin.
- R-002 (level-triggered IRQ): Resolved. Replaced `DeviceIrq` atomic with `Device::irq_line()` trait method. PLIC re-samples all devices every tick — no one-shot event consumption.
- R-003 (MMIO shape alignment): Resolved. Guest-visible sizes now match qemu-virt DTS conventions. Internal decoded subset is documented separately.
- R-004 (validation gaps): Resolved. Added V-IT-4 (UART FIFO partial read re-claim), V-IT-5 (CLINT pause semantics), V-E-4 (UART xdb mode no stdin capture).

[**Master Compliance**]

- M-001 (no ExtIp/DeviceIrq type aliases): Applied. Removed type aliases, use `Arc<AtomicU64>` and `Device::irq_line()` directly.
- M-002 (replace raw arrays with Vec/proper structures): Applied. PLIC uses `Vec<u8>` for priority, `Vec<u32>` for enable, `Vec<u8>` for threshold, `Vec<u32>` for claimed.
- M-003 (TCP/PTY for UART RX): Applied. Phase 4B UART RX uses TCP socket backend.
- M-004 (more function detail): Applied. Each device now has full function-level pseudocode for read/write/tick dispatch, claim/complete logic, and register bank switching.

### Changes from Previous Round

[**Added**]
- `Device::irq_line(&self) -> bool` trait method for level-triggered IRQ
- Phase 4A / 4B split for UART (TX-only first, then TCP RX)
- Function-level pseudocode for CLINT, PLIC, UART, TestFinisher
- CLINT mtime snapshot model (frozen during xdb pause)
- Validation items V-IT-4, V-IT-5, V-E-4

[**Changed**]
- `DeviceIrq` (one-shot atomic) → `Device::irq_line()` (level-triggered). Why: one-shot model loses UART RX interrupts after partial FIFO drain.
- UART region size 8B → `0x100`. Why: qemu-virt DTS compatibility.
- PLIC priority/enable/threshold arrays → `Vec`. Why: M-002 directive.
- Removed `ExtIp` and `DeviceIrq` type aliases. Why: M-001 directive.

[**Removed**]
- `DeviceIrq` type alias and `Arc<AtomicU32>` device-pending atomic. Why: replaced by `Device::irq_line()`.
- UART stdin background thread from Phase 4A scope. Why: R-001 conflict with xdb.

[**Unresolved**]
- None. All blocking issues resolved.

### Response Matrix

| Source | ID | Decision | Resolution |
|--------|----|----------|------------|
| Review | R-001 | Accepted | Phase 4A TX-only; Phase 4B RX via TCP socket, never captures xdb stdin |
| Review | R-002 | Accepted | Replaced DeviceIrq with Device::irq_line(); PLIC re-samples every tick |
| Review | R-003 | Accepted | Guest-visible sizes aligned: UART 0x100, PLIC 0x400_0000, CLINT 0x1_0000 |
| Review | R-004 | Accepted | Added V-IT-4 (FIFO partial re-claim), V-IT-5 (pause semantics), V-E-4 (xdb mode) |
| Master | M-001 | Applied | Removed ExtIp and DeviceIrq type aliases |
| Master | M-002 | Applied | PLIC uses Vec instead of raw arrays |
| Master | M-003 | Applied | UART RX uses TCP backend in Phase 4B |
| Master | M-004 | Applied | Full function-level pseudocode for all devices |
| Trade-off | TR-1 | Adopted | Phase 4A TX-only; Phase 4B TCP RX |
| Trade-off | TR-2 | Justified | Host wall clock with snapshot model — mtime frozen during xdb pause; timebase-frequency = 10_000_000 |

---

## Spec

[**Goals**]

- G-1: CLINT with mtime (host 10MHz, snapshot model), mtimecmp, msip — drives MTIP/MSIP
- G-2: PLIC with 32 sources, 2 contexts (M/S), priority/pending/enable/threshold/claim/complete — drives MEIP/SEIP
- G-3a: UART 16550 TX-only (Phase 4A) — THR → stdout, LSR/IIR/LCR register model
- G-3b: UART 16550 RX via TCP (Phase 4B) — TCP socket backend, PLIC source 10
- G-4: SiFive Test Finisher for bare-metal test exit signaling
- G-5: Lock-free interrupt delivery via `Arc<AtomicU64>` shared between CPU and devices

- NG-1: OpenSBI / Device Tree / SBI handoff (future phase)
- NG-2: Multi-hart support (single hart only)
- NG-3: DMA or scatter-gather I/O
- NG-4: UART RX via raw stdin (conflicts with xdb)

[**Architecture**]

```
                          ┌─────────────────────────────────────┐
  RVCore                  │  Bus (Arc<Mutex<Bus>>)              │
  ├── csr, mmu, pmp      │  ├── Ram   [0x8000_0000, 128M]      │
  ├── ext_ip ────poll───► │  ├── CLINT [0x0200_0000, 0x1_0000]  │
  │   Arc<AtomicU64>      │  ├── PLIC  [0x0C00_0000, 0x400_0000]│
  │   ▲  ▲               │  ├── UART0 [0x1000_0000, 0x100]     │
  │   │  └─── CLINT ─────┤  └── Test  [0x0010_0000, 0x10]      │
  │   └────── PLIC ───┐  └─────────────────────────────────────┘
  │                    │
  │   PLIC samples irq_line() on each device during tick()
  │   CLINT directly sets MTIP/MSIP in ext_ip
  │
  └── step()
       1. bus.lock().tick()           — devices update state
       2. sync_external_interrupts()  — merge ext_ip → mip
       3. check_pending_interrupts()  — sample mip & mie → trap
       4. fetch → decode → execute
       5. retire()
```

**Interrupt delivery (CPU ← devices):**

Two channels share `Arc<AtomicU64>` (`ext_ip`), each bit = mip bit position:

| Bit | Interrupt | Writer |
|-----|-----------|--------|
| 3   | MSIP      | CLINT (on msip register write) |
| 7   | MTIP      | CLINT (on tick: mtime >= mtimecmp) |
| 9   | SEIP      | PLIC (on tick: S-mode context has qualified pending) |
| 11  | MEIP      | PLIC (on tick: M-mode context has qualified pending) |

**Interrupt delivery (peripheral → PLIC):**

Level-triggered via `Device::irq_line(&self) -> bool`. PLIC calls `irq_line()` on each registered device during its `tick()`. If line is high and source is enabled, pending bit is set. If line goes low, pending bit is cleared (unless already claimed).

```
UART tick():                          PLIC tick():
  drain rx_buf → rx_fifo               for each registered device:
  irq_line = !rx_fifo.is_empty()         if dev.irq_line():
             && ier & 0x01 != 0             self.pending |= 1 << src
                                          else:
                                            self.pending &= !(1 << src)
                                        re-evaluate MEIP/SEIP in ext_ip
```

[**Invariants**]

- I-1: Hardware-wired mip bits (MTIP, MEIP, SEIP, MSIP) are only modified via ext_ip merge, never by CSR write instructions
- I-2: Per-step ordering is fixed: `bus.tick()` → `sync_external_interrupts()` → `check_pending_interrupts()` → fetch/decode/execute → `retire()`
- I-3: PLIC pending bits reflect the **current** device line state, not consumed events. Claim atomically records the source for the context; complete releases it. A device that keeps its line high will be re-pended on the next tick after complete.
- I-4: Device read/write only access offset-relative addresses; devices have no knowledge of their base address
- I-5: mtime is derived from host wall clock but is sampled only during `Clint::tick()` — it does not advance while the emulator is paused in xdb

[**Data Structure**]

```rust
// device/mod.rs
pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
    fn irq_line(&self) -> bool { false }
}

// device/clint.rs
pub struct Clint {
    boot_instant: Instant,     // host wall clock anchor
    mtime_snapshot: u64,       // sampled in tick(), frozen during xdb pause
    msip: u32,                 // hart 0 software interrupt register
    mtimecmp: u64,             // hart 0 timer compare
    ext_ip: Arc<AtomicU64>,    // shared interrupt bits → CPU
}

// device/plic.rs
pub struct Plic {
    priority: Vec<u8>,         // [NUM_SOURCES] per-source priority
    pending: u32,              // 1 bit per source (max 32)
    enable: Vec<Vec<u32>>,     // [NUM_CONTEXTS][words] per-context enable bits
    threshold: Vec<u8>,        // [NUM_CONTEXTS] per-context threshold
    claimed: Vec<u32>,         // [NUM_CONTEXTS] source ID being serviced (0=none)
    ext_ip: Arc<AtomicU64>,    // writes MEIP/SEIP
    sources: Vec<PlicSource>,  // registered device references for irq_line polling
}

struct PlicSource {
    source_id: u32,
    // reference to device for irq_line polling — resolved via Bus during tick()
}

// device/uart.rs
pub struct Uart {
    // Registers
    ier: u8,                   // Interrupt Enable Register
    lcr: u8,                   // Line Control Register (bit 7 = DLAB)
    mcr: u8,                   // Modem Control Register
    dll: u8,                   // Divisor Latch Low
    dlm: u8,                   // Divisor Latch High
    scratch: u8,               // Scratch Register
    // I/O (Phase 4B only)
    rx_fifo: VecDeque<u8>,     // input buffer
    rx_buf: Arc<Mutex<VecDeque<u8>>>,  // TCP thread writes here
}

// device/test_finisher.rs
pub struct TestFinisher;
```

**PLIC device polling design:**

The PLIC needs to call `irq_line()` on registered devices, but devices live inside `Bus::mmio` regions. Since `Bus::tick()` iterates all devices, we solve this by having PLIC observe device state **after** other devices have ticked:

```rust
// Bus::tick() — two-phase tick
impl Bus {
    pub fn tick(&mut self) {
        // Phase 1: tick all non-PLIC devices (CLINT updates ext_ip, UART drains rx_buf)
        for region in &mut self.mmio {
            if region.name != "plic" {
                region.dev.tick();
            }
        }
        // Phase 2: collect irq lines, then tick PLIC
        let lines: u32 = self.mmio.iter()
            .filter(|r| r.name != "plic")
            .enumerate()
            .fold(0u32, |acc, (i, r)| {
                if r.dev.irq_line() { acc | (1 << r.irq_source) } else { acc }
            });
        if let Some(plic) = self.find_plic_mut() {
            plic.set_device_lines(lines);
            plic.tick();
        }
    }
}
```

Alternatively, simpler: store `device_lines: u32` in Bus, set by non-PLIC devices during tick, read by PLIC during its tick. This avoids cross-device references entirely.

```rust
impl Bus {
    pub fn tick(&mut self) {
        self.device_lines = 0;
        for region in &mut self.mmio {
            region.dev.tick();
            if region.dev.irq_line() {
                self.device_lines |= 1 << region.irq_source;
            }
        }
        // PLIC already ticked above; its tick() reads self.device_lines
        // → need PLIC to receive device_lines as parameter
    }
}
```

**Chosen approach:** PLIC holds an `Arc<AtomicU32>` for device lines. Each device that can raise IRQ also holds a clone. Devices update the atomic in their `tick()`. PLIC reads it in its `tick()`. No cross-device references needed.

Wait — this re-introduces the one-shot problem. Let me reconsider.

**Final approach:** The Bus maintains a `device_lines: u32` field. After ticking all devices, Bus collects `irq_line()` results into `device_lines`. Then Bus passes `device_lines` to PLIC via a dedicated method before PLIC's tick. This is clean: no cross-references, no atomics for device→PLIC, level-triggered by construction.

```rust
pub struct Bus {
    ram: Ram,
    mmio: Vec<MmioRegion>,
    device_lines: u32,  // collected each tick from device irq_line()
}

impl Bus {
    pub fn tick(&mut self) {
        // Tick all devices, collect IRQ lines
        self.device_lines = 0;
        for region in &mut self.mmio {
            region.dev.tick();
            if region.dev.irq_line() && region.irq_source > 0 {
                self.device_lines |= 1 << region.irq_source;
            }
        }
        // PLIC reads device_lines during its own tick — passed via a setter
        // Since PLIC is inside mmio, we need a post-tick pass
        for region in &mut self.mmio {
            region.dev.post_tick(self.device_lines);
        }
    }
}

pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
    fn irq_line(&self) -> bool { false }
    fn post_tick(&mut self, _device_lines: u32) {}  // only PLIC uses this
}
```

[**API Surface**]

```rust
// Device trait (extended)
pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
    fn irq_line(&self) -> bool { false }
    fn post_tick(&mut self, _device_lines: u32) {}
}

// MmioRegion (extended)
struct MmioRegion {
    name: &'static str,
    range: Range<usize>,
    dev: Box<dyn Device>,
    irq_source: u32,  // PLIC source ID (0 = no IRQ)
}

// Bus
impl Bus {
    pub fn add_mmio(&mut self, name: &'static str, base: usize, size: usize,
                    dev: Box<dyn Device>, irq_source: u32);
    pub fn tick(&mut self);
}

// Clint
impl Clint {
    pub fn new(ext_ip: Arc<AtomicU64>) -> Self;
    fn mtime(&self) -> u64;             // returns mtime_snapshot
    fn refresh_mtime(&mut self);        // sample host clock → mtime_snapshot
    fn check_timer(&mut self);          // set/clear MTIP in ext_ip
    fn read_msip(&self) -> Word;
    fn write_msip(&mut self, val: Word);
    fn read_mtimecmp(&self, hi: bool) -> Word;
    fn write_mtimecmp(&mut self, hi: bool, val: Word);
    fn read_mtime(&self, hi: bool) -> Word;
}

// Plic
impl Plic {
    pub fn new(ext_ip: Arc<AtomicU64>, num_sources: usize, num_contexts: usize) -> Self;
    fn claim(&mut self, ctx: usize) -> u32;
    fn complete(&mut self, ctx: usize, source: u32);
    fn update_pending(&mut self, device_lines: u32);  // merge level-triggered lines
    fn evaluate_ext_ip(&mut self);                     // re-evaluate MEIP/SEIP
    fn read_priority(&self, source: usize) -> Word;
    fn write_priority(&mut self, source: usize, val: Word);
    fn read_pending(&self) -> Word;
    fn read_enable(&self, ctx: usize) -> Word;
    fn write_enable(&mut self, ctx: usize, val: Word);
    fn read_threshold(&self, ctx: usize) -> Word;
    fn write_threshold(&mut self, ctx: usize, val: Word);
}

// Uart
impl Uart {
    pub fn new() -> Self;
    fn is_dlab(&self) -> bool;          // lcr bit 7
    fn lsr(&self) -> u8;               // compute LSR from state
    fn iir(&self) -> u8;               // compute IIR from state
    fn read_reg(&mut self, offset: usize) -> Word;
    fn write_reg(&mut self, offset: usize, val: u8);
}

// TestFinisher
impl TestFinisher {
    pub fn new() -> Self;
}

// RVCore
impl RVCore {
    fn sync_external_interrupts(&mut self);
}
```

[**Constraints**]

- C-1: Guest-visible MMIO region sizes follow QEMU virt DTS conventions. Internal implementation only decodes the necessary register subset; unrecognized offsets return 0 / are ignored.
- C-2: Single hart only — no per-hart indexing in CLINT/PLIC
- C-3: UART byte-access only (size != 1 returns error for register ops)
- C-4: Device::read changed from `&self` to `&mut self` (UART needs to pop rx_fifo)
- C-5: mtime is host-wall-clock-based at 10 MHz (`timebase-frequency = 10_000_000`), sampled only during tick() — frozen during xdb pause
- C-6: PLIC source 0 is hardwired to "no interrupt" per spec

---

## Implement

### Execution Flow

[**Main Flow — step()**]

```rust
fn step(&mut self) -> XResult {
    // 1. Update all device state (CLINT refreshes mtime, UART drains rx_buf)
    //    Then collect IRQ lines and let PLIC evaluate
    {
        let mut bus = self.bus.lock().unwrap();
        bus.tick();
    }

    // 2. Merge hardware interrupt bits from ext_ip into mip
    self.sync_external_interrupts();

    // 3. Check for pending interrupts
    if self.check_pending_interrupts() {
        self.retire();
        return Ok(());
    }

    // 4. Normal instruction execution
    self.trap_on_err(|core| {
        let raw = core.fetch()?;
        let inst = core.decode(raw)?;
        core.execute(inst)
    })?;

    self.retire();
    Ok(())
}
```

[**sync_external_interrupts()**]

```rust
fn sync_external_interrupts(&mut self) {
    let ext = self.ext_ip.load(Relaxed);
    let mip = self.csr.get(CsrAddr::mip);
    // Hardware-wired bits: MSIP(3), MTIP(7), SEIP(9), MEIP(11)
    const HW_MASK: Word = (1 << 3) | (1 << 7) | (1 << 9) | (1 << 11);
    self.csr.set(CsrAddr::mip, (mip & !HW_MASK) | (ext as Word & HW_MASK));
}
```

[**Bus::tick() — two-phase**]

```rust
pub fn tick(&mut self) {
    // Phase 1: tick all devices, collect IRQ lines
    self.device_lines = 0;
    for region in &mut self.mmio {
        region.dev.tick();
        if region.dev.irq_line() && region.irq_source > 0 {
            self.device_lines |= 1 << region.irq_source;
        }
    }
    // Phase 2: pass device lines to PLIC (via post_tick)
    let lines = self.device_lines;
    for region in &mut self.mmio {
        region.dev.post_tick(lines);
    }
}
```

[**Failure Flow**]

1. TestFinisher write → `Err(XError::ProgramExit(code))` → CPU catches, calls `set_terminated()`
2. MMIO access to unmapped region → `Err(XError::BadAddress)` → trap
3. UART non-byte register access → `Err(XError::BadAddress)`

[**State Transition**]

- Device line high → PLIC pending set (tick) → PLIC evaluates → MEIP/SEIP set in ext_ip → sync → mip bit set → interrupt taken
- PLIC claim → records source for context, clears pending → guest services IRQ → PLIC complete → releases claim → if device line still high, re-pended next tick

### Implementation Plan

[**Step 0: Interrupt Plumbing**]

Files modified:
- `device/mod.rs`: Extend Device trait with `tick()`, `irq_line()`, `post_tick()`. Change `read(&self)` to `read(&mut self)`.
- `device/bus.rs`: Add `device_lines: u32` field. Add `irq_source: u32` to MmioRegion. Extend `add_mmio()` signature. Implement `Bus::tick()` two-phase logic.
- `cpu/riscv/mod.rs`: Add `ext_ip: Arc<AtomicU64>` field to RVCore. Implement `sync_external_interrupts()`. Update `step()` ordering.
- `error.rs`: Add `XError::ProgramExit(u32)` variant.
- `cpu/mod.rs`: Catch `XError::ProgramExit` in `CPU::step()`, call `set_terminated()`.

Function detail:

```rust
// device/mod.rs
pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
    fn irq_line(&self) -> bool { false }
    fn post_tick(&mut self, _device_lines: u32) {}
}

// device/bus.rs — MmioRegion extended
struct MmioRegion {
    name: &'static str,
    range: Range<usize>,
    dev: Box<dyn Device>,
    irq_source: u32,  // 0 = no IRQ
}

// Bus extended
pub struct Bus {
    ram: Ram,
    mmio: Vec<MmioRegion>,
    device_lines: u32,
}

impl Bus {
    pub fn add_mmio(&mut self, name: &'static str, base: usize, size: usize,
                    dev: Box<dyn Device>, irq_source: u32) { ... }

    pub fn tick(&mut self) {
        self.device_lines = 0;
        for region in &mut self.mmio {
            region.dev.tick();
            if region.dev.irq_line() && region.irq_source > 0 {
                self.device_lines |= 1 << region.irq_source;
            }
        }
        let lines = self.device_lines;
        for region in &mut self.mmio {
            region.dev.post_tick(lines);
        }
    }
}
```

[**Step 1: CLINT**]

New file: `device/clint.rs`

Register dispatch (32-bit access):

```rust
impl Device for Clint {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word> {
        let val = match offset {
            0x0000              => self.msip as Word,
            0x4000              => self.mtimecmp as u32 as Word,       // lo
            0x4004              => (self.mtimecmp >> 32) as u32 as Word, // hi
            0xBFF8              => self.mtime_snapshot as u32 as Word,  // lo
            0xBFFC              => (self.mtime_snapshot >> 32) as u32 as Word, // hi
            _                   => 0, // unmapped hart slots
        };
        Ok(val)
    }

    fn write(&mut self, offset: usize, _size: usize, value: Word) -> XResult {
        match offset {
            0x0000 => {
                self.msip = (value as u32) & 1;
                // Update MSIP bit (bit 3) in ext_ip
                if self.msip != 0 {
                    self.ext_ip.fetch_or(1 << 3, Relaxed);
                } else {
                    self.ext_ip.fetch_and(!(1 << 3), Relaxed);
                }
            }
            0x4000 => {
                // Write mtimecmp low 32 bits
                self.mtimecmp = (self.mtimecmp & 0xFFFF_FFFF_0000_0000)
                    | (value as u32 as u64);
                self.check_timer();
            }
            0x4004 => {
                // Write mtimecmp high 32 bits
                self.mtimecmp = (self.mtimecmp & 0x0000_0000_FFFF_FFFF)
                    | ((value as u32 as u64) << 32);
                self.check_timer();
            }
            _ => {} // mtime is read-only; unmapped slots ignored
        }
        Ok(())
    }

    fn tick(&mut self) {
        self.refresh_mtime();
        self.check_timer();
    }
}

impl Clint {
    pub fn new(ext_ip: Arc<AtomicU64>) -> Self {
        Self {
            boot_instant: Instant::now(),
            mtime_snapshot: 0,
            msip: 0,
            mtimecmp: u64::MAX, // timer disabled by default
            ext_ip,
        }
    }

    fn refresh_mtime(&mut self) {
        self.mtime_snapshot = self.boot_instant.elapsed().as_nanos() as u64 / 100;
    }

    fn check_timer(&mut self) {
        if self.mtime_snapshot >= self.mtimecmp {
            self.ext_ip.fetch_or(1 << 7, Relaxed);  // set MTIP
        } else {
            self.ext_ip.fetch_and(!(1 << 7), Relaxed); // clear MTIP
        }
    }
}
```

[**Step 2: PLIC**]

New file: `device/plic.rs`

Constants:

```rust
const NUM_SOURCES: usize = 32;
const NUM_CONTEXTS: usize = 2; // ctx 0 = M-mode, ctx 1 = S-mode
```

Register dispatch:

```rust
impl Device for Plic {
    fn read(&mut self, offset: usize, _size: usize) -> XResult<Word> {
        let val = match offset {
            // Priority: 0x000000..0x000080 (source * 4)
            o @ 0x000000..=0x00007F => {
                let src = o / 4;
                self.priority[src] as Word
            }
            // Pending: 0x001000
            0x001000 => self.pending as Word,
            // Enable: 0x002000 + ctx * 0x80
            o @ 0x002000..=0x002FFF => {
                let ctx = (o - 0x002000) / 0x80;
                if ctx < NUM_CONTEXTS { self.enable[ctx][0] as Word } else { 0 }
            }
            // Threshold: 0x200000 + ctx * 0x1000
            o if (0x200000..0x200000 + NUM_CONTEXTS * 0x1000).contains(&o)
                 && (o & 0xFFF) == 0 => {
                let ctx = (o - 0x200000) / 0x1000;
                self.threshold[ctx] as Word
            }
            // Claim: 0x200004 + ctx * 0x1000
            o if (0x200004..0x200004 + NUM_CONTEXTS * 0x1000).contains(&o)
                 && (o & 0xFFF) == 4 => {
                let ctx = (o - 0x200004) / 0x1000;
                self.claim(ctx) as Word
            }
            _ => 0,
        };
        Ok(val)
    }

    fn write(&mut self, offset: usize, _size: usize, value: Word) -> XResult {
        match offset {
            o @ 0x000000..=0x00007F => {
                let src = o / 4;
                self.priority[src] = value as u8;
            }
            o @ 0x002000..=0x002FFF => {
                let ctx = (o - 0x002000) / 0x80;
                if ctx < NUM_CONTEXTS {
                    self.enable[ctx][0] = value as u32;
                }
            }
            o if (0x200000..0x200000 + NUM_CONTEXTS * 0x1000).contains(&o)
                 && (o & 0xFFF) == 0 => {
                let ctx = (o - 0x200000) / 0x1000;
                self.threshold[ctx] = value as u8;
                self.evaluate_ext_ip();
            }
            o if (0x200004..0x200004 + NUM_CONTEXTS * 0x1000).contains(&o)
                 && (o & 0xFFF) == 4 => {
                let ctx = (o - 0x200004) / 0x1000;
                self.complete(ctx, value as u32);
            }
            _ => {}
        }
        Ok(())
    }

    fn post_tick(&mut self, device_lines: u32) {
        self.update_pending(device_lines);
        self.evaluate_ext_ip();
    }
}
```

Claim/complete logic:

```rust
impl Plic {
    /// Claim: find highest-priority enabled pending source above threshold.
    fn claim(&mut self, ctx: usize) -> u32 {
        let mut best_source = 0u32;
        let mut best_priority = 0u8;

        for src in 1..NUM_SOURCES {
            let bit = 1u32 << src;
            if self.pending & bit == 0 { continue; }
            if self.enable[ctx][0] & bit == 0 { continue; }
            if self.priority[src] <= self.threshold[ctx] { continue; }
            if self.priority[src] > best_priority {
                best_priority = self.priority[src];
                best_source = src as u32;
            }
        }

        if best_source > 0 {
            self.pending &= !(1 << best_source);
            self.claimed[ctx] = best_source;
        }
        best_source
    }

    /// Complete: release claimed source, re-evaluate.
    fn complete(&mut self, ctx: usize, source: u32) {
        if self.claimed[ctx] == source {
            self.claimed[ctx] = 0;
        }
        self.evaluate_ext_ip();
    }

    /// Merge level-triggered device lines into pending.
    fn update_pending(&mut self, device_lines: u32) {
        // For each source: if device line is high, set pending
        // If line is low AND source is not currently claimed, clear pending
        for src in 1..NUM_SOURCES {
            let bit = 1u32 << src;
            if device_lines & bit != 0 {
                self.pending |= bit;
            } else {
                // Only clear if not currently being serviced
                let is_claimed = self.claimed.iter().any(|&c| c == src as u32);
                if !is_claimed {
                    self.pending &= !bit;
                }
            }
        }
    }

    /// Re-evaluate MEIP/SEIP based on current pending/enable/threshold state.
    fn evaluate_ext_ip(&mut self) {
        for ctx in 0..NUM_CONTEXTS {
            let has_qualified = (1..NUM_SOURCES).any(|src| {
                let bit = 1u32 << src;
                self.pending & bit != 0
                    && self.enable[ctx][0] & bit != 0
                    && self.priority[src] > self.threshold[ctx]
            });
            let ip_bit = if ctx == 0 { 1u64 << 11 } else { 1u64 << 9 }; // MEIP / SEIP
            if has_qualified {
                self.ext_ip.fetch_or(ip_bit, Relaxed);
            } else {
                self.ext_ip.fetch_and(!ip_bit, Relaxed);
            }
        }
    }
}
```

[**Step 3: UART 16550 (Phase 4A: TX-only)**]

New file: `device/uart.rs`

Register bank switching:

```rust
impl Device for Uart {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word> {
        if size != 1 { return Err(XError::BadAddress); }
        let val = match offset {
            0 if self.is_dlab()  => self.dll,
            0                    => self.rx_fifo.pop_front().unwrap_or(0),
            1 if self.is_dlab()  => self.dlm,
            1                    => self.ier,
            2                    => self.iir(),
            3                    => self.lcr,
            4                    => self.mcr,
            5                    => self.lsr(),
            6                    => 0, // MSR: no modem
            7                    => self.scratch,
            _                    => 0, // unrecognized offset within 0x100 region
        };
        Ok(val as Word)
    }

    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult {
        if size != 1 { return Err(XError::BadAddress); }
        let byte = value as u8;
        match offset {
            0 if self.is_dlab()  => self.dll = byte,
            0                    => {
                // THR: write to stdout
                use std::io::Write;
                let _ = std::io::stdout().lock().write_all(&[byte]);
                let _ = std::io::stdout().flush();
            }
            1 if self.is_dlab()  => self.dlm = byte,
            1                    => self.ier = byte & 0x0F,
            2                    => {} // FCR: ignored
            3                    => self.lcr = byte,
            4                    => self.mcr = byte,
            7                    => self.scratch = byte,
            _                    => {} // unrecognized
        }
        Ok(())
    }

    fn irq_line(&self) -> bool {
        // Phase 4A: TX-only, no RX interrupts
        // Phase 4B: !self.rx_fifo.is_empty() && (self.ier & 0x01 != 0)
        false
    }
}

impl Uart {
    pub fn new() -> Self {
        Self {
            ier: 0, lcr: 0x03, mcr: 0, dll: 0, dlm: 0, scratch: 0,
            rx_fifo: VecDeque::new(),
            rx_buf: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    fn is_dlab(&self) -> bool { self.lcr & 0x80 != 0 }

    fn lsr(&self) -> u8 {
        let dr   = if self.rx_fifo.is_empty() { 0 } else { 0x01 };
        let thre = 0x20; // TX always ready
        let temt = 0x40; // TX always empty
        dr | thre | temt
    }

    fn iir(&self) -> u8 {
        // FIFO enabled bits (7:6) + interrupt identification
        if !self.rx_fifo.is_empty() && (self.ier & 0x01 != 0) {
            0xC4 // RX data available
        } else {
            0xC1 // no interrupt pending
        }
    }
}
```

[**Step 4: SiFive Test Finisher**]

New file: `device/test_finisher.rs`

```rust
impl Device for TestFinisher {
    fn read(&mut self, _offset: usize, _size: usize) -> XResult<Word> {
        Ok(0)
    }

    fn write(&mut self, offset: usize, _size: usize, value: Word) -> XResult {
        if offset == 0 {
            let val = value as u32;
            if val == 0x5555 {
                return Err(XError::ProgramExit(0));
            }
            if val & 0xFFFF == 0x3333 {
                return Err(XError::ProgramExit((val >> 16) as u32));
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

Wire devices into Bus at RVCore construction:

```rust
impl RVCore {
    pub fn new() -> Self {
        let ext_ip = Arc::new(AtomicU64::new(0));
        let bus = Arc::new(Mutex::new(Bus::new(CONFIG_MBASE, CONFIG_MSIZE)));
        {
            let mut b = bus.lock().unwrap();
            b.add_mmio("clint", 0x0200_0000, 0x1_0000,
                        Box::new(Clint::new(ext_ip.clone())), 0);
            b.add_mmio("plic",  0x0C00_0000, 0x400_0000,
                        Box::new(Plic::new(ext_ip.clone(), 32, 2)), 0);
            b.add_mmio("uart0", 0x1000_0000, 0x100,
                        Box::new(Uart::new()), 10);
            b.add_mmio("test",  0x0010_0000, 0x10,
                        Box::new(TestFinisher::new()), 0);
        }
        Self {
            // ... existing fields ...
            ext_ip,
            bus,
            // ...
        }
    }
}
```

CPU-level ProgramExit handling:

```rust
// cpu/mod.rs
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

- T-1: **Interrupt delivery** — `Arc<AtomicU64>` (lock-free, multi-core ready) vs direct mip write (simpler). Chose atomic for decoupling and future multi-core.

- T-2: **CLINT mtime source** — Host wall clock with snapshot model (realistic, frozen during xdb pause, `timebase-frequency = 10_000_000`) vs instruction-count (deterministic). Chose host clock for realism. Debugger semantics: mtime only advances during `Clint::tick()`, which only runs inside `step()`. When paused in xdb, `step()` is not called, so mtime freezes. Trade-off: mtime-to-wall-clock ratio varies with host speed and instruction complexity, but this is acceptable for an emulator not targeting cycle-accuracy.

- T-3: **UART RX backend** — Phase 4A: TX-only (no conflict). Phase 4B: TCP socket listener (e.g., `127.0.0.1:14514`), user connects via `nc` or `telnet`. No stdin capture, xdb interaction preserved. Trade-off: requires user to open a second terminal for serial input, but cleanly separates debugger and guest I/O.

- T-4: **Device-to-PLIC signaling** — `Device::irq_line()` (level-triggered, re-sampled every tick via Bus) vs `Arc<AtomicU32>` (event-based). Chose irq_line for correctness with level-triggered devices like UART RX. Bus collects lines after tick, passes to PLIC via `post_tick()`.

---

## Validation

[**Unit Tests**]

- V-UT-1: CLINT mtime reads increase after successive ticks
- V-UT-2: CLINT mtimecmp write → MTIP set in ext_ip when mtime >= mtimecmp
- V-UT-3: CLINT mtimecmp write → MTIP clear in ext_ip when mtime < mtimecmp
- V-UT-4: CLINT msip write 1 → MSIP set; write 0 → MSIP clear
- V-UT-5: PLIC priority register read/write for all sources
- V-UT-6: PLIC enable register per context
- V-UT-7: PLIC claim returns highest-priority enabled pending source above threshold
- V-UT-8: PLIC claim returns 0 when nothing pending
- V-UT-9: PLIC complete releases claimed source
- V-UT-10: PLIC threshold filters low-priority sources from claim
- V-UT-11: UART THR write (verify stdout side effect or mock)
- V-UT-12: UART LSR reports DR=0 when rx_fifo empty, DR=1 when non-empty
- V-UT-13: UART DLAB=1 switches offset 0/1 to DLL/DLM
- V-UT-14: UART IIR reports 0xC4 when rx data available and IER.rx enabled
- V-UT-15: TestFinisher write 0x5555 → Err(ProgramExit(0))
- V-UT-16: TestFinisher write (1<<16)|0x3333 → Err(ProgramExit(1))
- V-UT-17: TestFinisher read returns 0

[**Integration Tests**]

- V-IT-1: CLINT timer → MTIP → mip → check_pending_interrupts → M-mode timer trap
- V-IT-2: PLIC with manually set pending → MEIP → mip → trap
- V-IT-3: TestFinisher → CPU halts with correct exit code
- V-IT-4: UART FIFO partial read: push 3 bytes, claim interrupt, read 1 byte, complete, next tick re-asserts pending (level-triggered correctness)
- V-IT-5: CLINT mtime does not advance between two ticks when no tick() is called (pause semantics)

[**Failure / Robustness Validation**]

- V-F-1: CLINT access to unmapped hart offsets returns 0
- V-F-2: PLIC claim with nothing pending returns 0
- V-F-3: UART non-byte access returns BadAddress
- V-F-4: PLIC complete with wrong source ID — no state change

[**Edge Case Validation**]

- V-E-1: CLINT mtimecmp = u64::MAX — timer never fires
- V-E-2: PLIC complete with mismatched source — claimed state unchanged
- V-E-3: UART all 8 offsets read in both DLAB modes
- V-E-4: UART in Phase 4A: irq_line() always false, no stdin capture

[**Acceptance Mapping**]

| Goal / Constraint | Validation |
|-------------------|------------|
| G-1 (CLINT) | V-UT-1..4, V-IT-1, V-IT-5, V-E-1 |
| G-2 (PLIC) | V-UT-5..10, V-IT-2, V-IT-4, V-E-2 |
| G-3a (UART TX) | V-UT-11..14, V-E-3, V-E-4 |
| G-4 (TestFinisher) | V-UT-15..17, V-IT-3 |
| G-5 (ext_ip) | V-IT-1, V-IT-2 |
| C-1 (QEMU virt sizes) | Memory map constants |
| C-3 (UART byte-only) | V-F-3 |
| C-5 (mtime pause) | V-IT-5 |
| I-3 (level-triggered) | V-IT-4 |

---

## File Organization

```
xcore/src/device/
├── mod.rs            — Device trait (read, write, tick, irq_line, post_tick)
├── bus.rs            — Bus (+ tick two-phase, device_lines, irq_source)
├── ram.rs            — Ram
├── clint.rs          — CLINT (new)
├── plic.rs           — PLIC (new)
├── uart.rs           — UART 16550 (new)
└── test_finisher.rs  — SiFive Test Finisher (new)
```

## Memory Map

| Device | Base | Guest-visible Size | Internal Decoded | PLIC IRQ |
|--------|------|--------------------|------------------|----------|
| SiFive Test Finisher | `0x0010_0000` | `0x10` | offset 0 only | — |
| CLINT | `0x0200_0000` | `0x1_0000` | msip, mtimecmp, mtime | — |
| PLIC | `0x0C00_0000` | `0x400_0000` | priority, pending, enable, threshold, claim/complete | — |
| UART0 | `0x1000_0000` | `0x100` | offsets 0-7 | source 10 |
| RAM | `0x8000_0000` | 128 MB | full | — |
