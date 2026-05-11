# General Guidelines

> Cross-language rules for naming, comments, and API design across the workspace. Rust-specific rules live in `coding/rust/`; assembly rules in `coding/asm/`.

## R1 — Names convey meaning at the point of use

**Applies to:** `**/*`
**Evidence:** VERIFY

Avoid single-letter names and ambiguous abbreviations. Prefer full words over cryptic shorthand so readers do not need surrounding context to understand a variable's purpose. As short as possible while still unambiguous.

```rust
// Bad
fn p(b: &[u8], n: usize) -> Result<()> { ... }

// Good
fn parse(buf: &[u8], len: usize) -> Result<()> { ... }
```

## R2 — Names accurately reflect the work being done

**Applies to:** `**/*`
**Evidence:** VERIFY

If a name can be misread to imply the wrong meaning, behavior, or side effects, it must be corrected. Choose verbs that reflect the actual work: a method that performs MMIO is `read_command`, not `command`; a method that walks a collection is `collect_all`, not `get_all`.

```rust
// Bad — sounds like a plain field accessor
pub fn command(&self) -> Command { /* MMIO read */ }

// Good — implies a side-effecting hardware access
pub fn read_command(&self) -> Command { /* MMIO read */ }
```

## R3 — Names encode units and important attributes when the type does not

**Applies to:** `**/*.rs`, `**/*.S`, `**/*.toml`
**Evidence:** VERIFY

Kernel code deals in bytes, pages, frames, nanoseconds, ticks, and sectors — ambiguous units cause real bugs. Where the type system can enforce units (newtypes), prefer that; where it cannot, the name must carry the information.

```rust
// Bad
fn sleep(timeout: u64) { ... }
fn map(offset: usize, size: usize) { ... }

// Good
fn sleep(timeout_ns: u64) { ... }
fn map(offset_bytes: usize, size_pages: usize) { ... }
```

## R4 — Comments explain why, not what

**Applies to:** `**/*`
**Evidence:** VERIFY

If a comment merely paraphrases the code, it adds noise without insight. If a comment is needed to explain *what* code does, first try to rewrite the code to be clearer. Comments are for the *why* — intent, constraint, alternative considered.

```rust
// Bad — restates the code
// Increment counter by 1
counter += 1;

// Good — explains why this specific operation
// Compensate for the off-by-one in the hardware tick counter (errata 0x42).
counter += 1;
```

## R5 — Document non-obvious design decisions where they live

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

When the code makes a non-obvious choice — a particular data structure, a locking strategy, a deviation from a familiar pattern — add a comment explaining the rationale and any alternatives considered. Design-decision comments are the most valuable kind of comment.

```rust
// Bad — silent unusual choice
struct PageMap { entries: BTreeMap<Vaddr, PageEntry> }

// Good
// We use a BTreeMap rather than a HashMap because lookups must be
// O(log n) worst-case for the page-fault handler. A HashMap gives
// O(1) amortized but O(n) worst-case due to rehashing, which is
// unacceptable on the page-fault path.
struct PageMap { entries: BTreeMap<Vaddr, PageEntry> }
```

## R6 — Cite specifications and algorithm sources

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

When implementing behavior defined by an external specification or a non-trivial algorithm, cite the source: the relevant POSIX section, Linux man page, hardware reference manual, or academic paper.

```rust
// Bad
const PIPE_BUF: usize = 4096;

// Good
/// Maximum number of bytes guaranteed to be written to a pipe atomically.
///
/// See the description of `PIPE_BUF` in
/// <https://man7.org/linux/man-pages/man7/pipe.7.html>.
const PIPE_BUF: usize = 4096;
```

## R7 — Public APIs follow familiar Rust and Linux conventions

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Prefer names and API shapes that users already know from Rust and Linux. Do not invent new terms for well-known operations.

```rust
// Bad — unfamiliar synonyms for common operations
pub fn length(&self) -> usize { ... }
pub fn to_pointer(&self) -> *const u8 { ... }

// Good — established Rust naming conventions
pub fn len(&self) -> usize { ... }
pub fn as_ptr(&self) -> *const u8 { ... }
```

## R8 — Public APIs do not expose implementation details

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

A module's public surface contains only what its consumers need. Implementation details — internal data structures, internal field names, transient state — must not appear in public types, public method signatures, or their documentation.

```rust
// Bad — exposes internal HashMap representation
/// Returns the length of the internal `HashMap` that tracks connections.
pub fn connection_count(&self) -> usize { ... }

// Good — describes observable behavior only
/// Returns the number of active connections.
pub fn connection_count(&self) -> usize { ... }
```

## R9 — Validate at system boundaries, trust internally

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Designate certain interfaces as validation boundaries: hypercall entry points, syscall entry points, FFI surfaces. All externally-supplied data (pointers, file descriptors, sizes, flags, strings) must be validated at the boundary. Once validated, internal functions may trust these values without re-validation.

```rust
// Bad — every internal function re-validates user input
fn copy_from_user(addr: Vaddr, len: usize) -> Result<Vec<u8>> {
    if addr == 0 { return Err(...) }   // again
    if len > MAX { return Err(...) }   // again
    ...
}

// Good — validation lives at the syscall boundary; internal trusts it
pub fn sys_write(buf: Vaddr, len: usize) -> Result<usize> {
    let bytes = read_bytes_from_user(buf, len)?;  // boundary
    write_to_file(&bytes)                          // trusts `bytes`
}
```
