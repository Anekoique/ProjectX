[**Goals**]

- G-1: Split PLIC into three crate-internal layers — `Source` signal plane, per-source `Gateway` FSM, and `Core` priority arbiter — behind one `Plic` `Device`.
- G-2: Support level-triggered claim gating: once a source is claimed, the gateway suppresses re-pend until the matching complete arrives.
- G-3: Preserve the exact QEMU-virt MMIO register layout, claim/complete semantics, and per-hart MEIP / SEIP routing.
- G-4: Keep `Plic::new` and `with_irq_line` as the only public construction points; `Gateway` / `Core` are not part of the API.

[**Non-goals**]

- NG-1: No edge-triggered sources (gateway FSM has level-triggered semantics only; matches all current device IRQs).
- NG-2: No DTB / per-source priority defaults — guests write priority via MMIO at boot.
- NG-3: No multi-thread arbitration — the single CPU thread serializes claim / complete.

[**Architecture**]

```
xemu/xcore/src/arch/riscv/device/plic.rs
├── Plic            Device impl; owns Arc<PlicSignals>, gateways, core, irq sinks per hart
├── new(harts, irqs)
├── with_irq_line(src) → IrqLine for external device wiring
└── tick()          drain signals → feed gateways → arbitrate → drive MEIP / SEIP

xemu/xcore/src/arch/riscv/device/plic/
├── gateway.rs      enum GatewayDecision; struct Gateway { state machine per source }
└── core.rs         const NUM_SRC = 32; struct Core { priority, pending, enable, threshold, claimed }
```

Data flow per tick: producer (device) → `IrqLine::raise` → `PlicSignals` plane → `Plic::tick` drains plane → each `Gateway` evaluates `(level, claim_in_flight)` → `Core::set_pending(src)` when armed → `Core::evaluate()` selects highest-priority context-enabled source above threshold → drives MEIP / SEIP via `IrqState`.

[**Data Structure**]

```rust
pub struct Plic {
    signals:  Arc<PlicSignals>,
    gateways: [Gateway; NUM_SRC],
    core:     Core,
    irqs:     Vec<IrqState>,   // one per hart (M-mode + S-mode contexts)
}

pub(super) const NUM_SRC: usize = 32;

pub(super) struct Gateway { /* level FSM + claim_in_flight flag */ }
pub(super) enum   GatewayDecision { NoChange, PendNow, ClaimGate }
pub(super) struct Core {
    priority:  [u32; NUM_SRC],
    pending:   AtomicU32,
    enable:    [u32; NUM_CTX],
    threshold: [u32; NUM_CTX],
    claimed:   [u32; NUM_CTX],
}
```

[**API Surface**]

```rust
impl Plic {
    pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self;
    pub fn with_irq_line(&self, src: u32) -> IrqLine;
}
impl Device for Plic { /* read/write MMIO, tick */ }
```

[**Constraints**]

- C-1: Arbitration is equivalent to the pre-split monolithic PLIC for the level-triggered default — `xemu/xcore/src/arch/riscv/device/plic/core.rs`.
- C-2: Once source `s` is claimed, the gateway for `s` does not re-pend until a matching complete arrives — `xemu/xcore/src/arch/riscv/device/plic/gateway.rs`.
- C-3: Re-assertion of the level line while claimed is recorded by the gateway but held back from the core — pre-claim-clear (I-8).
- C-4: PLIC MMIO layout matches QEMU-virt: priority `0x000000..0x000080`, pending `0x001000`, enable `0x002000 + ctx*0x80`, threshold `0x200000 + ctx*0x1000`, claim/complete `0x200004 + ctx*0x1000` — `xemu/xcore/src/arch/riscv/device/plic.rs`.
- C-5: `NUM_SRC = 32` matches the `u32` bit-width of `PlicSignals::level` — raising any plumbing to >32 requires changing both atomics in lockstep — `xemu/xcore/src/arch/riscv/device/plic/core.rs:14`.
- C-6: `Gateway`, `Core`, `GatewayDecision` are `pub(super)` only — outside `arch/riscv/device/plic/`, the only PLIC surface is `Plic` + `IrqLine`.
- C-7: Source 0 is reserved (PLIC spec); writes are no-ops at the gateway level — `xemu/xcore/src/arch/riscv/device/plic/core.rs`.

[**CHANGELOG**]

- `2026-05-11` `port-to-ark`: rebuilt from current code under `xemu/xcore/src/arch/riscv/device/plic*`. Pre-port running notes preserved at `.ark/tasks/archive/legacy/plic-gateway/`.
