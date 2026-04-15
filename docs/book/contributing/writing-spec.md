# Writing a SPEC

The `SPEC.md` is the landed, canonical description of a feature.
It's extracted from the final PLAN's `## Spec` section after the
feature implementation lands.

See [`../../template/SPEC.template`](../../template/SPEC.template)
for the canonical shape.

## Sections

### `[**Goals**]`

What the feature provides, numbered `G-1`, `G-2`, ... Each goal is a
one-sentence claim about observable behaviour or a measurable
threshold.

```
- G-1: All 31 cpu-tests-rs pass with the new MMU implementation.
- G-2: Linux boots to initramfs shell in ≤ 5 seconds on the M4 host.
```

Follow Goals with Non-Goals `NG-1`, `NG-2`, ... — what the feature
explicitly does **not** cover.

### `[**Architecture**]`

A prose + diagram description of the component's shape. ASCII
diagrams are fine; keep them under 80 columns. Show the
data-flow arrows, not just boxes.

### `[**Invariants**]`

Numbered `I-1`, `I-2`, ... Properties that must hold at all times
across all execution paths.

```
- I-1: mip hardware bits are modified only via irq_state merge.
- I-2: Tick order: bus.tick → sync → check → fetch → execute → retire.
- I-3: Claimed PLIC sources are excluded from re-pending until complete.
```

### `[**Data Structure**]`

Core types — structs, enums, traits — with real Rust syntax. This
is the type-level signature of the feature.

```rust
pub struct Aclint {
    epoch: Instant,
    mtime: u64,
    msip: u32,
    mtimecmp: u64,
    irq_state: Arc<AtomicU64>,
}
```

### `[**API Surface**]`

Public function signatures and their contracts.

```rust
/// Read a word at `addr`. Returns `BadAddress` for unmapped paddrs
/// or `PageFault` for unmapped vaddrs.
pub fn checked_read(&mut self, addr: VirtAddr, size: usize) -> XResult<Word>;
```

### `[**Constraints**]`

Numbered `C-1`, `C-2`, ... Things that would look like bugs but are
intentional design boundaries.

```
- C-1: xemu internal layout matches QEMU-virt in shape; ACLINT replaces CLINT.
- C-2: Single hart (cooperative scheduler).
- C-3: UART byte-access only; word writes raise SizeMismatch.
```

## Extraction from PLAN

When a feature lands:

1. Read the final `NN_PLAN.md`.
2. Locate its `## Spec` section.
3. Copy everything between `## Spec` and the next `##` heading into
   `docs/spec/<feature>/SPEC.md`.
4. Prepend a banner:

```markdown
# `<feature>` SPEC

> Source: [`/docs/archived/<cat>/<feature>/NN_PLAN.md`](...) —
> iteration history lives under `docs/archived/<cat>/<feature>/`.

---
```

5. Commit the SPEC in the same PR as the IMPL.

## Pre-workflow features

Some features (e.g. `csr`, `klib`, `mm`) predate the template. Their
SPEC.md contains the original pre-workflow design verbatim with a
banner flagging it. **Do not rewrite** until the feature next sees
meaningful iteration — the rewrite is its own task.

## Updating a SPEC

When a feature iterates, the new PLAN's Response Matrix addresses
all prior CRITICAL / HIGH findings; the implementation lands; the
SPEC is **replaced** with the new round's `## Spec`. Never hand-edit
the SPEC in isolation.
