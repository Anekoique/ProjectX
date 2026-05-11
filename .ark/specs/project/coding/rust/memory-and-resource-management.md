# Memory and Resource Management

> Rules for resource acquisition and release. Rust's ownership model is the primary tool; this rule says use it.

## R1 — Resources are acquired and released through RAII guards, not paired calls

**Applies to:** `**/*.rs`
**Evidence:** VERIFY

IRQ enable/disable state, port numbers, file handles, DMA buffers, lock guards — all use the `Drop` trait for automatic cleanup. Manual `enable()`/`disable()` call pairs are a defect: a missed pair on an error path is a resource leak or worse. Prefer lexical lifetimes so the compiler inserts `drop` automatically; use explicit `drop()` only when the default drop order is wrong.

```rust
// Bad — caller can forget to re-enable on an error path
fn disable_local_irqs() { ... }
fn enable_local_irqs() { ... }

fn critical_section() -> Result<()> {
    disable_local_irqs();
    do_work()?;                    // ← leak if `?` returns
    enable_local_irqs();
    Ok(())
}

// Good — RAII guard re-enables on every exit path
fn disable_local() -> DisabledLocalIrqGuard { ... }

impl Drop for DisabledLocalIrqGuard {
    fn drop(&mut self) {
        enable_local_irqs();
    }
}

fn critical_section() -> Result<()> {
    let _guard = disable_local();
    do_work()?;                    // ← guard's drop runs even on early return
    Ok(())
}
```
