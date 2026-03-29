# `trace` MASTER `01`

> Feature: `trace`
> Iteration: `01`
> Target Plan: `01_PLAN.md`

## Decision

The PLAN still leave further discussion, still lack of detail codes, the next PLAN should make it more detailed and complete.


## Directives
### M-001
- MUST: abstract trace's behavior to a trait or other ways to abstract for further improve and make TraceState a Scalable. Such as if enable itrace use a macro like #[cfg(feature='itrace')]register_trace!(itrace),

### M-002

- MUST: Consider deeply about R-002, consider a better design to fit or refactor current CPU runtime model
- Reason: current CPU run's step and stop model is not elegant

### M-003

- MUST: Reduce CSR read like api add to CPU, such register are arch dependent, you must abstract it to a trait or arch abstract, or you shouldn't add them to CPU
- Reason: Raise the Level of Abstraction and fit in the CPU core's arch abstract design, maybe CPU should leave some door for debug but should only a little, and should only used with some feature enabled

## Status
- Not Approved
