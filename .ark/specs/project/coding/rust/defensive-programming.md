# Defensive Programming

> Rules for assertion choice. Assertions verify invariants that must hold for the program to be correct; the right assertion type balances safety against runtime cost.

## R1 — Use `debug_assert!` for invariants that must hold in correct code

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

`debug_assert!` is compiled out in release builds, so the check catches bugs during development without costing anything in production. Reserve `assert!` for boundary checks where the value comes from outside the trust boundary and a release-build failure would be worse than a release-build halt.

```rust
// Bad — paying release-build cost for a developer-only sanity check
pub fn map(&mut self, paddr: PhysAddr, vaddr: Vaddr) {
    assert!(paddr.is_multiple_of(PAGE_SIZE));     // paddr produced internally
    assert!(self.align.is_power_of_two());        // self.align is an invariant
    ...
}

// Good
pub fn map(&mut self, paddr: PhysAddr, vaddr: Vaddr) {
    debug_assert!(paddr.is_multiple_of(PAGE_SIZE));
    debug_assert!(self.align.is_power_of_two());
    ...
}
```
