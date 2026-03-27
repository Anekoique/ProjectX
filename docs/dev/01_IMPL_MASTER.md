# `Device Emulation` IMPL MASTER `01`

> Status: Active | Resolved
> Feature: `dev`
> Iteration: `01`
> Owner: Master
> Target Impl: `01_IMPL.md`
> Related Review: `01_IMPL_REVIEW.md`

---

## Decision

Still need to improve.

## Directives

### IM-001 

- Decision: The device AClint and PLIC are arch related which should be abstracted
- Scope: Fix
- Requirement: MUST
- Reason: the ACLint and PLIC all risk related devs, consider organize the directory of dev more better(like the organize of CPU), you can do high level abstraction and modelization is better

### IM-002

- Decision: Make code more clean, concise and elegant
- Scope: Optimization
- Requirement: MUST
- Reason: improve readability and code quality.

### IM-003

- Decision: Fix the error/warning the reviewer proposed(IR-001-IR-004)
- Scope: Fix
- Requirement: MUST
- Reason: Fix problems.

### IM-004

- Decision: We need to add real bare-metal applications to verify the correctness of the implementations
- Scope: Optimize
- Requirement: MAYBE
- Reason: We haven't tests in Real-world Scenario

## Status

- Implementation Status: Not Accepted
- Ready for Commit: No