# Traps & interrupts

## Two-phase trap handling

xemu uses a single-commit-point model: architectural state only
changes when `step()` commits at the retire stage.

**Phase 1 (raise):** an instruction handler or subsystem detects a
trap condition and sets `pending_trap`. Control returns up the stack
via `Ok(())`.

**Phase 2 (commit):** after execute, `step()` inspects `pending_trap`;
if set, `commit_trap()` updates `mepc`/`sepc`, `mcause`/`scause`,
`mtval`/`stval`, `mstatus`/`sstatus` (MPP/MPIE), sets `privilege`,
and writes `self.npc = trap_vector`.

The loop then commits: `self.pc = self.npc`.

## `PendingTrap`

```rust
pub struct PendingTrap {
    pub cause: TrapCause,
    pub tval: Word,
}
```

One canonical representation. Never carried inside `XError` for
architectural traps (see the `err2trap` discussion in
[`csr.md`](./csr.md)).

## Delegation

xemu implements the full `medeleg` / `mideleg` model:

- Faults at U-mode may be delegated to S-mode.
- Timer / software / external interrupts are routed via `mideleg`.
- Delegation happens in `commit_trap` — handlers just supply the
  `TrapCause`.

Vectored mtvec / stvec is supported: when `mtvec[1:0] = 1`, async
interrupts dispatch to `base + 4 * cause`; synchronous traps always
jump to `base`.

## Interrupt priority

Per the spec:

```
MEI > MSI > MTI > SEI > SSI > STI
```

`check_pending_interrupts()` walks in priority order, masked by
`mie` / `sie` / the global enable bit (`mstatus.MIE` / `sstatus.SIE`).

## Lock-free IRQ plane

Devices raise interrupts by flipping bits in a shared
`Arc<AtomicU64>` (`IrqState`). The CPU merges this into `mip` at
the top of each step. No locks; no vtable downcasts from the Bus
into the PLIC.

Device-to-PLIC is **direct**: the UART holds a reference to the
PLIC's source slot (`PlicSource`) and flips it on state change. No
Bus-mediated round-trip. This is the `directIrq` fix; see
[`../spec/directIrq/SPEC.md`](../../spec/directIrq/SPEC.md).

## Edge vs level

- **ACLINT MSIP / MTIP / SSIP** — level-triggered by bit state.
- **UART** — level; `!rx_fifo.is_empty() && (ier & 1)`.
- **PLIC** — level on its external sources; claim/complete exclusion
  prevents re-pending until the handler completes. `plicGateway`
  fixed a prior edge/level confusion; see
  [`../spec/plicGateway/SPEC.md`](../../spec/plicGateway/SPEC.md).
