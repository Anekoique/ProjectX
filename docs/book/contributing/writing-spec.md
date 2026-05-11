# Writing a SPEC

The `SPEC.md` is the landed, canonical description of a feature. For
deep-tier tasks, it is extracted from the final PLAN's `## Spec`
section automatically by `ark agent task commit`.

The canonical template lives at
[`/.ark/templates/SPEC.md`](../../../.ark/templates/SPEC.md). The
seven sections are:

## `[**Goals**]`

What the feature provides, numbered `G-1`, `G-2`, ... Each goal is a
one-sentence verb-led claim about user-visible capability. Soft cap
of 5 goals.

```
- G-1: All 31 cpu-tests-rs pass with the new MMU implementation.
- G-2: Linux boots to initramfs shell in ≤ 5 seconds on the M4 host.
```

## `[**Non-goals**]`

Numbered `NG-1`, `NG-2`, ... Only list a non-goal when a reasonable
reader would assume it is in scope. Soft cap of 3.

```
- NG-1: No A/D-bit emulation under `senvcfg.ADUE` (hardware A/D only).
```

## `[**Architecture**]`

Module / file layout with a one-line note per file. Prefer a tree or
diagram; avoid prose narration. Keep diagrams under 80 columns.

## `[**Data Structure**]`

Public types only — `struct`, `enum`, `trait`. Field names + types +
a one-line comment when meaning is non-obvious.

```rust
pub struct Aclint { /* see device/aclint.rs */ }
```

## `[**API Surface**]`

Public function signatures and one-line semantics. No bodies.

```rust
pub fn checked_read(&mut self, addr: VirtAddr, size: usize) -> XResult<Word>;
```

## `[**Constraints**]`

Numbered `C-1`, `C-2`, ... Invariants the implementation must hold.
One declarative sentence each, ≤120 chars. Cite a source of truth
(file path, test, constant) inline with an em-dash:

```
- C-1: mip hardware bits are modified only via irq_state merge — `xemu/xcore/src/device/irq.rs`.
- C-2: UART byte-access only; word writes raise SizeMismatch — `tests/uart_byte_access.rs`.
```

## `[**CHANGELOG**]`

Appended only when a later task modifies this SPEC. New SPECs
(extracted from a deep-tier PLAN at commit) start with this section
empty (or with the migration / promotion pointer if the SPEC was
ported from a pre-workflow source).

```
- 2026-05-11 port-to-ark: migrated from running-notes SPEC; full original preserved at `.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md`.
```

## Extraction from PLAN (deep tier)

`ark agent task commit` does this automatically: it locates the
latest `NN_PLAN.md`, extracts everything between `## Spec` and the
next `##` heading, and writes it verbatim to
`.ark/specs/features/<slug>/SPEC.md`. It also appends a row to
`.ark/specs/features/INDEX.md` between the `ARK:FEATURES` markers.
All of this lands in the closing commit.

## Pre-workflow features

A handful of features predate the seven-section template (the
running-notes SPECs for `csr`, `klib`, `mm`, `mem-opt`, `err2trap`).
Their migrated SPECs collapse the long-form prose into the template;
the original is preserved verbatim at
`.ark/tasks/archive/legacy/<slug>/SPEC_LEGACY.md` and referenced from
the migrated CHANGELOG.

## Updating an existing SPEC

A later deep-tier task touching the same slug iterates the SPEC by
landing a new PLAN whose `## Spec` block re-states everything in
full. `ark agent task commit` then overwrites the existing
`SPEC.md` and appends a `[**CHANGELOG**]` entry describing what
changed. Never hand-edit a SPEC in isolation.
