# `am-tests` MASTER `02`

> Feature: `am-tests`
> Iteration: `02`
> Target Plan: `02_PLAN.md`

## Decision

Some of code still low quality and the framework need to be improved.

## Directives

### M-001

- MUST: organize the xemu which can learn from xos/xark-core

### M-002

- MUST: the trap dispatch and entry can be designed for better which you can learn from xos/xark-core. shuch as abstract interrupt and trap entry... consider see the design of trap.rs
- Reason: current design just usable but not elegant which can be improved

### M-003

- MUST: we can add a main entry for am-tests to choose a test to run which can learn from nemu's am-kernels/am-tests
- Reason: dispatch tests make it easy to test mannually

### M-004

- MUST: Make sure both your rust and C code clean, concise and elegant
- Reason: improve code quality

## Status

Not Approved
