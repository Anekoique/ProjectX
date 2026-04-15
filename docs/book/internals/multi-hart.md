# Multi-hart

Today's multi-hart is a **single-threaded cooperative round-robin
scheduler** in `CPU::step`. N harts are no faster than 1. The
abstraction exists so the ISA code can reason about per-hart state,
not because the host is running them in parallel.

See [`../spec/multiHart/SPEC.md`](../../spec/multiHart/SPEC.md) for
the Hart abstraction design.

## What's shared, what's per-hart

| Shared | Per-hart |
|--------|----------|
| `Bus` (RAM + all devices) | GPR / PC / NPC |
| ACLINT mtime (one host wall-clock source) | `CsrFile` |
| PLIC state (2 contexts route to 2 harts) | `privilege` |
| IrqState `Arc<AtomicU64>` (one set of mip/mie bits per hart) | `mmu`, `pmp` |
| | `icache` |
| | `pending_trap` |

## Per-hart icache

Each hart has its own 4 K direct-mapped decoded-instruction cache. A
`satp` write on one hart does **not** flush the other hart's icache
— each has its own `ctx_tag`. `sfence.vma` with an explicit hart
target would too, but the current implementation flushes both harts
on any `sfence.vma` for simplicity (conservative, correct).

## Running

```bash
cd resource
make linux-2hart         # 2 harts, cooperative scheduler
make debian-2hart        # same, with VirtIO rootfs
```

Both cores share the same Bus instance. The scheduler gives each
hart a slice of steps in round-robin order before rotating.

## Why single-threaded today

P1 (`busFastPath`) removed the `Arc<Mutex<Bus>>` that was dead weight
under the cooperative scheduler — there's no real SMP, so the mutex
was pure overhead. Removing it gave 45–52 % wall-clock.

## True SMP (Phase 11 RFC)

Not in any landed phase. To get per-hart OS threads:

- Guest RAM becomes `&[AtomicU8]` (or `unsafe` typed access with
  explicit fences).
- LR/SC reservations become per-hart `AtomicUsize`.
- Per-device fine-grained sync (or the QEMU MTTCG "BQL on MMIO
  only" model).
- A runtime that joins / cancels hart threads cleanly.

None of this fits in the perf roadmap. See
[`../PROGRESS.md`](../../PROGRESS.md) §Phase 11 for reference designs
(QEMU MTTCG, rv8, Guo 2019 on fast TLB simulation).

## Pre-conditions before opening Phase 11

- P1, P2 (bus-access API), P5 (MMU inline) shipped. Done.
- A reproducible 2-hart Linux benchmark in
  `docs/perf/baselines/<date>/` showing the fraction of time
  actually parallelisable. **Not yet measured.**
- P7 re-profile results.
