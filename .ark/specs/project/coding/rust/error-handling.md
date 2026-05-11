# Error Handling

> Rules for `Result`, `?`, and the no-`.unwrap()`-in-production policy. The TCB-size argument depends on not silently absorbing errors.

## R1 — Propagate errors with `?`; do not call `.unwrap()` in non-test code

**Applies to:** `**/*.rs` (excluding `tests/**`, `**/test.rs`, and `#[cfg(test)]` blocks)
**Evidence:** `clippy::unwrap_used`

`AGENTS.md` codifies this rule for the whole workspace. Use `?` to propagate, or `.expect("invariant reason")` only when `None`/`Err` is logically impossible (e.g., a hardcoded regex, a mutex that cannot be poisoned given the surrounding ownership). A bare `.unwrap()` in production code is a defect.

```rust
// Bad — unwrap in production code hides the failure path
let tsc_info = cpuid.get_tsc_info().unwrap();
let frequency = tsc_info.nominal_frequency().unwrap();

// Good — propagate
let tsc_info = cpuid.get_tsc_info()?;
let frequency = tsc_info.nominal_frequency()?;

// Good — `expect` with a reason where None is logically impossible
let regex = Regex::new(r"^[A-Z]+$").expect("hardcoded regex always parses");
```
