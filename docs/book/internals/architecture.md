# Architecture overview

## The step loop

`CPU::step` is the per-instruction driver:

```
CPU::step()
  1. bus.tick()                   — ACLINT every step, UART/PLIC every 64
  2. sync_interrupts()             — merge irq_state → mip
  3. check_pending_interrupts()    — raise trap if priority/gating allows
  4. fetch → decode (icache) → execute
  5. retire + commit_trap()        — commit npc, enter trap vector if any
```

The loop owns the `Bus` directly — no `Arc<Mutex<Bus>>`. Field-level
borrow splitting lets MMU and Bus be accessed simultaneously without
locking. This is Phase P1 of the perf roadmap; see
[`performance.md`](./performance.md).

## Dispatch diagram

```
                ┌──────────────────────────────────────────────┐
                │                 xdb::main                    │
                │  (monolithic under LTO + codegen-units = 1)  │
                └───────────────┬──────────────────────────────┘
                                │
                    ┌───────────▼────────────┐
                    │  CPU<Core, Bus>        │
                    │   ├── Core: CoreOps    │   ← arch-agnostic trait
                    │   └── Bus: owned       │
                    └───────────┬────────────┘
                                │
                    ┌───────────▼────────────┐
                    │  RVCore (CoreOps impl) │
                    │   ├── GPR / PC / NPC   │
                    │   ├── csr: CsrFile     │
                    │   ├── privilege        │
                    │   ├── mmu: Mmu         │
                    │   ├── pmp: Pmp         │
                    │   ├── icache           │
                    │   └── pending_trap     │
                    └───────────┬────────────┘
                                │
                    ┌───────────▼──────────────────────────┐
                    │  Bus                                 │
                    │   ├── Ram [0x8000_0000, 1 GiB]        │
                    │   ├── ACLINT [0x0200_0000]            │
                    │   ├── PLIC   [0x0C00_0000]            │
                    │   ├── UART   [0x1000_0000]            │
                    │   ├── VirtIO [0x1000_1000]            │
                    │   └── Test   [0x0010_0000] (test-only)│
                    └──────────────────────────────────────┘
```

## Four-layer memory access

A guest load/store walks:

```
vaddr ─► align check ─► MMU.translate ─► paddr ─► PMP.check ─► Bus.access
                             │                         ▲
                             └── page walk:  pte_paddr ┘  (PMP checks PTE reads too)
```

Responsibility split (see
[`../spec/mm/SPEC.md`](../../spec/mm/SPEC.md) for the canonical table):

| Layer | Knows about | Does NOT know about |
|-------|-------------|---------------------|
| `Bus` | Physical addresses, device regions | Virtual addresses, privilege, traps, PMP |
| `Mmu` | Page tables, TLB, PTE bits, SUM / MXR | Trap codes, PMP (receives `&Pmp` for walks) |
| `Pmp` | Physical-address permissions, privilege | Virtual addresses, page tables |
| `RVCore` | Orchestrates: privilege, MPRV, trap mapping | Internal device state |

## Lock-free IRQ delivery

Devices raise interrupts through `IrqState` — a shared `Arc<AtomicU64>`
bitmap. Each bit maps to an `mip` hardware bit:

| Bit | Source |
|-----|--------|
| 1 | SSIP (ACLINT SSWI) |
| 3 | MSIP (ACLINT MSWI) |
| 7 | MTIP (ACLINT MTIMER) |
| 9 | SEIP (PLIC context 1) |
| 11 | MEIP (PLIC context 0) |

`sync_interrupts()` merges this into the CPU's `mip` register at the
top of each step. No locks, no downcasts.

## Related reading

- [CPU dispatch & ISA decode](./cpu.md)
- [CSR subsystem](./csr.md)
- [Memory: MMU, TLB, PMP](./memory.md)
- [Traps & interrupts](./traps.md)
- [Devices](./devices.md)
- [Performance: hot path & baselines](./performance.md)
