# Feature Specs

Feature specifications extracted from deep-tier tasks at commit. Layout: `<feature>/SPEC.md`.

The table below is managed by `ark agent spec register` — new rows appear when a deep-tier task is committed with a promoted SPEC. **Do not hand-edit rows between the markers.** Edit outside the block, or let the CLI do it.

## Index

<!-- ARK:FEATURES:START -->
| Feature | Scope | Promoted |
| ------- | ----- | -------- |
| csr                  | CSR file + trap pipe: WARL writes, shadow registers, trap delivery.    | 2026-05-11 |
| devices              | Device trait + Bus + ACLINT / PLIC / UART / VirtIO-blk wiring.         | 2026-05-11 |
| difftest             | Per-instruction DUT/REF comparison vs QEMU GDB-RSP and Spike FFI.      | 2026-05-11 |
| direct-irq           | Lock-free PlicSignals plane + per-source IrqLine handles.              | 2026-05-11 |
| float                | F/D extension via softfloat-pure; NaN-boxed fp regs; fcsr/fflags/frm.  | 2026-05-11 |
| inst                 | RV32I/RV64I + M/A/Zicsr/C/F/D/Privileged decode + execute.             | 2026-05-11 |
| klib                 | Freestanding C library for xam-built guests (string / format / stdio). | 2026-05-11 |
| mm                   | Memory subsystem: Bus access, MMU page walk, TLB, PMP, MMIO routing.   | 2026-05-11 |
| multi-hart           | Multi-hart abstraction: HartId, per-hart state, cooperative scheduler. | 2026-05-11 |
| perf-bus-fast-path   | Owned-bus hot path: zero per-instruction lock overhead.                | 2026-05-11 |
| perf-hot-path        | Mtimer deadline gate + icache + MMU inlining + typed RAM access.       | 2026-05-11 |
| plic-gateway         | PLIC Gateway + Core split with level-triggered claim gating.           | 2026-05-11 |
| `port-to-ark` | Port project workflow to Ark | 2026-05-11 from task `port-to-ark` |

<!-- ARK:FEATURES:END -->

---

## How to use

- **Read:** scan the table; open the SPEC for any feature you'll touch.
- **Modify a feature SPEC:** append a `[**CHANGELOG**]` entry. Ark re-writes the `Promoted` column with the latest touch date.
