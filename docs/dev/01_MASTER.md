# Device MASTER 01

> Feature: device
> Iteration: 01
> Target Plan: 01_PLAN.md

## Decision

This round introduce more details, but the design and code details still need to discuss more.

## Directives

### M-001

- SHOULD: current bus.tick() use synchronous to poll and update the state of interrupt and device, which is proper for current, but it's violate to real hardware, in the future it should modify to async. 
- Reason: emulate real hardware and better performance 

### M-002

- MUST: The trait BUS must abstract all device behavior, shouldn't add func which only one or two device use
- Reason: the trait BUS is abstract behavior of devices

### M-003

- MUST: reduce hard-encoding and introduce enums, but you also should keep structs/enum clean and concise
- Reason: make code more readable and concise

### M-004

- MUST: consider better naming, use short but meaningful item in structs and enums
- Reason: currently, some items are vague

### M-005

- MUST: include phase 4B into plan.
- Reason: Make the device emulation more detailed for this feature support

### M-006

- MUST: consider replace CLINT with ACLINT
- Reason: ACLINT is new standard for use

## Status

- Not Approved
- No