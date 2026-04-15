# `directIrq` MASTER `00`

> Feature: `directIrq`
> Iteration: `00`
> Target Plan: `00_PLAN.md`

## Decision

## Directives
### M-001
- MUST: See the MANUAL_REVIEW #7 detailly, We need to make interrupt handling asynchronous but current handle still seems synchronous.
- Reason: we may need to introduce async and await to handle asynchronous intterupt and search the web for detail about rust async.

### M-002
- MUST: Handle async cautiously which always lead to problems.

## Status
- Not Approved
- Ready for Implementation: No
