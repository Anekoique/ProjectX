# Device MASTER 00

> Feature: device
> Iteration: 00
> Target Plan: 00_PLAN.md

## Decision
The plan build a good baseline but need to extend to more detail. More design details need to be discussed. You should add more function details in the next round. You should give function detail in`Implementation Plan` which is only rough path currently. 

## Directives
### M-001
- MUST: No need to add type ExtIp and DeviceIrq
- Reason: rebundant

### M-002
- MUST: replace raw array with Vector or other proper data structures 
- Reason: more elegant and convenient to operate

### M-003

- SHOULD: foe T-3, we'd better use TCP / PTY
- Reason: avoid more problem.

### M-004

- MUST: more detail function design
- Reason: currently only give the api but no details in implementations

## Status
- Not Approved
- No
