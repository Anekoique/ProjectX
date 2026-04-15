# `trace` MASTER `00`

> Feature: `trace`
> Iteration: `00`
> Target Plan: `00_PLAN.md`

## Decision

The PLAN have built a strong base, but it still leave many problems, you should add more detail code next round.


## Directives
### M-001
- MUST: refactor your fille organization, current organize and structure is not reasonable
- Reason: the trace and debug function shouldn't placed in xcore, but may be the function of xdb which have the function to monitor the behavior of xcore. And some of the function should use the rust feature to enable.

### M-002

- MUST: Search web or crates.io to reuse some of usable crate to improve code quality which can make code more clean. concise and elegant.
- Reason: Some of structure maybe rebundant which can be replaced by external crate

### M-003

- MUST: Some of xdb commands maybe need to discuss more, shuch as info m maybe replaced by x? learn from gdb
- Reason: improve readability

## Status
- Not approved
