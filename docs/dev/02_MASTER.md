# Device MASTER 02

> Feature: device
> Iteration: 02
> Target Plan: 02_PLAN.md

## Decision

This round have excellent improvement which have good PLAN docs structure organize and prmote for a big step.

## Directives

### M-001

- MUST: improve code quality, Use functional programming techniques judiciously to make the implementation more concise and elegant.
- Reason:  Make code more clean, concise and elegant 

### M-002

- MUST: use qemu like address layout
- Reason: Unified Terminology and Standards

### M-003

- MUST: consider introduce MMIO registers and use macro or trait to abstract their behavior(like the design of CSR)
- Reason: currently use PlicReg, AclintReg are rebundant

### M-004

- MAYBE: remove test from mainline, consider use them at test
- Reason: consider the behavior of other emulators, I'm not sure should we use test in the mainline directly

## Status

- Not Approved
- No