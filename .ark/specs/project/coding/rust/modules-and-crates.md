# Modules and Crates

> Rules for visibility, import style, and dependency declaration. Narrow visibility keeps the TCB surface small; uniform import style keeps reviewer attention on the change, not the boilerplate.

## R1 — Default to the narrowest visibility; widen only when an external consumer requires it

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Start private, then widen to `pub(super)`, `pub(crate)`, or `pub` only when a real caller in that scope requires it. For workspace crates with no external consumers, `pub(crate)` and `pub` are equivalent — prefer the shorter `pub`.

```rust
// Bad — unnecessarily wide
pub static I8042_CONTROLLER:
    Once<SpinLock<I8042Controller, LocalIrqDisabled>> = Once::new();
pub fn init() -> Result<(), I8042ControllerError> { ... }

// Good — restricted to the parent module that actually uses it
pub(super) static I8042_CONTROLLER:
    Once<SpinLock<I8042Controller, LocalIrqDisabled>> = Once::new();
pub(super) fn init() -> Result<(), I8042ControllerError> { ... }
```

## R2 — Free functions and statics are called via their parent module, not imported directly

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

Import the parent module and access the item through it (`module::function()`, `module::CONSTANT`). The call site makes it clear an imported item is being used, not a local one, and the module name complements the item name. Types, traits, and enum variants are still imported directly by name.

```rust
// Bad — bare names; origin unclear at the call site
use ostd::irq::disable_local;
use ostd::mm::kspace::LINEAR_MAPPING_BASE_VADDR;

let guard = disable_local();
let base = LINEAR_MAPPING_BASE_VADDR;

// Good — module-qualified
use ostd::irq;
use ostd::mm::kspace;

let guard = irq::disable_local();
let base = kspace::LINEAR_MAPPING_BASE_VADDR;
```

## R3 — Shared dependencies are declared in `[workspace.dependencies]` and referenced as `workspace = true`

**Applies to:** `**/Cargo.toml` (member crates)
**Evidence:** grep

`AGENTS.md` codifies this: member `Cargo.toml` files declare deps via `workspace = true`, never duplicated version strings. Version drift is a real problem when several members independently bump.

```toml
# Bad — version string in a member crate
# in visor/Cargo.toml:
[dependencies]
ostd = { version = "0.1.0", path = "../ostd" }
bitflags = "1.3"

# Good
# in workspace root Cargo.toml:
[workspace.dependencies]
ostd = { version = "0.1.0", path = "ostd" }
bitflags = "1.3"

# in visor/Cargo.toml:
[dependencies]
ostd.workspace = true
bitflags.workspace = true
```
