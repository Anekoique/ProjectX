# `Debian Boot` MASTER `03`

> Feature: `debian`
> Iteration: `03`
> Target Plan: `03_PLAN.md`

## Decision

Bad Coding Quality, use /pua skill to improve your codings.

## Directives

### M-001

- MUST: read_u16,u32 ... such functions should been abstracted to make it more clear and elegant

### M-002

- MUST: too much const, some of them should be organized to enum which you should judge which one can be such handled

### M-003

- MUST: Too much items in VirtioBlock which lead it seems Bloat and Redundancy, make it more clean with clear structure, you can search the web for suitable crates

### M-004

- MUST: you can learn the impl from @~/Emulators

### M-005

- MUST: Don't write process_dma such ugly functions

### M-006

- MUST: Improve your code quality, Appropriate use of functional expressions. Every impl and codes should refer to our code framework and make it clean, concise and elegant

## Status
- Not Approved
- Ready for Implementation: No
