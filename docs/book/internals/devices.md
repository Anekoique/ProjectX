# Devices

## `Device` trait

```rust
pub trait Device: Send {
    fn read(&mut self, offset: usize, size: usize) -> XResult<Word>;
    fn write(&mut self, offset: usize, size: usize, value: Word) -> XResult;
    fn tick(&mut self) {}
    fn irq_line(&self) -> bool { false }
    fn notify(&mut self, _irq_lines: u32) {}
}
```

Five methods. Default-no-op for `tick`, `irq_line`, `notify` — device
authors override only what they need.

## Bus

```rust
pub struct Bus {
    ram: Ram,
    mmio: Vec<MmioRegion>,
    plic_idx: Option<usize>,
}

struct MmioRegion {
    name: &'static str,
    range: Range<usize>,
    dev: Box<dyn Device>,
    irq_source: u32,       // 0 = no IRQ
}
```

Every access goes through `Bus::read` / `Bus::write`:

- **Fast path — RAM.** Static dispatch, no vtable. Typed-read bypass
  for aligned 1/2/4/8-byte accesses (Phase P6).
- **Slow path — MMIO.** Linear scan for the covering region, then
  dispatch via `dyn Device`.

## `tick()` split

```rust
pub fn tick(&mut self) {
    // ... ACLINT every step (fast path) ...
    // ... UART + PLIC every 64 steps (slow path) ...
}
```

ACLINT fires on every step because the Mtimer deadline check is on
the critical path. UART and PLIC tick less frequently — their state
rarely changes per-instruction.

Inside ACLINT, the Mtimer deadline gate (Phase P3) short-circuits
99.99 % of checks:

```rust
if self.mtime < self.next_fire_mtime { return; }
self.check_all();  // slow path only when a deadline has arrived
```

## IRQ collection

The Bus collects level-triggered IRQ lines:

```rust
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
```

The PLIC is the **only** device whose `notify` is overridden — it
receives the full IRQ-line bitmap and evaluates MEIP/SEIP.

## Per-device pages

- [ACLINT (MSWI / MTIMER / SSWI)](./devices-aclint.md)
- [PLIC](./devices-plic.md)
- [UART 16550](./devices-uart.md)
- [VirtIO-blk](./devices-virtio.md)
