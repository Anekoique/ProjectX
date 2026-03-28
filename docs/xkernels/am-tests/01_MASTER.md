# `am-tests` MASTER `01`

> Feature: `am-tests`
> Iteration: `01`
> Target Plan: `01_PLAN.md`

## Decision

We still need to do a lot improvement regarding M-001 ~ M-003

## Directives

### M-001

- MUST: The introduce of am-tests should improve xam at the same time which should provide basic functions.
- Reason: The xam can seen as the abstract hal of an OS, or a platform adapted to bare-metel apps which learn from the concepts of abstract-machine. So you can seen it as a hal or seen it as a Smallest unikernel. The design or development you can learn from the design of xos/xark-core, which is the core of an OS which can be leaned from xam.

## M-002

- MUST: You should organize your tests better. which you named them timer-read which is a function but msip named as a reg which is vague.
- Reson: Organize am-tests more better and design a better framework.

### M-003

- MUST: Make sure your C code clean, concise and elegant
- Reason: improve code quality

## Status

- Not Approved
