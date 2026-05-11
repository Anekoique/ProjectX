# Variables, Expressions, and Statements

> Rules for variable binding, expression clarity, and overflow handling. Immutability and overflow discipline are correctness-bearing; explaining variables and block-scoped temporaries are reviewer-comprehension aids.

## R1 — Variables are immutable by default; `mut` is used only when mutation is required

**Applies to:** `**/*.rs`
**Evidence:** `clippy::unused_mut`

`AGENTS.md` codifies this: `mut` only when mutation is required. Every `let mut` is a small commitment that some code path will reassign or mutate this binding — leaving the keyword off when it is not needed forces a reviewer to find the mutation, which is a small but real cost on every read.

```rust
// Bad — mut without an actual mutation
let mut name = String::from("hello");
let updated = format!("{name} world");

// Good
let name = String::from("hello");
let updated = format!("{name} world");

// Good — mut is required because the binding is reassigned
let mut count = 0;
for item in items {
    if item.is_active() { count += 1; }
}
```

## R2 — Break complex expressions into named intermediates

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

An explaining variable turns an opaque expression into self-documenting code. Reviewers no longer have to parse the whole expression to understand intent.

```rust
// Bad — reader must reconstruct intent from the operator soup
debug_assert!(addr % PAGE_SIZE == 0 && addr < max_addr);

// Good — intent is named
let is_page_aligned = addr % PAGE_SIZE == 0;
let is_within_range = addr < max_addr;
debug_assert!(is_page_aligned && is_within_range);
```

## R3 — Use a block expression to scope temporaries that produce one final value

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Block expressions keep one-off intermediates from leaking into the outer scope, which makes later reads of the function unambiguous about what is in scope where.

```rust
// Bad — `bytes` lingers in outer scope after it is no longer needed
let bytes = read_bytes_from_user(addr, len as usize)?;
let socket_addr = parse_socket_addr(&bytes)?;
connect(socket_addr)?;

// Good — `bytes` is scoped to the block
let socket_addr = {
    let bytes = read_bytes_from_user(addr, len as usize)?;
    parse_socket_addr(&bytes)?
};
connect(socket_addr)?;
```

## R4 — Arithmetic that can overflow uses checked, saturating, or explicit wrapping operations

**Applies to:** `**/*.rs`
**Evidence:** `clippy::arithmetic_side_effects`

Plain `+`/`-`/`*` panics in debug builds and silently wraps in release — the worst of both worlds. Use `checked_*` when overflow is an error, `saturating_*` when clamping is correct, or `wrapping_*`/`overflowing_*` when wrapping is the intended semantics (and document why).

```rust
// Bad — silently wraps in release builds
let total = base + offset;

// Good — overflow handled explicitly
let total = base.checked_add(offset)
    .ok_or(Error::new(Errno::EOVERFLOW))?;

// Good — clamps
let remaining = budget.saturating_sub(cost);

// Good — wrap is intentional and explained
// Sequence numbers wrap by design at u32::MAX (RFC 793, §3.3).
let next = current.wrapping_add(1);
```
