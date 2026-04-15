# `difftest` MASTER `03`

> Feature: `difftest`
> Iteration: `03`
> Target Plan: `03_PLAN.md`

## Decision

## Directives

### M-001

- MUST: just use single cfg(feature) to enable difftest mod, remove the rebundant cfgs

### M-002

- MUST: Consider remote import structure from xcore.xcore should only use api pass basic information to debug target, which shouldn't expose more to external components. Or just construct a CoreContext structure pass out for xdb debug or difftest , which make the structure more clear, and use a debug trait for the CoreContext. dispatch the CoreContext to different ARCH such as the design of RVCore, the Context should be light enough to pass on.

## Status
- Not Approved
- Ready for Implementation: No
