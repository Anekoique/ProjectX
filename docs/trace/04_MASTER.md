# `trace` MASTER `04`

> Feature: `trace`
> Iteration: `04`
> Target Plan: `04_PLAN.md`

## Decision

Current design not well, consider refactor it and design it more clear.


## Directives
### M-001
- MUST: current design of DebugContext seems too Vast and complex, consider split it and use simple structure, make the design of debug system well designed, currently, structure of 02_PLAN is the best which you can revert to learn and improve.
- Reason: Current design of different debug mechanisms are coupling which not design Clear layering

### M-002

- MUST: A structure I proposed, add trace dir to xdb, dispatch the impl to files(itrace, ftrace and mtrace),add watchpoint .rs to xdb,trace.rs to abstract the behavior of trace.

### M-003

- MUST: add a debug feature for enable all debug, and itrace, ftrace, mtrace as derive feature. And use register_trace! to enable them. 
- Reason: Clear design to debug system

## Status
- Not Approved
