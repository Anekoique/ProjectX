# Types and Traits

> Rules for using the type system as the primary correctness tool. The language-isolation premise rests on the type system catching what hardware isolation does not — these rules are how that promise is kept.

## R1 — Use the type system to make illegal states unrepresentable

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Define newtypes to encode domain constraints; prefer enums over bare integers and boolean flags; encode invariants in generic parameters where appropriate. A value whose type rules out the invalid case cannot be misused — and the reviewer no longer has to verify that no caller ever passes a wrong value.

```rust
// Bad — i8 admits invalid values for nice levels
pub type Nice = i8;

// Good — newtype enforces the range
pub struct Nice(NiceValue);
type NiceValue = RangedI8<-20, 19>;

// Bad — u8 admits invalid access modes
pub type AccessMode = u8;

// Good — enum makes the closed set explicit
pub enum AccessMode {
    O_RDONLY = 0,
    O_WRONLY = 1,
    O_RDWR   = 2,
}

// Good — generic parameter encodes capability
impl IoMem<Sensitive> {
    pub unsafe fn write_u32(&self, offset: usize, val: u32) { ... }
}
impl IoMem<Insensitive> {
    pub fn write_u32(&self, offset: usize, val: u32) { ... }  // safe
}
pub enum Sensitive {}
pub enum Insensitive {}
```

## R2 — Closed sets of variants are enums, not `Box<dyn Trait>`

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

When the set of variants is known and closed, an enum is preferable to a trait object: pattern matching is exhaustive (the compiler tells you when a new variant needs handling), there is no heap allocation, and method dispatch is direct. Use `dyn Trait` only when callers genuinely need to register their own variants.

```rust
// Bad — trait-object dispatch where the variant set is closed
pub trait Status: core::fmt::Debug {
    fn exit_code(&self) -> Option<u8>;
}
pub struct Exited(u8);
pub struct Killed(SigNum);
let status: Box<dyn Status> = ...;

// Good — enum exposes exhaustive matching and zero allocation
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TermStatus {
    Exited(u8),
    Killed(SigNum),
}
```

## R3 — Public structs expose getters, not public fields

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

A `pub` field is an irrevocable commitment to a representation. A getter preserves naming flexibility, leaves room for future invariants (validation, derivation from other state), and isolates downstream code from refactors of the struct.

```rust
// Bad — representation is part of the public contract forever
pub struct Vma {
    pub perms: VmPerms,
}

// Good — getter mediates access
pub struct Vma {
    perms: VmPerms,
}

impl Vma {
    pub fn perms(&self) -> VmPerms {
        self.perms
    }
}
```
