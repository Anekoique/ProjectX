# Macros and Attributes

> Rules for attribute ordering, lint suppression, dead code, and when to reach for a macro. Macros are powerful and unaudited code generation; the rules here keep the audit surface small.

## R1 — `#[derive(...)]` is the last attribute on an item

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Derive macros run after attribute macros, so the item they see must already be transformed by any preceding `#[padding_struct]`, `#[pod_union]`, or similar. Placing `#[derive(...)]` last makes that ordering explicit. Derive helper attributes (`#[serde(...)]`, `#[clap(...)]`) sit immediately after `#[derive(...)]`.

```rust
// Bad — derive runs before #[repr(C)] is fully composed
#[derive(Clone, Copy, Debug, Default, Pod)]
#[cfg(feature = "alloc")]
#[repr(C)]
pub struct Foo { ... }

// Good — non-derive attributes first, derive last
#[cfg(feature = "alloc")]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod)]
pub struct Foo { ... }
```

## R2 — `#[expect(dead_code)]` is permitted only when a near-future use is concrete

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Dead code adds maintenance overhead and its correctness can only be checked by manual review. Allow it only when all four hold: a *concrete case* will turn it into used code soon; the semantics are *clear* without the use case; the dead code is *simple* enough to be confidently correct without testing; and it serves as a counterpart to existing non-dead code (e.g., ABI constants for a partially implemented feature).

```rust
// Bad — speculative dead code with no near-future caller
#[expect(dead_code)]
fn maybe_useful_someday(...) { ... }

// Good — counterpart constant for a partially implemented feature
#[expect(dead_code)]
const VIRTIO_F_RING_PACKED: u64 = 1 << 34;  // not yet used; packed-ring
                                             // support tracked in #99
```

## R3 — Lint suppressions cover the smallest possible scope

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Suppress at the item level, not the module or trait level. Readers can see the exact place the lint is generated; future committers can maintain or remove the suppression with confidence. Exception: a type whose every member triggers the lint may carry a type-level expectation.

```rust
// Bad — suppresses the entire trait, hides which methods need the expectation
#[expect(dead_code)]
trait SomeTrait {
    fn foo();
    fn bar();
    fn baz();
}

// Good — per-item, minimal scope
trait SomeTrait {
    #[expect(dead_code)]
    fn foo();
    #[expect(dead_code)]
    fn bar();
    fn baz();
}

// Good exception — every variant intentionally non-CamelCase
#[expect(non_camel_case_types)]
enum SomeEnum {
    FOO_ABC,
    BAR_DEF,
}
```

## R4 — Reach for a macro only when the type system cannot express the need

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Macros are harder to understand, debug, test, and format than functions. Use them for variadic arguments, compile-time code generation, or DSL syntax — and only after a generic function has been ruled out.

```rust
// Bad — macro where a generic function works
macro_rules! align_up {
    ($val:expr, $align:expr) => {
        ($val + $align - 1) & !($align - 1)
    };
}

// Good
fn align_up<T: Into<usize>>(val: T, align: usize) -> usize {
    let val = val.into();
    (val + align - 1) & !(align - 1)
}
```
