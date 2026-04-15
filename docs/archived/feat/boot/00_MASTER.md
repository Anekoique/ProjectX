# `OS Boot` MASTER `00`

> Feature: `boot`
> Iteration: `00`
> Target Plan: `00_PLAN.md`

## Decision

See the opinions from REVIWER and MASTER, generate a PLAN and step to impl it. Make code clean, concise and elegant.

## Directives

### M-001

- MUST: The boot logic shouldn't placed into xemu Makefile, which should placed out as new Makefile which are external kernel, consider better position to place it.

### M-002

- MUST: Before impl , you should consider the problems REVIWER proposed.

### M-003

- MUST: To boot successfully, you may add some modifications, you shouldn't break any code framework, and if you add some changes, you should make sure it's clean and clear.


## Status
- Approved
- Ready for Implementation: Yes
