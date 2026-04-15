# `trace` MASTER `02`

> Feature: `trace`
> Iteration: `02`
> Target Plan: `02_PLAN.md`

## Decision

This PLAN seems organized more better, but some design details still leave discuss.


## Directives
### M-001
- MUST:Rename TraceState, which the name is not reasonable and not clear.
- Reason: Improve readability 

### M-002

- MUST: We shouldn't add debug_step and debug_run, since we add debug feature, the control of debug content should use by cfg but not add more api for debug, but should stub them in exiting code framework
- Reason: Reduce Backdoors and reduce break changes to exiting framework.

## Status
- Not Approved
