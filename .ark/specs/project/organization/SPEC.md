# Organization Guidelines

> Rules for how code is laid out inside a crate: how a subsystem becomes a directory, how a directory becomes a `mod.rs` plus siblings, where architecture-specific code goes, and how the public surface of a subsystem is exposed.

## R1 — A subsystem is a directory containing `mod.rs`, not a single file at the parent level

**Applies to:** `**/src/**`
**Evidence:** VERIFY

A subsystem is a coherent area of functionality (memory management, scheduling, interrupt handling). Promote a leaf `.rs` file to a directory the moment a second sibling concept needs to live next to it. Resist creating a directory for a single file.

```text
// Bad
ostd/src/mm.rs            ← one file holding many concepts

// Good
ostd/src/mm/
├── mod.rs
├── frame/
├── page_table/
└── vm_space.rs
```

## R2 — `mod.rs` opens with a module-level doc comment explaining the subsystem

**Applies to:** `**/mod.rs`
**Evidence:** `clippy::missing_docs` (with `#![warn(missing_docs)]` on the crate)

A `mod.rs` whose first non-attribute item is `pub mod foo;` is missing its most important documentation. The doc comment is the entry point a new reader lands on.

```rust
// Bad
// SPDX-License-Identifier: MPL-2.0
pub mod frame;
pub mod page_table;

// Good
// SPDX-License-Identifier: MPL-2.0

//! Memory management.
//!
//! A frame is an aligned, contiguous range of bytes in physical memory.
//! Frames are accessed through reference-counted [`Frame`] handles. ...

pub mod frame;
pub mod page_table;
```

## R3 — `mod.rs` re-exports the subsystem's public surface through a single `pub use` block

**Applies to:** `**/mod.rs`
**Evidence:** VERIFY

The `pub use` block is the subsystem's contract with the rest of the crate. Anything not listed there is an implementation detail. Callers outside the subsystem import from the subsystem module, not from its child files.

```rust
// Bad — caller reaches into a child file
use crate::mm::frame::Frame;
use crate::mm::frame::allocator::FrameAllocOptions;

// Good — re-exports define the contract
// in mm/mod.rs:
pub use self::{
    frame::{Frame, allocator::FrameAllocOptions, segment::Segment},
    page_prop::{CachePolicy, PageFlags, PageProperty},
    vm_space::VmSpace,
};

// caller:
use crate::mm::{Frame, FrameAllocOptions};
```

## R4 — Crate-internal subsystems are declared `pub(crate) mod`, not `pub mod`

**Applies to:** `**/mod.rs`, `**/lib.rs`
**Evidence:** VERIFY

A bare `pub mod foo;` makes `foo` part of the crate's external API. Use `pub(crate) mod` for modules that exist only to support other parts of the same crate.

```rust
// Bad
pub mod kspace;       // not part of the crate's external API
pub mod page_table;

// Good
pub(crate) mod kspace;
pub(crate) mod page_table;
```

## R5 — One concept per leaf `.rs` file

**Applies to:** `**/src/**/*.rs`
**Evidence:** VERIFY

Each leaf `.rs` file holds one concept: one major data structure, one trait and its blanket impls, or one tightly coupled cluster of helpers. Filenames name the concept (`frame_ref.rs`, not `references.rs`).

```text
// Bad
ostd/src/mm/frame.rs       ← Frame, FrameAllocOptions, FrameRef,
                             Segment, UniqueFrame all in one file

// Good
ostd/src/mm/frame/
├── mod.rs          ← Frame
├── allocator.rs    ← FrameAllocOptions
├── frame_ref.rs    ← FrameRef
├── segment.rs      ← Segment
└── unique.rs       ← UniqueFrame
```

## R6 — Architecture-specific code lives under `arch/<name>/` using canonical short names

**Applies to:** `**/src/arch/**`
**Evidence:** VERIFY

`<name>` is the canonical short name (`x86`, `riscv`, `loongarch`), not the target-triple form (`x86_64`, `riscv64`). Generic code must not `cfg!`-branch on `target_arch` mid-function; push the variation down into the `arch` module.

```text
// Bad
ostd/src/arch/x86_64/
ostd/src/arch/riscv64/

// Good
ostd/src/arch/x86/
ostd/src/arch/riscv/
ostd/src/arch/loongarch/
```

## R7 — Each `arch/<name>/` mirrors the generic subsystem layout above it

**Applies to:** `**/src/arch/**`
**Evidence:** VERIFY

Architecture-specific MMU code lives in `arch/<name>/mm/`, architecture-specific timer code in `arch/<name>/timer/`, and so on. A reader who knows where a generic concept lives can predict where its architecture-specific implementation is.

```text
// Bad
ostd/src/arch/x86/
├── mod.rs
├── memory.rs        ← name diverges from generic mm/
└── interrupts.rs    ← name diverges from generic irq/

// Good
ostd/src/arch/x86/
├── mod.rs
├── mm/              ← matches ostd/src/mm
├── irq/             ← matches ostd/src/irq
└── timer/           ← matches ostd/src/timer
```

## R8 — The active arch is selected by a single `cfg_attr` path attribute on `pub mod arch`

**Applies to:** `**/lib.rs`
**Evidence:** grep

This is the only place in the crate where `target_arch` should appear at top level. Avoid scattering `#[cfg(target_arch = ...)]` blocks throughout subsystem code.

```rust
// Bad
#[cfg(target_arch = "x86_64")]
mod arch_x86;
#[cfg(target_arch = "riscv64")]
mod arch_riscv;

// Good
#[cfg_attr(target_arch = "x86_64",      path = "arch/x86/mod.rs")]
#[cfg_attr(target_arch = "riscv64",     path = "arch/riscv/mod.rs")]
#[cfg_attr(target_arch = "loongarch64", path = "arch/loongarch/mod.rs")]
pub mod arch;
```

## R9 — Subsystem tests live in a sibling `test.rs` declared with `#[cfg(ktest)]`

**Applies to:** `**/src/**`
**Evidence:** grep

Tests for a subsystem live in `test.rs` inside the subsystem directory, not scattered as `#[cfg(test)]` blocks through production source files.

```rust
// Bad — test functions interleaved with production code
pub fn allocate(...) -> Frame { ... }

#[cfg(test)]
fn test_allocate() { ... }

pub fn deallocate(...) { ... }

// Good
// in mod.rs:
#[cfg(ktest)]
mod test;

// in sibling test.rs:
use super::*;
#[ktest]
fn allocate_returns_aligned_frame() { ... }
```

## R10 — Top-level subsystems are flat siblings directly under `src/`

**Applies to:** `**/src/`
**Evidence:** VERIFY

Do not create a wrapper directory like `src/subsystems/` or `src/components/` to hold them. A reader scanning a crate root sees the subsystem list immediately.

```text
// Bad
visor/src/
├── lib.rs
└── subsystems/
    ├── vcpu/
    ├── memory/
    └── scheduler/

// Good
visor/src/
├── lib.rs
├── vcpu/
├── memory/
└── scheduler/
```

## R11 — Group subsystems hierarchically only when several closely related sub-areas share types

**Applies to:** `**/src/**`
**Evidence:** VERIFY

Introduce a parent directory only when several sub-areas benefit from sharing a parent's `mod.rs` for cross-cutting types. A subsystem with one or two sub-areas keeps them as flat sibling files.

```text
// Bad — wrapping a single sub-area in its own directory
ostd/src/heap/
└── allocator/
    └── mod.rs       ← only thing in heap/

// Good — parent justified when sub-areas share types
ostd/src/mm/
├── frame/
├── page_table/
├── dma/
├── heap/
└── io/
```

## R12 — `lib.rs` contains crate metadata and module declarations only

**Applies to:** `**/lib.rs`
**Evidence:** VERIFY

`lib.rs` should be short and structural: SPDX header, crate-level attributes (`no_std`, feature gates, `deny(unsafe_code)` where applicable), the `cfg_attr` arch selector, plain `pub mod`/`mod` declarations, a small set of `pub use` re-exports forming the prelude, and the crate's top-level entry function. Subsystem logic lives in subsystems, not at the crate root.

```rust
// Bad — subsystem logic inlined in lib.rs
#![no_std]
pub mod mm;

pub fn allocate_frame(...) -> Frame { /* 40 lines of allocator logic */ }

// Good
// SPDX-License-Identifier: MPL-2.0
#![no_std]
#![deny(unsafe_code)]

pub mod mm;
pub mod scheduler;

pub use self::mm::Frame;

pub fn init() { /* delegates into subsystems */ }
```
