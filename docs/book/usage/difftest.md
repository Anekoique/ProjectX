# Differential testing (QEMU / Spike)

Per-instruction comparison of xemu (the DUT) against a reference
implementation (QEMU or Spike). On any divergence, xdb halts and
reports the first mismatched register.

## Enabling

```bash
DIFFTEST=1 make run
(xdb) dt attach qemu       # or: dt attach spike
(xdb) c
```

## What gets compared

- **PC**
- **GPRs** (x0..x31)
- **Current privilege** (M / S / U)
- **14 whitelisted CSRs** (masked) — auto-generated from the
  `csr_table!` macro's `@ difftest` annotation.

## MMIO handling

MMIO reads are intentionally non-deterministic (wall-clock, interrupt
state). Difftest skips instructions that touch MMIO and **syncs
raw values** from DUT to REF to keep them aligned.

## Backends

### QEMU

- Protocol: GDB Remote Serial over TCP.
- Config: `sstep=0x7` (NOIRQ + NOTIMER), `PhyMemMode:1`.
- Initial state is synced once at attach.
- Easy to reproduce on any host that has `qemu-system-riscv64`.

### Spike

- Protocol: FFI into `libriscv`, wrapped by `tools/difftest/spike/`.
- Links `libriscv` + `libsoftfloat` + `libfesvr` + `libdisasm`.
- Closer to the ISA reference; harder to set up than QEMU.
- Used as the tiebreaker when QEMU and xemu disagree.

## Known limitations

- Difftest **cannot** run in `DEBUG=y` mode — PTY timing perturbs the
  reference.
- Very long runs amortize slowly; prefer reproducing divergences on a
  focused test kernel.
- Not yet wired into CI; tracked as a deferred item in
  [`../PROGRESS.md`](../../PROGRESS.md) Phase 6.
