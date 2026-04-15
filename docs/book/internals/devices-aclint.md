# ACLINT (MSWI / MTIMER / SSWI)

`ACLINT` replaces the legacy CLINT with three cleanly-split sub-devices
sharing the `0x0200_0000` / `0x1_0000` region. Wire-compatible with
the CLINT layout for software that expects one.

See the split spec at
[`../spec/aclintSplit/SPEC.md`](../../spec/aclintSplit/SPEC.md).

## MSWI — Machine Software Interrupt

- `msip` at offset `0x0000` — bit 0 only.
- Writing 1 sets MSIP in `irq_state`; writing 0 clears it.

## MTIMER — Machine Timer

- `mtime` at `0xBFF8` (lo) / `0xBFFC` (hi) — host wall clock at
  10 MHz. `timebase-frequency = 10_000_000`.
- `mtimecmp` at `0x4000` (lo) / `0x4004` (hi).
- **Amortized sync:** wall-clock samples are taken every 512 ticks,
  not every step (Phase P1-era optimisation).
- **Deadline gate:** per-step `tick()` short-circuits when
  `self.mtime < self.next_fire_mtime` (Phase P3).
- When `mtime ≥ mtimecmp`, MTIP is raised in `irq_state`.

## SSWI — Supervisor Software Interrupt

- `setssip` at `0xC000` — write-only.
- Writing 1 sets SSIP in `irq_state`.
- Read always returns 0.

## Sstc extension

xemu exposes `stimecmp` for Sstc — an S-mode direct timer register.
The xemu DTS advertises `riscv,isa = "rv64imafdcsu_sstc"`; Linux and
OpenSBI use Sstc when present, bypassing the SBI timer call.
