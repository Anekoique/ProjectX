# `{Feature Name}` MASTER `{NN}`

> Feature: `{feature-name}`
> Iteration: `{NN}`
> Target Plan: `{NN_PLAN.md}`

## Decision

Need more discussion.

## Directives
### M-001
- MUST: keep cfg-if
- Reason: don't introduce ARCH trait which is too high level, we need control the arch behaviour fine-grained with core-ops like.

### M-002
- MUST: rename irq_bits.rs with irq.rs
- Reason: evetry file or behaviour in arch directory should highly been abstracted by the upper dir of cpu/isa/device
  The file in arch should be topic/theme leaded or abstracted them in riscv/trap/intterupt or riscv/csr.

## Status
- Not Approved
- Ready for Implementation: No