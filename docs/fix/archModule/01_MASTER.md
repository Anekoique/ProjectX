# `{Feature Name}` MASTER `{NN}`

> Feature: `{feature-name}`
> Iteration: `{NN}`
> Target Plan: `{NN_PLAN.md}`

## Decision

make sure keep code clean, concise and elegant.

## Directives
### M-001
- MUST: rename selected to a more elegant name which can be more obviously
- Reason: currently the name `selected` not seems good. Consider rename to a proper name or just delete it just can dispatch directly like use arch::device?

### M-002
- MUST: make sure keep code clean, concise and elegant.

### M-003
- MUST: No need to add the check of arch which `build.rs` will handle which rebundant

### M-004(Critical)
- MUST: The CPU/device/isa dir should only a litter `#cfg(arch)` related patch which arch related behaviour should all dispached by trait and handled at `ARCH mod` the leaved in the dir of CPU/isa/device should all be high level abstracted API that can be used directly which should introduce arch related patch to reimport.(critical)

## Status
- Not Approved
- Ready for Implementation: No