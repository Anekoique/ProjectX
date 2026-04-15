# `difftest` MASTER `04`

> Feature: `difftest`
> Iteration: `04`
> Target Plan: `04_PLAN.md`

## Decision

the designs leave further discussion

## Directives

### M-001

- MUST: The CoreContext should be arch dependent, which shouldn't be placed in debug.rs, which may be use such`pub use self::{RVCore as Core, trap::PendingTrap};` way to dispatch

### M-002

- MUST: We have no need to leave some structure and api, we can get enough imformation from the structure CoreContext(which you can seems as a snapshot of Core), reduce rebundant DebugOps, remove ArchSnapshot like structures. Make the whole structure more clear. The xdb should get imformation from CoreContext as much as possible, and just con't get imformation from CoreConext can add new api to DebugOps to get informations.

## Status
- Approved | Approved with Directives | Not Approved
- Ready for Implementation: Yes | No
