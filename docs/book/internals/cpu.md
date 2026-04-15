# CPU dispatch & ISA decode

## The `CPU<Core, Bus>` generic

```rust
pub struct CPU<C: CoreOps, B> {
    cores: Vec<C>,
    current: usize,
    bus: B,
}
```

- `C: CoreOps` — ISA-specific core (e.g. `RVCore`). The generic
  boundary means xemu can host multiple ISAs; a LoongArch stub
  exists in `xemu/xcore/src/arch/loongarch/`.
- `B` — the bus type. Single-hart carries `Bus` inline; multi-hart
  carries `Arc<Mutex<Bus>>` when true SMP lands (Phase 11 RFC).

## Per-instruction flow

1. **Tick devices.** `bus.tick()` advances ACLINT mtime (every step),
   drives the UART and PLIC on a slower cadence (every 64 steps), and
   collects IRQ lines.
2. **Sync interrupts.** `sync_interrupts()` copies the atomic IRQ
   bitmap into `mip`.
3. **Check pending interrupts.** If any enabled, higher-priority
   interrupt is pending, `pending_trap` is set and the rest of the
   step is skipped.
4. **Fetch.** Read the instruction word at `pc` via the MMU.
5. **Decode.** First check the decoded-instruction cache (per-hart
   4 K direct-mapped). On miss, run the pest-based pattern matcher.
6. **Execute.** Dispatch on `DecodedInst`, updating `npc` (and
   registers / CSRs / memory as side effects).
7. **Retire.** `self.pc = self.npc`. If `pending_trap` is set,
   `commit_trap()` writes the trap vector address to `npc` first.

## Decoder

`xcore/src/arch/riscv/isa/decode/` contains ~200 instruction
patterns expressed in pest. Each pattern captures the opcode fields
into a `DecodedInst::*` variant:

- `R` / `I` / `S` / `B` / `U` / `J` — standard formats
- `FR` — floating-point with explicit `rm` (rounding mode) field
- `FR4` — FMA-style 4-register (`fmadd`, `fmsub`, ...)
- `C*` — compressed variants

The match tree after decode is the dispatch loop — one big `match`
on `DecodedInst` calling per-instruction handlers.

## Decoded-instruction cache

Phase P4 of the perf roadmap:

```rust
struct ICacheLine {
    pc:      usize,      // guest virtual address
    ctx_tag: u32,        // bumped on any mapping change
    raw:     u32,        // raw instruction word (sanity)
    decoded: DecodedInst,
}

icache: [ICacheLine; 4096]   // per-hart, direct-mapped
```

`ctx_tag` invalidates implicitly on:

- `satp` writes
- `sfence.vma`
- Privilege-mode transitions that change the effective translation
- `fence.i`

Self-modifying code: every guest store invalidates the whole icache
(simple, correct, loses the icache effect only on code-writing
guests — rare). See
[`../spec/perfHotPath/SPEC.md`](../../spec/perfHotPath/SPEC.md) for
the full invariant set.

## Trace

`LOG=trace` emits one line per instruction with PC, mnemonic,
operands, and the resulting GPR delta. Very verbose — use it only
for focused debugging.
