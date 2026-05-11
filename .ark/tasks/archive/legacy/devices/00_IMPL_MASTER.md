# `Device Emulation` IMPL MASTER `00`

> Status: Active | Resolved
> Feature: `dev`
> Iteration: `00`
> Owner: Master
> Target Impl: `00_IMPL.md`
> Related Review: `00_IMPL_REVIEW.md`

---

## Decision

{Final judgment for this implementation round.}

## Directives

### IM-001 
- Decision: The device AClint and PLIC are arch related which should be abstracted
- Scope: Fix
- Requirement: MUST

### IM-002
- Decision: reduce or remove the hard-encoding in plic.rs `read` and `write`
- Scope: Fix
- Requirement: SHOULD
- Reason: improve readability and code quality.

### IM-003

- Decision: use a type to replace irq_state
- Scope: Fix
- Requirement: SHOULD
- Reason: improve readability and code quality.

### IM-004

- Decision: should we really need to introduce XError::ProgramExit? which break the previous step framework
- Scope: Fix
- Requirement: MAYBE
- Reason: reduce breakchanges to framework

### IM-005

- Decision: Fix clippy error/warnings
- Scope: Fix
- Requirement: MUST
- Reason: we shouldn't leave clippy warnings, and test finisher should move to the test framework entirely 

## Status

- Implementation Status: Accepted | Accepted with Directives | Not Accepted
- Ready for Commit: No