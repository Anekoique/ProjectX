# Comments and Documentation

> Rules for rustdoc comments and inline comments. The `#![warn(missing_docs)]` lint enforces the public-API baseline; these rules cover form and content.

## R1 — Doc-comment summary lines follow RFC 1574 grammar

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

The first line of a doc comment is one concise sentence. Functions and methods use a third-person singular present indicative verb ("Returns", "Creates", "Acquires"). Types, modules, and fields use a noun phrase naming the thing, not describing an action.

```rust
// Bad — function docs as a noun phrase
/// The mapping's start address.
pub fn map_to_addr(&self) -> Vaddr { ... }

// Bad — type docs as a verb phrase
/// Releases a `SpinLock` when dropped.
pub struct SpinLockGuard<'a, T> { ... }

// Good
/// Returns the mapping's start address.
pub fn map_to_addr(&self) -> Vaddr { ... }

/// A guard that releases a [`SpinLock`] when dropped.
pub struct SpinLockGuard<'a, T> { ... }
```

## R2 — Identifiers in doc comments are wrapped in backticks or rustdoc links

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Type names, method names, and code identifiers in doc comments wrap in backticks for rustdoc rendering. When referring to a type, prefer a rustdoc link (`[TypeName]`) so generated docs cross-reference correctly.

```rust
// Bad — bare identifiers
/// Acquires the SpinLock and returns a guard
/// that releases the lock on Drop.
pub fn acquire(&self) -> SpinLockGuard<'_, T> { ... }

// Good
/// Acquires the [`SpinLock`] and returns a guard
/// that releases the lock on [`Drop`].
pub fn acquire(&self) -> SpinLockGuard<'_, T> { ... }
```

## R3 — Doc comments do not disclose implementation details

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Doc comments describe *what* the API does and *how to use it*, not *how it is implemented internally*. The implementation can change without warning; the docs cannot.

```rust
// Bad — leaks internal data structure
/// Returns the length of the internal `HashMap`
/// that tracks connections by socket address.
pub fn connection_count(&self) -> usize { ... }

// Good — describes observable behavior
/// Returns the number of active connections.
pub fn connection_count(&self) -> usize { ... }
```

## R4 — Modules that anchor a subsystem open with `//!` documentation

**Applies to:** `**/mod.rs`
**Evidence:** `clippy::missing_docs`

A module that serves as a subsystem entry point, a major data structure home, or a driver opens with a `//!` comment naming what the module does, the key types it exposes, and how it relates to neighboring modules.

```rust
// Bad
// SPDX-License-Identifier: MPL-2.0
pub use self::vmar::Vmar;
pub mod vmar;

// Good
// SPDX-License-Identifier: MPL-2.0

//! Virtual memory area (VMA) management.
//!
//! Defines [`VmMapping`] and associated types, which represent contiguous
//! regions of a process's virtual address space. VMAs are managed by the
//! [`Vmar`] tree in the parent module.

pub use self::vmar::Vmar;
pub mod vmar;
```
