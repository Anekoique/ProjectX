# PLIC

Platform-Level Interrupt Controller at `0x0C00_0000`, 64 MiB region.

- 32 sources (source 0 is reserved "no interrupt").
- 2 contexts — context 0 = M-mode, context 1 = S-mode.
- **Level-triggered** on external sources.
- Per-source priority, per-context threshold and enable bitmap.
- Claim / complete with **claimed-exclusion** — a claimed source
  does not re-pend until its `complete` write.

See [`../spec/plicGateway/SPEC.md`](../../spec/plicGateway/SPEC.md)
for the Gateway + Core split design and the level-trigger invariants.

## Register layout (offsets within the PLIC base)

| Offset | Register |
|--------|----------|
| `0x0000_0000` | Priority[source] (32-bit per source) |
| `0x0000_1000` | Pending bitmap (32 bits — one per source) |
| `0x0000_2000` | Enable bitmap, context 0 (M) |
| `0x0000_2080` | Enable bitmap, context 1 (S) |
| `0x0020_0000` | Threshold + Claim/Complete, context 0 |
| `0x0020_1000` | Threshold + Claim/Complete, context 1 |

## Update algorithm

On each bus tick the PLIC receives the current IRQ-line bitmap via
`Device::notify(irq_lines: u32)`:

```
for src in 1..32:
    if src in claimed:         continue
    if bit(irq_lines, src):    pending |= (1 << src)
    else:                      pending &= !(1 << src)   # level went low
evaluate(context 0) → MEIP
evaluate(context 1) → SEIP
```

`evaluate(ctx)` finds the highest-priority enabled pending source
above `threshold[ctx]`. If one exists, it sets MEIP/SEIP in
`irq_state`; otherwise clears.

## Claim / Complete

- **Claim read** — returns the highest-priority pending source,
  clears its pending bit, and records it in `claimed[ctx]`.
- **Complete write** — if the value matches `claimed[ctx]`, the slot
  is released and `evaluate()` reruns so a subsequent interrupt can
  re-pend.

## Direct IRQ delivery

Devices like UART hold a reference to their PLIC source slot and
signal state changes directly — no Bus round-trip. See
[`../spec/directIrq/SPEC.md`](../../spec/directIrq/SPEC.md).
