[**Goals**]

- G-1: Eliminate the device → bus → PLIC round-trip by giving each device a direct `IrqLine` handle into the PLIC's shared signal plane.
- G-2: Make signal delivery lock-free — atomic level-bitmap + epoch flag — so producers never contend with the CPU thread.
- G-3: Provide a one-acquire fast path in the drain when no producer has raised since the last drain (event-driven).
- G-4: Keep `IrqLine` arch-neutral so non-RISC-V backends can mint handles without leaking PLIC internals.

[**Non-goals**]

- NG-1: No multi-thread CPU drive (single-thread cooperative scheduler remains; happens-before is producer → drain, not producer ↔ producer).
- NG-2: No edge-triggered sources beyond what PLIC's per-source gateway FSM handles in `plic-gateway`.
- NG-3: No per-source priority on the signal plane — priority lives in PLIC core, not in `PlicSignals`.

[**Architecture**]

```
xemu/xcore/src/device/irq.rs
├── PlicSignals { level: AtomicU32, pending_raises: AtomicBool }
└── IrqLine     { signals: Arc<PlicSignals>, bit: u32 }

xemu/xcore/src/arch/riscv/device/plic.rs
├── Plic owns Arc<PlicSignals>
├── tick()       → calls signals.drain() and feeds gateway FSMs
└── with_irq_line(src) → mints a cloneable IrqLine for source `src`
```

Producer side (UART, VirtIO-blk, ACLINT softirqs): `line.raise()` does one `fetch_or` + one `Release` store. Consumer side (CPU thread inside `Bus::tick` slow path): `signals.drain()` does one `Acquire` swap; if `false`, returns `None` without touching `level`.

[**Data Structure**]

```rust
pub struct PlicSignals {
    level:          AtomicU32,
    pending_raises: AtomicBool,
}

#[derive(Clone)]
pub struct IrqLine {
    signals: Arc<PlicSignals>,
    bit:     u32,
}
```

[**API Surface**]

```rust
impl PlicSignals {
    pub fn new()                   -> Self;
    pub fn drain(&self)            -> Option<u32>;   // None on fast path
    pub fn reset(&self);                              // forces next drain + clears level
}

impl IrqLine {
    pub fn new(signals: Arc<PlicSignals>, src: u32) -> Self;
    pub fn raise(&self);          // idempotent
    pub fn lower(&self);          // idempotent
}

impl Clone for IrqLine { /* clones alias the same source bit */ }
```

[**Constraints**]

- C-1: Producers' last store is `Release`; the drain's first load is `Acquire` swap — establishes happens-before from pre-raise device state to post-drain CPU observation — `xemu/xcore/src/device/irq.rs:44`.
- C-2: `drain` returns `None` when no producer has stored since the last drain — fast path is one `Acquire` swap and no per-source work — `xemu/xcore/src/device/irq.rs:51`.
- C-3: `IrqLine::raise` and `lower` are idempotent; redundant calls are no-ops at the bit level (I-D2 / I-D3) — `xemu/xcore/src/device/irq.rs:94`.
- C-4: `IrqLine::new` requires `src` in `1..=31` — source 0 reserved by PLIC spec; source 31 is the highest bit in the `u32` plane (I-D12) — `xemu/xcore/src/device/irq.rs:83`.
- C-5: `IrqLine` clones alias the same source — multiple devices on one source coalesce naturally — `xemu/xcore/src/device/irq.rs:72`.
- C-6: `PlicSignals::reset` sets `pending_raises = true` so the next drain is forced and de-asserts IRQ lines — `xemu/xcore/src/device/irq.rs:59`.
- C-7: PLIC-last ordering in `Bus::tick`: every non-PLIC device ticks before the PLIC so raises produced in the same slow-tick are observed in one pass — `xemu/xcore/src/device/bus.rs:264`.

[**CHANGELOG**]

- `2026-05-11` `port-to-ark`: rebuilt from current code at `xemu/xcore/src/device/irq.rs` and `xemu/xcore/src/arch/riscv/device/plic.rs`. Pre-port running notes preserved at `.ark/tasks/archive/legacy/direct-irq/`.
