[**Goals**]

- G-1: Provide a minimal device set for console I/O, timer, interrupt routing, and block-device boot — ACLINT, PLIC, UART 16550, VirtIO-blk, TestFinisher.
- G-2: Expose a 2-method `Device` trait (`read` / `write`) plus optional `tick` / `reset` / `mtime` defaults so adding a new device is a self-contained patch.
- G-3: Split ACLINT into independently constructible sub-devices (MSWI, MTIMER, SSWI) installed via one `Aclint::install` call returning the MTIMER region index.
- G-4: Offer two UART backends behind the same MMIO surface: stdio (firmware boot) and PTY (debug-mode interactive RX).
- G-5: Match QEMU-virt MMIO layout for plug-compatibility; intentional deltas: ACLINT replaces CLINT, TestFinisher is test-only.

[**Non-goals**]

- NG-1: No VGA / framebuffer / audio / keyboard-controller devices.
- NG-2: No per-hart PLIC contexts beyond M-mode and S-mode (two contexts per hart).
- NG-3: No DMA-coherent caches — MMIO writes visible immediately, no write buffering.

[**Architecture**]

```
xemu/xcore/src/device/
├── mod.rs              Device trait, IrqState, IrqLine re-export, MMIO constants
├── bus.rs              Bus + DmaCtx + LeBytes (RAM + MMIO routing; M-001 owned-bus)
├── ram.rs              Ram (RAM-backed Device, 128 MiB default)
├── irq.rs              PlicSignals (lock-free level-bitmap + epoch flag) + IrqLine
├── uart.rs             Uart (16550 TX + stdio/PTY RX, THRE interrupt)
├── virtio_blk.rs       VirtioBlk (MMIO legacy v1 transport, split virtqueue)
├── virtio/             Virtqueue + Desc + BlkReqType / BlkStatus / DescFlags
└── test_finisher.rs    TestFinisher (SiFive shutdown/reboot at 0x10_0000)

xemu/xcore/src/arch/riscv/device/
├── aclint.rs           Aclint (composite MSWI + MTIMER + SSWI) + install(bus, base)
├── aclint/             mswi.rs · mtimer.rs · sswi.rs (each its own Device)
└── plic.rs / plic/     Plic Device + gateway.rs (per-source FSM) + core.rs (arbiter)
```

Bus dispatches MMIO by physical-address range; each `Device` sees offset-relative addresses. The PLIC owns shared `PlicSignals`; devices hold cloneable `IrqLine` handles that toggle their source bit without entering the Bus. `Bus::tick` runs ACLINT every step and UART / PLIC every 64 steps (split tick).

[**Data Structure**]

```rust
pub trait Device: Send {
    fn name(&self) -> &str;
    fn read (&mut self, offset: usize, size: usize)              -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, val: Word)   -> XResult;
    fn tick (&mut self) {}
    fn reset(&mut self) {}
    fn mtime(&self) -> Option<u64> { None }
}

pub struct IrqState { /* per-hart mip merge */ }
pub struct IrqLine  { signals: Arc<PlicSignals>, bit: u32 }
pub struct PlicSignals { level: AtomicU32, pending_raises: AtomicBool }

pub struct Aclint     { mswi: Mswi, mtimer: Mtimer, sswi: Sswi }
pub struct Plic       { gateways: [Gateway; NUM_SRC], core: Core, signals: Arc<PlicSignals> }
pub struct Uart       { /* THR / RBR / LSR / IER / IIR / LCR + backend (stdio | pty) */ }
pub struct VirtioBlk  { /* MMIO regs + Virtqueue + BlkStorage + IrqLine */ }
pub struct TestFinisher;
```

[**API Surface**]

```rust
impl Aclint {
    pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self;
    pub fn install(self, bus: &mut Bus, base: usize) -> usize; // returns MTIMER region idx
}

impl Plic {
    pub fn new(num_harts: usize, irqs: Vec<IrqState>) -> Self;
    pub fn with_irq_line(&self, src: u32) -> IrqLine;
}

impl Uart {
    pub fn new       (irq: IrqLine) -> Self;                        // TX-only (no RX)
    pub fn with_stdio(irq: IrqLine) -> Self;                        // stdin/stdout RX
    pub fn with_pty  (irq: IrqLine) -> Result<Self, String>;        // PTY-backed RX
}

impl VirtioBlk    { pub fn new(disk: Vec<u8>, irq: IrqLine) -> Self; }
impl TestFinisher { pub fn new() -> Self; }
```

[**Constraints**]

- C-1: MMIO layout matches QEMU-virt with two deltas — ACLINT replaces CLINT; TestFinisher is test-only — `xemu/xcore/src/device/mod.rs`.
- C-2: `mip` hardware bits are modified only via `IrqState` atomic merge; no device writes `mip` directly — enforced via `HW_IP_MASK` — `xemu/xcore/src/arch/riscv/cpu/trap/interrupt.rs:22`.
- C-3: Bus::tick orders ACLINT first, devices next, PLIC last per slow-tick — `xemu/xcore/src/device/bus.rs:264`.
- C-4: Claimed PLIC sources are excluded from re-pending until matching complete — gateway FSM enforces — `xemu/xcore/src/arch/riscv/device/plic/gateway.rs`.
- C-5: Devices see offset-relative addresses; absolute paddr arithmetic stays in `Bus` — `xemu/xcore/src/device/bus.rs:302`.
- C-6: `mtime` runs at host 10 MHz and freezes during `xdb` pause; DTS `timebase-frequency = 10_000_000` — `resource/xemu.dts`.
- C-7: PLIC source 0 reserved as "no interrupt"; SSWI read returns 0 (write-only) — `xemu/xcore/src/arch/riscv/device/plic/core.rs`.
- C-8: `Device::read` takes `&mut self` — read may have side effects (UART RBR consume, PLIC claim) — `xemu/xcore/src/device/mod.rs:28`.
- C-9: UART supports byte access only (size = 1); larger sizes raise `BadAddress` — `xemu/xcore/src/device/uart.rs`.
- C-10: ACLINT is a composite of `Mswi`, `Mtimer`, `Sswi` — each sub-device is independently constructible and Device-trait conformant — `xemu/xcore/src/arch/riscv/device/aclint/`.
- C-11: `IrqLine::raise` / `lower` are idempotent and may be called from any thread — `xemu/xcore/src/device/irq.rs:94`.
- C-12: `Bus` is owned inline by `CPU` — no `Mutex<Bus>` / `RwLock<Bus>` anywhere — enforced by `scripts/ci/verify_no_mutex.sh`.
- C-13: VirtIO-blk uses MMIO legacy (v1) + split virtqueue (128 entries) + synchronous DMA — `xemu/xcore/src/device/virtio_blk.rs`.
- C-14: UART PTY backend (`Uart::with_pty`) is the keyboard-input mechanism for debug-mode boot; firmware-mode boot uses `Uart::with_stdio` — `xemu/xcore/src/device/uart.rs:211`.

[**CHANGELOG**]

- `2026-05-11` `port-to-ark`: rebuilt from current code under `xemu/xcore/src/device/` and `xemu/xcore/src/arch/riscv/device/`. Absorbs the legacy `aclint-split` (sub-device split) and `keyboard` (PTY UART RX) features. Pre-port running notes preserved at `.ark/tasks/archive/legacy/devices/`, `.ark/tasks/archive/legacy/aclint-split/`, `.ark/tasks/archive/legacy/keyboard/`.
