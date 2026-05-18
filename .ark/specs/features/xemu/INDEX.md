# `xemu` Feature Specs

Subtree index for the xemu RISC-V emulator. Rows are managed by `ark agent task commit`'s leaf-to-root walk. Edit prose outside the markers freely; the rows between the markers are auto-maintained.

## Index

<!-- ARK:FEATURES:START -->
| Feature                      | Scope                                                                  | Promoted   |
| ---------------------------- | ---------------------------------------------------------------------- | ---------- |
| `csr/SPEC.md`                | CSR file + trap pipe: WARL writes, shadow registers, trap delivery.    | 2026-05-11 |
| `devices/SPEC.md`            | Device trait + Bus + ACLINT / PLIC / UART / VirtIO-blk wiring.         | 2026-05-11 |
| `difftest/SPEC.md`           | Per-instruction DUT/REF comparison vs QEMU GDB-RSP and Spike FFI.      | 2026-05-11 |
| `direct-irq/SPEC.md`         | Lock-free PlicSignals plane + per-source IrqLine handles.              | 2026-05-11 |
| `float/SPEC.md`              | F/D extension via softfloat-pure; NaN-boxed fp regs; fcsr/fflags/frm.  | 2026-05-11 |
| `inst/SPEC.md`               | RV32I/RV64I + M/A/Zicsr/C/F/D/Privileged decode + execute.             | 2026-05-11 |
| `mm/SPEC.md`                 | Memory subsystem: Bus access, MMU page walk, TLB, PMP, MMIO routing.   | 2026-05-11 |
| `multi-hart/SPEC.md`         | Multi-hart abstraction: HartId, per-hart state, cooperative scheduler. | 2026-05-11 |
| `perf-bus-fast-path/SPEC.md` | Owned-bus hot path: zero per-instruction lock overhead.                | 2026-05-11 |
| `perf-hot-path/SPEC.md`      | Mtimer deadline gate + icache + MMU inlining + typed RAM access.       | 2026-05-11 |
| `plic-gateway/SPEC.md`       | PLIC Gateway + Core split with level-triggered claim gating.           | 2026-05-11 |
<!-- ARK:FEATURES:END -->

---

## How to use

- **Read:** scan the table; open the SPEC for any xemu feature you'll touch.
- **Modify a feature SPEC:** append a `[**CHANGELOG**]` entry. Ark re-writes the `Promoted` column with the latest touch date.
