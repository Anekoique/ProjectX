# Naming

> Rules for Rust-idiomatic naming. Names must be accurate, unabbreviated, and follow [Rust API Guidelines on naming](https://rust-lang.github.io/api-guidelines/naming.html).

## R1 — Type names use Rust CamelCase with title-cased acronyms

**Applies to:** `**/*.rs`
**Evidence:** `clippy::upper_case_acronyms`

Per the Rust API Guidelines, acronyms in type names are title-cased, not all-caps: `IoMemoryArea`, not `IOMemoryArea`. Single-letter and two-letter acronyms (`Tcp`, `Pci`, `Nvme`) follow the same rule.

```rust
// Bad — all-caps acronyms
struct IOMemoryArea { ... }
struct PCIDeviceLocation { ... }
struct NVMe { ... }
struct TCP { ... }

// Good
struct IoMemoryArea { ... }
struct PciDeviceLocation { ... }
struct Nvme { ... }
struct Tcp { ... }
```

## R2 — Closure and function-pointer variables end with `_fn`

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Variables holding closures or function pointers signal they are callable by ending with `_fn`. Treating a closure variable as if it were a data object misleads readers.

```rust
// Bad — looks like a data field
let task = self.func.take().unwrap();
let thread = move || {
    let _ = oops::catch_panics_as_oops(task);
    current_thread!().exit();
};

// Good — _fn suffix marks the callable
let task_fn = self.func.take().unwrap();
let thread_fn = move || {
    let _ = oops::catch_panics_as_oops(task_fn);
    current_thread!().exit();
};
```
